// Starts GPUI Grace (ADR 0031) the way the bench measures it, without Playwright (migration guide R7.2): its own data
// folder, and the probe on, so it prints `PROBE <name> <unix ms>` lines on stderr (crates/grace/src/probe.rs). The
// counterpart of app.mjs `startApp`, from the GPUI prototype's measure/windows/gpui-app.mjs.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { killTree, removeDir } from "./app.mjs";

/** How long a start may take until the first frame. */
const START_LIMIT_MS = 60_000;

/**
 * True for a GPUI build: Electron Grace keeps its app in `resources/app.asar` beside the exe, GPUI Grace has none.
 * @param {string} exe
 */
export const isGpui = (exe) =>
  !existsSync(path.join(path.dirname(path.resolve(exe)), "resources", "app.asar"));

/**
 * One probe line, or null for any other line on stderr.
 * @param {string} line
 * @returns {{ name: string, ms: number } | null}
 */
export function parseProbe(line) {
  const m = /^PROBE (\S+) (\d+)$/.exec(line.replace(/\r$/, ""));
  return m ? { name: m[1], ms: Number(m[2]) } : null;
}

/**
 * Starts GPUI Grace and waits for its first frame. Rejects when it cannot start, dies first or draws nothing in 60 s;
 * whatever it started is stopped then. The rest is as app.mjs `startApp`: `dataDir` is a fresh temp folder unless
 * given, and `onSpawn` gets the pid as soon as the process exists. `paintMs` is the first frame, from the spawn.
 * @param {string} exe
 * @param {{ dataDir?: string, onSpawn?: (pid: number) => void }} opts
 * @returns {Promise<import("./app.mjs").App & { paintMs: number }>}
 */
export async function startGpui(exe, { dataDir, onSpawn } = {}) {
  const dir = dataDir ?? mkdtempSync(path.join(tmpdir(), "grace-bench-"));
  mkdirSync(dir, { recursive: true });
  const t0 = Date.now();
  // No windowsHide: it would hide the app's window as well as a console.
  const child = spawn(exe, [], {
    env: { ...process.env, GRACE_USER_DATA: dir, GRACE_PROBE: "1" },
    stdio: ["ignore", "ignore", "pipe"],
  });
  if (child.pid !== undefined) onSpawn?.(child.pid);
  /** @type {Promise<void> | null} */
  let stopping = null;
  // Once only: a second call would kill whatever process has taken the pid since.
  const stop = ({ keepData = false } = {}) =>
    (stopping ??= (async () => {
      try {
        if (child.pid) await killTree(child.pid, t0);
      } finally {
        if (!keepData) await removeDir(dir);
      }
    })());

  /** @type {string[]} */
  const tail = [];
  try {
    const firstFrame = await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () =>
          reject(
            new Error(
              `no first frame in ${START_LIMIT_MS / 1000} s; the app said:\n${tail.join("\n")}`,
            ),
          ),
        START_LIMIT_MS,
      );
      const done = (/** @type {() => void} */ settle) => {
        clearTimeout(timer);
        settle();
      };
      child.on("error", (e) => done(() => reject(e)));
      child.on("exit", (code) =>
        done(() =>
          reject(new Error(`${exe} exited (${code}) before its first frame:\n${tail.join("\n")}`)),
        ),
      );
      let buf = "";
      child.stderr.setEncoding("utf8").on("data", (/** @type {string} */ d) => {
        buf += d;
        for (let i; (i = buf.indexOf("\n")) >= 0; buf = buf.slice(i + 1)) {
          const line = buf.slice(0, i);
          tail.push(line);
          if (tail.length > 20) tail.shift();
          const probe = parseProbe(line);
          if (probe?.name === "first_frame") done(() => resolve(probe.ms));
        }
      });
    });
    // Keep reading, so a full pipe never blocks the app.
    child.stderr.resume();
    return {
      pid: /** @type {number} */ (child.pid),
      t0,
      dataDir: dir,
      page: null,
      cdp: null,
      paintMs: /** @type {number} */ (firstFrame) - t0,
      stop,
    };
  } catch (e) {
    await stop({ keepData: dataDir !== undefined }).catch(() => {});
    throw e;
  }
}
