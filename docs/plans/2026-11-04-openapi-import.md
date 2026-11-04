# OpenAPI import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From a folder's right-click menu or the empty tree, read an OpenAPI 3.0–3.2 spec from a URL or a file, preview the feature files it gives, and write them into the project only as the person confirms.

**Architecture:** A main-process module `src/main/openapi/` loaded by a dynamic `import()` on the first import: `fetch.ts` (guarded `net.fetch`), `docs.ts` (format, strict JSON, `yaml` documents with lines, the document cache and `$ref` targets), `walk.ts` (what `paths` reaches, R4 checks, a resolved model), `generate.ts` (R5, R6, pure), `write.ts` (folders, preview, write) and `session.ts` (the pending import behind the IPC calls). The Runner gains one method that keeps watch mode off the written files and refreshes. The renderer gets a two-step dialog, a folder menu on the tree and a button on the empty tree.

**Tech Stack:** Electron main (Node `fs/promises`, `net.fetch`), `yaml` 2.9.1, SolidJS 2 renderer, Vitest, Playwright.

**Spec:** `docs/specs/2026-11-04-openapi-import.md`. Decision: `docs/decisions/0030-yaml-as-the-openapi-reader.md`.

## Global Constraints

- `yaml` pinned exact `2.9.1` in `dependencies`; imported only by files under `src/main/openapi/`, which `src/main.ts` reaches only through `import("./main/openapi/session")`.
- Versions: `openapi` matching `^3\.[012]\.\d+` accepted. Messages, verbatim: "Swagger 2.0 is not supported: Grace imports OpenAPI 3.0 to 3.2", "OpenAPI <value> is not supported: Grace imports OpenAPI 3.0 to 3.2", "Not an OpenAPI spec: no openapi field", "No operation to generate", "More than 100 referenced documents", "<url>: a remote spec may not refer to a local file".
- URL rules (R2.2): absolute `http:`/`https:`, at most 2,000 characters, else "Enter an http or https URL"; with a user name or password, "Remove the user name and password from the URL".
- Fetch limits (R2.3): 30 s and 10 MB per document, 5 redirects followed by hand, each `http(s)`; non-2xx is "<url> answered <status>". Any host allowed.
- At most 100 documents per import; at most 20 errors shown, then "and <n> more"; error lines are `<file or URL>:<line>: <reason>`, a URL shown without user info and query.
- Generated text: double-quoted JSON for every value (`JSON.stringify`, `U+2028`/`U+2029` escaped); names lose control characters and line breaks; descriptions only as `#` lines; a string value starting with `#` becomes its placeholder. Line ends `\n`, UTF-8 without BOM.
- Karate syntax checked on Karate 2.1.4 (2026-11-04): `* url karate.get("baseUrl", "<server>")`, `And params {…}`, `And headers {…}`, `When method query`, `match response contains deep {…}` and `match each response contains deep {…}` with `##` on absent or null keys, `Then assert responseStatus >= 200 && responseStatus < 300`.
- Preview copy: "<n> operations in <m> features", "Exists", **Replace**, **Generate <n> files** (`1 file` singular), **Change…**, "Choose a folder inside the project", "Reading…". Notice: "Generated <n> files in <folder>".
- Settings key per project: `openapi`, an array of at most 50 `{ source, kind, folder, files, date }`, one per source.
- Solid 2 rules and UI conventions of `AGENTS.md`: classes from `src/renderer/buttons.ts` and `ui.ts`, `showModalWhile`, `FieldMessage`, `Icon`, no raw palette class, no `title=`, dialog content kept through its exit (`EXIT_MS`).
- Commits: Conventional Commits, subject ≤ 72 characters, body `Part of #69`, attribution footer. Keep out of `src/renderer/Details.tsx`, `src/main/karate/run-state.ts` and `src/main/karate/pom.ts`.
- Machine sharing: single unit test files any time; ask the parent session before full `npm test`, `npm run e2e`, `npm run package`.

## Review Focus

