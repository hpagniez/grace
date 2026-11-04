# Import an OpenAPI spec to feature files

Date 2026-11-04. Status: approved. Covers [#69](https://github.com/hpagniez/grace/issues/69). Prepares [#181](https://github.com/hpagniez/grace/issues/181) (sync with a remote spec). Decision record: [ADR 0030](../decisions/0030-yaml-as-the-openapi-reader.md).

## Intent

A team that starts testing an API with Karate has its OpenAPI spec already, and writes the first features by hand from it: one call per operation, the path, the parameters, a body, the status and the response shape. Karate's OSS CLI has no import. Karate Labs' paid IDE plugin has one (Enterprise, `docs/inventory/commercial.md`); Grace learns the feature set only and copies nothing.

With this feature the person right-clicks a folder of the tree, picks **Import OpenAPI here…**, gives a URL or a file, sees which feature files Grace will write, and gets one feature per tag with one scenario per operation, ready to run against the API. It is the first time Grace writes into a project, so every write is shown first and no file is replaced unless the person ticks it.

## Goals, non-goals, anti-goals

- Goals: OpenAPI 3.0, 3.1 and 3.2, in JSON or YAML, from an `http(s)` URL or a local file, split over several files or URLs; an invalid input refused with its file, line and reason, and nothing written; a preview of the files before anything is written; scenarios that pass against a server that follows the spec; each import's source kept for #181.
- Non-goals: Swagger 2.0 (refused with a message that says so); Postman and cURL import; error-case or negative scenarios; authentication steps (a comment names the security scheme; `karate-config.js` sets the headers, as Karate's own docs do); path- or operation-level `servers`; webhooks and callbacks; multipart, form and XML bodies (a comment names the media type); mocks; full validation of every rule of the OpenAPI standard (R4 says what is checked); syncing or merging an earlier import (#181); editing `karate-config.js`.
- Anti-goals: any cost while idle or for a person who never imports; a file replaced or removed without the person ticking it; any text of the spec becoming a Karate step or expression (R6); following a remote spec to a local file; running anything the spec holds; a run started by the import, watch mode's included.

## Constraints

- Follow the Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)): the CLI has no import, so this is Grace's own way. The generated files are plain Karate the CLI runs unchanged.
- Fetching and parsing happen in the main process; the window stays sandboxed. A URL typed by the person is a trust boundary (R2).
- Idle costs nothing ([ADR 0023](../decisions/0023-run-lightly-as-a-release-requirement.md)): the parser loads on the first import, not at start.
- One new dependency, `yaml` ([ADR 0030](../decisions/0030-yaml-as-the-openapi-reader.md)); the `$ref` resolver and the checks are Grace's own.

### What exists today

- Grace writes only into its own data folder (settings, runs, argument files). It has never written into a project.
- Per-project settings live in `settings.json`, keyed by the project folder (`src/main/settings.ts`).
- The updater fetches with Electron's `net.fetch`, which honours the system proxy (`src/main.ts`).
- The tree's feature and scenario rows open a native menu with **Open in VS Code** on right-click (`editorMenu`). Folder rows have no menu. A folder row's key is `d:<file>`, `file` relative to the feature roots, `""` for the root (`src/shared/model.ts`).
- The empty tree shows "No feature files found" and, when Grace found some, `Open <folder>/` buttons (`src/renderer/Tree.tsx`, [subfolder offer](2026-11-04-subfolder-offer.md)).
- `detectLayout` (`src/main/karate/command.ts`) gives the feature roots: `src/test/java` and `src/test/resources` for Maven and Gradle when they exist, `karate-pom.json` `paths`, or the project folder.
- Watch mode runs a feature when it is saved (`src/main/watch.ts`), new files included.

## Approaches considered

1. **`yaml` alone, with Grace's own `$ref` resolver and checks. Chosen by the owner.** One dependency with none of its own, about 115 KB minified. It reads YAML 1.2 and keeps every node's position, so each error gets its line. Grace checks what it reads and generates from (R4). Cost: it does not check every rule of the standard; a rule Grace does not read is not checked.
2. **`@apidevtools/swagger-parser`.** Validates against the official schemas with `ajv` and `json-schema-ref-parser`: 7 dependencies, several hundred KB more. Its errors name a JSON path, not a line, so Grace would still need `yaml` to find the line, and its OpenAPI 3.2 support is not confirmed.
3. **`@redocly/openapi-core`.** A full linter with lines, but 13 dependencies, `graphql` among them: too heavy for a `could` feature.

