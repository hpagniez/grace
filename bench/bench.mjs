// `npm run bench`: measures a packaged Grace exe, or two to compare, and writes bench.json and bench.md
// (docs/specs/2026-10-31-light-and-fast.md, R2).
//
//   node bench/bench.mjs [--b <exe>] [--a <exe>] [--a-zip <zip>] [--b-zip <zip>] [--label-a <text>] [--label-b <text>]
//                        [--release] [--rounds <n>] [--out <dir>]
//   node bench/bench.mjs --limits-from <bench.json> --commit <sha>
//
// B is the build under test (out/Grace-win32-x64/Grace.exe unless given), A the one it is compared with. Rounds run
// after one warm-up per build and alternate between the builds. Release mode adds the fresh-profile start, the minimised
// idle CPU and the 10,000-test project, and needs bench/limits.json. A worse number never fails the bench; only a failure
// to measure does (exit 1). The windows open on the first screen that is not the main one, when there is one.
// `--limits-from` measures nothing: it turns the rounds of a release-mode bench.json of build B into bench/limits.json.
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { parseArgs } from "node:util";
import { writeBigProject } from "../e2e/big-project.ts";
import { startApp, startTimes } from "./app.mjs";
import { judge, makeLimits } from "./judge.mjs";
import { MEASURES, RUN_PHASE } from "./measures.mjs";
import {
  clickToPaint,
  finishedLagMs,
  framesDuring,
  instrument,
  newestEventsFile,
  pageStats,
  scrollTree,
  TREE,
} from "./probes.mjs";
import { markdown } from "./report.mjs";
import { median, order } from "./stats.mjs";
import {
  folderMB,
  minimise,
  sample,
  screens,
  startSampler,
  waitForQuiet,
  windowSpot,
} from "./system.mjs";

/** @typedef {import("./judge.mjs").Report} Report */
/** @typedef {import("./judge.mjs").Values} Values */
/** @typedef {import("@playwright/test").Page} Page */
/** @typedef {import("./app.mjs").App} App */
/** @typedef {{ dir: string, tests: number }} Project */

/** The machine the limits are measured on (docs/specs/2026-10-31-light-and-fast.md, R5.2). */
const REFERENCE_MACHINE = "Windows 11 Pro, Ryzen 7 5800X, 32 GB";

/** Idle: settle for 20 s after the start, then take the CPU over 60 s. */
const SETTLE_MS = 20_000;
const IDLE_CPU_MS = 60_000;
/** After minimising, the window's change of state is over before the CPU is counted. */
const MINIMISE_SETTLE_MS = 2_000;
/** How long the tree may take to hold the project (the dry run of 10,000 tests is slow). */
const TREE_LOAD_MS = 120_000;
/** Run all gets 30 minutes, the probes included; a run that takes longer cannot be measured. */
const RUN_LIMIT_MS = 30 * 60_000;
/** A run that has not started a minute after the click never will: the build cannot be driven. */
const RUN_START_MS = 60_000;

const NODE = "[data-testid=tree-node]";
const TEST_ROWS = `${NODE}[data-kind=scenario], ${NODE}[data-kind=row]`;
const TOP = `${NODE}[data-key="d:tests"]`;
const tab = (/** @type {string} */ id) => ({ sel: `[data-testid=tab-${id}]` });
const selected = (/** @type {{ sel: string, index?: number }} */ of) => ({
  attr: "aria-selected",
  of,
});

const log = (/** @type {string} */ msg) =>
  console.error(`[bench ${new Date().toTimeString().slice(0, 8)}] ${msg}`);
const round1 = (/** @type {number} */ n) => Math.round(n * 10) / 10;

/**
 * What is running that must not outlive the bench: apps and samplers (anything with a `stop` and a `pid`). An entry
 * leaves only once its stop has settled, so a signal during a stop still finds it.
 * @type {Set<{ pid: number | undefined, stop(...args: any[]): Promise<unknown> }>}
 */
const live = new Set();
/** Pids of apps spawned whose start has not returned yet: nothing to `stop` yet, but a process to kill. */
const starting = new Set();
/** @type {string | null} */
let workDir = null;
/** Where every window opens: chosen once per bench run, so each start draws on the same screen. */
let windowAt = { x: 40, y: 40 };

let dataCount = 0;
/** A data folder for one start, inside the work folder: removing that one folder removes them all. */
const freshDir = () => path.join(/** @type {string} */ (workDir), `data-${++dataCount}`);

