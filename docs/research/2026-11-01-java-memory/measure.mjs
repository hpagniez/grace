// Spike for #149: runs Karate the way Grace does, with extra JVM flags, and prints one JSON line per run:
// Java's peak private working set and CPU (bench/system.mjs), and the heap before, after and committed at each GC.
//   node --no-warnings measure.mjs <repo> <2k|10k> <threads> <label> [--dry] -- <jvm flags...>
// The projects and the output go under the system temp folder.
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";

const [repo, size, threadsArg, label, ...rest] = process.argv.slice(2);
const dry = rest.includes("--dry");
const flags = rest.slice(rest.indexOf("--") + 1);
const here = path.join(os.tmpdir(), "grace-java-memory");
const { startSampler } = await import(pathToFileURL(path.join(repo, "bench/system.mjs")).href);
const { writeBigProject } = await import(pathToFileURL(path.join(repo, "e2e/big-project.ts")).href);

const root = path.join(here, `project-${size}`);
if (!existsSync(root)) writeBigProject(root, size === "10k" ? 25 : 5);
const out = path.join(here, "out");
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
const gcLog = path.join(out, "gc.log");
const java = path.join(repo, "vendor/jre/bin/java.exe");
const args = [
  ...flags,
  `-Xlog:gc:file=${gcLog.replaceAll("\\", "/")}`,
  "-Dstdout.encoding=UTF-8",
  "-Dstderr.encoding=UTF-8",
  "-cp",
  [path.join(repo, "vendor/karate.jar"), root].join(";"),
  "io.karatelabs.Main",
  "run",
  "--no-color",
  "-w",
  root,
  "-o",
  out,
  "-B",
  "false",
  "-f",
  "html,karate:jsonl",
  ...(dry ? ["-D", "-T", "1"] : ["-T", threadsArg]),
  root,
];
const env = Object.fromEntries(
  Object.entries(process.env).filter(
    ([k]) => !/^(karate_options|karate_env|karate_config_dir|java_tool_options|jdk_java_options|_java_options)$/i.test(k),
  ),
);
const t0 = Date.now();
const child = spawn(java, args, { cwd: root, env, windowsHide: true });
let text = "";
child.stdout.on("data", (d) => (text += d));
child.stderr.on("data", (d) => (text += d));
const sampler = startSampler(child.pid, 250);
await sampler.ready.catch(() => {});
const code = await new Promise((r) => child.on("exit", r));
const wallMs = Date.now() - t0;
const s = await sampler.stop().catch((e) => ({ error: String(e) }));
const gc = existsSync(gcLog) ? readFileSync(gcLog, "utf8") : "";
const pauses = [...gc.matchAll(/(\d+)M->(\d+)M\((\d+)M\)/g)].map((m) => m.slice(1).map(Number));
const afterGc = pauses.map((p) => p[1]);
const scen = text.match(/scenarios:\s*(\d+)\s*\|\s*passed:\s*(\d+)\s*\|\s*failed:\s*(\d+)/i);
console.log(
  JSON.stringify({
    label,
    size,
    threads: dry ? 1 : Number(threadsArg),
    dry,
    exit: code,
    oom: /OutOfMemoryError/.test(text),
    wallMs,
    javaPeakMB: s.javaPeakMB,
    javaCpuMs: s.javaCpuMs,
    gcs: pauses.length,
    maxAfterGcMB: afterGc.length ? Math.max(...afterGc) : null,
    maxCommittedMB: pauses.length ? Math.max(...pauses.map((p) => p[2])) : null,
    scenarios: scen ? scen.slice(1).join("/") : null,
    error: s.error,
  }),
);
if (/OutOfMemoryError/.test(text) || code > 1)
  console.error(text.split("\n").filter((l) => /Error|Exception/.test(l)).slice(0, 5).join("\n"));
