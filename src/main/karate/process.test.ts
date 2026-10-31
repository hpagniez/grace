import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { constants, getPriority, tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { copyFixture } from "../../../test/fixtures/copy";
import { vendorPaths } from "../vendor";
import { buildArgs, detectLayout } from "./command";
import { killOrphan, spawnKarate, within, type KarateProcess } from "./process";

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
  const args = buildArgs({
    jar,
    classpath,
    root,
    outDir,
    dryRun,
    targets,
    tags: [],
    env: null,
    argFile: null,
    threads: 1,
  });
  const p = spawnKarate(java, args, root, onLines);
  procs.push(p);
  return Object.assign(p, { root });
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

  it("runs java below normal priority (#148)", { timeout: 60_000 }, async () => {
    const p = start(false, () => {}, "tests/slow.feature");
    expect(getPriority(p.pid)).toBe(constants.priority.PRIORITY_BELOW_NORMAL);
  });

  it("kill leaves no java behind", { timeout: 60_000 }, async () => {
    const p = start(false, () => {}, "tests/slow.feature");
    await sleep(1500);
    await p.kill();
    await p.exit;
    expect(alive(p.pid)).toBe(false);
    // no java of this project may survive; other test files run their own java meanwhile
    const left = execFileSync(
      "powershell",
      [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").CommandLine",
      ],
      { encoding: "utf8" },
    );
    expect(left).not.toContain(p.root);
    await p.kill(); // already gone: must not throw
  });
});

describe("spawnKarate kill", () => {
  it("gives up waiting when the process does not end", { timeout: 60_000 }, async () => {
    // The child starts a detached grandchild that keeps its pipes open, then exits:
    // taskkill finds no process and `exit` cannot resolve until the grandchild ends.
    const script = [
      "const { spawn } = require('node:child_process');",
      "const g = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 60000)'],",
      "  { stdio: 'inherit', detached: true, windowsHide: true });",
      "console.log(g.pid); g.unref();",
    ].join("\n");
    const lines: string[] = [];
    const p = spawnKarate(process.execPath, ["-e", script], tmpdir(), (l) => lines.push(...l), 200);
    const end = Date.now() + 20_000;
    while ((lines.length === 0 || alive(p.pid)) && Date.now() < end) await sleep(100);
    const grandchild = Number(lines[0]);
    try {
      expect(alive(p.pid)).toBe(false);
      const t0 = Date.now();
      await p.kill();
      expect(Date.now() - t0).toBeLessThan(10_000);
      expect(await within(p.exit, 100)).toBe(false); // still open: kill() returned anyway
    } finally {
      process.kill(grandchild);
    }
    expect(await p.exit).toBe(0);
  });
});

describe("within", () => {
  it("tells whether a promise settles in time", async () => {
    expect(await within(Promise.resolve(), 50)).toBe(true);
    expect(await within(new Promise(() => {}), 50)).toBe(false);
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