- A URL that answers an HTML page (a login wall) instead of a spec: refused with "Not an OpenAPI spec: no openapi field", not a crash. Task 2 test `html answer`.
- Tag names that slug to nothing (`"!!!"`), to a Windows reserved name (`con`, `nul`, `aux`, `com1`), or past 100 characters: a valid, unique file name. Task 4 test `file names`.
- Paths with a trailing slash, `//`, a parameter mid-segment, a non-ASCII segment, or a path parameter with no schema: a `Given path` that Karate accepts. Task 4 test `path segments`.
- The project closed, switched or busy between the preview and **Generate**: nothing written, a notice says why. Task 7 test `project changed`.
- A spec with 2,000 operations: generated in under a second, file names unique. Task 4 test `large spec`.

---

### Task 1: Guarded fetch

**Files:**
- Create: `src/main/openapi/fetch.ts`, `src/main/openapi/fetch.test.ts`
- Create: `src/shared/openapi.ts`

**Interfaces:**
- Produces (`src/shared/openapi.ts`, no Node imports): `urlError(url: string): string | null` (R2.2 messages, `null` when fine); `interface OpenApiPreview { title: string; version: string; operations: number; features: number; folder: string; files: { name: string; exists: boolean }[] }` (`folder` relative to the project with `/`, `""` for the project folder); `type OpenApiRead = { preview: OpenApiPreview } | { errors: string[] }`.
- Produces (`fetch.ts`): `type Fetch = (url: string, init: RequestInit) => Promise<Response>`; `displayUrl(url: string): string` (no user info, no query, no fragment); `fetchText(url: string, fetch: Fetch, signal: AbortSignal, limits?: { timeoutMs: number; maxBytes: number; maxRedirects: number }): Promise<{ text: string; url: string }>` (`url` is the final URL after redirects). Throws `Error` with messages `"<displayUrl> answered <status>"`, `"<displayUrl> is larger than 10 MB"`, `"<displayUrl> did not answer within 30 seconds"`, `"<displayUrl> redirects more than 5 times"`, `"<displayUrl> redirects to a URL that is not http or https"`.

- [ ] **Step 1: Write the failing tests** in `fetch.test.ts`, using a `node:http` server on `127.0.0.1:0` and Node's global `fetch` as the `Fetch`:
  - `urlError`: `"https://a.test/x.yaml"` → `null`; `"file:///C:/x.yaml"`, `"ftp://a"`, `"a.test/x"`, `""`, a 2,001-character URL → `"Enter an http or https URL"`; `"https://u:p@a.test/x"` and `"https://u@a.test/x"` → `"Remove the user name and password from the URL"`.
  - `displayUrl("https://u:p@a.test/x.yaml?token=1#f")` → `"https://a.test/x.yaml"`.
  - 200 body returned as `text` with the final `url`; 302 → 301 → 200 followed, `url` is the last; 6 redirects → the "more than 5" error; a redirect to `file:///C:/x` → the "not http or https" error, and the server sees no further request; 404 → `"http://127.0.0.1:<port>/x answered 404"`; a body of 10 MB + 1 byte streamed → the "larger than 10 MB" error; a server that never answers with `limits.timeoutMs = 200` → the "within 30 seconds" message (the text keeps 30 s, the test shortens the limit); an aborted `signal` → rejects with an `AbortError`.
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/fetch.test.ts`. Expected: FAIL, module not found.
- [ ] **Step 3: Implement** `urlError` in `src/shared/openapi.ts` (`URL.canParse`, `protocol`, `username`/`password`, length) and `fetchText` in `fetch.ts`: `redirect: "manual"`, `Location` resolved against the current URL and checked with `urlError`-like protocol rules; the body read from `response.body` in chunks, stopping past `maxBytes`; `AbortSignal.any([signal, AbortSignal.timeout(timeoutMs)])`, a timeout told apart from the caller's abort.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(openapi): fetch a spec with guarded redirects and limits`.

### Task 2: Documents and their lines

