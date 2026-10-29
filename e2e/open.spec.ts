import type { ElectronApplication, Page } from "@playwright/test";
import { mkdirSync, rmSync } from "node:fs";
import path from "node:path";
import {
  emptyProject,
  expect,
  launch,
  node,
  nodeByKey,
  openProject,
  project,
  recordOpenDialog,
  test,
} from "./helpers";

test("Open a project and see its tests", async () => {
  const dir = project("clean");
  const { app, win, userData } = await launch();
  expect(await app.evaluate(({ app }) => app.getPath("userData"))).toBe(userData);
  await expect(win.getByTestId("home")).toBeVisible();

  await openProject(app, dir);
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 2000 });
  await expect(nodeByKey(win, "d:tests")).toContainText("tests");
  for (const f of ["basic", "outline", "slow"]) {
    const feature = win.locator(`li:has(> [data-key="d:tests"]) [data-key="f:tests/${f}.feature"]`);
    await expect(feature).toBeVisible();
    await expect(feature).toContainText(`${f}.feature`);
    await expect(node(win, `${f}.feature`)).toHaveAttribute("data-key", `f:tests/${f}.feature`);
  }
  const outline = nodeByKey(win, "o:tests/outline.feature#1");
  await expect(outline).toContainText("add <a> and <b>");
  await expect(
    win.locator(
      `li:has(> [data-key="o:tests/outline.feature#1"]) [data-testid=tree-node][data-kind=row]`,
    ),
  ).toHaveCount(3);
  await expect(win.locator('[data-testid=tree-node][data-key*="common/create-user"]')).toHaveCount(
    0,
  );
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(dir));
  await expect(win.getByTestId("project-layout")).toHaveText("Standalone");
});

test("The toolbar cuts a long path from its start and keeps its characters in order", async () => {
  // a folder name that ends in a neutral character: right-to-left text would move it to the front
  const dir = path.join(emptyProject(), "final copy!");
  mkdirSync(dir);
  const { app, win } = await launch();
  await openProject(app, dir);
  const full = path.resolve(dir);
  const box = win.getByTestId("project-path");
  await expect(box).toHaveText(full);
  // pin its width, so that the path is longer than the box on every machine
  await box.evaluate((el) => void ((el.parentElement as HTMLElement).style.flex = "0 0 160px"));
  const seen = await box.evaluate((el) => {
    const text = el.firstElementChild!.firstChild as Text;
    const at = (i: number) => {
      const r = document.createRange();
      r.setStart(text, i);
      r.setEnd(text, i + 1);
      return r.getBoundingClientRect();
    };
    const view = el.getBoundingClientRect();
    const last = text.length - 1;
    const probes = [0, 1, 2, last - 2, last - 1, last];
    return {
      startCut: at(0).right <= view.left + 0.5,
      endShown: at(last).left >= view.left && at(last).right <= view.right + 0.5,
      leftToRight: probes.every((i, k) => k === 0 || at(i).left > at(probes[k - 1]).left),
    };
  });
  expect(seen).toEqual({ startCut: true, endShown: true, leftToRight: true });
  await box.hover();
  await expect(win.getByRole("tooltip")).toHaveText(full);
});

const TOOLBAR_BUTTONS = {
  "home-button": "Home",
  "properties-button": "Properties…",
  "refresh-button": "Refresh",
  "open-button": "Open…",
  "report-button": "Report",
  "rerun-failed-button": /^Re-run failed \d+$/,
  "cancel-button": "Cancel",
  "run-all-button": "Run all",
};

/** Sets the window's content width, and waits until the page has it. */
async function contentWidth(app: ElectronApplication, win: Page, width: number) {
  await app.evaluate(
    ({ BrowserWindow }, w) => BrowserWindow.getAllWindows()[0].setContentSize(w, 700),
    width,
  );
  await expect.poll(() => win.evaluate(() => innerWidth)).toBe(width);
}

/** Every toolbar button is whole inside the window, and the toolbar does not overflow. */
async function expectToolbarFits(win: Page) {
  const width = await win.evaluate(() => innerWidth);
  const seen = await win.locator("header button").evaluateAll((buttons) =>
    buttons.map((b) => {
      const r = b.getBoundingClientRect();
      return {
        id: (b as HTMLElement).dataset.testid,
        left: r.left,
        right: r.right,
        width: r.width,
      };
    }),
  );
  expect(seen.map((b) => b.id)).toEqual(Object.keys(TOOLBAR_BUTTONS));
  for (const b of seen) {
    expect(b.left, `${b.id} left at ${width}`).toBeGreaterThanOrEqual(0);
    expect(b.right, `${b.id} right at ${width}`).toBeLessThanOrEqual(width);
  }
  expect(await win.locator("header").evaluate((h) => h.scrollWidth <= h.clientWidth)).toBe(true);
  return seen;
}

/** The layout chip, when it shows, ends before the Env label starts. */
async function chipClearOfEnv(win: Page) {
  const chip = await win.getByTestId("project-layout").boundingBox();
  const env = await win.locator("label", { has: win.getByTestId("env-input") }).boundingBox();
  if (chip) expect(chip.x + chip.width).toBeLessThanOrEqual(env!.x);
  return chip;
}

