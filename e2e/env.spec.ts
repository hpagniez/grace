import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import {
  addProperty,
  chooseEnv,
  emptyProject,
  expect,
  launch,
  node,
  openEnv,
  openEnvWith,
  openProject,
  openProperties,
  openWithSecret,
  ourJavaCommandLines,
  ourJavaPids,
  project,
  runAlways,
  saved,
  saveProperties,
  scopeItem,
  SECRET,
  test,
} from "./helpers";

const input = (win: Page) => win.getByTestId("env-input");
const consoleText = (win: Page) => win.getByTestId("console");

test("Suggestions come from overlay files and env tags", async () => {
  const { win } = await openEnv();

  await expect(input(win)).toHaveValue("");
  await expect(input(win)).toHaveAttribute("placeholder", "(none)");
  await expect
    .poll(() =>
      win
        .locator("#env-suggestions option")
        .evaluateAll((os) => os.map((o) => (o as HTMLOptionElement).value)),
    )
    .toEqual(["(none)", "prod", "qa"]);
});

test("Changing the environment updates the tree", async () => {
  const { win } = await openEnv();
  await expect(node(win, "qa only")).toHaveCount(0);
  await expect(node(win, "not prod")).toBeVisible();

  await input(win).fill("qa");
  await input(win).press("Enter");
  await expect(node(win, "qa only")).toBeVisible({ timeout: 30_000 });
  await expect(node(win, "not prod")).toBeVisible();

  await input(win).fill("prod");
  await input(win).press("Enter");
  await expect(node(win, "qa only")).toHaveCount(0, { timeout: 30_000 });
  await expect(node(win, "not prod")).toHaveCount(0);
  await expect(node(win, "always")).toBeVisible();
});

test("Changing the environment clears results", async () => {
  test.setTimeout(90_000);
  const { win, userData, dir } = await openEnvWith("qa");
  await runAlways(win);
  await expect(node(win, "always")).toHaveAttribute("data-status", "passed");
  await expect(win.getByTestId("report-button")).toBeEnabled();

  await chooseEnv(win, userData, dir, "prod");
  await expect(node(win, "always")).toHaveAttribute("data-status", "notRun");
  await expect(win.getByTestId("report-button")).toBeDisabled();
});

test("A run uses the environment", async () => {
  test.setTimeout(90_000);
  const { win } = await openEnvWith("qa");

  await runAlways(win);
  await expect(consoleText(win)).toContainText("karate.env = qa");
  await expect(win.getByTestId("run-env")).toHaveText("qa");
});

test("An environment without an overlay file", async () => {
  test.setTimeout(90_000);
  const { win } = await openEnv();

  await input(win).fill("staging");
  await input(win).press("Enter");
  await expect(win.getByTestId("env-hint")).toHaveText(
    "No karate-config-staging.js found. Fine if karate-config.js handles it.",
    { timeout: 30_000 },
  );
  await expect(input(win)).toHaveValue("staging");

  await runAlways(win);
  await expect(consoleText(win)).toContainText("karate.env = staging");
  await expect(win.getByTestId("run-env")).toHaveText("staging");
});

test("Shared and environment properties, environment wins", async () => {
  test.setTimeout(120_000);
  const { win, userData, dir } = await openEnvWith("qa");
  await openProperties(win);
  await addProperty(win, "", "timeout", "5000");
  await addProperty(win, "qa", "timeout", "9000");
  await saveProperties(win);
  await expect
    .poll(() => saved(userData, dir)?.properties)
    .toEqual({
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: { qa: [{ name: "timeout", value: "9000" }] },
    });

  await runAlways(win);
  await expect(consoleText(win)).toContainText("timeout = 9000");

  // "(none)" typed in the box: it is the empty box again once main has echoed the change
  await input(win).fill("(none)");
  await input(win).press("Enter");
  await expect(input(win)).toHaveValue("", { timeout: 30_000 });
  await expect(node(win, "qa only")).toHaveCount(0);
  await expect(win.getByTestId("run-outcome")).toHaveText("");
  await runAlways(win);
  await expect(consoleText(win)).toContainText("timeout = 5000");
  await expect(consoleText(win)).not.toContainText("timeout = 9000");
  await expect(win.getByTestId("run-env")).toHaveText("");
});

