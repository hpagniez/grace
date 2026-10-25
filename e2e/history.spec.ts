import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import {
  chooseEnv,
  emptyProject,
  expect,
  launch,
  node,
  nodeByKey,
  openProject,
  openWithSecret,
  ourJavaPids,
  project,
  projectDataDir,
  runAlways,
  SECRET,
  seedRuns,
  stubOpenPath,
  test,
} from "./helpers";
import type { RunRecord } from "../src/shared/history";

const tab = (win: Page) => win.getByTestId("tab-history");
const rows = (win: Page) => win.getByTestId("history-row");
const notice = (win: Page) => win.getByTestId("notice");
const runIds = (win: Page) =>
  rows(win).evaluateAll((els) => els.map((e) => e.getAttribute("data-id")));
const keyOf = async (win: Page, name: string) => (await node(win, name).getAttribute("data-key"))!;

/** The clean sample project in a fresh app, with `seed` runs already in its history. */
async function openClean(seed = 0, opts: { report?: boolean; record?: Partial<RunRecord> } = {}) {
  const dir = project("clean");
  const userData = emptyProject();
  const ids = seedRuns(userData, dir, seed, opts);
  const { app, win } = await launch(userData);
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  return { dir, app, win, userData, ids };
}

/** The history sample project in a fresh app. */
async function openFix() {
  const dir = project("history");
  const { app, win, userData } = await launch();
  await openProject(app, dir);
  await expect(node(win, "fixable")).toBeVisible({ timeout: 15_000 });
  return { dir, app, win, userData };
}

/** Run all, then waits until the run is in the history (`n` runs): a stored run has ended. */
async function runAll(win: Page, n: number) {
  await win.getByTestId("run-all-button").click();
  await expect(tab(win)).toHaveText(`History (${n})`, { timeout: 90_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished");
}

/**
 * Records every status each scenario and row of the tree shows from now on. A test that goes pending
 * shows it for a moment only, so the tree after the run cannot tell which tests the run selected.
 */
async function trackStatuses(win: Page): Promise<() => Promise<Record<string, string[]>>> {
  await win.evaluate(() => {
    const seen: Record<string, string[]> = {};
    const scan = () => {
      const leaves =
        "[data-testid=tree-node][data-kind=scenario], [data-testid=tree-node][data-kind=row]";
      for (const el of document.querySelectorAll(leaves)) {
        const list = (seen[el.getAttribute("data-key")!] ??= []);
        const status = el.getAttribute("data-status")!;
        if (!list.includes(status)) list.push(status);
      }
    };
    scan();
    new MutationObserver(scan).observe(document.body, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["data-status"],
    });
    (window as unknown as { seen: typeof seen }).seen = seen;
  });
  return () => win.evaluate(() => (window as unknown as { seen: Record<string, string[]> }).seen);
}

/** Fails unless exactly the tests `keys` went pending, out of the 6 tests of the history sample project. */
function expectPendingOnly(seen: Record<string, string[]>, keys: string[]) {
  expect(Object.keys(seen)).toHaveLength(6);
  for (const [key, statuses] of Object.entries(seen)) {
    if (keys.includes(key)) expect(statuses, key).toContain("pending");
    else expect(statuses, key).not.toContain("pending");
  }
}

test("A run appears in the history", async () => {
  test.setTimeout(120_000);
  const { win } = await openClean();
  await expect(tab(win)).toHaveText("History (0)");

  await runAll(win, 1);
  await tab(win).click();
  const row = rows(win);
  await expect(row).toHaveCount(1);
  await expect(row.getByTestId("history-when")).toHaveText(/^Today \d\d:\d\d$/);
  await expect(row.getByTestId("history-env")).toHaveText("—");
  await expect(row.getByTestId("history-tags")).toHaveText("—");
  await expect(row.getByTestId("history-what")).toHaveText("All");
  await expect(row.getByTestId("history-result")).toHaveText("8 passed, 1 failed of 9");
  await expect(row.getByTestId("history-duration")).toHaveText(/^(\d+m \d\ds|\d+s)$/);
  await expect(row.getByTestId("history-report")).toBeEnabled();
  await expect(row.getByTestId("history-rerun")).toBeEnabled();
  await expect(row.getByTestId("history-rerun-failed")).toBeEnabled();
});

test("History survives a restart", async () => {
  test.setTimeout(90_000);
  const { app, dir, userData, ids } = await openClean(2, { report: true });

  // one app per user-data folder: close it before the second starts
  await app.close();
  const second = await launch(userData);
  const { win } = second;
  await expect(tab(win)).toHaveText("History (2)", { timeout: 15_000 });
  await tab(win).click();
  await expect.poll(() => runIds(win)).toEqual([ids[1], ids[0]]);

  const opened = await stubOpenPath(second.app);
  await rows(win).first().getByTestId("history-report").click();
  await expect.poll(opened).toHaveLength(1);
  expect((await opened())[0]).toBe(
    path.join(projectDataDir(userData, dir), "runs", ids[1], "karate-summary.html"),
  );
});

test("A cancelled run is listed without a report", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();

  await node(win, "slow loop").getByTestId("run-node").click();
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
  await win.getByTestId("cancel-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Cancelled", { timeout: 15_000 });
  await expect(tab(win)).toHaveText("History (1)");
  await tab(win).click();
  const row = rows(win);
  await expect(row.getByTestId("history-result")).toHaveText("Cancelled, 0 passed, 0 failed of 1");
  await expect(row.getByTestId("history-report")).toBeDisabled();
  await expect(row.getByTestId("history-rerun")).toBeEnabled();
});

