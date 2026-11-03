import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import {
  failureReason,
  findTool,
  isStale,
  keptJars,
  libraryKey,
  noToolReason,
  parseLibraries,
  readKept,
  TOOL_COMMAND,
  toolEnv,
  writeKept,
} from "./libraries";

const dirs: string[] = [];
/** A temp folder holding `files` (relative path → content). */
function folder(files: Record<string, string> = {}): string {
  const d = mkdtempSync(path.join(tmpdir(), "kk-lib-"));
  dirs.push(d);
  for (const [f, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(d, f)), { recursive: true });
    writeFileSync(path.join(d, f), text);
  }
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

describe("findTool", () => {
  it("takes the project's wrapper before PATH", () => {
    const root = folder({ "mvnw.cmd": "" });
    const bin = folder({ "mvn.cmd": "" });
    expect(findTool(root, "maven", { PATH: bin })).toBe(path.join(root, "mvnw.cmd"));
  });
  it("searches PATH in order, mvn.cmd before mvn.bat", () => {
    const a = folder({ "mvn.bat": "" });
    const b = folder({ "mvn.cmd": "" });
    expect(findTool(folder(), "maven", { PATH: `;${a};${b}` })).toBe(path.join(a, "mvn.bat"));
    const c = folder({ "mvn.bat": "", "mvn.cmd": "" });
    expect(findTool(folder(), "maven", { PATH: c })).toBe(path.join(c, "mvn.cmd"));
  });
  it("finds gradlew.bat, else gradle.bat", () => {
    const root = folder({ "gradlew.bat": "" });
    const bin = folder({ "gradle.bat": "" });
    expect(findTool(root, "gradle", { PATH: bin })).toBe(path.join(root, "gradlew.bat"));
    expect(findTool(folder(), "gradle", { PATH: bin })).toBe(path.join(bin, "gradle.bat"));
  });
  it("reads a quoted PATH entry as cmd.exe does", () => {
    const bin = folder({ "mvn.cmd": "", "java.exe": "" });
    expect(findTool(folder(), "maven", { PATH: `"${bin}"` })).toBe(path.join(bin, "mvn.cmd"));
    const o = { tool: "t", file: "f", initScript: "i", searchPath: `"${bin}"`, jreDir: "jre" };
    expect(toolEnv({}, o).JAVA_HOME).toBeUndefined();
  });
  it("then looks in MAVEN_HOME, then M2_HOME, and in GRADLE_HOME for Gradle", () => {
    const m = folder({ "bin/mvn.cmd": "" });
    const m2 = folder({ "bin/mvn.cmd": "" });
    const g = folder({ "bin/gradle.bat": "" });
    const env = { MAVEN_HOME: m, M2_HOME: m2, GRADLE_HOME: `"${g}"` };
    expect(findTool(folder(), "maven", env)).toBe(path.join(m, "bin", "mvn.cmd"));
    expect(findTool(folder(), "maven", { ...env, MAVEN_HOME: folder() })).toBe(
      path.join(m2, "bin", "mvn.cmd"),
    );
    expect(findTool(folder(), "gradle", env)).toBe(path.join(g, "bin", "gradle.bat"));
    expect(findTool(folder(), "maven", { GRADLE_HOME: g })).toBeNull();
    const bin = folder({ "mvn.cmd": "" });
    expect(findTool(folder(), "maven", { ...env, PATH: bin })).toBe(path.join(bin, "mvn.cmd"));
  });
  it("then takes IntelliJ IDEA's Maven, the newest build first", () => {
    const mvn = "plugins/maven/lib/maven3/bin/mvn.cmd";
    const programFiles = folder({
      [`JetBrains/IntelliJ IDEA 2024.3/${mvn}`]: "",
      "JetBrains/IntelliJ IDEA 2024.3/build.txt": "IU-243.21565.193",
      "JetBrains/Rider/build.txt": "RD-261.1.1",
    });
    // a higher build, whatever the product code before it
    const localAppData = folder({
      [`Programs/IntelliJ IDEA/${mvn}`]: "",
      "Programs/IntelliJ IDEA/build.txt": "IC-243.21565.1000",
      "Programs/Microsoft VS Code/Code.exe": "",
    });
    const env = { ProgramFiles: programFiles, LOCALAPPDATA: localAppData };
    const newest = path.join(localAppData, "Programs", "IntelliJ IDEA", mvn);
    expect(findTool(folder(), "maven", env)).toBe(newest);
    rmSync(path.join(localAppData, "Programs", "IntelliJ IDEA", "build.txt"));
    expect(findTool(folder(), "maven", env)).toBe(
      path.join(programFiles, "JetBrains", "IntelliJ IDEA 2024.3", mvn),
    );
    const m2 = folder({ "bin/mvn.cmd": "" });
    expect(findTool(folder(), "maven", { ...env, M2_HOME: m2 })).toBe(
      path.join(m2, "bin", "mvn.cmd"),
    );
    expect(findTool(folder(), "gradle", env)).toBeNull();
  });
  it("finds nothing", () => {
    expect(findTool(folder(), "maven", {})).toBeNull();
    expect(findTool(folder({ "mvnw.cmd": "" }), "gradle", {})).toBeNull();
  });
  it("names where it looked", () => {
    expect(noToolReason("maven")).toBe(
      "the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA",
    );
    expect(noToolReason("gradle")).toBe(
      "the project has no gradlew.bat, and gradle is not on PATH or in GRADLE_HOME",
    );
  });
});

