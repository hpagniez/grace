import { existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import pkg from "../package.json";
import { feedFile, feedServer } from "./feed";
import {
  expect,
  launch,
  launchWithSettings,
  nodeByKey,
  openProject,
  project,
  settingsOf,
  tempDir,
  test,
} from "./helpers";

const rc99 = [{ version: "1.0.0-rc.99", prerelease: true }];

/** A fresh userData folder holding the result of a switch to this version, as the script writes it. */
function updatedTo(version: string): { userData: string; result: string } {
  const userData = tempDir("kk-userdata-");
  const result = path.join(userData, "update-result.json");
  writeFileSync(
    result,
    JSON.stringify({
      from: "1.0.0-rc.2",
      to: version,
      rollback: false,
      ok: true,
      reverted: false,
      error: null,
      updates: null,
    }),
  );
  return { userData, result };
}

test("the status bar shows the version on every view (R3.1)", async () => {
  const { app, win } = await launch();
  await expect(win.getByTestId("home")).toBeVisible();
  await expect(win.getByTestId("status-bar")).toContainText(`Grace ${pkg.version}`);
  await expect(win.getByTestId("settings-button")).toHaveText("⚙ Settings");
  await openProject(app, project("clean"));
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("status-bar")).toContainText(`Grace ${pkg.version}`);
  await expect(win.getByTestId("update-marker")).toHaveCount(0);
});

test("out of date on the pre-release channel shows the marker (R3.2)", async () => {
  const { win } = await launch(undefined, { GRACE_UPDATE_FEED: feedFile(rc99) });
  await expect(win.getByTestId("update-marker")).toHaveText(/1\.0\.0-rc\.99 available/);
  await expect(win.getByTestId("update-link")).toHaveText("Update");
  // e2e runs a build that is not packaged (R13.1)
  await expect(win.getByTestId("update-link")).toBeDisabled();
  await expect(win.getByTestId("update-link")).toHaveAttribute(
    "title",
    "Updates install only in the packaged app",
  );
  await expect(
    win.getByRole("button", { name: `Grace ${pkg.version}, 1.0.0-rc.99 available` }),
  ).toBeVisible();
});

test("the startup check reads an http feed once, through main (R2.1)", async () => {
  const feed = await feedServer(rc99);
  const { win } = await launch(undefined, { GRACE_UPDATE_FEED: feed.url });
  await expect(win.getByTestId("update-marker")).toHaveText(/1\.0\.0-rc\.99 available/);
  expect(feed.requests()).toBe(1);
  await feed.close();
});

test("the last switch's result shows as a notice once the window is ready (R10.7)", async () => {
  const { userData, result } = updatedTo(pkg.version);
  const { win } = await launch(userData);
  await expect(win.getByTestId("notice")).toHaveText(`Updated to ${pkg.version}.`);
  expect(existsSync(result)).toBe(false);
});

test("the last switch's result outlasts the last project's reopening (R10.7)", async () => {
  const dir = project("clean");
  const { userData } = updatedTo(pkg.version);
  writeFileSync(
    path.join(userData, "settings.json"),
    JSON.stringify({ lastProject: path.resolve(dir) }),
  );
  const { win } = await launch(userData);
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(dir));
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("notice")).toHaveText(`Updated to ${pkg.version}.`);
});

test("main refuses update requests with invalid arguments", async () => {
  const { app, win, userData } = await launch(undefined, { GRACE_UPDATE_FEED: feedFile(rc99) });
  await expect(win.getByTestId("update-marker")).toBeVisible(); // the check saved its time
  const before = settingsOf(userData).updates;
  await app.evaluate(({ shell }) => {
    const opened: string[] = [];
    (globalThis as { opened?: string[] }).opened = opened;
    shell.openExternal = async (url: string) => void opened.push(url);
  });
  const opened = () => app.evaluate(() => (globalThis as { opened?: string[] }).opened);

  await win.evaluate(async () => {
    const api = window.api as unknown as Record<string, (...args: unknown[]) => Promise<void>>;
    await api.setUpdateSettings({ channel: "beta" });
    await api.setUpdateSettings({ autoUpdate: true, extra: 1 });
    await api.setUpdateSettings(["x"]);
    await api.setUpdateSettings(null);
    await api.skipVersion("2.0.0-beta");
    await api.openWhatsNew("1.0.0-rc.99/../../x");
    await api.installVersion("1.0.0-rc.99", "yes");
  });
  await expect(win.getByTestId("notice")).toHaveText("Invalid request");
  expect(settingsOf(userData).updates).toEqual(before);
  expect(await opened()).toEqual([]);

  // valid requests reach the Updater
  await win.evaluate(() => window.api.installVersion("1.0.0-rc.99", false));
  await expect(win.getByTestId("notice")).toHaveText("Updates install only in the packaged app");
  await win.evaluate(() => window.api.openWhatsNew("1.0.0-rc.99"));
  expect(await opened()).toEqual(["https://github.com/hpagniez/grace/releases/tag/v1.0.0-rc.99"]);
  await win.evaluate(() => window.api.setUpdateSettings({ channel: "stable" }));
  await expect(win.getByTestId("update-marker")).toHaveCount(0);
  expect(settingsOf(userData).updates).toMatchObject({ channel: "stable" });
});

