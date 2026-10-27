import type { Page } from "@playwright/test";
import { expect, launch, nodeByKey, openProject, project, test } from "./helpers";

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

  // Playwright compiles specs to CommonJS: only a dynamic import loads the ES module
  const { VERSIONS } = await import("../scripts/fetch-vendor.mjs");
  const banner = win.getByTestId("console-banner");
  await expect(banner).toContainText(`Karate ${VERSIONS.karate}`);
  await expect(banner).toContainText(`Java ${VERSIONS.java}`);
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

const backgrounds = async (colorScheme: "light" | "dark") => {
  const { app, win } = await launch(undefined, {}, { colorScheme });
  const bg = (id: string) =>
    win.getByTestId(id).evaluate((e) => getComputedStyle(e).backgroundColor);
  const found = { console: await bg("console"), app: await bg("app") };
  await app.close();
  return found;
};

test("the console is darker than the window in dark mode only", async () => {
  const dark = await backgrounds("dark");
  const light = await backgrounds("light");
  expect(dark.console).not.toBe(dark.app);
  expect(light.console).toBe(dark.app); // both zinc-900
});
