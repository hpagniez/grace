import { describe, expect, it } from "vitest";
import type { Status, TestNode } from "../shared/model";
import { NO_FILTER } from "../shared/tags";
import { configFailure, failureGroups, reasonOf, shownLeaves, type Result } from "./failures";

const test = (key: string, tags: string[] = []): TestNode => ({
  key,
  kind: "scenario",
  name: key,
  file: "x.feature",
  line: 1,
  refId: `[1:${key}]`,
  tags,
  children: [],
});
const tests = [test("a", ["fast"]), test("b", ["fast"]), test("c"), test("d")];
const root: TestNode = {
  key: "d:",
  kind: "folder",
  name: "p",
  file: "",
  line: null,
  refId: null,
  tags: [],
  children: tests,
};
/** A failed result whose error is `head` then the reason, as Karate's SCENARIO_EXIT gives it. */
const failed = (reason: string, head = ""): Result => ({
  error: head + reason,
  reasonAt: head.length,
});
const groupsOf = (results: Record<string, Result>, statuses?: Record<string, Status>) =>
  failureGroups(
    tests,
    statuses ?? Object.fromEntries(Object.keys(results).map((k) => [k, "failed"])),
    results,
  ).map((g) => g.keys);

describe("reasonOf (#229 R2.1)", () => {
  it("is the error from where Karate's reason starts, or the whole error", () => {
    expect(reasonOf({ error: "C:/p/x.feature:5\nThen status 200\nboom", reasonAt: 33 })).toBe(
      "boom",
    );
    expect(reasonOf({ error: "boom" })).toBe("boom");
    expect(reasonOf({})).toBeNull();
    expect(reasonOf(undefined)).toBeNull();
  });
});

describe("failureGroups (#229 R2.2, R3.1)", () => {
  it("groups failures at different places with one message", () => {
    expect(
      groupsOf({
        a: failed("expected status: 200, actual: 503", "C:/p/x.feature:5\nThen status 200\n"),
        b: failed(
          "expected status: 200, actual: 503",
          "C:/p/y.feature:9\n# why\nThen status 200\n",
        ),
        c: failed("expected status: 200, actual: 404", "C:/p/y.feature:12\nThen status 200\n"),
      }),
    ).toEqual([["a", "b"]]);
  });

  it("leaves out a failed test with no error, and a test that did not fail", () => {
    expect(
      groupsOf(
        { a: failed("boom"), b: {}, c: failed("boom") },
        { a: "failed", b: "failed", c: "passed" },
      ),
    ).toEqual([]);
  });

  it("groups karate.fail at different lines", () => {
    const at = (line: number) =>
      `js failed:\n==========\n  Line: ${line}, Col: 0\n  Code: karate.fail('no vpn')\n  Error: no vpn\n==========`;
    expect(groupsOf({ a: failed(at(10)), b: failed(at(7)) })).toEqual([["a", "b"]]);
  });

  it("groups a thrown string at different positions", () => {
    expect(groupsOf({ a: failed("bad\n    at 15:1"), b: failed("bad\n    at 3:5") })).toEqual([
      ["a", "b"],
    ]);
  });

  it("orders groups by their first test, each in tree order", () => {
    expect(
      groupsOf({ d: failed("two"), c: failed("one"), b: failed("two"), a: failed("one") }),
    ).toEqual([
      ["a", "c"],
      ["b", "d"],
    ]);
  });
});

describe("shownLeaves (#229 R3.1)", () => {
  it("keeps only the matching tests under Hide non-matching", () => {
    const fast = { ...NO_FILTER, include: ["fast"] };
    expect(shownLeaves(root, { ...fast, hideNonMatching: true }).map((t) => t.key)).toEqual([
      "a",
      "b",
    ]);
    expect(shownLeaves(root, fast).map((t) => t.key)).toEqual(["a", "b", "c", "d"]);
  });
});

describe("configFailure (#229 R6)", () => {
  const reason =
    "Config evaluation failed: karate-config.js - js failed:\n==========\n  Line: 4, Col: 2\n  Code: karate.callSingle('classpath:auth.feature', { baseUrl: baseUrl });\n  Error: callSingle failed\n==========";

  it("names the file, its line, the first reason and the count", () => {
    expect(
      configFailure(tests, ["b", "a"], { a: { error: reason, reasonAt: 0 }, b: { error: reason } }),
    ).toEqual({ file: "karate-config.js", line: 4, reason, count: 2 });
  });

  it("drops classpath: and opens line 1 when the message has no line", () => {
    const dev = "Config evaluation failed: classpath:karate-config-dev.js - boom";
    expect(configFailure(tests, ["c"], { c: { error: dev } })).toEqual({
      file: "karate-config-dev.js",
      line: 1,
      reason: dev,
      count: 1,
    });
  });

  it("counts only the given failed keys, and is null without a config reason", () => {
    const results = { a: { error: reason }, b: { error: reason }, c: { error: "boom" } };
    expect(configFailure(tests, ["a", "c"], results)?.count).toBe(1);
    expect(configFailure(tests, ["c"], results)).toBeNull();
  });
});
