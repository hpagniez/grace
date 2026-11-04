// Says when a Maven or Gradle project's Java code is not built: Grace runs what the build compiled, and builds only when
// the person clicks Build (docs/specs/2026-11-02-project-libraries.md R8, docs/specs/2026-11-04-build-from-notice.md).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
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

const builtFile = (dir: string) => path.join(dir, "built.json");

/** Build from the notice R4.3: when Grace's last successful Build started, in ms; null if none. Never throws. */
export function readBuilt(dir: string): number | null {
  try {
    const at = JSON.parse(readFileSync(builtFile(dir), "utf8")).at;
    return Number.isFinite(at) ? at : null;
  } catch {
    return null;
  }
}

export function writeBuilt(dir: string, at: number): void {
  mkdirSync(dir, { recursive: true });
  writeFileSync(builtFile(dir), JSON.stringify({ at }));
}

/**
 * R8: when src/test/java holds Java code other than JUnit runners, the notice if the build's compiled test folder has no
 * class, or if such a source is newer than its newest class and than `builtAt`, the start of the last successful Build;
 * else null. Reads the `.java` files and the `.class` file times, on open and Refresh.
 */
export async function testCodeNotice(
  root: string,
  layout: Layout,
  builtAt: number | null = null,
): Promise<string | null> {
  if (layout === "standalone") return null;
  const sources = await newest(path.join(root, "src", "test", "java"), ".java", notRunner);
  if (sources === null) return null;
  const classes = await newest(path.join(root, ...COMPILED_TESTS[layout].split("/")), ".class");
  const build = BUILD[layout];
  if (classes === null)
    return `The project's Java code in src/test/java is not built: build it (${build}) before running.`;
  return sources > Math.max(classes, builtAt ?? classes)
    ? `The project's Java code in src/test/java changed since its last build: build it again (${build}) before running.`
    : null;
}
