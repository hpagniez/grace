import type { Page } from "@playwright/test";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import path from "node:path";
import {
  emptyProject,
  expect,
  launch,
  launchWithSettings,
  node,
  nodeByKey,
  openProject,
  ourJavaPids,
  project,
  recordOpenDialog,
  settingsOf,
  test,
} from "./helpers";

const rows = (win: Page) => win.getByTestId("recent-row");

/** The absolute path of a folder that existed and was deleted. */
function deleted(): string {
  const dir = emptyProject();
  rmSync(dir, { recursive: true, force: true });
  return path.resolve(dir);
}

/** Opens the clean project from the folder picker and waits for its tests. */
async function openClean(app: Parameters<typeof openProject>[0], win: Page, dir: string) {
  await openProject(app, dir);
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
}

// R1: when it shows

test("A first start shows the home screen", async () => {
  const { win } = await launch();
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(win.getByRole("heading", { level: 1 })).toHaveText("Open a Karate project");
  await expect(
    win.getByText(
      "Pick the folder that holds your .feature files. Grace brings its own Java and Karate.",
    ),
  ).toBeVisible();
  await expect(win.getByTestId("open-button")).toHaveText("Open folder…");
  await expect(win.getByTestId("open-button").locator("svg")).toBeVisible();
  const mascot = win.getByTestId("mascot");
  await expect(mascot).toBeVisible();
  // loaded, so the CSP let it through
  await expect.poll(() => mascot.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(400);
  await expect(win.getByText("Recent folders")).toHaveCount(0);
  for (const id of [
    "recent-list",
    "home-button",
    "project-path",
    "tree",
    "tree-message",
    "details",
    "bottom-panel",
  ])
    await expect(win.getByTestId(id)).toHaveCount(0);
});

test("Opening a folder from the home screen", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await expect(win.getByTestId("home")).toBeVisible();
  await openClean(app, win, dir);
  await expect(win.getByTestId("home")).toHaveCount(0);
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(dir));
});

test("The last project opens at start without the home screen", async () => {
  const dir = project("clean");
  const first = await launch();
  await openClean(first.app, first.win, dir);
  await first.app.close();

  const { win } = await launch(first.userData);
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(dir));
  await expect(win.getByTestId("tree-message")).toHaveText("Finding tests…");
  await expect(win.getByTestId("home")).toHaveCount(0);
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("home")).toHaveCount(0);
});

test("A missing last project shows the home screen", async () => {
  const gone = deleted();
  const { win } = await launchWithSettings({ lastProject: gone, recentProjects: [gone] });
  await expect(win.getByTestId("home")).toBeVisible();
  const row = rows(win);
  await expect(row).toHaveCount(1);
  await expect(row).toBeDisabled();
  await expect(win.getByTestId("recent-name")).toHaveText(path.basename(gone));
  await expect(win.getByTestId("recent-missing")).toHaveText("Not found");
  await expect(win.getByText("Pick a folder, or reopen one you used before.")).toBeVisible();

  // a disabled row still explains itself: the tooltip listens on its wrapper
  await row.hover({ force: true });
  await expect(win.getByRole("tooltip")).toHaveText("Folder not found");
  await win.mouse.move(0, 0);

  await row.click({ force: true }); // a disabled button takes no click
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(win.getByTestId("project-path")).toHaveCount(0);
  await expect(win.getByTestId("notice")).toHaveCount(0);
});

test("A dry-run error still opens the project", async () => {
  const dir = project("broken");
  const { app, win } = await launch();
  await expect(win.getByTestId("home")).toBeVisible();
  await openProject(app, dir);
  await expect(win.getByTestId("tree-message")).toContainText("syntax.feature", {
    timeout: 15_000,
  });
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(dir));

  await win.getByTestId("home-button").click();
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(rows(win).first()).toContainText(path.resolve(dir));
});

// R12: the home screen's ground

const homeBackground = async (colorScheme: "light" | "dark") => {
  const { app, win } = await launch(undefined, {}, { colorScheme });
  const bg = await win.getByTestId("home").evaluate((e) => getComputedStyle(e).backgroundColor);
  await app.close();
  return bg;
};

test("The home screen is on the ground in light and on the surface in dark", async () => {
  expect(await homeBackground("light")).toBe("rgb(243, 239, 230)"); // #f3efe6
  expect(await homeBackground("dark")).toBe("rgb(16, 18, 23)"); // #101217
});

// R2: the home screen

