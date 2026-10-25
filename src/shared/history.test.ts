import { describe, expect, it } from "vitest";
import {
  RUN_ID,
  parseRunRecord,
  resultLabel,
  toEntry,
  whatRanLabel,
  whenLabel,
  type HistoryEntry,
  type RunRecord,
  type StoredTest,
  type TestStatusFinal,
} from "./history";

// The R2 example of docs/specs/2026-10-25-run-history.md.
const specExample = (): RunRecord => ({
  version: 1,
  startedAt: 1792937525123,
  durationMillis: 72000,
  env: "qa",
  filter: { include: ["smoke"], exclude: ["slow"], hideNonMatching: false },
  selection: {
    kind: "feature",
    file: "tests/users.feature",
    name: "users",
    line: null,
    key: "f:tests/users.feature",
  },
  failedOf: null,
  outcome: "finished",
  exitCode: 1,
  tests: [
    {
      kind: "scenario",
      file: "tests/users.feature",
      refId: "[1:6]",
      name: "create user",
      line: 6,
      key: "s:tests/users.feature#1",
      status: "passed",
      durationMillis: 412,
    },
    {
      kind: "row",
      file: "tests/users.feature",
      refId: "[2.1:14]",
      name: "age 3",
      line: 21,
      key: "s:tests/users.feature#2.1",
      status: "failed",
      durationMillis: 95,
    },
  ],
});

const test = (status: TestStatusFinal, n: number): StoredTest => ({
  kind: "scenario",
  file: "tests/x.feature",
  refId: `[${n}:${n}]`,
  name: `t${n}`,
  line: n,
  key: `s:tests/x.feature#${n}`,
  status,
});

const entry = (over: Partial<HistoryEntry> = {}): HistoryEntry => ({
  id: "20261025-143205-123",
  startedAt: new Date(2026, 9, 25, 14, 32).getTime(),
  durationMillis: 1000,
  env: null,
  filter: { include: [], exclude: [], hideNonMatching: false },
  selection: { kind: "folder", file: "", name: "clean", line: null, key: "d:" },
  failedOf: null,
  outcome: "finished",
  exitCode: 0,
  passed: 41,
  failed: 2,
  total: 43,
  reportAvailable: true,
  ...over,
});

describe("RUN_ID", () => {
  it("accepts ids and rejects traversal", () => {
    expect(RUN_ID.test("20261025-143205-123")).toBe(true);
    expect(RUN_ID.test("20261025-143205-123-2")).toBe(true);
    for (const bad of ["..", "20261025-143205-123\\..", "x/20261025-143205-123", ""])
      expect(RUN_ID.test(bad)).toBe(false);
  });
});

describe("parseRunRecord", () => {
  it("accepts the spec example", () => {
    expect(parseRunRecord(JSON.parse(JSON.stringify(specExample())))).toEqual(specExample());
  });

  it("accepts a re-run failed record with no selection and exit code", () => {
    const r = { ...specExample(), selection: null, failedOf: 1792937525123, exitCode: null };
    expect(parseRunRecord(r)).toEqual(r);
  });

  it("rejects bad shapes", () => {
    const { outcome: _outcome, ...noOutcome } = specExample();
    const bad: unknown[] = [
      null,
      "x",
      { ...specExample(), version: 2 },
      { ...specExample(), tests: {} },
      { ...specExample(), tests: [{ ...test("passed", 1), status: "pending" }] },
      { ...specExample(), tests: [test("passed", 1), { ...test("passed", 2), line: "2" }] },
      { ...specExample(), env: "../x" },
      { ...specExample(), filter: { include: ["a b"], exclude: [], hideNonMatching: false } },
      { ...specExample(), selection: { ...specExample().selection, kind: "suite" } },
      { ...specExample(), outcome: "done" },
      { ...specExample(), exitCode: "1" },
      { ...specExample(), startedAt: "now" },
      noOutcome,
    ];
    for (const b of bad) expect(parseRunRecord(b)).toBeNull();
  });
});

describe("toEntry", () => {
  it("counts from tests", () => {
    const tests = [
      ...[1, 2, 3].map((n) => test("passed", n)),
      ...[4, 5].map((n) => test("failed", n)),
      test("notRun", 6),
      test("interrupted", 7),
    ];
    const e = toEntry("20261025-143205-123", { ...specExample(), tests }, false);
    expect([e.passed, e.failed, e.total]).toEqual([3, 2, 7]);
    expect(e.id).toBe("20261025-143205-123");
    expect(e.reportAvailable).toBe(false);
    expect(e).not.toHaveProperty("tests");
  });
});

describe("whenLabel", () => {
  const now = new Date(2026, 9, 25, 16, 0).getTime();
  it("says Today for the same day", () => {
    expect(whenLabel(new Date(2026, 9, 25, 14, 32).getTime(), now)).toBe("Today 14:32");
  });
  it("says Yesterday across midnight", () => {
    const justAfterMidnight = new Date(2026, 9, 25, 0, 1).getTime();
    expect(whenLabel(new Date(2026, 9, 24, 23, 59).getTime(), justAfterMidnight)).toBe(
      "Yesterday 23:59",
    );
  });
  it("gives the date for older runs", () => {
    expect(whenLabel(new Date(2026, 9, 21, 9, 5).getTime(), now)).toBe("2026-10-21 09:05");
  });
});

describe("whatRanLabel", () => {
  const now = new Date(2026, 9, 25, 16, 0).getTime();
  const sel = (
    kind: NonNullable<HistoryEntry["selection"]>["kind"],
    file: string,
    name: string,
  ) => ({
    selection: { kind, file, name, line: null, key: "k" },
  });
  it("names what ran", () => {
    expect(whatRanLabel(entry(), now)).toBe("All");
    expect(whatRanLabel(entry(sel("folder", "tests/api", "api")), now)).toBe("tests/api");
    expect(whatRanLabel(entry(sel("feature", "tests/users.feature", "users")), now)).toBe(
      "tests/users.feature",
    );
    expect(whatRanLabel(entry(sel("row", "tests/users.feature", "age 3")), now)).toBe("age 3");
  });
  it("says Failed tests of <when> for a re-run failed", () => {
    const failedOf = new Date(2026, 9, 25, 14, 32).getTime();
    expect(whatRanLabel(entry({ selection: null, failedOf }), now)).toBe(
      "Failed tests of Today 14:32",
    );
  });
});

describe("resultLabel", () => {
  it("counts a finished run", () => {
    expect(resultLabel(entry())).toBe("41 passed, 2 failed of 43");
  });
  it("counts a cancelled run", () => {
    expect(resultLabel(entry({ outcome: "cancelled", passed: 3, failed: 1 }))).toBe(
      "Cancelled, 3 passed, 1 failed of 43",
    );
  });
  it("reports an error with its exit code", () => {
    expect(resultLabel(entry({ outcome: "failed", exitCode: 3 }))).toBe("Error (exit 3)");
    expect(resultLabel(entry({ outcome: "failed", exitCode: null }))).toBe("Error");
  });
});
