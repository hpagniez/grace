// Settings › Appearance (spec 2026-10-29-premium-ui R8). These tests are about
// `nativeTheme.themeSource`, which drives the page's media and the window's background, so no
// colour scheme is emulated for the page (an emulated one would hide the theme).
import type { ElectronApplication, Page } from "@playwright/test";
import { feedFile } from "./feed";
import {
  addProperty,
  expect,
  launch,
  launchWithSettings,
  nodeByKey,
  openProject,
  openProperties,
  project,
  settingsOf,
  test,
} from "./helpers";

/** Playwright emulates a light page unless told not to: here the page follows `nativeTheme`. */
const NATIVE = { colorScheme: null } as const;

/** `ground` in each theme, as the browser reports it. */
const GROUND_RGB = { light: "rgb(243, 239, 230)", dark: "rgb(21, 24, 33)" };
/** `raised`, the dialogs' fill. */
const RAISED_RGB = { light: "rgb(255, 255, 255)", dark: "rgb(27, 30, 38)" };

const windowBackground = (app: ElectronApplication) =>
  app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].getBackgroundColor());
const themeSource = (app: ElectronApplication) =>
  app.evaluate(({ nativeTheme }) => nativeTheme.themeSource);

/** Opens Settings › Appearance from the status bar. */
async function openAppearance(win: Page) {
  await win.getByTestId("settings-button").click();
  await expect(win.getByTestId("settings")).toBeVisible();
  await win.getByTestId("settings-section-appearance").click();
  await expect(win.getByRole("heading", { name: "Appearance" })).toBeVisible();
}

test("Follow Windows is the default", async () => {
  const { app, win } = await launch(undefined, {}, NATIVE);
  await openAppearance(win);
  await expect(win.getByRole("radio", { name: "Follow Windows" })).toBeChecked();
  await expect(win.getByRole("radio", { name: "Light" })).not.toBeChecked();
  await expect(win.getByRole("radio", { name: "Dark" })).not.toBeChecked();
  expect(await themeSource(app)).toBe("system");
});

test("A saved Dark is in force at start, whatever Windows is", async () => {
  const { app, win } = await launchWithSettings({ appearance: { theme: "dark" } }, {}, NATIVE);
  expect(await themeSource(app)).toBe("dark");
  await expect(win.getByTestId("app")).toHaveCSS("background-color", GROUND_RGB.dark);
  // the window's own background is the dark ground too
  expect((await windowBackground(app)).toLowerCase()).toBe("#151821");
  await openAppearance(win);
  await expect(win.getByRole("radio", { name: "Dark" })).toBeChecked();
});

test("Choosing a theme in Settings applies at once, with a project open, and is still there after a restart", async () => {
  const { app, win, userData } = await launch(undefined, {}, NATIVE);
  await openProject(app, project("clean"));
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await win.evaluate(() => ((window as unknown as { marker: number }).marker = 1));

  await openAppearance(win);
  await win.getByRole("radio", { name: "Dark" }).check();
  await expect(win.getByTestId("app")).toHaveCSS("background-color", GROUND_RGB.dark);
  await expect.poll(async () => (await windowBackground(app)).toLowerCase()).toBe("#151821");
  expect(await themeSource(app)).toBe("dark");
  await expect.poll(() => settingsOf(userData).appearance).toEqual({ theme: "dark" });

  await win.getByRole("radio", { name: "Light" }).check();
  await expect(win.getByTestId("app")).toHaveCSS("background-color", GROUND_RGB.light);
  await expect.poll(async () => (await windowBackground(app)).toLowerCase()).toBe("#f3efe6");
  expect(await themeSource(app)).toBe("light");
  await expect.poll(() => settingsOf(userData).appearance).toEqual({ theme: "light" });

  // back to the project as it was, and no reload on the way
  await win.getByTestId("settings-back").click();
  await expect(win.getByTestId("settings")).toBeHidden();
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible();
  expect(await win.evaluate(() => (window as unknown as { marker?: number }).marker)).toBe(1);

  await app.close();
  const again = await launch(userData, {}, NATIVE);
  expect(await themeSource(again.app)).toBe("light");
  expect((await windowBackground(again.app)).toLowerCase()).toBe("#f3efe6");
  await expect(again.win.getByTestId("app")).toHaveCSS("background-color", GROUND_RGB.light);
  await openAppearance(again.win);
  await expect(again.win.getByRole("radio", { name: "Light" })).toBeChecked();
});

