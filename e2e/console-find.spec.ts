import { writeFileSync } from "node:fs";
import path from "node:path";
import type { ElectronApplication, Page } from "@playwright/test";
import { emptyProject, expect, launch, openProject, tempDir, test } from "./helpers";

/** A one-feature project, deleted after the test. */
function oneFeature(text: string): string {
  const dir = tempDir("kk-find-");
  writeFileSync(path.join(dir, "find.feature"), text);
  return dir;
}

async function runAll(app: ElectronApplication, win: Page, dir: string) {
  await openProject(app, dir);
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
}

/** The text of the ranges in one of the console's highlights. */
const painted = (win: Page, name: string) =>
  win.evaluate(
    (n) =>
      [...(CSS.highlights.get(n) ?? [])].map((r) =>
        (r.startContainer as Text).data.slice(r.startOffset, r.endOffset),
      ),
    name,
  );

test("find in a real run", async () => {
  test.setTimeout(120_000);
  const dir = oneFeature(
    "Feature: find\n\nScenario: one\n  * print 'zebra-42'\n\nScenario: two\n  * print 'zebra-42'\n",
  );
  const { app, win } = await launch();
  await runAll(app, win, dir);
  await win.keyboard.press("Control+f");
  await expect(win.getByTestId("find-field")).toBeFocused();
  await win.keyboard.type("zebra-42");
  const count = win.getByTestId("find-count");
  await expect(count).toHaveText(/^[12] of 2$/);
  expect(await painted(win, "console-match")).toEqual(["zebra-42", "zebra-42"]);
  const first = await count.textContent();
  await win.keyboard.press("Enter");
  await expect(count).toHaveText(first === "1 of 2" ? "2 of 2" : "1 of 2");
});

test("step through a failing run's errors", async () => {
  test.setTimeout(120_000);
  const dir = oneFeature(
    "Feature: errors\n\nScenario: fails\n  * match 1 == 2\n\nScenario: logs\n  * karate.logger.error('custom boom')\n",
  );
  const { app, win } = await launch();
  await runAll(app, win, dir);
  const count = win.getByTestId("errors-count");
  await expect(count).toHaveText("2 errors");
  // the ERROR line comes during the run, the failed test in Karate's summary at its end
  await win.keyboard.press("F8");
  await expect(count).toHaveText(/^[12] of 2 errors$/);
  if ((await count.textContent()) === "1 of 2 errors") await win.keyboard.press("F8");
  await expect(count).toHaveText("2 of 2 errors");
  expect(await painted(win, "console-error-current")).toEqual(["    - fails"]);
  await win.keyboard.press("F8");
  await expect(count).toHaveText("1 of 2 errors");
  expect((await painted(win, "console-error-current"))[0]).toContain("ERROR");
});

test("a full console", async () => {
  const { app, win } = await launch();
  await openProject(app, emptyProject());
  await expect(win.getByTestId("console")).toBeVisible();
  // Sent as main sends them: a real run's summary and banner would push matching lines out at the cap.
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].webContents.send("app:event", [
      {
        type: "console",
        lines: Array.from({ length: 10_000 }, (_, i) => `line ${i} match-me`),
      },
    ]),
  );
  await expect(win.getByTestId("console").locator(":scope > div").last()).toHaveText(
    "line 9999 match-me",
  );
  await win.keyboard.press("Control+f");
  await win.keyboard.type("match-m");
  // the last keystroke's work, synchronous, and until the next frame is painted (spec Cost: under 16 ms)
  const ms = await win.evaluate(async () => {
    const field = document.querySelector<HTMLInputElement>('[data-testid="find-field"]')!;
    const start = performance.now();
    field.value = "match-me";
    field.dispatchEvent(new InputEvent("input", { bubbles: true }));
    const work = performance.now() - start;
    // a frame's callbacks run before its paint: the task after it runs once the highlights are painted
    await new Promise((done) => requestAnimationFrame(() => setTimeout(done)));
    return { work: Math.round(work), frame: Math.round(performance.now() - start) };
  });
  test.info().annotations.push({
    type: "keystroke on 10,000 matching lines",
    description: `${ms.work} ms work, ${ms.frame} ms to the next frame`,
  });
  console.log(`keystroke on 10,000 matching lines: ${ms.work} ms work, ${ms.frame} ms to frame`);
  await expect(win.getByTestId("find-count")).toHaveText(/^\d+ of 10000$/);
});
