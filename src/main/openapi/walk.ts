// Walks what `paths` reaches, checks it (spec R3.3, R4) and builds the model the generator reads (model.ts).
// Errors are collected rather than thrown, so the person sees all of them; only a wrong version stops the walk.

import { pathToFileURL } from "node:url";
import { isMap, isScalar, isSeq, type Node, type Pair, type YAMLMap } from "yaml";
import {
  Docs,
  format,
  ImportError,
  resolveAlias as named,
  toPlain,
  type Doc,
  type Loc,
} from "./docs";
import type { Fetch } from "./fetch";
import type { Api, Body, Operation, Param, Schema } from "./model";

export type Source = { kind: "url"; url: string } | { kind: "file"; path: string };

const SUPPORTED = "Grace imports OpenAPI 3.0 to 3.2";
const METHODS = ["get", "put", "post", "delete", "options", "head", "patch", "trace"];
const TYPES = ["string", "number", "integer", "boolean", "array", "object", "null"];
const PLACES = ["query", "header", "path", "cookie"];
const SHOWN = 20;
/** Karate needs an absolute URL, and a spec read from a file has no address to resolve a relative server against. */
const LOCAL_SERVER = "http://localhost:8080";

type N = Node | null | undefined;

const entry = (map: YAMLMap, key: string): Pair | undefined =>
  // By the key's text: YAML reads `200:` as a number, JSON and `"200":` as a string.
  map.items.find((p) => isScalar(p.key) && String(p.key.value) === key);
const val = (p: Pair | undefined) => p?.value as N;
const keyOf = (p: Pair) => p.key as N;
/** The scalar a node is, an alias to one included. */
const scalar = (doc: Doc, n: unknown) => {
  const s = named(doc, n);
  return isScalar(s) ? s : undefined;
};
const text = (doc: Doc, n: unknown) => {
  const v = scalar(doc, n)?.value;
  return typeof v === "string" ? v : undefined;
};
const keyText = (p: Pair) => String(isScalar(p.key) ? p.key.value : "");
/** `x-` keys are extensions, not data the import reads (R3.3), wherever the spec allows them. */
const isExtension = (p: Pair) => keyText(p).startsWith("x-");

/** Whether a value contains itself, as `&a {x: *a}` does: `JSON.stringify` would throw on it. */
function cyclic(value: unknown, ancestors = new Set<object>()): boolean {
  if (typeof value !== "object" || value === null) return false;
  if (ancestors.has(value)) return true;
  ancestors.add(value);
  const found = Object.values(value).some((v) => cyclic(v, ancestors));
  ancestors.delete(value);
  return found;
}
const pick = (bodies: Body[]) => bodies.find((b) => b.json) ?? bodies[0] ?? null;

const emptySchema = (): Schema => ({
  types: [],
  nullable: false,
  properties: null,
  required: [],
  items: null,
  allOf: [],
  oneOf: [],
  anyOf: [],
  format: null,
  minimum: null,
  readOnly: false,
  writeOnly: false,
  samples: [],
});

/**
 * Reads the spec, follows what its `paths` reach, and checks it. `errors` are formatted as R3.5 says:
 * one line each, repeats dropped, at most 20, then "and <n> more".
 */
export async function readApi(
  source: Source,
  o: { fetch: Fetch; signal: AbortSignal; readFile?: (path: string) => Promise<string> },
): Promise<{ api: Api } | { errors: string[] }> {
  const walker = new Walker(new Docs(o));
  let api: Api | null = null;
  try {
    const uri = source.kind === "file" ? pathToFileURL(source.path).href : source.url;
    api = await walker.run(await walker.docs.load(uri));
  } catch (e) {
    if (!(e instanceof ImportError)) throw e;
    walker.errors.push(e);
  }
  if (api && walker.errors.length === 0) return { api };
  const lines = [...new Set(walker.errors.map(format))];
  return {
    errors: [
      ...lines.slice(0, SHOWN),
      ...(lines.length > SHOWN ? [`and ${lines.length - SHOWN} more`] : []),
    ],
  };
}

class Walker {
  readonly errors: ImportError[] = [];
  /** By the node a schema is built from, so a `$ref`, however it is spelled, and an alias both give the same object. */
  private readonly schemas = new Map<Node, Schema>();
  private minor = 0;
  private rootSecurity: string[] = [];

  constructor(readonly docs: Docs) {}

