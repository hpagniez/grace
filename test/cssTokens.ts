import { readFileSync } from "node:fs";
import path from "node:path";

// A path, not a URL: jsdom replaces the URL class, which readFileSync then refuses.
export const INDEX_CSS = path.resolve(import.meta.dirname, "../src/index.css");

/** The text between the braces after the first match of `header` (nested braces balanced). */
function block(css: string, header: RegExp): string {
  const m = header.exec(css);
  if (!m) throw new Error(`no block matches ${header}`);
  const start = m.index + m[0].length;
  let depth = 1;
  let end = start;
  for (; end < css.length && depth > 0; end++) {
    if (css[end] === "{") depth++;
    else if (css[end] === "}") depth--;
  }
  return css.slice(start, end - 1);
}

const colors = (css: string): Record<string, string> =>
  Object.fromEntries(
    [...css.matchAll(/--color-([a-z-]+):\s*(#[0-9a-f]{6})\s*;/gi)].map((m) => [
      m[1],
      m[2].toLowerCase(),
    ]),
  );

/**
 * The `#rrggbb` colour tokens of src/index.css: the light ones from `@theme`, the dark overrides from the dark media block.
 * `darkProps` lists every custom property (`--name`) the dark block declares, colour or not.
 */
export function readTokens() {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const light = colors(block(css, /@theme\s*\{/));
  const darkBlock = block(css, /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/);
  const darkOverrides = colors(darkBlock);
  const darkProps = [...darkBlock.matchAll(/(?:^|[{;\s])(--[\w-]+)\s*:/g)].map((m) => m[1]);
  return { light, darkOverrides, darkProps, dark: { ...light, ...darkOverrides } };
}

/** A duration token of src/index.css (`--duration-fast: 100ms`) in milliseconds. */
export function readDuration(name: string): number {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const m = new RegExp(String.raw`--duration-${name}:\s*(\d+)ms\s*;`).exec(css);
  if (!m) throw new Error(`no --duration-${name} in src/index.css`);
  return Number(m[1]);
}
