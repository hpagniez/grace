import type { Status, TestNode } from "../shared/model";

export const findNode = (n: TestNode, key: string): TestNode | null => {
  if (n.key === key) return n;
  for (const c of n.children) {
    const hit = findNode(c, key);
    if (hit) return hit;
  }
  return null;
};

/** A node's name; Karate gives an unnamed `Scenario:` a null name. */
export const nodeName = (n: TestNode): string => n.name || `Scenario at line ${n.line}`;

/** Scenarios and rows at or under `n`. */
export const leaves = (n: TestNode): TestNode[] =>
  n.kind === "scenario" || n.kind === "row" ? [n] : n.children.flatMap(leaves);

export const STATUS_LABEL: Record<Status, string> = {
  notRun: "Not run",
  pending: "Pending",
  running: "Running",
  passed: "Passed",
  failed: "Failed",
  interrupted: "Interrupted",
};

export function formatMillis(ms: number): string {
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(2)} s`;
}
