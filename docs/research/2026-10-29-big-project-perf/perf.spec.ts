// Times the renderer on the generated big project. Run from the repo root after `npm run package`:
//   node docs/research/2026-10-29-big-project-perf/generate.mjs <dir>
//   PERF_PROJECT=<dir> PERF_OUT=<file.json> npx playwright test --config docs/research/2026-10-29-big-project-perf/playwright.config.ts
// Each probe clicks inside the page and stops at the first task after the next frame: click to paint.
// Chromium's Performance.getMetrics gives the renderer's busy time, split into script, style and layout.
import type { CDPSession, Page } from "@playwright/test";
import { appendFileSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, openProject, projectDataDir, test } from "../../../e2e/helpers";

// The sizes generate.mjs wrote: tests, HTTP calls, and tree rows with every folder open.
const { tests: TESTS, calls: CALLS, rows: TOTAL } = JSON.parse(
  readFileSync(path.join(process.env.PERF_PROJECT!, "perf-meta.json"), "utf8"),
) as { tests: number; calls: number; rows: number };
const result: Record<string, unknown> = {};

type Metrics = Record<string, number>;
const metrics = async (cdp: CDPSession): Promise<Metrics> =>
  Object.fromEntries(
    (await cdp.send("Performance.getMetrics")).metrics.map((m) => [m.name, m.value]),
  );
/** The renderer's busy time between two snapshots, in ms. */
const busy = (a: Metrics, b: Metrics) => ({
  task: Math.round((b.TaskDuration - a.TaskDuration) * 1000),
  script: Math.round((b.ScriptDuration - a.ScriptDuration) * 1000),
  style: Math.round((b.RecalcStyleDuration - a.RecalcStyleDuration) * 1000),
  layout: Math.round((b.LayoutDuration - a.LayoutDuration) * 1000),
});

/** Installs the page side: long-task log, frame log, and `clickToPaint`. */
async function instrument(win: Page) {
  await win.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const longTasks: number[] = [];
    new PerformanceObserver((l) => {
      for (const e of l.getEntries()) longTasks.push(e.duration);
    }).observe({ type: "longtask" });
    w.longTasks = longTasks;
    w.paint = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
    // Rows in the list, drawn or not: a virtual list draws only those in view and says how many in data-rows.
    const count = (box: string, row: string) =>
      Number((document.querySelector(box) as HTMLElement | null)?.dataset.rows) ||
      document.querySelectorAll(row).length;
    w.treeRows = () => count("[data-testid=tree]", "[data-testid=tree-node]");
    w.httpRows = () => count("[data-testid=http-calls]", "[data-testid=http-call]");
    // Clicks `el`, then waits until `done()` holds (checked each frame) and the frame after it is painted.
    w.clickToPaint = async (el: HTMLElement, done: () => boolean) => {
      const t0 = performance.now();
      el.click();
      while (!done()) await new Promise((r) => requestAnimationFrame(r));
      await (w.paint as () => Promise<void>)();
      return Math.round(performance.now() - t0);
    };
    // Frame gaps while `frames.on`: the longest gap is the worst freeze a person would see.
    const frames = { on: false, count: 0, maxGap: 0, last: 0 };
    w.frames = frames;
    const tick = (t: number) => {
      if (frames.on) {
        if (frames.last) frames.maxGap = Math.max(frames.maxGap, t - frames.last);
        frames.count++;
        frames.last = t;
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  });
}

const longTaskMark = (win: Page) =>
  win.evaluate(() => ((window as unknown as { longTasks: number[] }).longTasks).length);
const longTasksSince = (win: Page, from: number) =>
  win.evaluate((f) => {
    const all = (window as unknown as { longTasks: number[] }).longTasks.slice(f);
    return {
      count: all.length,
      total: Math.round(all.reduce((a, b) => a + b, 0)),
      max: Math.round(Math.max(0, ...all)),
    };
  }, from);

/** Times one click to paint, with the renderer's busy time and long tasks around it. */
async function probe(
  win: Page,
  cdp: CDPSession,
  name: string,
  click: string,
  done: string,
  arg?: unknown,
) {
  const lt = await longTaskMark(win);
  const before = await metrics(cdp);
  // A string, not a function: Playwright evaluates it over CDP, where the page's CSP does not apply.
  const ms = await win.evaluate<number>(
    `(() => { const arg = ${JSON.stringify(arg ?? null)}; return window.clickToPaint((() => { ${click} })(), () => { ${done} }); })()`,
  );
  const after = await metrics(cdp);
  const entry = { ms, busy: busy(before, after), longTasks: await longTasksSince(win, lt) };
  result[name] = entry;
  console.log(name, JSON.stringify(entry));
  return entry;
}

