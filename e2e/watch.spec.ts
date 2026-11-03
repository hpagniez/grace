import { writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, nodeByKey, openProject, project, settingsOf, test } from "./helpers";

const FAILING = "Feature: w1\n\nScenario: one\n  * match 1 == 2\n";

test("Re-run on save (docs/specs/2026-11-03-watch-mode.md)", async () => {
  test.setTimeout(120_000);
  const dir = project("tags");
  const { app, win, userData } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/tags.feature")).toBeVisible({ timeout: 15_000 });
  const watch = win.getByTestId("watch-button");
  const history = win.getByTestId("tab-history");
  await expect(watch).toHaveAttribute("aria-pressed", "false");

  await watch.click();
  await expect(watch).toHaveAttribute("aria-pressed", "true");
  expect((settingsOf(userData).projects as Record<string, { watch?: boolean }>)[dir].watch).toBe(
    true,
  );
  writeFileSync(path.join(dir, "tests", "w1.feature"), FAILING);
  await expect(nodeByKey(win, "f:tests/w1.feature")).toHaveAttribute("data-status", "failed", {
    timeout: 60_000,
  });
  await expect(history).toHaveText("History (1)", { timeout: 30_000 });

  await watch.click();
  await expect(watch).toHaveAttribute("aria-pressed", "false");
  writeFileSync(path.join(dir, "tests", "w1.feature"), FAILING.replace("1 == 2", "2 == 3"));
  await win.waitForTimeout(2_000);
  await expect(history).toHaveText("History (1)");
  await expect(win.getByTestId("run-all-button")).toBeEnabled();
});
