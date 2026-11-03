import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import type { TestNode } from "../../shared/model";
import { NO_FILTER } from "../../shared/tags";
import {
  buildArgs,
  childEnv,
  detectLayout,
  extensionJars,
  lineTargets,
  locate,
  OUT_OF_MEMORY,
  outOfMemory,
  selectionTargets,
  tagArgs,
} from "./command";

const dirs: string[] = [];
function project(files: string[], folders: string[] = []): string {
  const d = mkdtempSync(path.join(tmpdir(), "kk-cmd-"));
  dirs.push(d);
  for (const f of files) writeFileSync(path.join(d, f), "");
  for (const f of folders) mkdirSync(path.join(d, f), { recursive: true });
  return d;
}
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

const pom = (root: string, json: object | string) =>
  writeFileSync(
    path.join(root, "karate-pom.json"),
    typeof json === "string" ? json : JSON.stringify(json),
  );

describe("extensionJars", () => {
  const ext = (root: string) => path.join(root, ".karate", "ext");
  it("lists the home folder, then the project's, as wildcards", () => {
    const home = project([], [".karate/ext"]);
    const root = project([], [".karate/ext"]);
    expect(extensionJars(home, root)).toEqual([
      path.join(ext(home), "*"),
      path.join(ext(root), "*"),
    ]);
  });
  it("skips a folder that does not exist", () => {
    const home = project([]);
    const root = project([], [".karate/ext"]);
    expect(extensionJars(home, root)).toEqual([path.join(ext(root), "*")]);
    expect(extensionJars(home, project([]))).toEqual([]);
  });
});

