import type { ElectronApplication, Locator, Page } from "@playwright/test";
import { expect, launch, nodeByKey, openProject, project, test } from "./helpers";

const height = async (l: Locator) => (await l.boundingBox())!.height;
const panel = (win: Page) => win.getByTestId("bottom-panel");
const splitter = (win: Page) => win.getByTestId("splitter");
const near = (actual: number, expected: number) =>
  expect(Math.abs(actual - expected), `${actual} ≈ ${expected}`).toBeLessThanOrEqual(1);

/** Waits until the panel (and, when given, the tree/details area) is this tall, give or take 1 px. */
async function expectHeights(win: Page, want: { panel?: number; main?: number }) {
  if (want.panel !== undefined)
    await expect
      .poll(async () => Math.abs((await height(panel(win))) - want.panel!))
      .toBeLessThanOrEqual(1);
  if (want.main !== undefined)
    await expect
      .poll(async () => Math.abs((await height(win.locator("main"))) - want.main!))
      .toBeLessThanOrEqual(1);
}

/** Drags the splitter from its centre to `toY` (viewport px), in steps, and releases it there. */
async function dragTo(win: Page, toY: number) {
  const b = (await splitter(win).boundingBox())!;
  const x = b.x + b.width / 2;
  await win.mouse.move(x, b.y + b.height / 2);
  await win.mouse.down();
  await win.mouse.move(x, toY, { steps: 5 });
  await win.mouse.up();
}
const splitterY = async (win: Page) => {
  const b = (await splitter(win).boundingBox())!;
  return b.y + b.height / 2;
};
const dragBy = async (win: Page, dy: number) => dragTo(win, (await splitterY(win)) - dy);

const setWindowHeight = (app: ElectronApplication, h: number) =>
  app.evaluate(({ BrowserWindow }, h) => BrowserWindow.getAllWindows()[0].setSize(1200, h), h);

/**
 * The panel belongs to the project view, so a fresh start opens a sample project (clean unless `dir` is
 * given) and waits until it is saved as the last project; a restart reopens that one by itself.
 */
async function started(userData?: string, dir?: string) {
  const r = await launch(userData);
  if (!userData) {
    await openProject(r.app, dir ?? project("clean"));
    await expect(r.win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });
  }
  await expect(splitter(r.win)).toBeVisible();
  return r;
}

test("the bottom panel starts at its default height", async () => {
  const { win } = await started();
  await expectHeights(win, { panel: 192 });
});

test("dragging the splitter resizes the bottom panel", async () => {
  const dir = project("http");
  const { win } = await started(undefined, dir);
  await expect(nodeByKey(win, "f:tests/http.feature")).toBeVisible({ timeout: 15_000 });
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
  await win.getByTestId("tab-http").click();
  const before = await height(win.getByTestId("http-calls"));

  await dragBy(win, 200);
  await expectHeights(win, { panel: 392 });
  near(await height(win.getByTestId("http-calls")), before + 200);
});

test("the splitter works from the keyboard", async () => {
  const { win } = await started();
  for (
    let i = 0;
    i < 40 && !(await splitter(win).evaluate((e) => e === document.activeElement));
    i++
  )
    await win.keyboard.press("Tab");
  await expect(splitter(win)).toBeFocused();

  await win.keyboard.press("ArrowUp");
  await win.keyboard.press("ArrowUp");
  await expectHeights(win, { panel: 224 });
  await expect(splitter(win)).toHaveAttribute("aria-valuenow", "224");
  await win.keyboard.press("Home");
  await expectHeights(win, { panel: 96 });
  await win.keyboard.press("End");
  await expectHeights(win, { main: 120 });
});

test("the panel stays within its limits", async () => {
  const { win } = await started();
  const bottom = await win.evaluate(() => window.innerHeight);
  await dragTo(win, bottom);
  await expectHeights(win, { panel: 96 });

  await dragTo(win, 0);
  await expectHeights(win, { main: 120 });
  const header = (await win.locator("header").boundingBox())!;
  expect(header.y).toBeGreaterThanOrEqual(0);
  expect(header.y + header.height).toBeLessThanOrEqual(bottom);
});

