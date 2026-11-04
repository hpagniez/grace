import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import {
  expect,
  launch,
  node,
  nodeByKey,
  openFeatures,
  openProject,
  project,
  tempDir,
  test,
} from "./helpers";

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
      dryRun: true,
    }),
  });
  const { app, win } = await launch();
  await openProject(app, dir);
  await openFeatures(win);

  await expect(nodeByKey(win, "f:tests/a/a.feature")).toBeVisible({ timeout: 15_000 });
  await expect(nodeByKey(win, "f:tests/b/b.feature")).toHaveCount(0);
  await expect(win.getByTestId("notice")).toHaveText(
    "Grace does not use these karate-pom.json settings: dryRun.",
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

test("karate-pom.json starts the environment, filter and thread count", async () => {
  test.setTimeout(120_000);
  // fast passes only in qa; the other two must not run
  const dir = write(tempDir("kk-pom-start-"), {
    "a.feature":
      "Feature: a\n@smoke\nScenario: fast\n  * match cfg == 'qa'\n" +
      "@smoke @slow\nScenario: slow\n  * print 'slow'\nScenario: plain\n  * print 'plain'\n",
    "karate-config.js": "function fn() { return { cfg: 'base' }; }",
    "karate-config-qa.js": "function fn() { return { cfg: 'qa' }; }",
    "karate-pom.json": JSON.stringify({ env: "qa", tags: ["@smoke", "~@slow"], threads: 3 }),
  });
  const { app, win } = await launch();
  await openProject(app, dir);
  await openFeatures(win);

  await expect(node(win, "fast")).toBeVisible({ timeout: 15_000 });
  await expect(win.getByTestId("env-input")).toHaveValue("qa");
  const chip = (tag: string) => win.locator(`[data-testid=tag-chip][data-tag="${tag}"]`);
  await expect(chip("smoke")).toHaveAttribute("data-state", "include");
  await expect(chip("slow")).toHaveAttribute("data-state", "exclude");
  await expect(win.getByTestId("threads-input")).toHaveValue("3");
  await expect(win.getByTestId("notice")).toHaveCount(0);

  await win.getByTestId("run-all-button").click();
  await expect(node(win, "fast")).toHaveAttribute("data-status", "passed", { timeout: 60_000 });
  for (const name of ["slow", "plain"])
    await expect(node(win, name)).not.toHaveAttribute("data-status", /passed|failed/);
});

test("Maven's karate.env starts the environment", async () => {
  test.setTimeout(120_000);
  const dir = write(project("maven"), {
    "pom.xml":
      "<project><modelVersion>4.0.0</modelVersion><groupId>x</groupId><artifactId>x</artifactId><version>1</version>" +
      "<properties><karate.env>dev</karate.env></properties><build><plugins><plugin>" +
      "<artifactId>maven-surefire-plugin</artifactId><configuration><systemPropertyVariables>" +
      "<karate.env>${karate.env}</karate.env></systemPropertyVariables></configuration>" +
      "</plugin></plugins></build></project>",
  });
  const { app, win } = await launch();
  await openProject(app, dir);
  await openFeatures(win);
  await expect(win.getByTestId("env-input")).toHaveValue("dev", { timeout: 15_000 });
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
  await openFeatures(win);

  await expect(node(win, "reads compiled output")).toBeVisible({ timeout: 15_000 });
  await node(win, "reads compiled output")
    .getByRole("button", { name: "Run reads compiled output", exact: true })
    .click();
  await expect(node(win, "reads compiled output")).toHaveAttribute("data-status", "passed", {
    timeout: 60_000,
  });
});

/** `<dir>/lib.jar` holding a data.json, zipped by Windows' own tar; its folder has no space or `&`. */
function libJar(dir = tempDir("kk-libjar-")): string {
  const src = write(tempDir("kk-libsrc-"), { "data.json": '{ "lib": true }' });
  mkdirSync(dir, { recursive: true });
  const jar = path.join(dir, "lib.jar");
  const tar = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
  execFileSync(tar, ["--format", "zip", "-c", "-f", jar, "-C", src, "data.json"]);
  return jar;
}

const LIB_FEATURE =
  "Feature: lib\nScenario: reads a library\n  * match read('classpath:data.json').lib == true\n";

async function runsWithTheLibrary(dir: string) {
  const { app, win } = await launch();
  await openProject(app, dir);
  await openFeatures(win);
  await expect(node(win, "reads a library")).toBeVisible({ timeout: 15_000 });
  await node(win, "reads a library")
    .getByRole("button", { name: "Run reads a library", exact: true })
    .click();
  await expect(node(win, "reads a library")).toHaveAttribute("data-status", "passed", {
    timeout: 60_000,
  });
}

test("a Maven project's libraries come from its build", async () => {
  test.setTimeout(120_000);
  const dir = write(project("maven"), {
    "src/test/java/examples/lib.feature": LIB_FEATURE,
    // a stand-in for Maven: it lists the jar where Grace asks
    "mvnw.cmd": `@echo off\r\n>"%GRACE_CLASSPATH_FILE%" echo ${libJar()}\r\n`,
  });
  await runsWithTheLibrary(dir);
});

test("jars in .karate/ext are on the classpath", async () => {
  test.setTimeout(120_000);
  const dir = write(tempDir("kk-ext-"), { "lib.feature": LIB_FEATURE });
  libJar(path.join(dir, ".karate", "ext"));
  await runsWithTheLibrary(dir);
});
