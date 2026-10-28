import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, node, nodeByKey, openProject, project, test } from "./helpers";

async function openClean() {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  return { dir, app, win };
}

test("Run all with live progress", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();

  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("progress-stripe")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 9", { timeout: 15_000 });
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 45_000,
  });
  await expect(win.getByTestId("run-outcome")).toHaveText("Running…");
  await expect(
    win.locator("[data-testid=tree-node][data-kind=scenario][data-status=passed]").first(),
  ).toBeVisible();
  await expect(
    win.locator("[data-testid=tree-node][data-kind=scenario][data-status=failed]").first(),
  ).toBeVisible();
  await expect(win.getByTestId("run-outcome")).toHaveText("Running…");

  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("9 / 9");
  await expect(win.getByTestId("progress-stripe")).toHaveCount(0);
  await expect(win.getByTestId("progress-bar")).toHaveAttribute("aria-valuenow", "9");
  await expect(win.getByTestId("progress-passed")).toHaveText("8");
  await expect(win.getByTestId("progress-failed")).toHaveText("1");
  await expect(nodeByKey(win, "f:tests/basic.feature")).toHaveAttribute("data-status", "failed");
});

test("Run one outline row", async () => {
  const { win } = await openClean();
  const row = (n: number) => nodeByKey(win, `s:tests/outline.feature#1.${n}`);

  await row(1).getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await expect(row(1)).toHaveAttribute("data-status", "passed");
  await expect(row(2)).toHaveAttribute("data-status", "notRun");

  await row(2).getByTestId("run-node").click();
  await expect(row(2)).toHaveAttribute("data-status", "passed", { timeout: 30_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await expect(row(2)).toContainText("add 2 and 3");
  await expect(win.getByTestId("progress-text")).toHaveText("1 / 1");
  await expect(row(1)).toHaveAttribute("data-status", "passed");
  await expect(row(3)).toHaveAttribute("data-status", "notRun");
});

test("A feature edited outside the app", async () => {
  const { dir, win } = await openClean();
  const file = path.join(dir, "tests", "basic.feature");
  const text = readFileSync(file, "utf8").replace(
    /(\* url baseUrl\r?\n)/,
    "$1\nScenario: inserted on top\n  * def y = 2\n",
  );
  writeFileSync(file, text);

  await node(win, "failing match").getByTestId("run-node").click();
  await expect(node(win, "inserted on top")).toBeVisible({ timeout: 30_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await expect(win.getByTestId("progress-text")).toHaveText("1 / 1");
  await expect(node(win, "failing match")).toHaveAttribute("data-status", "failed");
  await expect(node(win, "inserted on top")).toHaveAttribute("data-status", "notRun");
  await expect(node(win, "passing hello")).toHaveAttribute("data-status", "notRun");
});

/** Relative path → file hash (or "dir"), skipping anything under a karate-temp folder. */
function snapshot(root: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const rel of readdirSync(root, { recursive: true }) as string[]) {
    if (rel.split(path.sep).includes("karate-temp")) continue;
    const abs = path.join(root, rel);
    out[rel] = statSync(abs).isDirectory()
      ? "dir"
      : createHash("sha256").update(readFileSync(abs)).digest("hex");
  }
  return out;
}

test("The project is not modified", async () => {
  test.setTimeout(90_000);
  const dir = project("clean");
  const before = snapshot(dir);
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });

  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
  expect(snapshot(dir)).toEqual(before);
});
