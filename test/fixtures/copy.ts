import { cpSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// __dirname, not import.meta: Playwright loads this file as CommonJS, Vitest provides __dirname too.
const projects = join(__dirname, "projects");

/**
 * Copies a fixture project to a fresh temp dir and returns it. `broken` = `clean` plus broken/syntax.feature.
 * `edge` = outline tables with comment and blank lines, and a dynamic Examples table.
 * `tags` = feature, outline and Examples tags on one outline and one scenario, an `@ignore` Examples
 * block, and a `fast/` folder named like a tag.
 */
export function copyFixture(name: "clean" | "broken" | "maven" | "edge" | "tags"): string {
  const dest = mkdtempSync(join(tmpdir(), `kk-${name}-`));
  cpSync(join(projects, name === "broken" ? "clean" : name), dest, { recursive: true });
  if (name === "broken") cpSync(join(projects, "broken-extra"), dest, { recursive: true });
  return dest;
}
