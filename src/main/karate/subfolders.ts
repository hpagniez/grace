// Folders below an opened folder with no test that look like a Karate project (docs/specs/2026-11-04-subfolder-offer.md R2).
import type { Dirent } from "node:fs";
import { readdir } from "node:fs/promises";
import path from "node:path";
import { OUTPUT_DIRS, SKIPPED_DIRS } from "../fingerprint";

export const SUBFOLDER_LIMITS = { depth: 3, found: 10, folders: 2_000, timeoutMs: 2_000 };

const SKIPPED: readonly string[] = [...SKIPPED_DIRS, ...OUTPUT_DIRS];
const MARKERS: readonly string[] = ["karate-pom.json", "karate-config.js"];
const BUILD_FILES: readonly string[] = ["pom.xml", "build.gradle", "build.gradle.kts"];
const STOP = new Error("limit");

const lower = (e: Dirent) => e.name.toLowerCase();
/** Sub-folders to look in, by name; links and junctions are not folders here. */
const folders = (entries: Dirent[]) =>
  entries
    .filter((e) => e.isDirectory() && !SKIPPED.includes(lower(e)))
    .map((e) => e.name)
    .sort((a, b) => a.localeCompare(b));

/**
 * Folders 1 to `depth` levels below `root` that hold `karate-pom.json` or `karate-config.js`, or a build file beside
 * a `src/test` with a `.feature` under it; relative with `/`, by level then path, never looked inside. Returns what it
 * found once a limit is reached, even if a read never returns. Never throws.
 */
export async function karateFolders(root: string, limits = SUBFOLDER_LIMITS): Promise<string[]> {
  const found: string[] = [];
  const deadline = Date.now() + limits.timeoutMs;
  let reads = 0;
  let done = false;

  const read = async (dir: string): Promise<Dirent[]> => {
    if (done || reads >= limits.folders || Date.now() >= deadline) throw STOP;
    reads++;
    try {
      return await readdir(dir, { withFileTypes: true });
    } catch {
      return []; // unreadable, gone, or not a folder
    }
  };
  const hasFeature = async (dir: string): Promise<boolean> => {
    const entries = await read(dir);
    if (entries.some((e) => e.isFile() && lower(e).endsWith(".feature"))) return true;
    for (const name of folders(entries)) if (await hasFeature(path.join(dir, name))) return true;
    return false;
  };
  const isProject = async (dir: string, entries: Dirent[]): Promise<boolean> => {
    const files = entries.filter((e) => e.isFile()).map(lower);
    if (files.some((f) => MARKERS.includes(f))) return true;
    const src = entries.find((e) => e.isDirectory() && lower(e) === "src");
    if (!src || !files.some((f) => BUILD_FILES.includes(f))) return false;
    const test = (await read(path.join(dir, src.name))).find(
      (e) => e.isDirectory() && lower(e) === "test",
    );
    return !!test && hasFeature(path.join(dir, src.name, test.name));
  };
  const walk = async () => {
    let level = [""];
    for (let depth = 0; depth <= limits.depth && level.length; depth++) {
      const next: string[] = [];
      for (const rel of level) {
        const dir = path.join(root, rel);
        const entries = await read(dir);
        if (depth > 0 && (await isProject(dir, entries))) {
          found.push(rel);
          if (found.length >= limits.found) return;
        } else if (depth < limits.depth) {
          // a build's own sources are that project's, not another one (#232 review)
          const build = entries.some((e) => e.isFile() && BUILD_FILES.includes(lower(e)));
          const below = folders(entries).filter((n) => !(build && n.toLowerCase() === "src"));
          next.push(...below.map((n) => (rel ? `${rel}/${n}` : n)));
        }
      }
      level = next;
    }
  };

  let timer: NodeJS.Timeout | undefined;
  const timeUp = new Promise<void>((resolve) => (timer = setTimeout(resolve, limits.timeoutMs)));
  try {
    await Promise.race([walk().catch(() => {}), timeUp]);
  } finally {
    done = true; // a walk still waiting on a read stops at its next one
    clearTimeout(timer);
  }
  return found.slice();
}
