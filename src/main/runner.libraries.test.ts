import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  calls,
  cleanUp,
  makeJar,
  projectDir,
  setup,
  stubTool,
  T,
  temps,
  type Of,
} from "../../test/runner";
import type { AppEvent } from "../shared/events";
import { INIT_SCRIPT, readKept } from "./karate/libraries";

afterEach(cleanUp);

const FEATURE = "Feature: t\nScenario: lib\n  * match read('classpath:data.json').lib == true\n";

/** A Maven (or Gradle) project whose one scenario passes only with the listed jar on the classpath. */
function project(build: "pom.xml" | "build.gradle" = "pom.xml", parent = tmpdir()): string {
  mkdirSync(parent, { recursive: true });
  const root = mkdtempSync(path.join(parent, "kk-libs-"));
  temps.push(root);
  writeFileSync(path.join(root, build), build === "pom.xml" ? "<project/>" : "");
  mkdirSync(path.join(root, "src", "test", "java"), { recursive: true });
  writeFileSync(path.join(root, "src", "test", "java", "t.feature"), FEATURE);
  return root;
}

/** A jar holding data.json, in a plain temp folder: the stub's echo is not quoted. */
function jar(): string {
  const dir = mkdtempSync(path.join(tmpdir(), "kk-libjar-"));
  temps.push(dir);
  return makeJar(dir, { "data.json": '{ "lib": true }' });
}

const lists = (j: string) => `>"%GRACE_CLASSPATH_FILE%" echo ${j}`;
const passed = (last: <K extends AppEvent["type"]>(type: K) => Of<K>) =>
  expect(last("runFinished").summary).toMatchObject({ scenarioCount: 1, scenariosPassed: 1 });
const touch = (root: string, build = "pom.xml") =>
  writeFileSync(path.join(root, build), `${readFileSync(path.join(root, build), "utf8")} `);

