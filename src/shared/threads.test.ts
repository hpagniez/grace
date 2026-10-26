import { describe, expect, it } from "vitest";
import { MAX_THREADS, THREADS_ERROR, validThreads } from "./threads";

describe("validThreads", () => {
  it.each([1, 8, 32])("accepts %s", (n) => {
    expect(validThreads(n)).toBe(true);
  });

  it.each([0, 33, 2.5, -1, NaN, Infinity, "4", null, undefined])("rejects %s", (n) => {
    expect(validThreads(n)).toBe(false);
  });

  it("the refusal notice names the range", () => {
    expect(MAX_THREADS).toBe(32);
    expect(THREADS_ERROR).toBe("Threads must be a whole number from 1 to 32");
  });
});
