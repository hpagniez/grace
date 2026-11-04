// Reads the documents of one OpenAPI import, JSON or YAML, and finds what a `$ref` points at.
// Every node keeps its position, so an error can name its line (spec R3.1, R3.3, R3.4).

import { readFile, stat } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  isAlias,
  isMap,
  isScalar,
  isSeq,
  LineCounter,
  parseDocument,
  visit,
  type Alias,
  type Document,
  type Node,
} from "yaml";
import { toJS } from "yaml/util";
import { displayUrl, fetchText, MAX_BYTES, type Fetch } from "./fetch";

const MAX_DOCS = 100;
/** A path with a drive letter or a UNC path: `new URL` would take `C:` for a scheme. */
const WINDOWS_PATH = /^(?:[a-zA-Z]:[\\/]|\\\\)/;

/** A file or URL and a line (1-based), as an error shows them (R3.5). */
export interface Loc {
  label: string;
  line: number;
}

/** A reason the import is refused. `message` is the reason alone. */
export class ImportError extends Error {
  constructor(
    readonly at: Loc,
    reason: string,
  ) {
    super(reason);
  }
}

export const format = (e: ImportError) => `${e.at.label}:${e.at.line}: ${e.message}`;

export interface Doc {
  /** A `file:` URL, or the final `http(s)` URL after redirects, without a fragment: references resolve against it. */
  uri: string;
  /** The absolute file path, or the URL without user info and query. */
  label: string;
  remote: boolean;
  /** Kept for its anchors: an alias node resolves against it. */
  document: Document;
  /** `document.contents`. */
  root: Node | null;
  lines: LineCounter;
}

/**
 * Parses JSON or YAML 1.2. The content tells which (R3.1): after a byte order mark and white space,
 * `{` or `[` is JSON and is checked strictly first, since YAML accepts what JSON does not.
 */
