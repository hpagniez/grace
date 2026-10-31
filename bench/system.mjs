// What the bench reads off a Windows machine: memory and CPU of the app's process tree, the machine's load, the
// window's state and a folder's size (docs/specs/2026-10-31-light-and-fast.md). Measuring must not load the machine,
// so each question is one PowerShell call and the sampler is one long-lived loop, never a PowerShell per sample.
import { execFile, spawn } from "node:child_process";
import { readdirSync, statSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import readline from "node:readline";
import { setTimeout as sleep } from "node:timers/promises";
import { promisify } from "node:util";

/** @typedef {{ privateMB: number, cpuMs: number }} Usage */
/** @typedef {{ app: Usage, java: Usage }} Sample `java.exe` and the `conhost.exe` under it are `java`, everything else is `app`. */
/** @typedef {{ id: number, parent: number, name: string, bytes: number, cpu: number }} Proc One process: its private working set in bytes and its CPU time in 100 ns ticks. */

const JAVA = /^java\.exe$/i;
const CONHOST = /^conhost\.exe$/i;
const PS_FLAGS = ["-NoProfile", "-NonInteractive", "-Command"];
const execFileAsync = promisify(execFile);
/** A cold PowerShell and its first CIM call take seconds on a loaded machine. */
const FIRST_SAMPLE_MS = 60_000;
/** How long `cpuLoad` watches the machine. */
const LOAD_WINDOW_MS = 1_000;

/** Runs `script` in a fresh PowerShell and gives its stdout; a failure carries PowerShell's own message, not the script. */
async function powershell(script) {
  try {
    const { stdout } = await execFileAsync("powershell.exe", [...PS_FLAGS, script], {
      encoding: "utf8",
      windowsHide: true,
    });
    return stdout;
  } catch (e) {
    throw new Error(`powershell failed: ${e.stderr?.trim() || e.message}`);
  }
}

/** PowerShell that defines `Measure-Tree`: one JSON line, the processes of the tree under `rootPid` (a `Proc` each). */
function treeScript(rootPid) {
  if (!Number.isInteger(rootPid)) throw new TypeError(`not a process id: ${rootPid}`);
  return `
function Measure-Tree {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name, KernelModeTime, UserModeTime
  $ids = [System.Collections.Generic.HashSet[int]]::new(); [void]$ids.Add(${rootPid})
  do { $n = $ids.Count; foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } } } while ($ids.Count -ne $n)
  # The raw class holds the same WorkingSetPrivate as the formatted one (Task Manager's Memory column) for a fifth of the cost.
  $private = @{}
  Get-CimInstance Win32_PerfRawData_PerfProc_Process -Filter (($ids | ForEach-Object { "IDProcess=$_" }) -join ' OR ') -Property IDProcess, WorkingSetPrivate |
    ForEach-Object { $private[[int]$_.IDProcess] = [long]$_.WorkingSetPrivate }
  $tree = @($all | Where-Object { $ids.Contains([int]$_.ProcessId) } | ForEach-Object {
    [pscustomobject]@{ id = [int]$_.ProcessId; parent = [int]$_.ParentProcessId; name = $_.Name; bytes = [long]$private[[int]$_.ProcessId]; cpu = [long]($_.KernelModeTime + $_.UserModeTime) }
  })
  ConvertTo-Json -Compress -InputObject $tree
}
`;
}

/**
 * Splits a process tree into the Java side and the app side. `java.exe` is Java, and so is a `conhost.exe` whose parent
 * is a `java.exe` of the tree: Windows gives a console process a conhost, and only Java's belongs to Java. Any other
 * conhost, such as one under a console tool the app started, is the app's.
 * @param {Proc[]} procs
 * @returns {Sample}
 */
export function splitTree(procs) {
  const javaIds = new Set(procs.filter((p) => JAVA.test(p.name)).map((p) => p.id));
  const sums = { app: { bytes: 0, cpu: 0 }, java: { bytes: 0, cpu: 0 } };
  for (const p of procs) {
    const isJava = javaIds.has(p.id) || (CONHOST.test(p.name) && javaIds.has(p.parent));
    const sum = isJava ? sums.java : sums.app;
    sum.bytes += p.bytes;
    sum.cpu += p.cpu;
  }
  const usage = ({ bytes, cpu }) => ({
    privateMB: Math.round(bytes / 1e5) / 10,
    cpuMs: Math.round(cpu / 1e4),
  });
  return { app: usage(sums.app), java: usage(sums.java) };
}

/**
 * The sample in one `Measure-Tree` line, or null when `rootPid` is not in it: the process is gone, and zeros would
 * read as a tree that uses no memory.
 * @param {string} line
 * @param {number} rootPid
 * @returns {Sample | null}
 */
function parse(line, rootPid) {
  const procs = [JSON.parse(line)].flat();
  return procs.some((p) => p.id === rootPid) ? splitTree(procs) : null;
}

/**
 * Private working set (MB of 10^6 bytes) and CPU time (kernel plus user, ms) of the process tree under `rootPid`.
 * Rejects when `rootPid` is not running.
 * @param {number} rootPid
 * @returns {Promise<Sample>}
 */
export async function sample(rootPid) {
  const out = await powershell(
    `$ErrorActionPreference = 'Stop'\n${treeScript(rootPid)}\nMeasure-Tree`,
  );
  const s = parse(out, rootPid);
  if (!s) throw new Error(`process ${rootPid} is not running`);
  return s;
}

/**
 * Samples the tree under `rootPid` every `everyMs` from one PowerShell loop. `stop()` ends the loop, waiting for the
 * first sample if none came yet, and gives the peak private memory seen. It rejects, rather than give a peak that
 * under-reports, when `rootPid` was not running at the first sample, no sample came in a minute, or the sampler's
 * PowerShell died before `stop()`. A root that exits later just ends the samples: the peaks so far stand.
 * @param {number} rootPid
 * @param {number} [everyMs]
 * @returns {{ stop(): Promise<{ appPeakMB: number, javaPeakMB: number }> }}
 */
export function startSampler(rootPid, everyMs = 500) {
  if (!Number.isInteger(everyMs) || everyMs < 1)
    throw new TypeError(`not a period in ms: ${everyMs}`);
  const ps = spawn(
    "powershell.exe",
    [
      ...PS_FLAGS,
      `${treeScript(rootPid)}
while ($true) {
  $tick = [Diagnostics.Stopwatch]::StartNew()
  [Console]::Out.WriteLine((Measure-Tree))
  $rest = ${everyMs} - $tick.ElapsedMilliseconds
  if ($rest -gt 0) { Start-Sleep -Milliseconds $rest }
}`,
    ],
    { stdio: ["ignore", "pipe", "ignore"], windowsHide: true },
  );
  const exited = new Promise((resolve) => {
    ps.once("exit", resolve);
    ps.once("error", resolve);
  });
  const first = Promise.withResolvers();
  first.promise.catch(() => {}); // `stop` reports it; until then it is not unhandled
  const fail = (/** @type {Error} */ e) => {
    clearTimeout(late);
    first.reject(e);
  };
  const late = setTimeout(
    () => fail(new Error(`no sample in ${FIRST_SAMPLE_MS / 1000} s`)),
    FIRST_SAMPLE_MS,
  );
  ps.once("error", fail);
  ps.once("exit", (code) =>
    fail(new Error(`the sampler exited (${code}) before its first sample`)),
  );

  let appPeakMB = 0;
  let javaPeakMB = 0;
  let sampled = false;
  readline.createInterface({ input: ps.stdout }).on("line", (line) => {
    const s = parse(line, rootPid);
    if (!s) {
      if (!sampled) fail(new Error(`process ${rootPid} is not running`));
      return;
    }
    appPeakMB = Math.max(appPeakMB, s.app.privateMB);
    javaPeakMB = Math.max(javaPeakMB, s.java.privateMB);
    sampled = true;
    clearTimeout(late);
    first.resolve();
  });

  return {
    async stop() {
      const died = ps.exitCode !== null || ps.signalCode !== null;
      try {
        if (died) throw new Error("the sampler died");
        await first.promise;
      } finally {
        ps.kill();
        await exited;
      }
      return { appPeakMB, javaPeakMB };
    },
  };
}

/** Busy and total CPU time of every core since boot, ms. Windows' kernel time (idle included) is split into sys and idle. */
function cpuTimes() {
  let busy = 0;
  let total = 0;
  for (const { times } of os.cpus()) {
    busy += times.user + times.sys;
    total += times.user + times.sys + times.idle;
  }
  return { busy, total };
}

/**
 * The machine's CPU use, 0 to 100, over the last second. Read in-process: no PowerShell to load the machine it asks about.
 * @returns {Promise<number>}
 */
export async function cpuLoad() {
  const before = cpuTimes();
  await sleep(LOAD_WINDOW_MS);
  const after = cpuTimes();
  return (100 * (after.busy - before.busy)) / (after.total - before.total);
}

/**
 * Reads the load every `everyMs` until it is under `under` percent, or `maxMs` pass. Gives the last reading either way:
 * a caller that must have a quiet machine checks `load`.
 * @param {{ under?: number, maxMs?: number, everyMs?: number, read?: () => Promise<number> }} [opts]
 * @returns {Promise<{ load: number, waitedMs: number }>}
 */
export async function waitForQuiet({
  under = 35,
  maxMs = 300_000,
  everyMs = 2_000,
  read = cpuLoad,
} = {}) {
  const start = Date.now();
  for (;;) {
    const load = await read();
    const waitedMs = Date.now() - start;
    if (load < under || waitedMs >= maxMs) return { load, waitedMs };
    await sleep(everyMs);
  }
}

/**
 * Minimises the main window of process `pid` (SW_MINIMIZE). Rejects when the process has none: CDP cannot reach the main
 * process, so this is a Win32 call, and silently leaving the window up would measure "minimised" CPU on a shown window.
 * @param {number} pid
 * @returns {Promise<void>}
 */
export async function minimise(pid) {
  if (!Number.isInteger(pid)) throw new TypeError(`not a process id: ${pid}`);
  await powershell(`
Add-Type -Namespace Win32 -Name User32 -MemberDefinition '[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);'
$handle = (Get-Process -Id ${pid}).MainWindowHandle
if ($handle -eq 0) { [Console]::Error.WriteLine('process ${pid} has no window'); exit 1 }
[void][Win32.User32]::ShowWindow($handle, 6)`);
}

/**
 * Size of the files under `dir`, in MB of 10^6 bytes to one decimal.
 * @param {string} dir
 * @returns {number}
 */
export function folderMB(dir) {
  let bytes = 0;
  for (const entry of readdirSync(dir, { recursive: true, withFileTypes: true }))
    if (entry.isFile()) bytes += statSync(path.join(entry.parentPath, entry.name)).size;
  return Math.round(bytes / 1e5) / 10;
}
