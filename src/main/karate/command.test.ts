import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import type { TestNode } from "../../shared/model";
import { buildArgs, childEnv, detectLayout, selectionTargets } from "./command";

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
    expect(selectionTargets(node("folder", "", null), root, roots)).toEqual(roots);
    expect(selectionTargets(node("folder", "tests", null), root, roots)).toEqual([
      path.join(root, "tests"),
    ]);
    expect(selectionTargets(node("feature", "tests/a.feature", null), root, roots)).toEqual([abs]);
    for (const kind of ["scenario", "outline", "row"] as const)
      expect(selectionTargets(node(kind, "tests/a.feature", 12), root, roots)).toEqual([
        `${abs}:12`,
      ]);
  });
});

describe("buildArgs", () => {
  const o = { jar: "k.jar", classpath: ["c1", "c2"], root: "R", outDir: "O", targets: ["t1"] };
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
    expect(buildArgs({ ...o, dryRun: false })).toEqual([...base, "t1"]);
    expect(buildArgs({ ...o, dryRun: true })).toEqual([...base, "-D", "t1"]);
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
});
