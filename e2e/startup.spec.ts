import type { ElectronApplication, Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import { GROUND } from "../src/shared/theme";
import { expect, launch, test } from "./helpers";

/** Once the toolbar shows, the startup screen is gone and the page holds the app alone. */
async function expectAppOnly(win: Page) {
  await expect(win.getByTestId("open-button")).toBeVisible();
  await expect(win.locator("#startup")).toHaveCount(0);
  await expect(win.locator("#root > *")).toHaveCount(1);
  await expect(win.locator("body > :not(script)")).toHaveCount(1);
}

const windowState = (app: ElectronApplication) =>
  app.evaluate(({ BrowserWindow, nativeTheme }) => {
    const w = BrowserWindow.getAllWindows()[0];
    return {
      visible: w.isVisible(),
      background: w.getBackgroundColor().toLowerCase(),
      dark: nativeTheme.shouldUseDarkColors,
    };
  });

test("the window opens painted in the app's theme", async () => {
  const { app, win } = await launch();
  await expect.poll(async () => (await windowState(app)).visible).toBe(true);
  const { background, dark } = await windowState(app);
  expect(background).toBe(dark ? GROUND.dark : GROUND.light);
  await expectAppOnly(win);
});

// `show: false` itself cannot be observed from a test; the by-hand flash check covers it.
test("the page starts with a static startup screen", () => {
  const html = readFileSync(".vite/renderer/main_window/index.html", "utf8");
  expect(html).toContain('<div id="startup" role="status" aria-label="Loading Grace">');
  const startup = html.slice(html.indexOf('<div id="startup"'), html.indexOf('<div id="root">'));
  expect(startup).toContain("<svg"); // the logo (assets/icon.svg, inlined)
  // the word on screen, after the logo and before the spinner: the aria-label holds it too
  expect(startup).toMatch(/<\/svg>\s*Grace\s*<div class="spinner">/);
  expect(html).toMatch(/prefers-color-scheme:\s*dark/);
  expect(html).toMatch(/prefers-reduced-motion:\s*reduce/);
  expect(html).toContain('<div id="root"></div>');
});

test("a reload shows the app once", async () => {
  const { win } = await launch();
  await expectAppOnly(win);
  await win.reload();
  await expectAppOnly(win);
});