The owner's answers:

- One feature per tag, one scenario per operation, payloads from the spec's examples, the documented success status and a schema match of the response, and `url` from `baseUrl` with the spec's server as fallback.
- A preview before writing: the target folder, changeable, and the file list; an existing file is skipped unless ticked; Grace never removes a file and never edits `karate-config.js`; the source is kept in Grace's settings, not in the project; the tree refreshes after.
- The action lives on a folder's right-click menu and on the empty tree, not on the toolbar, which has no room at 760 px for a `could` feature.
- A remote document's references may reach any `http(s)` host, not only its own origin (R3.3): specs that share schemas across internal hosts import whole. What comes back stays in the preview and the person's files; the risk left is a GET to a local network address, as a web page can send.

## Requirements

### R1. Where the import starts

1. Right-clicking a folder row of the tree, the root included, opens a native menu with **Import OpenAPI here…**. Feature, scenario and row menus are unchanged.
2. The empty tree ("No feature files found") shows an **Import OpenAPI…** button below its text and any `Open <folder>/` buttons.
3. Both open the **Import OpenAPI** dialog. The folder the import writes into starts as the right-clicked folder (R7.1), or for the empty tree the default of R7.1.
4. While Grace is busy (a run, a discovery), the menu item and the button are disabled, and **Generate** (R7) is refused as any action is.

### R2. The source

1. The dialog's first step has a **URL** field with a **Read** button, and a **Choose file…** button.
2. A URL is accepted only if it parses as an absolute `http:` or `https:` URL of at most 2,000 characters. Anything else shows "Enter an http or https URL" under the field and fetches nothing. A URL with a user name or password shows "Remove the user name and password from the URL" and fetches nothing: Grace sends no credentials and keeps none in settings (R8.2).
3. Main fetches the URL with `net.fetch` (the system proxy applies), following at most 5 redirects itself, each to an `http:` or `https:` URL only. It gives up after 30 seconds per document, on a status other than 2xx ("<url> answered <status>"), or past 10 MB per document.
4. **Choose file…** opens Windows' open dialog in main, filtered to `*.yaml`, `*.yml`, `*.json` with an **All files** choice. Main reads the file; the window never sends a file path.
5. While Grace reads, the step shows "Reading…" and a **Cancel** button that stops the fetch. Closing the dialog cancels too.

### R3. Reading the spec

1. The format is told by the content, never by the extension or `Content-Type`: after a byte order mark and white space, `{` or `[` means JSON, read strictly (`JSON.parse` rules); anything else is YAML 1.2. A JSON or YAML syntax error is refused with its line.
2. The root must be a mapping with an `openapi` string `3.0.x`, `3.1.x` or `3.2.x`. A root with a `swagger` field is refused: "Swagger 2.0 is not supported: Grace imports OpenAPI 3.0 to 3.2". Another `openapi` value is refused: "OpenAPI <value> is not supported: Grace imports OpenAPI 3.0 to 3.2". A root with neither: "Not an OpenAPI spec: no openapi field".
3. Grace reads only what it generates from: the root fields, and each path item, operation, parameter, request body, response, media type and schema reached from `paths`, following `$ref`s as it goes. Unused `components`, `webhooks`, `callbacks` and `x-` extensions are neither resolved nor checked. A `$ref` key inside an `example`, `examples`, `enum`, `const` or `default` value is data, not a reference. Each reached `$ref` is resolved:
   - `#/…` within the same document, as a JSON pointer (`~0`, `~1` and percent-encoding decoded);
   - a relative or absolute reference to another document, resolved against the URL or path of the document that holds it, and read by R3.1;
   - a document read from a URL resolves its references against that URL, and only to `http:` or `https:`, on any host, as a browser would. A reference from a remote document to `file:` or a local path is refused: "<url>: a remote spec may not refer to a local file".
   - A document read from a local file may refer to local files and to `http(s)` URLs.
4. Each document is read once per import, whatever number of references point at it. A cycle of references (a schema that contains itself) is followed once and stops there (R5.6). At most 100 documents per import; the 101st is refused: "More than 100 referenced documents".
5. Any error refuses the whole import: nothing is generated and nothing is written. The dialog lists each error as `<file or URL>:<line>: <reason>`, the URL without its user name, password and query; at most 20 errors, then "and <n> more".

