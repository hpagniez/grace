// Starts a packaged Grace exe the way the bench measures it: its own data folder, the last project reopened, no update
// check, and, when asked, a Chromium debugging port to drive the page over CDP. The exe's `EnableNodeCliInspectArguments`
// fuse is off, so Playwright's `_electron` cannot launch it (AGENTS.md, Stack); `connectOverCDP` can. Ported from the
// Tauri spike (docs/research/2026-10-29-tauri-spike/measure/measure.mjs).
import { execFile, spawn } from "node:child_process";
import { once } from "node:events";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import net from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";
import { chromium } from "@playwright/test";
import { TREE } from "./probes.mjs";

/** @typedef {import("@playwright/test").Page} Page */
/** @typedef {import("@playwright/test").CDPSession} CDPSession */
/**
 * A running Grace. `page` and `cdp` are null unless it was started with `cdp: true`. `t0` is the clock (ms since the
 * epoch) just before the process was spawned.
 * @typedef {{
 *   pid: number, t0: number, dataDir: string, page: Page | null, cdp: CDPSession | null,
 *   stop(opts?: { keepData?: boolean }): Promise<void>,
 * }} App
 */

const execFileAsync = promisify(execFile);
/** How long a start may take until the page can be driven, and until the tree shows. */
const START_LIMIT_MS = 60_000;
/** How long `stop` waits for the process tree to be gone. */
const GONE_LIMIT_MS = 10_000;
/**
 * Chromium slows frames and timers of a window that is in the background or covered by another, and the probes read frames.
 * Only a driven app (cdp) gets these: idle measures must see the app as a user's machine runs it.
 */
const UNTHROTTLED = [
  "--disable-renderer-backgrounding",
  "--disable-backgrounding-occluded-windows",
  "--disable-features=CalculateNativeWinOcclusion",
];

/** A port nothing listens on, so a leftover app of an earlier run is never the one the bench connects to. */
const freePort = () =>
  new Promise((resolve, reject) => {
    const server = net.createServer().once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = /** @type {net.AddressInfo} */ (server.address());
      server.close(() => resolve(port));
    });
  });

/**
 * Pids of `pid` and everything under it, from one PowerShell call (the parent links vanish once the root is dead).
 * @param {number} pid
 * @returns {Promise<number[]>}
 */
async function treePids(pid) {
  const { stdout } = await execFileAsync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `
$all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId
$ids = [System.Collections.Generic.HashSet[int]]::new(); [void]$ids.Add(${pid})
do { $n = $ids.Count; foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } } } while ($ids.Count -ne $n)
$ids -join ','`,
    ],
    { encoding: "utf8", windowsHide: true },
  );
  return stdout.trim().split(",").map(Number);
}

/**
 * Signal 0 only asks whether the process exists; EPERM means it does, and is not ours to touch.
 * @param {number} pid
 */
function alive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return /** @type {any} */ (e).code === "EPERM";
  }
}

/**
 * Kills `pid` and its whole tree, then waits until none of the tree is left. `child.kill()` would leave Electron's
 * helpers and Java running. Rejects, naming them, when a process outlives the wait.
 * @param {number} pid
 */
async function killTree(pid) {
  // The listed pids are killed one by one, not just the root with /T: once the root is dead (the app crashed), taskkill
  // cannot find it and so never reaches its children, which are still listed by their parent's pid. /T stays for what
  // the root started after the listing. If the listing fails, the root alone is killed.
  const tree = await treePids(pid).catch(() => [pid]);
  // A failure here means a process already gone; the wait below is what proves the rest is.
  await execFileAsync("taskkill", ["/F", "/T", ...tree.flatMap((p) => ["/PID", String(p)])], {
    windowsHide: true,
  }).catch(() => {});
  const until = Date.now() + GONE_LIMIT_MS;
  for (;;) {
    const left = tree.filter(alive);
    if (!left.length) return;
    if (Date.now() > until) throw new Error(`still running after taskkill: ${left.join(", ")}`);
    await sleep(50);
  }
}

/**
 * Removes `dir`, retrying for 10 s: Windows keeps the files of a process just killed locked for a moment, and `rmSync`'s
 * own retries do not wait on that. A folder that will not go is named and left, not an error: the measures are in.
 * @param {string} dir
 */
async function removeDir(dir) {
  for (let tries = 0; ; tries++) {
    try {
      rmSync(dir, { recursive: true, force: true });
      return;
    } catch (e) {
      if (tries === 40)
        return console.warn(`could not remove ${dir}: ${/** @type {Error} */ (e).message}`);
      await sleep(250);
    }
  }
}

