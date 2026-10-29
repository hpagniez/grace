import { describe, expect, it } from "vitest";
import {
  btn,
  btnDialog,
  btnIconSmall,
  danger,
  dangerDialog,
  primary,
  primaryDialog,
} from "./buttons";

describe("dialog buttons (R2.2)", () => {
  it("are the standard ones in 14 px text, with nothing to override", () => {
    for (const [standard, dialog] of [
      [btn, btnDialog],
      [primary, primaryDialog],
      [danger, dangerDialog],
    ]) {
      expect(dialog).toBe(standard.replace("text-sm", "text-base"));
      expect(dialog).not.toContain("text-sm");
      expect(dialog).not.toContain("!");
    }
  });
});

describe("btnIconSmall", () => {
  it("is 24 × 24 (R10.3) and borderless", () => {
    expect(btnIconSmall).toContain("size-6");
    expect(btnIconSmall).not.toMatch(/\b(border|h-\d|w-\d|px-\d)\b/);
  });
});
