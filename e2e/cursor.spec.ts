import type { Locator, Page } from "@playwright/test";
import { expect, launch, node, nodeByKey, openProject, project, test } from "./helpers";

const cursor = (l: Locator) => l.evaluate((e) => getComputedStyle(e).cursor);

/** The clean sample project with "slow loop" running. */
async function slowRun() {
  const dir = project("clean");
  const { app, win } = await launch();
  await openProject(app, dir);
  const slow = nodeByKey(win, "f:tests/slow.feature");
  await expect(slow).toBeVisible({ timeout: 15_000 });
  await slow.getByTestId("run-node").click();
  await expect(node(win, "slow loop")).toHaveAttribute("data-status", "running", {
    timeout: 30_000,
  });
  return { app, win };
}

async function cancel(win: Page) {
  await win.getByTestId("cancel-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Cancelled", { timeout: 15_000 });
}

test("cursors match what a control does", async () => {
  const dir = project("tags");
  const { app, win } = await launch();
  await openProject(app, dir);
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });

  const feature = nodeByKey(win, "f:tests/tags.feature");
  const checkbox = win.getByTestId("hide-non-matching");
  for (const l of [
    win.getByTestId("run-all-button"),
    win.getByTestId("tab-console"),
    feature.locator("button.flex-1"),
    checkbox,
    win.locator("label", { has: checkbox }),
  ])
    expect(await cursor(l)).toBe("pointer");
  expect(await cursor(win.getByTestId("cancel-button"))).toBe("not-allowed");
  expect(await cursor(win.getByTestId("env-input"))).toBe("text");
  expect(await cursor(win.getByTestId("threads-input"))).toBe("text");
  expect(await cursor(win.getByTestId("splitter"))).toBe("row-resize");
});

test("the busy cursor while running", async () => {
  const { win } = await slowRun();
  expect(await cursor(win.getByTestId("app"))).toBe("progress");
  expect(await cursor(win.getByTestId("cancel-button"))).toBe("pointer");
  expect(await cursor(win.getByTestId("run-all-button"))).toBe("not-allowed");

  await cancel(win);
  await expect.poll(() => cursor(win.getByTestId("app"))).not.toBe("progress");
});

test("the drag cursor wins over the busy cursor", async () => {
  const { win } = await slowRun();
  const b = (await win.getByTestId("splitter").boundingBox())!;
  await win.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
  await win.mouse.down();
  await win.mouse.move(b.x + b.width / 2, b.y - 40, { steps: 3 });
  await expect.poll(() => cursor(win.getByTestId("app"))).toBe("row-resize");

  await win.mouse.up();
  await expect.poll(() => cursor(win.getByTestId("app"))).toBe("progress");
  await cancel(win);
});
