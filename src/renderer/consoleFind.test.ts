import { describe, expect, it } from "vitest";
import { errorLines, findMatches, startIndex, stepIndex } from "./consoleFind";

describe("findMatches (console search R3.3)", () => {
  it("counts every occurrence, case aside, without overlap", () => {
    expect(findMatches(["get user 42", "create USER", "aaaa"], "user")).toEqual([
      { line: 0, start: 4, end: 8 },
      { line: 1, start: 7, end: 11 },
    ]);
    expect(findMatches(["aaaa"], "aa")).toEqual([
      { line: 0, start: 0, end: 2 },
      { line: 0, start: 2, end: 4 },
    ]);
  });

  it("takes the text as plain text", () => {
    expect(findMatches(["a.b(c", "axb(c"], "a.b(")).toEqual([{ line: 0, start: 0, end: 4 }]);
    expect(findMatches(["1*2\\3"], "*2\\")).toEqual([{ line: 0, start: 1, end: 4 }]);
  });

  it("keeps offsets on the line as written when case folding changes its length", () => {
    expect(findMatches(["İstanbul user"], "user")).toEqual([{ line: 0, start: 9, end: 13 }]);
  });

  it("keeps the spaces of the text", () => {
    expect(findMatches(["a ERROR b", "aERRORb"], " ERROR ")).toEqual([
      { line: 0, start: 1, end: 8 },
    ]);
  });

  it("finds nothing for no text", () => {
    expect(findMatches(["a"], "")).toEqual([]);
  });
});

describe("errorLines (console search R1)", () => {
  it("takes ERROR lines and failed tests, not WARN lines", () => {
    // Karate 2.1.4's own lines, and its end-of-run summary
    const lines = [
      "09:46:08.476 [virtual-36] INFO  karate.scenario - hello",
      "09:46:08.502 [virtual-40] WARN  karate.scenario - careful",
      "09:46:08.502 [virtual-46] ERROR karate.scenario - [ERROR] custom boom",
      "failed features:",
      "  C:\\work\\a.feature",
      "    - fails match",
      "      C:\\work\\a.feature:7",
      "      ==========",
      "    - line 2",
      "============================================================",
      "    - after the block",
    ];
    expect(errorLines(lines)).toEqual([2, 5, 8]);
  });

  it("starts the block only at the exact line", () => {
    expect(
      errorLines(["12:00 [main] INFO  karate.scenario - failed features:", "    - not a test"]),
    ).toEqual([]);
  });
});

describe("stepping", () => {
  it("starts from the top of the view, wrapping", () => {
    expect(startIndex([3, 7, 9], 5, 1)).toBe(1);
    expect(startIndex([3, 7, 9], 5, -1)).toBe(0);
    expect(startIndex([3, 7, 9], 10, 1)).toBe(0);
    expect(startIndex([3, 7, 9], 0, -1)).toBe(2);
    expect(startIndex([], 0, 1)).toBe(-1);
  });

  it("steps round", () => {
    expect(stepIndex(3, 2, 1)).toBe(0);
    expect(stepIndex(3, 0, -1)).toBe(2);
  });
});
