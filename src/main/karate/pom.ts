import { readFileSync } from "node:fs";
import path from "node:path";

/** What Grace takes from a project's `karate-pom.json` (docs/specs/2026-11-02-project-setup.md R1). */
export interface Pom {
  /** As written; null when absent or empty. */
  paths: string[] | null;
  /** The `-g` value, as written. */
  configDir: string | null;
  /** The other keys, in the file's order: Grace does not use them. */
  unused: string[];
  /** Why the file could not be read; the other fields are then empty. */
  error: string | null;
}

export const NO_POM: Pom = { paths: null, configDir: null, unused: [], error: null };

const broken = (error: string): Pom => ({ ...NO_POM, error });
const strings = (v: unknown): v is string[] =>
  Array.isArray(v) && v.every((s) => typeof s === "string");

/** `<root>/karate-pom.json`, or `NO_POM` when there is none. Never throws. */
export function readPom(root: string): Pom {
  let json: unknown;
  try {
    json = JSON.parse(readFileSync(path.join(root, "karate-pom.json"), "utf8"));
  } catch (err) {
    return (err as NodeJS.ErrnoException).code === "ENOENT"
      ? NO_POM
      : broken((err as Error).message);
  }
  if (typeof json !== "object" || json === null || Array.isArray(json))
    return broken("not a JSON object");
  const { paths, configDir, ...rest } = json as Record<string, unknown>;
  if (paths !== undefined && !strings(paths)) return broken("paths is not a list of strings");
  if (configDir !== undefined && typeof configDir !== "string")
    return broken("configDir is not a string");
  return {
    paths: paths?.length ? paths : null,
    configDir: configDir ?? null,
    unused: Object.keys(rest),
    error: null,
  };
}
