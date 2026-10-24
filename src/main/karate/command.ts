import { existsSync } from "node:fs";
import path from "node:path";
import type { Layout, TestNode } from "../../shared/model";
import { filterOn, matchingLeaves, NO_FILTER, type TagFilter } from "../../shared/tags";

export interface ProjectLayout {
  layout: Layout;
  classpath: string[];
  featureRoots: string[];
}

/** All paths absolute. Maven/Gradle: whichever of src/test/java and src/test/resources exist. */
export function detectLayout(root: string): ProjectLayout {
  const has = (f: string) => existsSync(path.join(root, f));
  const layout: Layout = has("pom.xml")
    ? "maven"
    : has("build.gradle") || has("build.gradle.kts")
      ? "gradle"
      : "standalone";
  if (layout === "standalone") return { layout, classpath: [root], featureRoots: [root] };
  const dirs = [path.join("src", "test", "java"), path.join("src", "test", "resources")]
    .map((d) => path.join(root, d))
    .filter((d) => existsSync(d));
  return { layout, classpath: dirs, featureRoots: dirs };
}

/**
 * `-t` arguments: included tags in one OR group, each excluded tag its own (AND). Attached as
 * `-t=…`: picocli reads a separate argument starting with `@` as an argument file from the cwd.
 */
export function tagArgs(f: TagFilter): string[] {
  return [
    ...(f.include.length ? [`-t=${f.include.map((t) => `@${t}`).join(",")}`] : []),
    ...f.exclude.map((t) => `-t=~@${t}`),
  ];
}

/**
 * Run targets for a node. A `file:LINE` target bypasses `-t` and `@ignore`, so an outline under a
 * filter lists its matching rows' lines, even when every row matches: the outline's own line would
 * also run rows the dry run does not list. Only when a row has no line (dynamic Examples) and every
 * row matches does it fall back to the outline's line. Null when a matching row has no line and
 * not every row matches, or when no row matches.
 */
export function selectionTargets(
  node: TestNode,
  root: string,
  featureRoots: string[],
  f: TagFilter,
): string[] | null {
  if (node.kind === "folder" && node.file === "") return featureRoots;
  const abs = path.join(root, ...node.file.split("/"));
  if (node.kind === "outline" && filterOn(f)) {
    const hits = matchingLeaves(node, f);
    const lines = hits.map((r) => r.line);
    if (hits.length === 0) return null;
    if (!lines.includes(null)) return [`${abs}:${lines.join(":")}`];
    return hits.length === matchingLeaves(node, NO_FILTER).length ? [`${abs}:${node.line}`] : null;
  }
  return node.kind === "folder" || node.kind === "feature" ? [abs] : [`${abs}:${node.line}`];
}

export function buildArgs(o: {
  jar: string;
  classpath: string[];
  root: string;
  outDir: string;
  dryRun: boolean;
  tags: string[];
  targets: string[];
}): string[] {
  return [
    "-Dstdout.encoding=UTF-8",
    "-Dstderr.encoding=UTF-8",
    "-cp",
    [o.jar, ...o.classpath].join(";"),
    "io.karatelabs.Main",
    "run",
    "--no-color",
    "-w",
    o.root,
    "-o",
    o.outDir,
    "-B",
    "false",
    "-f",
    "html,karate:jsonl",
    ...(o.dryRun ? ["-D"] : []),
    ...o.tags,
    ...o.targets,
  ];
}

const STRIPPED = ["karate_options", "java_tool_options", "jdk_java_options", "_java_options"];

/** Windows env keys come in any case, so compare lower-cased. Does not mutate `env`. */
export function childEnv(env: NodeJS.ProcessEnv): NodeJS.ProcessEnv {
  return Object.fromEntries(
    Object.entries(env).filter(([k]) => !STRIPPED.includes(k.toLowerCase())),
  );
}
