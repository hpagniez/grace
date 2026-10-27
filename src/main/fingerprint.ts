// The project fingerprint (F15): a hash of the path, size and modified time of every file under the
// project root, to tell whether two runs saw the same project files. No file content is read.
import { createHash } from "node:crypto";
import { lstat, readdir } from "node:fs/promises";
import path from "node:path";

/** Folder names, lower case, whose content is tool state, not the project, wherever they are. */
export const SKIPPED_DIRS: readonly string[] = [".git", "node_modules", ".gradle"];
/** Output folders, lower case: skipped only beside a build file, so a test folder of that name counts. */
const OUTPUT_DIRS: readonly string[] = ["target", "build"];
const BUILD_FILES: readonly string[] = ["pom.xml", "build.gradle", "build.gradle.kts"];
export const MAX_FILES = 50_000;
export const WALK_TIMEOUT_MS = 2_000;

/**
 * The SHA-256, lower-case hex, of one `<relative path>\t<size>\t<mtime ms>\n` line per file, sorted.
 * Symbolic links and junctions are neither followed nor listed. Null past `maxFiles` files, past
 * `timeoutMs`, or on any read or stat error. Never throws; the walk is asynchronous, so it never blocks.
 */
export async function projectFingerprint(
  root: string,
  limits: { maxFiles: number; timeoutMs: number } = {
    maxFiles: MAX_FILES,
    timeoutMs: WALK_TIMEOUT_MS,
  },
): Promise<string | null> {
  const deadline = Date.now() + limits.timeoutMs;
  const lines: string[] = [];
  let files = 0;

  // A walk that is still running when another gave up stops at its next limit check, which fails too.
  async function walk(dir: string, rel: string): Promise<void> {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const e of entries) if (!e.isDirectory() && !e.isSymbolicLink()) files++;
    if (Date.now() >= deadline || files > limits.maxFiles) throw new Error("limit");
    const hasBuildFile = entries.some(
      (e) => e.isFile() && BUILD_FILES.includes(e.name.toLowerCase()),
    );
    await Promise.all(
      entries.map(async (e) => {
        if (e.isSymbolicLink()) return;
        const entryRel = rel ? `${rel}/${e.name}` : e.name;
        const entryPath = path.join(dir, e.name);
        if (e.isDirectory()) {
          const name = e.name.toLowerCase();
          if (!SKIPPED_DIRS.includes(name) && !(hasBuildFile && OUTPUT_DIRS.includes(name)))
            await walk(entryPath, entryRel);
          return;
        }
        const st = await lstat(entryPath);
        lines.push(`${entryRel}\t${st.size}\t${Math.trunc(st.mtimeMs)}\n`);
      }),
    );
  }

  try {
    await walk(root, "");
  } catch {
    return null;
  }
  return createHash("sha256").update(lines.sort().join("")).digest("hex");
}
