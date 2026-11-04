import { existsSync, statSync } from "node:fs";
import path from "node:path";
import { STRIPPED_ENV_VARS } from "../../shared/env";
import type { Layout, TestNode } from "../../shared/model";
import { filterOn, matchingLeaves, NO_FILTER, type TagFilter } from "../../shared/tags";
import { DEBUG_LISTENER } from "./debug";
import { readPom } from "./pom";

export interface ProjectLayout {
  layout: Layout;
  /** The source folders, then the build's compiled output folders that exist. */
  classpath: string[];
  /** The karate-pom.json `paths` kept, or the source folders. */
  featureRoots: string[];
  /** Karate's `-g`, from karate-pom.json, as written. */
  configDir: string | null;
  /** Where `karate-config-<env>.js` files are read from; never a compiled folder, whose copies may be stale. */
  configDirs: string[];
  /** What the open notice says about karate-pom.json; null when there is nothing to say. */
  pomNotice: string | null;
  /** karate-pom.json's `output.junitXml`: runs also ask Karate for JUnit XML (docs/specs/2026-11-03-export-reports.md R1). */
  junitXml: boolean;
  /** Where Karate's names of the tree's files resolve, for `locate`: Karate's root, then the project. */
  bases: string[];
}

const SOURCE = ["src/test/java", "src/test/resources"];
const COMPILED: Record<Layout, string[]> = {
  maven: ["target/test-classes", "target/classes"],
  gradle: [
    "build/classes/java/test",
    "build/resources/test",
    "build/classes/java/main",
    "build/resources/main",
  ],
  standalone: [],
};

/** True for the project folder and anything under it, by the names alone; a folder named `..data` is under it. */
export const inside = (root: string, abs: string) => {
  const rel = path.relative(root, abs);
  return rel !== ".." && !rel.startsWith(`..${path.sep}`) && !path.isAbsolute(rel);
};

/**
 * The folder Karate names files relative to, as its `Suite.computeRoot` finds it: a configDir that is not on the
 * classpath, else the folder of the first `karate-config.js` (or configDir file) on the classpath, else the project.
 * shortcut: a library folder later on the classpath is not checked, as no tree file is under one; check them if one can be.
 */
function karateRoot(root: string, classpath: string[], configDir: string | null): string {
  if (configDir !== null && !configDir.startsWith("classpath:")) {
    const dir = path.resolve(root, configDir.replace(/^file:/, ""));
    return configDir.endsWith(".js") ? path.dirname(dir) : dir;
  }
  const rel = configDir?.slice("classpath:".length) ?? "";
  const config = rel.endsWith(".js") ? rel : path.join(rel, "karate-config.js");
  const hit = classpath.map((d) => path.join(d, config)).find((f) => existsSync(f));
  return hit ? path.dirname(hit) : root;
}

/**
 * All paths absolute. Maven/Gradle: whichever of src/test/java and src/test/resources exist. The project's
 * karate-pom.json gives the feature roots and the config folder (docs/specs/2026-11-02-project-setup.md).
 */
export function detectLayout(root: string): ProjectLayout {
  const has = (f: string) => existsSync(path.join(root, f));
  const layout: Layout = has("pom.xml")
    ? "maven"
    : has("build.gradle") || has("build.gradle.kts")
      ? "gradle"
      : "standalone";
  const existing = (dirs: string[]) => dirs.map((d) => path.join(root, d)).filter(existsSync);
  const source = layout === "standalone" ? [root] : existing(SOURCE);
  const pom = readPom(root);
  const kept = (pom.paths ?? []).filter(
    (p) =>
      !p.startsWith("classpath:") &&
      existsSync(path.resolve(root, p)) &&
      inside(root, path.resolve(root, p)),
  );
  const leftOut = (pom.paths ?? []).filter((p) => !kept.includes(p));
  const { configDir } = pom;
  const notice = pom.error
    ? [`karate-pom.json could not be read: ${pom.error}. Grace uses the default folders.`]
    : [
        ...(pom.unused.length
          ? [`Grace does not use these karate-pom.json settings: ${pom.unused.join(", ")}.`]
          : []),
        ...(leftOut.length
          ? [
              `These karate-pom.json paths are not folders or files in the project: ${leftOut.join(", ")}.`,
            ]
          : []),
      ];
  const classpath = [...source, ...existing(COMPILED[layout])];
  return {
    layout,
    classpath,
    featureRoots: pom.paths ? kept.map((p) => path.resolve(root, p)) : source,
    configDir,
    configDirs:
      configDir === null
        ? [...new Set([root, ...source])]
        : [
            ...(configDir.startsWith("classpath:")
              ? source.map((s) => path.join(s, configDir.slice("classpath:".length)))
              : [path.resolve(root, configDir)]),
            root,
          ],
    pomNotice: notice.length ? notice.join(" ") : null,
    junitXml: pom.junitXml,
    bases: [...new Set([karateRoot(root, classpath, configDir), root])],
  };
}

/**
 * The Karate CLI's extension folders that exist, home first, each as a `<folder>\*` classpath entry: its launcher
 * puts every jar of `~/.karate/ext` and `.karate/ext` on the classpath (docs/specs/2026-11-02-project-libraries.md R1).
 */
