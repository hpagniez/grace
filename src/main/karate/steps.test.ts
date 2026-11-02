import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { readAllEvents } from "./events";
import { extractSteps, fileSteps } from "./steps";

const basic = () => {
  const ev = readAllEvents(path.resolve("test/fixtures/events/run-clean.jsonl")).find(
    (e) =>
      e.type === "FEATURE_EXIT" &&
      e.data.callDepth === 0 &&
      e.data.relativePath === "tests/basic.feature",
  )!;
  return extractSteps(ev.data);
};

describe("extractSteps", () => {
  it("returns one entry per scenario, by refId", () => {
    expect(basic().map((s) => s.refId)).toEqual(["[1:7]", "[2:15]", "[3:22]", "[4:28]"]);
  });

  it("maps a failed step", () => {
    const step = basic()[1].steps.find((s) => s.status === "failed")!;
    expect(step.keyword).toBe("And");
    expect(step.text).toMatch(/^match response\.message/);
    expect(step.line).toBe(20);
  });

  it("keeps the log of a step and flags background steps", () => {
    const steps = basic()[0].steps;
    const get = steps.find((s) => s.text === "method get")!;
    expect(get.log).toContain("GET");
    expect(get.log).toContain("200");
    expect(steps[0].background).toBe(true);
    expect(steps[1].background).toBe(false);
    expect(steps[1].log).toBeNull();
  });
});

describe("fileSteps", () => {
  const debug = readFileSync(path.resolve("test/fixtures/projects/debug/debug.feature"), "utf8");

  it("lists the background's steps, then the test's own, as written", () => {
    const steps = fileSteps(debug, 6);
    expect(steps.map((s) => s.line)).toEqual([4, 7, 8, 9, 10]);
    expect(steps[0]).toEqual({ keyword: "*", text: "def base = 'from background'", line: 4, background: true });
    expect(steps[1]).toEqual({ keyword: "*", text: "def name = 'grace'", line: 7, background: false });
    expect(fileSteps(debug, 12).map((s) => s.line)).toEqual([4, 13]);
  });

  it("skips doc strings, tables and comments, and stops an outline at its Examples", () => {
    const text = [
      "Feature: f", //                        1
      "  Scenario Outline: o <n>", //         2
      "    Given def a = <n>", //             3
      "    * def doc =", //                   4
      '      """', //                         5
      "      * not a step", //                6
      '      """', //                         7
      "    # * a comment", //                 8
      "    And table t", //                   9
      "      | x |", //                       10
      "  Examples:", //                       11
      "    | n |", //                         12
      "    | 1 |", //                         13
      "  Scenario: next", //                  14
      "    * def b = 1", //                   15
    ].join("\r\n");
    expect(fileSteps(text, 2).map((s) => [s.keyword, s.line])).toEqual([
      ["Given", 3],
      ["*", 4],
      ["And", 9],
    ]);
  });
});