test("A secret is used but never shown", async () => {
  test.setTimeout(90_000);
  const { win } = await openWithSecret();

  await runAlways(win);
  await expect(consoleText(win)).toContainText("api.token = ****");
  // Step logs are collapsed until opened, and a print step's log is where an unmasked value would show.
  await node(win, "always").getByRole("button", { name: "always", exact: true }).click();
  await expect(win.getByTestId("details-location")).toHaveText("tests/env.feature:3");
  const toggles = win.getByTestId("step-toggle");
  await expect(toggles.first()).toBeVisible();
  for (const toggle of await toggles.all()) await toggle.click();
  await expect(win.getByTestId("step-log").filter({ hasText: "api.token = ****" })).toHaveCount(1);
  // markup, text and the value of every field: outerHTML and innerText alone miss field values
  const everything = () =>
    win.evaluate(() =>
      [
        document.documentElement.outerHTML,
        document.body.innerText,
        ...[...document.querySelectorAll("input, textarea")].map(
          (e) => (e as HTMLInputElement).value,
        ),
      ].join("\n"),
    );
  expect(await everything()).not.toContain(SECRET);

  await openProperties(win);
  await scopeItem(win, "qa").click();
  const row = win.getByTestId("property-row");
  await expect(row).toHaveCount(1);
  await expect(row.getByTestId("prop-name")).toHaveValue("api.token");
  await expect(row.getByTestId("prop-dots")).toBeVisible();
  await expect(row.getByTestId("prop-replace")).toBeVisible();
  await expect(row.getByTestId("prop-value")).toHaveCount(0);
  expect(await everything()).not.toContain(SECRET);
});

test("A secret stays off the command line and out of the settings file", async () => {
  test.setTimeout(90_000);
  const { win, userData } = await openWithSecret();

  await node(win, "slow").getByTestId("run-node").click();
  // the pre-run dry run has no argument file; the run itself has one
  let lines: string[] = [];
  await expect
    .poll(
      () => {
        lines = ourJavaCommandLines();
        return lines.some((l) => l.includes("run-args.txt"));
      },
      { timeout: 30_000, intervals: [200] },
    )
    .toBe(true);
  for (const line of lines) expect(line).not.toContain(SECRET);
  const settings = readFileSync(path.join(userData, "settings.json"), "utf8");
  expect(settings).not.toContain(SECRET);

  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  const argFiles = () =>
    (readdirSync(path.join(userData, "projects"), { recursive: true }) as string[]).filter(
      (f) => path.basename(f) === "run-args.txt",
    );
  await expect.poll(argFiles).toEqual([]);
});

test("Reserved names are refused", async () => {
  const { win } = await openEnv();
  await openProperties(win);

  await addProperty(win, "", "karate.options", "-t @x");
  await expect(win.getByTestId("row-error")).toHaveText("Reserved name");
  await expect(win.getByTestId("save-properties")).toBeDisabled();
});

test("An unreadable secret blocks runs", async () => {
  test.setTimeout(90_000);
  const dir = project("env");
  const userData = emptyProject();
  writeFileSync(
    path.join(userData, "settings.json"),
    JSON.stringify({
      projects: {
        [path.resolve(dir)]: {
          env: "qa",
          properties: {
            shared: [],
            byEnv: { qa: [{ name: "api.token", secret: "bm90LWEtcmVhbC1jaXBoZXJ0ZXh0" }] },
          },
        },
      },
    }),
  );
  const { app, win } = await launch(userData);
  await openProject(app, dir);
  await expect(node(win, "always")).toBeVisible({ timeout: 15_000 });
  await expect(input(win)).toHaveValue("qa");

  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("notice")).toHaveText(
    "Secret api.token (qa) can't be read. Replace it in Properties.",
    { timeout: 30_000 },
  );
  await expect(win.getByTestId("run-outcome")).toHaveText("");
  await expect(win.getByTestId("progress-text")).toHaveText("0 / 0");
  await expect(node(win, "always")).toHaveAttribute("data-status", "notRun");
  // refused before the pre-run dry run: no JVM launched
  await expect.poll(ourJavaPids).toEqual([]);

  await openProperties(win);
  await scopeItem(win, "qa").click();
  await expect(win.getByTestId("prop-unreadable")).toHaveText("Can't be read, enter it again");
});

test("Settings survive a restart", async () => {
  test.setTimeout(90_000);
  const { app, userData } = await openWithSecret();

  // one app per user-data folder: close it before the second starts
  await app.close();
  const { win } = await launch(userData);
  await expect(input(win)).toHaveValue("qa", { timeout: 15_000 });
  await expect(node(win, "qa only")).toBeVisible();

  await openProperties(win);
  await scopeItem(win, "qa").click();
  const row = win.getByTestId("property-row");
  await expect(row).toHaveCount(1);
  await expect(row.getByTestId("prop-name")).toHaveValue("api.token");
  await expect(row.getByTestId("prop-secret")).toBeChecked();
  await expect(row.getByTestId("prop-dots")).toBeVisible();
  await expect(row.getByTestId("prop-replace")).toBeVisible();
  await expect(row.getByTestId("prop-unreadable")).toHaveCount(0);
});

test("The machine's KARATE_ENV is ignored", async () => {
  test.setTimeout(90_000);
  const { win } = await openEnv({ KARATE_ENV: "prod" });

  // discovery ignored it too: with prod, "not prod" would be absent
  await expect(node(win, "not prod")).toBeVisible();
  await expect(input(win)).toHaveValue("");
  await runAlways(win);
  await expect(consoleText(win)).toContainText("karate.env = null");
});