export function extensionJars(home: string, root: string): string[] {
  const isDir = (d: string) => {
    try {
      return statSync(d).isDirectory();
    } catch {
      return false;
    }
  };
  return [home, root]
    .map((d) => path.join(d, ".karate", "ext"))
    .filter(isDir)
    .map((d) => path.join(d, "*"));
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
 * The absolute path of a tree file or folder: the first of `bases` (Karate's root, then the project folder) that holds it.
 * Karate gives it relative to its root when the file is under it, else to the working directory (#208, #223).
 */
export function locate(bases: string[], file: string): string {
  const all = bases.map((b) => path.join(b, ...file.split("/")));
  return all.find((p) => existsSync(p)) ?? all[0];
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
  bases: string[],
  featureRoots: string[],
  f: TagFilter,
): string[] | null {
  if (node.kind === "folder" && node.file === "") return featureRoots;
  const abs = locate(bases, node.file);
  if (node.kind === "outline" && filterOn(f)) {
    const hits = matchingLeaves(node, f);
    const lines = hits.map((r) => r.line);
    if (hits.length === 0) return null;
    if (!lines.includes(null)) return [`${abs}:${lines.join(":")}`];
    return hits.length === matchingLeaves(node, NO_FILTER).length ? [`${abs}:${node.line}`] : null;
  }
  return node.kind === "folder" || node.kind === "feature" ? [abs] : [`${abs}:${node.line}`];
}

/** One `<abs file>:L1:L2…` target per feature file for nodes that have a line: lines ascending, files in first-seen order. */
export function lineTargets(nodes: TestNode[], bases: string[]): string[] {
  const byFile = new Map<string, number[]>();
  for (const n of nodes) {
    if (n.line === null) continue;
    const abs = locate(bases, n.file);
    byFile.set(abs, [...(byFile.get(abs) ?? []), n.line]);
  }
  return [...byFile].map(([abs, lines]) => `${abs}:${lines.sort((a, b) => a - b).join(":")}`);
}

/** A heap that starts small and stops at 2 GB, and an exit at the first OutOfMemoryError (docs/specs/2026-11-01-java-memory.md). */
const HEAP = ["-Xms64m", "-Xmx2g", "-XX:+ExitOnOutOfMemoryError"];

/**
 * A dry run runs no step, so C1 alone lists the tests for a quarter of the CPU. A run keeps C2, which a suite of
 * heavy JavaScript needs, with one compiler thread of each kind instead of 12 on 16 cores (docs/specs/2026-11-02-java-jit.md).
 */
const JIT = { dry: "-XX:TieredStopAtLevel=1", run: "-XX:CICompilerCount=2" };

export const OUT_OF_MEMORY =
  "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once.";

/**
 * The JVM's own line for a full heap, not a test's output (a logged response body can name the error too), nor
 * another OutOfMemoryError (Metaspace, native threads) that the 2 GB ceiling did not cause.
 */
export const outOfMemory = (lines: string[]) =>
  lines.some((l) => l.startsWith("Terminating due to java.lang.OutOfMemoryError: Java heap space"));

export function buildArgs(o: {
  jar: string;
  classpath: string[];
  root: string;
  outDir: string;
  dryRun: boolean;
  tags: string[];
  targets: string[];
  env: string | null;
  argFile: string | null;
  /** Karate's `-T`. A dry run always gets 1: more threads would interleave its events. */
  threads: number;
  /** Karate's `-g`, as written in karate-pom.json. */
  configDir: string | null;
  /** A debug run's listener jar and socket (docs/specs/2026-11-02-step-debugger.md, R2.1). */
  debug?: { jar: string; port: number; token: string } | null;
  /** Adds Karate's JUnit XML to the formats. */
  junitXml: boolean;
}): string[] {
  const { debug } = o;
  return [
    // The launcher expands `@file` only before the main class, so it goes first.
    ...(o.argFile ? [`@${o.argFile}`] : []),
    ...HEAP,
    o.dryRun ? JIT.dry : JIT.run,
    "-Dstdout.encoding=UTF-8",
    "-Dstderr.encoding=UTF-8",
    ...(debug ? [`-Dgrace.debug.port=${debug.port}`, `-Dgrace.debug.token=${debug.token}`] : []),
    "-cp",
    [o.jar, ...(debug ? [debug.jar] : []), ...o.classpath].join(";"),
    "io.karatelabs.Main",
    "run",
    "--no-color",
    "--no-pom", // Grace reads karate-pom.json itself: Karate would apply its env, tags or dryRun unseen
    "-w",
    o.root,
    "-o",
    o.outDir,
    "-B",
    "false",
    "-f",
    o.junitXml ? "html,karate:jsonl,junit:xml" : "html,karate:jsonl",
    ...(debug ? ["--listener", DEBUG_LISTENER] : []),
    ...(o.env ? ["-e", o.env] : []),
    ...(o.configDir ? ["-g", o.configDir] : []),
    ...(o.dryRun ? ["-D", "-T", "1"] : ["-T", String(o.threads)]),
    ...o.tags,
    ...o.targets,
  ];
}

/**
 * `env` without the variables Grace strips, then with each of `vars` set, replacing one of the same name in any
 * letter case: Windows sees one variable. A later one wins. Windows env keys come in any case, so compare upper-cased.
 * Does not mutate `env`.
 */
export function childEnv(
  env: NodeJS.ProcessEnv,
  vars: { name: string; value: string }[] = [],
): NodeJS.ProcessEnv {
  const out = Object.fromEntries(
    Object.entries(env).filter(([k]) => !STRIPPED_ENV_VARS.includes(k.toUpperCase())),
  );
  for (const { name, value } of vars) {
    for (const k of Object.keys(out)) if (k.toUpperCase() === name.toUpperCase()) delete out[k];
    out[name] = value;
  }
  return out;
}
