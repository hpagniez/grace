import { describe, expect, it } from "vitest";
import { placeTip } from "./placeTip";

const view = { width: 1000, height: 800 };
const tip = { width: 100, height: 20 };

describe("placeTip", () => {
  describe("below (the default)", () => {
    it("sits 6 px below the target and centred on it", () => {
      const target = { top: 100, left: 200, bottom: 124, right: 240 };
      expect(placeTip(target, tip, view)).toEqual({ top: 130, left: 170, side: "below" });
      expect(placeTip(target, tip, view, "below")).toEqual(placeTip(target, tip, view));
    });

    it("flips above when it would cross 8 px from the bottom", () => {
      const target = { top: 760, left: 200, bottom: 784, right: 240 };
      expect(placeTip(target, tip, view)).toEqual({ top: 734, left: 170, side: "above" });
    });

    it("stays below when it just fits", () => {
      // bottom + 6 + height = 792 = view.height - 8
      const target = { top: 740, left: 200, bottom: 766, right: 240 };
      expect(placeTip(target, tip, view).side).toBe("below");
    });

    it("clamps left to 8 at the left edge", () => {
      const target = { top: 100, left: 0, bottom: 124, right: 10 };
      expect(placeTip(target, tip, view).left).toBe(8);
    });

    it("clamps left to view.width - 8 - tip.width at the right edge", () => {
      const target = { top: 100, left: 980, bottom: 124, right: 990 };
      expect(placeTip(target, tip, view).left).toBe(892);
    });

    it("never goes above 8 px from the top when flipped with no room either way", () => {
      const target = { top: 4, left: 200, bottom: 790, right: 240 };
      expect(placeTip(target, tip, view)).toMatchObject({ top: 8, side: "above" });
    });
  });

  describe("beside: a row's tooltip stays on its own row", () => {
    // a 24 px run button at the right end of a tree row
    const run = { top: 300, left: 700, bottom: 324, right: 724 };

    it("left sits 6 px left of the target, centred on it from top to bottom", () => {
      expect(placeTip(run, tip, view, "left")).toEqual({ top: 302, left: 594, side: "left" });
    });

    it("right sits 6 px right of the target, centred on it from top to bottom", () => {
      expect(placeTip(run, tip, view, "right")).toEqual({ top: 302, left: 730, side: "right" });
    });

    it("left flips right when it would come within 8 px of the window's left", () => {
      const target = { top: 300, left: 100, bottom: 324, right: 124 };
      // 100 - 6 - 100 = -6 < 8
      expect(placeTip(target, tip, view, "left")).toEqual({ top: 302, left: 130, side: "right" });
    });

    it("left stays left when it just fits", () => {
      const target = { top: 300, left: 114, bottom: 324, right: 138 }; // 114 - 6 - 100 = 8
      expect(placeTip(target, tip, view, "left")).toMatchObject({ left: 8, side: "left" });
    });

    it("right flips left when it would come within 8 px of the window's right", () => {
      const target = { top: 300, left: 870, bottom: 324, right: 894 };
      // 894 + 6 + 100 = 1000 > 992
      expect(placeTip(target, tip, view, "right")).toEqual({ top: 302, left: 764, side: "left" });
    });

    it("right stays right when it just fits", () => {
      const target = { top: 300, left: 862, bottom: 324, right: 886 }; // 886 + 6 + 100 = 992
      expect(placeTip(target, tip, view, "right")).toMatchObject({ left: 892, side: "right" });
    });

    it("stays 8 px inside the window when there is no room on either side", () => {
      const wide = { top: 300, left: 50, bottom: 324, right: 950 };
      expect(placeTip(wide, tip, view, "left")).toMatchObject({ left: 892, side: "right" });
      expect(placeTip(wide, tip, view, "right")).toMatchObject({ left: 8, side: "left" });
    });

    it("stays 8 px inside the window's top and bottom", () => {
      const tall = { width: 100, height: 60 };
      const top = { top: 0, left: 700, bottom: 24, right: 724 };
      expect(placeTip(top, tall, view, "left").top).toBe(8);
      const bottom = { top: 780, left: 700, bottom: 800, right: 724 };
      expect(placeTip(bottom, tall, view, "right").top).toBe(732); // 800 - 8 - 60
    });
  });

  describe("over: cut text in a row", () => {
    const label = { top: 300, left: 80, bottom: 324, right: 400 };

    it("lies on the target's own top-left", () => {
      expect(placeTip(label, tip, view, "over")).toEqual({ top: 300, left: 80, side: "over" });
    });

    it("stays 8 px inside the window", () => {
      const corner = { top: 2, left: 3, bottom: 20, right: 200 };
      expect(placeTip(corner, tip, view, "over")).toEqual({ top: 8, left: 8, side: "over" });
      const far = { top: 790, left: 950, bottom: 800, right: 1000 };
      expect(placeTip(far, tip, view, "over")).toEqual({ top: 772, left: 892, side: "over" });
    });
  });
});