test("the window cannot get too short for the layout", async () => {
  const { app, win } = await started();
  await setWindowHeight(app, 300);
  await expect
    .poll(() => app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].getSize()[1]))
    .toBe(480);
  expect(await height(win.locator("main"))).toBeGreaterThanOrEqual(119);
  expect(await height(panel(win))).toBeGreaterThanOrEqual(95);
  expect(
    await win.evaluate(
      () => document.scrollingElement!.scrollHeight <= document.scrollingElement!.clientHeight,
    ),
  ).toBe(true);
});

/** 600 px tall fits CI's small screen, so the restart (F41) restores the same height whatever the first start's. */
async function fitScreen(app: ElectronApplication, win: Page) {
  const width = await app.evaluate(
    ({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].getSize()[0],
  );
  await app.evaluate(
    ({ BrowserWindow }, w) => BrowserWindow.getAllWindows()[0].setSize(w, 600),
    width,
  );
  await expect.poll(() => win.evaluate(() => window.innerHeight)).toBeLessThan(600);
}

test("the height survives a restart", async () => {
  const first = await started();
  await fitScreen(first.app, first.win);
  // 250, not 300: the project view's header leaves CI's 600 px window room for a panel of only about 290
  await dragBy(first.win, 58);
  await expectHeights(first.win, { panel: 250 });
  await first.app.close();

  const { win } = await started(first.userData);
  await expectHeights(win, { panel: 250 });
});

test("a smaller window shrinks the panel without forgetting its height", async () => {
  const { app, win } = await started();
  // 700 px fits CI's small screen, where an 800 px window is cut down; the target comes from the room left.
  await setWindowHeight(app, 700);
  await expect.poll(() => win.evaluate(() => window.innerHeight)).toBeLessThan(700);
  const room = (await height(win.locator("main"))) + (await height(panel(win)));
  const want = Math.round(room) - 120 - 40;
  await dragBy(win, want - 192);
  await expectHeights(win, { panel: want });

  await setWindowHeight(app, 500);
  await expectHeights(win, { main: 120 });
  await setWindowHeight(app, 700);
  await expectHeights(win, { panel: want });
});

test("a bad saved height falls back to the default", async () => {
  const { win } = await started();
  for (const v of ["abc", "-5", "50"]) {
    await win.evaluate((v) => localStorage.setItem("karate-kid.panelHeight", v), v);
    await win.reload();
    await expect(splitter(win)).toBeVisible();
    await expectHeights(win, { panel: 192 });
  }
});

test("a saved height taller than the window is clamped at start", async () => {
  const { win } = await started();
  await win.evaluate(() => localStorage.setItem("karate-kid.panelHeight", "2000"));
  await win.reload();
  await expect(splitter(win)).toBeVisible();
  await expectHeights(win, { main: 120 });
});

test("releasing a drag above the window keeps the maximum and ends the drag", async () => {
  const first = await started();
  await fitScreen(first.app, first.win);
  await dragTo(first.win, -100);
  await expectHeights(first.win, { main: 120 });
  const shown = await height(panel(first.win));
  expect(await first.win.getByTestId("app").evaluate((e) => getComputedStyle(e).cursor)).not.toBe(
    "row-resize",
  );
  await first.app.close();

  const { win } = await started(first.userData);
  await expectHeights(win, { panel: shown });
});

test("focus cannot scroll the toolbar out of a cut-off layout", async () => {
  const { app, win } = await started();
  await app.evaluate(({ BrowserWindow }) => {
    const w = BrowserWindow.getAllWindows()[0];
    w.setSize(520, 480);
    w.webContents.send("app:event", { type: "notice", message: "a long notice ".repeat(300) });
  });
  const root = win.getByTestId("app");
  await expect
    .poll(() => root.evaluate((e) => e.scrollHeight > e.clientHeight)) // the panel is cut off
    .toBe(true);

  await win.getByTestId("tab-http").focus();
  expect(await root.evaluate((e) => e.scrollTop)).toBe(0);
  expect((await win.locator("header").boundingBox())!.y).toBeGreaterThanOrEqual(0);
});

test("Tab moves focus past the splitter", async () => {
  const { win } = await started();
  await splitter(win).focus();
  await win.keyboard.press("Tab");
  await expect(splitter(win)).not.toBeFocused();
  await expectHeights(win, { panel: 192 });
});