test("Reopening a folder from the list", async () => {
  const clean = project("clean");
  const tags = project("tags");
  const { app, win } = await launch();
  await openClean(app, win, clean);
  await openProject(app, tags);
  await expect(nodeByKey(win, "f:tests/tags.feature")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(tags));

  await win.getByTestId("home-button").click();
  await expect(rows(win)).toHaveCount(2);
  await expect(rows(win).nth(0)).toContainText(path.resolve(tags));
  await expect(rows(win).nth(1)).toContainText(path.resolve(clean));

  await rows(win).nth(1).click();
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(clean));

  await win.getByTestId("home-button").click();
  await expect(rows(win)).toHaveCount(2);
  await expect(rows(win).nth(0)).toContainText(path.resolve(clean));
  await expect(rows(win).nth(1)).toContainText(path.resolve(tags));
});

test("The folder dialog starts next to the newest existing folder", async () => {
  // Three different parents, so the starting folder tells which row was picked.
  const base = emptyProject();
  const gone = path.join(base, "one", "gone");
  const a = path.join(base, "two", "a");
  const b = path.join(base, "three", "b");
  mkdirSync(a, { recursive: true });
  mkdirSync(b, { recursive: true });
  const { app, win } = await launchWithSettings({ recentProjects: [gone, a, b] });
  await expect(rows(win)).toHaveCount(3);
  const calls = await recordOpenDialog(app);

  await win.getByTestId("open-button").click();
  await expect.poll(calls).toEqual([{ parent: true, defaultPath: path.dirname(a) }]);
});

test("Removing a row", async () => {
  const dir = project("clean");
  const clean = path.resolve(dir);
  const { win, userData } = await launchWithSettings({
    recentProjects: [clean],
    projects: { [clean]: { threads: 2 } },
  });
  const remove = win.getByTestId("recent-remove");
  await expect(remove).toHaveAttribute("aria-label", "Remove from list");
  const size = (await remove.boundingBox())!; // R10.3: a target of at least 24 × 24
  expect(size.width).toBeGreaterThanOrEqual(24);
  expect(size.height).toBeGreaterThanOrEqual(24);

  await remove.click();
  await expect(win.getByTestId("recent-list")).toHaveCount(0);
  await expect(win.getByTestId("home")).toBeVisible();
  await expect.poll(() => settingsOf(userData).recentProjects).toEqual([]);
  expect(
    (settingsOf(userData).projects as Record<string, { threads: number }>)[clean].threads,
  ).toBe(2);
  expect(existsSync(clean)).toBe(true);
});

test("The list works from the keyboard", async () => {
  const dir = project("clean");
  const { win } = await launchWithSettings({ recentProjects: [path.resolve(dir)] });
  await expect(rows(win)).toBeVisible();

  let focused = false;
  for (let i = 0; i < 5 && !focused; i++) {
    await win.keyboard.press("Tab");
    focused = await rows(win).evaluate((el) => el === document.activeElement);
  }
  expect(focused).toBe(true);
  await win.keyboard.press("Enter");
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
});

// R3: the Home button

test("Home closes the project and survives a restart", async () => {
  const dir = project("clean");
  const first = await launch();
  await openClean(first.app, first.win, dir);

  await first.win.getByTestId("home-button").click();
  await expect(first.win.getByTestId("home")).toBeVisible();
  await expect(rows(first.win).first()).toContainText(path.resolve(dir));
  expect(settingsOf(first.userData)).not.toHaveProperty("lastProject");
  await first.app.close();

  const { win } = await launch(first.userData);
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(win.getByTestId("project-path")).toHaveCount(0);
  await expect(rows(win).first()).toContainText(path.resolve(dir));
});

test("Home is disabled during a run", async () => {
  test.setTimeout(90_000);
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  const slow = nodeByKey(win, "f:tests/slow.feature");
  await expect(slow).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("home-button")).toBeEnabled();

  await slow.getByTestId("run-node").click();
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
  await expect(win.getByTestId("home-button")).toBeDisabled();

  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await expect(win.getByTestId("home-button")).toBeEnabled();

  await win.getByTestId("home-button").click();
  await expect(win.getByTestId("home")).toBeVisible();
  await expect.poll(ourJavaPids, { timeout: 15_000 }).toEqual([]);
});

test("A reload after Home stays home", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await openClean(app, win, dir);
  await win.getByTestId("home-button").click();
  await expect(win.getByTestId("home")).toBeVisible();

  await win.reload();
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(win.getByTestId("project-path")).toHaveCount(0);
  await expect(rows(win).first()).toContainText(path.resolve(dir));
});

// R5: calls from the renderer

test("A path not in the list is refused", async () => {
  const clean = project("clean");
  const tags = project("tags");
  const { win } = await launchWithSettings({ recentProjects: [path.resolve(clean)] });
  await expect(rows(win)).toBeVisible();

  await win.evaluate((d) => window.api.openRecent(d), path.resolve(tags));
  await expect(win.getByTestId("notice")).toHaveText("Invalid request");
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(win.getByTestId("project-path")).toHaveCount(0);
});
