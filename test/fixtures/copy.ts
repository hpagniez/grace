import { cpSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const projects = join(dirname(fileURLToPath(import.meta.url)), "projects");

/** Copies a fixture project to a fresh temp dir and returns it. `broken` = `clean` plus broken/syntax.feature. */
export function copyFixture(name: "clean" | "broken" | "maven"): string {
  const dest = mkdtempSync(join(tmpdir(), `kk-${name}-`));
  cpSync(join(projects, name === "broken" ? "clean" : name), dest, { recursive: true });
  if (name === "broken") cpSync(join(projects, "broken-extra"), dest, { recursive: true });
  return dest;
}
