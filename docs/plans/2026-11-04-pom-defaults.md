# Start from the project's pom: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** a control with nothing saved (environment, tag filter, thread count) starts from `karate-pom.json`, and the environment from a Maven `pom.xml`'s `karate.env`, on open and Refresh.

**Architecture:** `maven.ts` reads `karate.env` from `pom.xml` with a small XML reader. `readPom` also parses `env`, `tags` and `threads`, and `pomStart` merges both files. `settings.ts` says which controls are saved. `Runner.open` and `refresh` take the saved value or the pom's, per control.

**Tech Stack:** TypeScript, Node `fs`, Vitest (Node), Playwright `_electron`.

**Spec:** [docs/specs/2026-11-04-pom-defaults.md](../specs/2026-11-04-pom-defaults.md)

## Global Constraints

- No new dependency. Main process only; the renderer does not change.
- Runs still pass `--no-pom`, `-e`, `-t`, `-T` from the controls (`buildArgs`, `tagArgs` unchanged).
- Environment names: `validEnvName` (`src/shared/env.ts`). Threads: `validThreads` (1 to 32, `src/shared/threads.ts`). Tags: `filterable`, then `parseFilterState` (`src/shared/tags.ts`).
- Notice format: `Grace does not use these karate-pom.json settings: <items>.`, items in file order; an unused value is `<key> <JSON.stringify(value)>`, e.g. `env "x y"`, `threads 64`, `tags "anyOf('@a','@b')"`.
- Idle costs nothing: reads happen on open and Refresh only.
- Keep `detectLayout` untouched apart from what `readPom`'s new `unused` gives it (#232 also edits it).
- No backward compatibility: no migration of `settings.json`.

## Review Focus

- A `pom.xml` with a namespace or attributes on `<project>` (`<project xmlns=… xsi:schemaLocation=…>`): the reader ignores attributes, so `dev` is still found. Test in Task 1.
- A `pom.xml` with a UTF-8 BOM or Windows line ends: still read. Test in Task 1.
- A pom filter on a project whose tree has none of its tags: the filter shows empty and `settings.json` gets no filter. Test in Task 4.
- Toggling hide or dim while a pom filter is in force saves the whole filter shown (a choice). Test in Task 4.
- An open that is cancelled keeps the previous project's controls, with no pom read leaking in (the values are only assigned once `rediscover` succeeds). Covered by the existing `runner.threads.test.ts` "a cancelled open restores the count before"; keep it green.

---

### Task 1: Maven's `karate.env`

**Files:**
- Create: `src/main/karate/maven.ts`
- Test: `src/main/karate/maven.test.ts`

**Interfaces:**
- Produces: `export function mavenEnv(root: string): string | null`: spec R3.2 to R3.6. Never throws.

- [ ] **Step 1: Write the failing tests** in `maven.test.ts`. Each one writes a `pom.xml` into a temp folder (the pattern of `pom.test.ts`) and checks `mavenEnv(dir)`. A helper `surefire(entry, where = "plugins")` builds `<build><plugins><plugin><artifactId>maven-surefire-plugin</artifactId><configuration><systemPropertyVariables><karate.env>${entry}</karate.env>…`; for `"pluginManagement"`, it wraps that in `<pluginManagement>`.
  - one test per row of the spec's "Maven's karate.env" outline, with the env it gives (`null` for `(none)`);
  - `<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="…">` with a BOM and `\r\n` line ends → `"dev"`;
  - `<![CDATA[dev]]>` as the property's text → `"dev"`;
  - an empty `<karate.env/>` in Surefire's entry, with the property `dev` → `null` (the entry decides);
  - no `pom.xml` → `null`.

- [ ] **Step 2: Run** `npx vitest run src/main/karate/maven.test.ts`. Expected: FAIL, module not found.

- [ ] **Step 3: Implement `maven.ts`**. A private reader `element(xml: string): El | null`, where `El` is `{ name: string; children: El[]; text: string }`. It returns the root element, or null when tags are not well nested. Scan with one global regex whose alternatives are, in this order: a comment, CDATA (adds its text), `<?…?>` or `<!…>` (skipped), an end tag, a start or empty tag (attributes ignored), and text. The text of an element is its own text chunks joined. `mavenEnv` then reads `<root>/pom.xml` as UTF-8, drops a BOM and takes the root `project`:
  - `props`: the direct children of `project > properties`, mapping name to trimmed text;
  - the entry: the first `configuration > systemPropertyVariables > karate.env` of a `plugin` whose `artifactId` is `maven-surefire-plugin`, then `maven-failsafe-plugin`, each looked up in `build > plugins`, then `build > pluginManagement > plugins`; else `props["karate.env"]`;
  - resolving: more than one `${` gives null; one `${name}` is replaced by `props[name]` (missing gives null); a result still holding `${` gives null; then trim and check `validEnvName`.

- [ ] **Step 4: Run** the tests again. Expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): read karate.env from a Maven pom.xml`, with `Part of #228`.

