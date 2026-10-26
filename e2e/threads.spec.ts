import { writeFileSync } from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import {
  expect,
  launch,
  node,
  nodeByKey,
  openProject,
  project,
  ranThreads,
  saved,
  test,
} from "./helpers";

const tab = (win: Page) => win.getByTestId("tab-history");
const rows = (win: Page) => win.getByTestId("history-row");
const box = (win: Page) => win.getByTestId("threads-input");
const notice = (win: Page) => win.getByTestId("notice");

/** The clean sample project in a fresh app; `pom` is written to its karate-pom.json first. */
async function openClean(pom?: object) {
  const dir = project("clean");
  if (pom) writeFileSync(path.join(dir, "karate-pom.json"), JSON.stringify(pom));
  const { app, win, userData } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  return { dir, app, win, userData };
}

/** Waits until the run is in the history (`n` runs): a stored run has ended. */
async function finished(win: Page, n: number) {
  await expect(tab(win)).toHaveText(`History (${n})`, { timeout: 90_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished");
}

/** Run all, then waits until the run is in the history (`n` runs). */
async function runAll(win: Page, n: number) {
  await win.getByTestId("run-all-button").click();
  await finished(win, n);
}

/** The History row `i` (0 is the newest) shows `n` threads, and Karate reports `n` for that run. */
async function expectRow(win: Page, userData: string, dir: string, i: number, n: number) {
  await tab(win).click();
  const row = rows(win).nth(i);
  await expect(row.getByTestId("history-threads")).toHaveText(String(n));
  expect(ranThreads(userData, dir, (await row.getAttribute("data-id"))!)).toBe(n);
}

/** Types the count and presses Enter. */
async function setThreads(win: Page, value: string) {
  await box(win).fill(value);
  await box(win).press("Enter");
}

test("A project with no saved count runs with 1 thread", async () => {
  test.setTimeout(120_000);
  const { dir, win, userData } = await openClean();
  await expect(box(win)).toBeEnabled(); // a disabled box shows 1 before main sends the count
  await expect(box(win)).toHaveValue("1");

  await runAll(win, 1);
  await expectRow(win, userData, dir, 0, 1);
});

test("Run all uses the count in the box", async () => {
  test.setTimeout(120_000);
  const { dir, win, userData } = await openClean();

  // no Enter: pressing Run all leaves the box, which commits the count
  await box(win).fill("2");
  await runAll(win, 1);
  await expectRow(win, userData, dir, 0, 2);
});

test("A single test's run uses the count", async () => {
  test.setTimeout(120_000);
  const { dir, win, userData } = await openClean();

  await setThreads(win, "2");
  await node(win, "passing hello").getByTestId("run-node").click();
  await finished(win, 1);
  await expectRow(win, userData, dir, 0, 2);
});

test("Re-run failed uses the current count", async () => {
  test.setTimeout(120_000);
  const { dir, win, userData } = await openClean();
  await setThreads(win, "2");
  await runAll(win, 1);
  await expect(node(win, "failing match")).toHaveAttribute("data-status", "failed");

  await setThreads(win, "1");
  await win.getByTestId("rerun-failed-button").click();
  await finished(win, 2);
  await expectRow(win, userData, dir, 0, 1);
  await expectRow(win, userData, dir, 1, 2);
});

test("The box overrides a karate-pom.json", async () => {
  test.setTimeout(120_000);
  const { dir, win, userData } = await openClean({ threads: 3 });
  await expect(box(win)).toBeEnabled();
  await expect(box(win)).toHaveValue("1");

  await runAll(win, 1);
  await expectRow(win, userData, dir, 0, 1);
});

test("The box is disabled while a run is going", async () => {
  test.setTimeout(120_000);
  const { win } = await openClean();
  await expect(box(win)).toBeEnabled();

  await win.getByTestId("run-all-button").click();
  await expect(box(win)).toBeDisabled();
  await finished(win, 1);
  await expect(box(win)).toBeEnabled();
});

test("The count is saved per project", async () => {
  test.setTimeout(120_000);
  const { app, dir, win, userData } = await openClean();
  await setThreads(win, "3");
  await expect.poll(() => saved(userData, dir)?.threads).toBe(3);

  // one app per user-data folder: close it before the second starts
  await app.close();
  const second = await launch(userData);
  await expect(nodeByKey(second.win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await expect(box(second.win)).toHaveValue("3");

  // one step with the arrow keys is a change too
  await box(second.win).press("ArrowUp");
  await expect(box(second.win)).toHaveValue("4");
  await expect.poll(() => saved(userData, dir)?.threads).toBe(4);

  await openProject(second.app, project("tags"));
  await expect(nodeByKey(second.win, "f:tests/tags.feature")).toBeVisible({ timeout: 15_000 });
  await expect(box(second.win)).toBeEnabled();
  await expect(box(second.win)).toHaveValue("1");
  await openProject(second.app, dir);
  await expect(nodeByKey(second.win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await expect(box(second.win)).toHaveValue("4");
});

test("A value out of range is refused", async () => {
  test.setTimeout(120_000);
  const { dir, win, userData } = await openClean();
  await setThreads(win, "2");
  await expect.poll(() => saved(userData, dir)?.threads).toBe(2);

  for (const [i, value] of ["0", "33", "2.5", ""].entries()) {
    await setThreads(win, value);
    // the box takes the typed value and goes back to 2 only when main re-sends the count it holds
    await expect(box(win)).toHaveValue("2");
    // the notice stays on screen: the box reset and the unchanged saved count prove each later refusal
    if (i === 0)
      await expect(notice(win)).toHaveText("Threads must be a whole number from 1 to 32");
    expect(saved(userData, dir)?.threads, value).toBe(2);
  }
});
