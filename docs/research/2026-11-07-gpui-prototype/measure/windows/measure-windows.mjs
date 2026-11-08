// Measures Grace (Electron) and the GPUI prototype the same way on Windows, in alternating rounds, with Grace's own
// bench helpers: the Windows sampler of bench/system.mjs (private working set and CPU time of the app's process tree,
// Java apart), bench/app.mjs to start and drive the packaged Grace over CDP, and the big-project generator of
// e2e/big-project.ts. It is the Windows port of measure/measure.mjs and measure/fluid.mjs for #267, and its measures
// carry the bench's ids (idle.memory, idle.cpu.shown, run.memory.app, ...) where the bench has one.
//
//   node --no-warnings measure-windows.mjs --grace <Grace checkout> --gpui <grace-gpui.exe> --java <java.exe> --jar <karate.jar>
//        [--grace-exe <Grace.exe>] [--grace-zip <zip>] [--gpui-floor <floor.exe>] [--electron <electron.exe>]
//        [--rounds 5] [--floor-rounds 3] [--settle-s 20] [--idle-s 60] [--apps grace,gpui]
//        [--skip-large] [--skip-fluid] [--skip-floor] [--keep-gpui-window] [--label gpu] [--out <dir>]
//
// build-and-measure.ps1 builds everything and calls this. Needs Windows (the sampler is PowerShell and CIM) and a
// Grace checkout with `npm install` done. Written on Linux and never run on Windows: see README.md.
//
// Each round, for one app: two starts (a fresh profile, then the same profile again; the prototype has no profile),
// idle (settle, then CPU over the idle window, shown and minimised), Run all on the 2,000-test project, Run all on the
// 10,000-test project, and the wheel probe over the tree, idle and during Run all. A phase that fails is recorded
// and the rest goes on, so an hour of rounds is not lost to one bad step.
import { execFile, execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parseArgs, promisify } from "node:util";
import { deflateRawSync } from "node:zlib";
import { killTree, startGpui } from "./gpui-app.mjs";
import { markdown, stats as summarise } from "./report.mjs";

const here = path.dirname(fileURLToPath(import.meta.url));
const execFileAsync = promisify(execFile);
const log = (/** @type {string} */ msg) => console.error(`[measure ${new Date().toTimeString().slice(0, 8)}] ${msg}`);
const round1 = (/** @type {number} */ n) => Math.round(n * 10) / 10;

/** Exits with a message and code 2: a usage error is not a failure to measure. */
function usage(/** @type {string} */ message) {
  console.error(message);
  process.exit(2);
}

let args;
try {
  ({ values: args } = parseArgs({
    options: {
      grace: { type: "string" },
      "grace-exe": { type: "string" },
      "grace-zip": { type: "string" },
      gpui: { type: "string" },
      "gpui-floor": { type: "string" },
      electron: { type: "string" },
      java: { type: "string" },
      jar: { type: "string" },
      rounds: { type: "string", default: "5" },
      "floor-rounds": { type: "string", default: "3" },
      "settle-s": { type: "string", default: "20" },
      "idle-s": { type: "string", default: "60" },
      apps: { type: "string", default: "grace,gpui" },
      "skip-large": { type: "boolean" },
      "skip-fluid": { type: "boolean" },
      "skip-floor": { type: "boolean" },
      "keep-gpui-window": { type: "boolean" },
      label: { type: "string", default: "gpu" },
      out: { type: "string" },
    },
  }));
} catch (e) {
  usage(/** @type {Error} */ (e).message);
}
if (process.platform !== "win32") usage("measure-windows.mjs measures with PowerShell and CIM: it runs on Windows only.");
const need = (/** @type {string} */ name) => {
  const v = /** @type {string | undefined} */ (/** @type {any} */ (args)[name]);
  if (!v) usage(`--${name} is required`);
  return /** @type {string} */ (v);
};
const whole = (/** @type {string} */ name, /** @type {number} */ min) => {
  const n = Number(/** @type {any} */ (args)[name]);
  if (!Number.isInteger(n) || n < min) usage(`--${name} must be a whole number of ${min} or more`);
  return n;
};

