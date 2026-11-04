// Says when a Maven or Gradle project's Java code is not built: Grace runs what the build compiled and never compiles
// (docs/specs/2026-11-02-project-libraries.md R8).
import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";
import type { Layout } from "../../shared/model";

const COMPILED_TESTS = { maven: "target/test-classes", gradle: "build/classes/java/test" };
const BUILD = { maven: "mvn test-compile", gradle: "gradle testClasses" };

/**
 * The newest modified time of the files under `dir` named `*<ext>` that `keep` accepts; null without such a file or
 * folder. Never throws.
 */
async function newest(
  dir: string,
  ext: string,
  keep: (file: string) => Promise<boolean> = async () => true,
): Promise<number | null> {
  let names: string[];
  try {
    names = await readdir(dir, { recursive: true });
  } catch {
    return null;
  }
  const times = await Promise.all(
    names
      .filter((n) => n.endsWith(ext))
      .map(async (n) => {
        const file = path.join(dir, n);
        try {
          return (await keep(file)) ? (await stat(file)).mtimeMs : null;
        } catch {
          return null;
        }
      }),
  );
  return times.reduce<number | null>(
    (a, t) => (t === null || (a !== null && a >= t) ? a : t),
    null,
  );
}

// A JUnit runner imports JUnit or Karate's JUnit module (Karate 1 `com.intuit.karate.junit5`, Karate 2
// `io.karatelabs.junit6`). Grace runs the features itself and never loads one, so it needs no build.
const RUNNER =
  /^\s*import\s+(?:static\s+)?(?:org\.junit|com\.intuit\.karate\.junit|io\.karatelabs\.junit)/m;
const notRunner = async (file: string) => !RUNNER.test(await readFile(file, "utf8"));

/**
 * R8: when src/test/java holds Java code other than JUnit runners, the notice if the build's compiled test folder has no
 * class, or if such a source is newer than its newest class; else null. Reads the `.java` files and the `.class` file
 * times, on open and Refresh.
 */
export async function testCodeNotice(root: string, layout: Layout): Promise<string | null> {
  if (layout === "standalone") return null;
  const sources = await newest(path.join(root, "src", "test", "java"), ".java", notRunner);
  if (sources === null) return null;
  const classes = await newest(path.join(root, ...COMPILED_TESTS[layout].split("/")), ".class");
  const build = BUILD[layout];
  if (classes === null)
    return `The project's Java code in src/test/java is not built: build it (${build}) before running.`;
  return sources > classes
    ? `The project's Java code in src/test/java changed since its last build: build it again (${build}) before running.`
    : null;
}