test("Re-run failed runs only the failed tests", async () => {
  test.setTimeout(120_000);
  const { dir, win } = await openFix();
  await runAll(win, 1);
  await expect(node(win, "fixable")).toHaveAttribute("data-status", "failed");
  await expect(node(win, "row 2")).toHaveAttribute("data-status", "failed");
  await expect(node(win, "stays green")).toHaveAttribute("data-status", "passed");
  // "dynamic 2" fails too: it is the third failed test
  await expect(win.getByTestId("rerun-failed-button")).toHaveText("Re-run failed (3)");
  const fixable = await keyOf(win, "fixable");
  const row2 = await keyOf(win, "row 2");

  writeFileSync(path.join(dir, "flag.json"), JSON.stringify({ fixed: true }));
  const seen = await trackStatuses(win);
  await win.getByTestId("rerun-failed-button").click();
  await expect(tab(win)).toHaveText("History (2)", { timeout: 60_000 });

  expectPendingOnly(await seen(), [fixable, row2]);
  await expect(win.getByTestId("progress-text")).toHaveText("2 / 2");
  await expect(node(win, "fixable")).toHaveAttribute("data-status", "passed");
  await expect(node(win, "row 2")).toHaveAttribute("data-status", "failed");
  await tab(win).click();
  const newest = rows(win).first();
  await expect(newest.getByTestId("history-what")).toHaveText(/^Failed tests of Today \d\d:\d\d$/);
  await expect(newest.getByTestId("history-result")).toHaveText("1 passed, 1 failed of 2");
});

test("A failed dynamic row is skipped", async () => {
  test.setTimeout(120_000);
  const { win } = await openFix();
  await runAll(win, 1);
  await expect(node(win, "dynamic 2")).toHaveAttribute("data-status", "failed");
  const dynamic2 = await keyOf(win, "dynamic 2");
  const seen = await trackStatuses(win);

  await tab(win).click();
  await rows(win).first().getByTestId("history-rerun-failed").click();
  await expect(notice(win)).toHaveText(
    "1 failed row of a dynamic Examples table can't be run alone. Run its outline.",
    { timeout: 60_000 },
  );
  await expect(tab(win)).toHaveText("History (2)", { timeout: 60_000 });

  // the two other failed tests ran: only the dynamic row stayed out
  const pending = await seen();
  expect(pending[dynamic2]).not.toContain("pending");
  expect(pending[await keyOf(win, "fixable")]).toContain("pending");
  expect(pending[await keyOf(win, "row 2")]).toContain("pending");
  await expect(win.getByTestId("progress-text")).toHaveText("2 / 2");
});

