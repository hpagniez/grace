# Environment Switcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-project Karate environment (`-e`) picked from a combo box, and per-environment run properties (plain or encrypted secret) passed to Java through an argument file, with secrets masked in everything the app shows.

**Architecture:** Pure rules (names, validation, merging, suggestions) live in `src/shared/env.ts`, used by main and the window. Main owns the environment and the properties: `settings.ts` stores them in the per-project entry, `secrets.ts` wraps Electron `safeStorage` behind a seam, `argfile.ts` writes the `java @file`, `mask.ts` hides secret values, and the `Runner` ties them into discovery and runs. The window gets an `environment` event with a view that never carries a secret value, and sends changes over two new IPC calls.

**Tech Stack:** Electron 45 (`safeStorage`), Solid 2 RC, Tailwind v4, TypeScript, Vitest 5, Playwright 1.64, bundled Temurin 21 + Karate 2.1.4.

**Spec:** [docs/specs/2026-10-25-environment-switcher.md](../specs/2026-10-25-environment-switcher.md) (R1–R6) and [ADR 0008](../decisions/0008-secret-run-properties-and-java-argument-file.md). Builds on the walking-skeleton and tag-filter specs.

## Global Constraints

- Branch `feature/env-switcher`, cut from `feature/env-switcher-spec` (spec commit `898e94e`). Conventional Commits; AGENTS.md rules, including the Solid 2 rules.
- No new dependencies. Secrets only through Electron `safeStorage`.
- Environment name: `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`. Property name: `^[A-Za-z_][A-Za-z0-9_.-]{0,199}$`. Value: at most 10,000 characters, no line breaks.
- **Ruling (spec "Left to the builder", code page):** values must be printable ASCII (U+0020–U+007E). Anything else is rejected with "This value cannot be passed to Java exactly." Java reads the argument file in the Windows ANSI code page and Node has no encoder for it; ASCII is the same in every code page. Task 1 records this in the spec.
- Reserved property names (case-insensitive): Karate's own `karate.*` keys (Task 1 scans the jar; at least `karate.env`, `karate.options`, `karate.config.dir`, `karate.output.dir`, `karate.debug.port`); prefixes `java.`, `jdk.`, `sun.`; `user.dir`, `user.home`, `file.encoding`, `native.encoding`, `stdout.encoding`, `stderr.encoding`, `line.separator`, `path.separator`.
- A secret value never crosses IPC to the window, never goes into `settings.json` in plain text, never appears as a command-line argument. The only plain copy on disk is the argument file during a run.
- Discovery dry runs get `-e` but never properties. Runs get both. The app never passes `-Dkarate.env`.
- `childEnv` also strips `KARATE_ENV` and `KARATE_CONFIG_DIR`.
- Never run `npm test` and `npm run e2e` at the same time.
- Exact copy:
  - hint `No karate-config-<name>.js found. Fine if karate-config.js handles it.`
  - row error `This value cannot be passed to Java exactly.`
  - secret row `Can't be read, enter it again`
  - notice `Secret <name> (<scope>) can't be read. Replace it in Properties.` where scope is `All environments` or the environment name
  - checkbox note `Secret storage is not available on this system`
  - short-secret hint `Too short to hide in the console`
  - dialog note `Secrets are hidden in this app, except inside encoded text such as a Basic auth header. Karate's HTML report and event file can still contain them unless the project sets configure logging mask.`

## Review Focus