const rc98 = [{ version: "1.0.0-rc.98", prerelease: true }];

/** The startup modal's view of what the person saved about updates. */
const updatesOf = (userData: string) =>
  settingsOf(userData).updates as { skipped?: string | null } | undefined;

test("skipping a version keeps the marker but not the modal (R4.3)", async () => {
  const env = { GRACE_UPDATE_FEED: feedFile(rc99) };
  const first = await launch(undefined, env);
  const modal = first.win.getByTestId("update-modal");
  await expect(modal).toBeVisible();
  await expect(modal).toContainText("Grace 1.0.0-rc.99 is available");
  await expect(modal).toContainText(`You have ${pkg.version}. Download: 219 MB.`);
  await expect(first.win.getByRole("button", { name: "Update and restart" })).toBeFocused();
  await first.win.getByRole("button", { name: "Skip this version" }).click();
  await expect(modal).toBeHidden();
  await expect.poll(() => updatesOf(first.userData)?.skipped).toBe("1.0.0-rc.99");
  await first.app.close();

  const { win } = await launch(first.userData, env);
  await expect(win.getByTestId("update-marker")).toHaveText(/1\.0\.0-rc\.99 available/);
  await expect(win.getByTestId("update-modal")).toBeHidden();
});

test("a newer version after a skip shows the modal again", async () => {
  const first = await launch(undefined, { GRACE_UPDATE_FEED: feedFile(rc98) });
  await first.win.getByRole("button", { name: "Skip this version" }).click();
  await expect.poll(() => updatesOf(first.userData)?.skipped).toBe("1.0.0-rc.98");
  await first.app.close();

  const { win } = await launch(first.userData, { GRACE_UPDATE_FEED: feedFile(rc99) });
  await expect(win.getByTestId("update-modal")).toContainText("Grace 1.0.0-rc.99 is available");
});

test("closing the modal without a choice asks again (R4.5)", async () => {
  const env = { GRACE_UPDATE_FEED: feedFile(rc99) };
  const first = await launch(undefined, env);
  const modal = first.win.getByTestId("update-modal");
  await expect(modal).toBeVisible();
  await first.win.getByRole("checkbox").check(); // ticked, but nothing is saved
  await first.win.keyboard.press("Escape");
  await expect(modal).toBeHidden();
  // the marker stays: the version is out, only the question is closed
  await expect(first.win.getByTestId("update-marker")).toBeVisible();
  const saved = settingsOf(first.userData).updates as Record<string, unknown>;
  expect([saved.skipped ?? null, saved.autoUpdate ?? false]).toEqual([null, false]);
  await first.app.close();

  const { win } = await launch(first.userData, env);
  await expect(win.getByTestId("update-modal")).toContainText("Grace 1.0.0-rc.99 is available");
  await expect(win.getByRole("checkbox")).not.toBeChecked();
});

test("a failed check is quiet (R2.5)", async () => {
  const missing = path.join(tempDir("kk-feed-"), "missing.json");
  const { app, win } = await launch(undefined, { GRACE_UPDATE_FEED: missing });
  await app.evaluate(({ dialog }) => {
    const shown: string[] = [];
    (globalThis as { shown?: string[] }).shown = shown;
    dialog.showErrorBox = (title: string) => void shown.push(title);
    dialog.showMessageBox = (async () => (shown.push("message"), { response: 0 })) as never;
  });
  await expect(win.getByTestId("status-bar")).toContainText(`Grace ${pkg.version}`);
  await win.evaluate(() => window.api.checkUpdates()); // waits for the startup check (R2.4)
  await expect(win.getByTestId("update-modal")).toBeHidden();
  await expect(win.getByTestId("update-progress")).toBeHidden();
  await expect(win.getByTestId("update-marker")).toHaveCount(0);
  await expect(win.getByTestId("notice")).toHaveCount(0);
  expect(await app.evaluate(() => (globalThis as { shown?: string[] }).shown)).toEqual([]);
});

test("default channel follows the running version (R1.3)", async () => {
  const { win } = await launch();
  await win.getByTestId("settings-button").click();
  // the running version is a release candidate
  await expect(win.getByRole("radio", { name: "Stable and pre-releases" })).toBeChecked();
  await expect(win.getByRole("radio", { name: "Stable releases only" })).not.toBeChecked();
});