describe("detectLayout", () => {
  const at = (root: string, ...dirs: string[]) => dirs.map((d) => path.join(root, d));
  it("maven with both test folders", () => {
    const root = project(["pom.xml"], ["src/test/java", "src/test/resources"]);
    const both = at(root, "src/test/java", "src/test/resources");
    expect(detectLayout(root)).toEqual({
      layout: "maven",
      classpath: both,
      featureRoots: both,
      configDir: null,
      configDirs: [root, ...both],
      pomNotice: null,
      bases: [root],
    });
  });
  it("gradle", () => {
    const root = project(["build.gradle.kts"], ["src/test/resources"]);
    expect(detectLayout(root).layout).toBe("gradle");
  });
  it("only existing folders", () => {
    const root = project(["pom.xml"], ["src/test/java"]);
    const only = at(root, "src/test/java");
    expect(detectLayout(root)).toMatchObject({ classpath: only, featureRoots: only });
  });
  it("standalone", () => {
    const root = project([]);
    expect(detectLayout(root)).toEqual({
      layout: "standalone",
      classpath: [root],
      featureRoots: [root],
      configDir: null,
      configDirs: [root],
      pomNotice: null,
      bases: [root],
    });
  });

  // Karate's names of the files are relative to its root when they are under it (#208, #223)
  describe("bases", () => {
    it("Karate's root first: the classpath folder of the first karate-config.js", () => {
      const root = project(
        ["pom.xml"],
        ["src/test/java", "src/test/resources", "target/test-classes"],
      );
      for (const d of ["src/test/resources", "target/test-classes"])
        writeFileSync(path.join(root, d, "karate-config.js"), "");
      expect(detectLayout(root).bases).toEqual([path.join(root, "src/test/resources"), root]);
    });
    it("a configDir off the classpath, as a folder, a file: path or a .js file", () => {
      const root = project([]);
      for (const configDir of ["conf", "file:conf", "conf/my-config.js"]) {
        pom(root, { configDir });
        expect(detectLayout(root).bases).toEqual([path.join(root, "conf"), root]);
      }
    });
    it("the classpath folder of a classpath: configDir", () => {
      const root = project(["pom.xml"], ["src/test/java/conf", "src/test/resources/conf"]);
      writeFileSync(path.join(root, "src/test/resources/conf/karate-config.js"), "");
      pom(root, { configDir: "classpath:conf" });
      expect(detectLayout(root).bases).toEqual([path.join(root, "src/test/resources/conf"), root]);
      writeFileSync(path.join(root, "src/test/java/conf/my.js"), "");
      pom(root, { configDir: "classpath:/conf/my.js" });
      expect(detectLayout(root).bases).toEqual([path.join(root, "src/test/java/conf"), root]);
    });
    it("the project when no classpath folder holds the config", () => {
      const root = project(["pom.xml", "karate-config.js"], ["src/test/java"]);
      pom(root, { configDir: "classpath:conf" });
      expect(detectLayout(root).bases).toEqual([root]);
    });
  });

  describe("compiled output", () => {
    it("maven: after the source folders, never a feature root or a config folder", () => {
      const root = project(
        ["pom.xml"],
        ["src/test/java", "src/test/resources", "target/test-classes", "target/classes"],
      );
      const source = at(root, "src/test/java", "src/test/resources");
      expect(detectLayout(root)).toMatchObject({
        classpath: [...source, ...at(root, "target/test-classes", "target/classes")],
        featureRoots: source,
        configDirs: [root, ...source],
      });
    });
    it("gradle: in its order", () => {
      const root = project(
        ["build.gradle"],
        [
          "src/test/java",
          "build/resources/main",
          "build/resources/test",
          "build/classes/java/test",
        ],
      );
      expect(detectLayout(root).classpath).toEqual(
        at(
          root,
          "src/test/java",
          "build/classes/java/test",
          "build/resources/test",
          "build/resources/main",
        ),
      );
    });
    it("standalone: unchanged", () => {
      const root = project([], ["target/test-classes"]);
      expect(detectLayout(root).classpath).toEqual([root]);
    });
  });

  describe("karate-pom.json", () => {
    it("keeps the paths inside the project and names the others", () => {
      const root = project([], ["tests/a"]);
      pom(root, { paths: ["tests/a", "missing", "..", "classpath:x", "."] });
      expect(detectLayout(root)).toMatchObject({
        featureRoots: [path.join(root, "tests/a"), root],
        pomNotice:
          "These karate-pom.json paths are not folders or files in the project: missing, .., classpath:x.",
      });
    });
    it("keeps a folder whose name starts with ..", () => {
      const root = project([], ["..data"]);
      pom(root, { paths: ["..data"] });
      expect(detectLayout(root)).toMatchObject({
        featureRoots: [path.join(root, "..data")],
        pomNotice: null,
      });
    });
    it("keeps the default feature roots without paths", () => {
      const root = project(["pom.xml"], ["src/test/java"]);
      pom(root, { paths: [] });
      expect(detectLayout(root)).toMatchObject({
        featureRoots: at(root, "src/test/java"),
        pomNotice: null,
      });
    });
    it("passes configDir as written and reads environments there and in the project", () => {
      const root = project([]);
      pom(root, { configDir: "conf", env: "qa" });
      expect(detectLayout(root)).toMatchObject({
        configDir: "conf",
        configDirs: [path.join(root, "conf"), root],
        pomNotice: "Grace does not use these karate-pom.json settings: env.",
      });
    });
    it("resolves a classpath: configDir under each source folder", () => {
      const root = project(["pom.xml"], ["src/test/java", "src/test/resources", "target/classes"]);
      pom(root, { configDir: "classpath:conf" });
      expect(detectLayout(root).configDirs).toEqual([
        ...at(root, "src/test/java/conf", "src/test/resources/conf"),
        root,
      ]);
    });
    it("says both, settings first", () => {
      const root = project([]);
      pom(root, { threads: 3, paths: ["gone"], tags: ["@a"] });
      expect(detectLayout(root).pomNotice).toBe(
        "Grace does not use these karate-pom.json settings: threads, tags. " +
          "These karate-pom.json paths are not folders or files in the project: gone.",
      );
    });
    it("uses nothing from a file it cannot read", () => {
      const root = project(["pom.xml"], ["src/test/java"]);
      pom(root, '{"paths":"x","configDir":"conf"}');
      expect(detectLayout(root)).toMatchObject({
        featureRoots: at(root, "src/test/java"),
        configDir: null,
        configDirs: [root, ...at(root, "src/test/java")],
        pomNotice:
          "karate-pom.json could not be read: paths is not a list of strings. Grace uses the default folders.",
      });
    });
  });
});

const node = (kind: TestNode["kind"], file: string, line: number | null): TestNode => ({
  key: "x",
  kind,
  name: "n",
  file,
  line,
  refId: null,
  tags: [],
  children: [],
});

describe("locate", () => {
  it("takes the first base that holds the file, else the first base", () => {
    const root = project(["pom.xml"], ["src/test/java/examples", "tests"]);
    const java = path.join(root, "src/test/java");
    expect(locate([root, java], "examples")).toBe(path.join(java, "examples"));
    expect(locate([root, java], "tests")).toBe(path.join(root, "tests"));
    expect(locate([root, java], "gone/a.feature")).toBe(path.join(root, "gone", "a.feature"));
  });
});

