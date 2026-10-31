import type { Page } from "@playwright/test";
import {
  addProperty,
  expect,
  launch,
  node,
  nodeByKey,
  openProject,
  openProperties,
  project,
  saved,
  saveProperties,
  SECRET,
  test,
} from "./helpers";

const tab = (win: Page) => win.getByTestId("tab-http");
const rows = (win: Page) => win.getByTestId("http-call");
// The focus row is first in the DOM, so `nth` is not the place in the list: ask for the place.
const rowAt = (win: Page, n: number) =>
  win.locator(`[data-testid=http-call][aria-posinset="${n + 1}"]`);
const chip = (win: Page, group: string) =>
  win.locator(`[data-testid=http-group][data-group="${group}"]`);
const details = (win: Page, id: string) => win.getByTestId(`call-${id}`);

/** A sample project in a fresh app, with its tree showing. */
async function open(sample: "http" | "clean") {
  const dir = project(sample);
  const { app, win, userData } = await launch();
  await openProject(app, dir);
  const feature = sample === "http" ? "tests/http.feature" : "tests/basic.feature";
  await expect(nodeByKey(win, `f:${feature}`)).toBeVisible({ timeout: 15_000 });
  return { dir, app, win, userData };
}

async function runAll(win: Page) {
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
}

/** Clicks the scenario in the tree, which narrows the HTTP list to its calls. */
const select = (win: Page, name: string) =>
  node(win, name).getByRole("button", { name, exact: true }).click();