**Files:**
- Modify: `package.json`, `package-lock.json` (`npm install --save-exact yaml@2.9.1`)
- Create: `src/main/openapi/docs.ts`, `src/main/openapi/docs.test.ts`
- Create: `test/fixtures/openapi/` (`petstore.yaml`, `petstore.json`, `split/root.yaml` + `split/schemas/pet.yaml` + `split/schemas/error.json`, `swagger.json`, `bad-indent.yaml`, `bad.json`)

**Interfaces:**
- Consumes: `fetchText`, `displayUrl`, `Fetch` (Task 1).
- Produces:
  - `interface Loc { label: string; line: number }`; `class ImportError extends Error { constructor(at: Loc, reason: string) }` and `format(e: ImportError): string` → `"<label>:<line>: <reason>"`.
  - `interface Doc { uri: string; label: string; remote: boolean; root: yaml.Node | null; lines: yaml.LineCounter }`.
  - `class Docs { constructor(o: { fetch: Fetch; signal: AbortSignal; readFile?: (p: string) => Promise<string> }); load(uri: string, from?: Loc): Promise<Doc>; target(ref: string, holder: Doc, at: Loc): Promise<{ doc: Doc; node: yaml.Node | null }>; lineOf(doc: Doc, node: yaml.Node): number }`. `uri` is a `file:` URL for local files (from `pathToFileURL`) or the `http(s)` URL; `label` is the absolute file path or `displayUrl`.
  - `parseText(text: string, label: string): { root: yaml.Node | null; lines: yaml.LineCounter }`, throwing `ImportError`.

