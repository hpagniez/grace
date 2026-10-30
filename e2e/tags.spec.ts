import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { expect, launch, node, nodeByKey, openProject, project, test } from "./helpers";

// Keys of the clean sample project; `node()` cannot find these once they are dimmed (no run button).
const FAILING_MATCH = "s:tests/basic.feature#2";
const SMOKE_BUT_WIP = "s:tests/basic.feature#4";
const OUTLINE_FEATURE = "f:tests/outline.feature";
// Keys of the tags sample project.
const OTHER = "s:tests/tags.feature#1";
const OUTLINE = "o:tests/tags.feature#2";
const row = (n: number) => `s:tests/tags.feature#2.${n}`;

const chip = (win: Page, tag: string) => win.locator(`[data-testid=tag-chip][data-tag="${tag}"]`);

/** Clicks the chip once per state given and waits for main to echo each: the window never assumes. */
async function clickChip(win: Page, tag: string, ...states: string[]) {
  for (const state of states) {
    await chip(win, tag).click();
    await expect(chip(win, tag)).toHaveAttribute("data-state", state);
  }
}

async function hideNonMatching(win: Page) {
  await win.getByTestId("hide-non-matching").click();
  await expect(win.getByTestId("hide-non-matching")).toBeChecked();
}

async function openClean() {
  const dir = project("clean");
  const { app, win, userData } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  return { dir, win, userData };
}

/** The clean sample project with "@smoke" included and "@wip" excluded. */
async function openCleanSmokeNotWip() {
  const opened = await openClean();
  await clickChip(opened.win, "smoke", "include");
  await clickChip(opened.win, "wip", "include", "exclude");
  return opened;
}

async function openTags() {
  const dir = project("tags");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, OUTLINE)).toBeVisible({ timeout: 15_000 });
  return { dir, win };
}

/** Like another editor: strips every "@wip" tag from basic.feature. */
function removeWipTags(dir: string) {
  const file = path.join(dir, "tests", "basic.feature");
  writeFileSync(file, readFileSync(file, "utf8").replace(/@wip/g, ""));
}

const treeKeys = (win: Page) =>
  win
    .locator("[data-testid=tree-node]")
    .evaluateAll((els) => els.map((e) => e.getAttribute("data-key")));

test("Chips list the project's tags", async () => {
  const { win } = await openClean();

  await expect(win.getByTestId("filter-bar")).toBeVisible();
  await expect(win.getByTestId("tag-chip")).toHaveText(["@slow", "@smoke", "@wip"]);
  for (const tag of ["slow", "smoke", "wip"])
    await expect(chip(win, tag)).toHaveAttribute("data-state", "off");
  await expect(win.getByTestId("tag-counter")).toHaveCount(0);
});

test("Include and exclude dim the tree", async () => {
  const { win } = await openClean();

  await clickChip(win, "smoke", "include");
  await clickChip(win, "wip", "include", "exclude");
  await expect(win.getByTestId("tag-counter")).toHaveText("2 of 9 tests match");
  for (const name of ["passing hello", "smoke and slow"]) {
    await expect(node(win, name)).toHaveAttribute("data-match", "true");
  }
  for (const key of [FAILING_MATCH, SMOKE_BUT_WIP, OUTLINE_FEATURE]) {
    await expect(nodeByKey(win, key)).toHaveAttribute("data-match", "false");
    await expect(nodeByKey(win, key).getByTestId("run-node")).toHaveCount(0);
  }
});

test("Third click turns a chip off", async () => {
  const { win } = await openClean();
  await clickChip(win, "smoke", "include");
  await expect(win.locator("[data-testid=tree-node][data-match=false]").first()).toBeVisible();

  await clickChip(win, "smoke", "exclude", "off");
  await expect(win.locator("[data-testid=tree-node][data-match=false]")).toHaveCount(0);
  await expect(win.getByTestId("tag-counter")).toHaveCount(0);
});

test("Hide non-matching", async () => {
  const { win } = await openCleanSmokeNotWip();

  await hideNonMatching(win);
  await expect
    .poll(() => treeKeys(win))
    .toEqual([
      "d:tests",
      "f:tests/basic.feature",
      "s:tests/basic.feature#1",
      "f:tests/slow.feature",
      "s:tests/slow.feature#2",
    ]);
  await expect(node(win, "passing hello")).toBeVisible();
  await expect(node(win, "smoke and slow")).toBeVisible();
});