1. **A secret containing regex or replacement metacharacters** (`$&`, `.*`, `\`): masking must replace it literally. Test in Task 3 (`masks values with regex metacharacters literally`).
2. **A crash during a run leaves `run-args.txt` with plain secrets**: the next app start deletes it. Test in Task 4 (`removeLeftoverArgFiles deletes every run-args.txt`).
3. **A per-project entry written by the tag-filter version** (filter fields only): it loads as no environment and no properties. Saving the filter keeps env and properties, and the other way round. Tests in Task 2.
4. **A secret row renamed in the dialog:** "keep" cannot follow a rename. The save is refused until the value is entered again, and the old ciphertext is never copied under the new name. Test in Task 1 (`keep needs a stored secret of the same name`) and Task 7.
5. **A forged IPC payload** (an environment `../x`, a 1 MB value, a non-array list, `__proto__` as an environment key): main rejects it with "Invalid request" and changes nothing. Tests in Task 1 (`parsePropertiesEdit rejects bad shapes`) and Task 6/7.

---

### Task 1: Shared environment and property rules

**Files:**
- Create: `src/shared/env.ts`, `src/shared/env.test.ts`
- Modify: `docs/specs/2026-10-25-environment-switcher.md` (the R3 reserved list from the jar scan; the ASCII ruling in the "Resolved" line)

**Interfaces:**
- Produces (no Node imports):
  - `ENV_NAME`, `PROPERTY_NAME` (the regexes above), `MAX_VALUE_LENGTH = 10_000`, `RESERVED_KARATE_KEYS: readonly string[]`
  - `validEnvName(name: string): boolean`
  - `propertyNameError(name: string): string | null`: `null`, `"Invalid name"` or `"Reserved name"`
  - `propertyValueError(value: string): string | null`: `null`, `"Too long"`, `"No line breaks"` or the exact "cannot be passed" copy for non-printable-ASCII characters
  - `interface PlainRow { name: string; value: string }`
  - `interface SecretRowView { name: string; secret: true; state: "set" | "unreadable" }`
  - `type RowView = PlainRow | SecretRowView`
  - `interface PropertyListsView { shared: RowView[]; byEnv: Record<string, RowView[]> }`
  - `interface EnvView { env: string | null; suggestions: string[]; overlays: string[]; properties: PropertyListsView; secretsAvailable: boolean }`
  - `type RowEdit = { name: string; value: string; secret: boolean } | { name: string; secret: true; keep: true }`
  - `interface PropertiesEdit { shared: RowEdit[]; byEnv: Record<string, RowEdit[]> }`
  - `parsePropertiesEdit(x: unknown): PropertiesEdit | null`: shape only. At most 500 rows per list and 100 environments. Environment keys must be `validEnvName`, own enumerable keys only (so `__proto__` is rejected). Strings are capped at the limits above.
  - `editErrors(e: PropertiesEdit, storedSecrets: (scope: string | null, name: string) => boolean): string[]`: every rule from R3 as `"<scope>: <name>: <reason>"`. Includes duplicate names within a list, and `keep` without a stored secret of the same name in the same scope. Scope `null` is "All environments".
  - `effectiveRows<T extends { name: string }>(shared: T[], byEnv: Record<string, T[]>, env: string | null): T[]`: R4. Environment rows win by name, and order is shared first, then env-only.
  - `envTagValues(tags: string[]): string[]`: from tags written without `@`, e.g. `env=dev,qa` and `envnot=prod`, returns `dev`, `qa`, `prod`. Values failing `validEnvName` are dropped.
  - `sortedUnique(lists: string[][]): string[]`

- [ ] **Step 1: Scan the jar for Karate's own keys.** For example, unzip `vendor/karate.jar` to a scratch folder and search the class files for `karate\.[a-z][a-z.]*` string constants used with `System.getProperty`. Put the confirmed keys in `RESERVED_KARATE_KEYS`, and list them in spec R3. Also add the ASCII ruling to the spec's "Resolved" line.
- [ ] **Step 2: Write the failing tests** in `src/shared/env.test.ts`:
  - `env names` accepts `qa`, `prod-2`, `eu.west_1`, and rejects ``, `../x`, `a b`, a 65-character name, and `-x`
  - `reserved names in any case`: `karate.env`, `KARATE.OPTIONS`, `java.home`, `Sun.x` and `user.dir` are reserved; `karate.mything`, `timeout` and `api.token` are allowed
  - `value rules`: too long, `a\nb`, `é` and `\t` are rejected with the exact copy; `x y "q" C:\Users\me %PATH%` is accepted
  - `effective rows, environment wins`: shared `[timeout=5000, a=1]` plus qa `[timeout=9000, b=2]` for `qa` gives `[timeout=9000, a=1, b=2]` (shared order first); for `null` it gives shared only; for an environment without a list it gives shared only
  - `envTagValues splits values`
  - `parsePropertiesEdit rejects bad shapes`: non-object, list not an array, row missing a name, a 10,001-character value, `byEnv` key `../x`, an own `__proto__` key (built with `JSON.parse`), and 501 rows
  - `editErrors reports duplicates and reserved names`
  - `keep needs a stored secret of the same name`
- [ ] **Step 3: Run** `npx vitest run src/shared/env.test.ts`. Expected: FAIL, module not found.
- [ ] **Step 4: Implement** `src/shared/env.ts`.
- [ ] **Step 5: Run** `npx vitest run src/shared/env.test.ts`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 6: Commit** `feat(runner): add environment and run property rules`

### Task 2: Per-project environment settings

**Files:**
- Modify: `src/main/settings.ts`, `src/main/settings.test.ts`

**Interfaces:**
- Consumes: `validEnvName`, `propertyNameError` (Task 1)
- Produces:
  - `type StoredRow = { name: string; value: string } | { name: string; secret: string }` (secret holds base64 ciphertext)
  - `interface StoredLists { shared: StoredRow[]; byEnv: Record<string, StoredRow[]> }`
  - `interface EnvSettings { env: string | null; properties: StoredLists }`
  - `emptyEnvSettings(): EnvSettings` returns a fresh object each call
  - `loadProjectEnv(dataDir: string, root: string): EnvSettings`: never throws. A malformed `env` reads as `null`. Malformed rows and lists are dropped one by one (a row needs a valid name and exactly one of a string `value` or `secret`).
  - `saveProjectEnv(dataDir: string, root: string, s: EnvSettings): void`: writes `env` (omitted when null) and `properties` into `projects[root]` and keeps the entry's filter fields.
  - `saveProjectFilter` changes to merge `include`, `exclude` and `hideNonMatching` into the existing entry instead of replacing it.

- [ ] **Step 1: Write the failing tests:**
  - `env settings round-trip` (with plain and secret rows)
  - `saving the filter keeps env and properties`
  - `saving env keeps the filter`
  - `an entry from the tag-filter version loads as no environment`
  - `malformed rows are dropped, good rows kept`: a row with both `value` and `secret`, a reserved name, a non-string value, and `byEnv` holding a non-array
  - `env null is omitted from the file`
- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement.** Reuse the existing private `read`/`update` helpers.
- [ ] **Step 4: Run** `npx vitest run src/main/settings.test.ts` and `npm run typecheck`. Expected: PASS, and the existing filter tests are unchanged.
- [ ] **Step 5: Commit** `feat(runner): save the environment and run properties per project`

### Task 3: Secret storage seam and masking

**Files:**
- Create: `src/main/secrets.ts`, `src/main/mask.ts`, `src/main/mask.test.ts`, `src/main/secrets.test.ts`

**Interfaces:**
- Produces:
  - `interface SecretBox { available(): boolean; encrypt(plain: string): string; decrypt(cipher: string): string | null }`. `encrypt` returns base64; `decrypt` returns `null` on any failure and never throws.
  - `safeStorageBox(): SecretBox`: imports `safeStorage` from `electron`. Usable only after `app.whenReady()`.
  - `fakeSecretBox(available = true): SecretBox`, exported for tests only from `src/main/secrets.ts`. A reversible non-identity encoding, so tests can tell ciphertext from plain text, and corrupt input decrypts to `null`.
  - `secretForms(value: string): string[]`: the value, `encodeURIComponent(value)` and `JSON.stringify(value).slice(1, -1)`, de-duplicated and non-empty.
  - `makeMasker(values: string[]): (text: string) => string`: replaces every form of every value with `****`, longest form first, literally (split/join, not a regex built from the value). With no values it returns its input unchanged.

- [ ] **Step 1: Write the failing tests:**
  - `masks every occurrence`, including two on one line
  - `masks URL-encoded and JSON-escaped forms` (value `a b"c` matched as `a%20b%22c` and `a b\"c`)
  - `masks values with regex metacharacters literally` (`$&`, `.*`, `a\b`)
  - `longest first`: values `abc` and `abcdef` mask `xabcdefx` to `x****x`
  - `no values, no change`
  - `fake box round-trips and rejects corrupt input`
  - `decrypt never throws`
- [ ] **Step 2: Run** `npx vitest run src/main/mask.test.ts src/main/secrets.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement both files.** `safeStorageBox` is a thin wrapper around `isEncryptionAvailable`, `encryptString` and `decryptString` with base64.
- [ ] **Step 4: Run** the same tests and `npm run typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): add secret storage seam and console masking`

### Task 4: Argument file, `-e` and child environment

**Files:**
- Create: `src/main/karate/argfile.ts`, `src/main/karate/argfile.test.ts`
- Modify: `src/main/karate/command.ts`, `src/main/karate/command.test.ts`, `src/main/karate/process.test.ts` (new `buildArgs` fields)

**Interfaces:**
- Consumes: `PlainRow` (Task 1)
- Produces:
  - `argFileText(rows: PlainRow[]): string`: one line per row, `"-D<name>=<value>"`. Inside the quotes, `\` becomes `\\` and `"` becomes `\"`. Lines are joined with `\r\n`, with a trailing `\r\n`.
  - `writeArgFile(file: string, rows: PlainRow[]): void`: writes `argFileText` as `latin1` (all values are ASCII, per Task 1).
  - `removeLeftoverArgFiles(dataDir: string): void`: deletes every `<dataDir>/projects/*/run-args.txt` and never throws.
  - `buildArgs` gains `env: string | null` and `argFile: string | null`. `argFile` adds `@<argFile>` as the first argument. `env` adds `-e <env>` right after `-f html,karate:jsonl`.
  - `childEnv` also strips `KARATE_ENV` and `KARATE_CONFIG_DIR`, in any case.

