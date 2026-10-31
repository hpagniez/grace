// What the bench asks of Grace's page over CDP: click to paint, frames, page size, and how late "Finished" shows after
// Karate's last test. Ported from docs/checks/2026-10-30-virtual-lists-perf/ (perf.spec.ts and scroll.spec.ts), which
// were written against the e2e harness. A probe that cannot find what it acts on reads null, so a build with older test
// ids gives n/a, never a crash; only a page that cannot be reached at all throws.
import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { setTimeout as sleep } from "node:timers/promises";

/** @typedef {import("@playwright/test").Page} Page */
/** @typedef {import("@playwright/test").CDPSession} CDPSession */
/**
 * What a probe acts on: the `index`th match of `sel` from the top of the view (the first by default), because a virtual
 * list draws only the rows in view. With `label`, that row's label: a tree row's button with no aria-label, or the first
 * element inside an HTTP call, which takes the click. With `key`, the probe presses that key on the row instead.
 * @typedef {{ sel: string, index?: number, label?: boolean, key?: string }} Target
 */
/**
 * When a click has taken effect: a list's row count reached or fell under a number, or a row is selected.
 * @typedef {{ rows: "tree" | "http", atLeast: number }
 *   | { rows: "tree" | "http", under: number }
 *   | { attr: "aria-selected", of: Target }} Done
 */

export const TREE = "[data-testid=tree]";
/** How long a probe looks for its target before it reads null. */
const TARGET_MS = 5_000;
/** How long a click may take to show its effect before it reads null: a slow build gives a number, a stuck one null. */
const EFFECT_MS = 30_000;

/** A timeout is "not found"; anything else is a real failure. */
const absent = (/** @type {any} */ e) => {
  if (e?.name === "TimeoutError") return null;
  throw e;
};

/**
 * Installs the page side as `window.bench`: `clickToPaint`, a frame log, and a watch on when "Finished" first shows.
 * Safe to call again.
 * @param {Page} page
 * @returns {Promise<void>}
 */
