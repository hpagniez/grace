// The words of the trend strip (docs/specs/2026-11-04-trends.md R1.3, R3.4, R4.1).
import { whenLabel } from "../shared/history";
import type { TestNode } from "../shared/model";
import type { TrendPoint } from "../shared/trends";
import { findNode, formatMillis, labelOf } from "./nodes";

/** "92%", rounded down so one failure never reads 100%; "Passed" or "Failed" for one test. */
export function passRateLabel(p: TrendPoint): string {
  if (p.total === 1) return p.passed > 0 ? "Passed" : "Failed";
  return `${Math.floor((p.passed * 100) / (p.passed + p.failed))}%`;
}

/** "Today 14:32: 12 passed, 3 failed, 15 of 412 tests, 1.40 s per test"; for one test "Today 14:32: passed, 1.40 s". */
export function trendTip(p: TrendPoint, now: number): string {
  const one = p.total === 1;
  const parts = one
    ? [p.passed > 0 ? "passed" : "failed"]
    : [`${p.passed} passed`, `${p.failed} failed`, `${p.passed + p.failed} of ${p.total} tests`];
  if (p.meanMillis !== null)
    parts.push(one ? formatMillis(p.meanMillis) : `${formatMillis(p.meanMillis)} per test`);
  return `${whenLabel(p.startedAt, now)}: ${parts.join(", ")}`;
}

/** What the strip covers: "All tests" for the whole tree, else the selected node's tree label. */
export function trendCaption(tree: TestNode | null, selected: string | null): string {
  const n = tree && selected ? findNode(tree, selected) : null;
  return !n || n === tree ? "All tests" : labelOf(n);
}
