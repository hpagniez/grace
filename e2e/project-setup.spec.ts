import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, launch, node, nodeByKey, openProject, project, tempDir, test } from "./helpers";

/** Writes `files` (relative path → content) under `root`. */
function write(root: string, files: Record<string, string>) {
  for (const [f, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(root, f)), { recursive: true });
    writeFileSync(path.join(root, f), text);
  }
  return root;
}

test("karate-pom.json gives the tests and the config, and a notice names the rest", async () => {
  test.setTimeout(120_000);
  // a1 passes only when it really runs with conf/karate-config.js and no environment
  const dir = write(tempDir("kk-pom-"), {
    "tests/a/a.feature": "Feature: a\nScenario: a1\n  * match cfg == 'base'\n",
    "tests/b/b.feature": "Feature: b\nScenario: b1\n  * print 'b'\n",
    "conf/karate-config.js": "function fn() { return { cfg: 'base' }; }",
    "conf/karate-config-qa.js": "function fn() { return { cfg: 'qa' }; }",
    "karate-pom.json": JSON.stringify({
      paths: ["tests/a"],
      configDir: "conf",
      env: "qa",
      dryRun: true,
    }),
  });
  const { app, win } = await launch();
  await openProject(app, dir);

  await expect(nodeByKey(win, "f:tests/a/a.feature")).toBeVisible({ timeout: 15_000 });
  await expect(nodeByKey(win, "f:tests/b/b.feature")).toHaveCount(0);
  await expect(win.getByTestId("notice")).toHaveText(
    "Grace does not use these karate-pom.json settings: env, dryRun.",
  );
  await expect(win.getByTestId("env-input")).toHaveAttribute("placeholder", "(none)");
  await expect
    .poll(() =>
      win
        .locator("#env-suggestions option")
        .evaluateAll((os) => os.map((o) => (o as HTMLOptionElement).value)),
    )
    .toEqual(["(none)", "qa"]);

  await win.getByTestId("run-all-button").click();
  await expect(node(win, "a1")).toHaveAttribute("data-status", "passed", { timeout: 60_000 });
});

test("Maven's compiled output is on the classpath", async () => {
  test.setTimeout(120_000);
  const dir = write(project("maven"), {
    "target/test-classes/data/built.json": '{ "built": true }',
    "src/test/java/examples/built.feature":
      "Feature: built\nScenario: reads compiled output\n  * match read('classpath:data/built.json').built == true\n",
  });
  const { app, win } = await launch();
  await openProject(app, dir);

  await expect(node(win, "reads compiled output")).toBeVisible({ timeout: 15_000 });
  await node(win, "reads compiled output")
    .getByRole("button", { name: "Run reads compiled output", exact: true })
    .click();
  await expect(node(win, "reads compiled output")).toHaveAttribute("data-status", "passed", {
    timeout: 60_000,
  });
});
