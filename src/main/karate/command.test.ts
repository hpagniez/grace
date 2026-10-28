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
  lineTargets,
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

describe("detectLayout", () => {
  it("maven with both test folders", () => {
    const root = project(["pom.xml"], ["src/test/java", "src/test/resources"]);
    const both = [path.join(root, "src/test/java"), path.join(root, "src/test/resources")];
    expect(detectLayout(root)).toEqual({ layout: "maven", classpath: both, featureRoots: both });
  });
  it("gradle", () => {
    const root = project(["build.gradle.kts"], ["src/test/resources"]);
    expect(detectLayout(root).layout).toBe("gradle");
  });
  it("only existing folders", () => {
    const root = project(["pom.xml"], ["src/test/java"]);
    const only = [path.join(root, "src/test/java")];
    expect(detectLayout(root)).toMatchObject({ classpath: only, featureRoots: only });
  });
  it("standalone", () => {
    const root = project([]);
    expect(detectLayout(root)).toEqual({
      layout: "standalone",
      classpath: [root],
      featureRoots: [root],
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

describe("selectionTargets", () => {
  const root = path.join("D:", "Mes tests", "équipe");
  const roots = [path.join(root, "src", "test", "java")];
  const abs = path.join(root, "tests", "a.feature");
  it("maps each node kind", () => {
    expect(selectionTargets(node("folder", "", null), root, roots, NO_FILTER)).toEqual(roots);
    expect(selectionTargets(node("folder", "tests", null), root, roots, NO_FILTER)).toEqual([
      path.join(root, "tests"),
    ]);
    expect(
      selectionTargets(node("feature", "tests/a.feature", null), root, roots, NO_FILTER),
    ).toEqual([abs]);
    for (const kind of ["scenario", "outline", "row"] as const)
      expect(selectionTargets(node(kind, "tests/a.feature", 12), root, roots, NO_FILTER)).toEqual([
        `${abs}:12`,
      ]);
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
      expect(selectionTargets(outline(), root, roots, f("fast"))).toEqual([`${abs}:9:10`]);
    });
    it("lists every row's line when every row matches (the outline line would run @ignore rows)", () => {
      expect(selectionTargets(outline(), root, roots, f("fast", "slow"))).toEqual([
        `${abs}:9:10:11`,
      ]);
    });
    it("falls back to the outline line when every row matches and one has no line", () => {
      expect(selectionTargets(outline(null), root, roots, f("fast", "slow"))).toEqual([`${abs}:5`]);
    });
    it("is null when a matching row has no line", () => {
      expect(selectionTargets(outline(null), root, roots, f("fast"))).toBeNull();
    });
    it("is null when no row matches", () => {
      expect(selectionTargets(outline(), root, roots, f("other"))).toBeNull();
    });
  });
});

describe("lineTargets", () => {
  const root = path.join("D:", "Mes tests", "équipe");
  const at = (file: string, line: number) => node("scenario", file, line);
  it("groups by file with ascending lines, files in first-seen order", () => {
    const nodes = [at("tests/b.feature", 20), at("tests/a.feature", 12), at("tests/b.feature", 5)];
    expect(lineTargets(nodes, root)).toEqual([
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
  };
  const base = [
    "-Dstdout.encoding=UTF-8",
    "-Dstderr.encoding=UTF-8",
    "-cp",
    "k.jar;c1;c2",
    "io.karatelabs.Main",
    "run",
    "--no-color",
    "-w",
    "R",
    "-o",
    "O",
    "-B",
    "false",
    "-f",
    "html,karate:jsonl",
  ];
  it("builds the exact command line", () => {
    expect(buildArgs({ ...o, dryRun: false })).toEqual([...base, "-T", "1", "t1"]);
    expect(buildArgs({ ...o, dryRun: true })).toEqual([...base, "-D", "-T", "1", "t1"]);
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
      ...base,
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
  it("without env or argFile is unchanged", () => {
    expect(buildArgs({ ...o, dryRun: false })).toEqual([...base, "-T", "1", "t1"]);
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
