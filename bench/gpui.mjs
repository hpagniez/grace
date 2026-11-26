// Starts GPUI Grace (ADR 0031) the way the bench measures it, without Playwright (migration guide R7.2): its own data
// folder, and the probe on, so it prints `PROBE <name> <unix ms>` lines on stderr (crates/grace/src/probe.rs). The
// counterpart of app.mjs `startApp`, from the GPUI prototype's measure/windows/gpui-app.mjs.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { killTree, removeDir } from "./app.mjs";

/** How long a start may take until the first frame. */
const START_LIMIT_MS = 60_000;
/** How long it may take with watch mode on, which comes once the project is open, its dry run included. */
const WATCH_LIMIT_MS = 180_000;

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
 * Writes the `settings.json` of a start with watch mode on: `project` as the last project, and watch on for it, keyed
 * by the same string, as `grace_core::settings` reads it.
 * @param {string} dataDir
 * @param {string} project an absolute path
 */
export function seedWatch(dataDir, project) {
  const settings = { lastProject: project, projects: { [project]: { watch: true } } };
  writeFileSync(path.join(dataDir, "settings.json"), JSON.stringify(settings));
}

/**
 * Takes the probes of a start one at a time: null until Grace is ready to be measured, then the first frame's time.
 * Ready is the first frame drawn, and with `watch` also the project open and watch mode on (`watch_on`).
 * @param {boolean} watch
 * @returns {(probe: { name: string, ms: number }) => number | null}
 */
export function readiness(watch) {
  /** @type {number | null} */
  let frame = null;
  let watching = !watch;
  return ({ name, ms }) => {
    if (name === "first_frame") frame ??= ms;
    if (name === "watch_on") watching = true;
    return watching ? frame : null;
  };
}

/**
 * Starts GPUI Grace and waits for its first frame, and with `watch`, a project's folder, for that project open with
 * watch mode on. Rejects when it cannot start, dies first, or is not there in 60 s (180 s with `watch`); whatever it
 * started is stopped then. The rest is as app.mjs `startApp`: `dataDir` is a fresh temp folder unless
 * given, and `onSpawn` gets the pid as soon as the process exists. `paintMs` is the first frame, from the spawn.
 * @param {string} exe
 * @param {{ dataDir?: string, watch?: string, onSpawn?: (pid: number) => void }} opts
 * @returns {Promise<import("./app.mjs").App & { paintMs: number }>}
 */
export async function startGpui(exe, { dataDir, watch, onSpawn } = {}) {
  const dir = dataDir ?? mkdtempSync(path.join(tmpdir(), "grace-bench-"));
  mkdirSync(dir, { recursive: true });
  if (watch !== undefined) seedWatch(dir, watch);
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
  const limit = watch === undefined ? START_LIMIT_MS : WATCH_LIMIT_MS;
  const awaited = watch === undefined ? "first frame" : "first frame with watch mode on";
  try {
    const firstFrame = await new Promise((resolve, reject) => {
      const timer = setTimeout(
        () =>
          reject(
            new Error(`no ${awaited} in ${limit / 1000} s; the app said:\n${tail.join("\n")}`),
          ),
        limit,
      );
      const ready = readiness(watch !== undefined);
      const done = (/** @type {() => void} */ settle) => {
        clearTimeout(timer);
        settle();
      };
      child.on("error", (e) => done(() => reject(e)));
      child.on("exit", (code) =>
        done(() =>
          reject(new Error(`${exe} exited (${code}) before its ${awaited}:\n${tail.join("\n")}`)),
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
          const frame = probe && ready(probe);
          if (frame != null) done(() => resolve(frame));
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
