// The step debugger in the built app (docs/specs/2026-11-02-step-debugger.md, its e2e scenarios).
import type { Page } from "@playwright/test";
import { expect, launch, node, openProject, ourJavaPids, project, test } from "./helpers";

async function pausedAtLine8() {
  const dir = project("debug");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(node(win, "variables")).toBeVisible({ timeout: 15_000 });
  await node(win, "variables").getByRole("button", { name: "variables", exact: true }).click();
  await win.getByRole("button", { name: "Add breakpoint at line 8" }).click();
  await expect(win.getByRole("button", { name: "Remove breakpoint at line 8" })).toBeVisible();
  await win.getByRole("button", { name: "Debug variables", exact: true }).click();
  await expect(win.getByTestId("debug-continue")).toBeVisible({ timeout: 30_000 });
  return { app, win };
}

const pausedStep = (win: Page) => win.locator("[data-testid=step][data-paused]");
const variable = (win: Page, name: string) =>
  win.locator(`[data-testid=variable][data-name="${name}"]`);

test("Debugging from the tree: paused, stepping, finished", async () => {
  test.setTimeout(90_000);
  const { win } = await pausedAtLine8();
  await expect(win.getByTestId("debug-over")).toBeVisible();
  await expect(win.getByTestId("debug-into")).toBeVisible();
  await expect(win.getByTestId("cancel-button")).toBeEnabled();
  await expect(win.getByTestId("run-all-button")).toHaveCount(0);
  await expect(node(win, "variables")).toHaveClass(/bg-selected/); // the paused test is selected
  await expect(pausedStep(win)).toContainText("def user = { id: 7 }");
  await expect(pausedStep(win)).toContainText("Paused");
  await expect(win.getByTestId("tab-variables")).toHaveAttribute("aria-selected", "true");
  await expect(win.getByTestId("variable")).toHaveCount(2);
  await expect(variable(win, "base")).toContainText("from background");
  await expect(variable(win, "name")).toContainText("grace");
  await expect(win.getByTestId("run-outcome")).toHaveText("Paused");

  await win.keyboard.press("F10");
  await expect(pausedStep(win)).toContainText("call read('called.feature')", { timeout: 15_000 });
  await expect(variable(win, "user")).toBeVisible();

  await win.getByTestId("debug-continue").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });
  await expect(win.getByTestId("run-all-button")).toBeVisible();
  await expect(win.getByTestId("tab-variables")).toHaveCount(0);
  await expect(node(win, "variables")).toHaveAttribute("data-status", "passed");
});

test("The debug controls at 760 px", async () => {
  test.setTimeout(90_000);
  const { app, win } = await pausedAtLine8();
  await app.evaluate(({ BrowserWindow }) =>
    BrowserWindow.getAllWindows()[0].setContentSize(1, 700),
  );
  await expect
    .poll(() =>
      app.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0].getBounds().width),
    )
    .toBe(760);
  const width = await win.evaluate(() => innerWidth);
  for (const id of ["cancel-button", "debug-continue", "debug-over", "debug-into"]) {
    const box = (await win.getByTestId(id).boundingBox())!;
    expect(box.x, id).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width, id).toBeLessThanOrEqual(width);
    expect(box.width, `${id} is its icon`).toBeLessThanOrEqual(28);
  }
  expect(await win.locator("header").evaluate((h) => h.scrollWidth <= h.clientWidth)).toBe(true);
  await win.getByTestId("cancel-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Cancelled", { timeout: 30_000 });
  await expect.poll(() => ourJavaPids().length, { timeout: 15_000 }).toBe(0);
});
