// Starts and drives the GPUI prototype for measure-windows.mjs: the counterpart of bench/app.mjs `startApp` for an
// app that has no CDP. With GRACE_PROBE set the prototype prints `PROBE <name> <unix ms>` lines on stderr (first_frame,
// tree_shown, run_started, run_finished) and reads `run`, `frames-start`, `frames-stop` and `quit` on stdin (see
// ../../src/main.rs, `probe` and the end of `main`). Nothing here needs Windows except the tree kill, so the driver can
// be tried on Linux against a Linux build.
import { execFile, spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);
/** How long `stop` waits for the process to be gone. */
const GONE_LIMIT_MS = 10_000;

const alive = (/** @type {number} */ pid) => {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return /** @type {any} */ (e).code === "EPERM";
  }
};

/**
 * Kills `pid` and everything it started (the Karate JVM) and waits until `pid` is gone. `taskkill /T` walks the tree
 * from the root, so it must run while the root is alive: it is the first thing `stop` does.
 * @param {number} pid
 */
export async function killTree(pid) {
  if (process.platform === "win32")
    await execFileAsync("taskkill", ["/F", "/T", "/PID", String(pid)], { windowsHide: true }).catch(() => {});
  else {
    await execFileAsync("pkill", ["-KILL", "-P", String(pid)]).catch(() => {});
    try {
      process.kill(pid, "SIGKILL");
    } catch {
      // gone
    }
  }
  const until = Date.now() + GONE_LIMIT_MS;
  while (alive(pid)) {
    if (Date.now() > until) throw new Error(`process ${pid} is still running after the kill`);
    await sleep(50);
  }
}

/** Removes `dir`, retrying for 10 s: Windows keeps the files of a process just killed locked for a moment. */
async function removeDir(/** @type {string} */ dir) {
  for (let tries = 0; ; tries++) {
    try {
      rmSync(dir, { recursive: true, force: true });
      return;
    } catch (e) {
      if (tries === 40) return console.warn(`could not remove ${dir}: ${/** @type {Error} */ (e).message}`);
      await sleep(250);
    }
  }
}

/**
 * @typedef {{
 *   pid: number, t0: number, dataDir: string,
 *   times(): Promise<{ paintMs: number, treeMs: number }>,
 *   waitTree(): Promise<void>,
 *   startRun(): Promise<void>,
 *   waitRunEnd(limitMs?: number): Promise<void>,
 *   framesStart(): void,
 *   framesStop(): Promise<number[]>,
 *   stderrTail(): string,
 *   stop(opts?: { keepData?: boolean }): Promise<void>,
 * }} GpuiApp `t0` is the clock (ms since the epoch) just before the spawn, as in bench/app.mjs. `framesStop` gives the gaps (ms) between the frames that laid out the tree since `framesStart`.
 */

/**
 * Starts the prototype on `project`. Rejects when the exe cannot be started.
 * @param {string} exe the grace-gpui exe
 * @param {{ project: string, java: string, jar: string, dataDir?: string, onSpawn?: (pid: number) => void }} opts
 * @returns {Promise<GpuiApp>}
 */
export async function startGpui(exe, { project, java, jar, dataDir, onSpawn }) {
  const dir = dataDir ?? mkdtempSync(path.join(tmpdir(), "grace-gpui-bench-"));
  mkdirSync(dir, { recursive: true });
  const env = { ...process.env, GRACE_PROBE: "1", GRACE_USER_DATA: dir, GRACE_JAVA: java, GRACE_KARATE_JAR: jar };
  // No windowsHide: it would hide the window the app opens as well as a console.
  const t0 = Date.now();
  const child = spawn(exe, [project], { env, stdio: ["pipe", "ignore", "pipe"] });
  if (child.pid !== undefined) onSpawn?.(child.pid);
  /** @type {Error | undefined} */
  let childError;
  child.on("error", (e) => (childError = e));
  child.stdin.on("error", () => {}); // a write to a dead app is not the bench's failure
  const exited = () =>
    childError ?? (child.exitCode !== null ? new Error(`${exe} exited (${child.exitCode})`) : null);

  /** @type {Map<string, number>} */
  const probes = new Map();
  /** @type {Map<string, () => void>} */
  const waiting = new Map();
  /** @type {string[]} */
  const tail = [];
  let frames = /** @type {number[] | null} */ (null);
  let buf = "";
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (/** @type {string} */ d) => {
    buf += d;
    let i;
    while ((i = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, i).replace(/\r$/, "");
      buf = buf.slice(i + 1);
      tail.push(line);
      if (tail.length > 20) tail.shift();
      // "PROBE frames <count> <gap>,<gap>,..." has no clock; every other probe is "PROBE <name> <unix ms>".
      const m = /^PROBE (\S+) (\d+)(?: (\S*))?$/.exec(line);
      if (!m) continue;
      if (m[1] === "frames") frames = m[3] ? m[3].split(",").map(Number) : [];
      else probes.set(m[1], Number(m[2]));
      waiting.get(m[1])?.();
    }
  });

  /** The time of probe `name`, waiting up to `ms` for it. */
  const until = (/** @type {string} */ name, /** @type {number} */ ms = 120_000) => {
    if (name === "frames" ? frames !== null : probes.has(name))
      return Promise.resolve(name === "frames" ? 0 : /** @type {number} */ (probes.get(name)));
    return new Promise((resolve, reject) => {
      const poll = setInterval(() => {
        const err = exited();
        if (err) fail(err);
      }, 250);
      const timer = setTimeout(
        () => fail(new Error(`no PROBE ${name} in ${ms / 1000} s; the app said:\n${tail.join("\n")}`)),
        ms,
      );
      const fail = (/** @type {Error} */ e) => {
        clearTimeout(timer);
        clearInterval(poll);
        waiting.delete(name);
        reject(e);
      };
      waiting.set(name, () => {
        clearTimeout(timer);
        clearInterval(poll);
        waiting.delete(name);
        resolve(name === "frames" ? 0 : /** @type {number} */ (probes.get(name)));
      });
    });
  };

  /** @type {Promise<void> | null} */
  let stopping = null;
  // Once only: a second call would kill whatever process has taken the pid since.
  const stop = ({ keepData = false } = {}) =>
    (stopping ??= (async () => {
      try {
        if (child.pid) await killTree(child.pid);
      } finally {
        if (!keepData) await removeDir(dir);
      }
    })());

  if (child.pid === undefined) {
    await sleep(0); // the error event comes on the next tick
    await stop({ keepData: dataDir !== undefined }).catch(() => {});
    throw childError ?? new Error(`could not start ${exe}`);
  }
  const pid = child.pid;
  return {
    pid,
    t0,
    dataDir: dir,
    async times() {
      const paint = await until("first_frame");
      const tree = await until("tree_shown");
      return { paintMs: paint - t0, treeMs: tree - t0 };
    },
    async waitTree() {
      await until("tree_shown");
    },
    async startRun() {
      probes.delete("run_finished");
      probes.delete("run_started");
      child.stdin.write("run\n");
      await until("run_started", 60_000);
    },
    async waitRunEnd(limitMs = 30 * 60_000) {
      await until("run_finished", limitMs);
    },
    framesStart() {
      child.stdin.write("frames-start\n");
    },
    async framesStop() {
      frames = null;
      child.stdin.write("frames-stop\n");
      await until("frames", 10_000);
      return frames ?? [];
    },
    stderrTail: () => tail.join("\n"),
    stop,
  };
}
