---
status: built
issue: 230
summary: "A run property row can be an environment variable of the Karate process, and Import from file turns a .properties file into secret rows"
---

# Environment variable run properties and their import: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A run property row can be an environment variable of the Karate process, and **Import from file…** turns a `.properties` file into secret rows of one environment.

**Architecture:** The rows keep their storage, encryption and masking; a `envVar` flag picks the transport. Main sets the flagged rows in the spawn's environment and writes only the others to `run-args.txt`. Main reads and parses the picked file; the dialog maps, checks and adds the keys to its draft.

**Tech Stack:** Electron main (Node), SolidJS 2 window, Vitest (Node for `*.test.ts`, jsdom for `*.test.tsx`), Playwright e2e.

**Spec:** [docs/specs/2026-11-04-env-var-properties.md](../specs/2026-11-04-env-var-properties.md)

## Global Constraints

- Environment variable name: `^[A-Za-z_][A-Za-z0-9_]{0,199}$`, else "Invalid name"; reserved, case-insensitive: `KARATE_OPTIONS`, `KARATE_ENV`, `KARATE_CONFIG_DIR`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS`, else "Reserved name". One list in `src/shared/env.ts`, also used by `childEnv`.
- Value rules unchanged for both kinds (`propertyValueError`).
- Stored row: `envVar: true` only on environment variables; absent means system property.
- Messages, exact: "<file> is too large to import", "<file> could not be read: <reason>", "<file> could not be imported: bad \uXXXX escape on line <n>", "<file> has no properties", "Letters, digits and _ only", "Empty value", "Same name as <key>", "At most 500 properties per list", "At most 100 environments", "Duplicate name".
- File limit: 1 MB (1,048,576 bytes). Open dialog title "Import properties"; filters "Properties files" (`properties`) and "All files" (`*`).
- No value in a notice, a message, a log or a skip line. Grace never writes the picked file and never keeps its path.
- Solid 2 rules and UI conventions of `AGENTS.md`: tokens only, `buttons.ts`/`ui.ts` classes, `FieldMessage`, no `title=`.

## Review Focus

- A file saved by Notepad with a UTF-8 byte order mark: the first key is read without it (Task 4 test).
- An imported name that matches an existing variable only in letter case: it is a Replace, not a second row (Task 6 test).
- A shared variable and an environment's variable differing only in case: the environment's reaches Karate (Task 3 test).
- A continuation line ending the file with a lone `\`: the key keeps its value without the `\` (Task 4 test).
- Reopening the dialog after an import was left open: the import step is gone with the draft (Task 6 test).

---

### Task 1: Shared rules for the kind

**Files:**
- Modify: `src/shared/env.ts`, `src/main/karate/command.ts` (`STRIPPED`, `childEnv`)
- Test: `src/shared/env.test.ts`, `src/main/karate/command.test.ts`

**Interfaces:**
- Produces, in `src/shared/env.ts`:
  - `STRIPPED_ENV_VARS: readonly string[]` (upper case, the six names)
  - `envVarNameError(name: string): string | null`
  - `nameError(name: string, envVar: boolean): string | null` (the kind's rule)
  - `nameKeys(name: string, envVar: boolean): string[]`: `[name]`, plus `"\0" + name.toUpperCase()` for a variable; two rows of a list clash when their keys intersect
  - `envVarFromKey(prefix: string, key: string): string`: `prefix + key.toUpperCase().replace(/[^A-Z0-9_]/g, "_")`
  - `PREFIX = /^[A-Za-z0-9_]*$/`
  - `envVar?: boolean` on `PlainRow`, `SecretRowView` and both `RowEdit` shapes
  - `ImportEntry { key: string; value: string }`, `ImportResult = { file: string; entries: ImportEntry[] } | { error: string } | null`
- Produces, in `command.ts`: `childEnv(env: NodeJS.ProcessEnv, vars: { name: string; value: string }[] = []): NodeJS.ProcessEnv`: strips `STRIPPED_ENV_VARS` in any case, then sets each var after deleting every key equal to it ignoring case; a later var wins.

- [ ] **Step 1: Write the failing tests**
  - `env.test.ts`: `envVarNameError("APP_X")` is null; `"db.user"`, `"9A"`, `""` give "Invalid name"; `"java_tool_options"` gives "Reserved name"; `nameError("db.user", false)` is null; `envVarFromKey("APP_", "db.user")` is `"APP_DB_USER"`, `envVarFromKey("", "café-x")` is `"CAF__X"`; `parsePropertiesEdit` keeps `envVar: true`, refuses `envVar: "yes"`; `editErrors` gives `"All environments: java_tool_options: Reserved name"` for an `envVar` row, `"Duplicate name"` for variables `App_Token` then `APP_TOKEN`, none for a system property `app_token` beside the variable `APP_TOKEN`, and still `"Duplicate name"` for a property `X` beside a variable `X`.
  - `command.test.ts`: `childEnv({ PATH: "p", app_x: "m" }, [{ name: "APP_X", value: "g" }])` equals `{ PATH: "p", APP_X: "g" }`; with vars `[{APP_X:"a"}, {app_x:"b"}]` the result has only `app_x: "b"`; the existing tests still pass with the list from `env.ts`.
- [ ] **Step 2: Run** `npx vitest run src/shared/env.test.ts src/main/karate/command.test.ts`: FAIL on the missing names.
- [ ] **Step 3: Implement.** `parseRow` accepts `envVar` undefined or boolean and copies it. `editErrors` uses `nameError(row.name, row.envVar === true)` and a `seen` set of `nameKeys`. `STRIPPED` in `command.ts` goes; `childEnv` compares `k.toUpperCase()` with `STRIPPED_ENV_VARS`.
- [ ] **Step 4: Run** the same command: PASS. Run `npm run typecheck`.
- [ ] **Step 5: Commit** `feat(env): add the environment variable kind to run property rules`

### Task 2: Store the kind

**Files:**
- Modify: `src/main/settings.ts` (`StoredRow`, `parseStoredRow`)
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Consumes: `nameError` (Task 1).
- Produces: `StoredRow = ({ name: string; value: string } | { name: string; secret: string }) & { envVar?: true }`.

- [ ] **Step 1: Write the failing tests:** a saved `{ name: "APP_X", secret: "c", envVar: true }` loads with `envVar: true`; `{ name: "timeout", value: "5000" }` loads without `envVar`; `{ name: "A", value: "1", envVar: "yes" }` and `{ name: "db.user", value: "1", envVar: true }` are not loaded and `skippedProperties` names them; a save keeps the rejected row in the file.
- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts`: FAIL.
- [ ] **Step 3: Implement** in `parseStoredRow`: `envVar` must be `undefined` or `true`; name via `nameError(name, envVar === true)`; the row returned carries `envVar: true` only when set.
- [ ] **Step 4: Run** it again: PASS.
- [ ] **Step 5: Commit** `feat(settings): store the kind of a run property`

