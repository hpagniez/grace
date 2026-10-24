import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";

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

export function saveLastProject(dataDir: string, root: string): void {
  mkdirSync(dataDir, { recursive: true });
  writeFileSync(file(dataDir), JSON.stringify({ lastProject: root }));
}
