// VL-27's HTTP half, for the P3 exit check: the time from a click on the HTTP tab to the first changed pixel of the
// bottom panel, after one Run all of the generated 10,000-test project (3,000 HTTP calls). It drives the app from
// outside as docs/checks/2026-11-10-gpui-p2-exit/vl.mjs does, through that folder's uiprobe.exe (built from its
// uiprobe.cs with the .NET Framework's csc.exe). Run from the repository root:
//
//   node --no-warnings docs/checks/2026-11-23-gpui-p3-exit/vl-http.mjs <uiprobe.exe> <Grace.exe> <project> <rounds>
//
// <project> is a folder this script fills with writeBigProject (25 features per module). Prints the rounds as JSON.
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { execFile } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";
import { killTree } from "../../../bench/app.mjs";
import { median } from "../../../bench/stats.mjs";
import { placeWindow } from "../../../bench/system.mjs";
import { writeBigProject } from "../../../e2e/big-project.ts";

const run = promisify(execFile);
const [probe, exe, project, roundsArg] = process.argv.slice(2);
const AT = { x: 80, y: 60, width: 1400, height: 900 };

async function p(...args) {
  const { stdout } = await run(probe, args.map(String), { maxBuffer: 64 << 20 });
  return stdout.trim() === "null" ? null : stdout.startsWith("{") ? JSON.parse(stdout) : stdout;
}

const sizes = writeBigProject(project, 25);
console.error(JSON.stringify(sizes));

async function round() {
  const data = mkdtempSync(path.join(os.tmpdir(), "vl-http-data-"));
  writeFileSync(
    path.join(data, "settings.json"),
    JSON.stringify({ lastProject: project, updates: { checkAtStartup: false }, window: { ...AT, maximized: false } }),
  );
  const t0 = Date.now();
  const child = spawn(exe, [], { env: { ...process.env, GRACE_USER_DATA: data }, stdio: "ignore" });
  const pid = child.pid;
  try {
    const shown = await p("waitname", pid, "Collapse tests", 120000);
    if (!shown?.at) throw new Error("the tree never showed");
    await sleep(2000);
    await placeWindow(pid, AT);
    await sleep(1000);
    await p("front", pid);
    const runAll = await p("rect", pid, "Run all");
    await p("click", runAll.x + (runAll.w >> 1), runAll.y + (runAll.h >> 1));
    const fin = await p("waitname", pid, "Finished", 15 * 60000);
    if (!fin?.at) throw new Error("no Finished");
    await sleep(2000);
    const win = await p("winrect", pid);
    const out = { httpTab: [], console: [] };
    for (let i = 0; i < 3; i++) {
      for (const [name, key] of [["HTTP", "httpTab"], ["Console", "console"]]) {
        const tab = await p("rect", pid, name, "contains");
        const region = [win.x, tab.y + tab.h, win.w, 250];
        out[key].push((await p("clickpaint", tab.x + (tab.w >> 1), tab.y + (tab.h >> 1), ...region)).ms);
        await sleep(500);
      }
    }
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
const all = (k) => rounds.flatMap((r) => r[k]);
console.log(JSON.stringify({ project: sizes, rounds, medians: { httpTab: median(all("httpTab")), console: median(all("console")) } }));
