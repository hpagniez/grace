import { existsSync, writeFileSync } from "node:fs";
import path from "node:path";
import pkg from "../package.json";
import { feedFile, feedServer } from "./feed";
import {
  expect,
  launch,
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
