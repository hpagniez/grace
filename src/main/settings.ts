import { mkdirSync, readFileSync, renameSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { propertyNameError, propertyValueError, validEnvName } from "../shared/env";
import { NO_FILTER, parseFilterState, type FilterState } from "../shared/tags";
import { validThreads } from "../shared/threads";
import { defaultUpdateSettings, type UpdateSettings } from "../shared/updates";
import { parseVersion } from "../shared/version";

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

/**
 * Read-modify-write, so one key never drops the others. Written to a temp file, flushed, then renamed
 * over the old one: a save cut off by a crash or a power loss leaves the old file whole.
 */
function update(dataDir: string, change: (s: Record<string, unknown>) => void): void {
  const settings = read(dataDir);
  change(settings);
  mkdirSync(dataDir, { recursive: true });
  const tmp = `${file(dataDir)}.tmp`;
  writeFileSync(tmp, JSON.stringify(settings), { flush: true });
  renameSync(tmp, file(dataDir));
}

/** How many project folders the recent list keeps. */
export const RECENT_LIMIT = 10;

/** Two paths are the same folder when they resolve alike, ignoring case (Windows) and a trailing separator. */
const sameFolder = (a: string, b: string) =>
  path.resolve(a).toLowerCase() === path.resolve(b).toLowerCase();

/** The absolute paths of a saved list, newest first, one per folder, at most RECENT_LIMIT; anything else is dropped. */
function parseRecent(x: unknown): string[] {
  const kept: string[] = [];
  if (!Array.isArray(x)) return kept;
  for (const p of x) {
    if (kept.length === RECENT_LIMIT) break;
    if (typeof p === "string" && path.isAbsolute(p) && !kept.some((k) => sameFolder(k, p)))
      kept.push(p);
  }
  return kept;
}

/** The recent project folders, newest first, as saved (they may no longer exist). `[]` if none or the file cannot be read. Never throws. */
export function loadRecentProjects(dataDir: string): string[] {
  try {
    return parseRecent(read(dataDir).recentProjects);
  } catch {
    return [];
  }
}

/** Saves `root` as the last project and moves it to the top of the recent list. Throws if the file cannot be saved. */
export function saveOpenedProject(dataDir: string, root: string): void {
  update(dataDir, (s) => {
    s.lastProject = root;
    s.recentProjects = [
      root,
      ...parseRecent(s.recentProjects).filter((p) => !sameFolder(p, root)),
    ].slice(0, RECENT_LIMIT);
  });
}

/** Drops `root` from the recent list, and from lastProject if that is the same folder. Throws if the file cannot be saved. */
export function removeRecentProject(dataDir: string, root: string): void {
  update(dataDir, (s) => {
    s.recentProjects = parseRecent(s.recentProjects).filter((p) => p !== root);
    if (typeof s.lastProject === "string" && sameFolder(s.lastProject, root)) delete s.lastProject;
  });
}

/** Forgets the last project, so the next start shows the home screen. Throws if the file cannot be saved. */
export function forgetLastProject(dataDir: string): void {
  update(dataDir, (s) => {
    delete s.lastProject;
  });
}

/** The window's normal-state bounds and whether it was maximized. */
export interface WindowState {
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
}

/** The saved window state, or null if none is saved, it is malformed or the file cannot be read. Never throws. */
export function loadWindowState(dataDir: string): WindowState | null {
  try {
    const { window: w } = read(dataDir);
    if (!isObject(w)) return null;
    const { x, y, width, height, maximized } = w;
    const finite = (n: unknown): n is number => typeof n === "number" && Number.isFinite(n);
    return finite(x) &&
      finite(y) &&
      finite(width) &&
      finite(height) &&
      width > 0 &&
      height > 0 &&
      typeof maximized === "boolean"
      ? { x, y, width, height, maximized }
      : null;
  } catch {
    return null;
  }
}

export function saveWindowState(
  dataDir: string,
  { x, y, width, height, maximized }: WindowState,
): void {
  update(dataDir, (s) => {
    s.window = { x, y, width, height, maximized };
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

/** The saved thread count of a project; 1 if none is saved, it is malformed or the file cannot be read. Never throws. */
export function loadProjectThreads(dataDir: string, root: string): number {
  try {
    const entry = savedEntry(dataDir, root);
    const threads = isObject(entry) ? entry.threads : null;
    return validThreads(threads) ? threads : 1;
  } catch {
    return 1;
  }
}

export function saveProjectThreads(dataDir: string, root: string, n: number): void {
  updateEntry(dataDir, root, (e) => {
    e.threads = n;
  });
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

/**
 * The saved update settings (R12). Each malformed or missing field reads as its default; `running`
 * sets the default channel. A `skipped` or `failed` that is not a valid version reads as null, as
 * `compareVersions` throws on one. An unreadable file reads as the defaults, so it never throws.
 */
export function loadUpdateSettings(dataDir: string, running: string): UpdateSettings {
  const d = defaultUpdateSettings(running);
  let u: Record<string, unknown>;
  try {
    const { updates } = read(dataDir);
    u = isObject(updates) ? updates : {};
  } catch {
    return d;
  }
  const bool = (x: unknown, dflt: boolean) => (typeof x === "boolean" ? x : dflt);
  const version = (x: unknown) => (typeof x === "string" && parseVersion(x) ? x : null);
  return {
    channel: u.channel === "stable" || u.channel === "prerelease" ? u.channel : d.channel,
    checkAtStartup: bool(u.checkAtStartup, d.checkAtStartup),
    autoUpdate: bool(u.autoUpdate, d.autoUpdate),
    skipped: version(u.skipped),
    failed: version(u.failed),
    lastChecked:
      typeof u.lastChecked === "string" && !Number.isNaN(Date.parse(u.lastChecked))
        ? u.lastChecked
        : null,
  };
}

/** Saves the update settings and keeps the other keys. Throws if the file cannot be saved. */
export function saveUpdateSettings(dataDir: string, s: UpdateSettings): void {
  update(dataDir, (all) => {
    all.updates = {
      channel: s.channel,
      checkAtStartup: s.checkAtStartup,
      autoUpdate: s.autoUpdate,
      skipped: s.skipped,
      failed: s.failed,
      lastChecked: s.lastChecked,
    };
  });
}