describe("selectionTargets", () => {
  const root = path.join("D:", "Mes tests", "équipe");
  const roots = [path.join(root, "src", "test", "java")];
  const abs = path.join(root, "tests", "a.feature");
  it("maps each node kind", () => {
    expect(selectionTargets(node("folder", "", null), [root], roots, NO_FILTER)).toEqual(roots);
    expect(selectionTargets(node("folder", "tests", null), [root], roots, NO_FILTER)).toEqual([
      path.join(root, "tests"),
    ]);
    expect(
      selectionTargets(node("feature", "tests/a.feature", null), [root], roots, NO_FILTER),
    ).toEqual([abs]);
    for (const kind of ["scenario", "outline", "row"] as const)
      expect(selectionTargets(node(kind, "tests/a.feature", 12), [root], roots, NO_FILTER)).toEqual(
        [`${abs}:12`],
      );
  });

  describe("outline targets under a filter", () => {
    const row = (line: number | null, tag: string): TestNode => ({
      ...node("row", "tests/a.feature", line),
      tags: [tag],
    });
    const outline = (r9Line: number | null = 9): TestNode => ({
      ...node("outline", "tests/a.feature", 5),
      children: [row(r9Line, "fast"), row(10, "fast"), row(11, "slow")],
    });
    const f = (...include: string[]) => ({ include, exclude: [] });
    it("lists the matching rows' lines in one target", () => {
      expect(selectionTargets(outline(), [root], roots, f("fast"))).toEqual([`${abs}:9:10`]);
    });
    it("lists every row's line when every row matches (the outline line would run @ignore rows)", () => {
      expect(selectionTargets(outline(), [root], roots, f("fast", "slow"))).toEqual([
        `${abs}:9:10:11`,
      ]);
    });
    it("falls back to the outline line when every row matches and one has no line", () => {
      expect(selectionTargets(outline(null), [root], roots, f("fast", "slow"))).toEqual([
        `${abs}:5`,
      ]);
    });
    it("is null when a matching row has no line", () => {
      expect(selectionTargets(outline(null), [root], roots, f("fast"))).toBeNull();
    });
    it("is null when no row matches", () => {
      expect(selectionTargets(outline(), [root], roots, f("other"))).toBeNull();
    });
  });
});

describe("lineTargets", () => {
  const root = path.join("D:", "Mes tests", "équipe");
  const at = (file: string, line: number) => node("scenario", file, line);
  it("groups by file with ascending lines, files in first-seen order", () => {
    const nodes = [at("tests/b.feature", 20), at("tests/a.feature", 12), at("tests/b.feature", 5)];
    expect(lineTargets(nodes, [root])).toEqual([
      `${path.join(root, "tests", "b.feature")}:5:20`,
      `${path.join(root, "tests", "a.feature")}:12`,
    ]);
  });
});

describe("tagArgs", () => {
  it("is empty when the filter is off", () => {
    expect(tagArgs({ include: [], exclude: [] })).toEqual([]);
  });
  it("joins included tags in one -t and gives each excluded tag its own", () => {
    expect(tagArgs({ include: ["smoke", "api"], exclude: ["wip", "slow"] })).toEqual([
      "-t=@smoke,@api",
      "-t=~@wip",
      "-t=~@slow",
    ]);
  });
  it("omits the include group when nothing is included", () => {
    expect(tagArgs({ include: [], exclude: ["wip"] })).toEqual(["-t=~@wip"]);
  });
  it("attaches every value, so no argument starts with @ (picocli argument files)", () => {
    expect(tagArgs({ include: ["fast"], exclude: [] })).toEqual(["-t=@fast"]);
  });
});