/** Kills the trees of `pids` and waits. Synchronous, for a process that is about to exit. */
function killNow(/** @type {(number | undefined)[]} */ pids) {
  const list = pids.filter((p) => p !== undefined);
  if (!list.length) return;
  try {
    execFileSync("taskkill", ["/F", "/T", ...list.flatMap((p) => ["/PID", String(p)])], {
      stdio: "ignore",
      windowsHide: true,
    });
  } catch {
    // a process that is already gone
  }
}

/** What an exit that skipped `cleanup` (a crash, `process.exit`) leaves; `taskkill` also needs the root alive, so this is the last resort. */
const owned = () => [...starting, ...[...live].map((x) => x.pid)];

/** Removes the folder of the projects and data; best effort, as Windows may still hold a file of a process just killed. */
function removeWork() {
  if (!workDir) return;
  // Synchronous, so it works in an exit handler; rmSync's own retries do not wait, so wait here, up to 10 s.
  for (let tries = 0; ; tries++) {
    try {
      return rmSync(workDir, { recursive: true, force: true });
    } catch (e) {
      if (tries === 40)
        return console.error(`could not remove ${workDir}: ${/** @type {Error} */ (e).message}`);
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
    }
  }
}

/** Stops what is still running and removes the projects. Safe to call twice, and never rejects. */
async function cleanup() {
  killNow([...starting]); // an app still starting has no `stop` yet
  await Promise.allSettled([...live].map((x) => x.stop().then(() => live.delete(x))));
  removeWork();
}

// The last line of defence: whatever the bench still owns when the process exits is killed, whatever the reason.
process.on("exit", () => {
  killNow(owned());
  removeWork();
});

/** An error nobody caught, or a signal: clean up, then exit with 1. Once only. */
let dying = false;
function die(/** @type {string} */ why) {
  if (dying) return;
  dying = true;
  console.error(why);
  process.exitCode = 1;
  cleanup().finally(() => process.exit(1));
}

/** @returns {Promise<App>} */
async function launch(
  /** @type {string} */ exe,
  /** @type {Parameters<typeof startApp>[1]} */ opts,
) {
  /** @type {number | undefined} */
  let pid;
  try {
    const app = await startApp(exe, {
      ...opts,
      at: windowAt,
      onSpawn: (p) => starting.add((pid = p)),
    });
    live.add(app);
    return app;
  } finally {
    // A failed start has stopped what it spawned before it rejected.
    if (pid !== undefined) starting.delete(pid);
  }
}

/** Stops `app`. A failure is reported and fails the bench at the end, but never hides the error that got us here. */
async function halt(/** @type {App} */ app, /** @type {{ keepData?: boolean }} */ opts = {}) {
  await app.stop(opts).then(
    () => live.delete(app), // only once it is gone: a signal during the stop must still find it
    (e) => {
      console.error(`could not stop the app: ${e.message}`);
      process.exitCode = 1;
    },
  );
}

/** One start on `dataDir` (a fresh folder when not given): paint and tree times, null when the build shows no tree. */
async function starts(
  /** @type {string} */ exe,
  /** @type {string} */ project,
  dataDir = freshDir(),
) {
  const app = await launch(exe, { project, dataDir, cdp: true });
  try {
    const times = await startTimes(app).catch((e) => {
      log(`no start times: ${e.message}`);
      return { paintMs: null, treeMs: null };
    });
    return { dataDir: app.dataDir, ...times };
  } finally {
    await halt(app, { keepData: true });
  }
}

/** CPU time the tree has used in all, app and Java. */
const cpu = (/** @type {Awaited<ReturnType<typeof sample>>} */ s) => s.app.cpuMs + s.java.cpuMs;

/** Idle on `dataDir` with no CDP: memory after 20 s, then the CPU over 60 s shown and, in release mode, minimised. */
async function idle(
  /** @type {string} */ exe,
  /** @type {string} */ project,
  /** @type {string} */ dataDir,
  /** @type {boolean} */ release,
) {
  const app = await launch(exe, { project, dataDir, cdp: false });
  try {
    await sleep(SETTLE_MS);
    const first = await sample(app.pid);
    /** @type {Values} */
    const out = { "idle.memory": round1(first.app.privateMB + first.java.privateMB) };
    await sleep(IDLE_CPU_MS);
    out["idle.cpu.shown"] = cpu(await sample(app.pid)) - cpu(first);
    if (release) {
      await minimise(app.pid);
      await sleep(MINIMISE_SETTLE_MS);
      const before = await sample(app.pid);
      await sleep(IDLE_CPU_MS);
      out["idle.cpu.minimised"] = cpu(await sample(app.pid)) - cpu(before);
    }
    return out;
  } finally {
    await halt(app);
  }
}

