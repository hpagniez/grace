// Failed tests that share an error, and a failed karate-config.js (#229).
import type { Status, TestNode } from "../shared/model";
import { filterOn, matchingLeaves, type FilterState } from "../shared/tags";
import { leaves } from "./nodes";
import type { AppState } from "./reduce";

export type Result = AppState["results"][string];

/** Karate's reason: the error from where it starts (R2.1); null without an error. */
export const reasonOf = (r: Result | undefined): string | null =>
  r?.error === undefined ? null : r.error.slice(r.reasonAt ?? 0);

/** What two reasons are compared by: the reason less a JS failure's position, which is its place (R2.2). */
const sameError = (reason: string) =>
  reason
    .replace(/^[ \t]*Line: \d+, Col: \d+\r?(\n|$)/gm, "")
    .replace(/\r?\n[ \t]*at \d+:\d+\s*$/, "");

/** The tests of `n` the tree shows, in tree order: only the matching ones under "Hide non-matching" (R3.1). */
export const shownLeaves = (n: TestNode, f: FilterState): TestNode[] =>
  f.hideNonMatching && filterOn(f) ? matchingLeaves(n, f) : leaves(n);

export interface FailureGroup {
  /** In tree order. */
  keys: string[];
}

/** The failed tests among `tests` (in tree order) that share an error, 2 or more each, in the order of their first tests (R3.1). */
export function failureGroups(
  tests: TestNode[],
  statuses: Record<string, Status>,
  results: Record<string, Result>,
): FailureGroup[] {
  const by = new Map<string, string[]>();
  for (const { key } of tests) {
    const reason = statuses[key] === "failed" ? reasonOf(results[key]) : null;
    if (reason === null) continue;
    const id = sameError(reason);
    const keys = by.get(id);
    if (keys) keys.push(key);
    else by.set(id, [key]);
  }
  return [...by.values()].filter((keys) => keys.length > 1).map((keys) => ({ keys }));
}

export interface ConfigFailure {
  /** As Karate names it, without `classpath:`. */
  file: string;
  line: number;
  reason: string;
  count: number;
}

const CONFIG = "Config evaluation failed: ";

/** The config failure among the last run's failed tests (R6): the first in tree order, counting them all. */
export function configFailure(
  tests: TestNode[],
  failedKeys: readonly string[],
  results: Record<string, Result>,
): ConfigFailure | null {
  const failed = new Set(failedKeys);
  let first: string | null = null;
  let count = 0;
  for (const { key } of tests) {
    const reason = failed.has(key) ? reasonOf(results[key]) : null;
    if (!reason?.startsWith(CONFIG)) continue;
    first ??= reason;
    count++;
  }
  if (first === null) return null;
  const head = first.slice(CONFIG.length).split("\n", 1)[0];
  const dash = head.indexOf(" - ");
  return {
    file: (dash < 0 ? head : head.slice(0, dash)).replace(/^classpath:/, ""),
    line: Number(/Line: (\d+)/.exec(first)?.[1] ?? 1),
    reason: first,
    count,
  };
}
