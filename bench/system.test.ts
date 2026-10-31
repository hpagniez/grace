import { execFileSync, spawn, type ChildProcess } from "node:child_process";
import { once } from "node:events";
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { describe, expect, it } from "vitest";
import {
  cpuLoad,
  cpuMeter,
  folderMB,
  minimise,
  sample,
  screens,
  splitTree,
  startSampler,
  waitForQuiet,
  windowSpot,
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

/** The id of a process that has just exited. */
async function deadPid() {
  const proc = spawn(process.execPath, ["-e", "0"], { stdio: "ignore" });
  await once(proc, "exit");
  return proc.pid!;
}

/** Ends the PowerShell that `startSampler(rootPid)` runs: the one whose script walks the tree from `rootPid`. */
function killSampler(rootPid: number) {
  execFileSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" -Property ProcessId, CommandLine |
  Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -like '*ids.Add(${rootPid})*' } |
  ForEach-Object { Stop-Process -Id $_.ProcessId -Force }`,
    ],
    { stdio: "ignore" },
  );
}

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

describe("cpuMeter", () => {
  const TICKS = 1e4; // one ms in 100 ns ticks
  const proc = (id: number, parent: number, name: string, ms: number) => ({
    id,
    parent,
    name,
    bytes: 0,
    cpu: ms * TICKS,
  });

  it("keeps what a java that has exited used, which the tree's own total loses", () => {
    const meter = cpuMeter();
    meter.add([proc(1, 0, "Grace.exe", 100), proc(2, 1, "java.exe", 50)]);
    meter.add([
      proc(1, 0, "Grace.exe", 300),
      proc(2, 1, "java.exe", 200),
      proc(3, 1, "java.exe", 80),
      proc(4, 3, "conhost.exe", 10),
    ]);
    meter.add([proc(1, 0, "Grace.exe", 400)]); // both javas and the conhost are gone
    expect(meter.total()).toEqual({ appCpuMs: 300, javaCpuMs: 150 + 80 + 10 });
  });

  it("counts a process that appears later from zero, and one there at the start from its first reading", () => {
    const meter = cpuMeter();
    meter.add([proc(1, 0, "Grace.exe", 1000)]);
    meter.add([proc(1, 0, "Grace.exe", 1100), proc(5, 1, "Grace.exe", 40)]);
    expect(meter.total()).toEqual({ appCpuMs: 140, javaCpuMs: 0 });
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

  it("rejects for a process that is not running, not zeros", { timeout: SLOW }, async () => {
    await expect(sample(await deadPid())).rejects.toThrow(/is not running/);
  });

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

  it("rejects for a process that is not running, not a peak of 0", { timeout: SLOW }, async () => {
    await expect(startSampler(await deadPid(), 100).stop()).rejects.toThrow(/is not running/);
  });

  it(
    "is ready once its first sample is in, and reports the cpu seen",
    { timeout: SLOW },
    async () => {
      const child = await holder();
      try {
        const sampler = startSampler(child.pid!, 100);
        expect(sampler.pid).toBeGreaterThan(0); // its PowerShell, for a caller that must kill it
        await sampler.ready;
        const peaks = await sampler.stop();
        expect(peaks.appCpuMs).toBeGreaterThanOrEqual(0);
        expect(peaks.javaCpuMs).toBe(0);
      } finally {
        kill(child);
      }
    },
  );

  it("is never ready for a process that is not running", { timeout: SLOW }, async () => {
    const sampler = startSampler(await deadPid(), 100);
    await expect(sampler.ready).rejects.toThrow(/is not running/);
    await sampler.stop().catch(() => {});
  });

  it(
    "rejects when its PowerShell died, not an under-reported peak",
    { timeout: SLOW },
    async () => {
      const child = await holder();
      try {
        const sampler = startSampler(child.pid!, 100);
        await sleep(3000); // usually long enough for a few samples; if not, the sampler dies before its first
        killSampler(child.pid!);
        await sleep(500); // for Node to see the exit
        await expect(sampler.stop()).rejects.toThrow(/the sampler died/);
      } finally {
        kill(child);
      }
    },
  );
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

describe("screens", () => {
  it("lists one main screen with a work area", { timeout: SLOW }, async () => {
    const all = await screens();
    expect(all.filter((s) => s.primary)).toHaveLength(1);
    for (const s of all) {
      expect(s.name).toEqual(expect.any(String));
      expect(s.width).toBeGreaterThan(0);
      expect(s.height).toBeGreaterThan(0);
    }
  });
});

describe("windowSpot", () => {
  const screen = (name: string, primary: boolean, x: number, width = 2560, height = 1392) => ({
    name,
    primary,
    x,
    y: 0,
    width,
    height,
  });

  it("keeps the window at 40, 40 when the main screen is the only one", () => {
    expect(windowSpot([screen("\\\\.\\DISPLAY1", true, 0)])).toEqual({
      x: 40,
      y: 40,
      screen: null,
    });
  });

  it("centres it on the first other screen by device name", () => {
    const main = screen("\\\\.\\DISPLAY2", true, 0);
    const third = screen("\\\\.\\DISPLAY3", false, -1920, 1920, 1040);
    const first = screen("\\\\.\\DISPLAY1", false, 2560);
    expect(windowSpot([main, third, first])).toEqual({ x: 3140, y: 246, screen: first });
    expect(windowSpot([main, third])).toEqual({ x: -1660, y: 70, screen: third });
  });

  it("puts it at the top left of a work area smaller than the window", () => {
    const small = screen("\\\\.\\DISPLAY2", false, 2560, 1366, 728);
    expect(windowSpot([screen("\\\\.\\DISPLAY1", true, 0), small])).toMatchObject({
      x: 2560,
      y: 0,
    });
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
