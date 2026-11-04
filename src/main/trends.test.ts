import { describe, expect, it } from "vitest";
import type { RunRecord, StoredTest } from "../shared/history";
import type { TestNode } from "../shared/model";
import { NO_FILTER } from "../shared/tags";
import type { ListedRun } from "./history";
import { findTrend } from "./trends";

const node = (
  kind: TestNode["kind"],
  key: string,
  name: string,
  extra: Partial<TestNode> = {},
): TestNode => ({
  key,
  kind,
  name,
  file: "tests/a.feature",
  line: null,
  refId: null,
  tags: [],
  children: [],
  ...extra,
});

/** Feature `tests/a.feature`: the given scenarios as `[name, section]`, then the outline `rows`. */
const tree = (
  scenarios: [string, number][] = [
    ["one", 1],
    ["two", 2],
  ],
): TestNode =>
  node("folder", "d:", "p", {
    children: [
      node("feature", "f:tests/a.feature", "a", {
        children: [
          ...scenarios.map(([name, n]) => node("scenario", `s:tests/a.feature#${n}`, name)),
          node("outline", "o:tests/a.feature#3", "rows", {
            children: [
              node("row", "s:tests/a.feature#3.1", "rows 1"),
              node("row", "s:tests/a.feature#3.2", "rows 2"),
            ],
          }),
        ],
      }),
    ],
  });

const stored = (
  kind: StoredTest["kind"],
  name: string,
  section: string,
  status: StoredTest["status"],
  durationMillis?: number,
): StoredTest => ({
  kind,
  file: "tests/a.feature",
  refId: `[${section}:1]`,
  name,
  line: 1,
  key: `s:tests/a.feature#${section}`,
  status,
  ...(durationMillis === undefined ? {} : { durationMillis }),
});
const one = (status: StoredTest["status"], ms?: number) =>
  stored("scenario", "one", "1", status, ms);
const two = (status: StoredTest["status"], ms?: number) =>
  stored("scenario", "two", "2", status, ms);

/** A run that started at `i` with these tests. */
const run = (i: number, tests: StoredTest[], over: Partial<RunRecord> = {}): ListedRun => ({
  id: `run-${i}`,
  reportAvailable: false,
  record: {
    version: 1,
    startedAt: i,
    durationMillis: 1000,
    env: null,
    threads: 1,
    fingerprint: null,
    filter: NO_FILTER,
    selection: null,
    failedOf: null,
    outcome: "finished",
    exitCode: 1,
    tests,
    ...over,
  },
});

describe("findTrend", () => {
  it("covers the whole tree, oldest first", () => {
    // listRuns gives them newest first
    const runs = [run(2, [one("failed"), two("failed")]), run(1, [one("passed"), two("failed")])];
    expect(findTrend(runs, tree(), null, null)).toEqual([
      { id: "run-1", startedAt: 1, passed: 1, failed: 1, total: 4, meanMillis: null },
      { id: "run-2", startedAt: 2, passed: 0, failed: 2, total: 4, meanMillis: null },
    ]);
  });

  it("counts only the tests of the selection", () => {
    const runs = [run(1, [one("passed"), two("failed"), stored("row", "rows 1", "3.1", "passed")])];
    expect(findTrend(runs, tree(), "s:tests/a.feature#1", null)).toMatchObject([
      { passed: 1, failed: 0, total: 1 },
    ]);
    expect(findTrend(runs, tree(), "f:tests/a.feature", null)).toMatchObject([
      { passed: 2, failed: 1, total: 4 },
    ]);
    expect(findTrend(runs, tree(), "o:tests/a.feature#3", null)).toMatchObject([
      { passed: 1, failed: 0, total: 2 },
    ]);
  });

  it("gives no point to a run with no counted test", () => {
    expect(findTrend([run(1, [two("passed")])], tree(), "s:tests/a.feature#1", null)).toEqual([]);
  });

  it("does not count notRun and interrupted tests", () => {
    expect(findTrend([run(1, [one("notRun"), two("interrupted")])], tree(), null, null)).toEqual(
      [],
    );
  });

  it("leaves out a debug run", () => {
    const runs = [run(2, [one("failed")], { debug: true }), run(1, [one("passed")])];
    expect(findTrend(runs, tree(), null, null).map((p) => p.id)).toEqual(["run-1"]);
  });

  it("counts only the runs of the environment", () => {
    const runs = [run(2, [one("passed")], { env: "qa" }), run(1, [one("passed")], { env: "dev" })];
    expect(findTrend(runs, tree(), null, "dev").map((p) => p.id)).toEqual(["run-1"]);
    expect(findTrend(runs, tree(), null, null)).toEqual([]);
  });

  it("gives the mean time of the counted tests that have one", () => {
    const at = (tests: StoredTest[]) => findTrend([run(1, tests)], tree(), null, null)[0];
    expect(at([one("passed", 1000), two("failed", 2000)]).meanMillis).toBe(1500);
    expect(at([one("passed"), two("failed", 300)]).meanMillis).toBe(300);
    expect(at([one("passed"), two("failed")]).meanMillis).toBeNull();
    // a test that does not count brings no time
    expect(at([one("passed", 100), two("notRun", 900)]).meanMillis).toBe(100);
  });

  it("finds a test whose key moved by its name", () => {
    const moved = tree([
      ["two", 2],
      ["one", 5],
    ]);
    expect(findTrend([run(1, [one("passed")])], moved, "s:tests/a.feature#5", null)).toMatchObject([
      { passed: 1, total: 1 },
    ]);
  });

  it("does not count a test whose name is ambiguous", () => {
    const twice = tree([
      ["one", 4],
      ["one", 5],
    ]);
    expect(findTrend([run(1, [one("passed")])], twice, null, null)).toEqual([]);
  });

  it("gives nothing for an unknown key or no tree", () => {
    const runs = [run(1, [one("passed")])];
    expect(findTrend(runs, tree(), "s:nope#1", null)).toEqual([]);
    expect(findTrend(runs, null, null, null)).toEqual([]);
  });
});
