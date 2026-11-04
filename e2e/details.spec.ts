import {
  expect,
  launch,
  node,
  nodeByKey,
  openFeatures,
  openProject,
  project,
  test,
} from "./helpers";

test("Failure details", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  const feature = nodeByKey(win, "f:tests/basic.feature");
  await expect(feature).toBeVisible({ timeout: 15_000 });
  await openFeatures(win);
  // Before a run, the steps and their comments come from the file (step comments R1.2)
  const comment = "# expected greeting is wrong on purpose";
  await node(win, "failing match")
    .getByRole("button", { name: "failing match", exact: true })
    .click();
  await expect(
    win
      .getByTestId("step")
      .filter({ hasText: "response.message == 'goodbye'" })
      .getByTestId("step-comments"),
  ).toHaveText(comment);
  await feature.getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await expect(node(win, "failing match")).toHaveAttribute("data-status", "failed");

  await node(win, "failing match")
    .getByRole("button", { name: "failing match", exact: true })
    .click();
  await expect(win.getByTestId("details-location")).toHaveText("tests/basic.feature:15");
  await expect(win.getByTestId("details-error")).toContainText("basic.feature:20");
  await expect(win.getByTestId("details-error")).toContainText("match");
  const steps = win.getByTestId("step");
  await expect(steps).toHaveCount(5); // background `url` + 4 scenario steps
  await expect(win.locator("[data-testid=step][data-status=failed]")).toHaveCount(1);
  // Karate's comments on the run's failed step; its error already has the closest one (R2.4)
  await expect(
    win.locator("[data-testid=step][data-status=failed]").getByTestId("step-comments"),
  ).toHaveText(comment);
  await expect(win.getByTestId("details-error")).toContainText(
    /basic\.feature:20\s+# expected greeting is wrong on purpose\s+And match/,
  );
  await expect(win.getByTestId("step-comments")).toHaveCount(1);

  await steps.filter({ hasText: "method get" }).getByTestId("step-toggle").click();
  const log = win.getByTestId("step-log");
  await expect(log).toContainText("GET");
  await expect(log).toContainText("200");
});
