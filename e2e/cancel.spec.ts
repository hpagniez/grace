import {
  expect,
  launch,
  node,
  nodeByKey,
  openProject,
  ourJavaPids,
  project,
  test,
} from "./helpers";

async function startSlowRun() {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  const slow = nodeByKey(win, "f:tests/slow.feature");
  await expect(slow).toBeVisible({ timeout: 15_000 });
  await slow.getByTestId("run-node").click();
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
  expect(ourJavaPids().length).toBeGreaterThan(0); // the leak check below must see a live run
  return { app, win };
}

test("Cancel leaves nothing behind", async () => {
  test.setTimeout(90_000);
  const { win } = await startSlowRun();

  await win.getByTestId("cancel-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Cancelled", { timeout: 15_000 });
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "interrupted");
  await expect.poll(ourJavaPids, { timeout: 15_000 }).toEqual([]);
  await expect(win.getByTestId("report-button")).toBeDisabled();
});

test("Closing the app during a run", async () => {
  test.setTimeout(90_000);
  const { app } = await startSlowRun();

  await app.close();
  await expect.poll(ourJavaPids, { timeout: 15_000 }).toEqual([]);
});
