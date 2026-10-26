import { describe, expect, it } from "vitest";
import { placeWindow } from "./window";

const rect = (x: number, y: number, width: number, height: number) => ({ x, y, width, height });
const main = rect(0, 0, 1920, 1040);
const second = rect(1920, 0, 2560, 1400);
const areas = [main, second];

describe("placeWindow", () => {
  it.each([
    [rect(2000, 100, 1200, 800), rect(2000, 100, 1200, 800)],
    [rect(3900, 100, 1200, 800), rect(3280, 100, 1200, 800)],
    [rect(1800, 100, 1200, 800), rect(1920, 100, 1200, 800)],
    [rect(2000, 900, 1200, 800), rect(2000, 600, 1200, 800)],
    [rect(2000, 100, 3000, 1600), rect(1920, 0, 2560, 1400)],
    [rect(5000, 100, 1200, 800), null],
    [rect(100, -900, 1200, 800), null],
  ])("places %o at %o on two screens", (saved, placed) => {
    expect(placeWindow(saved, areas)).toEqual(placed);
  });

  it("keeps a window on a screen left of the main one", () => {
    const left = rect(-1920, 0, 1920, 1040);
    expect(placeWindow(rect(-1800, 50, 700, 500), [main, left])).toEqual(rect(-1800, 50, 700, 500));
  });

  it("equal overlap picks the first work area", () => {
    expect(placeWindow(rect(1620, 100, 600, 400), areas)).toEqual(rect(1320, 100, 600, 400));
  });

  it("rounds fractional bounds", () => {
    const placed = placeWindow(rect(100.4, 50.6, 700.5, 500.2), [main])!;
    for (const n of Object.values(placed)) expect(Number.isInteger(n), String(n)).toBe(true);
  });

  it("no work areas gives first-start bounds", () => {
    expect(placeWindow(rect(0, 0, 700, 500), [])).toBeNull();
  });
});
