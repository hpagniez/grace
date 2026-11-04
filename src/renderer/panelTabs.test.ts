import { describe, expect, it } from "vitest";
import { loadTabOrder, moveTab, saveTabOrder, TABS_KEY, type TabId } from "./panelTabs";

const DEFAULT = ["console", "history", "http"];
const stored = (value: string | null) => ({ getItem: () => value });

describe("moveTab (reorder panel tabs R2.1, R3.1)", () => {
  const order: TabId[] = ["console", "history", "http"];

  it("puts the tab in the place asked and shifts the ones between", () => {
    expect(moveTab(order, "console", 2)).toEqual(["history", "http", "console"]);
    expect(moveTab(order, "http", 0)).toEqual(["http", "console", "history"]);
    expect(moveTab(order, "history", 0)).toEqual(["history", "console", "http"]);
  });

  it("stops at the ends and returns the same list when nothing moves", () => {
    expect(moveTab(order, "console", -1)).toBe(order);
    expect(moveTab(order, "http", 3)).toBe(order);
    expect(moveTab(order, "history", 1)).toBe(order);
  });

  it("leaves the list it was given as it was", () => {
    moveTab(order, "console", 2);
    expect(order).toEqual(DEFAULT);
  });
});

describe("loadTabOrder (R4.2, R4.3)", () => {
  it("reads a saved order", () => {
    expect(loadTabOrder(stored('["http","console","history"]'))).toEqual([
      "http",
      "console",
      "history",
    ]);
  });

  it.each([
    null,
    "not JSON",
    '["history","console"]',
    '["history","console","http","x"]',
    '["history","history","http"]',
    '["history","console","logs"]',
    '["history","console","variables"]',
    '{"console":0}',
    "7",
    "null",
  ])("falls back to the default for %s", (raw) => {
    expect(loadTabOrder(stored(raw))).toEqual(DEFAULT);
  });

  it("falls back to the default when storage throws", () => {
    const broken = {
      getItem: () => {
        throw new Error("denied");
      },
    };
    expect(loadTabOrder(broken)).toEqual(DEFAULT);
  });
});

describe("saveTabOrder", () => {
  it("writes the list as JSON under its key", () => {
    const writes: Record<string, string> = {};
    saveTabOrder(["history", "http", "console"], {
      setItem: (k, v) => void (writes[k] = v),
    });
    expect(writes).toEqual({ [TABS_KEY]: '["history","http","console"]' });
    expect(TABS_KEY).toBe("grace.panelTabs");
  });

  it("never throws: an order that cannot be saved is only forgotten", () => {
    expect(() =>
      saveTabOrder(["history", "http", "console"], {
        setItem: () => {
          throw new Error("full");
        },
      }),
    ).not.toThrow();
  });
});