- [ ] **Step 1: Write the failing tests** in `docs.test.ts`:
  - `parseText`: YAML and JSON of the Petstore give the same `toJS()`; JSON with tabs and a BOM parses; `{"a": 1,\n "b": }` throws at line 2 (strict JSON, though YAML would accept it); `bad-indent.yaml` throws at line 7; a JSON duplicate key throws at its line; `"<html><body>login</body></html>"` parses to a scalar (refused later by Task 3).
  - `Docs.load` of a local file twice reads it once (count `readFile` calls); loading a 101st document throws "More than 100 referenced documents".
  - `Docs.target`: `#/components/schemas/Pet` in the same document; `schemas/pet.yaml#/Pet` relative to a file; `pet.json` relative to an `http://127.0.0.1` document served by a test server; `#/a~1b/c~0d` and `#/x%20y` decoded; a missing pointer throws `ImportError` at the `$ref`'s line with reason `$ref <ref> points at nothing`; a remote holder with `file:///C:/x.yaml` or `C:/x.yaml` throws "<url>: a remote spec may not refer to a local file" and calls `readFile` zero times; a local holder may target `http://127.0.0.1:<port>/x.yaml`.
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/docs.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** `docs.ts`: format by the first character after a BOM and white space (`{` or `[` → `JSON.parse` first, its V8 message's `line <n>` taken as the line, else 1); then `parseDocument(text, { lineCounter, uniqueKeys: true, version: "1.2", customTags: [] })`, its first error thrown with `linePos[0].line`. References resolve with `new URL(ref, holder.uri)`; a remote holder whose result is not `http(s)`, or a bare Windows path, is refused. Cache by URI without fragment.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(openapi): read json and yaml documents with their lines` (body names the dependency and its size: 686 KB unpacked, about 115 KB minified, no dependencies).

### Task 3: Walk, check and model

**Files:**
- Create: `src/main/openapi/model.ts`, `src/main/openapi/walk.ts`, `src/main/openapi/walk.test.ts`
- Create: fixtures `test/fixtures/openapi/` `v31.yaml` (type arrays, `examples` arrays, boolean schemas), `v32.yaml` (`query` method, `additionalOperations`, `querystring` param), `cycle.yaml`, `invalid/*.yaml` (one per R4 check)

**Interfaces:**
- Consumes: `Docs`, `Doc`, `Loc`, `ImportError`, `format` (Task 2).
- Produces (`model.ts`):
  - `interface Schema { types: string[]; nullable: boolean; properties: Record<string, Schema> | null; required: string[]; items: Schema | null; allOf: Schema[]; oneOf: Schema[]; anyOf: Schema[]; format: string | null; minimum: number | null; readOnly: boolean; writeOnly: boolean; samples: unknown[] }` — `samples` in R5.5 order (`example`, `examples[0]`, `const`, `default`, `enum[0]`); the same `$ref` target gives the same object, so a cycle is a cyclic graph.
  - `interface Param { name: string; in: "query" | "header" | "path" | "cookie" | "querystring"; required: boolean; samples: unknown[]; schema: Schema | null }`
  - `interface Body { mediaType: string; json: boolean; samples: unknown[]; schema: Schema | null }`
  - `interface Operation { method: string; path: string; tag: string | null; summary: string | null; params: Param[]; body: Body | null; success: { code: string; body: Body | null } | null; security: string[] }`
  - `interface Api { title: string; version: string; server: string | null; tags: { name: string; description: string | null }[]; operations: Operation[] }`
- Produces (`walk.ts`): `type Source = { kind: "url"; url: string } | { kind: "file"; path: string }`; `readApi(source: Source, o: { fetch: Fetch; signal: AbortSignal; readFile?: (p: string) => Promise<string> }): Promise<{ api: Api } | { errors: string[] }>` — errors already formatted and capped (20, then `"and <n> more"`).

- [ ] **Step 1: Write the failing tests** in `walk.test.ts`:
  - Petstore YAML and JSON give equal `Api`s: 4 operations, tags `pets` then `store`, server `http://petstore.example/v1`: `GET /pets` (required query `limit` with example 10, optional `offset`; 200, an array of `Pet`), `POST /pets` (a JSON `Body` of `Pet`; `"201"`), `GET /pets/{petId}` (one required path `Param`; `"200"`, a `Pet`), all tagged `pets`; `GET /store/inventory` tagged `store` (`"200"`, an object). `Pet` has required `id` (integer) and `name` (string), optional `tag` (string).
  - Versions: `swagger.json` → `["<path>:1: Swagger 2.0 is not supported: Grace imports OpenAPI 3.0 to 3.2"]`; `openapi: 3.3.0` and `openapi: 4.0.0` → the "OpenAPI <value> is not supported" line; neither field → "Not an OpenAPI spec: no openapi field"; the HTML scalar → the same.
  - Reach (R3.3): an unused `components.schemas.Broken: { type: 7 }`, a `webhooks` entry and an `x-thing: { $ref: nowhere.yaml }` give no error and load no document; an `example: { $ref: "x" }` is kept as data.
  - Each R4 check, one fixture each, gives its line: `info` without `version`; a path key `pets`; a parameter without `in` (`<file>:14: parameter limit has no in`); a `{petId}` template with no `petId` parameter; `type: str`; `required: "id"`; `content: []`; a `$ref: 5`; no operation → "No operation to generate".
  - 21 errors → 20 lines and `"and 1 more"`.
  - Split spec: `split/root.yaml` resolves both other files; a remote split spec served by a test server resolves too.
  - 3.1: `type: [string, "null"]` → `types: ["string"]`, `nullable: true`; `true` schema → empty `Schema`. 3.2: `query` and `additionalOperations: { COPY: … }` operations listed after the standard ones; a `querystring` parameter accepted.
  - `cycle.yaml`: `Node.properties.next` is the same object as `Node`.
  - Server: `{scheme}://{host}/v1` with variable defaults substituted; a relative `/v1` resolved against a URL source; no `servers` → `null`.
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/walk.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** `walk.ts`: an async walk of `paths` → path items → operations (methods in R5.2 order) → parameters (path item's merged under the operation's by `name`+`in`), `requestBody`, `responses` (lowest `2xx`, then `2XX`), media types (the first JSON one: `application/json` or `+json`, else the first), schemas, following `$ref` through `Docs.target` with a `Map<string, Schema>` keyed `uri#pointer` filled before descending (cycles). Errors collected, not thrown, so all of them show; the version check stops the walk.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(openapi): check what the paths reach and build the model`.

### Task 4: Generate features

**Files:**
- Create: `src/main/openapi/generate.ts`, `src/main/openapi/generate.test.ts`, `test/fixtures/openapi/petstore/pets.feature`, `test/fixtures/openapi/petstore/store.feature` (the expected text), `test/fixtures/openapi/hostile.yaml`

**Interfaces:**
- Consumes: `Api`, `Operation`, `Schema` (Task 3).
- Produces: `interface FeatureFile { name: string; text: string }`; `slug(s: string): string` (R5.1, `""` when nothing is left); `generate(api: Api, o: { source: string; date: string }): FeatureFile[]` (`source` already shown as R3.5 says, `date` as `yyyy-mm-dd`).

- [ ] **Step 1: Write the failing tests** in `generate.test.ts`:
  - Petstore: `generate(...)` equals the two fixture files byte for byte (write the fixtures by hand from R5 first; they are the review point). They start with `# Generated by Grace from petstore.yaml on 2026-11-04: Petstore 1.0.0`, have `* url karate.get("baseUrl", "http://petstore.example/v1")`, `Scenario: GET /pets/{petId}`, `Given path "pets", 1`, `And params {"limit":10}` only for a required `limit`, `# Optional params: …`, `Then status 200`, `And match each response contains deep {"id":"#number","name":"#string","tag":"##string"}`.
  - Rules: each R5.4 marker (`"#[] #string"`, `"#object"`, `"##…"` for optional and nullable, `"#present"` / `"#ignore"` for `oneOf`, `allOf` merged, depth 5 → `"#object"`, an optional object property → `"##object"`); each R5.5 value source in its order and each `format` placeholder; `readOnly` left out of a request; `2XX` → the `assert` line; no success response → the comment and no `Then`; a non-JSON body → `# Request body multipart/form-data not generated`; `security` → `# Security: api_key`; no server → `http://localhost:8080`; a cycle → `null` in a value, `"#ignore"` in a match.
  - `file names`: tags `Pets`, `pets` → `pets.feature`, `pets-2.feature`; `"!!!"` → the path's first segment, then `default`; `con`, `nul`, `aux`, `com1` → `con-feature.feature` style suffix so Windows accepts it; 150 characters → cut to 100; untagged `/store/order` → `store.feature`.
  - `path segments`: `/pets/` → `Given path "pets", ""`-free output (empty segments dropped); `//a` → `"a"`; `/files/{name}.json` → `"files", "report.json"` with example `report`; `/città/{id}` keeps `"città"`; a path parameter with no schema or sample → `"string"`.
  - Hostile (`hostile.yaml`): a description with `Scenario: x` and `* karate.exec('calc')` lines becomes only `#` lines; a tag named `a\nScenario: b` and one with `U+2028` produce single-line `Feature:` lines; an example `"#(karate.exec('calc'))"` and a nested `{ "n": "#(x)" }` become `"string"`; no generated line outside a comment contains `karate.exec`.
  - `large spec`: 2,000 synthetic operations over 300 tags generate in under 1,000 ms with 300 unique names.
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/generate.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** `generate.ts`: one `js(value)` that is `JSON.stringify` with `\u2028`/`\u2029` escaped, used for every value; one `line(text)` that drops control and line-break characters; `comment(text)` that splits on any line break into `# ` lines. Windows reserved names: `con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`, compared case-insensitively.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(openapi): generate karate features from the model`.

### Task 5: The generated features pass in Karate

**Files:**
- Create: `src/main/openapi/karate.test.ts`

**Interfaces:**
- Consumes: `readApi` (Task 3), `generate` (Task 4), `vendorPaths` (`src/main/vendor.ts`).

- [ ] **Step 1: Write the test**: a `node:http` server that follows `petstore.yaml` (returns documented bodies, plus one undocumented field per object, and `201` for `POST /pets`); `readApi` of the fixture, `generate`, the files written to a temp folder with a `karate-config.js` that sets `baseUrl` to the server; Karate run with `execFile` (async: a sync call blocks the server) as `java -cp <karate.jar> io.karatelabs.Main run --no-color -w <dir> -o <dir>/out <dir>`; expect the output to hold `scenarios:   4 | passed:    4`. A second case serves a wrong type (`id: "1"`) and expects a failure, so the match is not vacuous.
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/karate.test.ts`. Expected: PASS (both cases), under 30 s.
- [ ] **Step 3: Commit** `test(openapi): run the generated petstore features in karate`.

### Task 6: Folders, preview and write; the kept source

**Files:**
- Create: `src/main/openapi/write.ts`, `src/main/openapi/write.test.ts`
- Modify: `src/main/settings.ts`, `src/main/settings.test.ts`

**Interfaces:**
- Consumes: `FeatureFile` (Task 4); `detectLayout`, `locate`, `inside` (`src/main/karate/command.ts`); `Layout` (`src/shared/model.ts`).
- Produces (`write.ts`):
  - `startFolder(root: string, folder: string | null): string | null` — absolute; `null` input → the first feature root, else `src/test/java` for `maven`/`gradle`, else `root`; a tree folder → `locate(detectLayout(root).bases, folder)`; `null` result when that is not `inside(root, …)`.
  - `relFolder(root: string, abs: string): string` — with `/`, `""` for the root.
  - `existing(dir: string, names: string[]): Set<string>`.
  - `writeFeatures(dir: string, files: FeatureFile[], replace: Set<string>, existed: Set<string>): Promise<{ written: string[]; skipped: string[]; failed: { name: string; reason: string } | null }>` — new files with flag `wx` (an `EEXIST` is skipped), ticked existing ones with `w`, unticked existing ones never opened; stops at the first other error.
- Produces (`settings.ts`): `interface OpenApiSource { source: string; kind: "url" | "file"; folder: string; files: string[]; date: string }`; `saveOpenApiSource(dataDir: string, root: string, e: OpenApiSource): void`; `loadOpenApiSources(dataDir: string, root: string): OpenApiSource[]`.

- [ ] **Step 1: Write the failing tests**:
  - `startFolder`: a Maven fixture with `src/test/java` → that folder for `null`; a Maven folder with no `src/test` → `<root>/src/test/java`; a standalone folder → `root`; a tree folder `api` → `<karate root>/api`; `"../x"` → `null`.
  - `writeFeatures`: two new files written with `\n` ends and no BOM; an existing unticked file keeps its bytes and is neither written nor skipped-listed; a ticked one is replaced; a file created after the preview (not in `existed`, present at write) is in `skipped` and keeps its bytes; a read-only target folder → `failed` with the first file named, earlier files kept.
  - `saveOpenApiSource`: saves under the project's entry key `openapi`; a second save of the same `source` replaces it; the 51st drops the oldest; other keys of the entry survive.
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/write.test.ts src/main/settings.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** with `fs/promises`; `saveOpenApiSource` through the existing `updateEntry`, `loadOpenApiSources` through `savedEntry`, dropping entries of the wrong shape.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(openapi): preview and write the features, and keep the source`.

### Task 7: Session, IPC and main wiring

**Files:**
- Create: `src/main/openapi/session.ts`, `src/main/openapi/session.test.ts`, `src/main/runner.openapi.test.ts`
- Modify: `src/shared/ipc.ts`, `src/preload.ts`, `src/shared/events.ts`, `src/main.ts`, `src/main/runner.ts`

**Interfaces:**
- Consumes: Tasks 1–6.
- Produces:
  - Event `{ type: "openApiImport"; folder: string }` in `AppEvent`.
  - `CH.folderMenu = "openapi:menu"`, `CH.openApiUrl = "openapi:url"`, `CH.openApiFile = "openapi:file"`, `CH.openApiFolder = "openapi:folder"`, `CH.cancelOpenApi = "openapi:cancel"`, `CH.writeOpenApi = "openapi:write"`.
  - `Api`: `folderMenu(folder: string, x: number, y: number): Promise<void>`; `openApiUrl(folder: string | null, url: string): Promise<OpenApiRead | null>`; `openApiFile(folder: string | null): Promise<OpenApiRead | null>` (`null`: cancelled or no file picked); `openApiFolder(): Promise<OpenApiPreview | null>`; `cancelOpenApi(): Promise<void>`; `writeOpenApi(replace: string[]): Promise<boolean>`.
  - `class OpenApiImport { constructor(d: { dataDir: string; project(): Project | null; busy(): boolean; fetch: Fetch; imported(paths: string[]): Promise<void>; emit(e: AppEvent): void; now?: () => Date }); read(folder: string | null, source: Source): Promise<OpenApiRead | null>; cancel(): void; folderPath(): string | null; changeFolder(abs: string): OpenApiPreview | null; write(replace: unknown): Promise<boolean> }`.
  - `Runner.projectOf(): Project | null`; `Runner.imported(paths: string[]): Promise<void>` — records each path with its `mtimeMs`, skips them in the watch batch while their `mtimeMs` is unchanged, then `refresh()`.

- [ ] **Step 1: Write the failing tests**:
  - `session.test.ts` (fake deps, fixtures, temp project): `read(null, file)` → preview with folder `src/test/java/petstore`, both files not existing; `read` while `busy()` → refused with the busy notice; a second `read` drops the first pending import; `cancel()` during a slow fetch → `read` resolves `null`; `changeFolder` outside the root → `null` and the notice "Choose a folder inside the project"; `write(["pets.feature"])` writes, saves the source (URL kept whole with its query), emits "Generated 2 files in src/test/java/petstore" with `success: true`, calls `imported` with both absolute paths; `write(["../x"])` or a name it did not list → `false`, "Invalid request"; `project changed`: `project()` now returns another root, or `busy()` is true → `false`, nothing written, notice "The project changed: read the spec again" or the busy notice.
  - `runner.openapi.test.ts`: with watch on, `imported([f])` on a newly written feature starts no run; saving `f` again afterwards does start one; `imported` refreshes the tree (the new feature appears).
- [ ] **Step 2: Run** `npx vitest run src/main/openapi/session.test.ts src/main/runner.openapi.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** `session.ts`, the Runner methods, the IPC entries and `main.ts`: `const openApi = () => (importer ??= import("./main/openapi/session").then((m) => new m.OpenApiImport({ ..., fetch: (u, i) => net.fetch(u, i) })))`. Handlers check arguments before the session sees them (folder: `null` or a string ≤ 1,000 characters; URL: `urlError`; `replace`: an array of ≤ 1,000 strings). `folderMenu` pops `Menu.buildFromTemplate([{ label: "Import OpenAPI here…", enabled: !runner.isBusy(), click: () => emit({ type: "openApiImport", folder }) }])` as `editorMenu` does. `openApiFile` and `openApiFolder` show the dialogs of R2.4 and R7.1 (`defaultPath` = `folderPath()` or its nearest existing parent).
- [ ] **Step 4: Run** the same command, then `npx vitest run src/main/runner.watch.test.ts`. Expected: PASS.
- [ ] **Step 5: Check the lazy chunk** (ask the parent session before packaging): `npm run package`, then `ls .vite/build` shows a chunk besides `main.cjs` and `preload.cjs`, and `grep -c LineCounter .vite/build/main.cjs` prints `0`.
- [ ] **Step 6: Commit** `feat(openapi): wire the import to the window through main`.

### Task 8: The dialog, the tree menu and the empty tree

**Files:**
- Create: `src/renderer/OpenApiDialog.tsx`, `src/renderer/OpenApiDialog.test.tsx`
- Modify: `src/renderer/reduce.ts`, `src/renderer/store.ts`, `src/renderer/App.tsx`, `src/renderer/Tree.tsx`, `src/renderer/Tree.test.tsx`, `src/renderer/store.test.tsx`

**Interfaces:**
- Consumes: the `Api` calls and the event of Task 7; `urlError`, `OpenApiPreview`, `OpenApiRead` (Task 1).
- Produces: `AppState.importing: { folder: string | null } | null`; store actions `openImport(folder: string | null): void` and `closeImport(): void`; `Tree` prop `openImport(folder: string | null): void`; `OpenApiDialog(props: { state: AppState; api: Api; close(): void })`.

- [ ] **Step 1: Write the failing tests**:
  - `store.test.tsx`: an `openApiImport` event sets `importing` to `{ folder: "api" }`; `closeImport()` clears it.
  - `Tree.test.tsx`: right-clicking a folder row calls `api.folderMenu("api", x, y)`; a feature row still calls `editorMenu`; the empty tree shows **Import OpenAPI…**, disabled while busy, and a click calls `openImport(null)`.
  - `OpenApiDialog.test.tsx`: **Read** with `ftp://x` shows "Enter an http or https URL" and calls nothing; **Read** with a URL calls `openApiUrl(folder, url)` and shows "Reading…" with **Cancel** (calls `cancelOpenApi`); errors list each line; a preview shows "Petstore 1.0.0", "4 operations in 2 features", `src/test/java/petstore/`, `pets.feature` and `store.feature` with "Exists" and an unticked **Replace** on the existing one, and **Generate 1 file**; ticking **Replace** makes it **Generate 2 files** and sends `["pets.feature"]`; nothing to write disables it; **Change…** replaces the preview with `openApiFolder()`'s; **Back** returns to step 1; Escape calls `cancelOpenApi` and `close`; the content stays for `EXIT_MS` after closing.
- [ ] **Step 2: Run** `npx vitest run src/renderer/OpenApiDialog.test.tsx src/renderer/Tree.test.tsx src/renderer/store.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement**: the dialog as `PropertiesDialog` draws its `<dialog>` (`dialog` style from `ui.ts`, `showModalWhile`, `btnDialog` / `primaryDialog`, `FieldMessage`, `input`), mounted in `App.tsx` beside `UpdateModal` with `close={closeImport}`; the empty tree's button with `btnSmall` beside the subfolder buttons; `onContextMenu` sends a folder row's `node.file` to `folderMenu`.
- [ ] **Step 4: Run** the same command, then `npx vitest run src/renderer/palette.test.ts`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): import an openapi spec from the tree`.

### Task 9: End to end, README

**Files:**
- Create: `e2e/openapi.spec.ts`
- Modify: `README.md`

**Interfaces:**
- Consumes: `launch`, `emptyProject`, `openProject`, `test`, `expect`, the `dialog.showOpenDialog` stub pattern (`e2e/helpers.ts`), the menu stub of `e2e/editor.spec.ts`.

- [ ] **Step 1: Write the tests** in `e2e/openapi.spec.ts`:
  - Local file: an empty Maven project (`pom.xml` only); **Import OpenAPI…** on the empty tree; `showOpenDialog` stubbed to `test/fixtures/openapi/petstore.yaml`; **Choose file…**; "4 operations in 2 features"; **Generate 2 files**; the notice "Generated 2 files in src/test/java/petstore"; the tree shows `pets.feature` and `store.feature`; a second import from the menu of the folder row that holds `petstore` (its target is that folder's `petstore/` again) shows both "Exists" with **Replace** unticked and a disabled **Generate 0 files**.
  - URL: a `node:http` server in the test serves `split/root.yaml` behind one redirect and its two referenced files; the URL typed, **Read**; the preview lists the features; **Generate**; the files exist on disk.
- [ ] **Step 2: Update** `README.md`'s features list with one line per R9.1.
- [ ] **Step 3: Ask the parent session for a go**, then run `npm run e2e`. Expected: all pass; paste the summary into the PR body.
- [ ] **Step 4: Commit** `test(e2e): import openapi from a file and a url` and `docs(readme): mention the openapi import`.
