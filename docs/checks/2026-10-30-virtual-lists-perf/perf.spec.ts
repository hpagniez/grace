// Times the renderer on the generated big project. Copied from docs/research/2026-10-29-big-project-perf/
// with the selectors of the virtual lists. Run from the repo root after `npm run package`:
//   node docs/research/2026-10-29-big-project-perf/generate.mjs <dir>
//   PERF_PROJECT=<dir> PERF_OUT=<file.jsonl> npx playwright test --config docs/checks/2026-10-30-virtual-lists-perf/playwright.config.ts
// Each probe clicks inside the page and stops at the first task after the next frame: click to paint.
// Chromium's Performance.getMetrics gives the renderer's busy time, split into script, style and layout.
import type { CDPSession, Page } from "@playwright/test";
import { appendFileSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, openProject, projectDataDir, test } from "../../../e2e/helpers";

// The sizes generate.mjs wrote: HTTP calls, and tree rows with every folder open.
const { calls: CALLS, rows: TOTAL } = JSON.parse(
  readFileSync(path.join(process.env.PERF_PROJECT!, "perf-meta.json"), "utf8"),
) as { calls: number; rows: number };
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

/**
 * What a probe acts on: the `index`th match of `sel` from the top of the list (the first by default), because
 * a virtual list draws its focus row first in the DOM, whatever its place. With `label`, that row's label: a
 * tree row's button with no aria-label, or the first element inside an HTTP call, which takes the click. With
 * `key`, the probe presses that key on the row instead of clicking.
 */
interface Target {
  sel: string;
  index?: number;
  label?: boolean;
  key?: string;
}
/** When a click has taken effect: a list's row count reached or fell under a number, or a row is selected. */
type Done =
  | { rows: "tree" | "http"; atLeast: number }
  | { rows: "tree" | "http"; under: number }
  | { attr: "aria-selected"; of: Target };
/** The page side `instrument` installs. */
interface Probe {
  clickToPaint(target: Target, done: Done): Promise<number>;
}