- [ ] **Step 1: Write the failing tests:**
  - `argFileText escapes backslashes and quotes`: `a` = `x y "q" C:\Users\me %P%` gives the exact line `"-Da=x y \"q\" C:\\Users\\me %P%"`
  - `buildArgs puts the argument file first and -e after -f`
  - `buildArgs without env or argFile is unchanged`
  - `childEnv strips KARATE_ENV and karate_config_dir`
  - `removeLeftoverArgFiles deletes every run-args.txt`
  - **Java round-trip (Node test, bundled Java):** write the argument file for `a` = `x y "q" C:\Users\me %P% $x 'z'` and `b` = `plain`. Run a real (not dry) run of a one-scenario scratch feature that prints `karate.properties['a']`, and assert the printed value equals the input exactly. Pattern: `dryRunTree` in `src/main/runner.test.ts`, spawning through `buildArgs` and `spawnKarate`.
- [ ] **Step 2: Run** `npx vitest run src/main/karate`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement**, and update the `buildArgs` callers (`runner.ts`, tests) with `env: null, argFile: null` so behaviour is unchanged until Task 6.
- [ ] **Step 4: Run** `npx vitest run src/main/karate` and `npm run typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): pass run properties in a java argument file and strip KARATE_ENV`

### Task 5: Environment suggestions from the project

