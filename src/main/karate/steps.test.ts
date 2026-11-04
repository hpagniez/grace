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

  it("keeps the comments Karate attached to a step, Background's too (step comments R1)", () => {
    const steps = basic()[1].steps;
    expect(steps.find((s) => s.line === 20)!.comments).toEqual([
      "# expected greeting is wrong on purpose",
    ]);
    expect(steps.find((s) => s.line === 18)).not.toHaveProperty("comments");
    const [bg] = extractSteps({
      scenarioResults: [
        {
          refId: "[1:3]",
          stepResults: [
            {
              step: {
                prefix: "*",
                keyword: "url",
                text: "u",
                line: 4,
                background: true,
                comments: ["# base"],
              },
              result: { status: "passed", millis: 1 },
            },
          ],
        },
      ],
    })[0].steps;
    expect(bg.comments).toEqual(["# base"]);
  });
});

describe("fileSteps", () => {
  const debug = readFileSync(path.resolve("test/fixtures/projects/debug/debug.feature"), "utf8");

  it("lists the background's steps, then the test's own, as written", () => {
    const steps = fileSteps(debug, 6);
    expect(steps.map((s) => s.line)).toEqual([4, 7, 8, 9, 10]);
    expect(steps[0]).toEqual({
      keyword: "*",
      text: "def base = 'from background'",
      line: 4,
      background: true,
    });
    expect(steps[1]).toEqual({
      keyword: "*",
      text: "def name = 'grace'",
      line: 7,
      background: false,
    });
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

  it("attaches the comments above a step as Karate does (step comments R1)", () => {
    const text = [
      "Feature: f", //                        1
      "  # above Background", //              2
      "  Background:", //                     3
      "    # the base", //                    4
      "    * url 'u'", //                     5
      "  # above the tag", //                 6
      "  @t", //                              7
      "  # above Scenario", //                8
      "  Scenario: s", //                     9
      "    # why", //                         10
      "", //                                  11
      "  #where from  ", //                   12
      "    * def a = 1", //                   13
      "    * def doc =", //                   14
      "    # before the doc string", //       15
      '      """', //                         16
      "      # in the doc string", //         17
      '      """', //                         18
      "    * table t", //                     19
      "      | x |", //                       20
      "      # between rows", //              21
      "      | 1 |", //                       22
      "    * def b = 2", //                   23
      "    # after the last step", //         24
      "  Scenario: next", //                  25
      "    * def c = 3", //                   26
      "  # at the end", //                    27
    ].join("\n");
    const steps = fileSteps(text, 9);
    expect(steps.map((s) => [s.line, s.comments])).toEqual([
      [5, ["# the base"]],
      [13, ["# why", "#where from"]],
      [14, undefined],
      [19, undefined],
      [23, undefined],
    ]);
    expect(steps[2]).not.toHaveProperty("comments");
    expect(fileSteps(text, 25).map((s) => [s.line, s.comments])).toEqual([
      [5, ["# the base"]],
      [26, undefined],
    ]);
  });
});
