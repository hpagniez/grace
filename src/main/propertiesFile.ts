import { readFileSync, statSync } from "node:fs";
import path from "node:path";
import type { ImportEntry, ImportResult } from "../shared/env";

const MAX_BYTES = 1024 * 1024;
const BOM = [0xef, 0xbb, 0xbf];
const SPACE = new Set([" ", "\t", "\f"]);
const ESCAPED: Record<string, string> = { t: "\t", n: "\n", r: "\r", f: "\f" };

const trimStart = (s: string) => {
  let i = 0;
  while (i < s.length && SPACE.has(s[i])) i++;
  return s.slice(i);
};

/** Whether `s` ends in an odd number of backslashes: its line goes on in the next one. */
const continues = (s: string) => {
  let n = 0;
  while (n < s.length && s[s.length - 1 - n] === "\\") n++;
  return n % 2 === 1;
};

/** `\t`, `\n`, `\r`, `\f`, `\uXXXX` and `\<char>` as `java.util.Properties` reads them. */
function unescape(s: string, line: number): string {
  let out = "";
  for (let i = 0; i < s.length; i++) {
    if (s[i] !== "\\") {
      out += s[i];
      continue;
    }
    const c = s[++i];
    if (c === "u") {
      const hex = s.slice(i + 1, i + 5);
      if (!/^[0-9A-Fa-f]{4}$/.test(hex)) throw new Error(`bad \\uXXXX escape on line ${line}`);
      out += String.fromCharCode(parseInt(hex, 16));
      i += 4;
    } else if (c !== undefined) out += ESCAPED[c] ?? c;
  }
  return out;
}

/**
 * The keys of a `.properties` text, by the rules of `java.util.Properties.load`: comment and blank
 * lines, `=`, `:` or white space between key and value, continued lines and escapes. A repeated key
 * keeps its last value at its first place. Throws on a malformed `\uXXXX`, naming the line its
 * property starts on, never a value.
 */
export function parseProperties(text: string): ImportEntry[] {
  const lines = text.split(/\r\n|\r|\n/);
  const found = new Map<string, string>();
  for (let i = 0; i < lines.length; i++) {
    let logical = trimStart(lines[i]);
    if (logical === "" || logical[0] === "#" || logical[0] === "!") continue;
    const start = i + 1;
    while (continues(logical)) {
      logical = logical.slice(0, -1);
      if (++i >= lines.length) break;
      logical += trimStart(lines[i]);
    }
    let end = 0; // the key's end
    let escaped = false;
    while (end < logical.length) {
      const c = logical[end];
      if (!escaped && (c === "=" || c === ":" || SPACE.has(c))) break;
      escaped = c === "\\" && !escaped;
      end++;
    }
    let from = end; // the value's start
    while (from < logical.length && SPACE.has(logical[from])) from++;
    if (logical[from] === "=" || logical[from] === ":") from++;
    while (from < logical.length && SPACE.has(logical[from])) from++;
    found.set(unescape(logical.slice(0, end), start), unescape(logical.slice(from), start));
  }
  return [...found].map(([key, value]) => ({ key, value }));
}

/**
 * The name and keys of the `.properties` file at `file`, read once and never written, or a message
 * for the window. Never throws, and no message holds a value.
 */
export function readPropertiesFile(file: string): ImportResult {
  const name = path.basename(file);
  let bytes: Buffer;
  try {
    if (statSync(file).size > MAX_BYTES) return { error: `${name} is too large to import` };
    bytes = readFileSync(file);
  } catch (err) {
    return { error: `${name} could not be read: ${(err as Error).message}` };
  }
  // Windows editors write a byte order mark; Java would keep it in the first key's name
  if (BOM.every((b, i) => bytes[i] === b)) bytes = bytes.subarray(BOM.length);
  let entries: ImportEntry[];
  try {
    entries = parseProperties(bytes.toString("latin1")); // as Properties.load(InputStream)
  } catch (err) {
    return { error: `${name} could not be imported: ${(err as Error).message}` };
  }
  return entries.length ? { file: name, entries } : { error: `${name} has no properties` };
}
