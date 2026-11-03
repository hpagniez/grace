// Says when a Maven or Gradle project's Java code is not built: Grace runs what the build compiled and never compiles
// (docs/specs/2026-11-02-project-libraries.md R8).
import { readdir, stat } from "node:fs/promises";
import path from "node:path";
import type { Layout } from "../../shared/model";

const COMPILED_TESTS = { maven: "target/test-classes", gradle: "build/classes/java/test" };
const BUILD = { maven: "mvn test-compile", gradle: "gradle testClasses" };

/** The newest modified time of the files under `dir` named `*<ext>`; null without such a file or folder. Never throws. */
async function newest(dir: string, ext: string): Promise<number | null> {
  let names: string[];
  try {
    names = await readdir(dir, { recursive: true });
  } catch {
    return null;
  }
  const times = await Promise.all(
    names
      .filter((n) => n.endsWith(ext))
      .map((n) =>
        stat(path.join(dir, n)).then(
          (s) => s.mtimeMs,
          () => null,
        ),
      ),
  );
  return times.reduce<number | null>(
    (a, t) => (t === null || (a !== null && a >= t) ? a : t),
    null,
  );
}

/**
 * R8: when src/test/java holds Java code, the notice if the build's compiled test folder has no class, or if a source is
 * newer than its newest class; else null. Reads file times only, on open and Refresh.
 */
export async function testCodeNotice(root: string, layout: Layout): Promise<string | null> {
  if (layout === "standalone") return null;
  const sources = await newest(path.join(root, "src", "test", "java"), ".java");
  if (sources === null) return null;
  const classes = await newest(path.join(root, ...COMPILED_TESTS[layout].split("/")), ".class");
  const build = BUILD[layout];
  if (classes === null)
    return `The project's Java code in src/test/java is not built: build it (${build}) before running.`;
  return sources > classes
    ? `The project's Java code in src/test/java changed since its last build: build it again (${build}) before running.`
    : null;
}
