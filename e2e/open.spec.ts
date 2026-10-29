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
