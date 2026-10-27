import { describe, expect, it } from "vitest";
import type { FlakyTest } from "../shared/flaky";
import type { RunRecord, StoredTest } from "../shared/history";
import type { TestNode } from "../shared/model";
import { NO_FILTER } from "../shared/tags";
import { findFlaky } from "./flaky";
import type { ListedRun } from "./history";

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
): StoredTest => ({
  kind,
  file: "tests/a.feature",
  refId: `[${section}:1]`,
  name,
  line: 1,
  key: `s:tests/a.feature#${section}`,
  status,
});
const one = (status: StoredTest["status"]) => stored("scenario", "one", "1", status);
const two = (status: StoredTest["status"]) => stored("scenario", "two", "2", status);

const A = "a".repeat(64);
const B = "b".repeat(64);

/** A run that started at `i`, with `one` as given unless `over` has other tests. */
const run = (
  i: number,
  status: StoredTest["status"],
  over: Partial<RunRecord> = {},
  reportAvailable = false,
): ListedRun => ({
  id: `run-${i}`,
  reportAvailable,
  record: {
    version: 1,
    startedAt: i,
    durationMillis: 1000,
    env: null,
    threads: 1,
    fingerprint: A,
    filter: NO_FILTER,
    selection: null,
    failedOf: null,
    outcome: "finished",
    exitCode: 1,
    tests: [one(status)],
    ...over,
  },
});

const keys = (flaky: FlakyTest[]) => flaky.map((f) => f.key);

