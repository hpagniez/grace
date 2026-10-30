import type { Status, TestNode } from "../shared/model";
import { filterOn, matchingLeaves, type FilterState } from "../shared/tags";
import { STATUS_LABEL } from "./nodes";

export interface TreeRow {
  node: TestNode;
  /** aria-level: 1 for the root's children. */
  level: number;
  /** Shown siblings. */
  setSize: number;
  /** 1-based among shown siblings. */
  posInSet: number;
  /** Null at level 1. */
  parentKey: string | null;
}

/**
 * The rows a person can scroll to, in order: children of closed nodes left out, and non-matching
 * rows left out under "Hide non-matching" (R2.1). The root itself is not a row. Reads no status,
 * so a run never changes the rows (R2.3).
 */
export function treeRows(
  root: TestNode,
  closed: ReadonlySet<string>,
  filter: FilterState,
): TreeRow[] {
  const hide = filter.hideNonMatching && filterOn(filter);
  const rows: TreeRow[] = [];
  const walk = (parent: TestNode, level: number, parentKey: string | null) => {
    // A folder, feature or outline matches when a test under it does.
    const shown = hide
      ? parent.children.filter((c) => matchingLeaves(c, filter).length > 0)
      : parent.children;
    shown.forEach((node, i) => {
      rows.push({ node, level, setSize: shown.length, posInSet: i + 1, parentKey });
      if (!closed.has(node.key)) walk(node, level + 1, node.key);
    });
  };
  walk(root, 1, null);
  return rows;
}

/** A row's accessible name (R5.2): label, status word in lower case, the flaky chip's words if any, then tags. */
export function rowName(
  label: string,
  status: Status,
  flaky: string | null,
  tags: readonly string[],
): string {
  return [label, STATUS_LABEL[status].toLowerCase(), ...(flaky ? [flaky] : []), ...tags].join(", ");
}
