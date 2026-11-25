// Takes the screenshots of docs/checks/2026-11-25-gpui-p3-exit (Electron's side, the twins of
// crates/grace-e2e/tests/screens.rs). Bash: `SHOTS_DIR=<dir> npm run screens:p3`.
// PowerShell: `$env:SHOTS_DIR="docs/checks/2026-11-25-gpui-p3-exit/electron"; npm run screens:p3`.
// Never run it with the unit tests: the Java leak checks of the e2e helpers see every java.exe.
//
// Not taken: GPUI's `tree-row-menu` (the tree's right-click menu is a native menu, which a page screenshot cannot take),
// and its narrow-window shots of History (`history-narrow`, `history-narrow-tooltip`: a GPUI-only layout).
// The bottom panel's height is the app's `grace.panelHeight` in localStorage, set before a project opens (192 px
// without it). Details is above the panel here, not in it: the failure shots take the least panel and a taller window.
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { createRunDir, writeRunRecord } from "../src/main/history";
import type { RunRecord } from "../src/shared/history";
import { NO_FILTER } from "../src/shared/tags";
import {
  emptyProject,
  expect,
  flakyProject,
  launch,
  node,
  nodeByKey,
  openFeatures,
  openProject,
  project,
  projectDataDir,
  tempDir,
  test,
} from "./helpers";

const SHOTS_DIR = path.resolve(
  process.env.SHOTS_DIR ?? "docs/checks/2026-11-25-gpui-p3-exit/electron",
);
const WIDTH = 1280;
const HEIGHT = 800;

// The sample projects and app data folders go to the temp folder, and their paths show in the shots (toolbar,
// console, notice): keep them in the git-ignored out/, so the public record never holds the local user name.
const tmp = path.resolve("out", "shots-tmp");
mkdirSync(tmp, { recursive: true });
process.env.TEMP = process.env.TMP = tmp;

/** Starts the app at 1280 × `height` under this colour scheme; `panel` is the bottom panel's height in px. */
async function start(
  theme: "light" | "dark",
  opts: { userData?: string; panel?: number; height?: number } = {},
) {
  const started = await launch(opts.userData, {}, { colorScheme: theme });
  await started.app.evaluate(
    ({ BrowserWindow }, [w, h]) => BrowserWindow.getAllWindows()[0].setContentSize(w, h),
    [WIDTH, opts.height ?? HEIGHT],
  );
  if (opts.panel) {
    await started.win.evaluate(
      (h) => localStorage.setItem("grace.panelHeight", String(h)),
      opts.panel,
    );
    await started.win.reload(); // the height is read at load
    await expect(started.win.getByTestId("home")).toBeVisible();
  }
  return started;
}

/** Opens a sample project and waits for the node `key` of its tree. */
async function open(
  app: Awaited<ReturnType<typeof start>>["app"],
  win: Page,
  dir: string,
  key: string,
) {
  await openProject(app, dir);
  await openFeatures(win);
  await expect(nodeByKey(win, key)).toBeVisible({ timeout: 15_000 });
}

const runAll = async (win: Page) => {
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
};

/** Selects the test (or group) named `name` in the tree, which shows it in Details. */
const select = (win: Page, name: string) =>
  node(win, name).getByRole("button", { name, exact: true }).click();

/** The two tests of the clean sample project, as seedRuns of helpers.ts stores them. */
const CLEAN_TESTS: RunRecord["tests"] = [
  {
    kind: "scenario",
    file: "tests/basic.feature",
    refId: "[1:7]",
    name: "passing hello",
    line: 7,
    key: "s:tests/basic.feature#1",
    status: "passed",
    durationMillis: 400,
  },
  {
    kind: "scenario",
    file: "tests/basic.feature",
    refId: "[1:15]",
    name: "failing match",
    line: 15,
    key: "s:tests/basic.feature#2",
    status: "failed",
    durationMillis: 300,
  },
];

