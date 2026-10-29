import { describe, expect, it } from "vitest";
import { placeTip } from "./placeTip";

const view = { width: 1000, height: 800 };
const tip = { width: 100, height: 20 };

describe("placeTip", () => {
  it("sits 6 px below the target and centred on it", () => {
    const target = { top: 100, left: 200, bottom: 124, width: 40 };
    expect(placeTip(target, tip, view)).toEqual({ top: 130, left: 170, above: false });
  });

  it("flips above when it would cross 8 px from the bottom", () => {
    const target = { top: 760, left: 200, bottom: 784, width: 40 };
    expect(placeTip(target, tip, view)).toEqual({ top: 734, left: 170, above: true });
  });

  it("stays below when it just fits", () => {
    // bottom + 6 + height = 792 = view.height - 8
    const target = { top: 740, left: 200, bottom: 766, width: 40 };
    expect(placeTip(target, tip, view).above).toBe(false);
  });

  it("clamps left to 8 at the left edge", () => {
    const target = { top: 100, left: 0, bottom: 124, width: 10 };
    expect(placeTip(target, tip, view).left).toBe(8);
  });

  it("clamps left to view.width - 8 - tip.width at the right edge", () => {
    const target = { top: 100, left: 980, bottom: 124, width: 10 };
    expect(placeTip(target, tip, view).left).toBe(892);
  });

  it("never goes above 8 px from the top when flipped with no room either way", () => {
    const target = { top: 4, left: 200, bottom: 790, width: 40 };
    expect(placeTip(target, tip, view)).toMatchObject({ top: 8, above: true });
  });
});
