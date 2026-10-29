import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  linkSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { defaultUpdateSettings } from "../../shared/updates";
import { POWERSHELL, startSwitch, type SwitchPlan } from "./switch";
import script from "./switch.ps1?raw";

const T = { timeout: 60_000 };
const SETTINGS = '{"theme":"Zoë"}';
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const alive = (pid: number) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
};

describe.runIf(process.platform === "win32")("switch script", () => {
  let root: string;
  let template: string;
  let env: NodeJS.ProcessEnv;
  let install: string;
  let staging: string;
  let data: string;
  const children: ChildProcess[] = [];

  /** Ids of the processes whose exe lives under `folder`. */
  const pidsUnder = (folder: string): number[] =>
    execFileSync(
      POWERSHELL,
      [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Get-Process | Where-Object { $_.Path -and $_.Path.StartsWith($env:UNDER, 'OrdinalIgnoreCase') } | ForEach-Object { $_.Id }",
      ],
      { env: { ...process.env, UNDER: folder + path.sep }, encoding: "utf8", windowsHide: true },
    )
      .split(/\s+/)
      .filter(Boolean)
      .map(Number);

  const read = (dir: string, file: string) => readFileSync(path.join(dir, file), "utf8");
  const result = () => JSON.parse(read(data, "update-result.json"));
  const write = (file: string, text = "") => {
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, text);
  };
  /** A stand-in Grace: node under the name Grace.exe, made to behave by the stub in NODE_OPTIONS. */
  const app = (dir: string, version: string) => {
    mkdirSync(path.join(dir, "resources"), { recursive: true });
    linkSync(template, path.join(dir, "Grace.exe"));
    write(path.join(dir, "v.txt"), version);
    write(path.join(dir, "resources", "app.asar"));
  };
  const savedCopy = (version: string, savedAt: string, files: Record<string, string> = {}) => {
    const dir = path.join(data, "backups", version);
    write(path.join(dir, "backup.json"), JSON.stringify({ version, savedAt }));
    for (const [f, text] of Object.entries(files)) write(path.join(dir, f), text);
  };
  const plan = (p: Partial<SwitchPlan>): SwitchPlan => ({
    pid: 0,
    from: "1.0.0-rc.5",
    to: "1.0.0-rc.6",
    rollback: false,
    restore: false,
    installDir: install,
    stagingDir: staging,
    dataDir: data,
    updates: { ...defaultUpdateSettings("1.0.0-rc.5"), skipped: "1.0.0-rc.4" },
    exitTimeoutSec: 10,
    startTimeoutSec: 20,
    keepBackups: 3,
    ...p,
  });

  /** Another process holds `file` open without sharing, as Explorer or an antivirus can. */
  async function hold(file: string) {
    const holder = spawn(
      POWERSHELL,
      [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "$f = [IO.File]::Open($env:HOLD, 'Open', 'Read', 'None'); 'held'; [Console]::In.ReadLine()",
      ],
      { env: { ...process.env, HOLD: file }, windowsHide: true },
    );
    children.push(holder);
    await new Promise<void>((resolve, reject) => {
      holder.stdout.on("data", (d) => String(d).includes("held") && resolve());
      holder.on("exit", () => reject(new Error("the holder exited")));
    });
  }

  /**
   * Starts the old Grace in its own folder, as Explorer does, hands over to the script from that
   * folder too, quits Grace unless `quit` is false, and waits until the script has finished.
   */
  async function run(p: Partial<SwitchPlan> = {}, quit = true) {
    const old = spawn(path.join(install, "Grace.exe"), [], {
      cwd: install,
      env,
      stdio: "ignore",
      windowsHide: true,
    });
    children.push(old);
    const cwd = process.cwd();
    process.chdir(install);
    try {
      startSwitch(plan({ pid: old.pid, ...p }), env);
    } finally {
      process.chdir(cwd);
    }
    // R10.9: the script runs on its own and holds its id in switching, though the launcher is gone
    const pid = Number(read(staging, "switching"));
    expect(alive(pid)).toBe(true);
    if (quit) setTimeout(() => old.kill(), 300);
    for (const end = Date.now() + 45_000; Date.now() < end; await sleep(200)) {
      if (!alive(pid)) {
        expect(existsSync(path.join(staging, "switching"))).toBe(false);
        return;
      }
    }
    throw new Error("the switch script did not finish");
  }

  async function until(check: () => boolean) {
    for (const end = Date.now() + 15_000; Date.now() < end; await sleep(200)) if (check()) return;
    throw new Error("timed out");
  }

  beforeAll(() => {
    root = mkdtempSync(path.join(tmpdir(), "grace-switch-"));
    // node.exe cannot be hard-linked from Program Files: copy it once, link each Grace.exe to it
    template = path.join(root, "node.exe");
    copyFileSync(process.execPath, template);
    const stub = path.join(root, "stub.js");
    writeFileSync(
      stub,
      `const fs = require("fs"), path = require("path");
const dir = path.dirname(process.execPath);
if (fs.existsSync(path.join(dir, "die.flag"))) process.exit(1);
if (!fs.existsSync(path.join(dir, "noStart.flag"))) fs.writeFileSync(dir + ".update/started", "");
setInterval(() => {}, 1 << 30);
`,
    );
    // NODE_OPTIONS reads backslashes in quotes as escapes
    env = { ...process.env, NODE_OPTIONS: `--require "${stub.replaceAll("\\", "/")}"` };
  });

  beforeEach(() => {
    // a space, a non-ASCII letter and a wildcard in every path (plan Review Focus 1)
    const dir = mkdtempSync(path.join(root, "t-"));
    install = path.join(dir, "Grace [dir] Zoë");
    staging = `${install}.update`;
    data = path.join(dir, "Grace [data] Zoë");
    app(install, "1.0.0-rc.5");
    // the old version starting again must not look like the new one starting
    write(path.join(install, "noStart.flag"));
    app(path.join(staging, "app"), "1.0.0-rc.6");
    write(path.join(data, "settings.json"), SETTINGS);
    write(path.join(data, "projects", "p", "runs", "r", "run.json"), "{}");
    write(path.join(data, "Cache", "x"));
    write(path.join(data, "Local Storage", "y"));
  });

  afterEach(() => {
    for (const c of children.splice(0)) {
      c.stdin?.end();
      c.kill();
    }
    if (existsSync(path.join(staging, "switching"))) {
      try {
        process.kill(Number(read(staging, "switching")));
      } catch {
        // already gone
      }
    }
    for (const pid of pidsUnder(root)) {
      try {
        process.kill(pid);
      } catch {
        // already gone
      }
    }
    rmSync(path.dirname(install), {
      recursive: true,
      force: true,
      maxRetries: 20,
      retryDelay: 250,
    });
  });

  afterAll(() => rmSync(root, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 }));

  it("is ASCII: Windows PowerShell reads a script without a BOM in the ANSI code page", () => {
    expect(script).toMatch(/^[\t\n\r -~]*$/);
  });

  it("swaps the install folder and starts the new version", T, async () => {
    await run();
    expect(read(install, "v.txt")).toBe("1.0.0-rc.6");
    expect(read(`${install}.old`, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(path.join(staging, "app"))).toBe(false);
    expect(result()).toEqual({
      from: "1.0.0-rc.5",
      to: "1.0.0-rc.6",
      rollback: false,
      ok: true,
      reverted: false,
      error: null,
      updates: plan({}).updates,
    });
    await until(() => existsSync(path.join(staging, "started")));
    // the data stays on an update (R9.4)
    expect(read(data, "settings.json")).toBe(SETTINGS);
  });

  it("backs up the data before the swap, without caches (R9.1)", T, async () => {
    write(path.join(data, "projects", "p", "Cache", "z"));
    write(path.join(data, "Code Cache", "c"));
    write(path.join(data, "lockfile"));
    write(path.join(data, "update-result.json"), "{}");
    savedCopy("1.0.0-rc.5", "2026-01-01T00:00:00.000Z", { "stale.txt": "" });
    await run();
    const b = path.join(data, "backups", "1.0.0-rc.5");
    expect(existsSync(path.join(b, "projects", "p", "runs", "r", "run.json"))).toBe(true);
    expect(existsSync(path.join(b, "projects", "p", "Cache", "z"))).toBe(true);
    expect(existsSync(path.join(b, "Local Storage", "y"))).toBe(true);
    expect(read(b, "settings.json")).toBe(SETTINGS);
    for (const left of [
      "Cache",
      "Code Cache",
      "lockfile",
      "update-result.json",
      "backups",
      "stale.txt",
    ])
      expect(existsSync(path.join(b, left)), left).toBe(false);
    const meta = JSON.parse(read(b, "backup.json"));
    expect(meta.version).toBe("1.0.0-rc.5");
    expect(meta.savedAt).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$/);
    expect(Date.now() - Date.parse(meta.savedAt)).toBeLessThan(60_000);
  });

  it("restores the target's saved data when the plan says so (R9.3)", T, async () => {
    write(path.join(staging, "app", "v.txt"), "1.0.0-rc.4");
    savedCopy("1.0.0-rc.4", "2026-01-01T00:00:00.000Z", {
      "settings.json": "rc.4",
      "Local Storage/old": "",
    });
    await run({ to: "1.0.0-rc.4", rollback: true, restore: true });
    expect(result()).toMatchObject({ to: "1.0.0-rc.4", rollback: true, ok: true });
    expect(read(data, "settings.json")).toBe("rc.4");
    expect(existsSync(path.join(data, "Local Storage", "old"))).toBe(true);
    expect(existsSync(path.join(data, "projects"))).toBe(false);
    expect(existsSync(path.join(data, "backup.json"))).toBe(false);
    // backups\ itself untouched, plus the copy of the running version
    expect(read(path.join(data, "backups", "1.0.0-rc.4"), "settings.json")).toBe("rc.4");
    expect(existsSync(path.join(data, "backups", "1.0.0-rc.4", "backup.json"))).toBe(true);
    expect(read(path.join(data, "backups", "1.0.0-rc.5"), "settings.json")).toBe(SETTINGS);
  });

  it("leaves the data when the target has no saved copy (R9.3)", T, async () => {
    await run({ restore: true });
    expect(result()).toMatchObject({ ok: true });
    expect(read(data, "settings.json")).toBe(SETTINGS);
    expect(existsSync(path.join(data, "projects", "p", "runs", "r", "run.json"))).toBe(true);
  });

  it("keeps the copies of the 3 most recently backed-up versions (R9.5)", T, async () => {
    // saved in another order than their versions: the date decides
    savedCopy("1.0.0-rc.2", "2026-03-01T00:00:00.000Z");
    savedCopy("1.0.0-rc.3", "2026-01-01T00:00:00.000Z");
    savedCopy("1.0.0-rc.4", "2026-02-01T00:00:00.000Z");
    await run();
    const kept = ["1.0.0-rc.2", "1.0.0-rc.3", "1.0.0-rc.4", "1.0.0-rc.5"].filter((v) =>
      existsSync(path.join(data, "backups", v)),
    );
    expect(kept).toEqual(["1.0.0-rc.2", "1.0.0-rc.4", "1.0.0-rc.5"]);
  });

  it("puts the old folder and data back when a rename fails (R10.4)", T, async () => {
    write(path.join(staging, "app", "v.txt"), "1.0.0-rc.4");
    savedCopy("1.0.0-rc.4", "2026-01-01T00:00:00.000Z", { "settings.json": "rc.4" });
    await hold(path.join(install, "resources", "app.asar"));
    await run({ to: "1.0.0-rc.4", rollback: true, restore: true });
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(`${install}.old`)).toBe(false);
    expect(read(path.join(staging, "app"), "v.txt")).toBe("1.0.0-rc.4");
    expect(result()).toMatchObject({ ok: false, reverted: false });
    expect(result().error).toEqual(expect.any(String));
    // the running version's data, put back from the copy just made
    expect(read(data, "settings.json")).toBe(SETTINGS);
    expect(existsSync(path.join(data, "projects", "p", "runs", "r", "run.json"))).toBe(true);
    // the old version started again
    expect(pidsUnder(install)).not.toHaveLength(0);
  });

  it("puts the old folder back when the second rename fails (R10.4)", T, async () => {
    write(path.join(staging, "app", "v.txt"), "1.0.0-rc.4");
    savedCopy("1.0.0-rc.4", "2026-01-01T00:00:00.000Z", { "settings.json": "rc.4" });
    await hold(path.join(staging, "app", "resources", "app.asar"));
    await run({ to: "1.0.0-rc.4", rollback: true, restore: true });
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(`${install}.old`)).toBe(false);
    expect(read(path.join(staging, "app"), "v.txt")).toBe("1.0.0-rc.4");
    expect(result()).toMatchObject({ ok: false, reverted: false });
    expect(read(data, "settings.json")).toBe(SETTINGS);
    expect(pidsUnder(install)).not.toHaveLength(0);
  });

  it("stops before the swap when the backup fails (R9.6)", T, async () => {
    await hold(path.join(data, "settings.json"));
    await run();
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(`${install}.old`)).toBe(false);
    expect(result()).toMatchObject({ ok: false, reverted: false });
    expect(result().error).toContain("robocopy");
    // no half copy left behind
    expect(existsSync(path.join(data, "backups", "1.0.0-rc.5"))).toBe(false);
    expect(pidsUnder(install)).not.toHaveLength(0);
  });

  it("gives up when Grace does not exit (R10.2)", T, async () => {
    await run({ exitTimeoutSec: 2 }, false);
    expect(result()).toMatchObject({ ok: false, reverted: false });
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(`${install}.old`)).toBe(false);
    expect(existsSync(path.join(staging, "app", "Grace.exe"))).toBe(true);
    expect(existsSync(path.join(data, "backups"))).toBe(false);
  });

  it.each([".old", ".failed"])(
    "refuses a %s that is not a Grace folder and changes nothing",
    T,
    async (suffix) => {
      const mine = path.join(install + suffix, "mine.txt");
      write(mine, "mine");
      await run();
      expect(result()).toMatchObject({ ok: false, reverted: false });
      expect(result().error).toBe(
        `${install}${suffix} is in the way and Grace didn't make it; move or rename it, then try again`,
      );
      expect(read(path.dirname(mine), "mine.txt")).toBe("mine");
      expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
      expect(existsSync(path.join(staging, "app", "Grace.exe"))).toBe(true);
      expect(existsSync(path.join(data, "backups"))).toBe(false);
      // the old version started again
      expect(pidsUnder(install)).not.toHaveLength(0);
    },
  );

  it("removes a .old left by an earlier switch", T, async () => {
    write(path.join(`${install}.old`, "resources", "app-files.json"), "[]");
    write(path.join(`${install}.old`, "left.txt"));
    await run();
    expect(result()).toMatchObject({ ok: true });
    expect(read(`${install}.old`, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(path.join(`${install}.old`, "left.txt"))).toBe(false);
  });

  it("writes update-result.json and backup.json without a byte order mark", T, async () => {
    await run();
    expect(readFileSync(path.join(data, "update-result.json"))[0]).not.toBe(0xef);
    expect(readFileSync(path.join(data, "backups", "1.0.0-rc.5", "backup.json"))[0]).not.toBe(0xef);
  });

  it("switches back when the new version does not start in time (R10.6)", T, async () => {
    write(path.join(staging, "app", "noStart.flag"));
    // the new version's saved data, restored before the swap, must go again
    savedCopy("1.0.0-rc.6", "2026-01-01T00:00:00.000Z", { "settings.json": "rc.6" });
    // files left by an earlier try prove nothing about this one
    write(path.join(staging, "started"));
    write(path.join(staging, "switching"), "999999");
    await run({ restore: true, startTimeoutSec: 3 });
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(read(`${install}.failed`, "v.txt")).toBe("1.0.0-rc.6");
    expect(existsSync(`${install}.old`)).toBe(false);
    // data put back from backups\1.0.0-rc.5
    expect(read(data, "settings.json")).toBe(SETTINGS);
    expect(existsSync(path.join(data, "projects", "p", "runs", "r", "run.json"))).toBe(true);
    expect(result()).toEqual({
      from: "1.0.0-rc.5",
      to: "1.0.0-rc.6",
      rollback: false,
      ok: false,
      reverted: true,
      error: "1.0.0-rc.6 didn't start within 2 minutes",
      updates: plan({}).updates,
    });
    // the new version stopped, the old one started again
    expect(pidsUnder(`${install}.failed`)).toHaveLength(0);
    expect(pidsUnder(install)).not.toHaveLength(0);
  });

  it("switches back at once when the new version quits without starting (R10.6)", T, async () => {
    write(path.join(staging, "app", "die.flag"));
    const start = Date.now();
    await run({ startTimeoutSec: 40 });
    expect(Date.now() - start).toBeLessThan(20_000);
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(result()).toMatchObject({
      ok: false,
      reverted: true,
      error: "1.0.0-rc.6 didn't start within 2 minutes",
    });
  });

  it("deletes old backups only after the new version started (R9.5)", T, async () => {
    write(path.join(staging, "app", "noStart.flag"));
    savedCopy("1.0.0-rc.2", "2026-01-01T00:00:00.000Z");
    savedCopy("1.0.0-rc.3", "2026-02-01T00:00:00.000Z");
    savedCopy("1.0.0-rc.4", "2026-03-01T00:00:00.000Z");
    await run({ startTimeoutSec: 3 });
    expect(result()).toMatchObject({ ok: false, reverted: true });
    for (const v of ["1.0.0-rc.2", "1.0.0-rc.3", "1.0.0-rc.4", "1.0.0-rc.5"])
      expect(existsSync(path.join(data, "backups", v, "backup.json")), v).toBe(true);
  });

  it("keeps a version installed when the old folder cannot be put back", T, async () => {
    write(path.join(staging, "app", "noStart.flag"));
    const old = `${install}.old`;
    const done = run({ startTimeoutSec: 5, exitTimeoutSec: 2 });
    await until(() => existsSync(path.join(old, "resources", "app.asar")));
    await hold(path.join(old, "resources", "app.asar"));
    await done;
    expect(read(install, "v.txt")).toBe("1.0.0-rc.6");
    expect(read(old, "v.txt")).toBe("1.0.0-rc.5");
    expect(result()).toMatchObject({ ok: false, reverted: true });
    // the failure to undo is reported: Grace must not say the old version is back
    expect(result().error).toMatch(
      /^1\.0\.0-rc\.6 didn't start within 2 minutes; putting 1\.0\.0-rc\.5 back failed: .+, so 1\.0\.0-rc\.6 stays installed$/,
    );
    expect(read(data, "settings.json")).toBe(SETTINGS);
  });

  it("writes a failure result when something unexpected fails before the swap", T, async () => {
    await run({ exitTimeoutSec: "soon" as unknown as number });
    expect(result()).toMatchObject({ ok: false, reverted: false });
    expect(result().error).toEqual(expect.any(String));
    expect(read(install, "v.txt")).toBe("1.0.0-rc.5");
    expect(existsSync(path.join(staging, "app", "Grace.exe"))).toBe(true);
  });

  it(
    "removes an old copy's backup.json first: a half-removed copy is never trusted",
    T,
    async () => {
      savedCopy("1.0.0-rc.2", "2026-01-01T00:00:00.000Z", { "a.txt": "" });
      savedCopy("1.0.0-rc.3", "2026-02-01T00:00:00.000Z");
      savedCopy("1.0.0-rc.4", "2026-03-01T00:00:00.000Z");
      await hold(path.join(data, "backups", "1.0.0-rc.2", "a.txt"));
      await run();
      expect(result()).toMatchObject({ ok: true });
      expect(existsSync(path.join(data, "backups", "1.0.0-rc.2", "a.txt"))).toBe(true);
      expect(existsSync(path.join(data, "backups", "1.0.0-rc.2", "backup.json"))).toBe(false);
    },
  );
});
