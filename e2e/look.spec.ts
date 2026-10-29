// The premium look (spec 2026-10-29-premium-ui): what only the built app can show. Token values,
// contrast, class strings and the status marks' shapes are checked by unit tests.
import type { Locator, Page } from "@playwright/test";
import { rmSync } from "node:fs";
import path from "node:path";
import {
  emptyProject,
  expect,
  launch,
  launchWithSettings,
  node,
  nodeByKey,
  openProject,
  ourJavaPids,
  project,
  seedRuns,
  test,
} from "./helpers";

type Scheme = "light" | "dark";

/** The ground and ink tokens (R1.3) and the focus colour, as the browser reports them. */
const THEME: Record<Scheme, { ground: string; ink: string; focus: string }> = {
  light: { ground: "rgb(243, 239, 230)", ink: "rgb(21, 24, 33)", focus: "rgb(21, 24, 33)" },
  dark: { ground: "rgb(21, 24, 33)", ink: "rgb(241, 239, 233)", focus: "rgb(63, 207, 110)" },
};

/** Opens the clean sample project and waits for its tree. */
async function openClean(colorScheme?: Scheme) {
  const dir = project("clean");
  const { app, win, userData } = await launch(undefined, {}, { colorScheme });
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  return { dir, app, win, userData };
}

/** Starts the slow feature and waits until "slow loop" is running. */
async function startSlowRun(win: Page) {
  await nodeByKey(win, "f:tests/slow.feature").getByTestId("run-node").click();
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
}

/** Cancels the run and waits until no Java of ours is left. */
async function cancelRun(win: Page) {
  await win.getByTestId("cancel-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Cancelled", { timeout: 15_000 });
  await expect.poll(ourJavaPids, { timeout: 15_000 }).toEqual([]);
}

/** The test ids of the buttons filled with the accent colour: the primary buttons (R13.2). */
const primaryButtons = (win: Page) =>
  win.evaluate(() => {
    const probe = document.createElement("i");
    probe.style.backgroundColor = "var(--color-accent)";
    document.body.append(probe);
    const accent = getComputedStyle(probe).backgroundColor;
    probe.remove();
    return [...document.querySelectorAll("button")]
      .filter((b) => getComputedStyle(b).backgroundColor === accent)
      .map((b) => b.dataset.testid ?? b.textContent);
  });

// R1, R3: the window follows Windows

for (const scheme of ["light", "dark"] as const) {
  test(`The window follows Windows in ${scheme} mode`, async () => {
    const { app, win } = await launch(undefined, {}, { colorScheme: scheme });
    await openProject(app, project("http"));
    await expect(win.getByTestId("tree-node").first()).toBeVisible({ timeout: 15_000 });

    const want = THEME[scheme];
    const colour = (selector: string, prop: "backgroundColor" | "color") =>
      win
        .locator(selector)
        .first()
        .evaluate((e, p) => getComputedStyle(e)[p], prop);
    expect(await colour("html", "backgroundColor")).toBe(want.ground);
    expect(await colour("[data-testid=app]", "backgroundColor")).toBe(want.ground);
    expect(await colour("header", "color")).toBe(want.ink);
  });
}

// R2: type, offline

test("Grace draws its own typefaces without any network request", async () => {
  const { win } = await openClean();
  await win.context().setOffline(true);
  const urls: string[] = [];
  win.on("request", (r) => urls.push(r.url()));
  await win.reload(); // everything the page needs is requested again, with the listener on
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("console-banner")).toBeVisible();

  await win.evaluate(async () => void (await document.fonts.ready)); // the faces in use have loaded
  // document.fonts.check() is true also for a family nobody declared: ask for the loaded faces too
  const loaded = (family: string) =>
    win.evaluate(
      (f) =>
        [...document.fonts].some((face) => face.family.includes(f) && face.status === "loaded"),
      family,
    );
  expect(await win.evaluate(() => document.fonts.check("13px 'IBM Plex Sans Variable'"))).toBe(
    true,
  );
  expect(await win.evaluate(() => document.fonts.check("12px 'IBM Plex Mono'"))).toBe(true);
  expect(await loaded("IBM Plex Sans Variable")).toBe(true);
  expect(await loaded("IBM Plex Mono")).toBe(true);
  const family = (l: Locator) => l.evaluate((e) => getComputedStyle(e).fontFamily);
  expect(
    await family(node(win, "passing hello").getByRole("button", { name: /^passing hello/ })),
  ).toContain("IBM Plex Sans Variable");
  expect(await family(win.getByTestId("console"))).toContain("IBM Plex Mono");

  expect(urls.length).toBeGreaterThan(0);
  expect(urls.filter((u) => !/^(file|devtools):/.test(u))).toEqual([]);
  expect(urls.some((u) => u.endsWith(".woff2"))).toBe(true);
});