describe("libraryKey", () => {
  it("changes with the layout's build files only", () => {
    const root = folder({ "pom.xml": "<project/>", "README.md": "a" });
    const key = libraryKey(root, "maven");
    expect(libraryKey(root, "maven")).toBe(key);
    writeFileSync(path.join(root, "README.md"), "b");
    writeFileSync(path.join(root, "build.gradle"), "");
    expect(libraryKey(root, "maven")).toBe(key);
    writeFileSync(path.join(root, "pom.xml"), "<project></project>");
    const edited = libraryKey(root, "maven");
    expect(edited).not.toBe(key);
    mkdirSync(path.join(root, ".mvn"));
    writeFileSync(path.join(root, ".mvn", "maven.config"), "-Pqa");
    expect(libraryKey(root, "maven")).not.toBe(edited);
  });
  it("reads Gradle's version catalog", () => {
    const root = folder({ "build.gradle": "", "gradle/libs.versions.toml": "a" });
    const key = libraryKey(root, "gradle");
    writeFileSync(path.join(root, "gradle", "libs.versions.toml"), "b");
    expect(libraryKey(root, "gradle")).not.toBe(key);
  });
});

describe("the kept list", () => {
  it("round-trips", () => {
    const dir = path.join(folder(), "projects", "abc");
    writeKept(dir, { tool: "maven", key: "k", jars: ["a.jar"] });
    expect(readKept(dir)).toEqual({ tool: "maven", key: "k", jars: ["a.jar"] });
  });
  it("reads anything else as none", () => {
    const dir = folder();
    expect(readKept(dir)).toBeNull();
    for (const text of ["nope", "{}", JSON.stringify({ tool: "ant", key: "k", jars: [] })]) {
      writeFileSync(path.join(dir, "libraries.json"), text);
      expect(readKept(dir)).toBeNull();
    }
  });
  it("is stale when missing, of another tool or key, or a jar is gone", () => {
    const jar = path.join(folder({ "a.jar": "" }), "a.jar");
    const kept = { tool: "maven" as const, key: "k", jars: [jar] };
    expect(isStale(null, "maven", "k")).toBe(true);
    expect(isStale(kept, "gradle", "k")).toBe(true);
    expect(isStale(kept, "maven", "other")).toBe(true);
    expect(isStale({ ...kept, jars: [jar, `${jar}.gone`] }, "maven", "k")).toBe(true);
    expect(isStale(kept, "maven", "k")).toBe(false);
  });
  it("gives the jars only to the same tool", () => {
    const dir = folder();
    writeKept(dir, { tool: "maven", key: "k", jars: ["a.jar"] });
    expect(keptJars(dir, "maven")).toEqual(["a.jar"]);
    expect(keptJars(dir, "gradle")).toEqual([]);
    expect(keptJars(dir, "standalone")).toEqual([]);
    expect(keptJars(folder(), "maven")).toEqual([]);
  });
});

describe("the command", () => {
  it("passes every path through a quoted variable", () => {
    expect(TOOL_COMMAND.maven).toBe(
      '"%GRACE_TOOL%" -q -B dependency:build-classpath "-Dmdep.outputFile=%GRACE_CLASSPATH_FILE%"',
    );
    expect(TOOL_COMMAND.gradle).toBe(
      '"%GRACE_TOOL%" -q --no-daemon --console=plain --no-configuration-cache --init-script "%GRACE_INIT_SCRIPT%" graceClasspath',
    );
  });
  const o = { tool: "t.cmd", file: "f.txt", initScript: "i.gradle", searchPath: "", jreDir: "jre" };
  it("keeps the person's environment and adds Grace's variables", () => {
    const env = toolEnv({ MAVEN_OPTS: "-Xmx1g", JAVA_HOME: "jdk" }, o);
    expect(env).toEqual({
      MAVEN_OPTS: "-Xmx1g",
      JAVA_HOME: "jdk",
      GRACE_TOOL: "t.cmd",
      GRACE_CLASSPATH_FILE: "f.txt",
      GRACE_INIT_SCRIPT: "i.gradle",
    });
  });
  it("lends the bundled Java only when there is no other", () => {
    expect(toolEnv({}, o).JAVA_HOME).toBe("jre");
    expect(toolEnv({ JAVA_HOME: "" }, o).JAVA_HOME).toBe("jre");
    const bin = folder({ "java.exe": "" });
    expect(toolEnv({}, { ...o, searchPath: bin }).JAVA_HOME).toBeUndefined();
  });
  it("lends nothing without a Java to lend (Gradle needs a JDK)", () => {
    expect(toolEnv({}, { ...o, jreDir: null })).not.toHaveProperty("JAVA_HOME");
  });
});

