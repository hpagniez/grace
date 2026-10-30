// Virtual lists (spec 2026-10-30-virtual-lists): what only a real browser can show, on a generated project
// of 2,000 tests. Rows, keys and ARIA are checked by unit tests.
import type { Page } from "@playwright/test";
import { bigProject, expect, launch, nodeByKey, openProject, project, test } from "./helpers";

const ROW = 24; // px, a tree row
const PAD = 8; // px before the first row and after the last
// tests/, 8 areas, 40 modules, 200 features, 200 outlines and 2,000 tests
const ROWS = 2449;
const LAST_TEST = "s:tests/area-8/module-5/feature-5.feature#7.4"; // "outline 8.5.5 row 2000"
// The 1,500th row: "scenario 1224", tagged module-5-5. It makes no HTTP call, so a run of it is short.
const ROW_1500 = "s:tests/area-5/module-5/feature-3.feature#4";
const FEATURE_2 = "f:tests/area-1/module-1/feature-2.feature";
// Every 7th test name is long enough to be cut in the tree: this is test 14.
const CUT = "s:tests/area-1/module-1/feature-2.feature#4";
const CUT_NAME =
  "scenario 14 checks that the order service returns the item with every field filled and the price rounded to cents";
const BEFORE_CUT = "s:tests/area-1/module-1/feature-2.feature#3"; // a short name

/** The generated project, open, once discovery has put all its rows in the tree. */
async function openBig() {
  const dir = bigProject();
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(tree(win)).toHaveAttribute("data-rows", String(ROWS), { timeout: 60_000 });
  return { app, win };
}

const tree = (win: Page) => win.getByTestId("tree");
const scroller = (win: Page) => tree(win).locator("..");
/** The row element of a key: the treeitem, which takes the focus. */
const item = (win: Page, key: string) => win.locator(`[role=treeitem][data-key="${key}"]`);
// The row's label is the second button of its row: the chevron, the label, the run button.
const label = (win: Page, key: string) => nodeByKey(win, key).locator("button").nth(1);

interface Drawn {
  key: string;
  /** The place in the list, from the row's `top`. */
  index: number;
  /** The row's box lies inside the scroller's. */
  inView: boolean;
  focused: boolean;
}

/** Every row in the page. The focus row is first in the DOM, so this is not in list order. */
const drawn = (win: Page): Promise<Drawn[]> =>
  win.evaluate(
    ([row, pad]) => {
      const list = document.querySelector("[data-testid=tree]")!;
      const box = list.parentElement!.getBoundingClientRect();
      return [...list.children].map((el) => {
        const r = el.getBoundingClientRect();
        return {
          key: (el as HTMLElement).dataset.key!,
          index: (parseFloat((el as HTMLElement).style.top) - pad) / row,
          inView: r.top >= box.top && r.bottom <= box.bottom,
          focused: el === document.activeElement,
        };
      });
    },
    [ROW, PAD],
  );
const rowAt = async (win: Page, index: number) => (await drawn(win)).find((r) => r.index === index);
const focusedRow = async (win: Page) => (await drawn(win)).find((r) => r.focused);
/** The key of the first row whose box lies inside the scroller's. */
const topInView = async (win: Page) =>
  (await drawn(win)).filter((r) => r.inView).sort((a, b) => a.index - b.index)[0]?.key;

const scrollTop = (win: Page) => scroller(win).evaluate((el) => el.scrollTop);
const maxScroll = (win: Page) => scroller(win).evaluate((el) => el.scrollHeight - el.clientHeight);
/** Scrolls so that the row at `index` is the top row in view, and waits for it to be drawn there. */
async function scrollToRow(win: Page, index: number) {
  await scroller(win).evaluate((el, top) => (el.scrollTop = top), PAD + index * ROW);
  await expect.poll(async () => (await rowAt(win, index))?.inView).toBe(true);
}

/** The key of the row at `index`; it must be drawn. */
async function keyAt(win: Page, index: number) {
  const row = await rowAt(win, index);
  expect(row, `row ${index} is drawn`).toBeDefined();
  return row!.key;
}

/** The mouse wheel over the tree, `deltaY` px. */
async function wheel(win: Page, deltaY: number) {
  const box = (await scroller(win).boundingBox())!;
  await win.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await win.mouse.wheel(0, deltaY);
}

