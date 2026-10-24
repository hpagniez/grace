import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { copyFixture } from "../../../test/fixtures/copy";
import { vendorPaths } from "../vendor";
import { buildArgs, detectLayout } from "./command";
import { killOrphan, spawnKarate, type KarateProcess } from "./process";

const { java, jar, jreDir } = vendorPaths(path.resolve("vendor"));
const temps: string[] = [];
const procs: KarateProcess[] = [];

afterEach(async () => {
  await Promise.all(procs.splice(0).map((p) => p.kill()));
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true });
});

function start(dryRun: boolean, onLines: (l: string[]) => void, feature?: string) {
  const root = copyFixture("clean");
  const outDir = mkdtempSync(path.join(tmpdir(), "kk-out-"));
  temps.push(root, outDir);
  const { classpath, featureRoots } = detectLayout(root);
  const targets = feature ? [path.join(root, ...feature.split("/"))] : featureRoots;
  const args = buildArgs({ jar, classpath, root, outDir, dryRun, targets });
  const p = spawnKarate(java, args, root, onLines);
  procs.push(p);
  return p;
}

const alive = (pid: number) =>
  execFileSync("tasklist", ["/FI", `PID eq ${pid}`, "/NH"], { encoding: "utf8" }).includes(
    String(pid),
  );
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const pidFile = (pid: number | string) => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-pid-"));
  temps.push(d);
  const f = path.join(d, "running.pid");
  writeFileSync(f, String(pid));
  return f;
};

describe("spawnKarate", () => {
  it("runs karate and streams console lines", { timeout: 60_000 }, async () => {
    const lines: string[] = [];
    const p = start(true, (l) => lines.push(...l));
    expect(await p.exit).toBe(0);
    expect(lines.some((l) => l.includes("DRY RUN"))).toBe(true);
  });

  it("reports a missing java as one line and a null exit", async () => {
    const lines: string[] = [];
    const p = spawnKarate(path.join(tmpdir(), "nope", "java.exe"), [], tmpdir(), (l) =>
      lines.push(...l),
    );
    expect(await p.exit).toBeNull();
    expect(lines).toHaveLength(1);
    expect(lines[0]).toContain("ENOENT");
  });

  it("kill leaves no java behind", { timeout: 60_000 }, async () => {
    const p = start(false, () => {}, "tests/slow.feature");
    await sleep(1500);
    await p.kill();
    await p.exit;
    expect(alive(p.pid)).toBe(false);
    // no java from our jre may survive (tests run serially in this file)
    const left = execFileSync(
      "powershell",
      [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").ExecutablePath",
      ],
      { encoding: "utf8" },
    );
    expect(left.toLowerCase()).not.toContain(path.resolve(jreDir).toLowerCase());
    await p.kill(); // already gone: must not throw
  });
});

describe("killOrphan", () => {
  it("kills only our java", { timeout: 60_000 }, async () => {
    const p = start(false, () => {}, "tests/slow.feature");
    await sleep(1500);
    const file = pidFile(p.pid);
    await killOrphan(file, jreDir);
    expect(await p.exit).not.toBe(0);
    expect(alive(p.pid)).toBe(false);
    expect(existsSync(file)).toBe(false);

    const other = pidFile(process.pid);
    await killOrphan(other, jreDir);
    expect(alive(process.pid)).toBe(true);
    expect(existsSync(other)).toBe(false);
  });

  it("ignores a missing or garbage pid file", async () => {
    await killOrphan(path.join(tmpdir(), "kk-missing.pid"), jreDir);
    const f = pidFile("1; calc");
    await killOrphan(f, jreDir);
    expect(existsSync(f)).toBe(false);
  });
});
