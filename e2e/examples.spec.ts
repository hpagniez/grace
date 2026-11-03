import { expect, launch, node, nodeByKey, openProject, project, test } from "./helpers";

const ROW = (n: number) => `s:tests/outline.feature#1.${n}`;

test("An outline's Examples as a table", async () => {
  test.setTimeout(90_000);
  const { app, win } = await launch();
  await openProject(app, project("clean"));
  const outline = node(win, "add <a> and <b>");
  await expect(outline).toBeVisible({ timeout: 15_000 });
  await outline.getByRole("button", { name: "add <a> and <b>", exact: true }).click();

  const details = win.getByTestId("details");
  const line = (n: number) => details.locator(`[data-testid=example][data-key="${ROW(n)}"]`);
  await expect(details.getByTestId("example-column")).toHaveText(["a", "b", "expected"]);
  await expect(details.getByTestId("example")).toHaveCount(3);
  const values = [
    ["1", "2", "3"],
    ["2", "3", "5"],
    ["5", "5", "10"],
  ];
  for (const [i, v] of values.entries()) {
    await expect(line(i + 1)).toBeVisible();
    await expect(line(i + 1).getByTestId("example-value")).toHaveText(v);
    await expect(line(i + 1)).toHaveAttribute("data-status", "notRun");
  }

  await outline.getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  for (const n of [1, 2, 3]) {
    await expect(line(n)).toHaveAttribute("data-status", "passed");
    await expect(line(n).getByTestId("example-time")).toHaveText(/^\d+ ms$/);
  }
  await details.screenshot({ path: test.info().outputPath("examples.png") });

  await line(2).getByTestId("example-value").first().click();
  await expect(details.getByRole("heading")).toHaveText("add 2 and 3");
  await expect(nodeByKey(win, ROW(2)).locator("..")).toHaveAttribute("aria-selected", "true");
});