export function parseText(
  text: string,
  label: string,
): { document: Document; root: Node | null; lines: LineCounter } {
  text = text.replace(/^\uFEFF/, "");
  if (/^\s*[{[]/.test(text)) {
    try {
      JSON.parse(text);
    } catch (e) {
      throw new ImportError({ label, line: jsonLine(text) }, jsonReason(e));
    }
  }
  const lines = new LineCounter();
  const doc = parseDocument(text, {
    lineCounter: lines,
    uniqueKeys: true,
    version: "1.2",
    customTags: [],
    prettyErrors: false,
  });
  const [error] = doc.errors;
  if (error)
    throw new ImportError({ label, line: lines.linePos(error.pos[0]).line }, error.message);
  return { document: doc, root: doc.contents, lines };
}

/** V8 words an error without its position when it quotes the text, so look for the line ourselves. */
function jsonLine(text: string): number {
  // A prefix of a valid text either parses or fails at its own end; a prefix that fails earlier has passed the error.
  const failsEarly = (n: number) => {
    try {
      JSON.parse(text.slice(0, n));
      return false;
    } catch (e) {
      const { message } = e as Error;
      const at = / at position (\d+)(?: \(line \d+ column \d+\))?$/.exec(message);
      return at ? Number(at[1]) < n : message !== "Unexpected end of JSON input";
    }
  };
  let lo = 1;
  let hi = text.length + 1; // past the end: where a text that stops too soon fails
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (failsEarly(mid)) hi = mid;
    else lo = mid + 1;
  }
  return text.slice(0, lo - 1).split("\n").length;
}

/** V8's message without the text and position it quotes. */
function jsonReason(e: unknown): string {
  const message = e instanceof Error ? e.message : String(e);
  return /^(.*?)(?: (?:in|after) JSON at position|, ["."])/s.exec(message)?.[1] ?? message;
}

async function readLimited(path: string): Promise<string> {
  if ((await stat(path)).size > MAX_BYTES) throw new Error(`${path} is larger than 10 MB`);
  return readFile(path, "utf8");
}

/** What a document's anchors give, found once: `Alias.resolve` alone reads the whole document on every call. */
interface Anchors {
  /** Every anchored node and every alias, in document order: yaml's `aliasResolveCache`. */
  nodes: Node[];
  /** The node each alias names: the last node with its anchor before it. */
  named: Map<Alias, Node | undefined>;
}
const anchorsOf = new WeakMap<Document, Anchors>();

function anchors(document: Document): Anchors {
  let found = anchorsOf.get(document);
  if (!found) {
    const nodes: Node[] = [];
    visit(document, {
      Node: (_key, node) => {
        if (isAlias(node) || node.anchor) nodes.push(node);
      },
    });
    const latest = new Map<string, Node>();
    const named = new Map<Alias, Node | undefined>();
    for (const node of nodes) {
      if (isAlias(node)) named.set(node, latest.get(node.source));
      else latest.set(node.anchor!, node);
    }
    found = { nodes, named };
    anchorsOf.set(document, found);
  }
  return found;
}

const aliasTarget = (doc: Doc, alias: Alias) => anchors(doc.document).named.get(alias);

/** `n`, or the node it names when it is an alias; `null` for nothing. */
export function resolveAlias(doc: Doc, n: unknown): Node | null {
  return isAlias(n) ? (aliasTarget(doc, n) ?? null) : ((n as Node | null | undefined) ?? null);
}

/**
 * A node's plain data, `undefined` for nothing. Throws a ReferenceError when aliases expand past
 * yaml's limit. A value can contain itself (`&a {x: *a}`). The anchors are handed to yaml found
 * once, so a call does not read the document again.
 */
export function toPlain(doc: Doc, node: Node | null | undefined): unknown {
  const target = resolveAlias(doc, node);
  if (!target) return undefined;
  return toJS(target, "", {
    anchors: new Map(),
    aliasResolveCache: anchors(doc.document).nodes,
    doc: doc.document,
    keep: true,
    mapAsMap: false,
    mapKeyWarned: false,
    maxAliasCount: 100,
  });
}

/** The node a JSON pointer names, `null` for a null value, `undefined` for nothing. An alias is the node it names. */
function follow(doc: Doc, pointer: string): Node | null | undefined {
  if (pointer !== "" && !pointer.startsWith("/")) return undefined;
  const named = (n: Node | null | undefined) => (isAlias(n) ? aliasTarget(doc, n) : n);
  let node = named(doc.root);
  try {
    for (const segment of pointer === "" ? [] : pointer.slice(1).split("/")) {
      const key = decodeURIComponent(segment).replaceAll("~1", "/").replaceAll("~0", "~");
      if (isMap(node)) {
        // By the key's text: YAML reads `200:` as a number, JSON and `"200":` as a string.
        const pair = node.items.find((p) => isScalar(p.key) && String(p.key.value) === key);
        node = named(pair ? ((pair.value as Node | null) ?? null) : undefined);
      } else if (isSeq(node) && /^(?:0|[1-9]\d*)$/.test(key)) {
        node = named(node.items[Number(key)] as Node | undefined);
      } else {
        return undefined;
      }
    }
  } catch {
    return undefined; // a malformed %-escape
  }
  return node;
}

/** The documents of one import: each read once, at most 100, and the `$ref`s between them. */
export class Docs {
  private readonly docs = new Map<string, Promise<Doc>>();

  constructor(
    private readonly o: {
      fetch: Fetch;
      signal: AbortSignal;
      readFile?: (path: string) => Promise<string>;
    },
  ) {}

  /** `uri` is a `file:` or `http(s)` URL. `from` is where it was asked for, `ref` the `$ref` that asked. */
  async load(uri: string, from?: Loc, ref?: string): Promise<Doc> {
    const url = new URL(uri);
    url.hash = "";
    const key = url.href;
    let path: string | null = null;
    if (url.protocol === "file:") {
      try {
        path = fileURLToPath(url);
      } catch (e) {
        throw new ImportError(from ?? { label: key, line: 1 }, (e as Error).message);
      }
    }
    const at = from ?? { label: path ?? displayUrl(key), line: 1 };
    let doc = this.docs.get(key);
    if (!doc) {
      if (this.docs.size === MAX_DOCS)
        throw new ImportError(at, "More than 100 referenced documents");
      doc = this.read(key, path, at, ref);
      this.docs.set(key, doc);
    }
    return doc;
  }

  private async read(key: string, path: string | null, at: Loc, ref?: string): Promise<Doc> {
    let text: string;
    let uri = key;
    try {
      if (path === null) {
        const fetched = await fetchText(key, this.o.fetch, this.o.signal);
        text = fetched.text;
        uri = fetched.url;
      } else {
        text = await (this.o.readFile ?? readLimited)(path);
      }
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      if (path !== null) {
        const missing = (e as NodeJS.ErrnoException).code === "ENOENT";
        throw new ImportError(
          at,
          !missing ? message : ref === undefined ? "file not found" : `$ref ${ref}: file not found`,
        );
      }
      // fetchText's own messages name their URL; a network failure ("fetch failed") does not.
      throw new ImportError(
        at,
        /^https?:\/\//.test(message) ? message : `${displayUrl(key)}: ${message}`,
      );
    }
    const label = path ?? displayUrl(uri);
    return { uri, label, remote: path === null, ...parseText(text, label) };
  }

  /** The document `ref` names and the node at its pointer. `at` is the `$ref`'s own position. */
  async target(ref: string, holder: Doc, at: Loc): Promise<{ doc: Doc; node: Node | null }> {
    const hash = ref.indexOf("#");
    const location = hash < 0 ? ref : ref.slice(0, hash);
    const doc =
      location === "" ? holder : await this.load(this.address(location, holder, ref, at), at, ref);
    const node = follow(doc, hash < 0 ? "" : ref.slice(hash + 1));
    if (node === undefined) throw new ImportError(at, `$ref ${ref} points at nothing`);
    return { doc, node };
  }

  /** Where a reference to another document leads: against the holder's URI, as a browser would. */
  private address(location: string, holder: Doc, ref: string, at: Loc): string {
    const refuse = () => new ImportError(at, "a remote spec may not refer to a local file");
    if (WINDOWS_PATH.test(location)) {
      if (holder.remote) throw refuse();
      return pathToFileURL(location).href;
    }
    if (!URL.canParse(location, holder.uri)) {
      throw new ImportError(at, `$ref ${ref} is not a file path or an http or https URL`);
    }
    const url = new URL(location, holder.uri);
    if (/^https?:$/.test(url.protocol)) return url.href;
    if (url.protocol === "file:") {
      if (holder.remote) throw refuse();
      return url.href;
    }
    throw new ImportError(at, `$ref ${ref} is not a file path or an http or https URL`);
  }

  lineOf(doc: Doc, node: Node): number {
    return doc.lines.linePos(node.range?.[0] ?? 0).line;
  }
}