describe("findFlaky", () => {
  it("a pass and a fail with the same files and environment make a test flaky", () => {
    const runs = [run(2, "failed", { threads: 4 }, true), run(1, "passed")];
    expect(findFlaky(runs, tree())).toEqual([
      {
        key: "s:tests/a.feature#1",
        envs: [
          {
            env: null,
            passed: 1,
            failed: 1,
            runs: [
              { id: "run-2", startedAt: 2, status: "failed", threads: 4, reportAvailable: true },
              { id: "run-1", startedAt: 1, status: "passed", threads: 1, reportAvailable: false },
            ],
          },
        ],
      },
    ]);
  });

  it("two passes are not flaky", () => {
    expect(findFlaky([run(2, "passed"), run(1, "passed")], tree())).toEqual([]);
  });

  it("a null fingerprint proves nothing", () => {
    const none = { fingerprint: null };
    expect(findFlaky([run(2, "failed", none), run(1, "passed", none)], tree())).toEqual([]);
    expect(findFlaky([run(2, "failed", none), run(1, "passed")], tree())).toEqual([]);
  });

  it("different fingerprints are not compared", () => {
    expect(findFlaky([run(2, "failed", { fingerprint: B }), run(1, "passed")], tree())).toEqual([]);
  });

  it("different environments are not compared", () => {
    expect(findFlaky([run(2, "failed", { env: "qa" }), run(1, "passed")], tree())).toEqual([]);
  });

  it("the same pair in two environments gives two envs", () => {
    const runs = [
      run(4, "failed"),
      run(3, "passed"),
      run(2, "failed", { env: "qa" }),
      run(1, "passed", { env: "qa" }),
    ];
    const [flaky, ...more] = findFlaky(runs, tree());
    expect(more).toEqual([]);
    expect(flaky.envs.map((e) => [e.env, e.passed, e.failed, e.runs.length])).toEqual([
      [null, 1, 1, 2],
      ["qa", 1, 1, 2],
    ]);
  });

  it("threads, filter and outcome play no part", () => {
    const runs = [
      run(2, "failed", { threads: 8, outcome: "cancelled", exitCode: null }),
      run(1, "passed", { filter: { include: ["smoke"], exclude: [], hideNonMatching: false } }),
    ];
    expect(keys(findFlaky(runs, tree()))).toEqual(["s:tests/a.feature#1"]);
  });

  it("notRun and interrupted do not count", () => {
    const runs = [
      run(4, "interrupted", { tests: [one("interrupted"), two("passed")] }),
      run(3, "notRun", { tests: [one("notRun"), two("notRun")] }),
      run(2, "failed", { tests: [one("failed"), two("interrupted")] }),
      run(1, "passed", { tests: [one("passed"), two("notRun")] }),
    ];
    const [flaky, ...more] = findFlaky(runs, tree());
    expect(more).toEqual([]);
    expect(flaky.key).toBe("s:tests/a.feature#1");
    expect(flaky.envs[0].runs.map((r) => r.id)).toEqual(["run-2", "run-1"]);
  });

  it("proofs of two fingerprints of one environment merge", () => {
    const runs = [
      run(4, "failed", { fingerprint: B }),
      run(3, "failed"),
      run(2, "passed", { fingerprint: B }),
      run(1, "passed"),
    ];
    const [flaky, ...more] = findFlaky(runs, tree());
    expect(more).toEqual([]);
    expect(flaky.envs).toHaveLength(1);
    expect(flaky.envs[0]).toMatchObject({ env: null, passed: 2, failed: 2 });
    expect(flaky.envs[0].runs.map((r) => r.id)).toEqual(["run-4", "run-3", "run-2", "run-1"]);
  });

  describe("placed by resolveNode", () => {
    const flakyOne = [run(2, "failed"), run(1, "passed")];

    it("a test that moved gets its key in the tree", () => {
      const moved = tree([
        ["added", 1],
        ["one", 2],
      ]);
      expect(keys(findFlaky(flakyOne, moved))).toEqual(["s:tests/a.feature#2"]);
    });

    it("an ambiguous name gives nothing", () => {
      const twice = tree([
        ["added", 1],
        ["one", 2],
        ["one", 4],
      ]);
      expect(findFlaky(flakyOne, twice)).toEqual([]);
    });

    it("a vanished test gives nothing", () => {
      expect(findFlaky(flakyOne, tree([["two", 2]]))).toEqual([]);
    });

    // two scenarios named "x": #1 stable, #2 flaky
    const x = (n: string, status: StoredTest["status"]) => stored("scenario", "x", n, status);
    const twins = [
      run(2, "failed", { tests: [x("1", "passed"), x("2", "failed")] }),
      run(1, "passed", { tests: [x("1", "passed"), x("2", "passed")] }),
    ];

    it("same-named scenarios are told apart by their keys", () => {
      const both = tree([
        ["x", 1],
        ["x", 2],
      ]);
      expect(keys(findFlaky(twins, both))).toEqual(["s:tests/a.feature#2"]);
    });

    it("a name the group's runs stored twice is not placed by name", () => {
      // #2 was renamed: its key now holds "y", and "x" is unique in the tree, but it is #1
      const renamed = tree([
        ["x", 1],
        ["y", 2],
      ]);
      expect(findFlaky(twins, renamed)).toEqual([]);
    });

    it("both twins flipped list each run once when one is gone", () => {
      const runs = [
        run(4, "failed", { tests: [x("1", "failed"), x("2", "failed")] }),
        run(3, "passed", { tests: [x("1", "passed"), x("2", "passed")] }),
      ];
      const [flaky, ...more] = findFlaky(runs, tree([["x", 1]]));
      expect(more).toEqual([]);
      expect(flaky.key).toBe("s:tests/a.feature#1");
      expect(flaky.envs).toHaveLength(1);
      expect(flaky.envs[0]).toMatchObject({ passed: 1, failed: 1 });
      expect(flaky.envs[0].runs.map((r) => r.id)).toEqual(["run-4", "run-3"]);
    });

    it("a node is placed from one stored key of a group only", () => {
      // no run stored both twins, so only the second rule keeps #2's proof off the one "x" left
      const runs = [
        run(4, "failed", { tests: [x("1", "failed")] }),
        run(3, "passed", { tests: [x("1", "passed")] }),
        run(2, "failed", { tests: [x("2", "failed")] }),
        run(1, "passed", { tests: [x("2", "passed")] }),
      ];
      const [flaky, ...more] = findFlaky(runs, tree([["x", 1]]));
      expect(more).toEqual([]);
      expect(flaky.envs[0]).toMatchObject({ passed: 1, failed: 1 });
      expect(flaky.envs[0].runs.map((r) => r.id)).toEqual(["run-4", "run-3"]);
    });
  });

  it("an outline row is marked by its row key", () => {
    const row = (status: StoredTest["status"]) => stored("row", "rows 2", "3.2", status);
    const runs = [
      run(2, "failed", { tests: [row("failed")] }),
      run(1, "passed", { tests: [row("passed")] }),
    ];
    expect(keys(findFlaky(runs, tree()))).toEqual(["s:tests/a.feature#3.2"]);
  });

  it("no tree, no flaky", () => {
    expect(findFlaky([run(2, "failed"), run(1, "passed")], null)).toEqual([]);
  });
});
