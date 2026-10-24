import { existsSync } from "node:fs";
import path from "node:path";
import type { Layout, TestNode } from "../../shared/model";

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

export function selectionTargets(node: TestNode, root: string, featureRoots: string[]): string[] {
  if (node.kind === "folder" && node.file === "") return featureRoots;
  const abs = path.join(root, ...node.file.split("/"));
  return node.kind === "folder" || node.kind === "feature" ? [abs] : [`${abs}:${node.line}`];
}

export function buildArgs(o: {
  jar: string;
  classpath: string[];
  root: string;
  outDir: string;
  dryRun: boolean;
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
