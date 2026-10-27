import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { createRunDir, writeRunRecord } from "../src/main/history";
import type { RunRecord, StoredTest } from "../src/shared/history";
import { NO_FILTER } from "../src/shared/tags";
import {
  chooseEnv,
  emptyProject,
  expect,
  flakyProject,
  launch,
  node,
  nodeByKey,
  openProject,
  project,
  projectDataDir,
  saved,
  stubOpenPath,
  test,
} from "./helpers";

const tab = (win: Page) => win.getByTestId("tab-history");
const mark = (win: Page, name: string) => node(win, name).getByTestId("flaky-mark");

/** The two tests of the flaky sample project, as a run stores them. */
const COIN_TESTS: StoredTest[] = [
  {
    kind: "scenario",
    file: "tests/coin.feature",
    refId: "[1:3]",
    name: "coin",
    line: 3,
    key: "s:tests/coin.feature#1",
    status: "passed",
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
 * Writes two runs of the flaky sample project at `root` into its history, before the app opens it: the
 * older with `coin` failed on 2 threads, the newer with `coin` passed on 1 thread, `steady` passed in
 * both, the same fingerprint and no environment. `report` adds a `karate-summary.html` to each.
 */
function seedCoin(userData: string, root: string, opts: { report?: boolean } = {}) {
  const projectDir = projectDataDir(userData, root);
  const now = Date.now();
  const write = (startedAt: number, coin: "passed" | "failed", threads: number) => {
    const { id, dir } = createRunDir(projectDir, new Date(startedAt));
    const record: RunRecord = {
      version: 1,
      startedAt,
      durationMillis: 5_000,
      env: null,
      threads,
      fingerprint: "f".repeat(64),
      filter: { ...NO_FILTER },
      selection: { kind: "folder", file: "", name: path.basename(root), line: null, key: "d:" },
      failedOf: null,
      outcome: "finished",
      exitCode: coin === "failed" ? 1 : 0,
      tests: COIN_TESTS.map((t) => (t.name === "coin" ? { ...t, status: coin } : t)),
    };
    writeRunRecord(dir, record);
    if (opts.report) writeFileSync(path.join(dir, "karate-summary.html"), "<html></html>");
    return id;
  };
  return { failed: write(now - 120_000, "failed", 2), passed: write(now - 60_000, "passed", 1) };
}

/** The flaky sample project in a fresh app, with the two runs of `seedCoin` already in its history. */
async function openSeeded(opts: { report?: boolean } = {}) {
  const { dir } = flakyProject();
  const userData = emptyProject();
  const ids = seedCoin(userData, dir, opts);
  const { app, win } = await launch(userData);
  await openProject(app, dir);
  await expect(node(win, "coin")).toBeVisible({ timeout: 15_000 });
  await expect(tab(win)).toHaveText("History (2)");
  return { dir, app, win, userData, ids };
}

/** The flaky sample project in a fresh app, with `coin.json` holding `heads`. */
async function openCoin(heads: boolean) {
  const { dir, coin } = flakyProject();
  coin(heads);
  const { app, win, userData } = await launch();
  await openProject(app, dir);
  await expect(node(win, "coin")).toBeVisible({ timeout: 15_000 });
  await trackEvents(win);
  return { dir, coin, app, win, userData };
}

/**
 * Records the `type` of every event main sends from now on in `window.events`, oldest first. The app's
 * own listener stays: `onEvent` adds a listener each call.
 */
async function trackEvents(win: Page) {
  await win.evaluate(() => {
    const events: string[] = [];
    (window as unknown as { events: string[] }).events = events;
    window.api.onEvent((e) => events.push(e.type));
  });
}

/**
 * Waits until the run is in the history (`n` runs): a stored run has ended. Main sends `flaky` after
 * the `history` of that run in a message of its own, so it also waits for that one: a "no mark" check
 * proves nothing before the marks of the new history are applied. Needs `trackEvents`.
 */
async function finished(win: Page, n: number) {
  await expect(tab(win)).toHaveText(`History (${n})`, { timeout: 90_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished");
  // not a count of `flaky` events: choosing an environment sends two (the new tree, then the history)
  await expect
    .poll(() =>
      win.evaluate(() => {
        const { events } = window as unknown as { events: string[] };
        return events.lastIndexOf("flaky") > events.lastIndexOf("history");
      }),
    )
    .toBe(true);
}

/** Types the count, presses Enter and waits until main has saved it. */
async function setThreads(win: Page, userData: string, dir: string, n: number) {
  const box = win.getByTestId("threads-input");
  await box.fill(String(n));
  await box.press("Enter");
  await expect.poll(() => saved(userData, dir)?.threads, { timeout: 30_000 }).toBe(n);
}

/** Selects the test in the tree, which shows it in Details. */
const select = (win: Page, name: string) =>
  node(win, name).getByRole("button", { name, exact: true }).click();

test("A test that flips with no change is marked flaky", async () => {
  test.setTimeout(120_000);
  const { dir, coin, win, userData } = await openCoin(false);

  await setThreads(win, userData, dir, 2);
  await win.getByTestId("run-all-button").click();
  await finished(win, 1);
  await expect(node(win, "coin")).toHaveAttribute("data-status", "failed");
  await expect(node(win, "steady")).toHaveAttribute("data-status", "passed");

  coin(true);
  await setThreads(win, userData, dir, 1);
  await win.getByTestId("rerun-failed-button").click();
  await finished(win, 2);

  await expect(node(win, "coin")).toHaveAttribute("data-status", "passed");
  await expect(mark(win, "coin")).toBeVisible();
  await expect(win.getByTestId("flaky-mark")).toHaveCount(1); // "steady" has none
  await expect(nodeByKey(win, "f:tests/coin.feature").getByTestId("flaky-count")).toHaveText(
    "1 flaky",
  );
  await expect(nodeByKey(win, "d:tests").getByTestId("flaky-count")).toHaveText("1 flaky");

  await select(win, "coin");
  const proof = win.getByTestId("flaky-proof");
  await expect(proof).toHaveCount(1);
  await expect(proof).toContainText("Flaky with no environment");
  await expect(proof).toContainText("Passed 1 time, failed 1 time with the same project files");
  const runs = proof.getByTestId("flaky-run");
  await expect(runs).toHaveCount(2);
  await expect(runs.nth(0)).toHaveAttribute("data-status", "passed");
  await expect(runs.nth(0).getByText("1 thread", { exact: true })).toBeVisible();
  await expect(runs.nth(1)).toHaveAttribute("data-status", "failed");
  await expect(runs.nth(1).getByText("2 threads", { exact: true })).toBeVisible();
});

test("A fix in the project is not flaky", async () => {
  test.setTimeout(120_000);
  const dir = project("history");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(node(win, "fixable")).toBeVisible({ timeout: 15_000 });
  await trackEvents(win);

  await win.getByTestId("run-all-button").click();
  await finished(win, 1);
  await expect(node(win, "fixable")).toHaveAttribute("data-status", "failed");

  writeFileSync(path.join(dir, "flag.json"), JSON.stringify({ fixed: true }));
  await win.getByTestId("rerun-failed-button").click();
  await finished(win, 2);

  await expect(node(win, "fixable")).toHaveAttribute("data-status", "passed");
  await expect(mark(win, "fixable")).toHaveCount(0);
  await expect(win.getByTestId("flaky-mark")).toHaveCount(0);
});

test("Different environments are not compared", async () => {
  test.setTimeout(120_000);
  const { dir, coin, win, userData } = await openCoin(false);

  await node(win, "coin").getByTestId("run-node").click();
  await finished(win, 1);
  await expect(node(win, "coin")).toHaveAttribute("data-status", "failed");

  coin(true);
  await chooseEnv(win, userData, dir, "qa");
  await node(win, "coin").getByTestId("run-node").click();
  await finished(win, 2);

  await expect(node(win, "coin")).toHaveAttribute("data-status", "passed");
  await expect(mark(win, "coin")).toHaveCount(0);
  await expect(win.getByTestId("flaky-mark")).toHaveCount(0);
});

test("The proof opens a run's report", async () => {
  test.setTimeout(120_000);
  const { app, win, dir, userData, ids } = await openSeeded({ report: true });
  const opened = await stubOpenPath(app);

  await select(win, "coin");
  await win
    .locator("[data-testid=flaky-run][data-status=failed]")
    .getByTestId("flaky-run-report")
    .click();
  await expect
    .poll(opened)
    .toEqual([path.join(projectDataDir(userData, dir), "runs", ids.failed, "karate-summary.html")]);
});

test("The mark stays on its test when the tree changes", async () => {
  test.setTimeout(120_000);
  const { win, dir } = await openSeeded();
  await expect(mark(win, "coin")).toBeVisible();

  const file = path.join(dir, "tests", "coin.feature");
  writeFileSync(
    file,
    readFileSync(file, "utf8").replace(
      "Scenario: coin",
      "Scenario: added\n  * match 1 == 1\n\nScenario: coin",
    ),
  );
  await win.getByTestId("refresh-button").click();

  await expect(node(win, "added")).toBeVisible({ timeout: 30_000 });
  await expect(mark(win, "coin")).toBeVisible();
  await expect(mark(win, "added")).toHaveCount(0);
  await expect(win.getByTestId("flaky-mark")).toHaveCount(1);
});

test("Marks survive a restart", async () => {
  test.setTimeout(120_000);
  const { app, userData } = await openSeeded();

  // one app per user-data folder: close it before the second starts
  await app.close();
  const { win } = await launch(userData);
  await expect(node(win, "coin")).toBeVisible({ timeout: 15_000 });
  await expect(mark(win, "coin")).toBeVisible();
});

test("Clear history removes the marks", async () => {
  test.setTimeout(120_000);
  const { win } = await openSeeded();
  await expect(mark(win, "coin")).toBeVisible();
  await expect(win.getByTestId("flaky-count").first()).toBeVisible();

  await tab(win).click();
  await win.getByTestId("clear-history").click();
  await win.getByTestId("clear-history-confirm").click();

  await expect(tab(win)).toHaveText("History (0)");
  await expect(win.getByTestId("flaky-mark")).toHaveCount(0);
  await expect(win.getByTestId("flaky-count")).toHaveCount(0);
});
