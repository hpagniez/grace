import { readFile as read } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api, Operation, Schema } from "./model";
import { readApi } from "./walk";

type Result = Awaited<ReturnType<typeof readApi>>;

const fixture = (name: string) => path.resolve("test/fixtures/openapi", name);
const options = (readFile?: (p: string) => Promise<string>) => ({
  fetch,
  signal: new AbortController().signal,
  readFile,
});
const readFixture = (name: string) => readApi({ kind: "file", path: fixture(name) }, options());

function apiOf(r: Result): Api {
  if (!("api" in r)) throw new Error(`failed:\n${r.errors.join("\n")}`);
  return r.api;
}
function errorsOf(r: Result): string[] {
  if (!("errors" in r)) throw new Error("did not fail");
  return r.errors;
}
/** The one operation of a spec, found by method and path. */
function find(api: Api, method: string, p: string): Operation {
  const op = api.operations.find((o) => o.method === method && o.path === p);
  if (!op) throw new Error(`no ${method} ${p}`);
  return op;
}

/** A spec held in memory, as a file in a folder that does not exist; `files` are the others it may refer to. */
const virtual = (name: string) => path.resolve("virtual", name);
const SPEC = virtual("spec.yaml");
const readText = (text: string, files: Record<string, string> = {}) =>
  readApi(
    { kind: "file", path: SPEC },
    options(async (p) => {
      const found = p === SPEC ? text : files[path.basename(p)];
      if (found === undefined) throw Object.assign(new Error("ENOENT"), { code: "ENOENT" });
      return found;
    }),
  );

const empty: Schema = {
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
};

const HEAD = 'openapi: 3.0.3\ninfo:\n  title: T\n  version: "1"\n';
/** A spec whose one operation has the given response schema (indented to sit under `schema:`). */
const withSchema = (schema: string, head = HEAD) =>
  `${head}paths:\n  /a:\n    get:\n      responses:\n        "200":\n          description: ok\n          content:\n            application/json:\n              schema:\n${schema}`;
const successSchema = (api: Api) => api.operations[0]!.success!.body!.schema!;

describe("the Petstore", () => {
  it("gives equal Apis from YAML and JSON", async () => {
    const yaml = apiOf(await readFixture("petstore.yaml"));
    const json = apiOf(await readFixture("petstore.json"));
    expect(json).toEqual(yaml);
    expect(yaml).toMatchObject({
      title: "Petstore",
      version: "1.0.0",
      server: "http://petstore.example/v1",
      tags: [
        { name: "pets", description: null },
        { name: "store", description: null },
      ],
    });
    expect(yaml.operations.map((o) => [o.method, o.path, o.tag, o.summary])).toEqual([
      ["get", "/pets", "pets", "List pets"],
      ["post", "/pets", "pets", "Add a pet"],
      ["get", "/pets/{petId}", "pets", "Show a pet"],
      ["get", "/store/inventory", "store", "Count the pets by status"],
    ]);
  });

  it("carries parameters, bodies and the success response", async () => {
    const api = apiOf(await readFixture("petstore.yaml"));
    const list = find(api, "get", "/pets");
    expect(list.params).toMatchObject([
      { name: "limit", in: "query", required: true, samples: [10], schema: { types: ["integer"] } },
      { name: "offset", in: "query", required: false, samples: [] },
    ]);
    expect(list.body).toBeNull();
    expect(list.security).toEqual([]);
    expect(list.success).toMatchObject({
      code: "200",
      body: { mediaType: "application/json", json: true, schema: { types: ["array"] } },
    });
    const pet = list.success!.body!.schema!.items!;
    expect(pet).toMatchObject({
      types: ["object"],
      required: ["id", "name"],
      properties: {
        id: { types: ["integer"], samples: [1] },
        name: { types: ["string"] },
        tag: { types: ["string"] },
      },
    });

    const add = find(api, "post", "/pets");
    expect(add.body).toMatchObject({ mediaType: "application/json", json: true });
    expect(add.body!.schema).toBe(pet);
    expect(add.success).toEqual({ code: "201", body: null });

    const show = find(api, "get", "/pets/{petId}");
    expect(show.params).toMatchObject([{ name: "petId", in: "path", required: true }]);
    expect(show.success!.body!.schema).toBe(pet);

    const inventory = find(api, "get", "/store/inventory");
    expect(inventory.success!.code).toBe("200");
    expect(inventory.success!.body!.schema).toMatchObject({ types: ["object"], properties: null });
  });
});