describe("parseLibraries", () => {
  it("keeps existing entries in order, once, less the classpath", () => {
    const d = folder({ "a.jar": "", "b.jar": "" });
    const [a, b] = [path.join(d, "a.jar"), path.join(d, "b.jar")];
    expect(parseLibraries(`${a};${b}\r\n`, [])).toEqual([a, b]);
    expect(parseLibraries(`${b};${a}.gone;${a};${b}`, [])).toEqual([b, a]);
    expect(parseLibraries(`${a};${d.toUpperCase()}`, [d])).toEqual([a]);
    expect(parseLibraries("", [])).toEqual([]);
  });
});

describe("failureReason", () => {
  it("takes Maven's first error", () => {
    expect(
      failureReason(["[INFO] x", "[ERROR] Could not resolve dependencies", "[ERROR] other"]),
    ).toBe("Could not resolve dependencies");
  });
  // recorded from Maven 3.9.16 on three broken poms, paths shortened
  it("takes the cause under Maven's POM problems line", () => {
    expect(
      failureReason([
        "[ERROR] [ERROR] Some problems were encountered while processing the POMs:",
        "[FATAL] Non-readable POM C:\\p\\pom.xml: no more data available - expected end tags </modelVersion></project> to close start tag <modelVersion> from line 1 and start tag <project> from line 1, parser stopped on TEXT seen ...<modelVersion>4.0.0</modelVersion\\r\\n... @2:1 @ ",
        " @ ",
        "[ERROR] The build could not read 1 project -> [Help 1]",
      ]),
    ).toMatch(/^Non-readable POM C:\\p\\pom\.xml: no more data available/);
    expect(
      failureReason([
        "[ERROR] [ERROR] Some problems were encountered while processing the POMs:",
        "[ERROR] 'dependencies.dependency.version' for org.xerial:sqlite-jdbc:jar is missing. @ line 1, column 136",
        " @ ",
        "[ERROR] The build could not read 1 project -> [Help 1]",
      ]),
    ).toBe(
      "'dependencies.dependency.version' for org.xerial:sqlite-jdbc:jar is missing. @ line 1, column 136",
    );
    expect(
      failureReason([
        "[ERROR] Failed to execute goal on project y: Could not resolve dependencies for project x:y:jar:1",
        "[ERROR] dependency: org.acme:nope:jar:9.9 (compile)",
        "[ERROR] -> [Help 1]",
      ]),
    ).toBe(
      "Failed to execute goal on project y: Could not resolve dependencies for project x:y:jar:1",
    );
  });
  it("takes what Gradle says went wrong", () => {
    expect(
      failureReason([
        "FAILURE: Build failed with an exception.",
        "",
        "* What went wrong:",
        "Task 'graceClasspath' not found in root project 'x'.",
      ]),
    ).toBe("Task 'graceClasspath' not found in root project 'x'");
  });
  it("takes the deepest cause Gradle gives", () => {
    expect(
      failureReason([
        "* What went wrong:",
        "Execution failed for task ':graceClasspath' (registered in initialization script 'x').",
        "> Could not resolve all dependencies for configuration ':testRuntimeClasspath'.",
        "   > Could not find org.acme:client:1.0.",
        "",
        "* Try:",
        "> Run with --stacktrace option to get the stack trace.",
      ]),
    ).toBe("Could not find org.acme:client:1.0");
  });
  it("takes a launcher's ERROR line", () => {
    expect(
      failureReason([
        "",
        "ERROR: JAVA_HOME is not set and no 'java' command could be found in your PATH.",
        "",
        "Please set the JAVA_HOME variable in your environment to match the",
        "location of your Java installation.",
      ]),
    ).toBe("JAVA_HOME is not set and no 'java' command could be found in your PATH");
  });
  it("else the last line, cut to 300 characters", () => {
    expect(failureReason(["a", "b", ""])).toBe("b");
    expect(failureReason(["x".repeat(400)])).toHaveLength(300);
    expect(failureReason([])).toBe("it stopped with no message");
  });
});
