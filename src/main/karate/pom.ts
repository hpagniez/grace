import { readFileSync } from "node:fs";
import path from "node:path";
import { validEnvName } from "../../shared/env";
import { filterable, parseFilterState, type TagFilter } from "../../shared/tags";
import { validThreads } from "../../shared/threads";
import { mavenEnv } from "./maven";

/** What Grace takes from a project's `karate-pom.json` (docs/specs/2026-11-02-project-setup.md R1). */
export interface Pom {
  /** As written; null when absent or empty. */
  paths: string[] | null;
  /** The `-g` value, as written. */
  configDir: string | null;
  /** `output.junitXml`: Karate also writes JUnit XML (docs/specs/2026-11-03-export-reports.md R1). */
  junitXml: boolean;
  /** `env` (docs/specs/2026-11-04-pom-defaults.md R2.1): undefined when the file gives none, so pom.xml's may; null when given but unused. */
  env: string | null | undefined;
  /** The `tags` entries the filter can show (R2.3); null when none. */
  filter: TagFilter | null;
  /** `threads` when Grace can take it (R2.2). */
  threads: number | null;
  /** The other keys, in the file's order, `output`'s as `output.<key>`: Grace does not use them; an unused `env`, `tags` entry or `threads` as `<key> <JSON>`. */
  unused: string[];
  /** Why the file could not be read; the other fields are then empty. */
  error: string | null;
}

export const NO_POM: Pom = {
  paths: null,
  configDir: null,
  junitXml: false,
  env: undefined,
  filter: null,
  threads: null,
  unused: [],
  error: null,
};

const broken = (error: string): Pom => ({ ...NO_POM, error });
const strings = (v: unknown): v is string[] =>
  Array.isArray(v) && v.every((s) => typeof s === "string");
const object = (v: unknown): v is Record<string, unknown> =>
  typeof v === "object" && v !== null && !Array.isArray(v);

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
  if (!object(json)) return broken("not a JSON object");
  const { paths, configDir } = json;
  if (paths !== undefined && !strings(paths)) return broken("paths is not a list of strings");
  if (configDir !== undefined && typeof configDir !== "string")
    return broken("configDir is not a string");
  // an output that is not an object turns nothing on and is named whole
  const output = object(json.output) ? json.output : null;
  const junitXml = output?.junitXml ?? false;
  if (typeof junitXml !== "boolean") return broken("output.junitXml is not true or false");
  const env = json.env === "" ? undefined : json.env;
  const usedEnv = typeof env === "string" && validEnvName(env) ? env : null;
  const threads = validThreads(json.threads) ? json.threads : null;
  const tags = readTags(json.tags);
  const value = (k: string, v: unknown) => `${k} ${JSON.stringify(v)}`;
  // the keys Grace takes a starting value from: named only with the values it cannot take (R2.4)
  const own: Record<string, string[]> = {
    env: env === undefined || usedEnv !== null ? [] : [value("env", env)],
    threads: threads !== null ? [] : [value("threads", json.threads)],
    tags: tags.unused.map((t) => value("tags", t)),
  };
  const named = (k: string): string[] =>
    k === "output" && output
      ? Object.keys(output).flatMap((o) => (o === "junitXml" ? [] : [`output.${o}`]))
      : Object.hasOwn(own, k)
        ? own[k]
        : [k];
  return {
    paths: paths?.length ? paths : null,
    configDir: configDir ?? null,
    junitXml,
    env: env === undefined ? undefined : usedEnv,
    filter: tags.filter,
    threads,
    unused: Object.keys(json)
      .filter((k) => k !== "paths" && k !== "configDir")
      .flatMap(named),
    error: null,
  };
}

/** R2.3: `~@t` excludes, the first `@a,@b` entry includes; every other entry is unused, as written. */
function readTags(tags: unknown): { filter: TagFilter | null; unused: unknown[] } {
  if (tags === undefined) return { filter: null, unused: [] };
  if (!strings(tags)) return { filter: null, unused: [tags] };
  const include: string[] = [];
  const exclude: string[] = [];
  const unused: string[] = [];
  for (const entry of tags) {
    const parts = entry.split(",").map((p) => p.trim());
    const out = parts.length === 1 && parts[0].startsWith("~@") ? exclude : include;
    const other = out === exclude ? include : exclude;
    const names = parts.map((p) => p.slice(out === exclude ? 2 : 1));
    const ok =
      (out === exclude || (include.length === 0 && parts.every((p) => p.startsWith("@")))) &&
      names.every((t) => filterable(t) && !other.includes(t));
    if (ok) out.push(...names.filter((t, i) => !out.includes(t) && names.indexOf(t) === i));
    else unused.push(entry);
  }
  const filter = { include, exclude };
  if (!parseFilterState({ ...filter, hideNonMatching: false }))
    return { filter: null, unused: [tags] };
  return { filter: include.length || exclude.length ? filter : null, unused };
}

/** The starting values the project's pom files give (R2, R3): pom.xml's `karate.env` when karate-pom.json gives no `env`. */
export interface PomStart {
  env: string | null;
  filter: TagFilter | null;
  threads: number | null;
}

export function pomStart(root: string): PomStart {
  const { env, filter, threads } = readPom(root);
  return { env: env === undefined ? mavenEnv(root) : env, filter, threads };
}
