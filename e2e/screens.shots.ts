// Takes the screenshots of docs/checks/2026-10-29-premium-ui. Bash: `SHOTS_DIR=<dir> npm run screens`.
// PowerShell: `$env:SHOTS_DIR="docs/checks/2026-10-29-premium-ui/before"; npm run screens`.
// Never run it with the unit tests: the Java leak checks of the e2e helpers see every java.exe.
import type { ElectronApplication, Page } from "@playwright/test";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import pkg from "../package.json";
import type { Release, UpdateView } from "../src/shared/updates";
import { feedFile } from "./feed";
import {
  emptyProject,
  expect,
  launch,
  node,
  nodeByKey,
  openProject,
  project,
  test,
} from "./helpers";

const SHOTS_DIR = path.resolve(process.env.SHOTS_DIR ?? "docs/checks/2026-10-29-premium-ui/after");
const WIDTH = 1280;
const HEIGHT = 800;

// The sample projects and app data folders go to the temp folder, and their paths show in the shots (toolbar,
// console, notice): keep them in the git-ignored out/, so the public record never holds the local user name.
const tmp = path.resolve("out", "shots-tmp");
mkdirSync(tmp, { recursive: true });
process.env.TEMP = process.env.TMP = tmp;

/** Starts the app with `userData`'s window at 1280 × 800, under this colour scheme, `env` added. */
async function start(theme: "light" | "dark", userData?: string, env: Record<string, string> = {}) {
  const app = await launch(userData, env, { colorScheme: theme });
  await app.app.evaluate(
    ({ BrowserWindow }, [w, h]) => BrowserWindow.getAllWindows()[0].setContentSize(w, h),
    [WIDTH, HEIGHT],
  );
  return app;
}

/** A release as main hands it to the renderer. */
const release = (version: string, publishedAt: string): Release => ({
  version,
  prerelease: true,
  publishedAt,
  url: `https://github.com/hpagniez/grace/releases/download/v${version}/Grace-win32-x64-${version}.zip`,
  size: 218_557_130,
  sha256: "0".repeat(64),
});

/** Sends `event` from main to the window, as main would. */
const send = (app: ElectronApplication, event: unknown) =>
  app.evaluate(
    ({ BrowserWindow }, e) => BrowserWindow.getAllWindows()[0].webContents.send("app:event", [e]),
    event,
  );

/** Opens a sample project and waits for the node `key` of its tree. */
async function open(app: ElectronApplication, win: Page, dir: string, key: string) {
  await openProject(app, dir);
  await expect(nodeByKey(win, key)).toBeVisible({ timeout: 15_000 });
}