test("Run all runs only matching tests", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  // "failing match" gets a status first, to see that a filtered run leaves it alone.
  await node(win, "failing match").getByTestId("run-node").click();
  await expect(nodeByKey(win, FAILING_MATCH)).toHaveAttribute("data-status", "failed", {
    timeout: 30_000,
  });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await clickChip(win, "smoke", "include");
  await clickChip(win, "wip", "include", "exclude");

  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 2", { timeout: 15_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("2 / 2");
  await expect(win.getByTestId("progress-passed")).toHaveText("2");
  await expect(win.getByTestId("progress-failed")).toHaveText("0");
  await expect(node(win, "passing hello")).toHaveAttribute("data-status", "passed");
  await expect(node(win, "smoke and slow")).toHaveAttribute("data-status", "passed");
  await expect(nodeByKey(win, FAILING_MATCH)).toHaveAttribute("data-status", "failed");
  await expect(nodeByKey(win, SMOKE_BUT_WIP)).toHaveAttribute("data-status", "notRun");
});

test("A feature run is filtered", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  await clickChip(win, "wip", "include", "exclude");

  await node(win, "basic.feature").getByTestId("run-node").click();
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 2", { timeout: 15_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("2 / 2");
  await expect(win.getByTestId("progress-passed")).toHaveText("2");
  await expect(win.getByTestId("progress-failed")).toHaveText("0");
  await expect(node(win, "passing hello")).toHaveAttribute("data-status", "passed");
  await expect(node(win, "calls helper feature")).toHaveAttribute("data-status", "passed");
  await expect(nodeByKey(win, FAILING_MATCH)).toHaveAttribute("data-status", "notRun");
  await expect(nodeByKey(win, SMOKE_BUT_WIP)).toHaveAttribute("data-status", "notRun");
});

test("A tag removed before a run does not widen it", async () => {
  test.setTimeout(90_000);
  const { dir, win } = await openClean();
  await clickChip(win, "wip", "include");

  removeWipTags(dir);
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("notice")).toHaveText("Nothing to run", { timeout: 30_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("");
  await expect(win.getByTestId("progress-text")).toHaveText(/^\d+ tests? · Not run yet$/);
  // the pre-run dry run sees the new tree but prunes nothing
  await expect(chip(win, "wip")).toHaveAttribute("data-state", "include");
});

test("Part of an outline matches", async () => {
  test.setTimeout(90_000);
  const { win } = await openTags();
  await clickChip(win, "fast", "include");

  await nodeByKey(win, OUTLINE).getByTestId("run-node").click();
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 2", { timeout: 15_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("2 / 2");
  await expect(win.getByTestId("progress-passed")).toHaveText("2");
  await expect(win.getByTestId("progress-failed")).toHaveText("0");
  await expect(nodeByKey(win, row(1))).toHaveAttribute("data-status", "passed");
  await expect(nodeByKey(win, row(2))).toHaveAttribute("data-status", "passed");
  await expect(nodeByKey(win, row(3))).toHaveAttribute("data-status", "notRun");
  await expect(nodeByKey(win, OTHER)).toHaveAttribute("data-status", "notRun");
});

test("Nothing matches", async () => {
  const { win } = await openTags();
  await expect(win.getByTestId("run-all-button")).toBeEnabled();

  await clickChip(win, "feat", "include", "exclude");
  await expect(win.getByTestId("tag-counter")).toHaveText("0 of 4 tests match");
  await expect(win.getByTestId("run-all-button")).toBeDisabled();
});

test("The filter survives a restart", async () => {
  const dir = project("clean");
  const first = await launch();
  await openProject(first.app, dir);
  await expect(chip(first.win, "smoke")).toBeVisible({ timeout: 15_000 });
  await clickChip(first.win, "smoke", "include");
  await hideNonMatching(first.win);
  // the checkbox ticks on click; the tree hides only once main has saved and echoed the filter
  await expect(nodeByKey(first.win, OUTLINE_FEATURE)).toHaveCount(0);

  // one app per user-data folder: close it before the second starts
  await first.app.close();
  const { win } = await launch(first.userData);
  await expect(chip(win, "smoke")).toHaveAttribute("data-state", "include", { timeout: 15_000 });
  await expect(win.getByTestId("hide-non-matching")).toBeChecked();
  await expect(chip(win, "wip")).toHaveAttribute("data-state", "off");
  await expect(nodeByKey(win, OUTLINE_FEATURE)).toHaveCount(0);
});

test("A tag removed from the project leaves the filter", async () => {
  const { dir, win, userData } = await openClean();
  await clickChip(win, "wip", "include", "exclude");

  removeWipTags(dir);
  await win.getByTestId("refresh-button").click();
  await expect(chip(win, "wip")).toHaveCount(0, { timeout: 30_000 });
  await expect(win.getByTestId("tag-chip")).toHaveText(["@slow", "@smoke"]);
  await expect(win.getByTestId("tag-counter")).toHaveCount(0);
  await expect(win.locator("[data-testid=tree-node][data-match=false]")).toHaveCount(0);
  const saved = () =>
    JSON.parse(readFileSync(path.join(userData, "settings.json"), "utf8")).projects[
      path.resolve(dir)
    ];
  expect(saved()).toEqual({ include: [], exclude: [], hideNonMatching: false });
});