describe("the version", () => {
  it("refuses Swagger 2.0 at the line of its field", async () => {
    expect(await readFixture("swagger.json").then(errorsOf)).toEqual([
      `${fixture("swagger.json")}:2: Swagger 2.0 is not supported: Grace imports OpenAPI 3.0 to 3.2`,
    ]);
  });

  it.each([
    ["openapi: 3.3.0\ninfo: {}\n", 1, "OpenAPI 3.3.0 is not supported"],
    ["info: {}\nopenapi: 4.0.0\n", 2, "OpenAPI 4.0.0 is not supported"],
    ["openapi: 3.0\n", 1, "OpenAPI 3.0 is not supported"],
    ["info: {}\npaths: {}\n", 1, "Not an OpenAPI spec: no openapi field"],
    ["<html><body>login</body></html>", 1, "Not an OpenAPI spec: no openapi field"],
    ["", 1, "Not an OpenAPI spec: no openapi field"],
  ])("refuses %j at line %i", async (text, line, reason) => {
    const suffix = reason.startsWith("OpenAPI") ? ": Grace imports OpenAPI 3.0 to 3.2" : "";
    expect(errorsOf(await readText(text))).toEqual([`${SPEC}:${line}: ${reason}${suffix}`]);
  });

  it("stops at the version and reports nothing else", async () => {
    const text = 'swagger: "2.0"\ninfo: 5\npaths:\n  pets: {}\n';
    expect(errorsOf(await readText(text))).toHaveLength(1);
  });

  it("words a failed read with its line, a missing URL included", async () => {
    expect(errorsOf(await readFixture("bad.json"))[0]).toContain(`${fixture("bad.json")}:9: `);
    const server = createServer((_, res) => {
      res.statusCode = 404;
      res.end();
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    try {
      const url = `http://127.0.0.1:${(server.address() as AddressInfo).port}/nope.yaml`;
      expect(errorsOf(await readApi({ kind: "url", url }, options()))).toEqual([
        `${url}:1: ${url} answered 404`,
      ]);
    } finally {
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    }
  });
});

describe("what the paths reach", () => {
  it("leaves unused components, webhooks and extensions unchecked and unloaded", async () => {
    const text = `${HEAD}x-thing:
  $ref: nowhere.yaml
webhooks:
  hook:
    $ref: other.yaml
paths:
  x-paths-note:
    $ref: gone-paths.yaml
  /a:
    get:
      x-internal:
        $ref: elsewhere.yaml
      responses:
        x-hook:
          $ref: gone-responses.yaml
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: object
                example:
                  $ref: x
components:
  schemas:
    Broken:
      type: 7
`;
    const readFile = vi.fn(async () => text);
    const api = apiOf(await readApi({ kind: "file", path: SPEC }, options(readFile)));
    expect(readFile).toHaveBeenCalledTimes(1);
    expect(successSchema(api).samples).toEqual([{ $ref: "x" }]);
  });

  it("follows an alias to the node it names", async () => {
    const text = `${HEAD}x-defs:
  plain: &plain
    type: string
  ok: &ok
    description: ok
    content:
      application/json:
        schema:
          type: object
          properties:
            copy: *plain
paths:
  /a:
    get:
      responses:
        "200": *ok
`;
    expect(successSchema(apiOf(await readText(text)))).toMatchObject({
      types: ["object"],
      properties: { copy: { types: ["string"] } },
    });
  });
});

describe("aliases", () => {
  it("resolves thousands of them without reading the document again for each", async () => {
    const count = 2000;
    const pathItem = (i: number) => `  /p${i}:
    get:
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema: ${i % 2 ? "*s" : '{$ref: "#/components/schemas/S"}'}
`;
    const text = `${HEAD}x-defs:
  s: &s
    type: string
paths:
${Array.from({ length: count }, (_, i) => pathItem(i)).join("")}components:
  schemas:
    S: *s
`;
    const started = performance.now();
    const api = apiOf(await readText(text));
    // A margin for a loaded machine: a scan of the document per alias takes about 28 s here.
    expect(performance.now() - started).toBeLessThan(5000);
    expect(api.operations).toHaveLength(count);
    // A `$ref` to `S` and the alias `*s` both name the same node, so they give the same schema.
    const [byRef, byAlias] = api.operations.map((o) => o.success!.body!.schema!);
    expect(byRef).toMatchObject({ types: ["string"] });
    expect(byRef).toBe(byAlias);
  });

  it("takes an aliased scalar for the scalar it names", async () => {
    const text = withSchema(
      `                type: *t
                format: *f
                required: [*id]
                properties:
                  id:
                    type: string
`,
      `${HEAD}x-defs:
  t: &t object
  f: &f binary
  id: &id id
`,
    );
    expect(successSchema(apiOf(await readText(text)))).toMatchObject({
      types: ["object"],
      format: "binary",
      required: ["id"],
    });
  });

  it("leaves out a sample that contains itself and keeps the others", async () => {
    const text = `${HEAD}paths:
  /a:
    get:
      parameters:
        - name: q
          in: query
          example: &p
            self: *p
          examples:
            ok:
              value: fine
      responses:
        "200":
          description: ok
          content:
            application/json:
              example: &m [*m]
              examples:
                loop:
                  value: &l
                    again: *l
                fine:
                  value: 7
              schema:
                type: object
                example: &a
                  x: *a
                default: 5
`;
    const api = apiOf(await readText(text));
    const op = api.operations[0]!;
    expect(op.params[0]!.samples).toEqual(["fine"]);
    expect(op.success!.body!.samples).toEqual([7]);
    expect(successSchema(api).samples).toEqual([5]);
    expect(() => JSON.stringify(api)).not.toThrow();
  });
});

describe("info", () => {
  it("reads a scalar title and version as written", async () => {
    const text =
      'openapi: 3.0.3\ninfo:\n  title: 42\n  version: 1.0\npaths:\n  /a:\n    get:\n      responses:\n        "200":\n          description: ok\n';
    expect(apiOf(await readText(text))).toMatchObject({ title: "42", version: "1.0" });
  });

  it("refuses a title or version that is not a scalar, or is empty", async () => {
    const text = "openapi: 3.0.3\ninfo:\n  title: [a]\n  version:\npaths: {}\n";
    expect(errorsOf(await readText(text))).toEqual([
      `${SPEC}:3: info title must be a scalar`,
      `${SPEC}:4: info has no version`,
    ]);
  });
});

describe("the checks of R4", () => {
  const rows: [string, number, string][] = [
    ["info-version.yaml", 2, "info has no version"],
    ["path-key.yaml", 6, "path pets must start with /"],
    ["param-in.yaml", 14, "parameter limit has no in"],
    ["path-template.yaml", 6, "path parameter petId is not declared"],
    [
      "schema-type.yaml",
      14,
      "type must be one of string, number, integer, boolean, array, object, null",
    ],
    ["required.yaml", 15, "required must be a list of strings"],
    ["content.yaml", 9, "content must be a mapping"],
    ["ref-type.yaml", 14, "$ref must be a string"],
    ["no-operation.yaml", 5, "No operation to generate"],
  ];
  it.each(rows)("%s gives its line", async (file, line, reason) => {
    const name = `invalid/${file}`;
    expect(errorsOf(await readFixture(name))).toEqual([`${fixture(name)}:${line}: ${reason}`]);
  });

  it("reports every error, in the order the walk finds them", async () => {
    const text = `openapi: 3.0.3
info:
  title: T
paths:
  pets:
    get:
      responses:
        "200":
          description: ok
          content:
            application/json:
              schema:
                type: str
                required: id
                items:
                  $ref: "#/components/schemas/Nope"
`;
    expect(errorsOf(await readText(text))).toEqual([
      `${SPEC}:2: info has no version`,
      `${SPEC}:5: path pets must start with /`,
      `${SPEC}:13: type must be one of string, number, integer, boolean, array, object, null`,
      `${SPEC}:14: required must be a list of strings`,
      `${SPEC}:16: $ref #/components/schemas/Nope points at nothing`,
    ]);
  });

  it("shows 20 errors, then how many more", async () => {
    const op = '    get:\n      responses:\n        "200":\n          description: ok\n';
    const paths = Array.from({ length: 21 }, (_, i) => `  p${i}:\n${op}`).join("");
    const errors = errorsOf(await readText(`${HEAD}paths:\n${paths}`));
    expect(errors).toHaveLength(21);
    expect(errors[0]).toBe(`${SPEC}:6: path p0 must start with /`);
    expect(errors[19]).toBe(`${SPEC}:${6 + 19 * 5}: path p19 must start with /`);
    expect(errors[20]).toBe("and 1 more");
  });

  it("shows an error once when several references reach the same missing file", async () => {
    const text = withSchema(
      `                type: object
                properties:
                  a:
                    $ref: missing.yaml#/A
                  b:
                    $ref: missing.yaml#/B
`,
    );
    expect(errorsOf(await readText(text))).toEqual([
      `${SPEC}:17: $ref missing.yaml#/A: file not found`,
    ]);
  });

  it("checks what 3.0 lacks: type lists, boolean schemas, querystring", async () => {
    const schema = `                type: object
                properties:
                  a:
                    type: [string, "null"]
                  b: true
`;
    expect(errorsOf(await readText(withSchema(schema)))).toEqual([
      `${SPEC}:17: type must be one of string, number, integer, boolean, array, object, null`,
      `${SPEC}:18: a schema must be a mapping`,
    ]);
    const querystring = `${HEAD.replace("3.0.3", "3.1.0")}paths:
  /a:
    get:
      parameters:
        - name: q
          in: querystring
      responses:
        "200":
          description: ok
`;
    expect(errorsOf(await readText(querystring))).toEqual([
      `${SPEC}:10: parameter q: in must be one of query, header, path, cookie`,
    ]);
  });
});

describe("operations", () => {
  const text = `${HEAD}security:
  - root_key: []
tags:
  - name: b
    description: The B tag
  - name: a
servers:
  - url: http://host.test
paths:
  /items/{id}:
    parameters:
      - name: id
        in: path
        required: true
        schema: {type: string}
      - name: shared
        in: query
        schema: {type: string}
        example: from-the-path-item
      - $ref: "#/components/parameters/Trace"
    post:
      tags: [c, b]
      summary: Make an item
      security:
        - oauth: [read]
          api_key: []
        - oauth: []
      parameters:
        - name: shared
          in: query
          required: true
          schema: {type: integer}
          examples:
            one: {value: 1}
            two: {value: 2}
      requestBody:
        content:
          application/xml:
            schema: {type: string}
          application/vnd.api+json:
            example: {a: 1}
            schema:
              $ref: "#/components/schemas/Item"
      responses:
        "404": {description: gone}
        "202": {description: accepted}
        "201":
          description: created
          content:
            text/plain:
              schema: {type: string}
        default: {description: boom}
    get:
      security: []
      responses:
        2XX:
          description: fine
  /files:
    put:
      requestBody:
        content:
          multipart/form-data:
            schema: {type: object}
      responses:
        "500": {description: boom}
components:
  parameters:
    Trace: {name: X-Trace, in: header, schema: {type: string}}
  schemas:
    Item:
      type: object
      nullable: true
      properties:
        id: {type: integer, format: int64, minimum: 1, readOnly: true}
        note: {type: string, writeOnly: true}
      allOf:
        - {type: object}
      oneOf:
        - {type: string}
      anyOf:
        - {type: number}
`;

  it("orders them as the paths and R5.2 do, and lists the tags", async () => {
    const api = apiOf(await readText(text));
    expect(api.operations.map((o) => `${o.method} ${o.path}`)).toEqual([
      "get /items/{id}",
      "post /items/{id}",
      "put /files",
    ]);
    expect(api.server).toBe("http://host.test");
    expect(api.tags).toEqual([
      { name: "b", description: "The B tag" },
      { name: "a", description: null },
      { name: "c", description: null },
    ]);
  });

  it("merges the path item's parameters under the operation's, and follows a $ref", async () => {
    const api = apiOf(await readText(text));
    const post = find(api, "post", "/items/{id}");
    expect(post.params.map((p) => [p.name, p.in, p.required])).toEqual([
      ["id", "path", true],
      ["shared", "query", true],
      ["X-Trace", "header", false],
    ]);
    expect(post.params[1]).toMatchObject({ samples: [1, 2], schema: { types: ["integer"] } });
    expect(find(api, "get", "/items/{id}").params[1]).toMatchObject({
      required: false,
      samples: ["from-the-path-item"],
    });
  });

  it("takes the operation's security, even an empty one, else the root's", async () => {
    const api = apiOf(await readText(text));
    expect(find(api, "post", "/items/{id}").security).toEqual(["oauth", "api_key"]);
    expect(find(api, "get", "/items/{id}").security).toEqual([]);
    expect(find(api, "put", "/files").security).toEqual(["root_key"]);
  });

  it("picks the JSON media type, else the first, and the lowest 2xx, else 2XX, else none", async () => {
    const api = apiOf(await readText(text));
    const post = find(api, "post", "/items/{id}");
    expect(post.tag).toBe("c");
    expect(post.summary).toBe("Make an item");
    expect(post.body).toMatchObject({
      mediaType: "application/vnd.api+json",
      json: true,
      samples: [{ a: 1 }],
    });
    expect(post.success).toMatchObject({
      code: "201",
      body: { mediaType: "text/plain", json: false, schema: { types: ["string"] } },
    });
    expect(find(api, "get", "/items/{id}").success).toEqual({ code: "2XX", body: null });
    const put = find(api, "put", "/files");
    expect(put.body).toMatchObject({ mediaType: "multipart/form-data", json: false });
    expect(put.success).toBeNull();
  });

  it("reads every schema keyword the model has", async () => {
    const item = find(apiOf(await readText(text)), "post", "/items/{id}").body!.schema!;
    expect(item).toMatchObject({
      types: ["object"],
      nullable: true,
      allOf: [{ types: ["object"] }],
      oneOf: [{ types: ["string"] }],
      anyOf: [{ types: ["number"] }],
      properties: {
        id: { format: "int64", minimum: 1, readOnly: true, writeOnly: false },
        note: { readOnly: false, writeOnly: true },
      },
    });
  });

  it("keeps a property called __proto__ as a property", async () => {
    const schema = `                type: object
                properties:
                  __proto__:
                    type: string
`;
    const found = successSchema(apiOf(await readText(withSchema(schema))));
    expect(Object.keys(found.properties!)).toEqual(["__proto__"]);
    expect(Object.getPrototypeOf(found.properties)).toBe(Object.prototype);
  });
});

describe("OpenAPI 3.1 and 3.2", () => {
  it("reads type lists, examples lists, constants and boolean schemas", async () => {
    const api = apiOf(await readFixture("v31.yaml"));
    const { name, anything, nothing, size } = successSchema(api).properties!;
    expect(name).toMatchObject({ types: ["string"], nullable: true, samples: ["first", "second"] });
    expect(anything).toEqual(empty);
    expect(nothing).toEqual(empty);
    expect(size!.samples).toEqual([1, 2, 3, 4, 5]);
    expect(api.server).toBeNull();
  });

  it("lists the query method and the additional operations after the standard ones", async () => {
    const api = apiOf(await readFixture("v32.yaml"));
    expect(api.operations.map((o) => `${o.method} ${o.path}`)).toEqual([
      "get /docs",
      "query /docs",
      "COPY /docs",
      "post /other",
    ]);
    expect(find(api, "query", "/docs").params).toEqual([
      { name: "filter", in: "querystring", required: false, samples: [], schema: null },
    ]);
    expect(find(api, "COPY", "/docs")).toMatchObject({
      summary: "Copy a doc",
      success: { code: "204", body: null },
    });
  });

  it("does not take a query key for an operation before 3.2", async () => {
    const text = `openapi: 3.1.0
info:
  title: T
  version: "1"
paths:
  /a:
    query:
      responses:
        "200":
          description: ok
`;
    expect(errorsOf(await readText(text))).toEqual([`${SPEC}:5: No operation to generate`]);
  });
});

describe("a cycle", () => {
  it("is the same object again", async () => {
    const node = successSchema(apiOf(await readFixture("cycle.yaml")));
    expect(node.properties!.value).toMatchObject({ types: ["string"] });
    expect(node.properties!.next).toBe(node);
  });

  it("through a chain of references ends too", async () => {
    const text = withSchema(
      '                $ref: "#/components/schemas/A"\ncomponents:\n  schemas:\n    A:\n      $ref: "#/components/schemas/B"\n    B:\n      $ref: "#/components/schemas/A"\n',
    );
    expect(errorsOf(await readText(text))).toEqual([
      `${SPEC}:18: $ref #/components/schemas/B leads back to itself`,
    ]);
  });
});

describe("the server", () => {
  const withServers = (servers: string) =>
    `openapi: 3.0.3\ninfo:\n  title: T\n  version: "1"\nservers:\n${servers}paths:\n  /a:\n    get:\n      responses:\n        "200":\n          description: ok\n`;

  it("replaces its variables with their defaults", async () => {
    const servers = `  - url: "{scheme}://{host}/v1"
    variables:
      scheme:
        default: https
      host:
        default: api.example.com
`;
    expect(apiOf(await readText(withServers(servers))).server).toBe("https://api.example.com/v1");
  });

  it("makes a relative URL absolute against localhost for a file", async () => {
    expect(apiOf(await readText(withServers("  - url: /v1\n"))).server).toBe(
      "http://localhost:8080/v1",
    );
  });
});

describe("split specs", () => {
  it("resolves the other files of a local spec", async () => {
    const seen: string[] = [];
    const readFile = async (p: string) => {
      seen.push(p);
      return read(p, "utf8");
    };
    const api = apiOf(
      await readApi({ kind: "file", path: fixture("split/root.yaml") }, options(readFile)),
    );
    expect(seen.sort()).toEqual(
      [
        fixture("split/root.yaml"),
        fixture("split/schemas/error.json"),
        fixture("split/schemas/pet.yaml"),
      ].sort(),
    );
    expect(successSchema(api)).toMatchObject({ required: ["id", "name"] });
  });

  describe("over http", () => {
    let server: Server;
    let base: string;
    const served: Record<string, string> = {};

    beforeEach(async () => {
      for (const name of ["root.yaml", "schemas/pet.yaml", "schemas/error.json"]) {
        served[`/dir/${name}`] = await read(fixture(`split/${name}`), "utf8");
      }
      served["/dir/relative.yaml"] =
        `${HEAD}servers:\n  - url: /v1\npaths:\n  /a:\n    get:\n      responses:\n        "200":\n          description: ok\n`;
      server = createServer((req, res) => {
        const body = served[req.url!];
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

    it("resolves the other files of a remote spec", async () => {
      const api = apiOf(await readApi({ kind: "url", url: `${base}/dir/root.yaml` }, options()));
      expect(successSchema(api)).toMatchObject({ required: ["id", "name"] });
      expect(api.server).toBeNull();
    });

    it("makes a relative server URL absolute against the spec's URL", async () => {
      const api = apiOf(
        await readApi({ kind: "url", url: `${base}/dir/relative.yaml` }, options()),
      );
      expect(api.server).toBe(`${base}/v1`);
    });

    it("words a missing referenced file at the $ref", async () => {
      delete served["/dir/schemas/pet.yaml"];
      const errors = errorsOf(
        await readApi({ kind: "url", url: `${base}/dir/root.yaml` }, options()),
      );
      expect(errors).toEqual([
        `${base}/dir/root.yaml:20: ${base}/dir/schemas/pet.yaml answered 404`,
      ]);
    });
  });
});