// R4, R7: statuses by shape and name

test("Each status has its own mark and name", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  await nodeByKey(win, "f:tests/basic.feature").getByTestId("run-node").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });

  const tree = win.getByTestId("tree");
  // beside a word the mark is decorative: the tree has no word, so its marks carry the names
  await expect(tree.getByRole("img", { name: "Passed", exact: true }).first()).toBeVisible();
  await expect(tree.getByRole("img", { name: "Failed", exact: true }).first()).toBeVisible();
  await expect(tree.getByRole("img", { name: "Not run", exact: true }).first()).toBeVisible();
  await expect(node(win, "passing hello").getByRole("img", { name: "Passed" })).toBeVisible();
  await expect(node(win, "failing match").getByRole("img", { name: "Failed" })).toBeVisible();
  await expect(
    nodeByKey(win, "f:tests/slow.feature").getByRole("img", { name: "Not run" }),
  ).toBeVisible();
});

test("A cancelled run marks its interrupted test", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  await startSlowRun(win);
  await cancelRun(win);

  await expect(node(win, "slow loop").getByRole("img", { name: "Interrupted" })).toBeVisible();
  await expect(win.getByTestId("run-outcome")).toHaveText("Cancelled");
  await expect(win.getByTestId("run-outcome").locator("svg")).toBeVisible();
});

// R5: motion

test("Reduced motion stops every animation", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  const running = () =>
    win.evaluate(() =>
      document
        .getAnimations()
        .filter((a) => a.playState === "running")
        .map((a) =>
          String((a as CSSAnimation).animationName ?? (a as CSSTransition).transitionProperty),
        ),
    );
  const cursor = win.getByTestId("console-cursor");
  await expect(cursor).toBeVisible(); // "Ready." while nothing runs
  // the control: with motion allowed, the cursor blinks
  expect(await cursor.evaluate((e) => e.getAnimations().length)).toBeGreaterThan(0);

  await win.emulateMedia({ reducedMotion: "reduce" });
  await expect.poll(() => cursor.evaluate((e) => e.getAnimations().length)).toBe(0);

  await nodeByKey(win, "f:tests/slow.feature").getByTestId("run-node").click();
  await expect(win.getByTestId("progress-stripe")).toBeVisible({ timeout: 15_000 });
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
  // the running mark, the waiting stripe: nothing is moving
  await expect.poll(running).toEqual([]);

  // the control again: with motion allowed, the running mark turns
  await win.emulateMedia({ reducedMotion: "no-preference" });
  await expect.poll(running).toContain("spin");
  await cancelRun(win);
});

// R6: the logo is the Home button

test("The logo is the Home button", async () => {
  const { win } = await openClean();
  const logo = win.getByRole("button", { name: "Home", exact: true });
  await expect(logo).toBeVisible();
  await expect(win.locator("header button").first()).toHaveAttribute("data-testid", "home-button");
  expect(
    await logo.locator("img").evaluate((i: HTMLImageElement) => i.naturalWidth),
  ).toBeGreaterThan(0);

  await logo.click();
  await expect(win.getByTestId("home")).toBeVisible();
});

// R9.4: tooltips (the toolbar is icon-only below 1280 px, so Refresh's tooltip is its name)

test("A tooltip shows on keyboard focus and closes with Escape", async () => {
  const { win } = await openClean();
  const refresh = win.getByTestId("refresh-button");
  await win.getByTestId("properties-button").focus();
  await win.keyboard.press("Tab");
  await expect(refresh).toBeFocused();
  // at once: well inside the 500 ms a pointer must rest
  await expect(win.getByRole("tooltip")).toHaveText("Refresh", { timeout: 300 });

  await win.keyboard.press("Escape");
  await expect(win.getByRole("tooltip")).toHaveCount(0);
  await expect(refresh).toBeFocused();
});