describe("buildArgs", () => {
  const o = {
    jar: "k.jar",
    classpath: ["c1", "c2"],
    root: "R",
    outDir: "O",
    targets: ["t1"],
    tags: [],
    env: null,
    argFile: null,
    threads: 1,
    configDir: null,
  };
  const base = [
    "-Xms64m",
    "-Xmx2g",
    "-XX:+ExitOnOutOfMemoryError",
    "-XX:CICompilerCount=2",
    "-Dstdout.encoding=UTF-8",
    "-Dstderr.encoding=UTF-8",
    "-cp",
    "k.jar;c1;c2",
    "io.karatelabs.Main",
    "run",
    "--no-color",
    "--no-pom",
    "-w",
    "R",
    "-o",
    "O",
    "-B",
    "false",
    "-f",
    "html,karate:jsonl",
  ];
  // A dry run compiles with C1 only; a run keeps C2 on fewer threads.
  const dryBase = base.with(3, "-XX:TieredStopAtLevel=1");
  it("builds the exact command line", () => {
    expect(buildArgs({ ...o, dryRun: false })).toEqual([...base, "-T", "1", "t1"]);
    expect(buildArgs({ ...o, dryRun: true })).toEqual([...dryBase, "-D", "-T", "1", "t1"]);
  });
  it("adds the debug listener and its socket to a debug run only", () => {
    const debug = { jar: "listener.jar", port: 5005, token: "t" };
    const args = buildArgs({ ...o, dryRun: false, debug });
    expect(args).toEqual([
      ...base.slice(0, 6),
      "-Dgrace.debug.port=5005",
      "-Dgrace.debug.token=t",
      "-cp",
      "k.jar;listener.jar;c1;c2",
      ...base.slice(8),
      "--listener",
      "grace.DebugListener",
      "-T",
      "1",
      "t1",
    ]);
    expect(buildArgs({ ...o, dryRun: false, debug: null })).toEqual([...base, "-T", "1", "t1"]);
  });
  it("passes the count on runs and -T 1 on dry runs", () => {
    const run = buildArgs({ ...o, threads: 8, dryRun: false });
    expect(run.slice(-3)).toEqual(["-T", "8", "t1"]);
    const dry = buildArgs({ ...o, threads: 8, dryRun: true });
    expect(dry.slice(-4)).toEqual(["-D", "-T", "1", "t1"]);
  });
  it("puts the tag arguments just before the targets", () => {
    const tags = ["-t=@smoke"];
    expect(buildArgs({ ...o, dryRun: true, tags })).toEqual([
      ...dryBase,
      "-D",
      "-T",
      "1",
      ...tags,
      "t1",
    ]);
    expect(buildArgs({ ...o, dryRun: false, tags })).toEqual([...base, "-T", "1", ...tags, "t1"]);
  });
  it("puts the argument file first and -e after -f", () => {
    expect(buildArgs({ ...o, dryRun: false, env: "qa", argFile: "A.txt" })).toEqual([
      "@A.txt",
      ...base,
      "-e",
      "qa",
      "-T",
      "1",
      "t1",
    ]);
  });
  it("puts -g after -f and -e", () => {
    expect(buildArgs({ ...o, dryRun: false, env: "qa", configDir: "conf" })).toEqual([
      ...base,
      "-e",
      "qa",
      "-g",
      "conf",
      "-T",
      "1",
      "t1",
    ]);
  });
  it("without env or argFile is unchanged", () => {
    expect(buildArgs({ ...o, dryRun: false })).toEqual([...base, "-T", "1", "t1"]);
  });
});

describe("outOfMemory", () => {
  it("finds Java's out-of-memory line among the others", () => {
    expect(
      outOfMemory(["a", "Terminating due to java.lang.OutOfMemoryError: Java heap space"]),
    ).toBe(true);
    // other out-of-memory errors are not the heap's ceiling: Java's own line is enough there
    expect(outOfMemory(["Terminating due to java.lang.OutOfMemoryError: Metaspace"])).toBe(false);
    expect(
      outOfMemory([
        "Terminating due to java.lang.OutOfMemoryError: unable to create native thread: possibly out of memory",
      ]),
    ).toBe(false);
    // a response body that names the error is the test's, not Java's
    expect(outOfMemory(['  "error": "java.lang.OutOfMemoryError: Java heap space"'])).toBe(false);
    expect(outOfMemory(["scenario failed", "OutOfMemory in a test name"])).toBe(false);
    expect(outOfMemory([])).toBe(false);
  });
  it("says what happened and what to do", () => {
    expect(OUT_OF_MEMORY).toBe(
      "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once.",
    );
  });
});

describe("childEnv", () => {
  it("drops the option variables in any case, keeps the rest, does not mutate", () => {
    const env = {
      PATH: "p",
      Karate_Options: "a",
      JAVA_TOOL_OPTIONS: "b",
      jdk_java_options: "c",
      _JAVA_OPTIONS: "d",
    };
    expect(childEnv(env)).toEqual({ PATH: "p" });
    expect(Object.keys(env)).toHaveLength(5);
  });
  it("strips KARATE_ENV and karate_config_dir", () => {
    expect(childEnv({ PATH: "p", KARATE_ENV: "prod", karate_config_dir: "x" })).toEqual({
      PATH: "p",
    });
  });
});