const ROWS = "[data-testid=tree-node]";
const TEST_ROWS = `${ROWS}[data-kind=scenario], ${ROWS}[data-kind=row]`;
const TOP = `${ROWS}[data-key="d:tests"]`; // the folder that holds every test

test("big project", async () => {
  test.setTimeout(3_600_000); // the slowest builds take minutes per run at 10,000 tests
  const dir = process.env.PERF_PROJECT!;
  const { app, win, userData } = await launch();
  // A real window size, not Playwright's viewport emulation: that pins the page at its size whatever the window.
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].setContentSize(1400, 900));
  await instrument(win);
  const cdp = await win.context().newCDPSession(win);
  await cdp.send("Performance.enable");

  // Opening: the dry run is Java's time; the long tasks and busy time are the renderer's.
  await app.evaluate(({ dialog }, d) => {
    dialog.showOpenDialog = (async () => ({ canceled: false, filePaths: [d] })) as never;
  }, dir);
  await probe(
    win,
    cdp,
    "open",
    `return document.querySelector("[data-testid=open-button]")`,
    `return window.treeRows() >= ${TOTAL}`,
  );
  void openProject; // the probe clicks Open itself, inside the page, to start its clock there
  result.rows = await win.locator(ROWS).count();
  const opened = await metrics(cdp);
  result.afterOpen = { nodes: opened.Nodes, listeners: opened.JSEventListeners, heapMB: Math.round(opened.JSHeapUsedSize / 1e6) };
  console.log("afterOpen", JSON.stringify(result.afterOpen));

  // Collapse and expand the top folder: every row goes, then comes back.
  const toggle = `return document.querySelector(${JSON.stringify(TOP)}).querySelector("button")`;
  await probe(win, cdp, "collapseAll", toggle, `return window.treeRows() < 10`);
  await probe(win, cdp, "expandAll", toggle, `return window.treeRows() >= ${TOTAL}`);

  // Select the first 20 tests, which every build draws: the label is the row's button with no aria-label.
  const pick = `const r = document.querySelectorAll(${JSON.stringify(TEST_ROWS)})[arg]; return [...r.querySelectorAll("button")].find(b => !b.hasAttribute("aria-label"))`;
  const selected = `return document.querySelectorAll(${JSON.stringify(TEST_ROWS)})[arg].closest("[aria-selected]").getAttribute("aria-selected") === "true"`;
  const selects: number[] = [];
  for (let i = 0; i < 20; i++)
    selects.push((await probe(win, cdp, `select${i}`, pick, selected, i)).ms);
  for (let i = 0; i < 20; i++) delete result[`select${i}`];
  selects.sort((a, b) => a - b);
  result.select = { median: selects[10], max: selects[19], all: selects };
  console.log("select", JSON.stringify(result.select));

  // A full run with the Console tab showing, then with the HTTP tab showing.
  async function run(name: string) {
    const lt = await longTaskMark(win);
    const before = await metrics(cdp);
    await win.evaluate(() => {
      const f = (window as unknown as { frames: { on: boolean; count: number; maxGap: number; last: number } }).frames;
      Object.assign(f, { on: true, count: 0, maxGap: 0, last: 0 });
    });
    // PERF_PROFILE=<prefix> saves a CPU profile of each run as <prefix>.<run>.cpuprofile (DevTools opens it).
    const profile = process.env.PERF_PROFILE;
    if (profile) {
      await cdp.send("Profiler.enable");
      await cdp.send("Profiler.start");
    }
    // The bar's value at each frame, when it changes: what a person sees fill.
    await win.evaluate(() => {
      const w = window as unknown as { prog: [number, number][]; progOn: boolean };
      w.prog = [];
      w.progOn = true;
      const rec = () => {
        if (!w.progOn) return;
        const v = Number(document.querySelector("[role=progressbar]")?.getAttribute("aria-valuenow") ?? 0);
        if (!w.prog.length || w.prog[w.prog.length - 1][1] !== v) w.prog.push([Date.now(), v]);
        requestAnimationFrame(rec);
      };
      requestAnimationFrame(rec);
    });
    const t0 = Date.now();
    await win.getByTestId("run-all-button").click();
    await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 1_500_000 });
    const wall = Date.now() - t0;
    const prog = await win.evaluate(() => {
      const w = window as unknown as { prog: [number, number][]; progOn: boolean };
      w.progOn = false;
      return w.prog;
    });
    // Karate's own times: the depth-0 SCENARIO_EXIT lines of the newest run's event stream.
    const runs = path.join(projectDataDir(userData, dir), "runs");
    const newest = readdirSync(runs).sort().pop()!;
    const exits = readFileSync(path.join(runs, newest, "karate-json", "karate-events.jsonl"), "utf8")
      .split(String.fromCharCode(10))
      .filter((l) => l.includes('"SCENARIO_EXIT"'))
      .map((l) => JSON.parse(l))
      .filter((e) => e.data.callDepth === 0)
      .map((e) => e.timeStamp as number)
      .sort((a, b) => a - b);
    // The bar still shows the previous run's end until this run resets it.
    prog.splice(0, Math.max(0, prog.findIndex(([, v]) => v < exits.length)));
    const at = (q: number) => {
      const n = Math.ceil(q * exits.length);
      const shown = prog.find(([, v]) => v >= n);
      return { karate: exits[n - 1] - t0, shown: shown ? shown[0] - t0 : null };
    };
    let maxJump = 0;
    for (let i = 1; i < prog.length; i++) maxJump = Math.max(maxJump, prog[i][1] - prog[i - 1][1]);
    const progress = {
      tests: exits.length,
      frames: prog.length, // frames in which the bar moved
      maxJump, // the most tests the bar took in one step
      q10: at(0.1),
      q50: at(0.5),
      q90: at(0.9),
      q100: at(1),
    };
    if (profile)
      writeFileSync(`${profile}.${name}.cpuprofile`, JSON.stringify((await cdp.send("Profiler.stop")).profile));
    const frames = await win.evaluate(() => {
      const f = (window as unknown as { frames: { on: boolean; count: number; maxGap: number } }).frames;
      f.on = false;
      return { count: f.count, maxGap: Math.round(f.maxGap) };
    });
    const after = await metrics(cdp);
    const entry = {
      wall,
      progress,
      fps: Math.round((frames.count * 1000) / wall),
      maxFrameGap: frames.maxGap,
      busy: busy(before, after),
      longTasks: await longTasksSince(win, lt),
    };
    result[name] = entry;
    console.log(name, JSON.stringify(entry));
  }
  // The HTTP tab lists the calls under the tree selection: select the whole project first.
  const label = (sel: string) =>
    `return [...document.querySelector(${JSON.stringify(sel)}).querySelectorAll("button")].find(b => !b.hasAttribute("aria-label"))`;
  await probe(win, cdp, "selectTop", label(TOP), `return document.querySelector(${JSON.stringify(TOP)}).closest("[aria-selected]").getAttribute("aria-selected") === "true"`);
  await run("runConsole");
  // PERF_ONLY_CONSOLE=1 stops here: a build whose HTTP-tab run would take too long to wait for.
  if (process.env.PERF_ONLY_CONSOLE) {
    if (process.env.PERF_OUT) appendFileSync(process.env.PERF_OUT, JSON.stringify(result) + "\n");
    return;
  }
  await expect(win.getByTestId("tab-http")).toContainText(String(CALLS), { timeout: 30_000 });

  const rows = `return window.httpRows()`;
  await probe(win, cdp, "httpTab", `return document.querySelector("[data-testid=tab-http]")`, `${rows} >= ${CALLS}`);
  await probe(
    win,
    cdp,
    "http2xxOnly",
    `return document.querySelector("[data-testid=http-group][data-group='2xx']")`,
    `${rows} < ${CALLS}`,
  );
  await probe(
    win,
    cdp,
    "httpAll",
    `return document.querySelector("[data-testid=http-group][data-group='all']")`,
    `${rows} >= ${CALLS}`,
  );
  await probe(
    win,
    cdp,
    "httpSelectCall",
    `return document.querySelectorAll("[data-testid=http-call]")[5]`,
    `return document.querySelectorAll("[data-testid=http-call]")[5].getAttribute("aria-pressed") === "true"`,
  );
  // Narrowing to one feature, then back to the whole project (the top folder).
  await probe(win, cdp, "httpNarrow", label(`${ROWS}[data-key^="f:tests/"]`), `${rows} < 10`);
  await probe(win, cdp, "httpWiden", label(TOP), `${rows} >= ${CALLS}`);

  await run("runHttp");
  await cdp.send("HeapProfiler.collectGarbage"); // what is left is held, not garbage
  const m = await metrics(cdp);
  result.end = {
    nodes: m.Nodes,
    listeners: m.JSEventListeners,
    heapMB: Math.round(m.JSHeapUsedSize / 1e6),
    attached: await win.evaluate(() => document.getElementsByTagName("*").length),
  };
  console.log("end", JSON.stringify(result.end));
  if (process.env.PERF_OUT) appendFileSync(process.env.PERF_OUT, JSON.stringify(result) + "\n");
});
