import { writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, node, openProject, stubVsCode, tempDir, test } from "./helpers";

/** A project whose karate-config.js fails on line 2, with 2 scenarios in a.feature and 1 in b.feature. */
function brokenConfigProject(): string {
  const dir = tempDir("kk-config-");
  const write = (file: string, lines: string[]) =>
    writeFileSync(path.join(dir, file), lines.join("\n") + "\n");
  write("karate-config.js", ["function fn() {", "  karate.fail('no vpn');", "  return {};", "}"]);
  const scenario = (name: string) => [`Scenario: ${name}`, "* print 'x'", ""];
  write("a.feature", ["Feature: a", "", ...scenario("one"), ...scenario("two")]);
  write("b.feature", ["Feature: b", "", ...scenario("three")]);
  return dir;
}

test("A broken karate-config.js, end to end (#229)", async () => {
  const dir = brokenConfigProject();
  const { app, win } = await launch();
  const opened = await stubVsCode(app, "Visual Studio Code");
  await openProject(app, dir);
  await expect(win.getByTestId("run-all-button")).toBeEnabled({ timeout: 15_000 });
  await win.getByTestId("run-all-button").click();
  await expect(win.getByTestId("run-outcome")).toHaveText("Finished", { timeout: 30_000 });

  for (const name of ["one", "two", "three"])
    await expect(node(win, name)).toHaveAttribute("data-status", "failed");
  const banner = win.getByTestId("config-failure");
  await expect(banner).toContainText("karate-config.js failed for 3 tests");
  await expect(banner).toContainText("no vpn");
  await win.getByRole("button", { name: "Open karate-config.js in VS Code" }).click();
  await expect
    .poll(opened)
    .toEqual([`vscode://file/${dir.replaceAll("\\", "/")}/karate-config.js:2`]);

  await node(win, "one").getByRole("button", { name: "one", exact: true }).click();
  await expect(win.getByTestId("error-group")).toHaveText("1 of 3 tests failed with this error");
  await win.getByRole("button", { name: "Previous test with this error" }).click();
  const three = node(win, "three");
  await expect(three).toBeVisible();
  await expect(three.locator("..")).toHaveAttribute("aria-selected", "true");
  await expect(win.getByTestId("error-group")).toHaveText("3 of 3 tests failed with this error");
});
