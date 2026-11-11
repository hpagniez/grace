// VL-29 without UI Automation: the app's CPU during a Run all with no assistive technology attached, so GPUI Grace
// caches its panes (#327) and Electron Grace's page builds no accessibility tree. Drives both apps the same way, from
// outside, through uiprobe.exe (built from docs/checks/2026-11-10-gpui-p2-exit/uiprobe.cs). Run from the repository root:
//
//   node --no-warnings docs/checks/2026-11-10-gpui-p2-recheck/vl-noat.mjs <probe.exe> <exe> <project> <rounds>
//
// One calibration launch, read through UI Automation, finds Run all in a 1400 x 900 window on the second screen; that
// process is then killed. Each measured round starts a fresh process that no UIA client ever touches: it is placed the
// same way, the tree is taken as shown once the dry run's Java is gone, Run all is clicked at the calibrated
// place, and the app's and Java's CPU are sampled until the run's Java has exited. Prints the medians as JSON.
import { execFile, spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";
import { killTree } from "../../../bench/app.mjs";
import { median } from "../../../bench/stats.mjs";
import { placeWindow, sample, startSampler } from "../../../bench/system.mjs";

const run = promisify(execFile);
const [probe, exe, project, roundsArg] = process.argv.slice(2);
const AT = { x: 2048 + 80, y: 60 }; // the second screen, logical pixels (125%), as vl.mjs
const SIZE = { width: 1400, height: 900 };

async function p(...args) {
  const { stdout } = await run(probe, args.map(String), { maxBuffer: 64 << 20 });
  return stdout.trim() === "null" ? null : stdout.startsWith("{") ? JSON.parse(stdout) : stdout;
}

function start() {
  const data = mkdtempSync(path.join(os.tmpdir(), "vl-noat-"));
  writeFileSync(
    path.join(data, "settings.json"),
    JSON.stringify({ lastProject: project, updates: { checkAtStartup: false }, window: { ...AT, ...SIZE, maximized: false } }),
  );
  const t0 = Date.now();
  const child = spawn(exe, [], { env: { ...process.env, GRACE_USER_DATA: data }, stdio: "ignore" });
  return { pid: child.pid, data, t0 };
}

async function stop({ pid, data, t0 }) {
  await killTree(pid, t0).catch((e) => console.error(e.message));
  rmSync(data, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
}

/** Waits until a Java process has run under `pid` and exited, polling every 500 ms. */
async function javaCameAndWent(pid, timeoutMs) {
  const end = Date.now() + timeoutMs;
  let seen = false;
  let gone = 0;
  while (Date.now() < end) {
    const { java } = await sample(pid);
    if (java.privateMB > 0) {
      seen = true;
      gone = 0;
    } else if (seen && ++gone >= 2) return;
    await sleep(500);
  }
  throw new Error("Java never came and went");
}

/**
 * Waits for the dry run that lists the tests to be over. On the 2,000-test project it lasts about 2 s, less than two of
 * sample()'s PowerShell readings, so its Java may never be seen: wait until 10 s after the start, then for two readings
 * in a row with no Java.
 */
async function dryRunOver({ pid, t0 }) {
  await sleep(Math.max(0, t0 + 10000 - Date.now()));
  const end = Date.now() + 120000;
  let none = 0;
  while (Date.now() < end) {
    const { java } = await sample(pid);
    none = java.privateMB > 0 ? 0 : none + 1;
    if (none >= 2) return;
    await sleep(500);
  }
  throw new Error("the dry run's Java never went");
}

// calibration, through UI Automation, in a process that is not measured
const cal = start();
let runAll;
try {
  const shown = await p("waitname", cal.pid, "Collapse tests", 120000);
  if (!shown?.at) throw new Error("the tree never showed");
  await placeWindow(cal.pid, { ...AT, ...SIZE });
  await sleep(1000);
  runAll = await p("rect", cal.pid, "Run all");
  if (!runAll) throw new Error("no Run all");
} finally {
  await stop(cal);
}
const click = { x: runAll.x + (runAll.w >> 1), y: runAll.y + (runAll.h >> 1) };
console.error(JSON.stringify({ runAll: click }));

const rounds = [];
for (let i = 0; i < Number(roundsArg); i++) {
  const app = start();
  try {
    await dryRunOver(app); // the dry run that lists the tests
    await placeWindow(app.pid, { ...AT, ...SIZE });
    await sleep(3000);
    await p("front", app.pid);
    const sampler = startSampler(app.pid);
    await sampler.ready;
    const t = Date.now();
    await p("click", click.x, click.y);
    await javaCameAndWent(app.pid, 15 * 60000);
    const wallMs = Date.now() - t;
    await sleep(1500); // the app takes in Java's last lines and draws Finished
    const peaks = await sampler.stop();
    const r = { runCpuApp: peaks.appCpuMs, runCpuJava: peaks.javaCpuMs, runMemApp: peaks.appPeakMB, wallMs };
    console.error(JSON.stringify(r));
    rounds.push(r);
  } finally {
    await stop(app);
  }
}
const keys = [...new Set(rounds.flatMap(Object.keys))];
console.log(
  JSON.stringify({ exe, project, rounds, medians: Object.fromEntries(keys.map((k) => [k, median(rounds.map((r) => r[k] ?? null))])) }),
);
