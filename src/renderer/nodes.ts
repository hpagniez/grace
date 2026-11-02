import type { Breakpoint } from "../shared/events";
import type { Status, TestNode } from "../shared/model";

export const findNode = (n: TestNode, key: string): TestNode | null => {
  if (n.key === key) return n;
  for (const c of n.children) {
    const hit = findNode(c, key);
    if (hit) return hit;
  }
  return null;
};

/** The keys of the nodes above `key`, from the root down; empty when `key` is not under `n`. */
export const ancestors = (n: TestNode, key: string): string[] => {
  for (const c of n.children) {
    if (c.key === key) return [n.key];
    const above = ancestors(c, key);
    if (above.length) return [n.key, ...above];
  }
  return [];
};

/**
 * The feature rows with a breakpoint in their file, and the scenario and outline rows with one on a line from
 * theirs up to the next sibling's (step debugger R5.2). A background's breakpoint marks only its feature.
 */
export function breakpointKeys(tree: TestNode | null, breakpoints: Breakpoint[]): Set<string> {
  const keys = new Set<string>();
  if (!tree || breakpoints.length === 0) return keys;
  const visit = (n: TestNode) => {
    if (n.kind !== "feature") return n.children.forEach(visit);
    const lines = breakpoints.filter((b) => b.file === n.file).map((b) => b.line);
    if (lines.length === 0) return;
    keys.add(n.key);
    n.children.forEach((c, i) => {
      const next = n.children[i + 1]?.line ?? Infinity;
      if (c.line !== null && lines.some((l) => l >= c.line! && l < next)) keys.add(c.key);
    });
  };
  visit(tree);
  return keys;
}

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
