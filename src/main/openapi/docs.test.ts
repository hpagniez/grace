import { mkdtemp, readFile as read, rm, truncate, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Document, type Node } from "yaml";
import { Docs, format, ImportError, parseText } from "./docs";

const fixture = (name: string) => path.resolve("test/fixtures/openapi", name);
const fixtureText = (name: string) => read(fixture(name), "utf8");
const fileUri = (name: string) => pathToFileURL(fixture(name)).href;
/** A node's plain data, whatever document it came from. */
const js = (root: Node | null) => new Document(root).toJS();

/** The ImportError that `run` throws or rejects with. */
async function failure(run: () => unknown): Promise<ImportError> {
  try {
    await run();
  } catch (e) {
    expect(e).toBeInstanceOf(ImportError);
    return e as ImportError;
  }
  throw new Error("did not fail");
}

const make = (o: Partial<ConstructorParameters<typeof Docs>[0]> = {}) =>
  new Docs({ fetch, signal: new AbortController().signal, ...o });

describe("parseText", () => {
  it("reads the Petstore in YAML and in JSON to the same data", async () => {
    const yaml = parseText(await fixtureText("petstore.yaml"), "petstore.yaml");
    const json = parseText(await fixtureText("petstore.json"), "petstore.json");
    expect(js(yaml.root)).toMatchObject({ openapi: "3.0.3", info: { title: "Petstore" } });
    expect(js(json.root)).toEqual(js(yaml.root));
  });

  it("reads JSON with tabs and a byte order mark", () => {
    const { root } = parseText('\uFEFF\n{\n\t"a": [1,\n\t\t2]\n}', "t.json");
    expect(js(root)).toEqual({ a: [1, 2] });
  });

  it("reads JSON strictly: a missing value is refused at its line, though YAML accepts it", async () => {
    const e = await failure(() => parseText('{"a": 1,\n "b": }', "x.json"));
    expect(e.at).toEqual({ label: "x.json", line: 2 });
    expect(format(e)).toMatch(/^x\.json:2: Unexpected token/);
  });

  it("refuses a trailing comma at its line", async () => {
    const e = await failure(async () => parseText(await fixtureText("bad.json"), "bad.json"));
    expect(e.at).toEqual({ label: "bad.json", line: 9 });
  });

  it("refuses a YAML syntax error at its line", async () => {
    const e = await failure(async () =>
      parseText(await fixtureText("bad-indent.yaml"), "bad-indent.yaml"),
    );
    expect(format(e)).toBe("bad-indent.yaml:7: All mapping items must start at the same column");
  });

  it("refuses a duplicate key, in JSON too, at the second one", async () => {
    const e = await failure(() => parseText('{\n  "a": 1,\n  "a": 2\n}', "d.json"));
    expect(format(e)).toBe("d.json:3: Map keys must be unique");
  });

  it("refuses a second YAML document", async () => {
    await failure(() => parseText("a: 1\n---\nb: 2\n", "m.yaml"));
  });

  it("reads an HTML page as a scalar and an empty text as nothing", () => {
    const html = "<html><body>login</body></html>";
    expect(js(parseText(html, "p.html").root)).toBe(html);
    expect(parseText("", "e.yaml").root).toBeNull();
  });
});

describe("Docs.load", () => {
  it("reads a local file once, whatever the fragment", async () => {
    const readFile = vi.fn((p: string) => read(p, "utf8"));
    const docs = make({ readFile });
    const doc = await docs.load(fileUri("petstore.yaml"));
    expect(doc).toMatchObject({
      uri: fileUri("petstore.yaml"),
      label: fixture("petstore.yaml"),
      remote: false,
    });
    expect(await docs.load(fileUri("petstore.yaml"))).toBe(doc);
    expect(await docs.load(`${fileUri("petstore.yaml")}#/info`)).toBe(doc);
    expect(readFile).toHaveBeenCalledTimes(1);
  });

  it("refuses the 101st document and keeps serving the first 100", async () => {
    const docs = make({ readFile: async () => "a: 1" });
    for (let i = 0; i < 100; i++) await docs.load(`file:///C:/d${i}.yaml`);
    const from = { label: "root.yaml", line: 5 };
    const e = await failure(() => docs.load("file:///C:/d100.yaml", from));
    expect(e.at).toEqual(from);
    expect(e.message).toBe("More than 100 referenced documents");
    await expect(docs.load("file:///C:/d0.yaml")).resolves.toBeDefined();
  });

  describe("the default file reader", () => {
    let dir: string;
    beforeEach(async () => {
      dir = await mkdtemp(path.join(tmpdir(), "kk-docs-"));
    });
    afterEach(() => rm(dir, { recursive: true, force: true }));

    it("refuses a file over 10 MB without reading it, and says when a file is missing", async () => {
      const big = path.join(dir, "big.yaml");
      await writeFile(big, "");
      await truncate(big, 10 * 1024 * 1024 + 1);
      const docs = make();
      expect(format(await failure(() => docs.load(pathToFileURL(big).href)))).toBe(
        `${big}:1: ${big} is larger than 10 MB`,
      );
      const missing = path.join(dir, "missing.yaml");
      expect(format(await failure(() => docs.load(pathToFileURL(missing).href)))).toBe(
        `${missing}:1: file not found`,
      );
    });
  });

  it("words a failed fetch with the URL it was for", async () => {
    const refused = make({
      fetch: async () => {
        throw new TypeError("fetch failed");
      },
    });
    const from = { label: "root.yaml", line: 3 };
    expect(format(await failure(() => refused.load("https://u:p@a.test/x.yaml?t=1", from)))).toBe(
      "root.yaml:3: https://a.test/x.yaml: fetch failed",
    );
    const e = await failure(() => refused.load("https://a.test/y.yaml"));
    expect(e.at).toEqual({ label: "https://a.test/y.yaml", line: 1 });
  });
});

