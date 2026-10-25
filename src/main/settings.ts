import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { propertyNameError, propertyValueError, validEnvName } from "../shared/env";
import { NO_FILTER, parseFilterState, type FilterState } from "../shared/tags";

const file = (dataDir: string) => path.join(dataDir, "settings.json");

/** The last opened project folder, or null if none is saved, the file is unreadable or the folder is gone. */
export function loadLastProject(dataDir: string): string | null {
  try {
    const { lastProject } = JSON.parse(readFileSync(file(dataDir), "utf8"));
    return typeof lastProject === "string" && statSync(lastProject).isDirectory()
      ? lastProject
      : null;
  } catch {
    return null;
  }
}

/**
 * The whole settings file; a missing, non-JSON or non-object file reads as empty. Any other read
 * error throws, so a save never overwrites a file it could not read (a transient EBUSY, say).
 */
function read(dataDir: string): Record<string, unknown> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(readFileSync(file(dataDir), "utf8"));
  } catch (err) {
    if (err instanceof SyntaxError || (err as NodeJS.ErrnoException).code === "ENOENT") return {};
    throw err;
  }
  return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
    ? (parsed as Record<string, unknown>)
    : {};
}

/** Read-modify-write, so one key never drops the others. */
function update(dataDir: string, change: (s: Record<string, unknown>) => void): void {
  const settings = read(dataDir);
  change(settings);
  mkdirSync(dataDir, { recursive: true });
  writeFileSync(file(dataDir), JSON.stringify(settings));
}

export function saveLastProject(dataDir: string, root: string): void {
  update(dataDir, (s) => {
    s.lastProject = root;
  });
}

/**
 * The saved tag filter of a project, or no filter if none is saved, the entry is malformed or the
 * file cannot be read. Never throws.
 */
export function loadProjectFilter(dataDir: string, root: string): FilterState {
  try {
    const { projects } = read(dataDir);
    const entry =
      typeof projects === "object" && projects !== null
        ? parseFilterState((projects as Record<string, unknown>)[root])
        : null;
    if (entry) return entry;
  } catch {
    // unreadable: no filter
  }
  return { ...NO_FILTER, include: [], exclude: [] };
}

const isObject = (x: unknown): x is Record<string, unknown> =>
  typeof x === "object" && x !== null && !Array.isArray(x);

/** Changes one project's entry and keeps its other fields; an entry that is not an object starts fresh. */
function updateEntry(
  dataDir: string,
  root: string,
  change: (entry: Record<string, unknown>) => void,
): void {
  update(dataDir, (s) => {
    const projects = isObject(s.projects) ? s.projects : {};
    const old = Object.hasOwn(projects, root) ? projects[root] : null;
    const entry = isObject(old) ? { ...old } : {};
    change(entry);
    s.projects = { ...projects, [root]: entry };
  });
}

export function saveProjectFilter(dataDir: string, root: string, f: FilterState): void {
  updateEntry(dataDir, root, (e) => {
    e.include = f.include;
    e.exclude = f.exclude;
    e.hideNonMatching = f.hideNonMatching;
  });
}

/** A saved property: a plain value, or a secret as base64 ciphertext. */
export type StoredRow = { name: string; value: string } | { name: string; secret: string };

export interface StoredLists {
  shared: StoredRow[];
  byEnv: Record<string, StoredRow[]>;
}

export interface EnvSettings {
  env: string | null;
  properties: StoredLists;
}

export const emptyEnvSettings = (): EnvSettings => ({
  env: null,
  properties: { shared: [], byEnv: {} },
});

/** A row needs a valid name and exactly one of a plain value that Java can take or a non-empty secret. */
function parseStoredRow(x: unknown): StoredRow | null {
  if (!isObject(x)) return null;
  const { name, value, secret } = x;
  if (typeof name !== "string" || propertyNameError(name)) return null;
  if (typeof value === "string" && secret === undefined)
    return propertyValueError(value) ? null : { name, value };
  if (typeof secret === "string" && secret !== "" && value === undefined) return { name, secret };
  return null;
}

// Array.from, not map: it turns the holes of a sparse array into dropped rows.
const parseStoredRows = (x: unknown): StoredRow[] =>
  Array.isArray(x) ? Array.from(x, parseStoredRow).filter((r) => r !== null) : [];

function parseEnvSettings(entry: unknown): EnvSettings {
  const out = emptyEnvSettings();
  if (!isObject(entry)) return out;
  const { env, properties } = entry;
  if (typeof env === "string" && validEnvName(env)) out.env = env;
  if (!isObject(properties)) return out;
  out.properties.shared = parseStoredRows(properties.shared);
  if (isObject(properties.byEnv))
    for (const [name, rows] of Object.entries(properties.byEnv))
      if (validEnvName(name) && Array.isArray(rows))
        out.properties.byEnv[name] = parseStoredRows(rows);
  return out;
}

/** The saved entry of a project, as written; null if none. Throws if the file cannot be read. */
function savedEntry(dataDir: string, root: string): unknown {
  const { projects } = read(dataDir);
  return isObject(projects) && Object.hasOwn(projects, root) ? projects[root] : null;
}

/**
 * The saved environment and run properties of a project. Malformed parts read as missing and an
 * unreadable file as no environment, so it never throws.
 */
export function loadProjectEnv(dataDir: string, root: string): EnvSettings {
  try {
    return parseEnvSettings(savedEntry(dataDir, root));
  } catch {
    return emptyEnvSettings();
  }
}

/** The rows of a saved list that the rules of today reject, as written. */
const rejectedRows = (x: unknown): unknown[] =>
  Array.isArray(x) ? x.filter((r) => parseStoredRow(r) === null) : [];

/**
 * `<scope>: <name>` of each rejected row in a list that loads (`(no name)` if it has none), never
 * its value: it may be a secret. Never throws.
 */
export function skippedProperties(dataDir: string, root: string): string[] {
  try {
    const entry = savedEntry(dataDir, root);
    const properties = isObject(entry) && isObject(entry.properties) ? entry.properties : {};
    const byEnv = isObject(properties.byEnv) ? properties.byEnv : {};
    const named = (scope: string, rows: unknown) =>
      rejectedRows(rows).map(
        (r) =>
          `${scope}: ${isObject(r) && typeof r.name === "string" && r.name ? r.name : "(no name)"}`,
      );
    return [
      ...named("All environments", properties.shared),
      ...Object.entries(byEnv).flatMap(([env, rows]) =>
        validEnvName(env) ? named(env, rows) : [],
      ),
    ];
  } catch {
    return [];
  }
}

/**
 * Saves the environment and the lists. The rows of the file that the rules of today reject stay after
 * the new rows of their list while that list exists, so a newer rule never erases them.
 */
export function saveProjectEnv(dataDir: string, root: string, s: EnvSettings): void {
  updateEntry(dataDir, root, (e) => {
    if (s.env === null) delete e.env;
    else e.env = s.env;
    const old = isObject(e.properties) ? e.properties : {};
    const oldByEnv = isObject(old.byEnv) ? old.byEnv : {};
    e.properties = {
      shared: [...s.properties.shared, ...rejectedRows(old.shared)],
      byEnv: Object.fromEntries(
        Object.entries(s.properties.byEnv).map(([env, rows]) => [
          env,
          [...rows, ...rejectedRows(Object.hasOwn(oldByEnv, env) ? oldByEnv[env] : null)],
        ]),
      ),
    };
  });
}
