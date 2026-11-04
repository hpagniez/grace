// Trends (docs/specs/2026-11-04-trends.md): the strip at the top of the History tab.
import type { Page } from "@playwright/test";
import {
  emptyProject,
  expect,
  launch,
  node,
  nodeByKey,
  openFeatures,
  openProject,
  project,
  seedRuns,
  test,
} from "./helpers";
import type { RunRecord } from "../src/shared/history";

const columns = (win: Page) => win.getByTestId("trend-run");

/**
 * The clean sample project with seeded runs, the History tab shown. Each seeded run stores `passing
 * hello` passed in 400 ms and `failing match` failed in 300 ms.
 */
async function openWithRuns(...seeds: [number, Partial<RunRecord>?][]) {
  const dir = project("clean");
  const userData = emptyProject();
  const ids = seeds.flatMap(([n, record]) => seedRuns(userData, dir, n, { record }));
  const { app, win } = await launch(userData);
  await openProject(app, dir);
  await openFeatures(win);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await win.getByTestId("tab-history").click();
  return { win, ids };
}

test("the strip follows the tree selection", async () => {
  const { win } = await openWithRuns([3]);
  await expect(win.getByTestId("trends-caption")).toHaveText("All tests");
  await expect(columns(win)).toHaveCount(3);
  await expect(win.getByTestId("trends-pass-label")).toHaveText("Pass rate 50%");
  await expect(win.getByTestId("trends-time-label")).toHaveText("Time per test 350 ms");
  await node(win, "passing hello").click();
  await expect(win.getByTestId("trends-caption")).toHaveText("passing hello");
  await expect(win.getByTestId("trends-pass-label")).toHaveText("Pass rate Passed");
  await expect(win.getByTestId("trends-time-label")).toHaveText("Time per test 400 ms");
  await expect(columns(win)).toHaveCount(3);
});

test("a column finds its run in the table", async () => {
  const { win, ids } = await openWithRuns([20]);
  const oldest = win.locator(`[data-testid="history-row"][data-id="${ids[0]}"]`);
  await expect(columns(win)).toHaveCount(20);
  await expect(oldest).not.toBeInViewport(); // below the fold at the panel's default height
  await columns(win).first().click();
  await expect(oldest).toBeInViewport();
});

test("a run on another environment has no column", async () => {
  const { win } = await openWithRuns([2, { env: "dev" }], [1]);
  await expect(win.getByTestId("history-row")).toHaveCount(3);
  await expect(columns(win)).toHaveCount(1);
});
