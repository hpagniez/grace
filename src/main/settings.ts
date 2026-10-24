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

/** The whole settings file; an unreadable or non-object file reads as empty. */
function read(dataDir: string): Record<string, unknown> {
  try {
    const parsed = JSON.parse(readFileSync(file(dataDir), "utf8"));
    return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed) ? parsed : {};
  } catch {
    return {};
  }
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

/** The saved tag filter of a project, or no filter if none is saved or the entry is malformed. */
export function loadProjectFilter(dataDir: string, root: string): FilterState {
  const { projects } = read(dataDir);
  const entry =
    typeof projects === "object" && projects !== null
      ? parseFilterState((projects as Record<string, unknown>)[root])
      : null;
  return entry ?? { ...NO_FILTER, include: [], exclude: [] };
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
