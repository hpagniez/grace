import { describe, expect, it } from "vitest";
import type { TestNode } from "../shared/model";
import type { TrendPoint } from "../shared/trends";
import { passRateLabel, trendCaption, trendTip } from "./trendLabels";

const at = new Date(2026, 10, 4, 14, 32).getTime();
const now = new Date(2026, 10, 4, 18, 0).getTime();
const point = (over: Partial<TrendPoint> = {}): TrendPoint => ({
  id: "20261104-143200-000",
  startedAt: at,
  passed: 12,
  failed: 3,
  total: 412,
  meanMillis: 1400,
  ...over,
});

describe("passRateLabel", () => {
  it("gives a percentage rounded down", () => {
    expect(passRateLabel(point())).toBe("80%");
    expect(passRateLabel(point({ passed: 299, failed: 1 }))).toBe("99%");
    expect(passRateLabel(point({ passed: 4, failed: 0 }))).toBe("100%");
  });

  it("words one test as passed or failed", () => {
    expect(passRateLabel(point({ passed: 1, failed: 0, total: 1 }))).toBe("Passed");
    expect(passRateLabel(point({ passed: 0, failed: 1, total: 1 }))).toBe("Failed");
  });
});

describe("trendTip", () => {
  it("names the run, its counts and its time per test", () => {
    expect(trendTip(point(), now)).toBe(
      "Today 14:32: 12 passed, 3 failed, 15 of 412 tests, 1.40 s per test",
    );
    expect(trendTip(point({ meanMillis: 120 }), now)).toMatch(/, 120 ms per test$/);
  });

  it("words one test by its status and time", () => {
    expect(trendTip(point({ passed: 1, failed: 0, total: 1 }), now)).toBe(
      "Today 14:32: passed, 1.40 s",
    );
  });

  it("leaves out a time it does not have", () => {
    expect(trendTip(point({ meanMillis: null }), now)).toBe(
      "Today 14:32: 12 passed, 3 failed, 15 of 412 tests",
    );
  });
});

describe("trendCaption", () => {
  const n = (
    kind: TestNode["kind"],
    key: string,
    name: string,
    file: string,
    kids: TestNode[] = [],
  ) => ({ key, kind, name, file, line: 3, refId: null, tags: [], children: kids }) as TestNode;
  const scenario = n(
    "scenario",
    "s:tests/users.feature#1",
    "creates a user",
    "tests/users.feature",
  );
  const feature = n("feature", "f:tests/users.feature", "Users", "tests/users.feature", [scenario]);
  const tree = n("folder", "d:", "project", "", [feature]);

  it("says All tests for the whole tree", () => {
    expect(trendCaption(tree, null)).toBe("All tests");
    expect(trendCaption(tree, "d:")).toBe("All tests");
    expect(trendCaption(tree, "s:gone#1")).toBe("All tests");
    expect(trendCaption(null, null)).toBe("All tests");
  });

  it("names a node as the tree does", () => {
    expect(trendCaption(tree, "f:tests/users.feature")).toBe("users.feature");
    expect(trendCaption(tree, "s:tests/users.feature#1")).toBe("creates a user");
  });
});
