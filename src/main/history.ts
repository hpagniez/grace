// Run history on disk: `<dataDir>/projects/<hash>/runs/<id>/` holds Karate's output and our `run.json`.
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import {
  HISTORY_LIMIT,
  parseRunRecord,
  RUN_ID,
  toEntry,
  type HistoryEntry,
  type RunRecord,
} from "../shared/history";

/** About 200 bytes per test: room for 300k tests, still a bound on a hand-made file. */
const MAX_RECORD_BYTES = 64 * 1024 * 1024;

const runsDir = (projectDir: string) => path.join(projectDir, "runs");

/** The run folder names of a project in name order (oldest first); other names are not ours. Never throws. */
function runIds(projectDir: string): string[] {
  try {
    return readdirSync(runsDir(projectDir))
      .filter((n) => RUN_ID.test(n))
      .sort();
  } catch {
    return [];
  }
}

/**
 * Windows antivirus can hold a file a moment, hence the retries a person who asked for the deletion
 * waits for; a background cleanup passes 0. False if the folder stayed.
 */
function removeDir(dir: string, maxRetries = 5): boolean {
  try {
    rmSync(dir, { recursive: true, force: true, maxRetries });
    return true;
  } catch {
    return false;
  }
}

/** Local start time `yyyyMMdd-HHmmss-SSS`; `-2`, `-3`… is appended while `taken`. */
export function runId(now: Date, taken: (id: string) => boolean): string {
  const p = (n: number, width = 2) => String(n).padStart(width, "0");
  const base =
    `${p(now.getFullYear(), 4)}${p(now.getMonth() + 1)}${p(now.getDate())}` +
    `-${p(now.getHours())}${p(now.getMinutes())}${p(now.getSeconds())}-${p(now.getMilliseconds(), 3)}`;
  let id = base;
  for (let n = 2; taken(id); n++) id = `${base}-${n}`;
  return id;
}

export function createRunDir(projectDir: string, now: Date): { id: string; dir: string } {
  const runs = runsDir(projectDir);
  const id = runId(now, (candidate) => existsSync(path.join(runs, candidate)));
  const dir = path.join(runs, id);
  mkdirSync(dir, { recursive: true });
  return { id, dir };
}

export function writeRunRecord(dir: string, r: RunRecord): void {
  writeFileSync(path.join(dir, "run.json"), JSON.stringify(r));
}

/** Null for an id that is not a run id, a missing folder or file, a file over 64 MB or an invalid record. */
export function readRunRecord(projectDir: string, id: string): RunRecord | null {
  if (!RUN_ID.test(id)) return null; // also keeps `..` and separators out of the path
  try {
    const file = path.join(runsDir(projectDir), id, "run.json");
    if (statSync(file).size > MAX_RECORD_BYTES) return null;
    return parseRunRecord(JSON.parse(readFileSync(file, "utf8")));
  } catch {
    return null;
  }
}

/** The valid runs, newest `startedAt` first, at most 20. Never throws. */
export function listHistory(projectDir: string): HistoryEntry[] {
  return runIds(projectDir)
    .flatMap((id) => {
      const r = readRunRecord(projectDir, id);
      const report = path.join(runsDir(projectDir), id, "karate-summary.html");
      return r ? [toEntry(id, r, existsSync(report))] : [];
    })
    .sort((a, b) => b.startedAt - a.startedAt)
    .slice(0, HISTORY_LIMIT);
}

/**
 * Keeps the 20 newest folders by name, valid or not. A folder that stays is tried again after the next
 * run; no retries here, they would block the main process.
 */
export function pruneHistory(projectDir: string): void {
  for (const id of runIds(projectDir).slice(0, -HISTORY_LIMIT))
    removeDir(path.join(runsDir(projectDir), id), 0);
}

/**
 * Deletes a run folder; false if it stayed. A recursive delete removes the files before it fails, and
 * the `run.json` with them, which would drop the run from the list: put the record back.
 */
function removeRun(dir: string): boolean {
  const file = path.join(dir, "run.json");
  let record: Buffer | null = null;
  try {
    record = readFileSync(file);
  } catch {
    // no record to keep
  }
  if (removeDir(dir)) return true;
  try {
    if (record && existsSync(dir)) writeFileSync(file, record);
  } catch {
    // the folder is then unlisted and removeLeftoverRuns deletes it at the next start
  }
  return false;
}

/** Deletes every run folder; false if any stayed, and those stay listed. */
export function clearHistory(projectDir: string): boolean {
  let ok = true;
  for (const id of runIds(projectDir)) ok = removeRun(path.join(runsDir(projectDir), id)) && ok;
  return ok;
}

/**
 * At app start: under each `<dataDir>/projects/<hash>/`, deletes run folders without `run.json` (the
 * app crashed during that run) and the `run/` folder of version 1.0. Never throws.
 */
export function removeLeftoverRuns(dataDir: string): void {
  const projects = path.join(dataDir, "projects");
  let hashes: string[];
  try {
    hashes = readdirSync(projects);
  } catch {
    return;
  }
  for (const hash of hashes) {
    const projectDir = path.join(projects, hash);
    removeDir(path.join(projectDir, "run"));
    for (const id of runIds(projectDir)) {
      const dir = path.join(runsDir(projectDir), id);
      if (!existsSync(path.join(dir, "run.json"))) removeDir(dir);
    }
  }
}
