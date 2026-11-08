// Measures Grace (Electron) and the GPUI prototype the same way on Linux, in alternating rounds: start, idle, Run all.
// A Linux port of the parts of bench/bench.mjs that do not need Windows: memory is USS (private clean + private dirty
// pages from /proc/<pid>/smaps_rollup, the Linux match of Windows' private working set), CPU is utime + stime from
// /proc/<pid>/stat, both summed over the app's process tree and split into the app and Java.
//
//   node measure.mjs --electron <Grace exe> --gpui <grace-gpui exe> --project <dir> --java <java> --jar <karate.jar>
//                    [--rounds 5] [--out results.json]
//
// Needs DISPLAY (Xvfb) and playwright (for Electron's CDP, as the bench uses it). Run from a folder whose
// node_modules has playwright, e.g. a Grace checkout.
import { spawn } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { parseArgs } from "node:util";
import { chromium } from "playwright";

const { values: opt } = parseArgs({
  strict: false,
  options: {
    electron: { type: "string" },
    gpui: { type: "string" },
    project: { type: "string" },
    java: { type: "string" },
    jar: { type: "string" },
    rounds: { type: "string", default: "5" },
    out: { type: "string", default: "results.json" },
    "settle-ms": { type: "string", default: "20000" },
    "idle-ms": { type: "string", default: "60000" },
  },
});
const ROUNDS = Number(opt.rounds);
const SETTLE_MS = Number(opt["settle-ms"]);
const IDLE_MS = Number(opt["idle-ms"]);
const TICK = 100; // USER_HZ on Linux x86_64
const PAGE = 4096;

const log = (m) => console.error(`[measure ${new Date().toTimeString().slice(0, 8)}] ${m}`);

// ---- /proc ----

function procs() {
  /** @type {Map<number, { ppid: number, comm: string, cpuMs: number }>} */
  const all = new Map();
  for (const d of readdirSync("/proc")) {
    if (!/^\d+$/.test(d)) continue;
    try {
      const s = readFileSync(`/proc/${d}/stat`, "utf8");
      const close = s.lastIndexOf(")");
      const comm = s.slice(s.indexOf("(") + 1, close);
      const f = s.slice(close + 2).split(" ");
      // fields after comm: state(0) ppid(1) ... utime(11) stime(12)
      all.set(Number(d), { ppid: Number(f[1]), comm, cpuMs: ((Number(f[11]) + Number(f[12])) * 1000) / TICK });
    } catch {
      // gone
    }
  }
  return all;
}

function tree(root) {
  const all = procs();
  const out = [];
  const want = new Set([root]);
  let grew = true;
  while (grew) {
    grew = false;
    for (const [pid, p] of all)
      if (!want.has(pid) && want.has(p.ppid)) {
        want.add(pid);
        grew = true;
      }
  }
  for (const pid of want) if (all.has(pid)) out.push({ pid, ...all.get(pid) });
  return out;
}

function ussMB(pid) {
  try {
    const t = readFileSync(`/proc/${pid}/smaps_rollup`, "utf8");
    const kb = (k) => Number(new RegExp(`^${k}:\\s+(\\d+) kB`, "m").exec(t)?.[1] ?? 0);
    return (kb("Private_Clean") + kb("Private_Dirty")) / 1024;
  } catch {
    return 0;
  }
}

const isJava = (p) => /^java(\.exe)?$/.test(p.comm);

/** Memory (USS, MB) and CPU (ms) of the tree under `root`, split into app and Java. */
function sample(root) {
  const t = tree(root);
  const r = { appMB: 0, javaMB: 0, appCpu: 0, javaCpu: 0, procs: t.length };
  for (const p of t) {
    const mb = ussMB(p.pid);
    if (isJava(p)) {
      r.javaMB += mb;
      r.javaCpu += p.cpuMs;
    } else {
      r.appMB += mb;
      r.appCpu += p.cpuMs;
    }
  }
  return r;
}

/**
 * CPU of processes that exit during a run (Java) leaves the tree with them, so the sampler keeps each pid's last
 * reading and sums those: a pid's CPU only grows.
 */
function startSampler(root, everyMs = 250) {
  /** @type {Map<number, { java: boolean, cpu: number, base: number }>} */
  const seen = new Map();
  let appPeak = 0;
  let javaPeak = 0;
  let stopped = false;
  const first = tree(root);
  for (const p of first) seen.set(p.pid, { java: isJava(p), cpu: p.cpuMs, base: p.cpuMs });
  const tick = () => {
    if (stopped) return;
    let app = 0;
    let java = 0;
    for (const p of tree(root)) {
      const s = seen.get(p.pid) ?? { java: isJava(p), cpu: 0, base: 0 };
      s.cpu = p.cpuMs;
      s.java ||= isJava(p);
      seen.set(p.pid, s);
      const mb = ussMB(p.pid);
      if (s.java) java += mb;
      else app += mb;
    }
    appPeak = Math.max(appPeak, app);
    javaPeak = Math.max(javaPeak, java);
    timer = setTimeout(tick, everyMs);
  };
  let timer = setTimeout(tick, 0);
  return {
    stop() {
      stopped = true;
      clearTimeout(timer);
      tick.call(null);
      let appCpu = 0;
      let javaCpu = 0;
      for (const s of seen.values()) {
        if (s.java) javaCpu += s.cpu - s.base;
        else appCpu += s.cpu - s.base;
      }
      return { appPeakMB: appPeak, javaPeakMB: javaPeak, appCpuMs: appCpu, javaCpuMs: javaCpu };
    },
  };
}