test("A tooltip shows after a pause on hover, also on a disabled button", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  await startSlowRun(win);
  await expect(win.getByTestId("refresh-button")).toBeDisabled();

  const tip = win.getByRole("tooltip");
  const before = Date.now();
  await win.getByTestId("refresh-button").hover({ force: true });
  await expect(tip).toHaveText("Refresh");
  expect(Date.now() - before).toBeGreaterThanOrEqual(480); // never earlier than the pause

  // onto the tooltip, across the gap to the button
  const box = (await tip.boundingBox())!;
  await win.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 5 });
  await win.waitForTimeout(300);
  await expect(tip).toHaveText("Refresh");

  await win.mouse.move(0, 0);
  await expect(tip).toHaveCount(0);
  await cancelRun(win);
});

test("A row's tooltip leaves the next row's run button to the pointer", async () => {
  test.setTimeout(90_000);
  const { win } = await openClean();
  const first = node(win, "passing hello");
  const second = node(win, "failing match");
  const centre = async (row: Locator) => {
    const b = (await row.getByTestId("run-node").boundingBox())!;
    return { x: b.x + b.width / 2, y: b.y + b.height / 2 };
  };
  const from = await centre(first);
  const to = await centre(second);
  expect(to.x).toBe(from.x); // one column: straight down
  expect(to.y).toBeGreaterThan(from.y);

  await win.mouse.move(from.x, from.y);
  await win.waitForTimeout(600);
  await expect(win.getByRole("tooltip")).toHaveText("Run passing hello");
  await win.mouse.move(to.x, to.y, { steps: 5 });
  await win.mouse.down();
  await win.mouse.up();

  await expect(second).toHaveAttribute("data-status", "failed", { timeout: 60_000 });
  await expect(first).toHaveAttribute("data-status", "notRun");
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
});

// R10: keyboard and accessibility

for (const scheme of ["light", "dark"] as const) {
  test(`Every control shows the focus ring in ${scheme} mode`, async () => {
    test.setTimeout(90_000);
    const { win } = await openClean(scheme);
    await node(win, "failing match").getByTestId("run-node").click();
    await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
    await node(win, "failing match")
      .getByRole("button", { name: /^failing match/ })
      .click();
    await expect(win.getByTestId("step-toggle").first()).toBeVisible(); // the details have buttons

    const stop = () =>
      win.evaluate(() => {
        const el = document.activeElement as HTMLElement | null;
        if (!el || el === document.body) return null;
        const style = getComputedStyle(el);
        const area = el.closest(
          "header, [data-testid=filter-bar], [data-testid=tree], [data-testid=details], [data-testid=bottom-panel], [data-testid=status-bar]",
        );
        return {
          name: `${el.tagName.toLowerCase()} ${el.dataset.testid ?? el.getAttribute("aria-label") ?? el.textContent?.slice(0, 20)}`,
          area: area
            ? area.tagName === "HEADER"
              ? "toolbar"
              : area.getAttribute("data-testid")
            : "elsewhere",
          outline: `${style.outlineWidth} ${style.outlineStyle} ${style.outlineColor}`,
        };
      });

    // start on the first control, then Tab through the whole page: every stop shows the ring
    await win.getByTestId("home-button").focus();
    const first = await stop();
    const stops: NonNullable<typeof first>[] = [];
    for (let i = 0; i < 200; i++) {
      await win.keyboard.press("Tab");
      const now = await stop();
      if (!now) continue;
      if (now.name === first!.name) break; // round again
      stops.push(now);
    }
    const ring = `2px solid ${THEME[scheme].focus}`;
    expect(stops.length).toBeGreaterThan(30); // 51 at the time of writing: the whole page, not the first few
    expect(stops.filter((s) => s.outline !== ring)).toEqual([]);
    expect(new Set(stops.map((s) => s.area))).toEqual(
      new Set(["toolbar", "filter-bar", "tree", "details", "bottom-panel", "status-bar"]),
    );
  });
}

