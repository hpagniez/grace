// Starts a desktop app several times and measures time to first paint, time until the project tree
// shows, and idle memory of the app's whole process tree. Same method for Electron and Tauri.
//
//   node measure.mjs <exe> <electron|tauri> <project folder> [runs=6]
//
// MEASURE_REUSE=1 keeps one data folder (settings and browser profile) for every run, as a person's
// second start; without it each run starts on a fresh folder, as a first start.
// Timing runs open a CDP port (Chromium switch for Electron, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
// for WebView2) and poll the page. Memory runs open no port and wait IDLE_MS. The first run of each
// kind is a warm-up and is left out of the medians.
import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { chromium } from "playwright";

const [exe, kind, project, runsArg] = process.argv.slice(2);
if (!exe || !["electron", "tauri"].includes(kind) || !project) {
  console.error("usage: node measure.mjs <exe> <electron|tauri> <project folder> [runs]");
  process.exit(2);
}
const RUNS = Number(runsArg ?? 6);
const IDLE_MS = 20_000;
const PORT = 9333;
const TREE = '[data-testid="tree"]';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor((xs.length - 1) / 2)];

const REUSE = process.env.MEASURE_REUSE === "1";
const shared = REUSE ? mkdtempSync(path.join(tmpdir(), "tauri-spike-measure-")) : null;

/** A data folder (fresh unless REUSE); the app reopens `project` at start, as Grace does with its last project. */
function launch(cdp) {
  const data = shared ?? mkdtempSync(path.join(tmpdir(), "tauri-spike-measure-"));
  const env = { ...process.env };
  const args = [];
  if (kind === "electron") {
    writeFileSync(path.join(data, "settings.json"), JSON.stringify({ lastProject: project }));
    env.GRACE_USER_DATA = data;
    if (cdp) args.push(`--remote-debugging-port=${PORT}`);
  } else {
    env.SPIKE_PROJECT = project;
    env.SPIKE_DATA = data;
    env.WEBVIEW2_USER_DATA_FOLDER = path.join(data, "webview");
    if (cdp) env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = `--remote-debugging-port=${PORT}`;
  }
  const t0 = Date.now();
  const child = spawn(exe, args, { env, stdio: "ignore" });
  return { child, data, t0 };
}

function stop({ child, data }) {
  try {
    execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  } catch {
    // already gone
  }
  return sleep(1000).then(() =>
    REUSE ? undefined : rmSync(data, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }),
  );
}

async function connect() {
  for (;;) {
    try {
      return await chromium.connectOverCDP(`http://127.0.0.1:${PORT}`);
    } catch {
      await sleep(25);
    }
  }
}

async function timingRun() {
  const run = launch(true);
  const browser = await connect();
  let page;
  while (!(page = browser.contexts()[0]?.pages()[0])) await sleep(20);
  let treeAt = null;
  while (treeAt === null) {
    const found = await page.evaluate((s) => !!document.querySelector(s), TREE).catch(() => false);
    if (found) treeAt = Date.now();
    else await sleep(20);
    if (Date.now() - run.t0 > 60_000) throw new Error("tree never showed");
  }
  const paintAt = await page.evaluate(() => {
    const e = performance.getEntriesByName("first-contentful-paint")[0];
    return e ? performance.timeOrigin + e.startTime : null;
  });
  await browser.close().catch(() => {});
  await stop(run);
  return { firstPaintMs: Math.round(paintAt - run.t0), treeMs: treeAt - run.t0 };
}

/** Private working set (Task Manager's Memory column) and working set of the process tree, Java excluded. */
function treeMemory(rootPid) {
  const ps = `
$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
$ids = [System.Collections.Generic.HashSet[int]]::new(); [void]$ids.Add(${rootPid})
do { $n = $ids.Count; foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } } } while ($ids.Count -ne $n)
$perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Where-Object { $ids.Contains([int]$_.IDProcess) }
$perf | ForEach-Object { [pscustomobject]@{ name = ($all | Where-Object ProcessId -eq $_.IDProcess).Name; private = [long]$_.WorkingSetPrivate; ws = [long]$_.WorkingSet } } | ConvertTo-Json -Compress`;
  const out = execFileSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", ps], {
    encoding: "utf8",
  });
  const rows = [JSON.parse(out)].flat().filter((r) => !/^(java|conhost)\.exe$/i.test(r.name));
  const mb = (n) => Math.round(n / 1e5) / 10;
  return {
    privateMB: mb(rows.reduce((s, r) => s + r.private, 0)),
    workingSetMB: mb(rows.reduce((s, r) => s + r.ws, 0)),
    processes: rows.map((r) => `${r.name} ${mb(r.private)}`),
  };
}

async function memoryRun() {
  const run = launch(false);
  await sleep(IDLE_MS);
  const m = treeMemory(run.child.pid);
  await stop(run);
  return m;
}

const timings = [];
for (let i = 0; i < RUNS; i++) timings.push(await timingRun());
const memory = [];
for (let i = 0; i < Math.max(2, Math.ceil(RUNS / 2)); i++) memory.push(await memoryRun());

const warm = timings.slice(1);
const warmMem = memory.slice(1);
if (shared) rmSync(shared, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
console.log(JSON.stringify({ exe, kind, reuse: REUSE, timings, memory }, null, 2));
console.log(
  `median of ${warm.length} warm runs: first paint ${median(warm.map((t) => t.firstPaintMs))} ms, ` +
    `tree ${median(warm.map((t) => t.treeMs))} ms; idle private working set ` +
    `${median(warmMem.map((m) => m.privateMB))} MB (working set ${median(warmMem.map((m) => m.workingSetMB))} MB)`,
);