export async function instrument(page) {
  await page.evaluate(
    ({ targetMs, effectMs }) => {
      const w = /** @type {any} */ (window);
      if (w.bench) return;
      const sleep = (/** @type {number} */ ms) => new Promise((r) => setTimeout(r, ms));
      const frame = () => new Promise((r) => requestAnimationFrame(r));
      const paint = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
      // Rows in the list, drawn or not: a virtual list draws only those in view and says how many in data-rows.
      const rows = (/** @type {"tree" | "http"} */ list) => {
        const [box, row] =
          list === "tree"
            ? ["[data-testid=tree]", "[data-testid=tree-node]"]
            : ["[data-testid=http-list]", "[data-testid=http-call]"];
        return (
          Number(/** @type {HTMLElement | null} */ (document.querySelector(box))?.dataset.rows) ||
          document.querySelectorAll(row).length
        );
      };
      const find = (/** @type {any} */ t) => {
        const top = (/** @type {Element} */ e) => e.getBoundingClientRect().top;
        const all = [...document.querySelectorAll(t.sel)].sort((a, b) => top(a) - top(b));
        const el = all[t.index ?? 0];
        if (!el || !t.label) return el ?? null;
        const button = [...el.querySelectorAll("button")].find(
          (b) => !b.hasAttribute("aria-label"),
        );
        return button ?? el.firstElementChild;
      };
      const holds = (/** @type {any} */ d) => {
        if ("attr" in d) return find(d.of)?.closest(`[${d.attr}]`)?.getAttribute(d.attr) === "true";
        return "atLeast" in d ? rows(d.rows) >= d.atLeast : rows(d.rows) < d.under;
      };

      // Frame times, logged only while a window is open: windows nest, and an idle page draws no frames of ours.
      /** @type {number[]} */
      const times = [];
      let open = 0;
      let ticking = false;
      const tick = (/** @type {number} */ t) => {
        times.push(t);
        if (open) requestAnimationFrame(tick);
        else ticking = false;
      };

      w.bench = {
        finishedAt: null,
        // Clicks the target (or presses its key), then waits until `done` holds (checked each frame) and the frame after
        // it is painted. Null when there is no target, `done` held already (nothing to time), or it never holds.
        async clickToPaint(/** @type {any} */ target, /** @type {any} */ done) {
          const giveUp = performance.now() + targetMs;
          let el;
          while (!(el = find(target))) {
            if (performance.now() > giveUp) return null;
            await sleep(50);
          }
          if (holds(done)) return null;
          const t0 = performance.now();
          if (target.key)
            el.dispatchEvent(
              new KeyboardEvent("keydown", { key: target.key, bubbles: true, cancelable: true }),
            );
          else el.click();
          while (!holds(done)) {
            if (performance.now() - t0 > effectMs) return null;
            await frame();
          }
          await paint();
          return Math.round(performance.now() - t0);
        },
        openFrames() {
          open++;
          if (!ticking) {
            ticking = true;
            requestAnimationFrame(tick);
          }
          return performance.now();
        },
        // The longest gap between frames since `from`, counting the stretch up to now: a freeze that ends after the last
        // frame still shows.
        closeFrames(/** @type {number} */ from) {
          const to = performance.now();
          const inside = times.filter((t) => t >= from && t <= to);
          if (!--open) times.length = 0;
          let maxGap = 0;
          let prev = from;
          for (const t of [...inside, to]) {
            maxGap = Math.max(maxGap, t - prev);
            prev = t;
          }
          return { count: inside.length, ms: to - from, maxGap };
        },
      };

      // When "Finished" first showed, on the epoch clock Karate's event times use. Only changes to the outcome are
      // watched, so the watch costs nothing while the page is busy elsewhere.
      const outcome = document.querySelector("[data-testid=run-outcome]");
      if (outcome)
        new MutationObserver(() => {
          const done = outcome.textContent?.trim() === "Finished";
          w.bench.finishedAt = done ? (w.bench.finishedAt ?? Date.now()) : null;
        }).observe(outcome, { childList: true, characterData: true, subtree: true });
    },
    { targetMs: TARGET_MS, effectMs: EFFECT_MS },
  );
}

/**
 * Times one click to paint: from the click until `done` holds and the next frame is on screen. Null when the target
 * is not found within 5 s, or `done` held before the click, or never came (30 s). The page's own waits run on its
 * timers and frames, which a frozen renderer stops, so the same limits are kept here too: a page that never answers
 * reads null instead of hanging the bench.
 * @param {Page} page
 * @param {Target} target
 * @param {Done} done
 * @returns {Promise<number | null>}
 */
