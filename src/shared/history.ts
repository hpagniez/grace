// Shared by main and renderer: keep this file free of Node imports.
// What a finished run stores and how the History tab words it (docs/specs/2026-10-25-run-history.md).
import { validEnvName } from "./env";
import type { RunOutcome } from "./events";
import type { NodeKind } from "./model";
import { parseFilterState, type FilterState } from "./tags";

export const HISTORY_LIMIT = 20;

/** The folder name of a run: local start time `yyyyMMdd-HHmmss-SSS`, plus `-2`, `-3`… if taken. */
export const RUN_ID = /^\d{8}-\d{6}-\d{3}(-\d+)?$/;

export type TestStatusFinal = "passed" | "failed" | "notRun" | "interrupted";

export interface StoredTest {
  kind: "scenario" | "row";
  file: string;
  refId: string;
  name: string;
  line: number | null;
  key: string;
  status: TestStatusFinal;
  durationMillis?: number;
}

export interface StoredSelection {
  kind: NodeKind;
  file: string;
  name: string;
  line: number | null;
  key: string;
}

/** `run.json`. Never holds a property value, an error text, a step log or a console line. */
export interface RunRecord {
  version: 1;
  startedAt: number;
  durationMillis: number;
  env: string | null;
  /** The count passed with `-T`; null for a run stored before F12. */
  threads: number | null;
  filter: FilterState;
  /** Null for a Re-run failed, whose `failedOf` is then the `startedAt` of its source run. */
  selection: StoredSelection | null;
  failedOf: number | null;
  outcome: RunOutcome;
  exitCode: number | null;
  tests: StoredTest[];
}

/** A listed run as the window gets it: the record without `tests`, with counts and its folder id. */
export interface HistoryEntry {
  id: string;
  startedAt: number;
  durationMillis: number;
  env: string | null;
  threads: number | null;
  filter: FilterState;
  selection: StoredSelection | null;
  failedOf: number | null;
  outcome: RunOutcome;
  exitCode: number | null;
  passed: number;
  failed: number;
  total: number;
  reportAvailable: boolean;
}

const NODE_KINDS: readonly string[] = ["folder", "feature", "outline", "scenario", "row"];
const STATUSES: readonly string[] = ["passed", "failed", "notRun", "interrupted"];
const OUTCOMES: readonly string[] = ["finished", "cancelled", "failed"];

const isRecord = (x: unknown): x is Record<string, unknown> => typeof x === "object" && x !== null;
const num = (x: unknown): x is number => typeof x === "number" && Number.isFinite(x);
const numOrNull = (x: unknown): x is number | null => x === null || num(x);
const str = (x: unknown): x is string => typeof x === "string";

function parseTest(x: unknown): StoredTest | null {
  if (!isRecord(x)) return null;
  const { kind, file, refId, name, line, key, status, durationMillis } = x;
  if (kind !== "scenario" && kind !== "row") return null;
  if (!str(file) || !str(refId) || !str(name) || !numOrNull(line) || !str(key)) return null;
  if (!str(status) || !STATUSES.includes(status)) return null;
  if (durationMillis !== undefined && !num(durationMillis)) return null;
  const t: StoredTest = { kind, file, refId, name, line, key, status: status as TestStatusFinal };
  if (durationMillis !== undefined) t.durationMillis = durationMillis;
  return t;
}

function parseSelection(x: unknown): StoredSelection | null {
  if (!isRecord(x)) return null;
  const { kind, file, name, line, key } = x;
  if (!str(kind) || !NODE_KINDS.includes(kind)) return null;
  if (!str(file) || !str(name) || !numOrNull(line) || !str(key)) return null;
  return { kind: kind as NodeKind, file, name, line, key };
}

/** Validates a `run.json` read from disk; null for anything that is not a version 1 record. */
export function parseRunRecord(x: unknown): RunRecord | null {
  if (!isRecord(x) || x.version !== 1) return null;
  const { startedAt, durationMillis, env, selection, failedOf, outcome, exitCode, tests } = x;
  if (!num(startedAt) || !num(durationMillis) || !numOrNull(failedOf) || !numOrNull(exitCode))
    return null;
  if (env !== null && !(str(env) && validEnvName(env))) return null;
  const threads = x.threads ?? null; // a record stored before F12 has none
  if (
    threads !== null &&
    !(typeof threads === "number" && Number.isInteger(threads) && threads >= 1)
  )
    return null;
  if (!str(outcome) || !OUTCOMES.includes(outcome)) return null;
  const filter = parseFilterState(x.filter);
  if (!filter) return null;
  const sel = selection === null ? null : parseSelection(selection);
  if (selection !== null && !sel) return null;
  if (!Array.isArray(tests)) return null;
  const parsed = Array.from(tests, parseTest); // not tests.map: it skips holes in a sparse array
  if (!parsed.every((t) => t !== null)) return null;
  return {
    version: 1,
    startedAt,
    durationMillis,
    env,
    threads,
    filter,
    selection: sel,
    failedOf,
    outcome: outcome as RunOutcome,
    exitCode,
    tests: parsed,
  };
}

export function toEntry(id: string, r: RunRecord, reportAvailable: boolean): HistoryEntry {
  const count = (s: TestStatusFinal) => r.tests.filter((t) => t.status === s).length;
  const { tests, version: _version, ...rest } = r;
  return {
    id,
    ...rest,
    passed: count("passed"),
    failed: count("failed"),
    total: tests.length,
    reportAvailable,
  };
}

const pad = (n: number) => String(n).padStart(2, "0");
const day = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** "Today 14:32", "Yesterday 17:20", else "2026-10-21 09:05", in local time. */
export function whenLabel(startedAt: number, now: number): string {
  const d = new Date(startedAt);
  const today = new Date(now);
  const yesterday = new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1);
  const date = day(d);
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  if (date === day(today)) return `Today ${time}`;
  return `${date === day(yesterday) ? "Yesterday" : date} ${time}`;
}

export function whatRanLabel(e: HistoryEntry, now: number): string {
  const s = e.selection;
  if (!s)
    return e.failedOf === null ? "Failed tests" : `Failed tests of ${whenLabel(e.failedOf, now)}`;
  if (s.kind === "folder") return s.file === "" ? "All" : s.file;
  return s.kind === "feature" ? s.file : s.name;
}

export function resultLabel(e: HistoryEntry): string {
  if (e.outcome === "failed") return e.exitCode === null ? "Error" : `Error (exit ${e.exitCode})`;
  const counts = `${e.passed} passed, ${e.failed} failed of ${e.total}`;
  return e.outcome === "cancelled" ? `Cancelled, ${counts}` : counts;
}