**Files:**
- Create: `src/main/karate/environments.ts`, `src/main/karate/environments.test.ts`, `test/fixtures/projects/env/` (the fixture described in the spec's Acceptance criteria, plus a `slow` scenario of about 3 s for the command-line check; every scenario passes in every environment)
- Modify: `test/fixtures/copy.ts` (name union adds `"env"`)

**Interfaces:**
- Consumes: `validEnvName`, `envTagValues` (Task 1)
- Produces:
  - `overlayEnvs(dirs: string[]): string[]`: the `<env>` of every `karate-config-<env>.js` directly inside each directory (not recursive), filtered with `validEnvName`, sorted and unique. The caller passes the project folder plus the classpath roots.
  - `featureTagEnvs(featureRoots: string[]): string[]`: reads every `*.feature` file under the roots, recursively, skipping `node_modules`, `target`, `build` and `.git`. It collects `@env=…`/`@envnot=…` tag values through `envTagValues`, sorted and unique, and never throws on unreadable files.

The env fixture: `karate-config.js` returns `{ env: karate.env }`; `karate-config-qa.js` returns `{}`. `tests/env.feature` has:
- `always`: prints `'karate.env =', karate.env`, `'timeout =', karate.properties['timeout']` and `'api.token =', karate.properties['api.token']`
- `qa only`, tagged `@env=qa`
- `not prod`, tagged `@envnot=prod`
- `slow`: `* java.lang.Thread.sleep(3000)`

- [ ] **Step 1: Write the failing tests:**
  - `overlay files in each layout`: standalone root, and Maven with `src/test/java` and `src/test/resources`
  - `ignores karate-config.js and bad names`
  - `tag values from feature files`: the env fixture gives `prod`, `qa`
  - `skips target and node_modules`
- [ ] **Step 2: Run** `npx vitest run src/main/karate/environments.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement**, and add the fixture.
- [ ] **Step 4: Run** the same tests. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): suggest environments from config overlays and env tags`

### Task 6: Runner owns the environment; IPC

**Files:**
- Modify: `src/main/runner.ts`, `src/main/runner.test.ts`, `src/shared/events.ts`, `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`, `src/renderer/Tree.test.tsx` (fake Api), `src/renderer/FilterBar.test.tsx` (fake Api)

**Interfaces:**
- Consumes: Tasks 1, 2, 3, 4 and 5
- Produces:
  - `AppEvent` gains `{ type: "environment"; view: EnvView }`. `runStarted` gains `env: string | null`.
  - `CH.env = "env:set"`; `Api.setEnv(name: string | null): Promise<void>`
  - The Runner constructor option `secrets: SecretBox`. `main.ts` passes `safeStorageBox()`; tests pass `fakeSecretBox()`.
  - `Runner.setEnv(name: string | null): Promise<void>`:
    - runs guarded (busy gives the busy notice);
    - an invalid name gives the notice "Invalid environment name";
    - the same environment is a no-op;
    - otherwise it re-discovers with the new environment.
    - On success, including exit 3, it saves the environment, sets `this.report = null` and emits `environment`. On cancel it restores the previous environment, tree and filter and saves nothing (spec R1).
  - `open` loads `loadProjectEnv` together with the filter, and discovery uses its environment.
  - Every discovery dry run passes `env`, and never `argFile`.
  - `replay()` order: project, filter, environment, busy.
  - The view: suggestions are `sortedUnique([overlays, tagEnvs, Object.keys(byEnv), current])`. `overlays` and the tag values are computed at each discovery and cached. Each secret row's state comes from a trial `decrypt` (never its value). `secretsAvailable` is `secrets.available()`.
  - `main.ts`: the `CH.env` handler accepts `null` or a string of at most 64 characters, and anything else gives "Invalid request". After `killOrphan` it calls `removeLeftoverArgFiles(dataDir)`.

- [ ] **Step 1: Write the failing tests** in `src/main/runner.test.ts`, under `describe("Runner environment")`, on the env fixture:
  - `suggestions from overlays and tags`: the view's suggestions are `["prod", "qa"]`, `overlays` is `["qa"]`, and `env` is `null`
  - `setEnv rediscovers`: `qa only` is absent, then present after `setEnv("qa")`; `setEnv("prod")` removes `qa only` and `not prod`
  - `a run uses -e`: with `qa`, running `always` puts `karate.env = qa` in the console lines and `runStarted.env === "qa"`
  - `setEnv saves and open restores`
  - `cancel during setEnv restores the previous environment`
  - `invalid env name is refused`
  - `setEnv while busy gets the busy notice`
  - `KARATE_ENV in the parent environment is ignored`: set `process.env.KARATE_ENV = "prod"` around the test, run `always` with no environment, and expect `karate.env = null`
- [ ] **Step 2: Run** `npx vitest run src/main/runner.test.ts -t "Runner environment"`. Expected: FAIL.
- [ ] **Step 3: Implement** the Runner, event, channel, preload and `main.ts` changes. Add `setEnv`, `saveProperties` and `setFilter` stubs to every fake Api.
- [ ] **Step 4: Run** `npx vitest run src/main` and `npm run typecheck`. Expected: PASS, and the existing Runner tests are unchanged except for the replay order assertion.
- [ ] **Step 5: Commit** `feat(runner): own the project environment and pass it to discovery and runs`

### Task 7: Run properties and secrets in runs; IPC

**Files:**
- Modify: `src/main/runner.ts`, `src/main/runner.test.ts`, `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`, fake Apis

**Interfaces:**
- Consumes: Tasks 1, 2, 3, 4 and 6
- Produces:
  - `CH.properties = "properties:save"`; `Api.saveProperties(edit: PropertiesEdit): Promise<void>`
  - `Runner.saveProperties(edit: PropertiesEdit): void`. While busy it gives the busy notice. If `editErrors` finds anything, it gives the notice `Properties not saved: <first error>` and changes nothing. A new secret with `secrets.available()` false gives the notice `Secret storage is not available on this system`. Otherwise it builds `StoredLists`: plain rows as `value`, new secrets as `encrypt(value)`, and kept secrets copied from the same scope and name. Then it calls `saveProjectEnv` and emits `environment`. A save failure becomes a notice, like `storeFilter`.
  - Run flow additions in `runGuarded`, after the existing checks and before spawning:
    1. `rows = effectiveRows(shared, byEnv, env)`
    2. Every secret row is decrypted. The first `null`, or any secret while `secrets.available()` is false, gives the notice `Secret <name> (<scope>) can't be read. Replace it in Properties.` and nothing runs.
    3. If there are rows, `writeArgFile(<dataDir>/projects/<hash>/run-args.txt, rows)` and spawn with `argFile`.
    4. `mask = makeMasker(secret values)` wraps everything the run emits: console lines, `scenarios` updates' `error`, and `steps` logs.
    5. The argument file is deleted in a `finally` covering spawn failure, the pid-file failure path, cancel and normal end.
  - `main.ts`: the `CH.properties` handler validates with `parsePropertiesEdit`, and `null` gives "Invalid request".

- [ ] **Step 1: Write the failing tests** under `describe("Runner properties")`, on the env fixture with `fakeSecretBox()`:
  - `shared and env properties, env wins`: shared `timeout=5000` and qa `timeout=9000` give `timeout = 9000` with qa and `timeout = 5000` with none
  - `a secret reaches the run and is masked`: the qa secret `api.token=s3cret-token-123` gives the console line `api.token = ****`; no emitted event, serialised, contains `s3cret-token-123`; the environment view row is `{ name: "api.token", secret: true, state: "set" }`
  - `settings never hold the plain secret`
  - `the secret is not on the command line`: during the `slow` scenario, read the Java process's `CommandLine` (PowerShell `Get-CimInstance Win32_Process`). It contains `@` and `run-args.txt`, and does not contain the secret.
  - `no argument file is left after a run, a cancel, or a failed spawn`
  - `an unreadable secret blocks the run`: corrupt the stored ciphertext in `settings.json`, re-open, and run. Expect the exact notice, no `runStarted`, and the row state `unreadable`.
  - `keep copies the stored ciphertext, a rename needs a new value`
  - `reserved names and bad values are refused`: no save, notice text
  - `no properties, no @ argument`
- [ ] **Step 2: Run** `npx vitest run src/main/runner.test.ts -t "Runner properties"`. Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `npx vitest run src/main` and `npm run typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): pass run properties and encrypted secrets to runs`

### Task 8: Environment combo, Properties dialog, progress

**Files:**
- Create: `src/renderer/EnvPicker.tsx`, `src/renderer/PropertiesDialog.tsx`, `src/renderer/EnvPicker.test.tsx`, `src/renderer/PropertiesDialog.test.tsx`
- Modify: `src/renderer/reduce.ts`, `src/renderer/store.test.tsx` or `src/renderer/reduce.test.ts`, `src/renderer/Toolbar.tsx`, `src/renderer/Progress.tsx`

**Interfaces:**
- Consumes: `EnvView`, `RowView`, `PropertiesEdit`, `validEnvName`, `propertyNameError`, `propertyValueError` (Task 1); the `environment` event and `runStarted.env` (Task 6); `Api.setEnv` and `Api.saveProperties` (Tasks 6 and 7)
- Produces (test ids used by Task 9):
  - **Reducer:**
    - `AppState.env: EnvView | null`, and `RunView.env: string | null`, set from `runStarted`.
    - An `environment` event for the same project whose `view.env` differs from the previous view's env clears statuses, results, steps, `selected` and the run view, the same as a project switch.
    - A `project` event for another root sets `env` to `null`.
  - **Env combo:**
    - Ids: `env-input` (an `<input list>` with a `<datalist>` holding the suggestions and `(none)`), `env-hint`, `env-error`.
    - A pick, Enter or blur commits through `api.setEnv`. The value `(none)` or an empty box means `null`.
    - An invalid name shows `env-error` and calls nothing.
    - The hint shows the exact copy when the env is set and not in `view.overlays`.
    - Disabled while busy or running.
  - **Properties dialog:**
    - `properties-button` opens a native `<dialog data-testid="properties-dialog">`, and is disabled while busy or running.
    - Scope list: items `scope-item` with `data-scope="" | <env>`, plus `add-env` and `delete-env`.
    - Rows `property-row` with `prop-name`, `prop-value`, `prop-secret` (checkbox), `prop-replace`, `prop-delete` and `row-error`; plus `add-property`, `secrets-note`, `save-properties` and `cancel-properties`.
    - A saved secret shows dots and Replace and never a value. Unticking Secret on a saved secret clears the value.
    - Save is disabled while any row has an error (computed with the Task 1 helpers). It calls `api.saveProperties` with a plain object, not a store proxy (see `plain()` in `FilterBar.tsx`).
  - **Progress:** `run-env` shows the env of the run in progress or last run, and is empty for none.

- [ ] **Step 1: Write the failing tests:**
  - **Reducer:** `env change clears results`; `same env keeps them`.
  - **EnvPicker:** `suggestions and none`; `Enter commits`; `invalid name shows error and does not call setEnv`; `hint for a name without overlay`; `disabled while busy`.
  - **PropertiesDialog:**
    - `lists scopes and rows`
    - `secret shows dots and Replace, never a value`
    - `reserved name blocks Save`
    - `non-ASCII value shows the exact error`
    - `save sends keep for an untouched secret and the new value after Replace`
    - `payload is structured-cloneable`
    - `secrets note shown when a secret exists`
    - `short secret shows the hint`
    - `secret checkbox disabled when secrets are unavailable`
  - **Progress:** `run env shown`.
- [ ] **Step 2: Run** `npx vitest run src/renderer`. Expected: FAIL.
- [ ] **Step 3: Implement.** Use the existing Tailwind button styles from `Toolbar.tsx`. Follow the Solid 2 rules in AGENTS.md.
- [ ] **Step 4: Run** `npx vitest run src/renderer src/shared`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add environment combo and run properties dialog`

### Task 9: End-to-end scenarios and ship

**Files:**
- Create: `e2e/env.spec.ts`
- Modify: `e2e/helpers.ts` (`project()` accepts `"env"`; `launch()` accepts extra environment variables), `docs/features.md` (F08 `done`)

- [ ] **Step 1: Write one Playwright test per Gherkin scenario** in the spec, using the same names and the Task 8 test ids.
  - "Settings survive a restart" closes the app before relaunching with the same userData, and waits for the echoed state first, like `e2e/tags.spec.ts`.
  - "An unreadable secret blocks runs" writes a corrupted ciphertext into that userData's `settings.json` before launch.
  - "The machine's KARATE_ENV is ignored" launches with `KARATE_ENV=prod`.
  - "A secret stays off the command line" reads the Java `CommandLine` during the `slow` scenario.
- [ ] **Step 2: Run** `npm run e2e`. If packaging hits `EBUSY` on `out\`, package with Forge's `api.package` into a scratch `outDir`, then run `npx playwright test`. Expected: all tests pass, including the 23 existing ones.
- [ ] **Step 3: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 4: Update** the F08 row in `docs/features.md` to `done`.
- [ ] **Step 5: Commit** `test(e2e): cover the environment switcher` and `docs: mark f08 done`