### R4. What is checked

Each failed check is an error of R3.5 at the line of the value it names. Only what R3.3 reaches is checked.

1. `info` is a mapping with a string `title` and a string `version`.
2. `paths`, when present, is a mapping whose keys start with `/`; each path item and each operation is a mapping.
3. A parameter is a mapping with a string `name` and `in` one of `query`, `header`, `path`, `cookie` (and `querystring` from 3.2). A path parameter in a path's `{…}` template exists for each name, on the path item or the operation.
4. A schema is a mapping (or, from 3.1, `true` or `false`). Its `type`, when present, is one of `string`, `number`, `integer`, `boolean`, `array`, `object`, `null` (or, from 3.1, an array of them); `properties` is a mapping; `items` is a schema; `required` is an array of strings; `allOf`, `oneOf` and `anyOf` are arrays of schemas.
5. `requestBody` and each response are mappings; `content` is a mapping of media types to mappings.
6. Every reached `$ref` is a string, and its document and pointer exist.
7. A spec with no operation is refused: "No operation to generate".

### R5. What is generated

1. One feature file per tag, named by the tag's name in lower case with each run of characters other than `a`–`z` and `0`–`9` turned into one `-` (`pets.feature`). An operation with several tags goes into its first. An operation with no tag goes into the feature of its path's first segment (`/store/order` → `store.feature`), or `default.feature` for `/`. A name cut past 100 characters; a Windows reserved name (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`) gets `-feature` added. Two names that end up equal get `-2`, `-3`. Features keep the order of the spec's `tags`, then of first use.
2. A feature holds, in order:
   - a comment `# Generated by Grace from <source> on <yyyy-mm-dd>: <info.title> <info.version>`, the source as in R3.5;
   - `Feature: <tag name>`, then the tag's description, if any, as `#` comment lines;
   - `Background:` with `* url karate.get("baseUrl", "<server>")`: the first of the root `servers`, its `{variables}` replaced by their `default`, resolved against the spec's URL when relative; `http://localhost:8080` when the spec has none. A person sets `baseUrl` in `karate-config.js` to point every generated feature at another server.
   - one scenario per operation, in the order of `paths`, then `get`, `put`, `post`, `delete`, `options`, `head`, `patch`, `trace` (and from 3.2 `query`, then `additionalOperations`).
3. A scenario:
   - its `summary`, if any, as a `#` comment line above it;
   - `Scenario: <METHOD> <path>`, the path as the spec writes it (`GET /pets/{petId}`);
   - `Given path` with the path's segments, each path parameter replaced by its value (R5.5): `Given path "pets", 1`. Empty segments (`/pets/`, `//a`) are dropped. A segment that holds a parameter among other text (`/files/{name}.json`) becomes one string with the value inside (`"files", "report.json"`);
   - `And params {…}` with every required query parameter, and `And headers {…}` with every required header parameter, as one-line JSON objects; nothing when there is none. One `#` comment line names the optional ones: `# Optional params: limit, offset`. Cookie and `querystring` parameters get only a comment.
   - `And request {…}`, one-line JSON, when the operation has a request body with a JSON media type (`application/json` or any `+json`); a body of another media type gets the comment `# Request body <media type> not generated`;
   - `When method <method>`;
   - `Then status <code>`, the lowest documented `2xx` code; for `2XX`, `Then assert responseStatus >= 200 && responseStatus < 300`; with no success response, the comment `# No success response documented` and nothing after it;
   - the response match (R5.4) when that response has a JSON media type with a schema;
   - when the operation, or else the root, has `security`, a comment `# Security: <scheme names>`.
4. The response match turns the schema into Karate fuzzy markers: `string` → `"#string"`, `number` and `integer` → `"#number"`, `boolean` → `"#boolean"`, `array` → `"#array"` or, of a `string`, `number` or `boolean` item, `"#[] #string"` and so on, `object` without `properties` → `"#object"`, `object` with `properties` → an object of its properties (an optional one → `"##object"`, as Karate has no optional object literal). A property not in `required`, or nullable (`nullable: true` in 3.0, `null` among the types from 3.1), gets the optional marker `"##…"`. A schema with none of these, `oneOf` or `anyOf` gets `"#present"`, or `"#ignore"` when optional. `allOf` merges its schemas' `properties` and `required`. An object response is matched with `And match response contains deep {…}`, an array of objects with `And match each response contains deep {…}`, any other with `And match response == "<marker>"`. `contains` keeps a server's extra, undocumented fields from failing the scenario. Objects nest down to 5 levels; deeper is `"#object"`.
5. A value (path parameter, query or header parameter, request body) is, in order of preference: the parameter's or media type's `example`, the first of its `examples` (`value`), the schema's `example`, the first of its `examples` (3.1 array), `const`, `default`, the first of `enum`, then a placeholder by type: `"string"`, or by `format` `"2026-01-01"` (`date`), `"2026-01-01T00:00:00Z"` (`date-time`), `"user@example.com"` (`email`), `"00000000-0000-0000-0000-000000000000"` (`uuid`), `"https://example.com"` (`uri`); `0` for `number` and `integer` (the `minimum` when it is above 0); `true`; `[<item>]`; an object of its properties. In a request body, `readOnly` properties are left out; `allOf` merges, `oneOf` and `anyOf` take the first.
6. A schema reached again through its own `$ref` while building a value or a match (a cycle) becomes `null` in a value and `"#ignore"` in a match.

### R6. The spec's text is data

The spec may come from anyone; the features run with the person's network and machine.

1. A name written into a `Feature:` or `Scenario:` line, a comment or a file name loses every control character and line break (including `U+2028` and `U+2029`). A description longer than one line becomes several `#` lines; nothing of it is ever a Gherkin keyword line.
2. Every value written into a step is a JSON value written by `JSON.stringify`, with `U+2028` and `U+2029` escaped, never text spliced into an expression. Path segments and the server URL are JSON strings too.
3. A string value that starts with `#` (which Karate would read as a fuzzy marker or an embedded expression `#(…)`) is replaced by its placeholder (R5.5).
4. Grace never evaluates anything the spec holds: the YAML reader builds plain data (no custom tags), and nothing fetched is run.

### R7. Preview and write

1. After a successful read, the dialog's second step shows the spec's title and version, "<n> operations in <m> features", and the folder the files go into: a folder named after `info.title` (R5.1's rule; `openapi` if empty) inside the starting folder of R1.3. The starting folder is the right-clicked folder, resolved against the feature roots as the tree resolves it; for the empty tree, the first feature root, or `src/test/java` for a Maven or Gradle project that has none, or else the project folder. **Change…** opens Windows' folder dialog in main, at the nearest existing folder above that one; the folder picked is where the files go, with no folder added to it. A folder outside the project is refused: "Choose a folder inside the project".
2. Below it, one line per file in R5.1's order: the file name, and for a file that exists already, "Exists" and a **Replace** checkbox, unticked. A new file has no checkbox: it is written.
3. **Generate <n> files** writes the new files and the ticked existing ones, creating the folder if needed, as UTF-8 without a byte order mark and with `\n` line ends. It is disabled when there is nothing to write. **Back** returns to step 1; **Cancel** and Escape close the dialog and write nothing.
4. A file that did not exist at the preview but exists at the write is not replaced: it is skipped and named in the result. Grace never removes or renames a file. When another project is open by then, nothing is written: "The project changed: read the spec again".
5. When a write fails, the files already written stay, and the notice names the files written and the one that failed with its reason.
6. After the write, the dialog closes, a success notice says "Generated <n> files in <folder>" (the folder relative to the project, with `/`), and Grace refreshes the tree as Refresh does.
7. The import never starts a run: the files it writes are not handed to watch mode.

### R8. The source is kept

1. A successful write saves, in the project's settings, one entry per source: the URL or the absolute file path, `url` or `file`, the folder relative to the project, the file names written, and the date. A later import of the same source replaces its entry. At most 50 entries per project; the oldest goes.
2. A URL is saved whole, as typed (its query included), since #181 fetches it again; it holds no user name or password (R2.2). It is never written into the project whole: the generated comment shows it without its query (R3.5).
3. Nothing in this feature reads the entries; #181 will.

### R9. Docs

1. The README's features list says Grace generates feature files from an OpenAPI 3 spec, by URL or file, from a folder's right-click menu.

## Cost

- **Idle, and a person who never imports:** nothing. No timer, poll or watcher; the `yaml` reader and the generator are a separate chunk of the main bundle that loads on the first import only. The menu item is one more entry in a menu built on right-click.
- **During an import:** one fetch or file read per document, at most 100 documents of 10 MB. Parsing and generating a typical spec (tens of operations) takes milliseconds on the main process; a 10 MB spec, about a second. The documents and the generated text are held until the write or the dialog closes, then dropped.
- **Start time:** unchanged (the chunk is not loaded at start).
- **Size:** `yaml` 2.9.1, about 115 KB minified (36 KB gzipped) in the main bundle's lazy chunk; Grace's resolver, checks and generator about 20 KB. The zip grows by about 50 KB.
- **During a run:** nothing.

## Design decisions already made

By the owner:

- Approach 1: `yaml` and Grace's own resolver and checks.
- What a feature holds (R5), the preview and write rules (R7), the source kept in settings (R8), the entry points (R1).

The builder's calls, open to the owner's correction:

- `contains deep` rather than `==` for the response (R5.4): OpenAPI objects allow extra properties by default, and real servers send undocumented fields; `==` would fail them.
- Parameters and headers as one JSON object (`params {…}`, `headers {…}`) rather than one `param name = value` line each: a parameter name such as `filter[name]` or `X-Rate.Limit` cannot break the step, and R6.2 holds with one rule.
- Path parameter values inline in `path` rather than `def` lines: no parameter name has to become a JavaScript identifier.
- Only required parameters are sent; optional ones are named in a comment, so a scenario sends what the spec requires and the person adds the rest.
- The files go into a folder named after the spec inside the chosen folder (R7.1), so one import's files stay together and do not mix with hand-written features.
- Limits: 100 documents, 10 MB and 30 seconds per document, 5 redirects, 20 errors shown, 50 sources kept.
- An invalid JSON that YAML would accept (a missing value, `{"a": }`) is refused (R3.1): the format is JSON when the content says so.
- Every value starting with `#` is replaced (R6.3), the spec's own examples included: Karate would evaluate `#(…)`.
- Only what `paths` reaches is resolved and checked (R3.3): a spec is not refused, and no document fetched, for parts the import never uses.
- A URL with a user name or password is refused (R2.2) rather than sent as Basic authentication: no credential is stored in settings in plain text.
- **Change…** picks the final folder (R7.1); only the default adds the spec's folder.
- Values in steps are double-quoted JSON (R6.2), one escaping rule for every value.

## Left to the builder

- Module layout: `src/main/openapi/` with `read.ts` (fetch, file, format, resolve), `check.ts` (R4), `generate.ts` (R5, R6) and `write.ts` (R7); the `yaml` import only inside them, loaded by a dynamic `import()` from `src/main.ts` or the Runner.
- The IPC calls: a folder menu (as `editorMenu`, for folder keys); read from a URL, read from a file, cancel, change folder, write. Main keeps the pending import (generated text, folder, file list) between read and write, and drops it on write, cancel, a new read, or a project change. The window sends only the tree folder key, the URL, and the file names to replace; main refuses names it did not list.
- How the write keeps watch mode from running the new files (R7.7).
- Whether `karate.get(name, default)` exists in the bundled Karate; if not, the equivalent expression that reads `baseUrl` when set.
- Whether Karate's `method` accepts `query` (R5.2); if not, the operation gets a comment line instead of a scenario.

### Tests

- Unit tests of the reader on fixtures in `test/fixtures/openapi/`: JSON and YAML of 3.0, 3.1 and 3.2; Swagger 2.0; no `openapi`; a syntax error in each format with its line; a spec split over three files; a remote spec with relative and absolute references served by a local HTTP server; a remote reference to a local file refused; a redirect to `file:` refused; a cycle; the document limit; a missing pointer with its line.
- Unit tests of the checks (R4), each with its line.
- Unit tests of the generator: the Petstore fixture's features as expected text; every rule of R5.4 and R5.5; R6 with a hostile spec (a description holding `Scenario:` and `* karate.exec(…)`, an example `#(java.lang.System.exit(0))`, a name with a line break and `U+2028`).
- A unit test that runs the generated Petstore features with the bundled Karate against a local HTTP server that follows the spec: they pass. This settles R5.4's markers and the `karate.get` question.
- Unit tests of the write: new files, an existing file skipped and replaced, a file that appeared since the preview, a folder outside the project refused, the source entry saved.
- Renderer unit tests of the dialog's two steps and the empty tree's button.
- Two e2e tests on the packaged app. A local file: an empty Maven project; **Import OpenAPI…**, the Petstore file, the preview, **Generate 2 files**; the tree shows the new features; a second import of the same file shows both as "Exists", unticked. A URL: the test serves a spec split over two documents, with one redirect, from a local HTTP server; the import through Electron's `net.fetch` writes the features.

## Acceptance criteria

```gherkin
Feature: Import an OpenAPI spec to feature files

  Background:
    Given a project open in Grace
    And "petstore.yaml", an OpenAPI 3.0.3 spec titled "Petstore" with tags "pets" and "store"

  Scenario: Import a local file from a folder's menu
    When the person right-clicks the folder "api" and picks "Import OpenAPI here…"
    And chooses the file "petstore.yaml"
    Then the dialog shows "Petstore 1.0.0" and "3 operations in 2 features"
    And the folder "api/petstore/"
    And the files "pets.feature" and "store.feature", neither marked "Exists"
    When the person clicks "Generate 2 files"
    Then "api/petstore/pets.feature" and "api/petstore/store.feature" exist
    And the notice says "Generated 2 files in api/petstore"
    And the tree shows both features
    And no run starts

  Scenario: Import from the empty tree
    Given a Maven project with no "src/test" folder
    When the person clicks "Import OpenAPI…" on the empty tree and imports "petstore.yaml"
    Then the files are written in "src/test/java/petstore/"

  Scenario: Import from a URL
    Given "https://api.example.test/openapi.json" serves the spec as JSON, with a "$ref" to "schemas/pet.json"
    When the person enters that URL and clicks "Read"
    Then Grace fetches both documents
    And the preview lists the features

  Scenario: An existing file is not replaced unless ticked
    Given "api/petstore/pets.feature" exists with the person's edits
    When the person imports "petstore.yaml" into "api"
    Then "pets.feature" is marked "Exists" with "Replace" unticked
    And the button reads "Generate 1 file"
    When the person clicks it
    Then "pets.feature" keeps the person's edits
    And "store.feature" is written

  Scenario: Replace an existing file
    Given "api/petstore/pets.feature" exists
    When the person ticks "Replace" for "pets.feature" and clicks "Generate 2 files"
    Then "pets.feature" holds the generated text

  Scenario: Swagger 2.0 is refused
    Given "old.json" with "swagger": "2.0"
    When the person chooses "old.json"
    Then the dialog says "Swagger 2.0 is not supported: Grace imports OpenAPI 3.0 to 3.2"
    And nothing is written

  Scenario: An invalid spec is refused with its file and line
    Given "petstore.yaml" whose "paths./pets.get.parameters[0]" has no "in", at line 14
    When the person chooses "petstore.yaml"
    Then the dialog lists "petstore.yaml:14: parameter limit has no in"
    And no preview is shown
    And nothing is written

  Scenario: A YAML syntax error is refused with its line
    Given "broken.yaml" with a bad indentation at line 7
    When the person chooses "broken.yaml"
    Then the dialog lists an error at "broken.yaml:7"

  Scenario: A remote spec may not read a local file
    Given "https://api.example.test/openapi.yaml" has a "$ref" to "file:///C:/Users/me/secret.yaml"
    When the person reads that URL
    Then the dialog lists "a remote spec may not refer to a local file"
    And Grace reads no local file

  Scenario: A URL that is not http or https
    When the person enters "file:///C:/specs/petstore.yaml" and clicks "Read"
    Then the field says "Enter an http or https URL"
    And nothing is fetched

  Scenario: The spec's text never becomes a step
    Given a spec whose tag description holds the lines "Scenario: x" and "* karate.exec('calc')"
    And whose example for "name" is "#(karate.exec('calc'))"
    When the person imports it
    Then the description is written as "#" comment lines
    And the request holds the placeholder "string" for "name"

  Scenario: The generated scenarios pass against the API
    Given a server that follows "petstore.yaml"
    And "baseUrl" set to that server in "karate-config.js"
    When the person runs the generated features
    Then every scenario passes

  Scenario: The source is kept
    When the person imports "https://api.example.test/openapi.json" into "api"
    Then the project's settings keep the URL, "url", "api/petstore", the file names and the date
    And the generated features' first comment names "https://api.example.test/openapi.json" without its query

  Scenario: Nothing loads while idle
    Given Grace started and no import was made
    Then the OpenAPI reader is not loaded
```
