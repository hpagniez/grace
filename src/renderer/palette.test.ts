import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Tailwind's default palette is gone (spec R1.1, R9.3): colours come from the tokens in src/index.css.
const PALETTE =
  /\b(slate|gray|zinc|neutral|stone|mauve|olive|mist|taupe|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3}\b/;
const DARK_VARIANT = /\bdark:/;
const TITLE_ATTRIBUTE = /\btitle=/; // a native tooltip: use Tooltip (R9.4)
// The tests name `dark:` and `title=` in their own "no raw class" regexes, so those two checks skip them.
const SOURCE = /^(?!.*\.test\.tsx?$)/;
const TSX_SOURCE = /^(?!.*\.test\.).*\.tsx$/; // a `title` attribute is JSX

const dir = new URL(".", import.meta.url);
const files = readdirSync(dir, { recursive: true, encoding: "utf8" }).filter((f) =>
  /\.tsx?$/.test(f),
);

/** `file:line: text` for every line of the `.ts` and `.tsx` files under src/renderer that matches `re`. */
function offenders(re: RegExp, only = /./) {
  return files
    .filter((f) => only.test(f))
    .flatMap((f) =>
      readFileSync(fileURLToPath(new URL(f, dir)), "utf8")
        .split(/\r?\n/)
        .flatMap((line, i) => (re.test(line) ? [`${f}:${i + 1}: ${line.trim()}`] : [])),
    );
}

describe("src/renderer", () => {
  it("scans the renderer's files", () => {
    expect(files.length).toBeGreaterThan(40);
  });

  it("knows what it looks for", () => {
    // built apart, so this file does not match itself
    for (const c of ["zinc", "red", "sky", "mauve", "olive", "mist", "taupe"])
      expect(`text-${c}-500`).toMatch(PALETTE);
    for (const c of ["text-ink", "bg-passed-tint", "w-100", "rounded-md"])
      expect(c).not.toMatch(PALETTE);
    expect("dark:bg-surface").toMatch(DARK_VARIANT);
    expect('<button title="Run">').toMatch(TITLE_ATTRIBUTE);
  });

  it("filters the production files, and only those", () => {
    for (const only of [SOURCE, TSX_SOURCE]) {
      expect(only.test("Tree.tsx")).toBe(true);
      expect(only.test("Tree.test.tsx")).toBe(false);
      expect(files.filter((f) => only.test(f)).length).toBeGreaterThan(20);
    }
  });

  it("uses no raw palette class", () => {
    expect(offenders(PALETTE)).toEqual([]);
  });

  it("uses no dark: variant", () => {
    expect(offenders(DARK_VARIANT, SOURCE)).toEqual([]);
  });

  it("uses no native title attribute", () => {
    expect(offenders(TITLE_ATTRIBUTE, TSX_SOURCE)).toEqual([]);
  });
});