describe("Docs.target", () => {
  const at = { label: "root", line: 20 };
  const memory = async (text: string) => {
    const docs = make({ readFile: async () => text });
    return { docs, doc: await docs.load("file:///C:/mem.yaml") };
  };

  it("finds a pointer in the same document and a line for its node", async () => {
    const docs = make();
    const doc = await docs.load(fileUri("petstore.yaml"));
    const found = await docs.target("#/components/schemas/Pet", doc, at);
    expect(found.doc).toBe(doc);
    expect(js(found.node)).toMatchObject({ type: "object", required: ["id", "name"] });
    const lines = (await fixtureText("petstore.yaml")).split("\n");
    expect(docs.lineOf(doc, found.node!)).toBe(lines.indexOf("      type: object") + 1);
    expect((await docs.target("#", doc, at)).node).toBe(doc.root);
  });

  it("decodes ~0, ~1 and percent-encoding, indexes a list and matches a number key", async () => {
    const { docs, doc } = await memory(
      "a/b:\n  c~d: 1\nx y: 2\nlist:\n  - zero\n  - one\ncodes:\n  200:\n    ok: 3\n",
    );
    const value = async (ref: string) => js((await docs.target(ref, doc, at)).node);
    expect(await value("#/a~1b/c~0d")).toBe(1);
    expect(await value("#/x%20y")).toBe(2);
    expect(await value("#/list/1")).toBe("one");
    expect(await value("#/codes/200")).toEqual({ ok: 3 });
  });

  it("goes through a YAML alias to the node it names", async () => {
    const { docs, doc } = await memory("base: &b\n  type: object\nuse: *b\n");
    expect(doc.root).toBe(doc.document.contents);
    const anchored = (await docs.target("#/base", doc, at)).node;
    expect((await docs.target("#/use", doc, at)).node).toBe(anchored);
    expect(js((await docs.target("#/use/type", doc, at)).node)).toBe("object");
  });

  it("names the last anchor before the alias, and nothing for an anchor not set yet", async () => {
    const { docs, doc } = await memory("first: *x\na: &x 1\nb: *x\nc: &x 2\nd: *x\n");
    const value = async (ref: string) => js((await docs.target(ref, doc, at)).node);
    expect(await value("#/b")).toBe(1);
    expect(await value("#/d")).toBe(2);
    const e = await failure(() => docs.target("#/first", doc, at));
    expect(e.message).toBe("$ref #/first points at nothing");
  });

  it("resolves thousands of aliases without reading the document again for each", async () => {
    const count = 2000;
    const pad = Array.from({ length: 20_000 }, (_, i) => i).join(", ");
    const aliases = Array.from({ length: count }, (_, i) => `k${i}: *s\n`).join("");
    const { docs, doc } = await memory(`s: &s\n  type: object\npad: [${pad}]\n${aliases}`);
    const started = performance.now();
    for (let i = 0; i < count; i++) await docs.target(`#/k${i}`, doc, at);
    // A margin for a loaded machine: a scan of the document per alias takes about 6 s here.
    expect(performance.now() - started).toBeLessThan(2500);
  });

  it("refuses a pointer that points at nothing, at the $ref's line", async () => {
    const { docs, doc } = await memory("x: 2\nlist: [zero]\n");
    for (const ref of ["#/nope", "#/x/deeper", "#/list/1", "#/list/x", "#nope"]) {
      const e = await failure(() => docs.target(ref, doc, at));
      expect(e.at, ref).toEqual(at);
      expect(e.message, ref).toBe(`$ref ${ref} points at nothing`);
    }
  });

  it("resolves a file next to a file, and a path with a drive letter", async () => {
    const docs = make();
    const root = await docs.load(fileUri("split/root.yaml"));
    const pet = await docs.target("schemas/pet.yaml#/Pet", root, at);
    expect(pet.doc.label).toBe(fixture("split/schemas/pet.yaml"));
    expect(js(pet.node)).toMatchObject({ required: ["id", "name"] });
    const error = await docs.target("schemas/error.json#/Error", root, at);
    expect(js(error.node)).toMatchObject({ required: ["message"] });
    const info = await docs.target(`${fixture("petstore.yaml")}#/info`, root, at);
    expect(js(info.node)).toEqual({ title: "Petstore", version: "1.0.0" });
  });

  it("says when the file is missing and refuses a scheme it does not read", async () => {
    const enoent = Object.assign(new Error("ENOENT: no such file"), { code: "ENOENT" });
    const docs = make({
      readFile: async (p) => {
        if (p.endsWith("mem.yaml")) return "a: 1";
        throw enoent;
      },
    });
    const doc = await docs.load("file:///C:/mem.yaml");
    const missing = await failure(() => docs.target("gone.yaml#/A", doc, at));
    expect(missing.at).toEqual(at);
    expect(missing.message).toBe("$ref gone.yaml#/A: file not found");
    const ftp = await failure(() => docs.target("ftp://a.test/x.yaml", doc, at));
    expect(ftp.message).toBe("$ref ftp://a.test/x.yaml is not a file path or an http or https URL");
  });

  describe("over http", () => {
    const routes: Record<string, string> = {
      "/dir/spec.yaml": "a: 1\n",
      "/dir/pet.json": '{"Pet": {"type": "object"}}',
      "/x.yaml": "x: 1\n",
    };
    let server: Server;
    let base: string;

    beforeEach(async () => {
      server = createServer((req, res) => {
        const body = routes[req.url!];
        res.statusCode = body === undefined ? 404 : 200;
        res.end(body);
      });
      await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
      base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
    });

    afterEach(async () => {
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    });

    it("resolves a reference against the URL of the document that holds it", async () => {
      const docs = make();
      const spec = await docs.load(`${base}/dir/spec.yaml`);
      expect(spec).toMatchObject({ uri: `${base}/dir/spec.yaml`, remote: true });
      const found = await docs.target("pet.json#/Pet", spec, at);
      expect(found.doc).toMatchObject({
        uri: `${base}/dir/pet.json`,
        label: `${base}/dir/pet.json`,
        remote: true,
      });
      expect(js(found.node)).toEqual({ type: "object" });
    });

    it("words a status other than 2xx at the $ref's line", async () => {
      const docs = make();
      const spec = await docs.load(`${base}/dir/spec.yaml`);
      expect(format(await failure(() => docs.target("../gone.json", spec, at)))).toBe(
        `root:20: ${base}/gone.json answered 404`,
      );
    });

    it("lets a local file refer to an http URL", async () => {
      const readFile = vi.fn((p: string) => read(p, "utf8"));
      const docs = make({ readFile });
      const root = await docs.load(fileUri("split/root.yaml"));
      const found = await docs.target(`${base}/x.yaml`, root, at);
      expect(found.doc.remote).toBe(true);
      expect(js(found.node)).toEqual({ x: 1 });
    });

    it("refuses a reference to a local file, reading nothing", async () => {
      const readFile = vi.fn(async () => "");
      const docs = make({ readFile });
      const spec = await docs.load(`${base}/dir/spec.yaml`);
      const from = { label: spec.label, line: 4 };
      for (const ref of [
        "file:///C:/x.yaml",
        "C:/x.yaml",
        "C:\\x.yaml#/A",
        "\\\\host\\share\\x.yaml",
      ]) {
        const e = await failure(() => docs.target(ref, spec, from));
        expect(format(e), ref).toBe(
          `${base}/dir/spec.yaml:4: a remote spec may not refer to a local file`,
        );
      }
      expect(readFile).not.toHaveBeenCalled();
    });

    it("says a scheme it does not read is not a path or an http URL, not a local file", async () => {
      const docs = make();
      const spec = await docs.load(`${base}/dir/spec.yaml`);
      const from = { label: spec.label, line: 4 };
      for (const ref of ["ftp://a.test/x.yaml", "data:text/plain,a"]) {
        expect(format(await failure(() => docs.target(ref, spec, from))), ref).toBe(
          `${base}/dir/spec.yaml:4: $ref ${ref} is not a file path or an http or https URL`,
        );
      }
    });
  });
});