test("Targets are at least 24 by 24", async () => {
  const dir = project("clean");
  const { win } = await launchWithSettings({ recentProjects: [path.resolve(dir)] });
  const atLeast24 = async (locator: Locator) => {
    await locator.first().waitFor();
    const all = await locator.all();
    expect(all.length).toBeGreaterThan(0);
    for (const one of all) {
      const box = (await one.boundingBox())!;
      expect(box.width).toBeGreaterThanOrEqual(24);
      expect(box.height).toBeGreaterThanOrEqual(24);
    }
  };
  await atLeast24(win.getByTestId("recent-remove"));

  await win.getByTestId("recent-row").click();
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await atLeast24(win.getByTestId("run-node"));
  await atLeast24(win.getByRole("tab"));
  await atLeast24(win.getByTestId("tag-chip"));
});

// R12, R16, R17, R18: states

test("A first start shows the first-run home screen", async () => {
  const { win } = await launch();
  await expect(win.getByTestId("mascot")).toBeVisible();
  await expect(win.getByRole("heading", { level: 1 })).toHaveText("Open a Karate project");
  await expect(
    win.getByText(
      "Pick the folder that holds your .feature files. Grace brings its own Java and Karate.",
    ),
  ).toBeVisible();
  await expect(win.getByTestId("open-button")).toHaveText("Open folder…");
  expect(await primaryButtons(win)).toEqual(["open-button"]);
});

test("A missing recent folder says so in words", async () => {
  const gone = emptyProject();
  rmSync(gone, { recursive: true, force: true });
  const { win } = await launchWithSettings({ recentProjects: [path.resolve(gone)] });
  const row = win.getByTestId("recent-row");
  await expect(row).toBeDisabled();
  await expect(win.getByTestId("recent-missing")).toHaveText("Not found");
  await expect(win.getByTestId("recent-missing").locator("svg")).toBeVisible();
});

test("A folder with no feature files shows an empty state", async () => {
  const { app, win } = await launch();
  await openProject(app, emptyProject());
  await expect(win.getByTestId("tree-message")).toHaveText("No feature files found", {
    timeout: 15_000,
  });
  await expect(
    win.getByText("Grace looks for .feature files under the project folder."),
  ).toBeVisible();
});

test("Before any run the progress line, the details and the tabs say so", async () => {
  const { win } = await openClean();
  await expect(win.getByTestId("progress-text")).toHaveText("9 tests · Not run yet");
  await expect(win.getByTestId("details")).toHaveText("Select a test to see its details");
  expect(await primaryButtons(win)).toEqual(["run-all-button"]); // R13.2

  await win.getByTestId("tab-history").click();
  await expect(win.getByTestId("history-empty")).toHaveText(
    "No runs yet. Runs appear here when they finish.",
  );
  await win.getByTestId("tab-http").click();
  await expect(win.getByTestId("http-empty")).toHaveText(
    "No HTTP calls yet. Calls appear when a feature finishes.",
  );
});

// R19: dialogs

test("Clear history keeps its button order and closes on Escape", async () => {
  const dir = project("clean");
  const userData = emptyProject();
  seedRuns(userData, dir, 2);
  const { app, win } = await launch(userData);
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/basic.feature")).toBeVisible({ timeout: 15_000 });
  await win.getByTestId("tab-history").click();
  await expect(win.getByTestId("history-row")).toHaveCount(2);

  const dialog = win.getByTestId("clear-history-dialog");
  const pointerEvents = () => dialog.evaluate((e) => getComputedStyle(e).pointerEvents);
  // a closed dialog takes no click while it fades out: the class holds from the start
  expect(await pointerEvents()).toBe("none");
  await win.getByTestId("clear-history").click();
  await expect(dialog).toBeVisible();
  expect(await pointerEvents()).toBe("auto");

  const del = (await win.getByTestId("clear-history-confirm").boundingBox())!;
  const cancel = (await win.getByTestId("clear-history-cancel").boundingBox())!;
  expect(del.x).toBeLessThan(cancel.x); // Delete before Cancel
  await expect(win.getByTestId("clear-history-cancel")).toBeFocused();

  await win.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  expect(await pointerEvents()).toBe("none");
  await expect(win.getByTestId("history-row")).toHaveCount(2); // nothing was deleted
});