  private at(doc: Doc, ...nodes: N[]): Loc {
    const n = nodes.find((x) => x?.range);
    return { label: doc.label, line: n ? this.docs.lineOf(doc, n) : 1 };
  }

  /** Records an error at the first of `nodes` that has a position. */
  private fail(doc: Doc, reason: string, ...nodes: N[]) {
    this.errors.push(new ImportError(this.at(doc, ...nodes), reason));
  }

  /**
   * Adds a node's plain data to `into`. A `$ref` key in it is data (R3.3). A value that contains
   * itself is left out, as if absent: the generator would write it with `JSON.stringify`.
   */
  private add(into: unknown[], doc: Doc, node: N) {
    let value: unknown;
    try {
      value = toPlain(doc, node);
    } catch (e) {
      this.fail(doc, e instanceof Error ? e.message : "too many aliases", node);
      return;
    }
    if (value !== undefined && !cyclic(value)) into.push(value);
  }

  async run(root: Doc): Promise<Api> {
    const map = named(root, root.root);
    if (!isMap(map)) throw new ImportError(this.at(root), "Not an OpenAPI spec: no openapi field");
    this.checkVersion(root, map);
    const infoPair = entry(map, "info");
    const info = named(root, val(infoPair));
    // R4.1: any scalar, as written (`version: 1.0` is "1.0"); an empty value is none.
    const field = (name: string) => {
      const pair = isMap(info) ? entry(info, name) : undefined;
      const value = scalar(root, val(pair));
      if (value && value.value !== null) return value.source ?? String(value.value);
      const reason = !isMap(info)
        ? "info must be a mapping"
        : pair && !value
          ? `info ${name} must be a scalar`
          : `info has no ${name}`;
      this.fail(root, reason, val(pair), infoPair && keyOf(infoPair), map);
      return "";
    };
    const title = field("title");
    const version = field("version");

    this.rootSecurity = this.security(root, entry(map, "security")) ?? [];
    const operations: Operation[] = [];
    const pathsPair = entry(map, "paths");
    const paths = named(root, val(pathsPair));
    if (pathsPair && !isMap(paths)) this.fail(root, "paths must be a mapping", val(pathsPair));
    for (const pair of isMap(paths) ? paths.items : [])
      if (!isExtension(pair)) await this.pathItem(root, pair, operations);
    if (operations.length === 0 && this.errors.length === 0)
      this.fail(root, "No operation to generate", pathsPair && keyOf(pathsPair), map);

    const tags = new Map<string, string | null>();
    const listed = named(root, val(entry(map, "tags")));
    for (const item of isSeq(listed) ? listed.items : []) {
      const tag = named(root, item as N);
      const name = isMap(tag) ? text(root, val(entry(tag, "name"))) : undefined;
      if (isMap(tag) && name !== undefined && !tags.has(name))
        tags.set(name, text(root, val(entry(tag, "description"))) ?? null);
    }
    for (const op of operations) if (op.tag !== null && !tags.has(op.tag)) tags.set(op.tag, null);
    return {
      title,
      version,
      server: this.server(root, map),
      tags: [...tags].map(([name, description]) => ({ name, description })),
      operations,
    };
  }

  /** R3.2. Throws: a spec of another version is not walked. */
  private checkVersion(doc: Doc, map: YAMLMap) {
    const swagger = entry(map, "swagger");
    if (swagger)
      throw new ImportError(
        this.at(doc, val(swagger), map),
        `Swagger 2.0 is not supported: ${SUPPORTED}`,
      );
    const field = entry(map, "openapi");
    if (!field) throw new ImportError(this.at(doc, map), "Not an OpenAPI spec: no openapi field");
    const v = named(doc, val(field));
    const minor = /^3\.([012])\.\d+/.exec(text(doc, v) ?? "")?.[1];
    if (minor === undefined) {
      // A number such as `3.0` shows as it was written.
      const shown = isScalar(v) ? v.source || String(v.value) : isSeq(v) ? "a list" : "a mapping";
      throw new ImportError(
        this.at(doc, v, map),
        `OpenAPI ${shown} is not supported: ${SUPPORTED}`,
      );
    }
    this.minor = Number(minor);
  }

