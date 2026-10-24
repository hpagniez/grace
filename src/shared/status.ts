import type { Status, TestNode } from "./model";

// Highest precedence first; a parent shows the first of these that any child has.
const PRECEDENCE: Status[] = ["failed", "running", "interrupted", "pending", "notRun"];

export function deriveStatus(node: TestNode, statuses: Record<string, Status>): Status {
  if (node.kind === "scenario" || node.kind === "row") return statuses[node.key] ?? "notRun";
  if (node.children.length === 0) return "notRun";
  const kids = new Set(node.children.map((c) => deriveStatus(c, statuses)));
  return PRECEDENCE.find((s) => kids.has(s)) ?? "passed";
}