test("Re-run switches back to the run's environment and filter", async () => {
  test.setTimeout(150_000);
  const { dir, win, userData } = await openFix();
  await chooseEnv(win, userData, dir, "qa");
  const smoke = win.locator("[data-testid=tag-chip][data-tag=smoke]");
  await smoke.click();
  await expect(smoke).toHaveAttribute("data-state", "include");
  await node(win, "fix.feature").getByTestId("run-node").click();
  await expect(tab(win)).toHaveText("History (1)", { timeout: 60_000 });
  await tab(win).click();
  await expect(rows(win).getByTestId("history-env")).toHaveText("qa");
  await expect(rows(win).getByTestId("history-tags")).toHaveText("@smoke");
  await expect(rows(win).getByTestId("history-what")).toHaveText("tests/fix.feature");
  await expect(rows(win).getByTestId("history-result")).toHaveText("1 passed, 0 failed of 1");

  const input = win.getByTestId("env-input");
  await input.fill("(none)");
  await input.press("Enter");
  await expect(input).toHaveValue("", { timeout: 30_000 });
  await win.getByTestId("tag-clear").click();
  await expect(smoke).toHaveAttribute("data-state", "off");
  const staysGreen = await keyOf(win, "stays green");
  const seen = await trackStatuses(win);

  await rows(win).first().getByTestId("history-rerun").click();
  await expect(input).toHaveValue("qa", { timeout: 60_000 });
  await expect(tab(win)).toHaveText("History (2)", { timeout: 60_000 });
  await expect(smoke).toHaveAttribute("data-state", "include");
  expectPendingOnly(await seen(), [staysGreen]);
  await expect(node(win, "stays green")).toHaveAttribute("data-status", "passed");
});

test("Cancelling the environment switch of a re-run runs nothing", async () => {
  test.setTimeout(90_000);
  const dir = project("env");
  const userData = emptyProject();
  seedRuns(userData, dir, 1, {
    record: {
      env: "qa",
      tests: [
        {
          kind: "scenario",
          file: "tests/env.feature",
          refId: "[1:3]",
          name: "always",
          line: 3,
          key: "s:tests/env.feature#1",
          status: "passed",
        },
      ],
    },
  });
  const { app, win } = await launch(userData);
  await openProject(app, dir);
  await expect(node(win, "always")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("env-input")).toHaveValue("");
  await expect(tab(win)).toHaveText("History (1)");
  await tab(win).click();

  await rows(win).first().getByTestId("history-rerun").click();
  // Cancel kills only a process that exists: click until the discovery of "qa" is cancelled
  await expect(async () => {
    await win
      .getByTestId("cancel-button")
      .click({ timeout: 1_000 })
      .catch(() => {}); // disabled before the switch starts and once it has ended
    await expect(notice(win)).toHaveText("Cancelled", { timeout: 300 });
  }).toPass({ timeout: 30_000, intervals: [100] });

  await expect(win.getByTestId("env-input")).toHaveValue("");
  await expect.poll(ourJavaPids).toEqual([]);
  await expect(win.getByTestId("run-outcome")).toHaveText("");
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 0");
  await expect(tab(win)).toHaveText("History (1)");
  expect(readdirSync(path.join(projectDataDir(userData, dir), "runs"))).toHaveLength(1);
});

test("A re-run of a test that no longer exists", async () => {
  test.setTimeout(90_000);
  const { dir, win } = await openClean(1, {
    record: {
      selection: {
        kind: "scenario",
        file: "tests/basic.feature",
        name: "failing match",
        line: 15,
        key: "s:tests/basic.feature#2",
      },
    },
  });
  const file = path.join(dir, "tests", "basic.feature");
  const before = readFileSync(file, "utf8");
  const after = before.replace(
    /@wip\r?\nScenario: failing match[\s\S]*?(?=Scenario: calls helper feature)/,
    "",
  );
  expect(after).not.toBe(before);
  writeFileSync(file, after);

  await tab(win).click();
  await rows(win).first().getByTestId("history-rerun").click();
  await expect(notice(win)).toHaveText("The tests of this run no longer exist", {
    timeout: 30_000,
  });
  await expect.poll(ourJavaPids).toEqual([]);
  await expect(win.getByTestId("run-outcome")).toHaveText("");
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 0");
  await expect(tab(win)).toHaveText("History (1)");
});