  /** The first server, its variables replaced by their defaults and made absolute (R5.2). */
  private server(doc: Doc, root: YAMLMap): string | null {
    const servers = named(doc, val(entry(root, "servers")));
    const first = named(doc, (isSeq(servers) ? servers.items[0] : undefined) as N);
    const raw = isMap(first) ? text(doc, val(entry(first, "url"))) : undefined;
    if (!isMap(first) || raw === undefined) return null;
    const variables = named(doc, val(entry(first, "variables")));
    const url =
      raw.replace(/\{([^}]*)\}/g, (all, name: string) => {
        const variable = isMap(variables) ? named(doc, val(entry(variables, name))) : null;
        const fallback = isMap(variable)
          ? scalar(doc, val(entry(variable, "default")))?.value
          : undefined;
        return typeof fallback === "string" || typeof fallback === "number"
          ? String(fallback)
          : all;
      }) || "/";
    if (/^[a-z][a-z\d+.-]*:/i.test(url)) return url;
    const base = doc.remote ? doc.uri : LOCAL_SERVER;
    return URL.canParse(url, base) ? new URL(url, base).href : url;
  }

  /** Scheme names of a `security` list; `null` when it is absent. */
  private security(doc: Doc, pair: Pair | undefined): string[] | null {
    const list = named(doc, val(pair));
    if (!isSeq(list)) return null;
    const names = new Set<string>();
    for (const item of list.items) {
      const requirement = named(doc, item as N);
      for (const p of isMap(requirement) ? requirement.items : []) names.add(keyText(p));
    }
    return [...names];
  }

  /**
   * Follows `$ref`s from `node` to what they name, which may be in another document.
   * `null` after recording why it could not.
   */
  private async deref(node: N, doc: Doc): Promise<{ node: Node | null; doc: Doc } | null> {
    const seen = new Set<Node>();
    try {
      for (;;) {
        const here = named(doc, node);
        const ref = isMap(here) ? entry(here, "$ref") : undefined;
        if (!here || !ref) return { node: here, doc };
        const at = this.at(doc, val(ref), here);
        const target = text(doc, val(ref));
        if (target === undefined) throw new ImportError(at, "$ref must be a string");
        if (seen.has(here)) throw new ImportError(at, `$ref ${target} leads back to itself`);
        seen.add(here);
        ({ doc, node } = await this.docs.target(target, doc, at));
      }
    } catch (e) {
      if (!(e instanceof ImportError)) throw e;
      this.errors.push(e);
      return null;
    }
  }

  private async pathItem(doc: Doc, pair: Pair, out: Operation[]) {
    const route = keyText(pair);
    if (!route.startsWith("/")) this.fail(doc, `path ${route} must start with /`, keyOf(pair));
    const found = await this.deref(val(pair), doc);
    if (!found) return;
    const { node: item, doc: itemDoc } = found;
    if (!isMap(item)) return this.fail(itemDoc, "path item must be a mapping", item, keyOf(pair));

    const methods: [string, Pair][] = [];
    for (const m of [...METHODS, ...(this.minor >= 2 ? ["query"] : [])]) {
      const p = entry(item, m);
      if (p) methods.push([m, p]);
    }
    const extra = this.minor >= 2 ? entry(item, "additionalOperations") : undefined;
    const additional = named(itemDoc, val(extra));
    if (extra && !isMap(additional))
      this.fail(itemDoc, "additionalOperations must be a mapping", val(extra));
    for (const p of isMap(additional) ? additional.items : []) methods.push([keyText(p), p]);
    if (methods.length === 0) return;

    const shared = await this.params(entry(item, "parameters"), itemDoc);
    for (const [method, p] of methods) {
      const op = await this.operation(method, route, p, shared, itemDoc);
      if (!op) continue;
      out.push(op);
      for (const [, name] of route.matchAll(/\{([^}]*)\}/g)) {
        if (!op.params.some((x) => x.in === "path" && x.name === name))
          this.fail(doc, `path parameter ${name} is not declared`, keyOf(pair));
      }
    }
  }

  private async operation(
    method: string,
    route: string,
    pair: Pair,
    shared: Param[],
    doc: Doc,
  ): Promise<Operation | null> {
    const op = named(doc, val(pair));
    if (!isMap(op)) {
      this.fail(doc, "operation must be a mapping", op, keyOf(pair));
      return null;
    }
    // The operation's parameter replaces the path item's of the same name and place.
    const params = new Map<string, Param>();
    for (const p of [...shared, ...(await this.params(entry(op, "parameters"), doc))])
      params.set(`${p.in} ${p.name}`, p);
    const body = await this.requestBody(entry(op, "requestBody"), doc);
    const responses = await this.responses(entry(op, "responses"), doc);
    const ok = responses
      .filter((r) => /^2\d\d$/.test(r.code))
      .sort((a, b) => Number(a.code) - Number(b.code))[0];
    const range = responses.find((r) => /^2xx$/i.test(r.code));
    const tags = named(doc, val(entry(op, "tags")));
    return {
      method,
      path: route,
      tag: (isSeq(tags) ? text(doc, tags.items[0] as N) : undefined) ?? null,
      summary: text(doc, val(entry(op, "summary"))) ?? null,
      params: [...params.values()],
      body,
      success: ok ?? (range && { code: "2XX", body: range.body }) ?? null,
      security: this.security(doc, entry(op, "security")) ?? this.rootSecurity,
    };
  }

  private async params(pair: Pair | undefined, doc: Doc): Promise<Param[]> {
    if (!pair) return [];
    const list = named(doc, val(pair));
    if (!isSeq(list)) {
      this.fail(doc, "parameters must be a list", list, keyOf(pair));
      return [];
    }
    const places = this.minor >= 2 ? [...PLACES, "querystring"] : PLACES;
    const out: Param[] = [];
    for (const item of list.items) {
      const found = await this.deref(item as N, doc);
      if (!found) continue;
      const { node: param, doc: paramDoc } = found;
      if (!isMap(param)) {
        this.fail(paramDoc, "parameter must be a mapping", param);
        continue;
      }
      const name = text(paramDoc, val(entry(param, "name")));
      const place = entry(param, "in");
      const where = text(paramDoc, val(place)) ?? "";
      if (name === undefined) {
        this.fail(paramDoc, "parameter has no name", param);
      } else if (!place) {
        this.fail(paramDoc, `parameter ${name} has no in`, param);
      } else if (!places.includes(where)) {
        this.fail(
          paramDoc,
          `parameter ${name}: in must be one of ${places.join(", ")}`,
          val(place),
        );
      } else {
        const schema = entry(param, "schema");
        out.push({
          name,
          in: where as Param["in"],
          required:
            where === "path" || scalar(paramDoc, val(entry(param, "required")))?.value === true,
          samples: this.examples(paramDoc, param),
          schema: schema ? await this.schema(val(schema), paramDoc) : null,
        });
      }
    }
    return out;
  }

  private async requestBody(pair: Pair | undefined, doc: Doc): Promise<Body | null> {
    if (!pair) return null;
    const found = await this.deref(val(pair), doc);
    if (!found) return null;
    if (!isMap(found.node)) {
      this.fail(found.doc, "requestBody must be a mapping", found.node, keyOf(pair));
      return null;
    }
    return pick(await this.content(found.node, found.doc));
  }

  /** Every response is walked, so a missing file or a wrong type shows; the model keeps the success one's body. */
  private async responses(
    pair: Pair | undefined,
    doc: Doc,
  ): Promise<{ code: string; body: Body | null }[]> {
    if (!pair) return [];
    const map = named(doc, val(pair));
    if (!isMap(map)) {
      this.fail(doc, "responses must be a mapping", map, keyOf(pair));
      return [];
    }
    const out: { code: string; body: Body | null }[] = [];
    for (const p of map.items) {
      if (isExtension(p)) continue;
      const found = await this.deref(val(p), doc);
      if (!found) continue;
      if (!isMap(found.node)) {
        this.fail(found.doc, "response must be a mapping", found.node, keyOf(p));
        continue;
      }
      out.push({ code: keyText(p), body: pick(await this.content(found.node, found.doc)) });
    }
    return out;
  }

  private async content(owner: YAMLMap, doc: Doc): Promise<Body[]> {
    const pair = entry(owner, "content");
    if (!pair) return [];
    const map = named(doc, val(pair));
    if (!isMap(map)) {
      this.fail(doc, "content must be a mapping", map, keyOf(pair));
      return [];
    }
    const out: Body[] = [];
    for (const p of map.items) {
      const media = named(doc, val(p));
      if (!isMap(media)) {
        this.fail(doc, "a media type must be a mapping", media, keyOf(p));
        continue;
      }
      const mediaType = keyText(p);
      const bare = mediaType.split(";")[0]!.trim().toLowerCase();
      const schema = entry(media, "schema");
      out.push({
        mediaType,
        json: bare === "application/json" || bare.endsWith("+json"),
        samples: this.examples(doc, media),
        schema: schema ? await this.schema(val(schema), doc) : null,
      });
    }
    return out;
  }

  /** `example`, then the `value` of each of `examples` (a parameter's or a media type's). */
  private examples(doc: Doc, owner: YAMLMap): unknown[] {
    const out: unknown[] = [];
    const one = entry(owner, "example");
    if (one) this.add(out, doc, val(one));
    const many = named(doc, val(entry(owner, "examples")));
    for (const p of isMap(many) ? many.items : []) {
      const example = named(doc, val(p));
      const value = isMap(example) ? entry(example, "value") : undefined;
      if (value) this.add(out, doc, val(value));
    }
    return out;
  }

  /** A schema is registered before its parts are read, so a schema that contains itself ends there (R3.4). */
  private async schema(node: N, doc: Doc): Promise<Schema> {
    const found = await this.deref(node, doc);
    if (!found) return emptySchema();
    const known = found.node && this.schemas.get(found.node);
    if (known) return known;
    const out = emptySchema();
    if (found.node) this.schemas.set(found.node, out);
    await this.fill(out, found.node, found.doc);
    return out;
  }

  private async fill(out: Schema, node: Node | null, doc: Doc) {
    // `true` and `false` mean anything and nothing: every field stays empty.
    if (this.minor >= 1 && isScalar(node) && typeof node.value === "boolean") return;
    if (!isMap(node)) return this.fail(doc, "a schema must be a mapping", node);
    const flag = (key: string) => scalar(doc, val(entry(node, key)))?.value === true;

    const type = entry(node, "type");
    if (type) {
      const t = named(doc, val(type));
      const types = (isSeq(t) && this.minor >= 1 ? t.items : [t]).map((x) => text(doc, x as N));
      if (types.length === 0 || types.some((x) => !TYPES.includes(x ?? "")))
        this.fail(doc, `type must be one of ${TYPES.join(", ")}`, val(type));
      else {
        out.nullable = types.includes("null");
        out.types = types.filter((x): x is string => x !== "null");
      }
    }
    out.nullable ||= flag("nullable");
    out.readOnly = flag("readOnly");
    out.writeOnly = flag("writeOnly");
    out.format = text(doc, val(entry(node, "format"))) ?? null;
    const minimum = scalar(doc, val(entry(node, "minimum")))?.value;
    out.minimum = typeof minimum === "number" ? minimum : null;

    const required = entry(node, "required");
    if (required) {
      const list = named(doc, val(required));
      const names = isSeq(list) ? list.items.map((x) => text(doc, x as N)) : [undefined];
      if (names.some((x) => x === undefined))
        this.fail(doc, "required must be a list of strings", val(required));
      else out.required = names as string[];
    }

    const properties = entry(node, "properties");
    const props = named(doc, val(properties));
    if (properties && !isMap(props))
      this.fail(doc, "properties must be a mapping", val(properties));
    if (isMap(props)) {
      out.properties = {};
      for (const p of props.items) {
        // defineProperty, not assignment: a property called `__proto__` stays a property.
        Object.defineProperty(out.properties, keyText(p), {
          value: await this.schema(val(p), doc),
          enumerable: true,
          writable: true,
          configurable: true,
        });
      }
    }
    const items = entry(node, "items");
    if (items) out.items = await this.schema(val(items), doc);
    for (const key of ["allOf", "oneOf", "anyOf"] as const) {
      const pair = entry(node, key);
      if (!pair) continue;
      const list = named(doc, val(pair));
      if (!isSeq(list)) this.fail(doc, `${key} must be a list of schemas`, val(pair));
      else for (const item of list.items) out[key].push(await this.schema(item as N, doc));
    }

    // R5.5's order: example, the items of examples (3.1), const, default, the first of enum.
    const samples: unknown[] = [];
    const take = (key: string) => {
      const pair = entry(node, key);
      if (pair) this.add(samples, doc, val(pair));
    };
    take("example");
    const examples = named(doc, val(entry(node, "examples")));
    for (const x of isSeq(examples) ? examples.items : []) this.add(samples, doc, x as N);
    take("const");
    take("default");
    const choices = named(doc, val(entry(node, "enum")));
    if (isSeq(choices) && choices.items.length > 0) this.add(samples, doc, choices.items[0] as N);
    out.samples = samples;
  }
}