for (const theme of ["light", "dark"] as const) {
  test(`screens in the ${theme} theme`, async () => {
    mkdirSync(SHOTS_DIR, { recursive: true });
    const shoot = async (win: Page, name: string) => {
      // top centre, 2 px down: above the toolbar's buttons and inputs and in no scrollbar, on every screen
      await win.mouse.move(WIDTH / 2, 2);
      await win.screenshot({
        path: path.join(SHOTS_DIR, `${theme}-${name}.png`),
        animations: "disabled",
        caret: "hide",
        scale: "css", // 1280 × 800 whatever the display scale
      });
    };

    await test.step("home, first run", async () => {
      const { win } = await start(theme);
      await expect(win.getByTestId("home")).toBeVisible();
      await shoot(win, "home-first-run");
    });

    await test.step("home with recent folders, then a notice", async () => {
      const base = emptyProject();
      const folders = ["payments-api", "storefront-tests", "legacy-suite"].map((n) =>
        path.join(base, n),
      );
      for (const dir of folders.slice(0, 2)) mkdirSync(dir);
      // listed but never created: the row is disabled; the second folder is deleted after the list shows
      const userData = emptyProject();
      writeFileSync(
        path.join(userData, "settings.json"),
        JSON.stringify({ recentProjects: [folders[1], folders[0], folders[2]] }),
      );
      const { win } = await start(theme, userData);
      await expect(win.getByTestId("recent-row")).toHaveCount(3);
      await expect(win.getByTestId("recent-missing")).toHaveCount(1);
      await shoot(win, "home-recent");

      rmSync(folders[1], { recursive: true, force: true });
      await win.getByTestId("recent-row").first().click();
      await expect(win.getByTestId("notice")).toContainText("Folder not found");
      await shoot(win, "notice");
    });

    await test.step("the update modal, settings and the switch and progress dialogs", async () => {
      const { app, win } = await start(theme, undefined, {
        GRACE_UPDATE_FEED: feedFile([{ version: "1.0.0-rc.99", prerelease: true }]),
      });
      const modal = win.getByTestId("update-modal");
      await expect(modal).toBeVisible();
      await shoot(win, "update-dialog");
      await win.keyboard.press("Escape"); // closes it and saves nothing
      await expect(modal).toBeHidden();

      // A packaged app with a saved copy of an older version, which a test build cannot be: main's view of it
      const view: UpdateView = {
        running: pkg.version,
        packaged: true,
        settings: {
          channel: null,
          checkAtStartup: true,
          autoUpdate: false,
          skipped: null,
          failed: null,
          lastChecked: new Date().toISOString(),
        },
        releases: [
          release("1.0.0-rc.99", "2026-10-29T10:00:00Z"),
          release("1.0.0-rc.3", "2026-10-28T10:00:00Z"),
        ],
        checking: false,
        error: null,
        backups: [{ version: "1.0.0-rc.3", savedAt: "2026-10-28T16:00:00Z" }],
      };
      await send(app, { type: "updates", view });
      await win.getByTestId("settings-button").click();
      await expect(win.getByTestId("version-row")).toHaveCount(3);
      await shoot(win, "settings");

      await win.getByRole("button", { name: "Roll back" }).click();
      await expect(win.getByTestId("switch-dialog")).toBeVisible();
      await shoot(win, "switch-dialog");
      await win.getByTestId("switch-cancel").click();
      await expect(win.getByTestId("switch-dialog")).toBeHidden();

      await send(app, {
        type: "updateProgress",
        progress: {
          version: "1.0.0-rc.99",
          rollback: false,
          step: "download",
          received: 82_000_000,
          total: 218_557_130,
        },
      });
      await expect(win.getByTestId("update-progress")).toBeVisible();
      await shoot(win, "update-progress");
    });

    await test.step("project idle", async () => {
      const { app, win } = await start(theme);
      await open(app, win, project("http"), "f:tests/http.feature");
      await shoot(win, "project-idle");
    });

    await test.step("filter bar on", async () => {
      const { app, win } = await start(theme);
      await open(app, win, project("tags"), "o:tests/tags.feature#2");
      for (const [tag, state] of [
        ["fast", "include"],
        ["slow", "include"],
        ["slow", "exclude"],
      ]) {
        await win.locator(`[data-testid=tag-chip][data-tag="${tag}"]`).click();
        await expect(win.locator(`[data-testid=tag-chip][data-tag="${tag}"]`)).toHaveAttribute(
          "data-state",
          state,
        );
      }
      await shoot(win, "filter-on");
    });

    await test.step("a run, its tests and its panels", async () => {
      const { app, win } = await start(theme);
      await open(app, win, project("clean"), "f:tests/basic.feature");

      await win.getByTestId("run-all-button").click();
      await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
        timeout: 45_000,
      });
      await expect(win.getByTestId("run-outcome")).toHaveText("Running…");
      await shoot(win, "running");
      await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });

      await node(win, "failing match")
        .getByRole("button", { name: "failing match", exact: true })
        .click();
      await expect(win.getByTestId("details-error")).toBeVisible();
      await shoot(win, "finished-failed");

      await nodeByKey(win, "d:tests").getByRole("button", { name: "tests", exact: true }).click();
      await expect(win.getByTestId("details-counts")).toBeVisible();
      await shoot(win, "group-selected");

      await win.getByTestId("tab-console").click();
      await expect(win.getByTestId("console")).toBeVisible();
      // the log sticks to its end, which is what the group shot above already shows: show its start
      await win.getByTestId("console").evaluate((el) => (el.scrollTop = 0));
      await shoot(win, "console");

      await win.getByTestId("tab-http").click();
      await win.getByTestId("http-call").first().click();
      await expect(win.getByTestId("call-details")).toBeVisible();
      await shoot(win, "http-call");

      await win.getByTestId("tab-history").click();
      await expect(win.getByTestId("history-row")).toHaveCount(1);
      await shoot(win, "history");

      await win.getByTestId("properties-button").click();
      await expect(win.getByTestId("properties-dialog")).toBeVisible();
      await shoot(win, "properties-dialog");
      await win.getByTestId("cancel-properties").click();
      await expect(win.getByTestId("properties-dialog")).toBeHidden();

      await win.getByTestId("clear-history").click();
      await expect(win.getByTestId("clear-history-dialog")).toBeVisible();
      await shoot(win, "clear-history-dialog");
    });
  });
}