### Task 3: Environment variables reach the run

**Files:**
- Modify: `src/main/runner.ts` (`saveProperties`, `envEvent`, `spawn`, `discover`, `runBody`), `test/fixtures/projects/env/tests/env.feature`
- Test: `src/main/runner.properties.test.ts`

**Interfaces:**
- Consumes: `childEnv(env, vars)` (Task 1), `StoredRow` (Task 2).
- Produces: `spawn(outDir, dryRun, targets, env, onLines, argFile, filter, threads, vars: PlainRow[], debug = null)`; `discover` passes `[]`.

- [ ] **Step 1: Add the fixture step** after the last step of `always` in `env.feature`: `  * print 'APP_API_TOKEN =', java.lang.System.getenv('APP_API_TOKEN')`. Grep `src`, `test` and `e2e` for line numbers of `env.feature` past 6 and for step counts of `always`; adjust any.
- [ ] **Step 2: Write the failing tests** (real Karate, `fixture("env")`, `fakeSecretBox`). Add at the top of the file a pass-through `vi.mock("./karate/process", …)` that wraps `spawnKarate` in `vi.fn`, so a test reads each call's `args` and `extra.env`; the other tests are unaffected.
  - `"an environment variable reaches the run, masked, and stays out of run-args.txt"`: save `{ byEnv: { qa: [{ name: "APP_API_TOKEN", value: SECRET, secret: true, envVar: true }] } }`, run `always` on `qa`; printed has `APP_API_TOKEN = ****`, no event text has `SECRET`, and the run's `spawnKarate` call has no `@` argument.
  - `"both kinds in one run"`: `timeout` plain property and `APP_API_TOKEN` plain variable in `qa`; printed has `timeout = 9000` and `APP_API_TOKEN = v`.
  - `"the machine's variable is replaced in any case"`: `process.env.app_api_token = "machine"` for the test (restore after); shared plain variable `APP_API_TOKEN = grace`; printed `APP_API_TOKEN = grace`.
  - `"the environment's variable wins over a shared one in another case"`: shared `app_api_token = shared`, `qa` `APP_API_TOKEN = qa`; printed `APP_API_TOKEN = qa`.
  - `"the kind is saved and switching it keeps the secret"`: save a secret property `API_TOKEN`, then a `keep` row with `envVar: true`; `loadProjectEnv` shows the same ciphertext with `envVar: true`; the `environment` event's row is `{ name, secret: true, state: "set", envVar: true }`.
  - `"dry runs get no environment variables"`: shared variable `APP_API_TOKEN`; after `open` and a run, the `spawnKarate` calls whose args hold Karate's dry run flag `-D` have no `APP_API_TOKEN` in `extra.env`, and the run's call has it.