/** Installs the page side: long-task log, frame log, and `clickToPaint`. */
async function instrument(win: Page) {
  await win.evaluate(() => {
    const w = window as unknown as Record<string, unknown>;
    const longTasks: number[] = [];
    new PerformanceObserver((l) => {
      for (const e of l.getEntries()) longTasks.push(e.duration);
    }).observe({ type: "longtask" });
    w.longTasks = longTasks;
    const paint = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
    // Rows in the list, drawn or not: a virtual list draws only those in view and says how many in data-rows.
    const count = (box: string, row: string) =>
      Number((document.querySelector(box) as HTMLElement | null)?.dataset.rows) ||
      document.querySelectorAll(row).length;
    const rows = (list: "tree" | "http") =>
      list === "tree"
        ? count("[data-testid=tree]", "[data-testid=tree-node]")
        : count("[data-testid=http-list]", "[data-testid=http-call]");
    const find = (t: Target) => {
      const top = (e: Element) => e.getBoundingClientRect().top;
      const all = [...document.querySelectorAll<HTMLElement>(t.sel)].sort((a, b) => top(a) - top(b));
      const el = all[t.index ?? 0];
      const button = [...el.querySelectorAll("button")].find((b) => !b.hasAttribute("aria-label"));
      return t.label ? (button ?? (el.firstElementChild as HTMLElement)) : el;
    };
    const holds = (d: Done) => {
      if ("attr" in d) return find(d.of).closest(`[${d.attr}]`)?.getAttribute(d.attr) === "true";
      return "atLeast" in d ? rows(d.rows) >= d.atLeast : rows(d.rows) < d.under;
    };
    // Clicks the target (or presses its key), then waits until `done` holds (checked each frame) and the frame
    // after it is painted. A `done` that held already would time nothing: it throws.
    w.clickToPaint = async (target: Target, done: Done) => {
      const el = find(target);
      if (holds(done)) throw new Error(`already done before the click: ${JSON.stringify(done)}`);
      const t0 = performance.now();
      if (target.key)
        el.dispatchEvent(new KeyboardEvent("keydown", { key: target.key, bubbles: true, cancelable: true }));
      else el.click();
      while (!holds(done)) await new Promise((r) => requestAnimationFrame(r));
      await paint();
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
async function probe(win: Page, cdp: CDPSession, name: string, target: Target, done: Done) {
  const lt = await longTaskMark(win);
  const before = await metrics(cdp);
  const ms = await win.evaluate(
    ([t, d]) => (window as unknown as Probe).clickToPaint(t, d),
    [target, done] as const,
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
  await win.getByTestId("open-button").waitFor(); // the home page draws after the store loads
  await probe(win, cdp, "open", { sel: "[data-testid=open-button]" }, { rows: "tree", atLeast: TOTAL });
  void openProject; // the probe clicks Open itself, inside the page, to start its clock there
  result.rows = await win.locator(ROWS).count(); // drawn: a virtual list draws only the rows in view
  const opened = await metrics(cdp);
  result.afterOpen = { nodes: opened.Nodes, listeners: opened.JSEventListeners, heapMB: Math.round(opened.JSHeapUsedSize / 1e6) };
  console.log("afterOpen", JSON.stringify(result.afterOpen));

  // Collapse and expand the top folder: every row goes, then comes back.
  const toggle = { sel: `${TOP} button` }; // its first button: expand or collapse
  await probe(win, cdp, "collapseAll", toggle, { rows: "tree", under: 10 });
  await probe(win, cdp, "expandAll", toggle, { rows: "tree", atLeast: TOTAL });

  // Select the first 20 tests with the mouse; they are in the rows drawn at the top of the list.
  const selects: number[] = [];
  for (let i = 0; i < 20; i++) {
    const row = { sel: TEST_ROWS, index: i };
    const label = { ...row, label: true };
    selects.push((await probe(win, cdp, `select${i}`, label, { attr: "aria-selected", of: row })).ms);
  }
  for (let i = 0; i < 20; i++) delete result[`select${i}`];
  selects.sort((a, b) => a - b);
  result.select = { median: selects[10], max: selects[19], all: selects };
  console.log("select", JSON.stringify(result.select));

  // Move down the list with the arrow key, from the top folder: each press selects the next row.
  const first = { sel: ROWS, label: true };
  await probe(win, cdp, "downStart", first, { attr: "aria-selected", of: { sel: ROWS } });
  const downs: number[] = [];
  for (let i = 0; i < 10; i++) {
    const press = { sel: ROWS, index: i, key: "ArrowDown" };
    const next = { sel: ROWS, index: i + 1 };
    downs.push((await probe(win, cdp, `down${i}`, press, { attr: "aria-selected", of: next })).ms);
  }
  delete result.downStart;
  for (let i = 0; i < 10; i++) delete result[`down${i}`];
  downs.sort((a, b) => a - b);
  result.down = { median: downs[5], max: downs[9], all: downs };
  console.log("down", JSON.stringify(result.down));

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
  await probe(win, cdp, "selectTop", { sel: TOP, label: true }, { attr: "aria-selected", of: { sel: TOP } });
  await run("runConsole");
  // PERF_ONLY_CONSOLE=1 stops here: a build whose HTTP-tab run would take too long to wait for.
  if (process.env.PERF_ONLY_CONSOLE) {
    if (process.env.PERF_OUT) appendFileSync(process.env.PERF_OUT, JSON.stringify(result) + "\n");
    return;
  }
  await expect(win.getByTestId("tab-http")).toContainText(String(CALLS), { timeout: 30_000 });

  const all: Done = { rows: "http", atLeast: CALLS };
  await probe(win, cdp, "httpTab", { sel: "[data-testid=tab-http]" }, all);
  const group = (g: string) => ({ sel: `[data-testid=http-group][data-group="${g}"]` });
  await probe(win, cdp, "http2xxOnly", group("2xx"), { rows: "http", under: CALLS });
  await probe(win, cdp, "httpAll", group("all"), all);
  const call = { sel: "[data-testid=http-call]", index: 5 };
  await probe(win, cdp, "httpSelectCall", { ...call, label: true }, { attr: "aria-selected", of: call });
  // Narrowing to one feature, then back to the whole project (the top folder).
  const feature = { sel: `${ROWS}[data-key^="f:tests/"]`, label: true };
  await probe(win, cdp, "httpNarrow", feature, { rows: "http", under: 10 });
  await probe(win, cdp, "httpWiden", { sel: TOP, label: true }, all);

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