const GRACE = path.resolve(need("grace"));
const GRACE_EXE = path.resolve(args["grace-exe"] ?? path.join(GRACE, "out", "Grace-win32-x64", "Grace.exe"));
const GPUI_EXE = path.resolve(need("gpui"));
const JAVA = path.resolve(need("java"));
const JAR = path.resolve(need("jar"));
const APPS = /** @type {("grace" | "gpui")[]} */ (args.apps.split(",").map((s) => s.trim()).filter(Boolean));
for (const a of APPS) if (a !== "grace" && a !== "gpui") usage(`--apps takes grace and gpui, not ${a}`);
const ROUNDS = whole("rounds", 1);
const FLOOR_ROUNDS = whole("floor-rounds", 1);
const SETTLE_MS = whole("settle-s", 0) * 1000;
const IDLE_MS = whole("idle-s", 1) * 1000;
const LABEL = String(args.label).replace(/[^\w.-]/g, "-");
const OUT = path.resolve(args.out ?? path.join(here, "..", "..", "results"));
const MOVE_GPUI = !args["keep-gpui-window"];
for (const [what, file] of [["Grace's exe", GRACE_EXE], ["the prototype's exe", GPUI_EXE], ["java.exe", JAVA], ["karate.jar", JAR]])
  if (!existsSync(file) && (what !== "Grace's exe" || APPS.includes("grace"))) usage(`${what} not found: ${file}`);

// Grace's own helpers, taken from the checkout so they are the ones the bench is built from.
const fromGrace = (/** @type {string} */ file) => import(pathToFileURL(path.join(GRACE, file)).href);
const { startApp, startTimes } = await fromGrace("bench/app.mjs");
const sys = await fromGrace("bench/system.mjs");
const { order } = await fromGrace("bench/stats.mjs");
const { writeBigProject } = await fromGrace("e2e/big-project.ts");

/** How long the tree may take to hold the project (the dry run of 10,000 tests is slow), as bench.mjs. */
const TREE_LOAD_MS = 120_000;
/** Run all gets 30 minutes; a run that takes longer cannot be measured. */
const RUN_LIMIT_MS = 30 * 60_000;
/** A run that has not started a minute after the click never will. */
const RUN_START_MS = 60_000;
/** The wheel is sent by a fresh PowerShell: it starts at a set time, after its slow setup, and the frame probe opens just before. */
const WHEEL_LEAD_MS = 5_000;
const OUTCOME = '[data-testid="run-outcome"]';

// ---- keeping track of what is running, so a Ctrl+C or a crash leaves nothing behind ----

/** @type {Set<{ pid: number | undefined, stop(...a: any[]): Promise<unknown> }>} */
const live = new Set();
/** @type {Set<number>} */
const starting = new Set();
/** @type {string | null} */
let workDir = null;
const errors = /** @type {string[]} */ ([]);

function killNow(/** @type {(number | undefined)[]} */ pids) {
  const list = pids.filter((p) => p !== undefined);
  if (!list.length) return;
  try {
    execFileSync("taskkill", ["/F", "/T", ...list.flatMap((p) => ["/PID", String(p)])], { stdio: "ignore", windowsHide: true });
  } catch {
    // already gone
  }
}
function removeWork() {
  if (!workDir) return;
  for (let tries = 0; ; tries++) {
    try {
      return rmSync(workDir, { recursive: true, force: true });
    } catch (e) {
      if (tries === 40) return console.error(`could not remove ${workDir}: ${/** @type {Error} */ (e).message}`);
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
    }
  }
}
async function cleanup() {
  killNow([...starting]);
  await Promise.allSettled([...live].map((x) => x.stop().then(() => live.delete(x))));
  removeWork();
}
process.on("exit", () => {
  killNow([...starting, ...[...live].map((x) => x.pid)]);
  removeWork();
});
let dying = false;
function die(/** @type {string} */ why) {
  if (dying) return;
  dying = true;
  console.error(why);
  process.exitCode = 1;
  cleanup().finally(() => process.exit(1));
}
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP", "SIGBREAK"]) process.on(signal, () => die(`stopped by ${signal}`));
for (const event of ["uncaughtException", "unhandledRejection"])
  process.on(event, (e) => die(`failed: ${/** @type {Error} */ (e)?.stack ?? e}`));

