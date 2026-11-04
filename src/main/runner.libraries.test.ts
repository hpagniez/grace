import { mkdirSync, mkdtempSync, readFileSync, rmSync, utimesSync, writeFileSync } from "node:fs";
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
  vendor,
  type Of,
} from "../../test/runner";
import type { AppEvent } from "../shared/events";
import { readBuilt } from "./karate/compiled";
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

  it("runs the Maven of MAVEN_HOME when the project has no wrapper", T, async () => {
    const home = mkdtempSync(path.join(tmpdir(), "kk-maven-"));
    temps.push(home);
    mkdirSync(path.join(home, "bin"));
    const log = stubTool(path.join(home, "bin"), "mvn.cmd", lists(jar()));
    const lookupEnv = { MAVEN_HOME: home };
    const { runner, last } = setup(undefined, undefined, undefined, undefined, { lookupEnv });
    await runner.open(project());
    expect(calls(log)).toHaveLength(1);
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
      "Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project.",
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

  // R5.1: the project's folders, the home .karate/ext, the project's .karate/ext, then the build's libraries
  it("puts the classpath in its order", T, async () => {
    const home = mkdtempSync(path.join(tmpdir(), "kk-home-"));
    temps.push(home);
    const { runner, last } = setup(undefined, undefined, undefined, undefined, { home });
    const root = project();
    const from = (where: string) => ({ "data.json": JSON.stringify({ from: where }) });
    const own = path.join(root, "src", "test", "java", "data.json");
    writeFileSync(own, JSON.stringify({ from: "project" }));
    const homeJar = makeJar(path.join(home, ".karate", "ext"), from("home"));
    const extJar = makeJar(path.join(root, ".karate", "ext"), from("ext"));
    const built = mkdtempSync(path.join(tmpdir(), "kk-libjar-"));
    temps.push(built);
    stubTool(root, "mvnw.cmd", lists(makeJar(built, from("build"))));
    await runner.open(root);
    const expects = (where: string) =>
      writeFileSync(
        path.join(root, "src", "test", "java", "t.feature"),
        `Feature: t\nScenario: lib\n  * match read('classpath:data.json').from == '${where}'\n`,
      );
    for (const [where, next] of [
      ["project", own],
      ["home", homeJar],
      ["ext", extJar],
      ["build", null],
    ] as const) {
      expects(where);
      await runner.run("d:");
      passed(last);
      if (next) rmSync(next);
    }
  });

  describe("the project's Java code", () => {
    const NOT_BUILT =
      "The project's Java code in src/test/java is not built: build it (mvn test-compile) before running.";
    const CHANGED =
      "The project's Java code in src/test/java changed since its last build: build it again (mvn test-compile) before running.";
    /** A Maven project with a helper's source, its libraries listed, and none of its code built. */
    function withHelper() {
      const root = project();
      stubTool(root, "mvnw.cmd", lists(jar()));
      writeFileSync(path.join(root, "src", "test", "java", "Helper.java"), "class Helper {}");
      return root;
    }
    const at = (file: string, seconds: number) => {
      mkdirSync(path.dirname(file), { recursive: true });
      writeFileSync(file, "", { flag: "a" });
      utimesSync(file, seconds, seconds);
    };

    it("says on open when it is not built", T, async () => {
      const { runner, last } = setup();
      await runner.open(withHelper());
      expect(last("notice").message).toBe(NOT_BUILT);
    });

    it("comes after the libraries' notice", T, async () => {
      const { runner, last } = setup();
      const root = project();
      writeFileSync(path.join(root, "src", "test", "java", "Helper.java"), "class Helper {}");
      await runner.open(root);
      expect(last("notice").message).toBe(
        `Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project. ${NOT_BUILT}`,
      );
    });

    it("says nothing once built, and says on Refresh when it changed since", T, async () => {
      const { runner, last, all } = setup();
      const root = withHelper();
      const source = path.join(root, "src", "test", "java", "Helper.java");
      at(source, 1_000_000_000);
      at(path.join(root, "target", "test-classes", "Helper.class"), 1_000_000_100);
      await runner.open(root);
      await runner.refresh();
      expect(all("notice")).toEqual([]);
      at(source, 1_000_000_200);
      await runner.refresh();
      expect(last("notice").message).toBe(CHANGED);
    });

    describe("Build", () => {
      /** Writes the helper's class, as test-compile would, and logs its JAVA_HOME. */
      const BUILDS = [
        String.raw`mkdir "%~dp0target\test-classes" 2>nul`,
        String.raw`type nul>"%~dp0target\test-classes\Helper.class"`,
        String.raw`echo JAVA_HOME=%JAVA_HOME%>>"%~dp0calls.log"`,
      ].join("\r\n");
      const FAILS = "echo [ERROR] COMPILATION ERROR\r\nexit /b 1";
      const NO_TOOL =
        "Grace could not build the project's Java code with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA.";
      const builds = (log: string) => calls(log).filter((l) => l.includes("test-compile"));
      /** A JDK's folder: bin/javac.exe. */
      function jdk(): string {
        const dir = mkdtempSync(path.join(tmpdir(), "kk-jdk-"));
        temps.push(dir);
        mkdirSync(path.join(dir, "bin"));
        writeFileSync(path.join(dir, "bin", "javac.exe"), "");
        return dir;
      }

      it("offers Build with the not-built notice", T, async () => {
        const { runner, last } = setup();
        await runner.open(withHelper());
        expect(last("notice")).toEqual({ type: "notice", message: NOT_BUILT, build: true });
      });

      it("offers no Build without a tool", T, async () => {
        const { runner, last } = setup();
        const root = project();
        writeFileSync(path.join(root, "src", "test", "java", "Helper.java"), "class Helper {}");
        await runner.open(root);
        expect(last("notice").message).toContain(NOT_BUILT);
        expect(last("notice")).not.toHaveProperty("build");
      });

      it("offers Build in a joined notice", T, async () => {
        const { runner, last } = setup();
        const root = withHelper();
        writeFileSync(path.join(root, "karate-pom.json"), JSON.stringify({ dryRun: true }));
        await runner.open(root);
        expect(last("notice")).toEqual({
          type: "notice",
          message: `Grace does not use these karate-pom.json settings: dryRun. ${NOT_BUILT}`,
          build: true,
        });
      });

      it("offers Build after the libraries' failure on Refresh", T, async () => {
        const { runner, last } = setup();
        const root = withHelper();
        await runner.open(root);
        stubTool(root, "mvnw.cmd", "echo [ERROR] Could not resolve dependencies\r\nexit /b 1");
        touch(root);
        await runner.refresh();
        expect(last("notice")).toEqual({
          type: "notice",
          message: `Grace could not list the project's libraries with Maven: Could not resolve dependencies. Grace uses the libraries it listed before. ${NOT_BUILT}`,
          build: true,
        });
      });

      it("builds with test-compile, then refreshes", T, async () => {
        const { runner, events, dataDir } = setup();
        const root = withHelper();
        await runner.open(root);
        const log = stubTool(root, "mvnw.cmd", BUILDS);
        const before = events.length;
        await runner.build();
        expect(builds(log)).toEqual(["-B -q test-compile"]);
        expect(calls(log).some((l) => l.includes(vendor.jreDir))).toBe(false);
        const after = events.slice(before);
        expect(after.some((e) => e.type === "project")).toBe(true);
        expect(after.filter((e) => e.type === "notice")).toEqual([]);
        expect(readBuilt(projectDir(dataDir, root))).toEqual(expect.any(Number));
      });

      it("builds in a folder whose path has spaces and &", T, async () => {
        const { runner } = setup();
        const parent = path.join(mkdtempSync(path.join(tmpdir(), "kk-odd-")), "a b&c");
        temps.push(path.dirname(parent));
        const root = project("pom.xml", parent);
        stubTool(root, "mvnw.cmd", lists(jar()));
        writeFileSync(path.join(root, "src", "test", "java", "Helper.java"), "class Helper {}");
        await runner.open(root);
        const log = stubTool(root, "mvnw.cmd", BUILDS);
        await runner.build();
        expect(builds(log)).toEqual(["-B -q test-compile"]);
      });

      it("says the build's first error and keeps Build", T, async () => {
        const { runner, events, last } = setup();
        const root = withHelper();
        await runner.open(root);
        stubTool(root, "mvnw.cmd", FAILS);
        const before = events.length;
        await runner.build();
        expect(last("notice")).toEqual({
          type: "notice",
          message:
            "Grace could not build the project's Java code with Maven: COMPILATION ERROR. Building needs a JDK: set JAVA_HOME to one.",
          build: true,
        });
        expect(events.slice(before).some((e) => e.type === "project")).toBe(false);
      });

      it("says nothing of a JDK when JAVA_HOME has javac.exe", T, async () => {
        const lookupEnv = { JAVA_HOME: jdk() };
        const { runner, last } = setup(undefined, undefined, undefined, undefined, { lookupEnv });
        const root = withHelper();
        await runner.open(root);
        stubTool(root, "mvnw.cmd", FAILS);
        await runner.build();
        expect(last("notice").message).toBe(
          "Grace could not build the project's Java code with Maven: COMPILATION ERROR.",
        );
      });

      it("stops the build on Cancel", T, async () => {
        const { runner, last, onEvent, dataDir } = setup();
        const root = withHelper();
        await runner.open(root);
        const log = stubTool(
          root,
          "mvnw.cmd",
          'ping -n 4 127.0.0.1 >nul\r\necho late>>"%~dp0calls.log"',
        );
        onEvent((e) => {
          if (e.type === "listing" && e.build) void runner.cancel();
        });
        await runner.build();
        expect(last("notice")).toEqual({
          type: "notice",
          message: "Build cancelled.",
          build: true,
        });
        await new Promise((r) => setTimeout(r, 5000));
        expect(calls(log)).not.toContain("late");
        expect(readBuilt(projectDir(dataDir, root))).toBeNull();
      });

      it("says it is building, also in a replay", T, async () => {
        const { runner, events, onEvent } = setup();
        const root = withHelper();
        await runner.open(root);
        stubTool(root, "mvnw.cmd", "ping -n 2 127.0.0.1 >nul");
        let replayed: AppEvent[] = [];
        onEvent((e) => {
          if (e.type === "listing" && e.build) replayed = runner.replay();
        });
        const before = events.length;
        await runner.build();
        expect(replayed).toContainEqual({ type: "listing", tool: "Maven", build: true });
        expect(events.slice(before).filter((e) => e.type === "listing")).toEqual([
          { type: "listing", tool: "Maven", build: true },
          { type: "listing", tool: null },
        ]);
      });

      it("refuses a second build while one runs", T, async () => {
        const { runner, last } = setup();
        const root = withHelper();
        await runner.open(root);
        const log = stubTool(root, "mvnw.cmd", "ping -n 2 127.0.0.1 >nul\r\nexit /b 1");
        const first = runner.build();
        await runner.build();
        expect(last("notice").message).toBe("A run is already in progress");
        await first;
        expect(builds(log)).toHaveLength(1);
      });

      it("says when the tool is gone at the click", T, async () => {
        const { runner, last } = setup();
        const root = withHelper();
        await runner.open(root);
        rmSync(path.join(root, "mvnw.cmd"));
        await runner.build();
        expect(last("notice")).toEqual({ type: "notice", message: NO_TOOL });
      });

      it("a Gradle build that rewrites no class still counts", T, async () => {
        const GRADLE_CHANGED =
          "The project's Java code in src/test/java changed since its last build: build it again (gradle testClasses) before running.";
        const { runner, events, last } = setup();
        const root = project("build.gradle");
        stubTool(root, "gradlew.bat", lists(jar()));
        const source = path.join(root, "src", "test", "java", "Helper.java");
        at(source, 1_000_000_200);
        at(path.join(root, "build", "classes", "java", "test", "Helper.class"), 1_000_000_100);
        await runner.open(root);
        expect(last("notice")).toEqual({ type: "notice", message: GRADLE_CHANGED, build: true });
        const log = stubTool(root, "gradlew.bat", "rem");
        const before = events.length;
        await runner.build();
        expect(calls(log).at(-1)).toBe("-q --no-daemon --console=plain testClasses");
        expect(events.slice(before).filter((e) => e.type === "notice")).toEqual([]);
        at(source, Date.now() / 1000 + 10);
        await runner.refresh();
        expect(last("notice").message).toBe(GRADLE_CHANGED);
      });

      it("runs nothing with no project or for a standalone project", T, async () => {
        const { runner, all } = setup();
        await runner.build();
        expect(all("busy")).toEqual([]);
        const root = mkdtempSync(path.join(tmpdir(), "kk-libs-"));
        temps.push(root);
        writeFileSync(path.join(root, "t.feature"), "Feature: t\nScenario: s\n  * print 1\n");
        const log = stubTool(root, "mvnw.cmd", "");
        await runner.open(root);
        const busy = all("busy").length;
        await runner.build();
        expect(calls(log)).toEqual([]);
        expect(all("busy")).toHaveLength(busy);
      });
    });
  });

  it("joins the open notice", T, async () => {
    const { runner, last } = setup();
    const root = project();
    writeFileSync(path.join(root, "karate-pom.json"), JSON.stringify({ dryRun: true }));
    await runner.open(root);
    expect(last("notice").message).toBe(
      "Grace does not use these karate-pom.json settings: dryRun. Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project.",
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