### Task 2: `env`, `tags` and `threads` from `karate-pom.json`

**Files:**
- Modify: `src/main/karate/pom.ts`
- Test: `src/main/karate/pom.test.ts`, `src/main/karate/command.test.ts` (the `env.` notice test)

**Interfaces:**
- Consumes: `mavenEnv` (Task 1).
- Produces:
  - `Pom` gains `env: string | null | undefined` (undefined: the file gives none, so `pom.xml` may; null: given but unused), `filter: TagFilter | null` (null: no used entry) and `threads: number | null`. `NO_POM` has `env: undefined`, `filter: null`, `threads: null`. `unused` now names unused values as `<key> <JSON>` and leaves out `env`, `tags`, `threads` when used.
  - `export interface PomStart { env: string | null; filter: TagFilter | null; threads: number | null }`
  - `export function pomStart(root: string): PomStart`: `readPom` values, with `env` from `mavenEnv(root)` when `readPom(root).env === undefined` (a broken file counts as absent, R3.1).

- [ ] **Step 1: Write the failing tests** in `pom.test.ts`:
  - `{"env":"qa","tags":["@smoke","~@slow"],"threads":3}` → `env: "qa"`, `filter: { include: ["smoke"], exclude: ["slow"] }`, `threads: 3`, `unused: []`;
  - each row of the spec's two tag outlines, checking `filter` and `unused` (`"@a"` not in a list → `unused: ['tags "@a"']`, `filter: null`);
  - `{"env":"x y"}` → `env: null`, `unused: ['env "x y"']`; `{"env":""}` → `env: undefined`, `unused: []`; `{"threads":64}` → `['threads 64']`; `{"threads":"4"}` → `['threads "4"']`;
  - the existing file-order test, now `{"paths":…,"configDir":"conf","env":"qa","dryRun":true}` → `unused: ["dryRun"]`, `env: "qa"`;
  - the `broken` helper gains `env: undefined, filter: null, threads: null`;
  - `pomStart`: a `pom.xml` with the property `karate.env` set to `dev` and no `karate-pom.json` → `env: "dev"`; with `{"env":"qa"}` → `"qa"`; with `{"env":"x y"}` → `null`; with `karate-pom.json` `"not json"` → `"dev"`.

  In `command.test.ts`, the test with `{ configDir: "conf", env: "qa" }` now expects `pomNotice: null`.

- [ ] **Step 2: Run** `npx vitest run src/main/karate/pom.test.ts src/main/karate/command.test.ts`. Expected: FAIL.

- [ ] **Step 3: Implement** in `pom.ts`. The tag rule, per entry: split on `,` and trim each part. One part starting `~@` is an exclusion. Parts all starting `@` are the include group (first such entry only). Every tag must pass `filterable` and not be in the other list, else the entry is unused; a tag already in the same list is not added twice. At the end, if `parseFilterState({ ...filter, hideNonMatching: false })` is null, the whole `tags` is unused.

- [ ] **Step 4: Run** the tests again. Expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): take env, tags and threads from karate-pom.json`, with `Part of #228`.

### Task 3: Which controls are saved

**Files:**
- Modify: `src/main/settings.ts`
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Produces:
  - `export interface SavedControls { env: boolean; filter: boolean; threads: boolean }`
  - `export function savedControls(dataDir: string, root: string): SavedControls`: `env` when the entry owns an `env` that is null or passes `validEnvName`; `filter` when `parseFilterState(entry)` is not null; `threads` when `validThreads(entry.threads)`. All false when unreadable. Never throws.
  - `saveProjectEnv` writes `env: null` instead of deleting it (R1.3).
  - `export function saveProjectProperties(dataDir: string, root: string, properties: StoredLists): void`: writes `properties` only, keeping rejected rows as `saveProjectEnv` does today. `saveProjectEnv` reuses it in the same `updateEntry`.

- [ ] **Step 1: Write the failing tests**:
  - `savedControls` on an empty data folder → all false; after `saveProjectThreads(…, 2)` → `{ env: false, filter: false, threads: true }`; after `saveProjectFilter` with an empty filter → `filter: true`; after `saveProjectEnv` with `env: null` → `env: true`; with `{"env":"x y"}` written by hand → `env: false`; with an unreadable `settings.json` → all false;
  - rename "env null is omitted from the file" to "env null is saved as null": `entry.env` is `null`;
  - `saveProjectProperties` after `saveProjectEnv(…, { env: "qa", … })` keeps `env: "qa"` and writes the new lists; on a fresh entry it writes no `env` key.

- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts`. Expected: FAIL.

- [ ] **Step 3: Implement**.

- [ ] **Step 4: Run** the tests again. Expected: PASS.

- [ ] **Step 5: Commit** `feat(settings): tell saved controls apart and save no environment`, with `Part of #228`.

### Task 4: The runner starts from the pom

**Files:**
- Modify: `src/main/runner.ts` (`open`, `refresh`, `rediscover`, `saveProperties`)
- Create: `src/main/runner.pom.test.ts`
- Modify: `src/main/runner.setup.test.ts` (its `POM_PROJECT` holds `env: "qa"`, which now applies: drop `env` from it and expect `dryRun.` alone in the notice)