// ---- Win32 helpers (window-tools.ps1) and the machine ----

/** Runs window-tools.ps1 and gives the JSON line it prints. */
async function tools(/** @type {string} */ action, /** @type {string[]} */ rest = []) {
  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", path.join(here, "window-tools.ps1"), "-Action", action, ...rest],
      { encoding: "utf8", windowsHide: true, timeout: 120_000 },
    );
    return JSON.parse(stdout.trim().split(/\r?\n/).pop() ?? "null");
  } catch (e) {
    throw new Error(`window-tools ${action} failed: ${/** @type {any} */ (e).stderr?.trim() || /** @type {Error} */ (e).message}`);
  }
}

async function describeMachine() {
  const cpus = os.cpus();
  /** @type {any} */
  let displays = null;
  try {
    displays = await tools("displays");
  } catch (e) {
    log(`could not read the displays: ${/** @type {Error} */ (e).message}`);
  }
  const text = (/** @type {string} */ cmd, /** @type {string[]} */ a) => {
    try {
      return execFileSync(cmd, a, { encoding: "utf8", windowsHide: true }).trim();
    } catch {
      return null;
    }
  };
  return {
    os: os.version(),
    cpu: `${cpus[0].model.trim()} (${cpus.length} threads)`,
    ramGB: Math.round(os.totalmem() / 2 ** 30),
    node: process.version,
    graceCommit: text("git", ["-C", GRACE, "rev-parse", "--short", "HEAD"]),
    power: text("powercfg", ["/getactivescheme"]),
    displays,
  };
}

// ---- starting the two apps ----

let dataCount = 0;
const freshDir = () => path.join(/** @type {string} */ (workDir), `data-${++dataCount}`);
/** Where Grace's windows open: the screen the bench uses (first one that is not the main one), as bench.mjs. */
let windowAt = { x: 40, y: 40 };
/** What the first move of the prototype's window said, for the report. */
let gpuiWindow = MOVE_GPUI ? "not moved yet" : "left where the prototype opens it (0, 0 of the main screen)";

/** @typedef {Awaited<ReturnType<typeof startApp>> | Awaited<ReturnType<typeof startGpui>>} App */

/**
 * @param {"grace" | "gpui"} key
 * @param {string} project
 * @param {{ dataDir?: string, cdp?: boolean }} [o] `cdp` drives Grace over CDP, and gives the prototype its probes (it always has them)
 * @returns {Promise<any>}
 */
async function launch(key, project, { dataDir = freshDir(), cdp = false } = {}) {
  /** @type {number | undefined} */
  let pid;
  try {
    const onSpawn = (/** @type {number} */ p) => starting.add((pid = p));
    const app =
      key === "grace"
        ? await startApp(GRACE_EXE, { project, dataDir, cdp, at: windowAt, onSpawn })
        : await startGpui(GPUI_EXE, { project, java: JAVA, jar: JAR, dataDir, onSpawn });
    live.add(app);
    return app;
  } finally {
    if (pid !== undefined) starting.delete(pid);
  }
}

/** Stops `app`; a failure to stop is reported and fails the run at the end, never hides the error that got us here. */
async function halt(/** @type {any} */ app, /** @type {{ keepData?: boolean }} */ opts = {}) {
  await app.stop(opts).then(
    () => live.delete(app),
    (/** @type {Error} */ e) => {
      errors.push(`could not stop the app: ${e.message}`);
      console.error(`could not stop the app: ${e.message}`);
    },
  );
}

/** Puts the prototype's window where Grace's is (its probe mode opens it at 0, 0 of the main screen). Never throws. */
async function place(/** @type {"grace" | "gpui"} */ key, /** @type {{ pid: number }} */ app) {
  if (key !== "gpui" || !MOVE_GPUI) return;
  try {
    const r = await tools("move", ["-ProcessId", String(app.pid)]);
    if (gpuiWindow === "not moved yet")
      gpuiWindow = r.moved ? `moved to the middle of ${r.screen}, the screen Grace uses` : `left on the main screen (${r.reason})`;
  } catch (e) {
    if (gpuiWindow === "not moved yet") gpuiWindow = `left on the main screen (${/** @type {Error} */ (e).message})`;
    log(`could not move the prototype's window: ${/** @type {Error} */ (e).message}`);
  }
}

