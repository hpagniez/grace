import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  cpuLoad,
  folderMB,
  minimise,
  sample,
  splitTree,
  startSampler,
  waitForQuiet,
} from "./system.mjs";

// PowerShell starts slowly on a loaded CI machine
const SLOW = 60_000;

// Touches every page of 200 MiB, burns one second of its own CPU time (not wall time, which a busy
// machine stretches), starts the program in CHILD if there is one, tells the test it is ready, then
// holds all of it until killed.
const HOLDER = `
const buf = Buffer.alloc(200 * 1024 * 1024, 1);
const used = () => { const u = process.cpuUsage(); return u.user + u.system; };
while (used() < 1_000_000);
if (process.env.CHILD) require("node:child_process").spawn(process.env.CHILD, ["-n", "60", "127.0.0.1"], { stdio: "ignore" });
console.log("ready");
setInterval(() => { buf[0] = (buf[0] + 1) % 256; }, 1000);
`;

/** A Node child holding 200 MiB and 1 s of CPU time, once it says so, and `child` running under it. The caller `kill`s it. */
async function holder(child?: string): Promise<ChildProcess> {
  const proc = spawn(process.execPath, ["-e", HOLDER], {
    stdio: ["ignore", "pipe", "inherit"],
    env: { ...process.env, CHILD: child },
  });
  await new Promise<void>((resolve, reject) => {
    proc.once("error", reject);
    proc.once("exit", () => reject(new Error("the holder exited before it was ready")));
    proc.stdout!.once("data", () => resolve());
  });
  return proc;
}

/** Ends `proc` and everything it started. */
function kill(proc: ChildProcess) {
  try {
    execFileSync("taskkill", ["/PID", String(proc.pid), "/T", "/F"], { stdio: "ignore" });
  } catch {
    // already gone
  }
}

/** A reader that gives `values` one by one, then the last again. */
const fake = (values: number[]) => {
  let i = 0;
  return async () => values[Math.min(i++, values.length - 1)];
};

/** A copy of a small console program under the name `conhost.exe`, so a conhost sits in the tree without any Java. */
function fakeConhost(dir: string) {
  const exe = path.join(dir, "conhost.exe");
  copyFileSync(path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "ping.exe"), exe);
  return exe;
}

describe("splitTree", () => {
  const MB = 1e6;
  const proc = (id: number, parent: number, name: string, bytes: number, cpu: number) => ({
    id,
    parent,
    name,
    bytes,
    cpu,
  });

  it("counts java.exe and the conhost.exe under it as java, the rest as app", () => {
    const split = splitTree([
      proc(1, 0, "Grace.exe", 50 * MB, 10 * 1e4),
      proc(2, 1, "java.exe", 100 * MB, 40 * 1e4),
      proc(3, 2, "conhost.exe", 5 * MB, 1e4),
      proc(4, 1, "Grace.exe", 20 * MB, 5 * 1e4),
    ]);
    expect(split.app).toEqual({ privateMB: 70, cpuMs: 15 });
    expect(split.java).toEqual({ privateMB: 105, cpuMs: 41 });
  });

  it("counts a conhost.exe under anything but java.exe as app", () => {
    const split = splitTree([
      proc(1, 0, "Grace.exe", 50 * MB, 10 * 1e4),
      proc(2, 1, "conhost.exe", 5 * MB, 2e4),
      proc(3, 1, "cmd.exe", 3 * MB, 1e4),
      proc(4, 3, "conhost.exe", 4 * MB, 3e4),
    ]);
    expect(split.app).toEqual({ privateMB: 62, cpuMs: 16 });
    expect(split.java).toEqual({ privateMB: 0, cpuMs: 0 });
  });

  it("ignores the case of the names", () => {
    const split = splitTree([
      proc(1, 0, "Grace.exe", 50 * MB, 0),
      proc(2, 1, "JAVA.EXE", 100 * MB, 0),
      proc(3, 2, "Conhost.exe", 5 * MB, 0),
    ]);
    expect(split.java.privateMB).toBe(105);
  });
});

describe("sample", () => {
  it(
    "sums the memory and cpu of a process tree, with no java in it",
    { timeout: SLOW },
    async () => {
      const child = await holder();
      try {
        const s = await sample(child.pid!);
        expect(s.app.privateMB).toBeGreaterThanOrEqual(190);
        expect(s.app.cpuMs).toBeGreaterThanOrEqual(500);
        expect(s.java).toEqual({ privateMB: 0, cpuMs: 0 });
      } finally {
        kill(child);
      }
    },
  );

  it("counts a conhost.exe that is not under a java.exe as app", { timeout: SLOW }, async () => {
    const dir = mkdtempSync(path.join(tmpdir(), "bench-system-"));
    const child = await holder(fakeConhost(dir));
    try {
      const s = await sample(child.pid!);
      expect(s.java).toEqual({ privateMB: 0, cpuMs: 0 });
    } finally {
      kill(child);
      rmSync(dir, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
    }
  });
});

describe("startSampler", () => {
  it("reports the peak memory seen from one long-lived loop", { timeout: SLOW }, async () => {
    const child = await holder();
    try {
      const peaks = await startSampler(child.pid!, 100).stop();
      expect(peaks.appPeakMB).toBeGreaterThanOrEqual(190);
      expect(peaks.javaPeakMB).toBe(0);
    } finally {
      kill(child);
    }
  });
});

describe("waitForQuiet", () => {
  it("returns the first reading under the limit", async () => {
    const quiet = await waitForQuiet({ read: fake([80, 80, 20]), everyMs: 1 });
    expect(quiet.load).toBe(20);
  });

  it("gives up after maxMs with the last reading", async () => {
    const quiet = await waitForQuiet({ read: async () => 90, maxMs: 50, everyMs: 10 });
    expect(quiet.load).toBe(90);
    expect(quiet.waitedMs).toBeGreaterThanOrEqual(50);
  });
});

describe("cpuLoad", () => {
  it("is a percentage", { timeout: SLOW }, async () => {
    const load = await cpuLoad();
    expect(load).toBeGreaterThanOrEqual(0);
    expect(load).toBeLessThanOrEqual(100);
  });
});

describe("minimise", () => {
  it("rejects for a process that has no window", { timeout: SLOW }, async () => {
    const child = await holder();
    try {
      await expect(minimise(child.pid!)).rejects.toThrow(/no window/);
    } finally {
      kill(child);
    }
  });
});

describe("folderMB", () => {
  it("sizes a folder and its subfolders in MB of 10^6 bytes, to one decimal", () => {
    const dir = mkdtempSync(path.join(tmpdir(), "bench-system-"));
    try {
      mkdirSync(path.join(dir, "sub"));
      writeFileSync(path.join(dir, "a.bin"), Buffer.alloc(1024 * 1024));
      writeFileSync(path.join(dir, "sub", "b.bin"), Buffer.alloc(1024 * 1024));
      expect(folderMB(dir)).toBe(2.1);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