// ---- apps ----

function killTree(root) {
  for (const p of tree(root).reverse())
    try {
      process.kill(p.pid, "SIGKILL");
    } catch {
      // gone
    }
}

const ENV = { ...process.env };
delete ENV.JAVA_TOOL_OPTIONS; // both apps remove it from Java's environment anyway

async function freePort() {
  const net = await import("node:net");
  return new Promise((res) => {
    const s = net.createServer().listen(0, () => {
      const { port } = /** @type {any} */ (s.address());
      s.close(() => res(port));
    });
  });
}

/** Electron Grace, as bench/app.mjs starts it: a fresh data folder whose settings open the project. */
async function startElectron({ cdp }) {
  const dir = mkdtempSync(path.join(os.tmpdir(), "m-electron-"));
  writeFileSync(
    path.join(dir, "settings.json"),
    JSON.stringify({
      lastProject: opt.project,
      updates: { checkAtStartup: false },
      window: { x: 0, y: 0, width: 1400, height: 900, maximized: false },
    }),
  );
  const port = cdp ? await freePort() : null;
  const t0 = Date.now();
  const child = spawn(opt.electron, ["--no-sandbox", ...(port ? [`--remote-debugging-port=${port}`] : [])], {
    env: { ...ENV, GRACE_USER_DATA: dir },
    stdio: "ignore",
  });
  let browser = null;
  let page = null;
  if (port) {
    for (;;) {
      try {
        browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`, { timeout: 5000 });
        break;
      } catch {
        if (Date.now() - t0 > 60_000) throw new Error("no CDP");
        await sleep(25);
      }
    }
    while (!(page = browser.contexts()[0]?.pages()[0])) await sleep(20);
  }
  return {
    pid: child.pid,
    t0,
    page,
    async times() {
      let treeAt = null;
      while (treeAt === null) {
        if (Date.now() - t0 > 120_000) throw new Error("no tree");
        const n = await page
          .evaluate(() => document.querySelectorAll("[data-testid=tree-node]").length)
          .catch(() => 0);
        if (n > 0) treeAt = Date.now();
        else await sleep(20);
      }
      const paintAt = await page.evaluate(() => {
        const e = performance.getEntriesByName("first-contentful-paint")[0];
        return e ? performance.timeOrigin + e.startTime : null;
      });
      return { paint: Math.round(paintAt - t0), tree: treeAt - t0 };
    },
    async waitTree() {
      while (!(await page.evaluate(() => document.querySelectorAll("[data-testid=tree-node]").length).catch(() => 0)))
        await sleep(100);
      // and the Run all button enabled
      await page.waitForFunction(() => !document.querySelector("[data-testid=run-all-button]")?.disabled, null, {
        timeout: 120_000,
      });
    },
    async startRun() {
      await page.getByTestId("run-all-button").click();
      await page.waitForFunction(
        () => /^(Running|Finished|Cancelled|Run failed)/.test(document.querySelector("[data-testid=run-outcome]")?.textContent?.trim() ?? ""),
        null,
        { timeout: 60_000, polling: 100 },
      );
    },
    async waitRunEnd() {
      await page.waitForFunction(
        () => /^(Finished|Cancelled|Run failed)/.test(document.querySelector("[data-testid=run-outcome]")?.textContent?.trim() ?? ""),
        null,
        { timeout: 30 * 60_000, polling: 250 },
      );
    },
    async run() {
      await page.getByTestId("run-all-button").click();
      await page.waitForFunction(
        () => /^(Running|Finished|Cancelled|Run failed)/.test(document.querySelector("[data-testid=run-outcome]")?.textContent?.trim() ?? ""),
        null,
        { timeout: 60_000, polling: 100 },
      );
      await page.waitForFunction(
        () => /^(Finished|Cancelled|Run failed)/.test(document.querySelector("[data-testid=run-outcome]")?.textContent?.trim() ?? ""),
        null,
        { timeout: 30 * 60_000, polling: 250 },
      );
      return page.evaluate(() => document.querySelector("[data-testid=run-outcome]")?.textContent?.trim());
    },
    async stop() {
      killTree(child.pid);
      await browser?.close().catch(() => {});
      rmSync(dir, { recursive: true, force: true });
    },
  };
}

/** The prototype, which prints PROBE lines on stderr and takes "run" on stdin. */
async function startGpui() {
  const dir = mkdtempSync(path.join(os.tmpdir(), "m-gpui-"));
  const t0 = Date.now();
  const child = spawn(opt.gpui, [opt.project], {
    env: { ...ENV, GRACE_PROBE: "1", GRACE_USER_DATA: dir, GRACE_JAVA: opt.java, GRACE_KARATE_JAR: opt.jar },
    stdio: ["pipe", "ignore", "pipe"],
  });
  /** @type {Map<string, number>} */
  const probes = new Map();
  let lastFrames = "";
  /** @type {Map<string, () => void>} */
  const waiting = new Map();
  let buf = "";
  child.stderr.on("data", (d) => {
    buf += d;
    let i;
    while ((i = buf.indexOf("\n")) >= 0) {
      const line = buf.slice(0, i);
      buf = buf.slice(i + 1);
      const m = /^PROBE (\S+) (\d+)/.exec(line);
      if (m?.[1] === "frames") lastFrames = line.split(" ")[3] ?? "";
      if (m) {
        probes.set(m[1], Number(m[2]));
        waiting.get(m[1])?.();
      }
    }
  });
  const until = (name, ms = 120_000) =>
    probes.has(name)
      ? Promise.resolve(probes.get(name))
      : new Promise((res, rej) => {
          const t = setTimeout(() => rej(new Error(`no ${name} in ${ms} ms`)), ms);
          waiting.set(name, () => {
            clearTimeout(t);
            waiting.delete(name);
            res(probes.get(name));
          });
        });
  return {
    pid: child.pid,
    t0,
    async times() {
      const paint = await until("first_frame");
      const treeAt = await until("tree_shown");
      return { paint: paint - t0, tree: treeAt - t0 };
    },
    async waitTree() {
      await until("tree_shown");
    },
    send(cmd) {
      child.stdin.write(`${cmd}\n`);
    },
    /** The frame probe's gaps (ms), after "frames-stop". */
    async frames() {
      probes.delete("frames");
      child.stdin.write("frames-stop\n");
      await until("frames", 10_000);
      return lastFrames ? lastFrames.split(",").map(Number) : [];
    },
    async startRun() {
      probes.delete("run_finished");
      probes.delete("run_started");
      child.stdin.write("run\n");
      await until("run_started", 60_000);
    },
    async waitRunEnd() {
      await until("run_finished", 30 * 60_000);
    },
    async run() {
      probes.delete("run_finished");
      child.stdin.write("run\n");
      await until("run_started", 60_000);
      await until("run_finished", 30 * 60_000);
      return "finished";
    },
    async stop() {
      killTree(child.pid);
      rmSync(dir, { recursive: true, force: true });
    },
  };
}

const APPS = {
  electron: (o) => startElectron(o),
  gpui: () => startGpui(),
};

// ---- phases ----

async function startPhase(name) {
  const app = await APPS[name]({ cdp: true });
  try {
    return await app.times();
  } finally {
    await app.stop();
  }
}

async function idlePhase(name) {
  const app = await APPS[name]({ cdp: false });
  try {
    await sleep(SETTLE_MS);
    const a = sample(app.pid);
    const s = startSampler(app.pid, 1000);
    await sleep(IDLE_MS);
    const r = s.stop();
    const b = sample(app.pid);
    return { memMB: b.appMB, javaMB: b.javaMB, cpuMs: r.appCpuMs, procs: b.procs, memAtSettleMB: a.appMB };
  } finally {
    await app.stop();
  }
}

async function runPhase(name) {
  const app = await APPS[name]({ cdp: true });
  try {
    await app.waitTree();
    await sleep(2000);
    const s = startSampler(app.pid, 250);
    const t = Date.now();
    const outcome = await app.run();
    const wall = Date.now() - t;
    const r = s.stop();
    await sleep(3000);
    const after = sample(app.pid);
    return { ...r, wallMs: wall, outcome, appAfterMB: after.appMB };
  } finally {
    await app.stop();
  }
}

// ---- main ----

export { startElectron, startGpui, sample, startSampler, opt, log, killTree };

if (path.basename(process.argv[1]) === "measure.mjs") await main();

async function main() {
const results = { machine: `${os.cpus()[0].model}, ${os.cpus().length} vCPU, ${Math.round(os.totalmem() / 1e9)} GB`, rounds: [] };
const names = ["electron", "gpui"];
for (let r = 0; r <= ROUNDS; r++) {
  const round = { round: r, warmup: r === 0 };
  // alternate which app goes first
  const order = r % 2 ? [...names].reverse() : names;
  for (const n of order) {
    log(`round ${r} ${n}: start`);
    const start = await startPhase(n);
    log(`round ${r} ${n}: idle`);
    const idle = await idlePhase(n);
    log(`round ${r} ${n}: run`);
    const run = await runPhase(n);
    round[n] = { start, idle, run };
    log(`round ${r} ${n}: ${JSON.stringify(round[n])}`);
  }
  results.rounds.push(round);
  writeFileSync(opt.out, JSON.stringify(results, null, 2));
}
log(`wrote ${opt.out}`);
}
