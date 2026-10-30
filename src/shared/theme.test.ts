import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { readTokens } from "../../test/cssTokens";
import { GROUND, THEMES, parseTheme } from "./theme";

describe("parseTheme", () => {
  it("keeps the three themes", () => {
    for (const t of THEMES) expect(parseTheme(t)).toBe(t);
    expect(parseTheme("dark")).toBe("dark");
  });

  it("reads anything else as system", () => {
    for (const x of ["purple", "", undefined, null, 1, {}, ["dark"]])
      expect(parseTheme(x)).toBe("system");
  });
});

describe("GROUND", () => {
  const { light, dark } = readTokens();

  it("equals the ground tokens of src/index.css", () => {
    expect(GROUND).toEqual({ light: light.ground, dark: dark.ground });
  });

  it("is what index.html's startup screen draws, in the ink of src/index.css", () => {
    const html = readFileSync(new URL("../../index.html", import.meta.url), "utf8").toLowerCase();
    for (const hex of [light.ground, dark.ground, light.ink, dark.ink]) expect(html).toContain(hex);
  });
});
