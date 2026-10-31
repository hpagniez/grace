import { describe, expect, it } from "vitest";
import { median, order } from "./stats.mjs";

describe("median", () => {
  it("is the middle value, ignoring nulls", () => {
    expect(median([3, null, 1, 2])).toBe(2);
  });

  it("is the mean of the two middle values for an even count", () => {
    expect(median([4, 1, 3, 2])).toBe(2.5);
  });

  it("is null when no number is left", () => {
    expect(median([null])).toBeNull();
    expect(median([])).toBeNull();
  });
});

describe("order", () => {
  it("runs one warm-up per build, then the rounds in turns: a, b, b, a, a, b", () => {
    expect(order(3, ["a", "b"])).toEqual(["a", "b", "a", "b", "b", "a", "a", "b"]);
  });

  it("runs a single build once for the warm-up and once per round", () => {
    expect(order(2, ["b"])).toEqual(["b", "b", "b"]);
  });
});
