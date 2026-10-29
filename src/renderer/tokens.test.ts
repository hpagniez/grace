import { describe, expect, it } from "vitest";
import { readTokens } from "../../test/cssTokens";
import { danger, primary } from "./buttons";

/** WCAG 2 relative luminance of `#rrggbb`. */
function luminance(hex: string) {
  const [r, g, b] = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG 2 contrast ratio of two `#rrggbb` colours. */
function contrast(a: string, b: string) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const STATUSES = ["passed", "failed", "skipped", "running", "cancelled"];
const TOKENS = [
  "ground",
  "surface",
  "raised",
  "sunken",
  "hover",
  "selected",
  "line",
  "control",
  "ink",
  "muted",
  "accent",
  "accent-hover",
  "on-accent",
  "on-danger",
  "brand",
  "focus",
  "play",
  "failed-hover",
  ...STATUSES,
  ...STATUSES.map((s) => `${s}-tint`),
];

describe("filled buttons on hover", () => {
  it("fill with a solid token, not a translucent one", () => {
    expect(primary).toContain("enabled:hover:bg-accent-hover");
    expect(danger).toContain("enabled:hover:bg-failed-hover");
    expect(`${primary} ${danger}`).not.toMatch(/hover:bg-\S+\//);
  });
});

describe("colour tokens (spec R1)", () => {
  const { light, darkOverrides } = readTokens();

  it("defines every token in the light theme", () => {
    expect(
      Object.keys(light)
        .filter((n) => TOKENS.includes(n))
        .sort(),
    ).toEqual([...TOKENS].sort());
  });

  // `readTokens().dark` falls back to light for a token the dark block lacks, so check the block itself.
  it("overrides every token in the dark theme except brand, the one that is the same", () => {
    expect(Object.keys(darkOverrides).sort()).toEqual(TOKENS.filter((n) => n !== "brand").sort());
  });

  for (const theme of ["light", "dark"] as const) {
    const t = readTokens()[theme];

    describe(theme, () => {
      it("keeps text at 4.5:1 on every background", () => {
        for (const bg of ["ground", "surface", "sunken", "hover", "selected", "raised"])
          for (const fg of ["ink", "muted"])
            expect(contrast(t[fg], t[bg]), `${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5);
      });

      it("keeps each status readable on surface, on its tint and on a selected row", () => {
        for (const s of STATUSES) {
          expect(contrast(t[s], t.surface), `${s} on surface`).toBeGreaterThanOrEqual(4.5);
          expect(contrast(t[s], t[`${s}-tint`]), `${s} on its tint`).toBeGreaterThanOrEqual(4.5);
          expect(contrast(t[s], t.selected), `${s} on selected`).toBeGreaterThanOrEqual(3);
        }
      });

      it("keeps the filled buttons' text readable", () => {
        expect(contrast(t["on-accent"], t.accent), "on-accent on accent").toBeGreaterThanOrEqual(
          4.5,
        );
        expect(contrast(t["on-danger"], t.failed), "on-danger on failed").toBeGreaterThanOrEqual(
          4.5,
        );
        // the hover fills are solid, so the text keeps its ratio while the pointer is over the button
        expect(
          contrast(t["on-accent"], t["accent-hover"]),
          "on-accent on accent-hover",
        ).toBeGreaterThanOrEqual(4.5);
        expect(
          contrast(t["on-danger"], t["failed-hover"]),
          "on-danger on failed-hover",
        ).toBeGreaterThanOrEqual(4.5);
      });

      it("keeps control borders and the focus ring at 3:1", () => {
        for (const bg of ["surface", "ground"])
          for (const fg of ["control", "focus"])
            expect(contrast(t[fg], t[bg]), `${fg} on ${bg}`).toBeGreaterThanOrEqual(3);
      });
    });
  }
});