- [ ] **Step 3: Run** `npx vitest run src/main/runner.properties.test.ts`: the new tests FAIL.
- [ ] **Step 4: Implement.** `saveProperties` writes `envVar: true` on flagged rows, a `keep` row included. `envEvent` adds `envVar: true` to a flagged secret's view. `runBody` splits `props.rows` into `vars` (flagged) and the rest; `argFile` exists only when the rest is not empty and gets only the rest; `spawn` passes `{ env: childEnv(process.env, vars) }` to `spawnKarate` (default wait). Masking stays on `props.secrets`.
- [ ] **Step 5: Run** the file and `src/main/runner.env.test.ts`, `src/main/runner.rerun.test.ts`, `src/main/karate/argfile.test.ts` one at a time: PASS.
- [ ] **Step 6: Commit** `feat(runner): pass environment variable properties to the run`

### Task 4: Read a .properties file

**Files:**
- Create: `src/main/propertiesFile.ts`
- Modify: `src/shared/ipc.ts` (`CH.importProperties = "properties:import"`, `Api.importProperties(): Promise<ImportResult>`), `src/preload.ts`, `src/main.ts`
- Test: `src/main/propertiesFile.test.ts`

**Interfaces:**
- Consumes: `ImportEntry`, `ImportResult` (Task 1).
- Produces: `parseProperties(text: string): ImportEntry[]` (throws `Error("bad \\uXXXX escape on line <n>")`, `<n>` the line the property starts on); `readPropertiesFile(file: string): ImportResult` (never throws; messages from Global Constraints with `path.basename(file)`).

