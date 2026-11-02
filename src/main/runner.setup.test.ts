import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, T, setup, fixture, temps } from "../../test/runner";
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
    env: "qa",
    dryRun: true,
  }),
};

describe("Runner project setup", () => {
  it("takes paths and configDir from karate-pom.json and names the rest", T, async () => {
    const { runner, last } = setup();
    await runner.open(write(POM_PROJECT));
    expect(leafKeys(last("project").tree!)).toEqual(["s:tests/a/a.feature#1"]);
    expect(last("notice").message).toBe(
      "Grace does not use these karate-pom.json settings: env, dryRun. " +
        "These karate-pom.json paths are not folders or files in the project: missing.",
    );
    expect(last("environment").view).toMatchObject({ env: null, overlays: ["qa"] });
    await runner.run("d:");
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
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
    await runner.run("s:examples/built.feature#1");
    expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
  });

  // Karate names the files relative to src/test/java, which holds karate-config.js (#208)
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
    await runner.run("s:examples/rows.feature#1.2");
    expect(last("runStarted")).toMatchObject({ keys: ["s:examples/rows.feature#1.2"], total: 1 });
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
});
