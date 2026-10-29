import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { INDEX_CSS, readTokens } from "../../test/cssTokens";
import { danger, dangerDialog, primary, primaryDialog } from "./buttons";

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
  "on-accent",
  "primary",
  "on-primary",
  "primary-hover",
  "on-danger",
  "brand",
  "focus",
  "failed-hover",
  ...STATUSES,
  ...STATUSES.map((s) => `${s}-tint`),
];
/** One value in both themes: the logo's green and the primary button (the person's call, R1.3). */
const SAME_IN_BOTH = ["brand", "primary", "on-primary", "primary-hover"];

describe("filled buttons", () => {
  it("fill with a solid token on hover, not a translucent one", () => {
    for (const b of [primary, primaryDialog]) expect(b).toContain("enabled:hover:bg-primary-hover");
    for (const b of [danger, dangerDialog]) expect(b).toContain("enabled:hover:bg-failed-hover");
    expect(`${primary} ${danger} ${primaryDialog} ${dangerDialog}`).not.toMatch(/hover:bg-\S+\//);
  });

  it("make the primary buttons brand green, whatever the theme", () => {
    for (const b of [primary, primaryDialog]) {
      expect(b).toContain("bg-primary");
      expect(b).toContain("text-on-primary");
      expect(b).not.toMatch(/accent/);
    }
  });
});

describe("colour tokens (spec R1)", () => {
  const { light, darkOverrides, darkProps } = readTokens();

  it("defines every token in the light theme", () => {
    expect(
      Object.keys(light)
        .filter((n) => TOKENS.includes(n))
        .sort(),
    ).toEqual([...TOKENS].sort());
  });

  // `readTokens().dark` falls back to light for a token the dark block lacks, so check the block itself.
  it("overrides every token in the dark theme except the ones that are the same", () => {
    expect(Object.keys(darkOverrides).sort()).toEqual(
      TOKENS.filter((n) => !SAME_IN_BOTH.includes(n)).sort(),
    );
  });

  it("gives the primary button one definition, with no dark override", () => {
    const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const n of ["primary", "on-primary", "primary-hover"])
      expect(css.match(new RegExp(`--color-${n}:`, "g")), `--color-${n}`).toHaveLength(1);
    expect([light.primary, light["on-primary"], light["primary-hover"]]).toEqual([
      "#3fcf6e",
      "#151821",
      "#5fd98a",
    ]);
  });

  // the person: "UX/UI and components must not change from one theme to the other; only colour must change"
  it("lets the dark theme override colours, shadows and the backdrop only", () => {
    const allowed = (n: string) =>
      n.startsWith("--color-") || ["--shadow-float", "--backdrop"].includes(n);
    expect(darkProps.filter((n) => !allowed(n))).toEqual([]);
    expect(darkProps).toContain("--color-ground");
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
        expect(
          contrast(t["on-primary"], t.primary),
          "on-primary on primary",
        ).toBeGreaterThanOrEqual(4.5);
        expect(contrast(t["on-danger"], t.failed), "on-danger on failed").toBeGreaterThanOrEqual(
          4.5,
        );
        // the hover fills are solid, so the text keeps its ratio while the pointer is over the button
        expect(
          contrast(t["on-primary"], t["primary-hover"]),
          "on-primary on primary-hover",
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

describe("the sheet (spec R3.3)", () => {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

  it("is inset 8 px, rounded 16 px and edged 1 px, once for both themes", () => {
    for (const [name, value] of [
      ["inset", "8px"],
      ["radius", "16px"],
      ["side", "1px"],
    ])
      expect(css.match(new RegExp(`--sheet-${name}:\\s*([^;]+);`, "g"))).toEqual([
        `--sheet-${name}: ${value};`,
      ]);
  });
});

describe("the focus ring (spec R10.1)", () => {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

  it("is 2 px in the focus colour, offset 2 px", () => {
    expect(css).toMatch(
      /:focus-visible\s*\{\s*outline:\s*2px solid var\(--color-focus\);\s*outline-offset:\s*2px;/,
    );
  });

  // transition-colors also moves outline-color: without this the ring fades in from the text colour
  it("has its colour before focus arrives", () => {
    expect(css).toMatch(/\*\s*\{\s*outline-color:\s*var\(--color-focus\);\s*\}/);
  });
});

describe("the palette (spec R1.1)", () => {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

  it("drops Tailwind's default colours and keeps white, black, transparent and current", () => {
    expect(css).toMatch(/@theme\s*\{\s*--color-\*:\s*initial;/);
    for (const name of ["white", "black", "transparent", "current"])
      expect(css, name).toMatch(new RegExp(`--color-${name}:`));
  });
});

describe("the native controls' scheme (spec R1)", () => {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

  // without it a radio, a checkbox, a spinner or a select popup stays light in the dark theme
  it("is light in the root and dark in the dark theme's root", () => {
    expect(css).toMatch(/@layer base\s*\{\s*:root\s*\{\s*color-scheme:\s*light;/);
    expect(css).toMatch(
      /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{\s*color-scheme:\s*dark;/,
    );
  });
});