- [ ] **Step 1: Write the failing tests** for `parseProperties`: comments `#`/`!` and blank lines skipped; `"a=1"`, `"b : 2"`, `"c 3"`, `"d=x\\"` + `"  y"` give `d = "xy"`; `"e=\\u0041"` gives `"A"`; `"t=a\\tb"` gives a tab; `"k\\=x=1"` gives key `k=x`; a key repeated keeps its last value at its first place; a file ending in `"f=1\\"` gives `f = "1"`; a comment line ending in `\` does not continue; `"k=\\u00G1"` on line 2 throws `bad \uXXXX escape on line 2`. For `readPropertiesFile` (temp files): BOM `EF BB BF` before `db.user=u` gives key `db.user`; byte `E9` reads as `é`; a 1,048,577-byte file gives `{ error: "big.properties is too large to import" }`; a missing file gives `could not be read:`; an only-comments file gives `has no properties`; the file's bytes and mtime are unchanged after.
- [ ] **Step 2: Run** `npx vitest run src/main/propertiesFile.test.ts`: FAIL.
- [ ] **Step 3: Implement** `parseProperties` by the rules of `java.util.Properties.load` (spec R4.3): split natural lines on `\r\n|\r|\n`; strip leading `[ \t\f]`; skip blank and comment lines; while the logical line ends in an odd number of `\`, drop it and append the next natural line, stripped; key ends at the first unescaped `=`, `:` or white space; skip white space, one `=` or `:`, white space; unescape key and value. Keep entries in a `Map`. `readPropertiesFile`: `statSync` size check, `readFileSync`, drop a leading BOM, `toString("latin1")`.
- [ ] **Step 4: Wire IPC.** `main.ts`: `handle(CH.importProperties, …)` shows `dialog.showOpenDialog` (with `win` when alive, as `CH.open` does) titled "Import properties", `properties: ["openFile"]`, the two filters; a cancel returns `null`; else `readPropertiesFile(r.filePaths[0])`. `preload.ts`: `importProperties: () => ipcRenderer.invoke(CH.importProperties)`.
- [ ] **Step 5: Run** the test file and `npm run typecheck`: PASS.
- [ ] **Step 6: Commit** `feat(properties): read a .properties file to import`

### Task 5: The Env variable checkbox

**Files:**
- Modify: `src/renderer/PropertiesDialog.tsx`
- Test: `src/renderer/PropertiesDialog.test.tsx`

**Interfaces:**
- Consumes: `nameError`, `nameKeys` (Task 1).
- Produces: `Row.envVar: boolean`; checkbox `data-testid="prop-envvar"`, label "Env variable", between the value and Secret.

- [ ] **Step 1: Write the failing tests:** a view row with `envVar: true` shows the box ticked; ticking it on `db.user` shows "Invalid name" and disables Save, renaming to `DB_USER` clears it; variables `App_Token` and `APP_TOKEN` both show "Duplicate name" while a property `app_token` shows none; ticking it on a saved secret keeps the dots and Save sends `{ name, secret: true, keep: true, envVar: true }`; a new row starts unticked and Save sends `envVar: false`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/PropertiesDialog.test.tsx`: FAIL.
- [ ] **Step 3: Implement.** `rowsOf` sets `envVar: r.envVar === true`; `addRow` starts `false`; the `errors` memo uses `nameError` and counts `nameKeys`; `edit()` sends `envVar` on every row; the checkbox mirrors Secret's markup and never calls `forget`.
- [ ] **Step 4: Run** it again: PASS.
- [ ] **Step 5: Commit** `feat(ui): mark a run property as an environment variable`

### Task 6: The import step

**Files:**
- Create: `src/renderer/importPlan.ts`, `src/renderer/ImportStep.tsx`
- Modify: `src/renderer/PropertiesDialog.tsx`
- Test: `src/renderer/importPlan.test.ts`, `src/renderer/PropertiesDialog.test.tsx`

**Interfaces:**
- Consumes: `ImportEntry`, `ImportResult`, `envVarFromKey`, `nameError`, `nameKeys`, `PREFIX`, `propertyValueError`, `MAX_ROWS`, `MAX_ENVS` (Task 1); `Api.importProperties` (Task 4); `Row` (Task 5).
- Produces: `importPlan(entries: ImportEntry[], o: { envVar: boolean; prefix: string; existing: { name: string; envVar: boolean }[] }): { rows: { name: string; value: string }[]; adds: number; replaces: string[]; skips: { key: string; reason: string }[] }`. `rows` in file order; `replaces` the new names that clash (`nameKeys`) with an existing row.
- `ImportStep` props: `{ file: string; entries: ImportEntry[]; scopes: string[]; scope: string; existing: (scope: string) => { name: string; envVar: boolean }[]; canAddList: (scope: string) => boolean; onImport: (scope: string, envVar: boolean, rows: { name: string; value: string }[]) => void; onBack: () => void }`. Test ids: `import-step`, `import-scope` (a `<select>`), `import-envvar`, `import-prefix`, `import-preview`, `import-summary`, `import-skip` (one per skip), `import-confirm`, `import-back`.

