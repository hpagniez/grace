import { describe, expect, test } from "vitest";
import {
  clampPanel,
  keyHeight,
  loadPanelHeight,
  maxPanel,
  PANEL_KEY,
  savePanelHeight,
} from "./panelHeight";

const stored = (value: string | null) => ({ getItem: () => value });

describe("panel height rules", () => {
  test("maxPanel leaves MAIN_MIN to main", () => {
    expect(maxPanel(500)).toBe(380);
    expect(maxPanel(150)).toBe(96);
  });

  test("clampPanel keeps within limits", () => {
    expect(clampPanel(50, 300)).toBe(96);
    expect(clampPanel(400, 300)).toBe(300);
    expect(clampPanel(200.6, 300)).toBe(201);
  });

  test("keyHeight steps and jumps", () => {
    expect(keyHeight("ArrowUp", 192, 400)).toBe(208);
    expect(keyHeight("ArrowDown", 192, 400)).toBe(176);
    expect(keyHeight("Home", 192, 400)).toBe(96);
    expect(keyHeight("End", 192, 400)).toBe(400);
  });

  test("keyHeight stays at a limit", () => {
    expect(keyHeight("ArrowDown", 96, 400)).toBe(96);
    expect(keyHeight("ArrowUp", 400, 400)).toBe(400);
  });

  test("keyHeight ignores other keys", () => {
    for (const key of ["Tab", "a", "Enter"]) expect(keyHeight(key, 192, 400)).toBeNull();
  });
});

describe("saved panel height", () => {
  test("loadPanelHeight reads a saved height", () => {
    expect(loadPanelHeight(stored("300"))).toBe(300);
    expect(loadPanelHeight(stored("2000"))).toBe(2000); // clamped when shown, not here
  });

  test("loadPanelHeight falls back to the default", () => {
    for (const v of [null, "abc", "-5", "50", "NaN", "Infinity", "1e999"])
      expect(loadPanelHeight(stored(v))).toBe(192);
    const throwing = {
      getItem: () => {
        throw new Error("blocked");
      },
    };
    expect(loadPanelHeight(throwing)).toBe(192);
  });

  test("savePanelHeight writes a rounded integer", () => {
    const writes: [string, string][] = [];
    savePanelHeight(300.4, { setItem: (k, v) => void writes.push([k, v]) });
    expect(writes).toEqual([[PANEL_KEY, "300"]]);
    const throwing = {
      setItem: () => {
        throw new Error("quota");
      },
    };
    expect(() => savePanelHeight(300, throwing)).not.toThrow();
  });
});