test("Reading the calls of one Run all", async () => {
  test.setTimeout(120_000);
  const { app, win } = await open("http");
  await runAll(win);

  await test.step("Every call of a run is listed and grouped", async () => {
    await expect(tab(win)).toHaveText("HTTP (13)");
    await tab(win).click();
    await expect(rows(win)).toHaveCount(13);
    for (const [group, text] of [
      ["all", "All 13"],
      ["1xx", "1xx Informational 0"],
      ["2xx", "2xx Success 6"],
      ["3xx", "3xx Redirect 1"],
      ["4xx", "4xx Client error 3"],
      ["5xx", "5xx Server error 1"],
      ["none", "No response 2"],
    ])
      await expect(chip(win, group)).toHaveText(text);
  });

  await test.step("Showing only the calls that went wrong", async () => {
    for (const group of ["4xx", "5xx", "none"]) await chip(win, group).click();
    await expect(chip(win, "all")).toHaveAttribute("aria-pressed", "false");
    await expect(rows(win)).toHaveCount(6);
    const statuses = await rows(win).evaluateAll((els) =>
      els.map((e) => e.getAttribute("data-status")),
    );
    expect(statuses.sort()).toEqual(["404", "404", "404", "500", "none", "none"]);

    await chip(win, "all").click(); // turns the others off
    await expect(rows(win)).toHaveCount(13);
  });

  await test.step("The tree selection narrows the list", async () => {
    await select(win, "retry");
    await expect(rows(win)).toHaveCount(3);
    await expect(tab(win)).toHaveText("HTTP (3)"); // the tab counts the rows it shows
    for (const status of await rows(win).evaluateAll((els) =>
      els.map((e) => e.getAttribute("data-status")),
    ))
      expect(status).toBe("404");
    await expect(win.getByTestId("http-call-attempt")).toHaveText([
      "attempt 1 of 3",
      "attempt 2 of 3",
      "attempt 3 of 3",
    ]);
  });

  await test.step("A call's request and response", async () => {
    await select(win, "xml");
    await expect(rows(win)).toHaveCount(1);
    await rows(win).click();
    await expect(rows(win)).toHaveAttribute("aria-selected", "true");
    await expect(details(win, "method")).toHaveText("POST");
    await expect(details(win, "url")).toHaveText(/\/echo$/);
    await expect(details(win, "status")).toHaveText("200 · 2xx Success");
    // `toContainText` squashes white space: the indent and the line breaks are the point
    expect(await details(win, "request-body").textContent()).toMatch(/^ {2}<name>bob<\/name>$/m);
  });

  await test.step("A call that got no response", async () => {
    await select(win, "refused");
    await expect(rows(win)).toHaveCount(1);
    await expect(win.getByTestId("http-call-status")).toHaveText("—");
    await expect(win.getByTestId("http-call-url")).toContainText("Connection refused");
    await rows(win).click();
    await expect(details(win, "error")).toContainText("Connection refused");
    await expect(win.getByTestId("copy-curl")).toBeDisabled();
  });

  await test.step("Copy as cURL", async () => {
    await select(win, "ok");
    await rows(win).click();
    await expect(details(win, "request-headers")).toContainText("Host:");
    await win.getByTestId("copy-curl").click();
    await expect(win.getByTestId("copy-curl")).toHaveText("Copied");
    const curl = await app.evaluate(({ clipboard }) => clipboard.readText());
    expect(curl.startsWith("curl -X GET 'http://localhost:")).toBe(true);
    expect(curl).toContain(" -H 'User-Agent: ");
    expect(curl).not.toMatch(/-H '(Host|Content-Length|Connection):/i);
  });

  await test.step("A shared setup call shows under the scenario Karate ran it in", async () => {
    await select(win, "first");
    await expect(rows(win)).toHaveCount(1);
    await expect(win.getByTestId("http-call-method")).toHaveText("POST");
    await expect(win.getByTestId("http-call-status")).toHaveText("201");
    await expect(win.getByTestId("http-call-via")).toHaveText(
      "via tests/common/create-user.feature:8",
    );
    await select(win, "second");
    await expect(win.getByTestId("http-empty")).toHaveText("No HTTP calls in second");
  });

  await test.step("Clicking a test gives the details pane back", async () => {
    await select(win, "ok");
    await rows(win).click();
    await expect(win.getByTestId("call-details")).toBeVisible();
    await select(win, "ok");
    await expect(win.getByTestId("call-details")).toBeHidden();
    await expect(win.getByTestId("details-location")).toHaveText("tests/http.feature:6");
    await expect(win.getByTestId("step")).toHaveCount(4); // background `url` + 3 scenario steps
  });

  await test.step("A row is selected with Enter or Space", async () => {
    await nodeByKey(win, "f:tests/http.feature")
      .getByRole("button", { name: /^http\.feature/ })
      .click();
    await expect(rows(win)).toHaveCount(12);
    const shows = async (n: number) => {
      const url = (await rowAt(win, n).getByTestId("http-call-url").textContent())!;
      await expect(rowAt(win, n)).toHaveAttribute("aria-selected", "true");
      await expect(win.getByTestId("call-details")).toBeVisible();
      await expect(details(win, "url")).toHaveText(url);
    };

    await rowAt(win, 0).focus();
    await win.keyboard.press("Enter");
    await shows(0);

    await rowAt(win, 3).focus();
    await win.keyboard.press("Space");
    await shows(3);
    await expect(rowAt(win, 0)).toHaveAttribute("aria-selected", "false");
  });
});

test("A call made in a called feature", async () => {
  test.setTimeout(120_000);
  const { win } = await open("clean");
  await runAll(win);

  await tab(win).click();
  await select(win, "calls helper feature");
  await expect(rows(win)).toHaveCount(1);
  await expect(win.getByTestId("http-call-method")).toHaveText("POST");
  await expect(win.getByTestId("http-call-status")).toHaveText("201");
  await expect(win.getByTestId("http-call-via")).toHaveText(
    "via tests/common/create-user.feature:8",
  );
});

test("A secret never shows in a call", async () => {
  test.setTimeout(90_000);
  const { app, win, userData, dir } = await open("http");
  await openProperties(win);
  await addProperty(win, "", "api.token", SECRET, true);
  await saveProperties(win);
  await expect
    .poll(() => saved(userData, dir)?.properties?.shared)
    .toEqual([{ name: "api.token", secret: expect.any(String) }]);

  await node(win, "secret header").getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 45_000 });
  await tab(win).click();
  await expect(rows(win)).toHaveCount(1);
  await rows(win).click();
  await expect(details(win, "request-headers")).toContainText("X-Token: ****");
  await win.getByTestId("copy-curl").click();
  await expect(win.getByTestId("copy-curl")).toHaveText("Copied");

  const clipboard = await app.evaluate(({ clipboard }) => clipboard.readText());
  expect(clipboard).toContain("-H 'X-Token: ****'");
  expect(clipboard).not.toContain(SECRET);
  // markup, text and the value of every field: outerHTML and innerText alone miss field values
  const everything = await win.evaluate(() =>
    [
      document.documentElement.outerHTML,
      document.body.innerText,
      ...[...document.querySelectorAll("input, textarea")].map(
        (e) => (e as HTMLInputElement).value,
      ),
    ].join("\n"),
  );
  expect(everything).not.toContain(SECRET);
});

test("A run of one test replaces only its calls", async () => {
  test.setTimeout(120_000);
  const { win } = await open("http");
  await runAll(win);
  await expect(tab(win)).toHaveText("HTTP (13)");
  await tab(win).click();

  await node(win, "ok").getByTestId("run-node").click();
  // a stored run has ended: the first Run all is the first entry
  await expect(win.getByTestId("tab-history")).toHaveText("History (2)", { timeout: 60_000 });
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished");
  await expect(tab(win)).toHaveText("HTTP (13)");
  // starting the run kept the HTTP tab open
  await expect(tab(win)).toHaveAttribute("aria-selected", "true");
  await expect(rows(win)).toHaveCount(13);
});
