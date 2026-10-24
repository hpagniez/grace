import path from "node:path";
import { emptyProject, expect, launch, nodeByKey, openProject, project, test } from "./helpers";

test("Open a project and see its tests", async () => {
  const dir = project("clean");
  const { app, win, userData } = await launch();
  expect(await app.evaluate(({ app }) => app.getPath("userData"))).toBe(userData);
  await expect(win.getByTestId("tree-message")).toHaveText("Open a Karate project folder");

  await openProject(app, dir);
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 2000 });
  await expect(nodeByKey(win, "d:tests")).toContainText("tests");
  for (const f of ["basic", "outline", "slow"]) {
    await expect(
      win.locator(`li:has(> [data-key="d:tests"]) [data-key="f:tests/${f}.feature"]`),
    ).toBeVisible();
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