/**
 * Starts the exe on `opts.project`. With `cdp` it connects to the page, and rejects when the exe cannot be driven in 60 s
 * or dies first; whatever it started is stopped then. `dataDir` is a fresh temp folder unless given (to start again on the
 * same profile); the window is placed at 1400 x 900 so every run draws the same page.
 * @param {string} exe
 * @param {{ project: string, dataDir?: string, cdp: boolean, port?: number }} opts
 * @returns {Promise<App>}
 */
export async function startApp(exe, { project, dataDir, cdp, port }) {
  const dir = dataDir ?? mkdtempSync(path.join(tmpdir(), "grace-bench-"));
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    path.join(dir, "settings.json"),
    JSON.stringify({
      lastProject: project,
      updates: { checkAtStartup: false },
      window: { x: 40, y: 40, width: 1400, height: 900, maximized: false },
    }),
  );
  const debugPort = cdp ? (port ?? (await freePort())) : null;
  const t0 = Date.now();
  const child = spawn(
    exe,
    debugPort ? [`--remote-debugging-port=${debugPort}`, ...UNTHROTTLED] : [],
    {
      env: { ...process.env, GRACE_USER_DATA: dir },
      stdio: "ignore",
    },
  );
  /** @type {Error | undefined} */
  let childError;
  child.on("error", (e) => (childError = e));
  const exited = () =>
    childError ?? (child.exitCode !== null ? new Error(`${exe} exited (${child.exitCode})`) : null);

  /** @type {import("@playwright/test").Browser | null} */
  let browser = null;
  /** @type {Promise<void> | null} */
  let stopping = null;
  // Once only: a second call would kill whatever process has taken the pid since.
  const stop = ({ keepData = false } = {}) =>
    (stopping ??= (async () => {
      try {
        if (child.pid) await killTree(child.pid);
        await browser?.close().catch(() => {});
      } finally {
        if (!keepData) await removeDir(dir);
      }
    })());

  try {
    // No pid: the exe could not be started, and its error comes on the next tick.
    if (child.pid === undefined) throw (await once(child, "error"))[0];
    let page = null;
    let session = null;
    if (debugPort) {
      for (;;) {
        const err = exited();
        if (err) throw err;
        try {
          browser = await chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`, {
            timeout: 10_000,
          });
          break;
        } catch (e) {
          if (Date.now() - t0 > START_LIMIT_MS)
            throw new Error(
              `could not connect in ${START_LIMIT_MS / 1000} s: ${/** @type {Error} */ (e).message}`,
            );
          await sleep(25);
        }
      }
      while (!(page = browser.contexts()[0]?.pages()[0])) {
        const err = exited();
        if (err) throw err;
        if (Date.now() - t0 > START_LIMIT_MS)
          throw new Error(`no page in ${START_LIMIT_MS / 1000} s`);
        await sleep(20);
      }
      session = await page.context().newCDPSession(page);
    }
    return { pid: /** @type {number} */ (child.pid), t0, dataDir: dir, page, cdp: session, stop };
  } catch (e) {
    // The caller never gets `stop`, so a failed start cleans up after itself, and keeps a folder it did not make.
    await stop({ keepData: dataDir !== undefined }).catch(() => {});
    throw e;
  }
}

/**
 * First paint and tree shown, in ms from `app.t0` (the process spawn): the spike's `timingRun`. The tree is found by
 * polling the page, so it reads up to one poll (20 ms) late. Rejects when the tree does not show in 60 s.
 * @param {App} app
 * @returns {Promise<{ paintMs: number, treeMs: number }>}
 */
export async function startTimes(app) {
  const { page } = app;
  if (!page) throw new TypeError("startTimes needs an app started with cdp");
  let treeAt = null;
  while (treeAt === null) {
    if (Date.now() - app.t0 > START_LIMIT_MS)
      throw new Error(`the tree did not show in ${START_LIMIT_MS / 1000} s`);
    // The page may be mid-navigation at first: no context to ask yet reads as "not there".
    const found = await page.evaluate((s) => !!document.querySelector(s), TREE).catch(() => false);
    if (found) treeAt = Date.now();
    else await sleep(20);
  }
  const paintAt = await page.evaluate(() => {
    const e = performance.getEntriesByName("first-contentful-paint")[0];
    return e ? performance.timeOrigin + e.startTime : null;
  });
  if (paintAt === null) throw new Error("the page has no first-contentful-paint");
  return { paintMs: Math.round(paintAt - app.t0), treeMs: treeAt - app.t0 };
}
