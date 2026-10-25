import { readdirSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { envTagValues, sortedUnique, validEnvName } from "../../shared/env";

const SKIPPED_DIRS = new Set(["node_modules", "target", "build", ".git"]);

/**
 * `<env>` of every `karate-config-<env>.js` directly inside each folder, as written. Case-insensitive:
 * on Windows Karate loads `Karate-Config-QA.js` for `-e QA`.
 */
export function overlayEnvs(dirs: string[]): string[] {
  return sortedUnique(
    dirs.map((dir) => {
      try {
        return readdirSync(dir).flatMap((file) => {
          const env = /^karate-config-(.+)\.js$/i.exec(file)?.[1];
          return env !== undefined && validEnvName(env) ? [env] : [];
        });
      } catch {
        return []; // folder missing or unreadable: no suggestions from it
      }
    }),
  );
}

async function featureFiles(dir: string): Promise<string[]> {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  const files = await Promise.all(
    entries.map((e) =>
      e.isDirectory()
        ? SKIPPED_DIRS.has(e.name)
          ? []
          : featureFiles(path.join(dir, e.name))
        : e.name.endsWith(".feature")
          ? [path.join(dir, e.name)]
          : [],
    ),
  );
  return files.flat();
}

/** Tag lines are lines whose first token starts with `@`; `# @env=x` comments are not tag lines. */
async function fileTagEnvs(file: string): Promise<string[]> {
  let text;
  try {
    text = await readFile(file, "utf8");
  } catch {
    return [];
  }
  const tags = text.split("\n").flatMap((line) => {
    const tokens = line.trim().split(/\s+/);
    return tokens[0].startsWith("@")
      ? tokens.filter((t) => t.startsWith("@")).map((t) => t.slice(1))
      : [];
  });
  return envTagValues(tags);
}

const MAX_OPEN_FILES = 64;

/**
 * Environment names written in `@env=`/`@envnot=` tags of the feature files under the roots. Async,
 * so a large tree does not freeze the window; it never rejects.
 */
export async function featureTagEnvs(featureRoots: string[]): Promise<string[]> {
  const files = (await Promise.all(featureRoots.map(featureFiles))).flat();
  const envs: string[][] = [];
  // A few readers, not one Promise.all: that opens every file before reading any (EMFILE).
  const reader = async () => {
    for (let file; (file = files.pop()) !== undefined;) envs.push(await fileTagEnvs(file));
  };
  await Promise.all(Array.from({ length: MAX_OPEN_FILES }, reader));
  return sortedUnique(envs);
}
