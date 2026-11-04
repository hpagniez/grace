// Trends (#93): pass rate and time per test of a tree selection over the stored runs.
import type { StoredTest } from "../shared/history";
import type { TestNode } from "../shared/model";
import type { TrendPoint } from "../shared/trends";
import type { ListedRun } from "./history";
import { findNode, leafKeys } from "./karate/tree";

const identity = (t: Pick<TestNode, "file" | "kind" | "name">) =>
  JSON.stringify([t.file, t.kind, t.name]);

/**
 * `resolveNode`'s rule through one index of `tree`: the node of the same key and name, else the one
 * node of the same file, kind and name. `resolveNode` walks the tree per call; this is called per
 * stored test.
 */
function resolver(tree: TestNode): (t: StoredTest) => TestNode | null {
  const byKey = new Map<string, TestNode>();
  const byIdentity = new Map<string, TestNode[]>();
  const walk = (n: TestNode) => {
    byKey.set(n.key, n);
    byIdentity.set(identity(n), [...(byIdentity.get(identity(n)) ?? []), n]);
    n.children.forEach(walk);
  };
  walk(tree);
  return (t) => {
    const same = byKey.get(t.key);
    if (same && same.name === t.name) return same;
    const matches = byIdentity.get(identity(t)) ?? [];
    return matches.length === 1 ? matches[0] : null;
  };
}

/**
 * The trend of the node `key` (the whole tree when null) in `runs` (newest first), oldest first. A
 * run counts when it ran on `env` and is no debug run; a test counts when it was stored `passed` or
 * `failed` and resolves to a test under the node. A run with no counted test has no point.
 */
export function findTrend(
  runs: ListedRun[],
  tree: TestNode | null,
  key: string | null,
  env: string | null,
): TrendPoint[] {
  const selected = tree && (key === null ? tree : findNode(tree, key));
  if (!tree || !selected) return [];
  const under = new Set(leafKeys(selected));
  const resolve = resolver(tree);
  const points: TrendPoint[] = [];
  for (const { id, record } of runs) {
    if (record.debug || record.env !== env) continue;
    let passed = 0;
    let failed = 0;
    let millis = 0;
    let timed = 0;
    for (const t of record.tests) {
      if (t.status !== "passed" && t.status !== "failed") continue;
      const n = resolve(t);
      if (!n || !under.has(n.key)) continue;
      if (t.status === "passed") passed++;
      else failed++;
      if (t.durationMillis !== undefined) {
        millis += t.durationMillis;
        timed++;
      }
    }
    if (passed + failed === 0) continue;
    const meanMillis = timed === 0 ? null : millis / timed;
    points.push({ id, startedAt: record.startedAt, passed, failed, total: under.size, meanMillis });
  }
  return points.reverse();
}