**Interfaces:**
- Consumes: `pomStart` (Task 2); `savedControls`, `saveProjectProperties` (Task 3).
- Produces: a private `start(root: string, filter: FilterState, envs: EnvSettings, threads: number): { filter: FilterState; envs: EnvSettings; threads: number }`. For each control, the value given if saved; otherwise the pom's; otherwise the default (`{ ...NO_FILTER, include: [], exclude: [] }`, `env: null`, `1`). A pom filter gets `hideNonMatching: false`.

- [ ] **Step 1: Write the failing tests** in `runner.pom.test.ts`, one per R1 scenario and the R2 "The notice names only what Grace does not use" scenario, plus Review Focus lines 3 and 4. Use `setup()`, `T` and `cleanUp` from `test/runner.ts`, with projects written to temp folders as `runner.setup.test.ts` does. Checks:
  - `last("environment").view.env`, `last("filter").filter`, `last("threads").count`;
  - `last("notice").message`, and that no `notice` event exists when none is expected;
  - `savedControls(dataDir, root)` for what was or was not saved;
  - a reopen is `setup(first.dataDir)` after `first.runner.dispose()`;
  - Refresh is `runner.refresh()` after rewriting `karate-pom.json`;
  - saving a run property is `runner.saveProperties({ shared: [{ name: "a", value: "1" }], byEnv: {} })` (check the `PropertiesEdit` shape in `src/shared`);
  - "the run passes …": after `runner.run("d:")`, the run's record (`record(dataDir, root, runId)`) has `env: "qa"`, `threads: 3` and `filter` including smoke and excluding slow.

  Add one test for a Maven project (a `pom.xml` whose Surefire entry is `${karate.env}`, with the property `dev`, and a feature under `src/test/java`): the env is `dev`.

- [ ] **Step 2: Run** `npx vitest run src/main/runner.pom.test.ts`. Expected: FAIL.

- [ ] **Step 3: Implement**:
  - `open`: `this.start(root, loadProjectFilter(…), loadProjectEnv(…), loadProjectThreads(…))` feeds `rediscover`.
  - `refresh`: `this.start(project.root, this.filter, this.envs, this.threads)`.
  - `rediscover`: when pruning changed the filter, `storeFilter(pruned)` only if `savedControls(…).filter`, else `this.filter = pruned`.
  - `saveProperties`: `saveProjectProperties(…, properties)` in place of `saveProjectEnv`.

- [ ] **Step 4: Run** `npx vitest run src/main/runner.pom.test.ts src/main/runner.setup.test.ts src/main/runner.env.test.ts src/main/runner.properties.test.ts src/main/runner.rerun.test.ts src/main/runner.tags.test.ts src/main/runner.threads.test.ts`, one file at a time. Expected: PASS. Fix any test that relied on `env` being deleted.

- [ ] **Step 5: Commit** `feat(runner): start the controls from the project's pom`, with `Part of #228`.

### Task 5: End-to-end

**Files:**
- Modify: `e2e/project-setup.spec.ts`, `e2e/threads.spec.ts`

- [ ] **Step 1: Update the existing tests**:
  - In `project-setup.spec.ts`, the first test's pom holds `env: "qa"`, which now applies and fails `a1`. Drop `env`; the notice is `Grace does not use these karate-pom.json settings: dryRun.`
  - In `threads.spec.ts`, "The box overrides a karate-pom.json" becomes "The box starts from karate-pom.json and the person's count wins". Opened with `{ threads: 3 }`, the box shows `3`. Set it to `1` and Run all; the row shows 1 thread (`expectRow(win, userData, dir, 0, 1)`).

- [ ] **Step 2: Add** two tests to `project-setup.spec.ts`:
  - **"karate-pom.json starts the environment, filter and thread count".** A project written to a temp folder holds `smoke.feature` (`@smoke` scenario `match cfg == 'qa'`), `slow.feature` (`@smoke @slow`) and `other.feature` (no tags), plus `karate-config.js` and `karate-config-qa.js` setting `cfg`. Its `karate-pom.json` holds `{"env":"qa","tags":["@smoke","~@slow"],"threads":3}`. Then:
    - `env-input` has the value `qa`;
    - `tag-chip` `smoke` has `data-state` `include` and `slow` has `exclude`;
    - `threads-input` has the value `3`;
    - there is no `notice`;
    - after Run all, the `smoke.feature` scenario is `passed` and the other two never ran.
  - **"Maven's karate.env starts the environment".** `project("maven")` with a `pom.xml` whose Surefire entry is `${karate.env}` and whose property is `dev`; `env-input` has the value `dev`.

- [ ] **Step 3: Ask the parent session for a go, then run** `npm run e2e`. Expected: all pass. Tell the parent when done.

- [ ] **Step 4: Commit** `test(e2e): start the controls from the project's pom`, with `Part of #228`.
