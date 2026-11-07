// Idle memory (USS) and CPU of each stack's empty window: settle 20 s, then 60 s, 3 rounds, alternating.
//   node floor.mjs --electron-bin <electron> --gpui-floor <floor exe>
import { spawn } from "node:child_process";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { parseArgs } from "node:util";
import { killTree, sample } from "./measure.mjs";

const { values: o } = parseArgs({ strict: false, options: { "electron-bin": { type: "string" }, "gpui-floor": { type: "string" } } });
const dir = path.dirname(new URL(import.meta.url).pathname);
const env = { ...process.env };
delete env.JAVA_TOOL_OPTIONS;
const start = {
  electron: () => spawn(o["electron-bin"], ["--no-sandbox", path.join(dir, "electron-floor")], { env, stdio: "ignore" }),
  gpui: () => spawn(o["gpui-floor"], [], { env, stdio: "ignore" }),
};
const out = [];
for (let r = 0; r < 3; r++)
  for (const name of r % 2 ? ["gpui", "electron"] : ["electron", "gpui"]) {
    const c = start[name]();
    await sleep(20_000);
    const a = sample(c.pid);
    await sleep(60_000);
    const b = sample(c.pid);
    out.push({ name, memMB: b.appMB, cpuMs: b.appCpu - a.appCpu, procs: b.procs });
    console.log(JSON.stringify(out.at(-1)));
    killTree(c.pid);
    await sleep(1000);
  }