/** The two tests of the flaky sample project; `coin` is passed or failed. */
const coinTests = (coin: "passed" | "failed"): RunRecord["tests"] => [
  {
    kind: "scenario",
    file: "tests/coin.feature",
    refId: "[1:3]",
    name: "coin",
    line: 3,
    key: "s:tests/coin.feature#1",
    status: coin,
  },
  {
    kind: "scenario",
    file: "tests/coin.feature",
    refId: "[2:7]",
    name: "steady",
    line: 7,
    key: "s:tests/coin.feature#2",
    status: "passed",
  },
];

/**
 * Writes one run of the project at `root`, `ago` minutes in the past, with a report, into its history before the
 * app opens it: "Run all" on the clean sample project that failed "failing match" unless `record` says otherwise.
 * (helpers.ts's seedRuns spaces its runs one minute apart and cannot place them.)
 */
function seed(userData: string, root: string, ago: number, record: Partial<RunRecord> = {}) {
  const startedAt = Date.now() - ago * 60_000 - 5_000;
  const { dir } = createRunDir(projectDataDir(userData, root), new Date(startedAt));
  writeRunRecord(dir, {
    version: 1,
    startedAt,
    durationMillis: 5_000,
    env: null,
    threads: 1,
    fingerprint: null,
    filter: { ...NO_FILTER },
    selection: { kind: "folder", file: "", name: path.basename(root), line: null, key: "d:" },
    failedOf: null,
    outcome: "finished",
    exitCode: 1,
    tests: CLEAN_TESTS,
    ...record,
  });
  writeFileSync(path.join(dir, "karate-summary.html"), "<html></html>");
}

/** A project whose karate-config.js fails, with scenarios one and two in a.feature, three and four in suite/b.feature. */
function brokenConfigProject(): string {
  const dir = tempDir("kk-config-");
  const write = (file: string, text: string) => writeFileSync(path.join(dir, file), text);
  write("karate-config.js", "function fn() {\n  karate.fail('no vpn');\n  return {};\n}\n");
  const scenario = (name: string) => `Scenario: ${name}\n* print 'x'\n`;
  write("a.feature", `Feature: a\n\n${scenario("one")}\n${scenario("two")}`);
  mkdirSync(path.join(dir, "suite"));
  write("suite/b.feature", `Feature: b\n\n${scenario("three")}\n${scenario("four")}`);
  return dir;
}

