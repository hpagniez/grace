import { httpGroup, type HttpCall, type HttpGroup } from "../shared/http";
import type { TestNode } from "../shared/model";
import { findNode, leaves } from "./nodes";
import type { AppState } from "./reduce";

/** A call of a test: the test's key and the call's place in that test's list. */
export interface CallRef {
  key: string;
  index: number;
}

export interface ListedCall {
  ref: CallRef;
  call: HttpCall;
  test: TestNode;
}

/** The tests under the tree selection (all of them with none). */
export function scopeTests(s: Pick<AppState, "tree" | "selected">): TestNode[] {
  if (!s.tree) return [];
  return leaves((s.selected && findNode(s.tree, s.selected)) || s.tree);
}

// One list per calls array: a test whose calls did not change keeps its rows, which are not drawn again.
const listed = new WeakMap<HttpCall[], ListedCall[]>();

/** The calls of these tests, in run order. */
export function callsOf(tests: TestNode[], calls: AppState["calls"]): ListedCall[] {
  return tests
    .flatMap((test) => {
      const list = calls[test.key] ?? [];
      let rows = listed.get(list);
      // a refresh keeps the calls of an unchanged test but gives it a new node
      if (!rows || rows[0]?.test !== test) {
        rows = list.map((call, index) => ({ ref: { key: test.key, index }, call, test }));
        listed.set(list, rows);
      }
      return rows;
    })
    .sort((a, b) => a.call.startTime - b.call.startTime); // stable: log order breaks ties
}

/** The calls of the tests under the tree selection (all of them with none), in run order. */
export const narrowedCalls = (s: Pick<AppState, "tree" | "selected" | "calls">): ListedCall[] =>
  callsOf(scopeTests(s), s.calls);

export function groupCounts(list: ListedCall[]): Record<HttpGroup | "all", number> {
  const counts = { all: list.length, "1xx": 0, "2xx": 0, "3xx": 0, "4xx": 0, "5xx": 0, none: 0 };
  for (const { call } of list) {
    const g = httpGroup(call.status);
    if (g) counts[g]++;
  }
  return counts;
}

/** The calls in any of the groups; with no group on, every call. */
export function shownCalls(list: ListedCall[], groups: HttpGroup[]): ListedCall[] {
  if (groups.length === 0) return list;
  return list.filter(({ call }) => {
    const g = httpGroup(call.status);
    return g !== null && groups.includes(g);
  });
}

export function toggledGroups(groups: HttpGroup[], g: HttpGroup | "all"): HttpGroup[] {
  if (g === "all") return [];
  return groups.includes(g) ? groups.filter((x) => x !== g) : [...groups, g];
}

export const totalCalls = (calls: Record<string, HttpCall[]>): number =>
  Object.values(calls).reduce((n, list) => n + list.length, 0);

/** The selected call with its test; null when it is not there any more. */
export function findCall(s: Pick<AppState, "tree" | "calls" | "selectedCall">): ListedCall | null {
  const ref = s.selectedCall;
  if (!ref || !s.tree) return null;
  const test = findNode(s.tree, ref.key);
  const call = s.calls[ref.key]?.[ref.index];
  return test && call ? { ref, call, test } : null;
}
