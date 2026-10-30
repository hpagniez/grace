import type { Page } from "@playwright/test";
import vendor from "../scripts/vendor.json";
import { emptyProject, expect, launch, nodeByKey, openProject, project, test } from "./helpers";

const runAll = async (win: Page) => {
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
};

// basic.feature's "passing hello" prints this once per run
const printed = (win: Page) =>
  win.getByTestId("console").locator(":scope > div").filter({ hasText: "env is" });

test("open, run and run again", async () => {
  test.setTimeout(150_000);
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });

  const banner = win.getByTestId("console-banner");
  await expect(banner).toContainText(`Karate ${vendor.karate.version}`);
  await expect(banner).toContainText(`Java ${vendor.jre.version}`);
  await expect(win.getByTestId("banner-project")).toContainText(dir);
  await expect(win.getByTestId("banner-tests")).toHaveText(
    /^tests {4}\d+ scenarios? in \d+ features?$/,
  );
  await expect(win.getByTestId("console-state")).toContainText("Ready.");

  await runAll(win);
  const first = win.getByTestId("console").locator(":scope > *").first();
  await expect(first).toHaveAttribute("data-testid", "console-banner");
  await expect(win.getByTestId("console-state")).toHaveCount(0);
  await expect(printed(win)).toHaveCount(1);

  await runAll(win);
  await expect(banner).toHaveCount(1);
  await expect(first).toHaveAttribute("data-testid", "console-banner");
  await expect(printed(win)).toHaveCount(1); // the first run's output is gone
});

const consoleBackground = async (colorScheme: "light" | "dark") => {
  const { app, win } = await launch(undefined, {}, { colorScheme });
  await openProject(app, emptyProject()); // the home screen has no console
  const bg = await win.getByTestId("console").evaluate((e) => getComputedStyle(e).backgroundColor);
  await app.close();
  return bg;
};

test("the console is on the sunken colour in both themes", async () => {
  expect(await consoleBackground("light")).toBe("rgb(248, 246, 241)"); // #f8f6f1
  expect(await consoleBackground("dark")).toBe("rgb(11, 13, 17)"); // #0b0d11
});
