// Flaky tests (F15): tests that passed and failed in runs of the same project files and environment.
import type { FlakyEnv, FlakyRun, FlakyTest } from "../shared/flaky";
import type { StoredTest } from "../shared/history";
import type { TestNode } from "../shared/model";
import type { ListedRun } from "./history";
import { findNode, resolveNode } from "./karate/tree";

/**
 * The tests of `tree` that are flaky in `runs` (newest first), by their key in `tree`. Runs are
 * compared when their fingerprint and environment are equal; thread count, filter and outcome play no
 * part. A flaky test that `resolveNode` cannot find in `tree` is left out. So is one whose key holds
 * no node of its name in `tree` while a run of its group stored two tests of that name: the name
 * could be the other one's. A node gets the proof of one stored test per group.
 */
export function findFlaky(runs: ListedRun[], tree: TestNode | null): FlakyTest[] {
  if (!tree) return [];
  const groups = new Map<string, ListedRun[]>();
  for (const r of runs) {
    if (r.record.fingerprint === null) continue;
    const group = JSON.stringify([r.record.fingerprint, r.record.env]);
    groups.set(group, [...(groups.get(group) ?? []), r]);
  }
  /** By key in `tree`, then environment. */
  const found = new Map<string, Map<string | null, FlakyEnv>>();
  for (const group of groups.values()) {
    const { env } = group[0].record;
    const identity = (t: StoredTest) => JSON.stringify([t.file, t.kind, t.name]);
    /** Identities that one run of the group stored twice. */
    const twice = new Set<string>();
    for (const r of group) {
      const seen = new Set<string>();
      for (const t of r.record.tests) {
        if (seen.has(identity(t))) twice.add(identity(t));
        seen.add(identity(t));
      }
    }
    const placed = new Set<TestNode>();
    // `test` is the first stored one, that of the newest run
    const proofs = new Map<string, { test: StoredTest; runs: FlakyRun[] }>();
    for (const r of group)
      for (const test of r.record.tests) {
        if (test.status !== "passed" && test.status !== "failed") continue;
        const proof = proofs.get(test.key) ?? { test, runs: [] };
        proofs.set(test.key, proof);
        proof.runs.push({
          id: r.id,
          startedAt: r.record.startedAt,
          status: test.status,
          threads: r.record.threads,
          reportAvailable: r.reportAvailable,
        });
      }
    for (const { test, runs: proof } of proofs.values()) {
      const passed = proof.filter((r) => r.status === "passed").length;
      if (passed === 0 || passed === proof.length) continue;
      if (findNode(tree, test.key)?.name !== test.name && twice.has(identity(test))) continue;
      const node = resolveNode(tree, test);
      if (!node || placed.has(node)) continue;
      placed.add(node);
      const envs = found.get(node.key) ?? new Map<string | null, FlakyEnv>();
      found.set(node.key, envs);
      const merged = envs.get(env) ?? { env, passed: 0, failed: 0, runs: [] };
      envs.set(env, merged);
      merged.passed += passed;
      merged.failed += proof.length - passed;
      merged.runs.push(...proof);
    }
  }
  return [...found].map(([key, envs]) => ({
    key,
    envs: [...envs.values()].map((e) => ({
      ...e,
      runs: e.runs.sort((a, b) => b.startedAt - a.startedAt),
    })),
  }));
}