/** Records every scroll of the tree from here on. */
const recordScrolls = (win: Page) =>
  scroller(win).evaluate((el) => {
    const tops: number[] = [];
    (window as unknown as { scrolls: number[] }).scrolls = tops;
    el.addEventListener("scroll", () => tops.push(el.scrollTop));
  });
const scrolls = (win: Page) =>
  win.evaluate(() => (window as unknown as { scrolls: number[] }).scrolls);

/** The focused element: its row key, whether the browser draws it as keyboard focus, and its aria-expanded. */
const focusState = (win: Page) =>
  win.evaluate(() => {
    const el = document.activeElement as HTMLElement;
    return {
      key: el.dataset.key ?? null,
      focusVisible: el.matches(":focus-visible"),
      expanded: el.getAttribute("aria-expanded"),
    };
  });

const chip = (win: Page, tag: string) => win.locator(`[data-testid=tag-chip][data-tag="${tag}"]`);

test("Only the rows in view are drawn", async () => {
  const { win } = await openBig();

  expect(await win.getByTestId("tree-node").count()).toBeLessThan(100);
  const rows = Number(await tree(win).getAttribute("data-rows"));
  expect(await tree(win).evaluate((el) => el.getBoundingClientRect().height)).toBe(
    rows * ROW + 2 * PAD,
  );
  // the scroll area is as tall as all the rows, so its scrollbar tells the truth
  expect(await scroller(win).evaluate((el) => el.scrollHeight)).toBe(rows * ROW + 2 * PAD);

  await scroller(win).evaluate((el) => (el.scrollTop = el.scrollHeight));
  await expect(nodeByKey(win, LAST_TEST)).toHaveAttribute("data-kind", "row");
  await expect(nodeByKey(win, LAST_TEST)).toContainText("outline 8.5.5 row 2000");
  expect((await rowAt(win, rows - 1))?.key).toBe(LAST_TEST);
  expect((await rowAt(win, rows - 1))?.inView).toBe(true);
  expect(await win.getByTestId("tree-node").count()).toBeLessThan(100);
});

test("A moved-to row scrolls into view", async () => {
  const { win } = await openBig();
  expect(await scroller(win).evaluate((el) => el.clientHeight)).toBeLessThan(40 * ROW); // it must scroll
  await item(win, await keyAt(win, 0)).focus();

  // holding the key: auto-repeat from the second press on
  for (let i = 0; i < 40; i++) await win.keyboard.down("ArrowDown");
  await win.keyboard.up("ArrowDown");

  await expect.poll(async () => (await focusedRow(win))?.index).toBe(40);
  expect((await focusedRow(win))?.inView).toBe(true);
  expect(await scrollTop(win)).toBeGreaterThan(0);
});

test("Focus survives scrolling away", async () => {
  const { win } = await openBig();
  const tenth = await keyAt(win, 9);
  await item(win, tenth).focus();
  await expect(item(win, tenth)).toBeFocused();

  await wheel(win, 100_000);
  await expect.poll(async () => (await scrollTop(win)) >= (await maxScroll(win)) - 1).toBe(true);
  // out of view, still in the page, still focused
  expect(await rowAt(win, 9)).toMatchObject({ key: tenth, inView: false, focused: true });
  await expect(item(win, tenth)).toBeFocused();

  await win.keyboard.press("ArrowDown");
  await expect.poll(async () => (await focusedRow(win))?.index).toBe(10);
  expect((await focusedRow(win))?.inView).toBe(true);
  expect(await scrollTop(win)).toBeLessThan(10 * ROW + PAD + 1); // scrolled back, not left at the bottom
});

test("A new tree keeps the rows in view still", async () => {
  test.setTimeout(120_000);
  const { win } = await openBig();
  await label(win, await keyAt(win, 9)).click(); // selected, and out of view below
  await scrollToRow(win, 1499);
  expect(await topInView(win)).toBe(ROW_1500);
  const top = await scrollTop(win);
  await recordScrolls(win);

  await nodeByKey(win, ROW_1500).getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Running…", { timeout: 60_000 });
  expect(await topInView(win)).toBe(ROW_1500);
  expect(await scrollTop(win)).toBe(top);

  // the status updates and the end of the run do not move the list either
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
  expect(await topInView(win)).toBe(ROW_1500);
  expect(await scrolls(win)).toEqual([]);
});

