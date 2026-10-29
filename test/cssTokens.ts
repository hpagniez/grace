import { readFileSync } from "node:fs";

export const INDEX_CSS = new URL("../src/index.css", import.meta.url);

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
    [...css.matchAll(/--color-([a-z-]+):\s*(#[0-9a-f]{6})\s*;/g)].map((m) => [m[1], m[2]]),
  );

/** The `#rrggbb` colour tokens of src/index.css: the light ones from `@theme`, the dark overrides from the dark media block. */
export function readTokens() {
  const css = readFileSync(INDEX_CSS, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  const light = colors(block(css, /@theme\s*\{/));
  const darkOverrides = colors(block(css, /@media\s*\(prefers-color-scheme:\s*dark\)\s*\{/));
  return { light, darkOverrides, dark: { ...light, ...darkOverrides } };
}