/** The run's outcome as the page shows it ("" until a run starts). */
const outcome = (/** @type {Page} */ page) =>
  page.evaluate(
    () => document.querySelector("[data-testid=run-outcome]")?.textContent?.trim() ?? "",
  );

/** Waits until the outcome reads like `pattern`; a timeout is the caller's failure to measure. */
const waitForOutcome = (
  /** @type {Page} */ page,
  /** @type {RegExp} */ pattern,
  timeout = RUN_LIMIT_MS,
) =>
  page.waitForFunction(
    (source) =>
      new RegExp(source).test(
        document.querySelector("[data-testid=run-outcome]")?.textContent?.trim() ?? "",
      ),
    pattern.source,
    { timeout, polling: 250 },
  );

/**
 * Clicks Run all and waits until the run has started: null once it has, else why not (no button to click in 30 s, or no
 * run a minute after the click). Any other error, such as a page that crashed, is the caller's failure to measure.
 * @param {Page} page
 * @returns {Promise<string | null>}
 */
async function startRun(page) {
  const timedOut = (/** @type {any} */ e, /** @type {string} */ why) => {
    if (e?.name !== "TimeoutError") throw e;
    return why;
  };
  try {
    await page.getByTestId("run-all-button").click({ timeout: 30_000 });
  } catch (e) {
    return timedOut(e, "no Run all button to click in 30 s");
  }
  try {
    await waitForOutcome(page, /^(Running|Finished|Cancelled|Run failed)/, RUN_START_MS);
    return null;
  } catch (e) {
    return timedOut(e, `Run all did not start in ${RUN_START_MS / 1000} s`);
  }
}

/**
 * Waits for the tree to hold the project's tests and gives its row count. A build whose tree never does (it reads
 * differently) gives the rows it has after 2 minutes, and the probes read null rather than the bench waits for ever.
 */
async function treeRows(/** @type {Page} */ page, /** @type {number} */ tests) {
  // A virtual list draws only the rows in view and says how many there are in data-rows.
  const rows = () =>
    page
      .evaluate(
        ([box, node]) =>
          Number(/** @type {HTMLElement | null} */ (document.querySelector(box))?.dataset.rows) ||
          document.querySelectorAll(node).length,
        [TREE, NODE],
      )
      .catch(() => 0); // the page may be mid-navigation at first
  const until = Date.now() + TREE_LOAD_MS;
  let count = await rows();
  while (count < tests && Date.now() < until) {
    await sleep(250);
    count = await rows();
  }
  if (count < tests)
    log(`the tree has ${count} rows, not ${tests}, after ${TREE_LOAD_MS / 1000} s`);
  return count;
}

/**
 * The four actions of the Smooth group, in the state `state` ("idle" or "run"): select a test, close then open the top
 * folder (the open is measured), scroll the tree, open the HTTP tab and go back to Console. A probe that finds nothing
 * to act on reads null, and so does its frame. In a run, an action starts only while the run is still going: one that
 * never got to start reads null. `index` is the test row to select, one that is not selected already, or the click
 * would time nothing.
 * @param {Page} page
 * @param {{ state: "idle" | "run", index: number, total: number }} opts `total` is the tree's rows with every folder open.
 * @returns {Promise<Values>}
 */
