import { existsSync, mkdirSync, mkdtempSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { byName, cleanUp, T, setup, fixture, makeJar, temps } from "../../test/runner";
import { leafKeys } from "./karate/tree";

afterEach(cleanUp);

/** Writes `files` (relative path → content) into a fresh temp project. */
function write(files: Record<string, string>, root = mkdtempSync(path.join(tmpdir(), "kk-pom-"))) {
  if (!temps.includes(root)) temps.push(root);
  for (const [f, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(root, f)), { recursive: true });
    writeFileSync(path.join(root, f), text);
  }
  return root;
}

/** a.feature passes only when it really runs with conf/karate-config.js and no environment. */
const POM_PROJECT = {
  "tests/a/a.feature": "Feature: a\nScenario: a1\n  * match cfg == 'base'\n",
  "tests/b/b.feature": "Feature: b\nScenario: b1\n  * print 'b'\n",
  "conf/karate-config.js": "function fn() { return { cfg: 'base' }; }",
  "conf/karate-config-qa.js": "function fn() { return { cfg: 'qa' }; }",
  "karate-pom.json": JSON.stringify({
    paths: ["tests/a", "missing"],
    configDir: "conf",
    dryRun: true,
  }),
};

describe("Runner project setup", () => {
  it("takes paths and configDir from karate-pom.json and names the rest", T, async () => {
    const { runner, last } = setup();
    await runner.open(write(POM_PROJECT));
    expect(leafKeys(last("project").tree!)).toEqual(["s:tests/a/a.feature#1"]);
    expect(last("notice").message).toBe(
      "Grace does not use these karate-pom.json settings: dryRun. " +
        "These karate-pom.json paths are not folders or files in the project: missing.",
    );
    expect(last("environment").view).toMatchObject({ env: null, overlays: ["qa"] });
    await runner.run("d:");
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  it("writes JUnit XML when karate-pom.json asks", T, async () => {
    const { runner, all, last, dataDir } = setup();
    await runner.open(
      write({
        "tests/a.feature": "Feature: a\nScenario: a1\n  * print 'a'\n",
        "karate-pom.json": '{"output":{"junitXml":true}}',
      }),
    );
    await runner.run("d:");
    expect(last("runFinished").outcome).toBe("finished");
    const [project] = readdirSync(path.join(dataDir, "projects"));
    const projectDir = path.join(dataDir, "projects", project);
    const [run] = readdirSync(path.join(projectDir, "runs"));
    expect(readdirSync(path.join(projectDir, "runs", run, "junit-xml"))).toHaveLength(1);
    // the dry run's folder is kept, so a junit-xml in it would show
    expect(existsSync(path.join(projectDir, "dryrun", "karate-json"))).toBe(true);
    expect(existsSync(path.join(projectDir, "dryrun", "junit-xml"))).toBe(false);
    expect(all("notice").filter((n) => n.message.includes("output"))).toEqual([]);
  });

  it("says it only on open", T, async () => {
    const { runner, all } = setup();
    await runner.open(write(POM_PROJECT));
    const seen = all("notice").length;
    await runner.refresh();
    expect(all("notice")).toHaveLength(seen);
  });

  it("puts Maven's compiled output on the classpath", T, async () => {
    const { runner, last } = setup();
    const root = write(
      {
        "target/test-classes/data/built.json": '{ "built": true }',
        "src/test/java/examples/built.feature":
          "Feature: built\nScenario: reads compiled output\n  * match read('classpath:data/built.json').built == true\n",
      },
      fixture("maven"),
    );
    await runner.open(root);
    // the key depends on how Karate names the file: find it by name
    await runner.run(byName(last("project").tree!, "reads compiled output").key);
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  // Karate may name the files relative to src/test/java, which holds karate-config.js (#208)
  it("runs one row of a Maven outline", T, async () => {
    const { runner, last } = setup();
    const root = write(
      {
        "src/test/java/examples/rows.feature":
          "Feature: rows\nScenario Outline: row <n>\n  * assert <n> > 0\nExamples:\n  | n |\n  | 1 |\n  | 2 |\n",
      },
      fixture("maven"),
    );
    await runner.open(root);
    expect(last("project").error).toBeNull();
    const row = byName(last("project").tree!, "row 2").key;
    await runner.run(row);
    expect(last("runStarted")).toMatchObject({ keys: [row], total: 1 });
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  // Karate names the files relative to a configDir that holds them (#223)
  it("runs one row of an outline in the configDir", T, async () => {
    const { runner, last, all } = setup();
    await runner.open(
      write({
        "tests/karate-config.js": "function fn() { return { cfg: 'base' }; }",
        "tests/a/rows.feature":
          "Feature: rows\nScenario Outline: row <n>\n  * match cfg == 'base'\nExamples:\n  | n |\n  | 1 |\n  | 2 |\n",
        "karate-pom.json": JSON.stringify({ configDir: "tests" }),
      }),
    );
    expect(last("project").error).toBeNull();
    const row = byName(last("project").tree!, "row 2").key;
    await runner.run(row);
    expect(all("scenarios").flatMap((e) => e.updates)).toContainEqual(
      expect.objectContaining({ key: row, status: "passed" }),
    );
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  it("uses the default folders when karate-pom.json cannot be read", T, async () => {
    const { runner, last } = setup();
    const root = write({ "karate-pom.json": "{" }, fixture("clean"));
    await runner.open(root);
    expect(
      leafKeys(last("project").tree!).some((k) => k.startsWith("s:tests/basic.feature#")),
    ).toBe(true);
    expect(last("notice").message).toMatch(
      /^karate-pom\.json could not be read: .+\. Grace uses the default folders\.$/,
    );
  });

  /** Passes only when a jar holding data.json is on the classpath. */
  const EXT_PROJECT = {
    "t.feature": "Feature: t\nScenario: ext\n  * match read('classpath:data.json').ext == true\n",
  };

  it("puts the jars of the project's .karate/ext on the classpath", T, async () => {
    const { runner, last } = setup();
    const root = write(EXT_PROJECT);
    makeJar(path.join(root, ".karate", "ext"), { "data.json": '{ "ext": true }' });
    await runner.open(root);
    await runner.run("d:");
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  it("puts the jars of the home .karate/ext on the classpath", T, async () => {
    const home = mkdtempSync(path.join(tmpdir(), "kk-home-"));
    temps.push(home);
    makeJar(path.join(home, ".karate", "ext"), { "data.json": '{ "ext": true }' });
    const { runner, last } = setup(undefined, undefined, undefined, undefined, { home });
    await runner.open(write(EXT_PROJECT));
    await runner.run("d:");
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });
});
