import { describe, expect, it } from "vitest";
import { lastCheckedText } from "./lastChecked";

const now = Date.parse("2026-10-29T12:00:00Z");
const ago = (ms: number) => new Date(now - ms).toISOString();

describe("lastCheckedText", () => {
  it("says how long ago the last successful check was", () => {
    expect(lastCheckedText(ago(2 * 60_000), now)).toBe("Last checked 2 minutes ago");
    expect(lastCheckedText(ago(3 * 3_600_000), now)).toBe("Last checked 3 hours ago");
    expect(lastCheckedText(ago(2 * 86_400_000), now)).toBe("Last checked 2 days ago");
  });

  it("uses plain words where English has them", () => {
    expect(lastCheckedText(ago(86_400_000), now)).toBe("Last checked yesterday");
    expect(lastCheckedText(ago(10_000), now)).toBe("Last checked this minute");
  });

  it("never says the future", () => {
    expect(lastCheckedText(ago(-5 * 60_000), now)).toBe("Last checked this minute");
  });

  it("says nothing before a first successful check", () => {
    expect(lastCheckedText(null, now)).toBe("");
    expect(lastCheckedText("not a date", now)).toBe("");
  });
});
