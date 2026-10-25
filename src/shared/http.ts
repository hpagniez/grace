// Shared by main and renderer: keep this file free of Node imports.
// The HTTP call model and its pure helpers (docs/specs/2026-10-25-http-inspector.md).

export interface HttpHeader {
  name: string;
  value: string;
}

export interface HttpCall {
  method: string;
  /** Null when the step log held no request. */
  url: string | null;
  requestHeaders: HttpHeader[];
  requestBody: string | null;
  /** Null for a call with no response. */
  status: number | null;
  responseHeaders: HttpHeader[];
  responseBody: string | null;
  millis: number | null;
  error: string | null;
  /** Where a called feature made the call, like `tests/common/create-user.feature:8`. */
  via: string | null;
  startTime: number;
  attempt: { n: number; of: number } | null;
}

export type HttpGroup = "1xx" | "2xx" | "3xx" | "4xx" | "5xx" | "none";

export const HTTP_GROUPS: readonly { group: HttpGroup; label: string }[] = [
  { group: "1xx", label: "1xx Informational" },
  { group: "2xx", label: "2xx Success" },
  { group: "3xx", label: "3xx Redirect" },
  { group: "4xx", label: "4xx Client error" },
  { group: "5xx", label: "5xx Server error" },
  { group: "none", label: "No response" },
];

/** The group of a status; null for a code outside 100–599, which counts in All only. */
export function httpGroup(status: number | null): HttpGroup | null {
  if (status === null) return "none";
  return status >= 100 && status <= 599 ? (`${Math.floor(status / 100)}xx` as HttpGroup) : null;
}

const CURL_SETS = new Set(["host", "content-length", "connection"]);
const sq = (s: string) => `'${s.replaceAll("'", "'\\''")}'`;

/** A bash-quoted `curl` command for the call; null when no request was logged. */
export function curlCommand(c: HttpCall): string | null {
  if (c.url === null) return null;
  const method = /^[A-Za-z]+$/.test(c.method) ? c.method : sq(c.method);
  const parts = [`curl -X ${method} ${sq(c.url)}`];
  for (const h of c.requestHeaders)
    if (!CURL_SETS.has(h.name.toLowerCase())) parts.push(`-H ${sq(`${h.name}: ${h.value}`)}`);
  if (c.requestBody) parts.push(`--data-raw ${sq(c.requestBody)}`);
  return parts.join(" ");
}

// Comments first: they may hold a `>`.
const XML_TOKENS = /(<!--[\s\S]*?-->|<[^>]*>)/;
const XML_TAG = /^<(\/?)([^\s/>]+)[^>]*?(\/?)>$/;

/** The body indented with 2 spaces, or null when it is not well-formed enough to trust. */
function indentXml(src: string): string | null {
  if (!src.startsWith("<")) return null;
  const tokens = src
    .split(XML_TOKENS)
    .map((t) => t.trim())
    .filter((t) => t !== "");
  const out: string[] = [];
  const open: string[] = [];
  const line = (s: string) => out.push("  ".repeat(open.length) + s);
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    if (!t.startsWith("<")) {
      if (open.length === 0) return null; // text after the root
      line(t);
    } else if (t.startsWith("<![CDATA[")) {
      return null;
    } else if (t.startsWith("<!--")) {
      if (!t.endsWith("-->")) return null;
      line(t);
    } else if (t.startsWith("<!") || t.startsWith("<?")) {
      line(t);
    } else {
      // A quote still open once the quoted strings are gone: a `>` inside an attribute value split the tag.
      if (/["']/.test(t.replace(/"[^"]*"|'[^']*'/g, ""))) return null;
      const m = XML_TAG.exec(t);
      if (!m || (m[1] && m[3])) return null;
      const [, close, name, selfClose] = m;
      if (close) {
        if (open.pop() !== name) return null;
        line(t);
      } else if (selfClose) {
        line(t);
      } else {
        // `<a>text</a>` and `<a></a>` stay on one line.
        const hasText = i + 1 < tokens.length && !tokens[i + 1].startsWith("<");
        const end = tokens[i + (hasText ? 2 : 1)];
        if (end === `</${name}>`) {
          line(t + (hasText ? tokens[i + 1] : "") + end);
          i += hasText ? 2 : 1;
        } else {
          line(t);
          open.push(name);
        }
      }
    }
  }
  return open.length === 0 ? out.join("\n") : null;
}

/**
 * Valid JSON text indented with 2 spaces, walked rather than re-serialised so that number
 * and string text stay as logged (`JSON.stringify` would turn 12345678901234567890 into 12345678901234567000).
 */
function indentJson(src: string): string {
  let out = "";
  let depth = 0;
  let inString = false;
  const newline = () => "\n" + "  ".repeat(depth);
  for (let i = 0; i < src.length; i++) {
    const ch = src[i];
    if (inString) {
      out += ch;
      if (ch === "\\") out += src[++i];
      else if (ch === '"') inString = false;
    } else if (ch === '"') {
      inString = true;
      out += ch;
    } else if (ch === "{" || ch === "[") {
      let j = i + 1;
      while (/\s/.test(src[j])) j++;
      if (src[j] === (ch === "{" ? "}" : "]")) {
        out += ch + src[j]; // an empty container stays on one line
        i = j;
      } else {
        depth++;
        out += ch + newline();
      }
    } else if (ch === "}" || ch === "]") {
      depth--;
      out += newline() + ch;
    } else if (ch === ",") {
      out += "," + newline();
    } else if (ch === ":") {
      out += ": ";
    } else if (!/\s/.test(ch)) {
      out += ch;
    }
  }
  return out;
}

/** A JSON object or array re-indented with 2 spaces, well-formed XML indented; anything else as logged. */
export function prettyBody(body: string): string {
  try {
    const json: unknown = JSON.parse(body); // only to check the text is valid
    // A bare scalar is left alone: there is nothing to indent.
    if (typeof json === "object" && json !== null) return indentJson(body);
  } catch {
    // not JSON: try XML
  }
  return indentXml(body.trim()) ?? body;
}
