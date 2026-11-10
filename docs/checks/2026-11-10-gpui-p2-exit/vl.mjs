// The 2026-11-10 GPUI P2 exit check's VL-26 to VL-29 harness: drives Electron Grace and GPUI Grace from outside, the
// same way, through uiprobe.exe (built from uiprobe.cs beside this file). Run from the repository root:
//
//   node --no-warnings docs/checks/2026-11-10-gpui-p2-exit/vl.mjs <probe.exe> <exe> <project> <rounds> [dump]
//
// Per round: start on the project (fresh data folder, 1400 x 900 on the second screen), time to the tree's count
// label, the UI thread's longest stall while opening (GPUI only: WM_NULL reaches Electron's browser process, not its
// page), memory after open; close and open the first folder and select a test, each from the click to the first
// changed pixel in the tree; Run all with the sampler, Finished's lag after Karate's last test, the longest stall and
// a scroll of the tree during the run. Prints the medians as JSON.
import { execFile, spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";
import { killTree } from "../../../bench/app.mjs";
import { median } from "../../../bench/stats.mjs";
import { placeWindow, sample, startSampler } from "../../../bench/system.mjs";

const run = promisify(execFile);
const [probe, exe, project, roundsArg, mode] = process.argv.slice(2);
const gpui = !process.env.NO_PING && !readdirSync(path.join(path.dirname(exe), "resources")).includes("app.asar");
const AT = { x: 2048 + 80, y: 60 }; // the second screen, logical pixels (125%)

async function p(...args) {
  const { stdout } = await run(probe, args.map(String), { maxBuffer: 64 << 20 });
  return stdout.trim() === "null" ? null : stdout.startsWith("{") ? JSON.parse(stdout) : stdout;
}

/** The UI thread's answers to WM_NULL from now until the returned stop() is called. */
function ping(pid) {
  const stopFile = path.join(os.tmpdir(), `vl-stop-${pid}-${Date.now()}`);
  const done = run(probe, ["ping", String(pid), stopFile], { maxBuffer: 64 << 20 }).then((r) => JSON.parse(r.stdout));
  return async () => {
    writeFileSync(stopFile, "");
    const out = await done;
    rmSync(stopFile, { force: true });
    return out;
  };
}

/** Karate's last top-level SCENARIO_EXIT, ms since the epoch, in the newest events file under the data folder or project. */
function lastExit(roots) {
  const files = [];
  const walk = (d) => {
    for (const e of readdirSync(d, { withFileTypes: true })) {
      const f = path.join(d, e.name);
      if (e.isDirectory()) walk(f);
      else if (e.name === "karate-events.jsonl") files.push(f);
    }
  };
  for (const r of roots) walk(r);
  const newest = files.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
  const exits = readFileSync(newest, "utf8")
    .split(/\r?\n/)
    .filter((l) => l.includes('"SCENARIO_EXIT"'))
    .map((l) => JSON.parse(l))
    .filter((e) => e.data.callDepth === 0)
    .map((e) => e.timeStamp);
  return Math.max(...exits);
}

async function round() {
  const data = mkdtempSync(path.join(os.tmpdir(), "vl-data-"));
  writeFileSync(
    path.join(data, "settings.json"),
    JSON.stringify({
      lastProject: project,
      updates: { checkAtStartup: false },
      window: { ...AT, width: 1400, height: 900, maximized: false },
    }),
  );
  const t0 = Date.now();
  const child = spawn(exe, [], { env: { ...process.env, GRACE_USER_DATA: data }, stdio: "ignore" });
  const pid = child.pid;
  const out = {};
  try {
    const stopOpen = gpui ? ping(pid) : null;
    const shown = await p("waitname", pid, "Collapse tests", 120000); // the project's top folder, open at start
    if (!shown?.at) throw new Error("the tree never showed");
    out.openMs = shown.at - t0;
    await sleep(2000);
    if (stopOpen) out.openStall = (await stopOpen()).max;
    out.openMemMB = Math.round((await sample(pid)).app.privateMB);
    if (mode === "dump") {
      console.log(await p("dump", pid));
      return out;
    }
    // GPUI Grace on develop opens a window saved on the second screen on the main one (#316): both are placed alike
    await placeWindow(pid, { ...AT, width: 1400, height: 900 });
    await sleep(1000);
    await p("front", pid);
    const tree = await p("rect", pid, "Tests"); // the tree, named "Tests" in both apps
    const region = [tree.x, tree.y, tree.w, Math.min(tree.h, 300)];
    const folder = await p("rect", pid, "Collapse ", "contains"); // the first open folder's chevron
    const cx = folder.x + (folder.w >> 1), cy = folder.y + (folder.h >> 1);
    out.folderClose = (await p("clickpaint", cx, cy, ...region)).ms;
    await sleep(500);
    out.folderOpen = (await p("clickpaint", cx, cy, ...region)).ms;
    await sleep(500);
    // a test row: open the first feature, then click its first test
    const feature = await p("rect", pid, "Expand ", "contains");
    await p("click", feature.x + (feature.w >> 1), feature.y + (feature.h >> 1));
    await sleep(1000);
    const rowY = feature.y + feature.h + (feature.h >> 1) + 2; // the row under the feature
    out.select = (await p("clickpaint", feature.x + 120, rowY, ...region)).ms;
    await sleep(500);
    out.down = (await p("keypaint", 0x28, ...region)).ms;
    await sleep(1000);

    // Run all
    const runAll = await p("rect", pid, "Run all");
    const sampler = startSampler(pid);
    await sampler.ready;
    const stopRun = gpui ? ping(pid) : null;
    await p("click", runAll.x + (runAll.w >> 1), runAll.y + (runAll.h >> 1));
    await sleep(5000);
    // scroll the tree during the run, with its own stall reading
    const stopScroll = gpui ? ping(pid) : null;
    await p("wheel", tree.x + (tree.w >> 1), tree.y + (tree.h >> 1), 60, 16);
    if (stopScroll) out.scrollStall = (await stopScroll()).max;
    const fin = await p("waitname", pid, "Finished", 15 * 60000);
    if (!fin?.at) throw new Error("no Finished");
    if (stopRun) out.runStall = (await stopRun()).max;
    const peaks = await sampler.stop();
    Object.assign(out, {
      runMemApp: peaks.appPeakMB,
      runMemJava: peaks.javaPeakMB,
      runCpuApp: peaks.appCpuMs,
      runCpuJava: peaks.javaCpuMs,
    });
    out.finishedLag = fin.at - lastExit([data, project]);
    return out;
  } finally {
    await killTree(pid, t0).catch((e) => console.error(e.message));
    rmSync(data, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
  }
}

const rounds = [];
for (let i = 0; i < Number(roundsArg); i++) {
  const r = await round();
  console.error(JSON.stringify(r));
  rounds.push(r);
}
const keys = [...new Set(rounds.flatMap(Object.keys))];
console.log(
  JSON.stringify({ exe, project, gpui, rounds, medians: Object.fromEntries(keys.map((k) => [k, median(rounds.map((r) => r[k] ?? null))])) }),
);