test("Only the 20 newest runs are kept", async () => {
  test.setTimeout(90_000);
  const { win, dir, userData, ids } = await openClean(20);
  await expect(tab(win)).toHaveText("History (20)");

  await node(win, "passing hello").getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
  await tab(win).click();
  await expect(rows(win).first().getByTestId("history-what")).toHaveText("passing hello", {
    timeout: 15_000,
  });
  await expect(rows(win)).toHaveCount(20);
  await expect(tab(win)).toHaveText("History (20)");
  const runs = path.join(projectDataDir(userData, dir), "runs");
  expect(existsSync(path.join(runs, ids[0]))).toBe(false);
  expect(existsSync(path.join(runs, ids[1]))).toBe(true);
  expect(readdirSync(runs)).toHaveLength(20);
});

test("Clear history", async () => {
  test.setTimeout(60_000);
  const { win, dir, userData } = await openClean(3);
  await expect(win.getByTestId("rerun-failed-button")).toHaveText("Re-run failed (1)");
  await expect(win.getByTestId("rerun-failed-button")).toBeEnabled();

  await tab(win).click();
  await win.getByTestId("clear-history").click();
  await expect(win.getByTestId("clear-history-dialog")).toContainText(
    "Delete the 3 runs of this project and their reports?",
  );
  await win.getByTestId("clear-history-confirm").click();
  await expect(win.getByTestId("history-empty")).toHaveText("No runs yet");
  await expect(tab(win)).toHaveText("History (0)");
  expect(readdirSync(path.join(projectDataDir(userData, dir), "runs"))).toEqual([]);
  await expect(win.getByTestId("report-button")).toBeDisabled();
  await expect(win.getByTestId("rerun-failed-button")).toBeDisabled();
});

test("A damaged run file does not hide the others", async () => {
  test.setTimeout(90_000);
  const { app, win: first, dir, userData, ids } = await openClean(3);
  await expect(tab(first)).toHaveText("History (3)");

  await app.close();
  writeFileSync(
    path.join(projectDataDir(userData, dir), "runs", ids[1], "run.json"),
    '{ "version": 1, "startedAt": ',
  );
  const { win } = await launch(userData);
  await expect(tab(win)).toHaveText("History (2)", { timeout: 15_000 });
  await tab(win).click();
  await expect.poll(() => runIds(win)).toEqual([ids[2], ids[0]]);
});

test("A secret never reaches the history files", async () => {
  test.setTimeout(120_000);
  const { win, userData } = await openWithSecret();

  await runAlways(win);
  await expect(tab(win)).toHaveText("History (1)", { timeout: 15_000 });
  const projects = path.join(userData, "projects");
  const records = (readdirSync(projects, { recursive: true }) as string[])
    .filter((f) => path.basename(f) === "run.json")
    .sort();
  expect(records).toHaveLength(1);
  const text = readFileSync(path.join(projects, records.at(-1)!), "utf8");
  expect(text).toContain('"env":"qa"'); // the record of this run
  expect(text).not.toContain(SECRET);
});

test("History actions wait for the runner", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean(1, { report: true });
  await tab(win).click();
  const actions = [
    win.getByTestId("history-report"),
    win.getByTestId("history-rerun"),
    win.getByTestId("history-rerun-failed"),
    win.getByTestId("clear-history"),
    win.getByTestId("rerun-failed-button"),
  ];
  for (const action of actions) await expect(action).toBeEnabled();

  await node(win, "slow loop").getByTestId("run-node").click();
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
  await tab(win).click(); // the run switched to the Console tab; switching tabs stays possible
  for (const action of actions) await expect(action).toBeDisabled();

  await win.getByTestId("cancel-button").click();
  await expect(tab(win)).toHaveText("History (2)", { timeout: 15_000 });
  // the cancelled run on top has no report and no failed test: the seeded run below is the one to check
  for (const action of actions.slice(0, 3)) await expect(action.last()).toBeEnabled();
  await expect(actions[3]).toBeEnabled();
});
