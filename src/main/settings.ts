import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
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

export function saveProjectFilter(dataDir: string, root: string, f: FilterState): void {
  update(dataDir, (s) => {
    const projects = s.projects;
    s.projects = {
      ...(typeof projects === "object" && projects !== null ? projects : {}),
      [root]: f,
    };
  });
}
