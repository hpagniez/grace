// A Maven or Gradle project's libraries, listed by its own build (docs/specs/2026-11-02-project-libraries.md, ADR 0027).
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import type { Layout } from "../../shared/model";

export type BuildLayout = "maven" | "gradle";

export const TOOL_NAME: Record<BuildLayout, "Maven" | "Gradle"> = {
  maven: "Maven",
  gradle: "Gradle",
};

/** The last list the build gave, kept in `<project data folder>/libraries.json`. */
export interface KeptLibraries {
  tool: BuildLayout;
  key: string;
  jars: string[];
}

const WRAPPER: Record<BuildLayout, string> = { maven: "mvnw.cmd", gradle: "gradlew.bat" };
const ON_PATH: Record<BuildLayout, string[]> = {
  maven: ["mvn.cmd", "mvn.bat"],
  gradle: ["gradle.bat"],
};

const folders = (searchPath: string) => searchPath.split(";").filter(Boolean);

/** R2: the project's wrapper, else the first launcher in a `searchPath` folder; null if none. */
export function findTool(root: string, layout: BuildLayout, searchPath: string): string | null {
  const wrapper = path.join(root, WRAPPER[layout]);
  if (existsSync(wrapper)) return wrapper;
  for (const dir of folders(searchPath))
    for (const name of ON_PATH[layout])
      if (existsSync(path.join(dir, name))) return path.join(dir, name);
  return null;
}

export const noToolReason = (layout: BuildLayout) =>
  layout === "maven"
    ? "the project has no mvnw.cmd and mvn is not on PATH"
    : "the project has no gradlew.bat and gradle is not on PATH";

/** R4.2: the files whose change can change the list. */
const BUILD_FILES: Record<BuildLayout, string[]> = {
  maven: [
    "pom.xml",
    ".mvn/maven.config",
    ".mvn/extensions.xml",
    ".mvn/wrapper/maven-wrapper.properties",
  ],
  gradle: [
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "gradle.properties",
    "gradle/libs.versions.toml",
    "gradle/wrapper/gradle-wrapper.properties",
  ],
};

export function libraryKey(root: string, layout: BuildLayout): string {
  const hash = createHash("sha256");
  for (const name of BUILD_FILES[layout]) {
    let content: Buffer | string;
    try {
      content = readFileSync(path.join(root, name));
    } catch {
      content = "-"; // absent
    }
    hash.update(`${name}\0`).update(content).update("\0");
  }
  return hash.digest("hex");
}

const keptFile = (dir: string) => path.join(dir, "libraries.json");

/** The kept list, or null when missing or not of that shape. Never throws. */
export function readKept(dir: string): KeptLibraries | null {
  try {
    const k = JSON.parse(readFileSync(keptFile(dir), "utf8"));
    return (k.tool === "maven" || k.tool === "gradle") &&
      typeof k.key === "string" &&
      Array.isArray(k.jars) &&
      k.jars.every((j: unknown) => typeof j === "string")
      ? { tool: k.tool, key: k.key, jars: k.jars }
      : null;
  } catch {
    return null;
  }
}

export function writeKept(dir: string, kept: KeptLibraries): void {
  mkdirSync(dir, { recursive: true });
  writeFileSync(keptFile(dir), JSON.stringify(kept));
}

/** R4.3: the tool must run again. */
export const isStale = (kept: KeptLibraries | null, layout: BuildLayout, key: string) =>
  !kept || kept.tool !== layout || kept.key !== key || !kept.jars.every((j) => existsSync(j));

/** R6.2: what a run puts on the classpath, as kept, even if stale. Never throws. */
export function keptJars(dir: string, layout: Layout): string[] {
  const kept = readKept(dir);
  return kept && kept.tool === layout ? kept.jars : [];
}

/**
 * R3: run by `cmd.exe /d /s /c "<command>"`. Every path is a variable that cmd.exe expands inside quotes, so no
 * path is ever parsed as part of the command (`&` or `%` in a folder name).
 */
export const TOOL_COMMAND: Record<BuildLayout, string> = {
  maven:
    '"%GRACE_TOOL%" -q -B dependency:build-classpath "-Dmdep.outputFile=%GRACE_CLASSPATH_FILE%"',
  gradle:
    '"%GRACE_TOOL%" -q --no-daemon --console=plain --no-configuration-cache --init-script "%GRACE_INIT_SCRIPT%" graceClasspath',
};

/**
 * R2.4: the person's environment as is, Grace's variables, and `jreDir` only when there is no other Java. Gradle gets
 * none (`jreDir` null): it needs a JDK to resolve a Java project's classpath, which the bundled Java is not.
 */
export function toolEnv(
  base: NodeJS.ProcessEnv,
  o: { tool: string; file: string; initScript: string; searchPath: string; jreDir: string | null },
): NodeJS.ProcessEnv {
  const lend =
    o.jreDir !== null &&
    !base.JAVA_HOME &&
    !folders(o.searchPath).some((d) => existsSync(path.join(d, "java.exe")));
  return {
    ...base,
    ...(lend ? { JAVA_HOME: o.jreDir! } : {}),
    GRACE_TOOL: o.tool,
    GRACE_CLASSPATH_FILE: o.file,
    GRACE_INIT_SCRIPT: o.initScript,
  };
}

/** R3.4: the listed entries that exist, in order, once, less those already on the classpath. */
export function parseLibraries(text: string, classpath: string[]): string[] {
  const key = (p: string) => path.resolve(p).toLowerCase();
  const seen = new Set(classpath.map(key));
  const out: string[] = [];
  for (const entry of text.split(/[;\r\n]/).map((e) => e.trim())) {
    if (!entry || seen.has(key(entry)) || !existsSync(entry)) continue;
    seen.add(key(entry));
    out.push(entry);
  }
  return out;
}

/**
 * R6.3: Maven's first `[ERROR]`, else a launcher's `ERROR:` (no Java), else the deepest cause Gradle says went wrong,
 * else the last line; without its full stop, which the notice adds, and 300 characters at most.
 */
export function failureReason(lines: string[]): string {
  const said = lines.map((l) => l.trim());
  const error = said.find((l) => l.startsWith("[ERROR]") || l.startsWith("ERROR:"));
  const reason =
    error?.replace(/^(\[ERROR\]|ERROR:)\s*/, "") ??
    gradleCause(said) ??
    said.filter(Boolean).at(-1) ??
    "it stopped with no message";
  return reason.replace(/\.+$/, "").slice(0, 300);
}

/** The last `> ` cause under Gradle's `* What went wrong:`, else its first line; the first names only the task. */
function gradleCause(said: string[]): string | undefined {
  const start = said.indexOf("* What went wrong:");
  if (start < 0) return undefined;
  const end = said.indexOf("", start + 1);
  const block = said.slice(start + 1, end < 0 ? undefined : end);
  return block.findLast((l) => l.startsWith(">"))?.replace(/^>\s*/, "") ?? block[0];
}

/** R3.2: adds `graceClasspath` to the root project; it writes the test runtime classpath to GRACE_CLASSPATH_FILE. */
export const INIT_SCRIPT = `// Grace: the test runtime classpath of the root project (docs/specs/2026-11-02-project-libraries.md R3.2).
rootProject {
    tasks.register('graceClasspath') {
        def files = project.extensions.getByName('sourceSets').getByName('test').runtimeClasspath
        def out = new File(System.getenv('GRACE_CLASSPATH_FILE'))
        doLast { out.setText(files.files.join(File.pathSeparator), 'UTF-8') }
    }
}
`;
