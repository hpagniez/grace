import { describe, expect, it } from "vitest";
import { anchoredScrollTop, drawnRange, pageRows, scrollToShow, survivor } from "./virtual";

const v = (scrollTop: number, height = 600, rowHeight = 24, pad = 0) => ({
  scrollTop,
  height,
  rowHeight,
  pad,
});

describe("drawnRange", () => {
  it("draws the rows in view with 10 either side", () => {
    expect(drawnRange(1000, v(0))).toEqual({ first: 0, last: 35 });
    expect(drawnRange(1000, v(2400))).toEqual({ first: 90, last: 135 });
  });

  it("counts the padding before the first row", () => {
    expect(drawnRange(1000, v(2408, 600, 24, 8))).toEqual({ first: 90, last: 135 });
  });

  it("measures a scroller of height 0 as 600 px", () => {
    expect(drawnRange(1000, v(0, 0))).toEqual({ first: 0, last: 35 });
  });

  it("clamps a scrollTop past the end of the content", () => {
    expect(drawnRange(5, v(4000))).toEqual({ first: 0, last: 5 });
    expect(drawnRange(1000, v(1e9))).toEqual({ first: 965, last: 1000 });
  });

  it("draws nothing for no rows", () => {
    expect(drawnRange(0, v(0))).toEqual({ first: 0, last: 0 });
  });
});

describe("scrollToShow", () => {
  it("scrolls the least to bring a row into view", () => {
    expect(scrollToShow(50, v(0))).toBe(624);
    expect(scrollToShow(10, v(624))).toBe(240);
    expect(scrollToShow(0, v(100, 600, 24, 8))).toBe(0);
  });

  it("does not scroll when the row is in view", () => {
    expect(scrollToShow(30, v(624))).toBe(624);
  });
});

describe("pageRows", () => {
  it("counts whole rows in view, at least 1", () => {
    expect(pageRows(v(0))).toBe(25);
    expect(pageRows(v(0, 84, 28))).toBe(3);
    expect(pageRows(v(0, 20, 28))).toBe(1);
  });

  it("measures a scroller of height 0 as 600 px", () => {
    expect(pageRows(v(0, 0))).toBe(25);
  });
});

describe("anchoredScrollTop", () => {
  const keys = ["a", "b", "c", "d", "e"];

  it("keeps the first row in view still when rows are added above", () => {
    expect(anchoredScrollTop(v(48), keys, ["x", "y", ...keys])).toBe(96);
    expect(anchoredScrollTop(v(60), keys, ["x", "y", ...keys])).toBe(108);
  });

  it("counts the padding", () => {
    expect(anchoredScrollTop(v(56, 600, 24, 8), keys, ["x", "y", ...keys])).toBe(104);
  });

  it("leaves a list at the top at the top", () => {
    expect(anchoredScrollTop(v(0), keys, ["x", ...keys])).toBe(0);
  });

  it("keeps the scrollTop when the anchor is gone", () => {
    expect(anchoredScrollTop(v(48), keys, ["a", "b"])).toBe(48);
    expect(anchoredScrollTop(v(4800), keys, keys)).toBe(4800);
  });
});

describe("survivor", () => {
  it("is the nearest row before the key that is still there", () => {
    expect(survivor("c", ["a", "b", "c", "d"], ["a", "d"])).toBe("a");
    expect(survivor("c", ["a", "b", "c", "d"], ["a", "b", "d"])).toBe("b");
  });

  it("falls back to the first row, then to null", () => {
    expect(survivor("a", ["a", "b"], ["b"])).toBe("b");
    expect(survivor("a", ["a"], [])).toBeNull();
  });
});