- [ ] **Step 1: Write the failing `importPlan` tests:** keys `db.user=u`, `db.password=p`, `9lives=x`, `empty=` as variables, existing variable `APP_DB_USER`: with no prefix, adds 2, replaces none, skips `9lives: Invalid name` and `empty: Empty value`; with prefix `APP_`, adds 2 (`APP_DB_PASSWORD`, `APP_9LIVES`), replaces `["APP_DB_USER"]`, skips only `empty: Empty value`; an existing `App_Db_User` variable is replaced by `APP_DB_USER`; `db.user=a` + `db_user=b` skips `db_user: Same name as db.user`; as properties, `db.user` keeps its name and `9lives` is `Invalid name`; a value `"é"` skips with `propertyValueError`'s reason.
- [ ] **Step 2: Write the failing dialog tests** (`api.importProperties` mocked): Import from file… is disabled when secrets are unavailable; a `null` answer shows the list unchanged; `{ error }` shows it under the buttons; with keys, the step shows "Import secrets-dev.properties", the scope choice starting on the shown list, Environment variables ticked, the preview `db.user → APP_DB_USER` after typing `APP_`, "Replaces 1: APP_DB_USER", and no value anywhere (`document.body` text and every input's value); prefix `APP-` shows "Letters, digits and _ only" and disables Import; a list of 499 rows plus 2 new keys shows "At most 500 properties per list" and disables Import; Import puts secret variable rows in the draft (Save sends them with `secret: true, envVar: true`), replaces in place, switches to that list, and creates the chosen environment's list; Back changes nothing; closing the dialog with the step open and reopening shows the list, not the step.
- [ ] **Step 3: Run** `npx vitest run src/renderer/importPlan.test.ts src/renderer/PropertiesDialog.test.tsx`: FAIL.
- [ ] **Step 4: Implement** `importPlan`, then `ImportStep` (summary lines "Adds <n>", "Replaces <n>: <names joined by ", ">", "Skips <n>:" then `<key>: <reason>` lines; Import disabled per spec R5.4), then wire it in the dialog: an **Import from file…** button beside Add property; a `session`-checked call to `api.importProperties()`; signals for the keys and the message, both cleared by `drop()`; while keys are held, the right side shows `ImportStep` instead of the list and buttons; `onImport` calls `ensureList` for a new environment, replaces rows that clash in place (`saved: null`, `secret: true`, the chosen kind, the new name and value) and pushes the rest, then sets the scope and clears the keys.
- [ ] **Step 5: Run** both files: PASS. Run `npm run lint` and `npm run typecheck`.
- [ ] **Step 6: Commit** `feat(ui): import a .properties file as secret rows`

### Task 7: End to end

**Files:**
- Modify: `e2e/env.spec.ts`

- [ ] **Step 1: Write** `test("Import a secrets file and run with it")`: `openEnvWith("qa")`; write `secrets-qa.properties` with `api.token=${SECRET}` in a temp folder; stub `dialog.showOpenDialog` to return it; open Properties, click Import from file…, type prefix `APP_`, Import, Save; poll `saved(userData, dir).properties.byEnv.qa` to equal `[{ name: "APP_API_TOKEN", secret: expect.any(String), envVar: true }]`; settings.json text has no `SECRET`; the file's bytes are unchanged; run `always`; console has `APP_API_TOKEN = ****`; the window holds no `SECRET` (the `everything()` check of "A secret is used but never shown").
- [ ] **Step 2: Ask the parent session for the machine**, then run `npm run e2e -- e2e/env.spec.ts`: PASS. Then the full `npm run e2e` for the PR.
- [ ] **Step 3: Commit** `test(e2e): import a secrets file and run with it`