/** The settings of a person who skipped rc.99, so the startup modal does not cover the page. */
const skippedRc99 = { updates: { skipped: "1.0.0-rc.99" } };

test("version list and dev-build install button (R6.3, R6.4)", async () => {
  const { win } = await launchWithSettings(skippedRc99, { GRACE_UPDATE_FEED: feedFile(rc99) });
  await expect(win.getByTestId("update-marker")).toBeVisible();
  await win.getByTestId("settings-button").click();
  await expect(win.getByTestId("settings-update")).toHaveText("Update to 1.0.0-rc.99");
  const rows = win.getByTestId("version-row");
  await expect(rows).toHaveCount(2); // the release and the running version, which no release lists
  await expect(rows.first()).toContainText("1.0.0-rc.99");
  await expect(rows.first()).toContainText("2026-10-29");
  await expect(rows.first()).toContainText("pre-release");
  // e2e runs a build that is not packaged (R13.1)
  const install = rows.first().getByRole("button", { name: "Install" });
  await expect(install).toBeDisabled();
  await expect(install).toHaveAttribute("title", "Updates install only in the packaged app");
  await expect(win.getByTestId("settings-update")).toBeDisabled();
  await expect(rows.last()).toContainText(pkg.version);
  await expect(rows.last()).toContainText("installed");
  await expect(rows.last().getByRole("button")).toHaveCount(0);
});

test("a channel change follows from the last check without a new request (R6.2)", async () => {
  const feed = await feedServer(rc99);
  const { win, userData } = await launchWithSettings(skippedRc99, { GRACE_UPDATE_FEED: feed.url });
  await expect(win.getByTestId("update-marker")).toBeVisible();
  await win.getByTestId("settings-button").click();
  await win.getByRole("radio", { name: "Stable releases only" }).check();
  await expect(win.getByTestId("update-marker")).toHaveCount(0);
  await expect(win.getByTestId("settings-marker")).toHaveCount(0);
  await expect(win.getByTestId("version-row")).toHaveCount(1);
  await expect
    .poll(() => (settingsOf(userData).updates as { channel?: string }).channel)
    .toBe("stable");
  expect(feed.requests()).toBe(1);
  await feed.close();
});

test("Back returns to the open project unchanged (R6.1)", async () => {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(nodeByKey(win, "d:tests")).toBeVisible({ timeout: 15_000 });
  await win.getByTestId("settings-button").click();
  await expect(win.getByTestId("settings")).toBeVisible();
  await expect(nodeByKey(win, "d:tests")).toHaveCount(0);
  await expect(win.getByTestId("status-bar")).toContainText(`Grace ${pkg.version}`);
  await win.getByRole("button", { name: "← Back" }).click();
  await expect(win.getByTestId("settings")).toHaveCount(0);
  await expect(nodeByKey(win, "d:tests")).toBeVisible();
  await expect(win.getByTestId("project-path")).toHaveText(path.resolve(dir));
  // Escape goes back too
  await win.getByTestId("settings-button").click();
  await win.keyboard.press("Escape");
  await expect(win.getByTestId("settings")).toHaveCount(0);
  await expect(nodeByKey(win, "d:tests")).toBeVisible();
});

test("a failed check shows its reason in the settings (R2.5)", async () => {
  const missing = path.join(tempDir("kk-feed-"), "missing.json");
  const { win } = await launch(undefined, { GRACE_UPDATE_FEED: missing });
  await win.getByTestId("settings-button").click();
  await win.getByRole("button", { name: "Check for updates" }).click();
  await expect(win.getByTestId("settings-error")).toContainText("Couldn't check for updates: ");
  await expect(win.getByTestId("update-modal")).toBeHidden();
});

test("no network when the startup check is off (R2.3)", async () => {
  const feed = await feedServer(rc99);
  const { win } = await launchWithSettings(
    { updates: { checkAtStartup: false } },
    { GRACE_UPDATE_FEED: feed.url },
  );
  await win.getByTestId("settings-button").click();
  await expect(win.getByRole("checkbox", { name: "Check for updates" })).not.toBeChecked();
  await expect(
    win.getByRole("checkbox", { name: "Update automatically, without asking" }),
  ).toBeDisabled();
  await expect(win.getByText("Check for updates to see other versions")).toBeVisible();
  await win.waitForTimeout(3000);
  expect(feed.requests()).toBe(0);
  await win.getByRole("button", { name: "Check for updates" }).click();
  await expect(win.getByTestId("settings-marker")).toHaveText("1.0.0-rc.99 available");
  expect(feed.requests()).toBe(1);
  await expect(win.getByTestId("settings-last-checked")).toContainText("Last checked");
  await feed.close();
});
