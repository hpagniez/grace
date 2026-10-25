import { expect, launch, nodeByKey, openProject, project, stubOpenPath, test } from "./helpers";

test("Open the HTML report", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  const outline = nodeByKey(win, "f:tests/outline.feature");
  await expect(outline).toBeVisible({ timeout: 15_000 });
  await outline.getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });

  const opened = await stubOpenPath(app);
  await win.getByTestId("report-button").click();
  await expect.poll(opened).toHaveLength(1);
  expect((await opened())[0]).toMatch(/karate-summary\.html$/);
});
