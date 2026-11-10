// The cursor rows of the 2026-11-10 GPUI P2 exit check (UP-13, UP-14): opens Grace on the clean sample project, moves the
// pointer over each control and reads the cursor's shape (uiprobe.exe `cursor`), then again during Run all.
//
//   node --no-warnings docs/checks/2026-11-10-gpui-p2-exit/cursor.mjs <probe.exe> <exe> <project>
import { execFile, spawn } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";
import { killTree } from "../../../bench/app.mjs";

const run = promisify(execFile);
const [probe, exe, project] = process.argv.slice(2);
const p = async (...args) => {
  const { stdout } = await run(probe, args.map(String), { maxBuffer: 64 << 20 }).catch((e) => e);
  return stdout?.startsWith("{") ? JSON.parse(stdout) : null;
};

// [what, accessible name, match by "contains"]
const CONTROLS = [
  ["Run all (enabled)", "Run all"],
  ["Cancel (disabled)", "Cancel"],
  ["Report (disabled)", "Report"],
  ["Properties…", "Properties", true],
  ["Open…", "Open", true],
  ["Refresh", "Refresh", true],
  ["Env box", "Env", true],
  ["Threads box", "Threads", true],
  ["tag chip @slow", "@slow"],
  ["Hide non-matching", "Hide non-matching"],
  ["tree row basic.feature", "basic.feature, ", true],
  ["row run button", "Run basic.feature"],
  ["Console tab", "Console"],
  ["History tab", "History", true],
  ["Settings", "Settings"],
];

const data = mkdtempSync(path.join(os.tmpdir(), "cursor-data-"));
writeFileSync(
  path.join(data, "settings.json"),
  JSON.stringify({
    lastProject: project,
    updates: { checkAtStartup: false },
    window: { x: 100, y: 60, width: 1400, height: 900, maximized: false },
  }),
);
const t0 = Date.now();
const child = spawn(exe, [], { env: { ...process.env, GRACE_USER_DATA: data }, stdio: "ignore" });
const pid = child.pid;
const read = async (label) => {
  const out = {};
  for (const [what, name, contains] of CONTROLS) {
    const r = await p("rect", pid, name, ...(contains ? ["contains"] : []));
    if (!r) {
      out[what] = "not found";
      continue;
    }
    // a row: a point on its label, left of the buttons; a control: its middle
    const x = what.startsWith("tree row") ? r.x + 140 : r.x + (r.w >> 1);
    out[what] = (await p("cursor", x, r.y + (r.h >> 1)))?.cursor;
  }
  // the splitter above the bottom panel: the line where the Console tab's strip starts, a few pixels up
  const tab = await p("rect", pid, "Console");
  if (tab) out["panel splitter"] = (await p("cursor", tab.x + 300, tab.y - 6))?.cursor;
  const details = await p("rect", pid, "Select a test to see its details", "contains");
  if (details) out["empty details"] = (await p("cursor", details.x, details.y + 60))?.cursor;
  console.log(JSON.stringify({ [label]: out }, null, 1));
};
try {
  if (!(await p("waitname", pid, "Collapse tests", 60000))?.at) throw new Error("no tree");
  await sleep(1500);
  await p("front", pid);
  await read("idle");
  const runAll = await p("rect", pid, "Run all");
  await p("click", runAll.x + (runAll.w >> 1), runAll.y + (runAll.h >> 1));
  await sleep(4000); // Java started, slow loop running
  await read("running");
  await p("waitname", pid, "Finished", 120000);
} finally {
  await killTree(pid, t0).catch((e) => console.error(e.message));
  rmSync(data, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
}
