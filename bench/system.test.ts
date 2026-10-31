import { spawn, type ChildProcess } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { cpuLoad, folderMB, minimise, sample, startSampler, waitForQuiet } from "./system.mjs";

// PowerShell starts slowly on a loaded CI machine
const SLOW = 60_000;

// Touches every page of 200 MiB, burns one second of its own CPU time (not wall time, which a busy
// machine stretches), tells the test it is ready, then holds both until killed.
const HOLDER = `
const buf = Buffer.alloc(200 * 1024 * 1024, 1);
const used = () => { const u = process.cpuUsage(); return u.user + u.system; };
while (used() < 1_000_000);
console.log("ready");
setInterval(() => { buf[0] = (buf[0] + 1) % 256; }, 1000);
`;

/** A Node child holding 200 MiB and 1 s of CPU time, once it says so. The caller kills it. */
async function holder(): Promise<ChildProcess> {
  const child = spawn(process.execPath, ["-e", HOLDER], { stdio: ["ignore", "pipe", "inherit"] });
  await new Promise<void>((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", () => reject(new Error("the holder exited before it was ready")));
    child.stdout!.once("data", () => resolve());
  });
  return child;
}

/** A reader that gives `values` one by one, then the last again. */
const fake = (values: number[]) => {
  let i = 0;
  return async () => values[Math.min(i++, values.length - 1)];
};

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
        child.kill();
      }
    },
  );
});

describe("startSampler", () => {
  it("reports the peak memory seen from one long-lived loop", { timeout: SLOW }, async () => {
    const child = await holder();
    try {
      const peaks = await startSampler(child.pid!, 100).stop();
      expect(peaks.appPeakMB).toBeGreaterThanOrEqual(190);
      expect(peaks.javaPeakMB).toBe(0);
    } finally {
      child.kill();
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
      child.kill();
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