// ---- driving Grace over CDP (bench.mjs startRun/waitForOutcome, shorter) ----

const outcomeNow = (/** @type {any} */ page) =>
  page.evaluate((s) => document.querySelector(s)?.textContent?.trim() ?? "", OUTCOME);

/** The tree holds rows and Run all is enabled. */
async function graceReady(/** @type {any} */ page) {
  await page.waitForFunction(
    () => document.querySelectorAll("[data-testid=tree-node]").length > 0 && !document.querySelector("[data-testid=run-all-button]")?.disabled,
    null,
    { timeout: TREE_LOAD_MS, polling: 100 },
  );
}
async function graceStartRun(/** @type {any} */ page) {
  await page.getByTestId("run-all-button").click({ timeout: 30_000 });
  await page.waitForFunction(
    (s) => /^(Running|Finished|Cancelled|Run failed)/.test(document.querySelector(s)?.textContent?.trim() ?? ""),
    OUTCOME,
    { timeout: RUN_START_MS, polling: 100 },
  );
}
async function graceEndRun(/** @type {any} */ page) {
  await page.waitForFunction(
    (s) => /^(Finished|Cancelled|Run failed)/.test(document.querySelector(s)?.textContent?.trim() ?? ""),
    OUTCOME,
    { timeout: RUN_LIMIT_MS, polling: 250 },
  );
  const end = await outcomeNow(page);
  if (!end.startsWith("Finished")) throw new Error(`Run all ended as "${end}", not Finished`);
}

/** Everything the same on both apps: ready, start a run, wait for its end. */
const ready = (/** @type {"grace" | "gpui"} */ key, /** @type {any} */ app) =>
  key === "grace" ? graceReady(app.page) : app.waitTree().then(() => place(key, app));
const startRun = (/** @type {"grace" | "gpui"} */ key, /** @type {any} */ app) =>
  key === "grace" ? graceStartRun(app.page) : app.startRun();
const endRun = (/** @type {"grace" | "gpui"} */ key, /** @type {any} */ app) =>
  key === "grace" ? graceEndRun(app.page) : app.waitRunEnd(RUN_LIMIT_MS);

// ---- phases ----

const cpu = (/** @type {{ app: { cpuMs: number }, java: { cpuMs: number } }} */ s) => s.app.cpuMs + s.java.cpuMs;

/** Number of processes in the tree under `pid` (the bench's Select-Tree, one PowerShell call). */
async function treeSize(/** @type {number} */ pid) {
  const { stdout } = await execFileAsync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `${sys.SELECT_TREE}\n$all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, CreationDate\n(Select-Tree $all ${pid}) -join ','`,
    ],
    { encoding: "utf8", windowsHide: true, timeout: 60_000 },
  );
  return stdout.trim().split(",").filter(Boolean).length;
}

/** One start: first paint and tree shown, in ms from the spawn. */
async function oneStart(/** @type {"grace" | "gpui"} */ key, /** @type {string} */ project, /** @type {string} */ dataDir) {
  const app = await launch(key, project, { dataDir, cdp: true });
  try {
    return key === "grace" ? await startTimes(app) : await app.times();
  } finally {
    await halt(app, { keepData: true });
  }
}

