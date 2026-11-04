// The model of a spec (model.ts) as Karate feature files (R5). The spec may come from anyone, so its
// text reaches a feature in three ways only (R6): in a `#` comment, as a name with its control
// characters and line breaks dropped, and as a JSON value. Nothing else of it is ever written.
import type { Api, Operation, Param, Schema } from "./model";

export interface FeatureFile {
  name: string;
  text: string;
}

/** Object levels a value or a match opens (R5.4). */
const MAX_DEPTH = 5;
/** Objects opened per value or match: `$ref`s that fan out would otherwise grow without end. */
const BUDGET = 1000;
const DEFAULT_SERVER = "http://localhost:8080";

// R6: the spec's text is data.

const BREAK = /\r\n|[\n\v\f\r\u0085  ]/;
// Control characters, the line separators, and the bidirectional marks that hide where a line ends.
const UNSAFE = /[\p{Cc}  ‪-‮⁦-⁩]/gu;

/** A name, or one piece of a line: no control character, no line break. */
const line = (text: string) => text.replace(UNSAFE, "").trim();

/** Text as `#` lines, one for each of its lines. */
const comment = (text: string, indent: string) =>
  text.split(BREAK).map((l) => `${indent}# ${line(l)}`.trimEnd());

/** A string Karate would read as a fuzzy marker or an embedded expression (R6.3). */
const hostile = (text: string) => /^\s*#|#\(/.test(text);

/**
 * Every value written into a step (R6.2). JSON leaves U+0085, U+2028, U+2029 and U+007F as they are,
 * and a reader that splits lines on Java's `\R` would end the step at the first three.
 */