for (const theme of ["light", "dark"] as const) {
  test(`P3 screens in the ${theme} theme`, async () => {
    test.setTimeout(600_000);
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

    await test.step("the HTTP tab: after a run, narrowed, what went wrong, a call", async () => {
      const { app, win } = await start(theme, { panel: 400 });
      await open(app, win, project("http"), "f:tests/http.feature");
      await runAll(win);
      await win.getByTestId("tab-http").click();
      await expect(win.getByTestId("tab-http")).toHaveAttribute("aria-selected", "true");
      await expect(win.getByTestId("tab-http")).toHaveText("HTTP (13)");
      await shoot(win, "http-after-a-run");

      await select(win, "retry");
      await expect(win.getByTestId("tab-http")).toHaveText("HTTP (3)");
      await shoot(win, "http-narrowed");

      await nodeByKey(win, "d:tests").getByRole("button", { name: "tests", exact: true }).click();
      for (const group of ["4xx", "5xx", "none"])
        await win.locator(`[data-testid=http-group][data-group="${group}"]`).click();
      await expect(win.getByTestId("tab-http")).toHaveText("HTTP (6)");
      await shoot(win, "http-went-wrong");

      await win.locator("[data-testid=http-call][data-status='500']").click();
      await expect(win.getByTestId("call-details")).toBeVisible();
      await shoot(win, "http-call-selected");
      await app.close();
    });

    await test.step("a failed test's Details, with the place of its error", async () => {
      const { app, win } = await start(theme);
      await open(app, win, project("clean"), "f:tests/basic.feature");
      await runAll(win);
      await select(win, "failing match");
      await expect(win.getByTestId("error-place")).toBeVisible();
      await shoot(win, "details-open-in-vs-code");
      await app.close();
    });

    await test.step("History, then its Clear dialog", async () => {
      const dir = project("clean");
      const userData = emptyProject();
      const scenario = {
        kind: "scenario",
        file: "tests/basic.feature",
        name: "failing match",
        line: 15,
        key: "s:tests/basic.feature#2",
      } as const;
      seed(userData, dir, 1500, { exitCode: 0, durationMillis: 83_000 });
      seed(userData, dir, 40, { selection: scenario, durationMillis: 400 });
      seed(userData, dir, 30, {
        selection: null,
        failedOf: 1_700_000_000_000,
        durationMillis: 1500,
      });
      seed(userData, dir, 6, {
        env: "qa",
        outcome: "cancelled",
        exitCode: null,
        durationMillis: 18_000,
        filter: { include: ["smoke"], exclude: ["slow"], hideNonMatching: false },
      });
      seed(userData, dir, 0, {});
      const { app, win } = await start(theme, { userData, panel: 340 });
      await open(app, win, dir, "f:tests/basic.feature");
      await win.getByTestId("tab-history").click();
      await expect(win.getByTestId("history-row")).toHaveCount(5);
      await expect(win.getByTestId("trends")).toBeVisible();
      await shoot(win, "history-tab");

      await win.getByTestId("clear-history").click();
      await expect(win.getByTestId("clear-history-dialog")).toBeVisible();
      await shoot(win, "history-clear-dialog");
      await app.close();
    });

    await test.step("a flaky test's proof", async () => {
      const { dir } = flakyProject();
      const userData = emptyProject();
      // the older run failed `coin` on 2 threads, the newer passed it on 1, the same project files
      const fingerprint = "f".repeat(64);
      seed(userData, dir, 1, { threads: 2, fingerprint, tests: coinTests("failed") });
      seed(userData, dir, 0, { threads: 1, fingerprint, exitCode: 0, tests: coinTests("passed") });
      const { app, win } = await start(theme, { userData });
      await open(app, win, dir, "f:tests/coin.feature");
      await expect(win.getByTestId("tab-history")).toHaveText("History (2)");
      await select(win, "coin");
      await expect(win.getByTestId("flaky-proof")).toBeVisible();
      await shoot(win, "flaky");
      await app.close();
    });

    await test.step("an outline's Examples table", async () => {
      const { app, win } = await start(theme);
      await open(app, win, project("clean"), "f:tests/outline.feature");
      await runAll(win);
      await select(win, "add <a> and <b>");
      await expect(win.getByTestId("details").getByTestId("example")).toHaveCount(3);
      await expect(win.locator("[data-testid=example][data-status=passed]")).toHaveCount(3);
      await shoot(win, "examples");
      await app.close();
    });

    await test.step("a broken config: banner, error and stepper, then the folder's list", async () => {
      // the least bottom panel and a tall window, so the banner, the error and the stepper show whole
      const { app, win } = await start(theme, { panel: 96, height: 1050 });
      await openProject(app, brokenConfigProject());
      await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });
      await openFeatures(win);
      await runAll(win);
      await select(win, "one");
      await expect(win.getByTestId("config-failure")).toContainText(
        "karate-config.js failed for 4",
      );
      await expect(win.getByTestId("error-group")).toHaveText(
        /^\d of 4 tests failed with this error$/,
      );
      await shoot(win, "failures");

      await nodeByKey(win, "d:suite").getByRole("button", { name: "suite", exact: true }).click();
      await expect(win.getByRole("heading", { name: "Failed with the same error" })).toBeVisible();
      await shoot(win, "failures-folder");
      await app.close();
    });
  });
}