async function startPhase(/** @type {"grace" | "gpui"} */ key, /** @type {{ dir: string }} */ small) {
  const dir = freshDir();
  try {
    const fresh = await oneStart(key, small.dir, dir);
    const reused = await oneStart(key, small.dir, dir);
    return {
      "start.paint.fresh": fresh.paintMs,
      "start.tree.fresh": fresh.treeMs,
      "start.paint.reused": reused.paintMs,
      "start.tree.reused": reused.treeMs,
    };
  } finally {
    rmSync(dir, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
  }
}

/** Idle: memory after the settle, CPU over the idle window shown and, when the window can be minimised, minimised. */
async function idlePhase(/** @type {"grace" | "gpui"} */ key, /** @type {{ dir: string }} */ small) {
  const app = await launch(key, small.dir, { cdp: false });
  try {
    if (key === "gpui") {
      await app.waitTree();
      await place(key, app);
    }
    await sleep(Math.max(0, SETTLE_MS - (Date.now() - app.t0)));
    const first = await sys.sample(app.pid);
    /** @type {Record<string, number | null>} */
    const out = {
      "idle.memory": round1(first.app.privateMB + first.java.privateMB),
      "idle.memory.app": first.app.privateMB,
      "idle.memory.java": first.java.privateMB,
      "idle.procs": await treeSize(app.pid),
    };
    await sleep(IDLE_MS);
    out["idle.cpu.shown"] = cpu(await sys.sample(app.pid)) - cpu(first);
    try {
      await sys.minimise(app.pid);
      await sleep(2000);
      const before = await sys.sample(app.pid);
      await sleep(IDLE_MS);
      out["idle.cpu.minimised"] = cpu(await sys.sample(app.pid)) - cpu(before);
    } catch (e) {
      log(`${key}: no minimised figure: ${/** @type {Error} */ (e).message}`);
      out["idle.cpu.minimised"] = null;
    }
    return out;
  } finally {
    await halt(app);
  }
}

/** Run all on `project`, with the sampler on from before the click to the end. `tag` ("@10k") suffixes every id. */
async function runPhase(/** @type {"grace" | "gpui"} */ key, /** @type {{ dir: string }} */ project, /** @type {string} */ tag) {
  const app = await launch(key, project.dir, { cdp: true });
  /** @type {ReturnType<typeof sys.startSampler> | null} */
  let sampler = null;
  try {
    await ready(key, app);
    await sleep(2000);
    sampler = sys.startSampler(app.pid, 250);
    live.add(sampler);
    await sampler.ready; // its PowerShell is slow to start, and a run is short
    const t = Date.now();
    await startRun(key, app);
    await endRun(key, app);
    const wall = Date.now() - t;
    const peaks = await sampler.stop().finally(() => live.delete(/** @type {any} */ (sampler)));
    sampler = null;
    await sleep(3000);
    const after = await sys.sample(app.pid);
    return {
      [`run.memory.app${tag}`]: peaks.appPeakMB,
      [`run.memory.java${tag}`]: peaks.javaPeakMB,
      [`run.cpu.app${tag}`]: peaks.appCpuMs,
      [`run.cpu.java${tag}`]: peaks.javaCpuMs,
      [`run.wall${tag}`]: wall,
      [`run.memory.after${tag}`]: after.app.privateMB,
    };
  } finally {
    if (sampler) {
      const s = sampler;
      await s.stop().then(() => {}, () => {}).finally(() => live.delete(s));
    }
    await halt(app);
  }
}

// ---- the wheel probe (fluid.mjs on Windows) ----

/** Gaps (ms) between consecutive requestAnimationFrame callbacks: the frames Electron draws, as the bench logs them. */
const rafProbe = (/** @type {any} */ page) => ({
  async begin() {
    await page.evaluate(() => {
      const w = /** @type {any} */ (window);
      w.__frames = [];
      w.__on = true;
      const tick = (/** @type {number} */ t) => {
        if (!w.__on) return;
        w.__frames.push(t);
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    });
  },
  async end() {
    const t = await page.evaluate(() => {
      const w = /** @type {any} */ (window);
      w.__on = false;
      return w.__frames;
    });
    return /** @type {number[]} */ (t).slice(1).map((x, i) => x - t[i]);
  },
});

/** The prototype's frames are the frames in which its tree lays out rows; calls under 2 ms apart are one frame. */
const gpuiProbe = (/** @type {any} */ app) => ({
  async begin() {
    app.framesStart();
  },
  async end() {
    return (await app.framesStop()).filter((/** @type {number} */ g) => g >= 2);
  },
});

/** Frame statistics of a list of gaps. */
function frameStats(/** @type {number[]} */ gaps) {
  const s = [...gaps].sort((a, b) => a - b);
  const pick = (/** @type {number} */ q) => s[Math.min(s.length - 1, Math.floor(q * s.length))] ?? null;
  return { frames: gaps.length + 1, maxFrame: s.at(-1) ?? null, p95Frame: pick(0.95) };
}

/**
 * 240 wheel notches over the tree, one every 4 ms (faster than the screen refreshes), sent by the OS (window-tools.ps1), while `probe` logs the frames.
 * The probe opens just before the first notch, not before PowerShell's slow setup, or the setup would read as a freeze.
 * `during` runs half a second before the first notch (it starts Run all), so the wheel turns while Karate runs.
 */
async function wheelOver(
  /** @type {number} */ pid,
  /** @type {{ begin(): Promise<void>, end(): Promise<number[]> }} */ probe,
  /** @type {(() => Promise<void>) | undefined} */ during = undefined,
) {
  const startAt = Date.now() + WHEEL_LEAD_MS;
  const sent = tools("wheel", ["-ProcessId", String(pid), "-StartAt", String(startAt)]);
  sent.catch(() => {}); // awaited below; until then a failure is not unhandled
  if (during) {
    await sleep(Math.max(0, startAt - 500 - Date.now()));
    await during();
  }
  await sleep(Math.max(0, startAt - Date.now() - 50));
  await probe.begin();
  const r = await sent;
  await sleep(300);
  const gaps = await probe.end();
  if (r.lateMs > 250) log(`the wheel started ${r.lateMs} ms late: this round's longest frame includes the wait`);
  return { gaps, wheelMs: r.ms };
}

async function fluidPhase(/** @type {"grace" | "gpui"} */ key, /** @type {{ dir: string }} */ small) {
  const app = await launch(key, small.dir, { cdp: true });
  try {
    await ready(key, app);
    await sleep(1500);
    const probe = key === "grace" ? rafProbe(app.page) : gpuiProbe(app);
    /** @type {Record<string, number | null>} */
    const out = {};
    const put = (/** @type {string} */ when, /** @type {number[]} */ gaps) => {
      const s = frameStats(gaps);
      out[`scroll.${when}.frames`] = s.frames;
      out[`scroll.${when}.maxFrame`] = s.maxFrame === null ? null : round1(s.maxFrame);
      out[`scroll.${when}.p95Frame`] = s.p95Frame === null ? null : round1(s.p95Frame);
    };
    const idle = await wheelOver(app.pid, probe);
    put("idle", idle.gaps);
    out["scroll.wheelMs"] = idle.wheelMs;
    // back to the top: a virtual list draws only the rows in view
    await tools("wheel", ["-ProcessId", String(app.pid), "-Up"]);
    await sleep(500);
    put("run", (await wheelOver(app.pid, probe, () => startRun(key, app))).gaps);
    await endRun(key, app);
    return out;
  } finally {
    await halt(app);
  }
}

// ---- the floors: each stack's empty window ----

async function floorPhase(/** @type {NonNullable<typeof report>} */ rep) {
  const electron = path.resolve(args.electron ?? path.join(GRACE, "node_modules", "electron", "dist", "electron.exe"));
  const gpuiFloor = args["gpui-floor"] ? path.resolve(args["gpui-floor"]) : null;
  /** @type {Record<string, () => import("node:child_process").ChildProcess>} */
  const start = {};
  if (existsSync(electron)) start.electron = () => spawn(electron, [path.join(here, "..", "electron-floor")], { stdio: "ignore" });
  else errors.push(`no Electron floor: ${electron} not found`);
  if (gpuiFloor && existsSync(gpuiFloor)) start.gpui = () => spawn(gpuiFloor, [], { stdio: "ignore" });
  else errors.push("no GPUI floor: build it with `cargo build --release --example floor` and pass --gpui-floor");
  const names = Object.keys(start);
  for (let r = 0; r < FLOOR_ROUNDS; r++)
    for (const name of r % 2 ? [...names].reverse() : names) {
      await sys.waitForQuiet();
      const child = start[name]();
      const entry = { pid: child.pid, stop: () => killTree(/** @type {number} */ child.pid) };
      live.add(entry);
      try {
        await sleep(SETTLE_MS);
        const a = await sys.sample(/** @type {number} */ (child.pid));
        await sleep(IDLE_MS);
        const b = await sys.sample(/** @type {number} */ (child.pid));
        const row = { app: name, memMB: b.app.privateMB, cpuMs: b.app.cpuMs - a.app.cpuMs, procs: await treeSize(/** @type {number} */ (child.pid)) };
        rep.floors.push(/** @type {any} */ (row));
        log(`floor ${name} round ${r + 1}: ${JSON.stringify(row)}`);
      } catch (e) {
        errors.push(`floor ${name} round ${r + 1}: ${/** @type {Error} */ (e).message}`);
      } finally {
        await entry.stop().catch(() => {});
        live.delete(entry);
      }
      await sleep(1000);
    }
}

// ---- sizes ----

/** The first folder called `name` under `dir`, down to `depth` levels. */
function findDir(/** @type {string} */ dir, /** @type {string} */ name, depth = 3) {
  if (depth < 0) return null;
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (!e.isDirectory()) continue;
    const p = path.join(dir, e.name);
    if (e.name === name) return p;
    const found = findDir(p, name, depth - 1);
    if (found) return found;
  }
  return null;
}

function measureSizes() {
  /** @type {{ grace: Record<string, number>, gpui: Record<string, number> }} */
  const out = { grace: {}, gpui: {} };
  const jarMB = (/** @type {string} */ f) => (existsSync(f) ? statSync(f).size / 1e6 : 0);
  const jreDir = path.resolve(JAVA, "..", ".."); // <jre>\bin\java.exe
  const jreMB = existsSync(jreDir) ? sys.folderMB(jreDir) : 0;
  if (APPS.includes("grace")) {
    const root = path.dirname(GRACE_EXE);
    out.grace["size.exe"] = round1(statSync(GRACE_EXE).size / 1e6);
    out.grace["size.unpacked"] = sys.folderMB(root);
    const jre = findDir(root, "jre");
    const javaMB = (jre ? sys.folderMB(jre) : 0) + jarMB(path.join(jre ? path.dirname(jre) : root, "karate.jar")) + jarMB(path.join(jre ? path.dirname(jre) : root, "listener.jar"));
    out.grace["size.withoutJava"] = round1(out.grace["size.unpacked"] - javaMB);
    const zip = args["grace-zip"] ?? findZip();
    if (zip && existsSync(zip)) out.grace["size.zip"] = round1(statSync(zip).size / 1e6);
  }
  if (APPS.includes("gpui")) {
    const bytes = statSync(GPUI_EXE).size;
    out.gpui["size.exe"] = round1(bytes / 1e6);
    out.gpui["size.withoutJava"] = out.gpui["size.exe"];
    out.gpui["size.unpacked"] = round1(out.gpui["size.exe"] + jreMB + jarMB(JAR)); // the exe with the same Java beside it
    out.gpui["size.zip"] = round1(deflateRawSync(readFileSync(GPUI_EXE)).length / 1e6); // the exe alone, as a zip would hold it
  }
  return out;
}
/** Grace's zip from `npm run make`, if there is one. */
function findZip() {
  const dir = path.join(GRACE, "out", "make", "zip", "win32", "x64");
  try {
    const zips = readdirSync(dir).filter((f) => f.endsWith(".zip")).sort();
    return zips.length ? path.join(dir, /** @type {string} */ (zips.at(-1))) : null;
  } catch {
    return null;
  }
}

// ---- main ----

/** @type {import("./report.mjs").Report | null} */
let report = null;

async function attempt(/** @type {string} */ what, /** @type {() => Promise<Record<string, number | null>>} */ fn) {
  const t = Date.now();
  try {
    const v = await fn();
    log(`${what}: ${JSON.stringify(v)} (${Math.round((Date.now() - t) / 1000)} s)`);
    return v;
  } catch (e) {
    const msg = `${what}: ${/** @type {Error} */ (e).message}`;
    errors.push(msg);
    log(`FAILED ${msg}`);
    return null;
  }
}

function save() {
  const rep = /** @type {NonNullable<typeof report>} */ (report);
  rep.errors = errors;
  mkdirSync(OUT, { recursive: true });
  writeFileSync(path.join(OUT, `windows-${LABEL}.json`), JSON.stringify(rep, null, 2) + "\n");
  let limits = null;
  try {
    limits = JSON.parse(readFileSync(path.join(GRACE, "bench", "limits.json"), "utf8"));
  } catch {
    // no limits to show
  }
  writeFileSync(path.join(OUT, `windows-${LABEL}.md`), markdown(rep, limits));
}

async function main() {
  const screens = await sys.screens().then(
    (/** @type {any[]} */ all) => all,
    (/** @type {Error} */ e) => {
      log(`could not read the screens: ${e.message}`);
      return [];
    },
  );
  const spot = sys.windowSpot(screens);
  windowAt = { x: spot.x, y: spot.y };
  log(`Grace's windows open at ${spot.x}, ${spot.y}${spot.screen ? ` on ${spot.screen.name}` : " on the main screen"}`);

  workDir = mkdtempSync(path.join(os.tmpdir(), "grace-gpui-measure-"));
  const makeProject = (/** @type {string} */ name, /** @type {number} */ perModule) => {
    const dir = path.join(/** @type {string} */ (workDir), name);
    return { dir, ...writeBigProject(dir, perModule) };
  };
  const small = makeProject("tests-2000", 5);
  const large = args["skip-large"] ? null : makeProject("tests-10000", 25);
  log(`projects: ${small.tests} tests${large ? `, ${large.tests} tests` : ""}`);

  report = {
    label: LABEL,
    date: new Date().toISOString(),
    machine: await describeMachine(),
    settings: { settleS: SETTLE_MS / 1000, idleS: IDLE_MS / 1000, rounds: ROUNDS, floorRounds: FLOOR_ROUNDS, apps: APPS, gpuiWindow },
    rounds: [],
    floors: [],
    sizes: measureSizes(),
    errors,
  };

  const sequence = order(ROUNDS, /** @type {any} */ (APPS));
  for (const [i, key] of /** @type {("grace" | "gpui")[]} */ (sequence).entries()) {
    const warmup = i < APPS.length;
    log(`run ${i + 1} of ${sequence.length}: ${key}${warmup ? " (warm-up)" : ""}`);
    const { load, waitedMs } = await sys.waitForQuiet();
    log(`machine load ${round1(load)}% after waiting ${Math.round(waitedMs / 1000)} s${load >= 35 ? ", not quiet: going ahead" : ""}`);
    /** @type {Record<string, number | null>} */
    const values = {};
    const phases = [
      ["starts", () => startPhase(key, small)],
      ["idle", () => idlePhase(key, small)],
      ["run all, 2,000 tests", () => runPhase(key, small, "")],
      ...(large ? [["run all, 10,000 tests", () => runPhase(key, large, "@10k")]] : []),
      ...(args["skip-fluid"] ? [] : [["wheel over the tree", () => fluidPhase(key, small)]]),
    ];
    let measured = 0;
    for (const [name, fn] of /** @type {[string, () => Promise<Record<string, number | null>>][]} */ (phases)) {
      const v = await attempt(`${key} round ${i + 1} ${name}`, fn);
      if (v) {
        Object.assign(values, v);
        measured++;
      }
    }
    report.settings.gpuiWindow = gpuiWindow;
    report.rounds.push({ app: key, warmup, load: round1(load), values });
    save();
    // A first run in which nothing could be measured is a broken setup, not a bad round: stop before it repeats for an hour.
    if (i < APPS.length && measured === 0) throw new Error(`nothing could be measured for ${key}:\n${errors.join("\n")}`);
  }

  if (!args["skip-floor"]) await floorPhase(report);
  report.settings.gpuiWindow = gpuiWindow;
  save();
  const stat = summarise(report, "gpui");
  log(`wrote ${path.join(OUT, `windows-${LABEL}.json`)} and .md (${Object.keys(stat).length} measures for the prototype)`);
  console.log(markdown(report, null));
  if (errors.length) process.exitCode = 1;
}

try {
  await main();
} catch (e) {
  console.error(`measure failed: ${/** @type {Error} */ (e).stack ?? e}`);
  process.exitCode = 1;
  if (report) save();
} finally {
  await cleanup();
}
