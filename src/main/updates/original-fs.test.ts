import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

// Electron's asar-aware fs takes an unpacked app.asar for a folder: access rejects and rm never
// settles, so an update hung at Unpacking… (#131). These tests run in Node, which has no asar
// support, so only this check keeps the updater on original-fs.
const dir = new URL(".", import.meta.url);
const sources = readdirSync(dir).filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts"));

describe("src/main/updates", () => {
  it.each(sources)("%s takes fs from original-fs, not node:fs", (f) => {
    expect(readFileSync(fileURLToPath(new URL(f, dir)), "utf8")).not.toMatch(
      /from "(node:)?fs(\/promises)?"/,
    );
  });
});