async function smoothProbes(page, { state, index, total }) {
  /** @type {Values} */
  const got = {};
  for (const action of ["select", "expand", "http"]) got[`smooth.${action}.${state}.paint`] = null;
  for (const action of ["select", "expand", "http", "scroll"])
    got[`smooth.${action}.${state}.frame`] = null;
  const going = async () => state === "idle" || (await outcome(page)).startsWith("Running");
  const timed = async (
    /** @type {string} */ action,
    /** @type {() => Promise<number | null>} */ act,
  ) => {
    /** @type {number | null} */
    let paint = null;
    const { frameMs } = await framesDuring(page, async () => {
      paint = await act();
    });
    if (paint === null) return;
    got[`smooth.${action}.${state}.paint`] = paint;
    got[`smooth.${action}.${state}.frame`] = frameMs;
  };

  const row = { sel: TEST_ROWS, index };
  if (await going())
    await timed("select", () => clickToPaint(page, { ...row, label: true }, selected(row)));
  if (await going()) {
    const toggle = { sel: `${TOP} button` };
    await clickToPaint(page, toggle, { rows: "tree", under: 10 });
    await timed("expand", () => clickToPaint(page, toggle, { rows: "tree", atLeast: total }));
  }
  if (await going()) got[`smooth.scroll.${state}.frame`] = (await scrollTree(page)).frameMs;
  if (await going()) {
    await timed("http", () => clickToPaint(page, tab("http"), selected(tab("http"))));
    await clickToPaint(page, tab("console"), selected(tab("console")));
  }
  return got;
}

/**
 * The Run group, the Smooth group and the Page group on one project: start with CDP, probe idle, Run all with the
 * sampler on and the same probes during it. A Run all that cannot be started reads null for every run figure (the
 * idle ones stand). `tag` suffixes every id (`@10k`).
 * @param {string} exe
 * @param {Project} project
 * @param {string} tag
 * @returns {Promise<Values>}
 */
async function interactive(exe, { dir, tests }, tag) {
  const app = await launch(exe, { project: dir, dataDir: freshDir(), cdp: true });
  /** @type {Values} */
  const out = {};
  const put = (/** @type {string} */ id, /** @type {number | null} */ v) => (out[id + tag] = v);
  /** @type {ReturnType<typeof startSampler> | null} */
  let sampler = null;
  try {
    const page = /** @type {Page} */ (app.page);
    const cdp = /** @type {import("@playwright/test").CDPSession} */ (app.cdp);
    const total = await treeRows(page, tests);
    await instrument(page); // before Run all: the watch on "Finished" starts here
    for (const [id, v] of Object.entries(
      await smoothProbes(page, { state: "idle", index: 4, total }),
    ))
      put(id, v);
    const open = await pageStats(cdp);
    put("page.nodes.open", open.nodes);
    put("page.heap.open", open.heapMB);

    sampler = startSampler(app.pid);
    live.add(sampler);
    await sampler.ready; // its PowerShell is slow to start, and a run is short
    const clickedAt = Date.now();
    const left = () => Math.max(1, RUN_LIMIT_MS - (Date.now() - clickedAt)); // a timeout of 0 is none
    const over = /^(Finished|Cancelled|Run failed)/;
    // A build the bench cannot drive (an older base, a renamed test id) gives no run figures, not a failed bench.
    const notStarted = await startRun(page);
    if (notStarted) {
      log(`${notStarted}: every run figure${tag && ` ${tag}`} reads null this round`);
      for (const id of RUN_PHASE) put(id, null);
      return out;
    }
    // The same probes as idle, now while Karate runs; each one that cannot start before the run ends reads null.
    const probeAndWait = async () => {
      for (const [id, v] of Object.entries(
        await smoothProbes(page, { state: "run", index: 5, total }),
      ))
        put(id, v);
      await waitForOutcome(page, over, left());
    };
    const running = (await outcome(page)).startsWith("Running");
    const frames = running
      ? await framesDuring(page, probeAndWait)
      : await probeAndWait().then(() => null);
    const end = await outcome(page);
    if (end !== "Finished") throw new Error(`Run all ended as "${end}", not Finished`);
    // The sampler ignores a root that exits mid-run and keeps its figures so far: that is not a measurement.
    if (!isAlive(app.pid)) throw new Error("the app exited during Run all");

    const peaks = await sampler.stop().finally(() => live.delete(sampler));
    put("run.memory.app", peaks.appPeakMB);
    put("run.memory.java", peaks.javaPeakMB);
    put("run.cpu.app", peaks.appCpuMs);
    put("run.cpu.java", peaks.javaCpuMs);
    put("run.finishedLag", await finishedLagMs(page, newestEventsFile(app.dataDir, dir)));
    put("smooth.run.fps", frames?.fps ?? null);
    put("smooth.run.frame", frames?.frameMs ?? null);
    const after = await pageStats(cdp);
    put("page.nodes.run", after.nodes);
    put("page.heap.run", after.heapMB);
    return out;
  } finally {
    if (sampler && live.has(sampler))
      await sampler
        .stop()
        .then(
          () => {},
          () => {},
        )
        .finally(() => live.delete(sampler));
    await halt(app);
  }
}