test("A filter change scrolls to the selected row", async () => {
  const { win } = await openBig();
  await scrollToRow(win, 1499);
  expect(await topInView(win)).toBe(ROW_1500);
  await label(win, ROW_1500).click();
  await expect(item(win, ROW_1500)).toHaveAttribute("aria-selected", "true");

  await chip(win, "module-5-5").click();
  await expect(chip(win, "module-5-5")).toHaveAttribute("data-state", "include");
  await scroller(win).evaluate((el) => (el.scrollTop = 0));
  await expect.poll(async () => (await rowAt(win, 1499))?.inView).toBe(false); // out of view again

  await win.getByTestId("hide-non-matching").click();
  await expect(win.getByTestId("hide-non-matching")).toBeChecked();
  // only module-5-5 and its parents are left
  await expect(tree(win)).toHaveAttribute("data-rows", String(3 + 5 * 12));
  await expect
    .poll(async () => (await drawn(win)).find((r) => r.key === ROW_1500)?.inView)
    .toBe(true);
});

// A mouse click is not keyboard focus (R4.10, R10.1): no ring, and no tooltip for a cut label.
test("A click shows neither the focus ring nor a tooltip", async () => {
  const { win } = await openBig();
  await label(win, CUT).click();

  await expect(item(win, CUT)).toBeFocused();
  expect(await focusState(win)).toMatchObject({ key: CUT, focusVisible: false });
  await win.waitForTimeout(700); // past the 500 ms a resting pointer needs
  await expect(win.getByRole("tooltip")).toHaveCount(0);
  expect(await focusState(win)).toMatchObject({ key: CUT, focusVisible: false });
});

test("A click on an HTTP call shows no focus ring", async () => {
  test.setTimeout(120_000);
  const dir = project("http");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/http.feature")).toBeVisible({ timeout: 15_000 });
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
  await win.getByTestId("tab-http").click();

  const call = win.locator('[data-testid=http-call][aria-posinset="3"]');
  await call.click();
  await expect(call).toBeFocused();
  await expect(call).toHaveAttribute("aria-selected", "true");
  expect(await call.evaluate((el) => el.matches(":focus-visible"))).toBe(false);
});

// The keyboard shows what a click does not (R4.9, R4.10).
test("The keyboard shows the focus ring and a cut label's tooltip", async () => {
  const { win } = await openBig();
  await item(win, BEFORE_CUT).focus();
  await expect(item(win, BEFORE_CUT)).toBeFocused();

  await win.keyboard.press("ArrowDown");
  await expect.poll(() => focusState(win)).toMatchObject({ key: CUT, focusVisible: true });
  const tip = win.getByRole("tooltip");
  await expect(tip).toHaveText(CUT_NAME);
  // over the row, as on hover
  const row = (await item(win, CUT).boundingBox())!;
  const over = (await tip.boundingBox())!;
  expect(over.y).toBeLessThan(row.y + row.height);
  expect(over.y + over.height).toBeGreaterThan(row.y);

  await win.keyboard.press("ArrowDown");
  await expect(tip).toHaveCount(0);
  await win.keyboard.press("ArrowUp");

  // Left and Right: to the parent, which closes and opens, then to the first child
  await win.keyboard.press("ArrowLeft");
  await expect.poll(() => focusState(win)).toMatchObject({ key: FEATURE_2, focusVisible: true });
  await win.keyboard.press("ArrowLeft");
  await expect
    .poll(() => focusState(win))
    .toMatchObject({ key: FEATURE_2, focusVisible: true, expanded: "false" });
  await win.keyboard.press("ArrowRight");
  await expect
    .poll(() => focusState(win))
    .toMatchObject({ key: FEATURE_2, focusVisible: true, expanded: "true" });
  await win.keyboard.press("ArrowRight");
  await expect
    .poll(() => focusState(win))
    .toMatchObject({ key: "s:tests/area-1/module-1/feature-2.feature#1", focusVisible: true });
});