export async function clickToPaint(page, target, done) {
  const probe = page.evaluate(
    ([t, d]) => /** @type {any} */ (window).bench.clickToPaint(t, d),
    [target, done],
  );
  probe.catch(() => {}); // if the cap wins, a late failure (the app stopped) is nobody's to handle
  /** @type {NodeJS.Timeout | undefined} */
  let timer;
  const frozen = new Promise(
    (resolve) => (timer = setTimeout(resolve, TARGET_MS + EFFECT_MS, null)),
  );
  try {
    return await Promise.race([probe, frozen]);
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Runs `fn` and gives the frame rate over it and the longest frame (the longest gap between frames, in ms).
 * @param {Page} page
 * @param {() => Promise<void>} fn
 * @returns {Promise<{ fps: number, frameMs: number }>}
 */
export async function framesDuring(page, fn) {
  const from = await page.evaluate(() => /** @type {any} */ (window).bench.openFrames());
  /** @type {{ count: number, ms: number, maxGap: number }} */
  let frames;
  try {
    await fn();
  } finally {
    frames = await page.evaluate((f) => /** @type {any} */ (window).bench.closeFrames(f), from);
  }
  return { fps: Math.round((frames.count * 1000) / frames.ms), frameMs: Math.round(frames.maxGap) };
}

/**
 * The longest frame while the mouse wheel scrolls the tree `notches` times, 100 px each, one per 16 ms, as a person
 * spins it (scroll.spec.ts), the pointer over the tree. Null when there is no tree within 5 s. Leaves the tree scrolled
 * back to the top: a virtual list draws only what is in view, so the next probe could not find its folder.
 * @param {Page} page
 * @param {number} [notches]
 * @returns {Promise<{ frameMs: number | null }>}
 */
export async function scrollTree(page, notches = 60) {
  // The tree's scroller is its parent, and stays where the view is however far it has scrolled.
  const box = await page
    .locator(TREE)
    .locator("..")
    .boundingBox({ timeout: TARGET_MS })
    .catch(absent);
  if (!box) return { frameMs: null };
  await page.mouse.move(box.x + 120, box.y + 200);
  const { frameMs } = await framesDuring(page, async () => {
    for (let i = 0; i < notches; i++) {
      await page.mouse.wheel(0, 100);
      await sleep(16);
    }
    await sleep(300);
  });
  await page.evaluate((s) => {
    const scroller = document.querySelector(s)?.parentElement;
    if (scroller) scroller.scrollTop = 0;
  }, TREE);
  return { frameMs };
}

/**
 * DOM nodes and JS heap in use now, after a garbage collection: what is left is held, not garbage.
 * @param {CDPSession} cdp
 * @returns {Promise<{ nodes: number, heapMB: number }>}
 */
export async function pageStats(cdp) {
  await cdp.send("Performance.enable");
  await cdp.send("HeapProfiler.collectGarbage");
  const { metrics } = await cdp.send("Performance.getMetrics");
  await cdp.send("Performance.disable");
  const m = Object.fromEntries(metrics.map((x) => [x.name, x.value]));
  return { nodes: m.Nodes, heapMB: Math.round(m.JSHeapUsedSize / 1e6) };
}

/**
 * Karate's event file of the newest run of `project` in the data folder `dataDir`, or null when it has no run: the
 * app keeps a project's runs in `projects/<first 12 hex of sha1 of the project path>/runs/<id>/` (the newest id sorts
 * last), as `projectDataDir` in e2e/helpers.ts says.
 * @param {string} dataDir
 * @param {string} project
 * @returns {string | null}
 */
export function newestEventsFile(dataDir, project) {
  const hash = createHash("sha1").update(path.resolve(project)).digest("hex").slice(0, 12);
  const runs = path.join(dataDir, "projects", hash, "runs");
  try {
    const newest = readdirSync(runs).sort().pop();
    return newest ? path.join(runs, newest, "karate-json", "karate-events.jsonl") : null;
  } catch (e) {
    if (/** @type {any} */ (e).code === "ENOENT") return null;
    throw e;
  }
}

/**
 * How long after Karate's last test the page showed "Finished", in ms, on one clock: the big-project note's method.
 * Karate's last test is the newest depth-0 `SCENARIO_EXIT` in `eventsFile`. Null when the page never showed it (no
 * outcome element, or the run is not over) or there is no event file or no test in it. Call it once the run is over.
 * @param {Page} page
 * @param {string | null} eventsFile
 * @returns {Promise<number | null>}
 */
export async function finishedLagMs(page, eventsFile) {
  const shownAt = await page.evaluate(() => /** @type {any} */ (window).bench?.finishedAt ?? null);
  if (shownAt === null || eventsFile === null) return null;
  let text;
  try {
    text = readFileSync(eventsFile, "utf8");
  } catch (e) {
    if (/** @type {any} */ (e).code === "ENOENT") return null;
    throw e;
  }
  const exits = text
    .split(/\r?\n/)
    .filter((line) => line.includes('"SCENARIO_EXIT"'))
    .map((line) => JSON.parse(line))
    .filter((e) => e.data.callDepth === 0)
    .map((e) => /** @type {number} */ (e.timeStamp));
  return exits.length ? Math.round(shownAt - Math.max(...exits)) : null;
}
