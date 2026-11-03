/** What the console's find and error stepping look for (console search spec R1, R3): pure, so Node tests it. */

/** A match of the find text: `line` indexes the lines searched, `start` and `end` the line's characters. */
export interface Match {
  line: number;
  start: number;
  end: number;
}

/** A line Karate logged at ERROR level: the test that colours it red. */
export const isErrorLine = (line: string) => line.includes(" ERROR ");

/** Every match of `query` as plain text, case aside, never overlapping. */
export function findMatches(lines: readonly string[], query: string): Match[] {
  if (!query) return [];
  // a case-insensitive pattern keeps offsets on the line as written: lower-casing can change a line's length
  const re = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi");
  const found: Match[] = [];
  lines.forEach((text, line) => {
    for (const m of text.matchAll(re))
      found.push({ line, start: m.index, end: m.index + m[0].length });
  });
  return found;
}

/**
 * The ERROR lines, and the failed tests of Karate's end-of-run summary: `    - <name>` between `failed features:`
 * and the next line that starts with `=`. Karate's text, not a contract: e2e runs real Karate against it.
 */
export function errorLines(lines: readonly string[]): number[] {
  const found: number[] = [];
  let summary = false;
  lines.forEach((text, i) => {
    if (text === "failed features:") summary = true;
    else if (text.startsWith("=")) summary = false;
    else if (summary ? /^ {4}- /.test(text) : isErrorLine(text)) found.push(i);
  });
  return found;
}

/**
 * Where stepping starts with nothing current: the first item at or below line `top` going down, the last above it
 * going up, each wrapping round; -1 for no item.
 */
export function startIndex(lineOf: readonly number[], top: number, dir: 1 | -1): number {
  if (lineOf.length === 0) return -1;
  const i = dir === 1 ? lineOf.findIndex((l) => l >= top) : lineOf.findLastIndex((l) => l < top);
  return i >= 0 ? i : dir === 1 ? 0 : lineOf.length - 1;
}

/** The next index going `dir`, wrapping round. */
export const stepIndex = (count: number, current: number, dir: 1 | -1) =>
  (current + dir + count) % count;