/** Signal 0 only asks whether the process exists; EPERM means it does. */
function isAlive(/** @type {number} */ pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (e) {
    return /** @type {any} */ (e).code === "EPERM";
  }
}

/**
 * One round of one build: the starts, idle, then Run all on the 2,000-test project, and in release mode on the 10,000-test one.
 * @param {string} exe
 * @param {{ small: Project, large: Project | null }} projects
 * @param {boolean} release
 * @returns {Promise<Values>}
 */
async function measureRound(exe, { small, large }, release) {
  /** @type {Values} */
  const values = {};
  /** @type {string | undefined} */
  let dataDir;
  try {
    const fresh = await starts(exe, small.dir);
    dataDir = fresh.dataDir;
    if (release) {
      values["start.paint.fresh"] = fresh.paintMs;
      values["start.tree.fresh"] = fresh.treeMs;
    }
    const reused = await starts(exe, small.dir, dataDir);
    values["start.paint.reused"] = reused.paintMs;
    values["start.tree.reused"] = reused.treeMs;
    Object.assign(values, await idle(exe, small.dir, dataDir, release));
  } finally {
    if (dataDir) rmSync(dataDir, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
  }
  Object.assign(values, await interactive(exe, small, ""));
  if (large) Object.assign(values, await interactive(exe, large, "@10k"));
  return values;
}

/** A description of this machine for the table's header. */
function machine() {
  const cpus = os.cpus();
  return `${os.version()}, ${cpus[0].model.trim()} (${cpus.length} threads), ${Math.round(os.totalmem() / 2 ** 30)} GB`;
}

/** Exits with a message and code 2: a usage error is not a failure to measure. */
function usage(/** @type {string} */ message) {
  console.error(message);
  process.exit(2);
}

async function main() {
  let args;
  try {
    ({ values: args } = parseArgs({
      options: {
        a: { type: "string" },
        b: { type: "string" },
        "a-zip": { type: "string" },
        "b-zip": { type: "string" },
        "label-a": { type: "string" },
        "label-b": { type: "string" },
        release: { type: "boolean" },
        rounds: { type: "string" },
        out: { type: "string" },
        "limits-from": { type: "string" },
        commit: { type: "string" },
      },
    }));
  } catch (e) {
    return usage(/** @type {Error} */ (e).message);
  }
  const limitsFile = path.join(import.meta.dirname, "limits.json");
  if (args["limits-from"]) {
    if (!args.commit) return usage("--limits-from needs --commit");
    const report = JSON.parse(readFileSync(args["limits-from"], "utf8"));
    const limits = makeLimits(report, { machine: REFERENCE_MACHINE, commit: args.commit });
    const missing = Object.keys(MEASURES).filter((id) => !limits.measures[id]);
    if (missing.length) console.error(`no value for ${missing.join(", ")}: they get no limit`);
    writeFileSync(limitsFile, JSON.stringify(limits, null, 2) + "\n");
    return console.error(`wrote ${limitsFile}`);
  }
  const release = args.release === true;
  if (release && !existsSync(limitsFile))
    return usage(`${limitsFile} is missing: a release bench is judged against it`);
  const rounds = Number(args.rounds ?? (release ? 5 : 3));
  if (!Number.isInteger(rounds) || rounds < 1)
    return usage(`--rounds must be a whole number of 1 or more`);
  if (!args.a && (args["a-zip"] || args["label-a"])) return usage("--a-zip and --label-a need --a");
  const repo = path.resolve(import.meta.dirname, "..");
  const builds = {
    a: args.a
      ? {
          label: String(args["label-a"] ?? "build A"),
          exe: String(args.a),
          zip: /** @type {string | undefined} */ (args["a-zip"]),
        }
      : null,
    b: {
      label: String(args["label-b"] ?? "this build"),
      exe: String(args.b ?? path.join(repo, "out", "Grace-win32-x64", "Grace.exe")),
      zip: /** @type {string | undefined} */ (args["b-zip"]),
    },
  };
  const outDir = path.resolve(String(args.out ?? "bench-out"));

  const spot = await screens().then(windowSpot, (e) => {
    log(`could not read the screens, so the windows open on the main one: ${e.message}`);
    return windowSpot([]);
  });
  windowAt = { x: spot.x, y: spot.y };
  const screen = spot.screen
    ? `${spot.screen.name.replace(/^\\\\\.\\/, "")}, not the main screen (work area ${spot.screen.width} x ${spot.screen.height} at ${spot.screen.x}, ${spot.screen.y})`
    : "the main screen";
  log(`windows open at ${spot.x}, ${spot.y} on ${screen}`);

  workDir = mkdtempSync(path.join(os.tmpdir(), "grace-bench-projects-"));
  /** @type {{ small: Project, large: Project | null }} */
  const projects = {
    small: makeProject("tests-2000", 5),
    large: release ? makeProject("tests-10000", 25) : null,
  };
  function makeProject(/** @type {string} */ name, /** @type {number} */ perModule) {
    const dir = path.join(/** @type {string} */ (workDir), name);
    return { dir, tests: writeBigProject(dir, perModule).tests };
  }

  const sizes = Object.fromEntries(
    Object.entries(builds)
      .filter(([, build]) => build)
      .map(([key, build]) => [
        key,
        {
          "size.unpacked": folderMB(path.dirname(path.resolve(build.exe))),
          ...(build.zip && { "size.zip": round1(statSync(build.zip).size / 1e6) }),
        },
      ]),
  );

  /** @type {Report["rounds"]} */
  const results = [];
  const keys = builds.a
    ? /** @type {("a" | "b")[]} */ (["a", "b"])
    : /** @type {("a" | "b")[]} */ (["b"]);
  const sequence = order(rounds, keys);
  for (const [i, key] of sequence.entries()) {
    const warmup = i < keys.length;
    log(`run ${i + 1} of ${sequence.length}: build ${key}${warmup ? " (warm-up)" : ""}`);
    const { load, waitedMs } = await waitForQuiet();
    log(
      `machine load ${round1(load)}% after waiting ${Math.round(waitedMs / 1000)} s${load >= 35 ? ", not quiet: going ahead" : ""}`,
    );
    const started = Date.now();
    const values = { ...sizes[key], ...(await measureRound(builds[key].exe, projects, release)) };
    log(`run ${i + 1} took ${Math.round((Date.now() - started) / 1000)} s`);
    results.push({ build: key, warmup, load: round1(load), values });
  }

  const medians = (/** @type {"a" | "b"} */ key) => {
    const rs = results.filter((r) => r.build === key && !r.warmup);
    const ids = new Set(rs.flatMap((r) => Object.keys(r.values)));
    return Object.fromEntries(
      [...ids].map((id) => [id, median(rs.map((r) => r.values[id] ?? null))]),
    );
  };
  // bench.json may be committed (a release's record): the home folder must not name the person
  const shown = (/** @type {string} */ exe) =>
    path.resolve(exe).replace(os.homedir(), "~").replaceAll("\\", "/");
  /** @type {Report} */
  const report = {
    mode: release ? "release" : "pr",
    machine: machine(),
    screen,
    builds: {
      a: builds.a && { label: builds.a.label, exe: shown(builds.a.exe) },
      b: { label: builds.b.label, exe: shown(builds.b.exe) },
    },
    rounds: results,
    medians: { a: builds.a ? medians("a") : null, b: medians("b") },
  };
  // written first: a limits file that cannot be read must not lose an hour of rounds
  mkdirSync(outDir, { recursive: true });
  writeFileSync(path.join(outDir, "bench.json"), JSON.stringify(report, null, 2) + "\n");
  const limits = existsSync(limitsFile)
    ? JSON.parse(readFileSync(limitsFile, "utf8"))
    : { machine: "", commit: "", measures: {} };
  const table = markdown(report, judge(report, limits));
  writeFileSync(path.join(outDir, "bench.md"), table);
  console.log(table);
}

for (const signal of ["SIGINT", "SIGTERM", "SIGHUP", "SIGBREAK"])
  process.on(signal, () => die(`bench stopped by ${signal}`));
for (const event of ["uncaughtException", "unhandledRejection"])
  process.on(event, (e) => die(`bench failed: ${/** @type {Error} */ (e)?.stack ?? e}`));

try {
  await main();
} catch (e) {
  console.error(`bench failed: ${/** @type {Error} */ (e).stack ?? e}`);
  process.exitCode = 1;
} finally {
  await cleanup();
}
