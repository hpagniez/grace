import type { Locator } from "@playwright/test";
import { expect, launch, nodeByKey, openProject, project, test } from "./helpers";

const scrollbar = (l: Locator) =>
  l.evaluate((e) => {
    const s = getComputedStyle(e);
    return { width: s.scrollbarWidth, color: s.scrollbarColor };
  });

/** Each scroll area's scrollbar style, in one colour scheme, after a run of the http sample project. */
async function areas(colorScheme: "light" | "dark") {
  const dir = project("http");
  const { app, win } = await launch(undefined, {}, { colorScheme });
  await openProject(app, dir);
  await expect(nodeByKey(win, "f:tests/http.feature")).toBeVisible({ timeout: 15_000 });
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 60_000 });
  const found = {
    tree: await scrollbar(win.getByTestId("tree").locator("..")),
    details: await scrollbar(win.getByTestId("details")),
    console: await scrollbar(win.getByTestId("console")),
    history: { width: "", color: "" },
    http: { width: "", color: "" },
  };
  await win.getByTestId("tab-history").click();
  found.history = await scrollbar(win.getByTestId("history"));
  await win.getByTestId("tab-http").click();
  found.http = await scrollbar(win.getByTestId("http-calls"));
  await app.close();
  return found;
}

test("scroll areas get thin themed scrollbars", async () => {
  const light = await areas("light");
  const dark = await areas("dark");
  for (const found of [light, dark])
    for (const { width, color } of Object.values(found)) {
      expect(width).toBe("thin");
      expect(color).toMatch(/ rgba\(0, 0, 0, 0\)$/); // transparent track
    }
  // the thumb is `control` at 60 % (R1.7): #8a8478 in the light theme, #6a707e in the dark one
  expect(light.tree.color).toMatch(/^color\(srgb 0\.541176 0\.517647 0\.470588 \/ 0\.6\) /);
  expect(dark.tree.color).toMatch(/^color\(srgb 0\.415686 0\.439216 0\.494118 \/ 0\.6\) /);
  expect(light.history.color).toBe(light.tree.color);
  expect(dark.http.color).toBe(dark.tree.color);
  expect(light.console.color).toBe(light.tree.color);
  expect(dark.console.color).toBe(dark.tree.color);
});