/** The chip is in sight, whole, and clear of the Env label: its text is not cut. */
async function expectChipShown(win: Page) {
  const chip = win.getByTestId("project-layout");
  await expect(chip).toBeVisible();
  expect(await chipClearOfEnv(win)).not.toBeNull();
  // the whole word: its text fits the chip's content box, to a hundredth of a pixel
  const room = await chip.evaluate((el) => {
    const text = document.createRange();
    text.selectNodeContents(el);
    const css = getComputedStyle(el);
    const box = el.getBoundingClientRect().width;
    const padding = parseFloat(css.paddingLeft) + parseFloat(css.paddingRight);
    return box - padding - text.getBoundingClientRect().width;
  });
  expect(room).toBeGreaterThanOrEqual(-0.01);
}

const hasTooltip = (win: Page) => win.locator('[role="tooltip"]');

test("A narrow window keeps every toolbar button on screen, with its name", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  const minimum = await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].getMinimumSize(),
  );
  expect(minimum[0]).toBe(760);
  await contentWidth(app, win, 780);
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });
  for (const [id, name] of Object.entries(TOOLBAR_BUTTONS))
    await expect(win.getByTestId(id), id).toHaveAccessibleName(name);

  const seen = await expectToolbarFits(win);
  // below 1100 px four labels are only spoken: those buttons are their icon, and say it in a tooltip
  for (const id of ["properties-button", "open-button", "report-button", "cancel-button"])
    expect(seen.find((b) => b.id === id)!.width, id).toBeLessThan(40);
  await chipClearOfEnv(win);
  expect((await win.getByTestId("project-name").boundingBox())!.width).toBeGreaterThan(30);
  await win.getByTestId("open-button").hover();
  await expect(win.getByRole("tooltip")).toHaveText("Open…");
  await win.mouse.move(5, 400);

  // the floor: the minimum is an outer width, so the content is narrower than 780 px
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1, 700),
  );
  await expect
    .poll(() =>
      app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].getBounds().width),
    )
    .toBe(760);
  const floor = await win.evaluate(() => innerWidth);
  expect(floor).toBeLessThan(780);
  await expectToolbarFits(win);
  await chipClearOfEnv(win);
  for (const id of ["run-all-button", "cancel-button"]) {
    const box = (await win.getByTestId(id).boundingBox())!;
    expect(box.x + box.width, `${id} inside ${floor} px`).toBeLessThanOrEqual(floor);
  }

  // just over 1100 px the labels show and the toolbar is at its tightest: the chip still shows
  await contentWidth(app, win, 1101);
  await expect
    .poll(() =>
      win
        .getByTestId("open-button")
        .boundingBox()
        .then((b) => b!.width),
    )
    .toBeGreaterThan(60);
  await expectToolbarFits(win);
  await expect(win.getByTestId("project-layout")).toBeVisible();
  const tight = await chipClearOfEnv(win);
  expect(tight!.width).toBeGreaterThan(15); // cut short, with its ellipsis, not collapsed

  // the common widths: labels, a whole chip, and no tooltip that only repeats a label
  for (const width of [1200, 1500]) {
    await contentWidth(app, win, width);
    await expectToolbarFits(win);
    await expectChipShown(win);
    expect(
      (await win.getByTestId("open-button").boundingBox())!.width,
      `open at ${width}`,
    ).toBeGreaterThan(60);
    await win.getByTestId("open-button").hover();
    await win.waitForTimeout(800);
    await expect(hasTooltip(win)).toHaveCount(0);
    await win.mouse.move(5, 400);
  }
});

test("Maven layout with classpath resources", async () => {
  const dir = project("maven");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(win.getByTestId("project-layout")).toHaveText("Maven");
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });

  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("1 / 1");
  await expect(win.getByTestId("progress-passed")).toHaveText("1");
  await expect(win.getByTestId("progress-failed")).toHaveText("0");
});

test("Syntax error is reported, not hidden", async () => {
  const dir = project("broken");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(win.getByTestId("tree-message")).toContainText("syntax.feature", {
    timeout: 15_000,
  });
  await expect(win.getByTestId("run-all-button")).toBeDisabled();
});

test("Nothing to run", async () => {
  const dir = emptyProject();
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(win.getByTestId("tree-message")).toHaveText("No feature files found");
  await expect(win.getByTestId("run-all-button")).toBeDisabled();
});

test("Open… starts in the last project's parent folder", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
  const calls = await recordOpenDialog(app);

  await win.getByTestId("open-button").click();
  await expect
    .poll(calls)
    .toEqual([{ parent: true, defaultPath: path.dirname(path.resolve(dir)) }]);
});

test("Open… with no last project", async () => {
  const { app, win } = await launch();
  const calls = await recordOpenDialog(app);
  await win.getByTestId("open-button").click();
  await expect.poll(calls).toEqual([{ parent: true }]);
});

test("the last project folder was deleted", async () => {
  const dir = project("clean");
  const first = await launch();
  await openProject(first.app, dir);
  await expect(nodeByKey(first.win, "d:tests")).toBeVisible({ timeout: 15_000 });
  await first.app.close();
  rmSync(dir, { recursive: true, force: true });

  const { app, win } = await launch(first.userData);
  const calls = await recordOpenDialog(app);
  await win.getByTestId("open-button").click();
  await expect.poll(calls).toEqual([{ parent: true }]);
});