describe("Runner project libraries", () => {
  it("runs mvnw.cmd and puts the listed jar on the classpath", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = project();
    const log = stubTool(root, "mvnw.cmd", lists(jar()));
    await runner.open(root);
    const file = path.join(projectDir(dataDir, root), "libraries.txt");
    expect(calls(log)).toHaveLength(1);
    expect(calls(log)[0]).toContain("dependency:build-classpath");
    expect(calls(log)[0]).toContain(`-Dmdep.outputFile=${file}`);
    await runner.run("d:");
    passed(last);
  });

  it("runs gradlew.bat without a daemon", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = project("build.gradle");
    const log = stubTool(root, "gradlew.bat", lists(jar()));
    await runner.open(root);
    const script = path.join(projectDir(dataDir, root), "libraries.gradle");
    expect(calls(log)).toHaveLength(1);
    for (const arg of ["--no-daemon", "--init-script", script, "graceClasspath"])
      expect(calls(log)[0]).toContain(arg);
    expect(readFileSync(script, "utf8")).toBe(INIT_SCRIPT);
    await runner.run("d:");
    passed(last);
  });

  it("keeps the list until a build file changes", T, async () => {
    const { runner } = setup();
    const root = project();
    const log = stubTool(root, "mvnw.cmd", lists(jar()));
    await runner.open(root);
    await runner.refresh();
    expect(calls(log)).toHaveLength(1);
    touch(root);
    await runner.refresh();
    expect(calls(log)).toHaveLength(2);
  });

  it("never lists on an environment switch or a run", T, async () => {
    const { runner, last } = setup();
    const root = project();
    writeFileSync(
      path.join(root, "src", "test", "java", "karate-config-qa.js"),
      "function fn() { return {}; }",
    );
    const log = stubTool(root, "mvnw.cmd", lists(jar()));
    await runner.open(root);
    touch(root);
    await runner.setEnv("qa");
    await runner.run("d:");
    expect(calls(log)).toHaveLength(1);
    passed(last);
  });

  it("says when there is no tool", T, async () => {
    const { runner, last } = setup();
    await runner.open(project());
    expect(last("notice").message).toBe(
      "Grace could not list the project's libraries with Maven: the project has no mvnw.cmd and mvn is not on PATH. Tests that use them will fail.",
    );
    expect(last("project").tree).not.toBeNull();
  });

  it("keeps the last list when the tool fails", T, async () => {
    const { runner, last } = setup();
    const root = project();
    stubTool(root, "mvnw.cmd", lists(jar()));
    await runner.open(root);
    stubTool(root, "mvnw.cmd", "echo [ERROR] Could not resolve dependencies\r\nexit /b 1");
    touch(root);
    await runner.refresh();
    expect(last("notice").message).toBe(
      "Grace could not list the project's libraries with Maven: Could not resolve dependencies. Grace uses the libraries it listed before.",
    );
    await runner.run("d:");
    passed(last);
  });

  it("ends the notice with one full stop", T, async () => {
    const { runner, last } = setup();
    const root = project();
    stubTool(root, "mvnw.cmd", "echo [ERROR] Could not resolve dependencies.\r\nexit /b 1");
    await runner.open(root);
    expect(last("notice").message).toBe(
      "Grace could not list the project's libraries with Maven: Could not resolve dependencies. Tests that use them will fail.",
    );
  });

  it("joins the open notice", T, async () => {
    const { runner, last } = setup();
    const root = project();
    writeFileSync(path.join(root, "karate-pom.json"), JSON.stringify({ env: "qa" }));
    await runner.open(root);
    expect(last("notice").message).toBe(
      "Grace does not use these karate-pom.json settings: env. Grace could not list the project's libraries with Maven: the project has no mvnw.cmd and mvn is not on PATH. Tests that use them will fail.",
    );
  });

  it("runs the tool in a folder whose path has spaces and &", T, async () => {
    const { runner, last } = setup();
    const parent = path.join(mkdtempSync(path.join(tmpdir(), "kk-odd-")), "a b&c");
    temps.push(path.dirname(parent));
    const root = project("pom.xml", parent);
    const log = stubTool(root, "mvnw.cmd", lists(jar()));
    await runner.open(root);
    expect(calls(log)).toHaveLength(1);
    await runner.run("d:");
    passed(last);
  });

  it("says it is listing, also in a replay", T, async () => {
    const { runner, all, onEvent } = setup();
    const root = project();
    stubTool(root, "mvnw.cmd", `ping -n 2 127.0.0.1 >nul\r\n${lists(jar())}`);
    let replayed: AppEvent[] = [];
    onEvent((e) => {
      if (e.type === "listing" && e.tool) replayed = runner.replay();
    });
    await runner.open(root);
    expect(replayed).toContainEqual({ type: "listing", tool: "Maven" });
    expect(all("listing")).toEqual([
      { type: "listing", tool: "Maven" },
      { type: "listing", tool: null },
    ]);
    expect(runner.replay()).not.toContainEqual(expect.objectContaining({ type: "listing" }));
  });

  it("stops the tool on Cancel", T, async () => {
    const { runner, last, onEvent, dataDir } = setup();
    const root = project();
    const log = stubTool(
      root,
      "mvnw.cmd",
      `ping -n 4 127.0.0.1 >nul\r\necho late>>"%~dp0calls.log"\r\n${lists(jar())}`,
    );
    onEvent((e) => {
      if (e.type === "listing" && e.tool) void runner.cancel();
    });
    expect(await runner.open(root)).toBe(false);
    expect(last("notice").message).toBe("Cancelled");
    await new Promise((r) => setTimeout(r, 5000));
    expect(calls(log)).not.toContain("late");
    expect(readKept(projectDir(dataDir, root))).toBeNull();
  });

  it("runs no tool for a standalone project", T, async () => {
    const { runner } = setup();
    const root = mkdtempSync(path.join(tmpdir(), "kk-libs-"));
    temps.push(root);
    writeFileSync(path.join(root, "t.feature"), "Feature: t\nScenario: s\n  * print 1\n");
    const log = stubTool(root, "mvnw.cmd", "");
    await runner.open(root);
    expect(calls(log)).toEqual([]);
  });
});
