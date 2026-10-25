import { readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { envTagValues, sortedUnique, validEnvName } from "../../shared/env";

const SKIPPED_DIRS = new Set(["node_modules", "target", "build", ".git"]);

/** `<env>` of every `karate-config-<env>.js` directly inside each folder. */
export function overlayEnvs(dirs: string[]): string[] {
  return sortedUnique(
    dirs.map((dir) => {
      try {
        return readdirSync(dir).flatMap((file) => {
          const env = /^karate-config-(.+)\.js$/.exec(file)?.[1];
          return env !== undefined && validEnvName(env) ? [env] : [];
        });
      } catch {
        return []; // folder missing or unreadable: no suggestions from it
      }
    }),
  );
}

function featureFiles(dir: string): string[] {
  let entries;
  try {
    entries = readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
  return entries.flatMap((e) =>
    e.isDirectory()
      ? SKIPPED_DIRS.has(e.name)
        ? []
        : featureFiles(path.join(dir, e.name))
      : e.name.endsWith(".feature")
        ? [path.join(dir, e.name)]
        : [],
  );
}

/** Tag lines are lines whose first token starts with `@`; `# @env=x` comments are not tag lines. */
function fileTagEnvs(file: string): string[] {
  let text;
  try {
    text = readFileSync(file, "utf8");
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

/** Environment names written in `@env=`/`@envnot=` tags of the feature files under the roots. */
export const featureTagEnvs = (featureRoots: string[]): string[] =>
  sortedUnique(featureRoots.flatMap(featureFiles).map(fileTagEnvs));