// Review Focus 1: the dialog is modal, so a person cannot reach Settings while it is up; the same
// change arrives from the Windows setting, or here from the page's api. The window, the dialog and
// the typed draft all follow, and nothing reloads.
test("A theme change with a dialog up switches the window and the dialog and loses no state", async () => {
  const { app, win } = await launchWithSettings({ appearance: { theme: "dark" } }, {}, NATIVE);
  await openProject(app, project("clean"));
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await openProperties(win);
  await addProperty(win, "", "draft.name", "draft-value");
  await win.evaluate(() => ((window as unknown as { marker: number }).marker = 1));
  const dialog = win.getByTestId("properties-dialog");
  await expect(dialog).toHaveCSS("background-color", RAISED_RGB.dark);

  await win.evaluate(() => window.api.setAppearance("light"));

  await expect(win.getByTestId("app")).toHaveCSS("background-color", GROUND_RGB.light);
  await expect(dialog).toHaveCSS("background-color", RAISED_RGB.light);
  await expect.poll(async () => (await windowBackground(app)).toLowerCase()).toBe("#f3efe6");
  await expect(dialog).toBeVisible();
  const row = win.getByTestId("property-row");
  await expect(row.getByTestId("prop-name")).toHaveValue("draft.name");
  await expect(row.getByTestId("prop-value")).toHaveValue("draft-value");
  expect(await win.evaluate(() => (window as unknown as { marker?: number }).marker)).toBe(1);

  await win.evaluate(() => window.api.setAppearance("dark"));
  await expect(dialog).toHaveCSS("background-color", RAISED_RGB.dark);
  await expect(row.getByTestId("prop-name")).toHaveValue("draft.name");
});

// Review Focus 4: one Escape closes one layer.
test("Escape closes a shown tooltip first, then the Settings page goes back", async () => {
  const { win } = await launch(undefined, {
    GRACE_UPDATE_FEED: feedFile([{ version: "1.0.0-rc.99", prerelease: true }]),
  });
  await expect(win.getByTestId("update-modal")).toBeVisible();
  await win.keyboard.press("Escape"); // the startup modal: its own layer
  await expect(win.getByTestId("update-modal")).toBeHidden();

  await win.getByTestId("settings-button").click();
  await expect(win.getByTestId("settings")).toBeVisible();
  // e2e runs a build that is not packaged: the button is disabled and says why
  await win.getByTestId("settings-update").hover({ force: true });
  await expect(win.getByRole("tooltip")).toHaveText("Updates install only in the packaged app");

  await win.keyboard.press("Escape");
  await expect(win.getByRole("tooltip")).toHaveCount(0);
  await expect(win.getByTestId("settings")).toBeVisible();

  await win.keyboard.press("Escape");
  await expect(win.getByTestId("settings")).toBeHidden();
});

test("Main refuses a theme it does not know and keeps the saved one", async () => {
  const { app, win, userData } = await launchWithSettings(
    { appearance: { theme: "dark" } },
    {},
    NATIVE,
  );
  await expect(win.getByTestId("home")).toBeVisible();
  await win.evaluate(async () => {
    const api = window.api as unknown as Record<string, (...args: unknown[]) => Promise<void>>;
    await api.setAppearance("sepia");
    await api.setAppearance(null);
    await api.setAppearance(3);
    await api.setAppearance({ theme: "light" });
    await api.setAppearance(["light"]);
  });
  await expect(win.getByTestId("notice")).toHaveText("Invalid request");
  expect(settingsOf(userData).appearance).toEqual({ theme: "dark" });
  expect(await themeSource(app)).toBe("dark");

  // a theme it knows is applied and saved
  await win.evaluate(() => window.api.setAppearance("system"));
  await expect.poll(() => settingsOf(userData).appearance).toEqual({ theme: "system" });
  expect(await themeSource(app)).toBe("system");
});
