// Times scrolling the expanded tree of the generated big project with the mouse wheel, pointer over the
// tree as a person's would be, idle and during Run all. Same set-up as perf.spec.ts; SCROLL_OUT gets one JSON line per run.
// Copied from docs/research/2026-10-29-big-project-perf/ unchanged but for this note: the tree's scroller is
// still its parent, and its `data-rows` still says how many rows there are.
// A Chromium trace of the scroll gives where the time went: the renderer main thread (style, layout,
// paint, hit tests, events), the compositor and the raster threads, and how many frames were drawn.
import type { CDPSession, Page } from "@playwright/test";
import { appendFileSync, readFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, test } from "../../../e2e/helpers";

// Tree rows with every folder open, as generate.mjs wrote it.
const { rows: TOTAL } = JSON.parse(
  readFileSync(path.join(process.env.PERF_PROJECT!, "perf-meta.json"), "utf8"),
) as { rows: number };
const NAMES = [
  "Paint",
  "PrePaint",
  "Layerize",
  "RasterTask",
  "UpdateLayoutTree",
  "Layout",
  "HitTest",
  "EventDispatch",
  "FunctionCall",
  "ScrollUpdate",
  "Commit",
  "DrawFrame",
];

interface TraceEvent {
  name: string;
  ph: string;
  pid: number;
  tid: number;
  dur?: number;
  args?: { name?: string };
}

async function trace(cdp: CDPSession, during: () => Promise<void>): Promise<TraceEvent[]> {
  await cdp.send("Tracing.start", {
    transferMode: "ReturnAsStream",
    traceConfig: {
      includedCategories: [
        "devtools.timeline",
        "disabled-by-default-devtools.timeline",
        "disabled-by-default-devtools.timeline.frame",
        "blink",
        "cc",
        "input",
        "toplevel",
      ],
    },
  });
  await during();
  const done = new Promise<string>((resolve) =>
    cdp.once("Tracing.tracingComplete", (e) => resolve(e.stream!)),
  );
  await cdp.send("Tracing.end");
  const stream = await done;
  let text = "";
  for (;;) {
    const chunk = await cdp.send("IO.read", { handle: stream, size: 1 << 20 });
    text += chunk.base64Encoded ? Buffer.from(chunk.data, "base64").toString() : chunk.data;
    if (chunk.eof) break;
  }
  await cdp.send("IO.close", { handle: stream });
  const parsed = JSON.parse(text);
  return Array.isArray(parsed) ? parsed : parsed.traceEvents;
}

/** Summed duration (ms) and count of each event name, by thread kind. */
function summarise(events: TraceEvent[]) {
  const threads = new Map<string, string>();
  for (const e of events)
    if (e.ph === "M" && e.name === "thread_name") threads.set(`${e.pid}:${e.tid}`, e.args!.name!);
  const out: Record<string, { ms: number; n: number }> = {};
  for (const e of events) {
    if (!NAMES.includes(e.name) || (e.ph !== "X" && e.ph !== "I" && e.ph !== "i")) continue;
    const t = threads.get(`${e.pid}:${e.tid}`) ?? "?";
    const kind = t.startsWith("CrRendererMain")
      ? "main"
      : t.startsWith("Compositor")
        ? "compositor"
        : /Raster|CompositorTileWorker/.test(t)
          ? "raster"
          : t.startsWith("VizCompositor")
            ? "viz"
            : "other";
    const k = `${kind}:${e.name}`;
    out[k] ??= { ms: 0, n: 0 };
    out[k].ms += (e.dur ?? 0) / 1000;
    out[k].n++;
  }
  for (const v of Object.values(out)) v.ms = Math.round(v.ms);
  return out;
}

async function frames(win: Page, on: boolean) {
  return win.evaluate((on) => {
    const w = window as unknown as {
      fr?: { on: boolean; count: number; maxGap: number; last: number; gaps: number[] };
    };
    if (!w.fr) {
      const fr = { on: false, count: 0, maxGap: 0, last: 0, gaps: [] as number[] };
      w.fr = fr;
      const tick = (t: number) => {
        if (fr.on) {
          if (fr.last) {
            fr.maxGap = Math.max(fr.maxGap, t - fr.last);
            fr.gaps.push(t - fr.last);
          }
          fr.count++;
          fr.last = t;
        }
        requestAnimationFrame(tick);
      };
      requestAnimationFrame(tick);
    }
    const fr = w.fr;
    if (on) Object.assign(fr, { on: true, count: 0, maxGap: 0, last: 0, gaps: [] });
    else fr.on = false;
    const slow = fr.gaps.filter((g) => g > 50).length;
    return { count: fr.count, maxGap: Math.round(fr.maxGap), over50ms: slow };
  }, on);
}

test("scroll the tree", async () => {
  test.setTimeout(1_800_000);
  const dir = process.env.PERF_PROJECT!;
  const { app, win } = await launch();
  // A real window size, not Playwright's viewport emulation: that pins the page at its size whatever the window.
  await app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].setContentSize(1400, 900));
  await app.evaluate(({ dialog }, d) => {
    dialog.showOpenDialog = (async () => ({ canceled: false, filePaths: [d] })) as never;
  }, dir);
  await win.getByTestId("open-button").click();
  // Every row is there, drawn or not (a virtual list says how many in data-rows).
  await win.waitForFunction(
    (total) => {
      const t = document.querySelector<HTMLElement>("[data-testid=tree]");
      const n = Number(t?.dataset.rows) || document.querySelectorAll("[data-testid=tree-node]").length;
      return n >= total;
    },
    TOTAL,
    { timeout: 300_000, polling: 500 },
  );
  const cdp = await win.context().newCDPSession(win);

  const tree = win.getByTestId("tree");
  const box = (await tree.boundingBox())!;
  await win.mouse.move(box.x + 120, box.y + 200);
  await win.waitForTimeout(1000); // let the open settle

  // 60 wheel notches of 100 px, one per 16 ms: about 1 s of steady scrolling, as a person spins the wheel.
  const result: Record<string, unknown> = {};
  // "down" and "up" with the app idle; "duringRun" scrolls down again while Run all goes on.
  for (const pass of ["down", "up", "duringRun"]) {
    if (pass === "duringRun") {
      await win.getByTestId("run-all-button").click();
      await expect(win.getByTestId("run-outcome")).toHaveText(/Running/, { timeout: 120_000 });
      await win.waitForTimeout(1000);
      await win.mouse.move(box.x + 120, box.y + 200); // the click took the pointer to Run all
    }
    await frames(win, true);
    const t0 = Date.now();
    const waits: number[] = []; // how long each notch took to be taken by the page
    const events = await trace(cdp, async () => {
      for (let i = 0; i < 60; i++) {
        const w0 = Date.now();
        await win.mouse.wheel(0, pass === "up" ? -100 : 100);
        waits.push(Date.now() - w0);
        await win.waitForTimeout(16);
      }
      await win.waitForTimeout(300);
    });
    const wall = Date.now() - t0;
    const f = await frames(win, false);
    const top = await win.evaluate(
      () => document.querySelector("[data-testid=tree]")!.parentElement!.scrollTop,
    );
    waits.sort((a, b) => a - b);
    result[pass] = {
      wall,
      rafFrames: f.count,
      maxGap: f.maxGap,
      over50ms: f.over50ms,
      scrollTop: top,
      notchMedian: waits[30],
      notchMax: waits[59],
      trace: summarise(events),
    };
    console.log(pass, JSON.stringify(result[pass]));
  }
  if (process.env.SCROLL_OUT) appendFileSync(process.env.SCROLL_OUT, JSON.stringify(result) + "\n");
});