const js = (value: unknown) =>
  (JSON.stringify(value) ?? "null").replace(
    /[\u007f-\u009f  ]/g,
    (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`,
  );

// R5.1: file names.

const RESERVED = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])$/;

/** A name as R5.1 writes it into a file or folder name; `""` when nothing is left of it. */
export function slug(s: string): string {
  const name = s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")
    .slice(0, 100)
    .replace(/-$/, "");
  return RESERVED.test(name) ? `${name}-feature` : name;
}

interface Group {
  name: string;
  description: string | null;
  ops: Operation[];
}

const firstSegment = (path: string) => path.split("/").find(Boolean) ?? "";

/** The operations by feature, in the order of the spec's tags, then of first use (R5.1). */
function groups(api: Api): Group[] {
  const out = new Map<string, Group>();
  const group = (name: string) => {
    let g = out.get(name);
    if (!g) out.set(name, (g = { name, description: null, ops: [] }));
    return g;
  };
  for (const t of api.tags) {
    const name = line(t.name);
    if (name) group(name).description ??= t.description;
  }
  for (const op of api.operations) {
    group((op.tag && line(op.tag)) || line(firstSegment(op.path)) || "default").ops.push(op);
  }
  return [...out.values()].filter((g) => g.ops.length > 0);
}

/** A file name for each group, the second of two equal names `-2`, the third `-3`. */
function fileNames(gs: Group[]): string[] {
  const taken = new Set<string>();
  const next = new Map<string, number>();
  return gs.map((g) => {
    const base = slug(g.name) || slug(firstSegment(g.ops[0]!.path)) || "default";
    let name = base;
    for (let n = next.get(base) ?? 2; taken.has(name); n++) {
      name = `${base}-${n}`;
      next.set(base, n + 1);
    }
    taken.add(name);
    return `${name}.feature`;
  });
}

// R5.4 and R5.5: markers and values.

/** What a walk over a schema keeps: the schemas it is inside of (a cycle), and what it may still open. */
interface Walk {
  path: Set<Schema>;
  budget: number;
}
const fresh = (): Walk => ({ path: new Set(), budget: BUDGET });

/**
 * The schema with `allOf` merged in (R5.4). With `first`, the first of each `oneOf` and `anyOf` is
 * merged in too (R5.5).
 */
function flat(s: Schema, first: boolean): Schema {
  const all = new Set<Schema>();
  const visit = (x: Schema) => {
    if (all.has(x)) return;
    all.add(x);
    x.allOf.forEach((m) => visit(m));
    const pick = first ? (x.oneOf[0] ?? x.anyOf[0]) : undefined;
    if (pick) visit(pick);
  };
  visit(s);
  const list = [...all];
  const properties = list.flatMap((x) => Object.entries(x.properties ?? {}));
  return {
    ...s,
    types: list.find((x) => x.types.length > 0)?.types ?? [],
    properties: properties.length > 0 ? Object.fromEntries(properties) : null,
    required: list.flatMap((x) => x.required),
    items: list.find((x) => x.items)?.items ?? null,
    allOf: [],
    oneOf: first ? [] : list.flatMap((x) => x.oneOf),
    anyOf: first ? [] : list.flatMap((x) => x.anyOf),
    format: list.find((x) => x.format)?.format ?? null,
    minimum: list.find((x) => x.minimum !== null)?.minimum ?? null,
    samples: list.find((x) => x.samples.length > 0)?.samples ?? [],
  };
}

const kind = (f: Schema) => f.types[0] ?? (f.properties ? "object" : f.items ? "array" : null);
/** More than one shape: nothing to say about it but that it is there. */
const union = (f: Schema) => f.types.length > 1 || f.oneOf.length > 0 || f.anyOf.length > 0;
/** The item markers of `#[] #…`. */
const ITEMS = new Map([
  ["string", "string"],
  ["number", "number"],
  ["integer", "number"],
  ["boolean", "boolean"],
]);

/** The marker, or the object of markers, that matches a schema. `depth` is the level it would open. */
function marker(s: Schema, optional: boolean, w: Walk, depth: number): unknown {
  if (w.path.has(s)) return "#ignore";
  const f = flat(s, false);
  const opt = optional || f.nullable;
  const mark = (name: string) => `${opt ? "##" : "#"}${name}`;
  if (union(f)) return opt ? "#ignore" : "#present";
  switch (kind(f)) {
    case "string":
      return mark("string");
    case "number":
    case "integer":
      return mark("number");
    case "boolean":
      return mark("boolean");
    case "array": {
      const item = f.items && flat(f.items, false);
      const type = item && !item.nullable && !union(item) ? ITEMS.get(kind(item) ?? "") : undefined;
      return type ? `${mark("[]")} #${type}` : mark("array");
    }
    case "object": {
      const open = !opt && f.properties && depth < MAX_DEPTH && w.budget-- > 0;
      const literal = open ? fields(s, w, depth) : [];
      return literal.length > 0 ? Object.fromEntries(literal) : mark("object");
    }
    default:
      // No type at all: `type: "null"` allows only null, anything else is not checked.
      return f.nullable ? `${optional ? "##" : "#"}null` : optional ? "#ignore" : "#present";
  }
}

/** The markers of an object schema's properties. */
function fields(s: Schema, w: Walk, depth: number): [string, unknown][] {
  const f = flat(s, false);
  const required = new Set(f.required);
  w.path.add(s);
  const out = Object.entries(f.properties ?? {})
    // A property only a request carries is not in a response.
    .filter(([name, p]) => !p.writeOnly && !hostile(name))
    .map(([name, p]): [string, unknown] => [name, marker(p, !required.has(name), w, depth + 1)]);
  w.path.delete(s);
  return out;
}

/** The step that matches a response body against its schema (R5.4). */
function matchStep(s: Schema): string {
  const w = fresh();
  const f = flat(s, false);
  const plain = (x: Schema) => !x.nullable && !union(x);
  if (plain(f) && kind(f) === "object") {
    const e = fields(s, w, 0);
    if (e.length > 0) return `And match response contains deep ${js(Object.fromEntries(e))}`;
  }
  if (plain(f) && kind(f) === "array" && f.items) {
    const item = flat(f.items, false);
    const e = plain(item) && kind(item) === "object" ? fields(f.items, w, 0) : [];
    if (e.length > 0) return `And match each response contains deep ${js(Object.fromEntries(e))}`;
  }
  return `And match response == ${js(marker(s, false, w, 0))}`;
}

const FORMATS = new Map([
  ["date", "2026-01-01"],
  ["date-time", "2026-01-01T00:00:00Z"],
  ["email", "user@example.com"],
  ["uuid", "00000000-0000-0000-0000-000000000000"],
  ["uri", "https://example.com"],
]);

/** A sample from the spec, a string that Karate would read as a marker or an expression replaced (R6.3). */
function clean(v: unknown): unknown {
  if (typeof v === "string") return hostile(v) ? "string" : v;
  if (Array.isArray(v)) return v.map((x) => clean(x));
  if (v && typeof v === "object") {
    const entries = Object.entries(v).filter(([k]) => !hostile(k));
    return Object.fromEntries(entries.map(([k, x]) => [k, clean(x)]));
  }
  return v;
}

/** A value for a schema (R5.5): its sample, else a placeholder. */
function build(s: Schema | null, w: Walk, depth: number): unknown {
  if (!s) return "string";
  if (w.path.has(s)) return null;
  const f = flat(s, true);
  if (f.samples.length > 0) return clean(f.samples[0]);
  switch (kind(f)) {
    case "string":
      return FORMATS.get(f.format ?? "") ?? "string";
    case "number":
    case "integer":
      return f.minimum !== null && f.minimum > 0 ? f.minimum : 0;
    case "boolean":
      return true;
    case "array": {
      if (!f.items) return [];
      w.path.add(s);
      const item = build(f.items, w, depth);
      w.path.delete(s);
      return [item];
    }
    case "object": {
      if (depth >= MAX_DEPTH || w.budget-- <= 0) return {};
      w.path.add(s);
      const out = Object.entries(f.properties ?? {})
        .filter(([name, p]) => !p.readOnly && !hostile(name))
        .map(([name, p]) => [name, build(p, w, depth + 1)]);
      w.path.delete(s);
      return Object.fromEntries(out);
    }
    default:
      return "string";
  }
}

/** The value of a parameter or a request body: the sample beside its schema, else the schema's. */
const valueOf = (h: { samples: unknown[]; schema: Schema | null }) =>
  h.samples.length > 0 ? clean(h.samples[0]) : build(h.schema, fresh(), 0);

// R5.3: a scenario.

/** A path segment as a JSON value: each `{name}` in it replaced by the parameter's value. */
function segment(seg: string, params: Param[]): string {
  const value = (name: string) => {
    const p = params.find((x) => x.in === "path" && x.name === name);
    const v = p ? valueOf(p) : "string";
    return typeof v === "object" ? JSON.stringify(v) : v;
  };
  const parts = seg.split(/\{([^}]*)\}/); // text, name, text, name, ..., text
  if (parts.length === 3 && !parts[0] && !parts[2]) return js(value(parts[1]!));
  return js(parts.map((x, i) => (i % 2 ? String(value(x)) : x)).join(""));
}

function scenario(op: Operation): string[] {
  // A method that is not a word is spec text that could end the step and start another.
  if (!/^[A-Za-z]+$/.test(op.method))
    return [`  # ${line(`${op.path} not generated: method ${op.method} is not supported`)}`];
  const out = [
    ...(op.summary?.trim() ? comment(op.summary, "  ") : []),
    `  Scenario: ${op.method.toUpperCase()} ${line(op.path)}`,
  ];
  const step = (text: string) => out.push(`    ${text}`);
  const note = (text: string) => out.push(`    # ${line(text)}`);

  const segments = op.path
    .split("/")
    .filter(Boolean)
    .map((seg) => segment(seg, op.params));
  if (segments.length > 0) step(`Given path ${segments.join(", ")}`);

  const sent = (place: Param["in"]) =>
    op.params.filter((p) => p.in === place && p.required && !hostile(p.name));
  const sentAs = (ps: Param[]) => js(Object.fromEntries(ps.map((p) => [p.name, valueOf(p)])));
  const query = sent("query");
  if (query.length > 0) step(`And params ${sentAs(query)}`);
  const headers = sent("header");
  if (headers.length > 0) step(`And headers ${sentAs(headers)}`);
  const optional = op.params.filter((p) => !p.required && (p.in === "query" || p.in === "header"));
  if (optional.length > 0) note(`Optional params: ${optional.map((p) => line(p.name)).join(", ")}`);
  const skipped = op.params.filter((p) => p.in === "cookie" || p.in === "querystring");
  if (skipped.length > 0)
    note(`Params not generated: ${skipped.map((p) => `${p.in} ${line(p.name)}`).join(", ")}`);

  if (op.body) {
    if (op.body.json) step(`And request ${js(valueOf(op.body))}`);
    else note(`Request body ${op.body.mediaType} not generated`);
  }
  step(`When method ${op.method.toLowerCase()}`);

  const code = op.success?.code ?? "";
  const known = /^\d{3}$/.test(code);
  if (known) step(`Then status ${code}`);
  else if (code === "2XX") step("Then assert responseStatus >= 200 && responseStatus < 300");
  else note("No success response documented");
  const response = op.success?.body;
  if ((known || code === "2XX") && response?.json && response.schema)
    step(matchStep(response.schema));

  if (op.security.length > 0) note(`Security: ${op.security.map((s) => line(s)).join(", ")}`);
  return out;
}

/**
 * One feature file for each tag (R5.1). `o.source` is the spec as R3.5 shows it, `o.date` is
 * `yyyy-mm-dd`.
 */
export function generate(api: Api, o: { source: string; date: string }): FeatureFile[] {
  const gs = groups(api);
  const files = fileNames(gs);
  const server = api.server ?? DEFAULT_SERVER;
  return gs.map((g, i) => {
    const out = [
      `# Generated by Grace from ${line(o.source)} on ${line(o.date)}: ${line(api.title)} ${line(api.version)}`,
      `Feature: ${g.name}`,
      ...(g.description?.trim() ? comment(g.description, "  ") : []),
      "",
      "  Background:",
      `    * url karate.get("baseUrl", ${js(server)})`,
    ];
    for (const op of g.ops) out.push("", ...scenario(op));
    return { name: files[i]!, text: `${out.join("\n")}\n` };
  });
}
