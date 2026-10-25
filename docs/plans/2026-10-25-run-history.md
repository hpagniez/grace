# Run History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the last 20 runs of each project in their own folders with an app-written `run.json`, list them in a History tab next to the console, and let the person open a past run's report, re-run it on its environment and tag filter, or re-run only its failed tests.

**Architecture:** Pure rules (the `run.json` shape and its parser, run ids, labels) live in `src/shared/history.ts`, used by main and the window. `src/main/history.ts` owns the disk side: run folders, `run.json` writing and listing, retention, clearing and start-up cleanup. The `Runner` gives each run its own folder, records the run when it ends, and gains `rerun`, `rerunFailed`, `clearHistory` and `runReportPath`; finding stored tests again uses `resolveNode`. The window gets a `history` event and shows it in a new bottom-panel tab.

**Tech Stack:** Electron 45, Solid 2 RC, Tailwind v4, TypeScript, Vitest 5, Playwright 1.64, bundled Temurin 21 + Karate 2.1.4.

**Spec:** [docs/specs/2026-10-25-run-history.md](../specs/2026-10-25-run-history.md) (R1–R8) and [ADR 0010](../decisions/0010-run-history-one-folder-per-run.md). Builds on the walking-skeleton, tag-filter and environment-switcher specs.

## Global Constraints

- Branch `feature/run-history`, cut from `develop` after the spec commit. Conventional Commits; AGENTS.md rules, including the Solid 2 rules.
- No new dependencies.
- Run folder: `<dataDir>/projects/<hash>/runs/<id>/`, `<id>` = local start time `yyyyMMdd-HHmmss-SSS`, then `-2`, `-3`… if taken. Id check: `^\d{8}-\d{6}-\d{3}(-\d+)?$`.
- Keep the 20 newest run folders per project (`HISTORY_LIMIT = 20`), by folder name.
- `run.json` has `version: 1` and never holds a property value, error text, step log or console line.
- Targets file: `<dataDir>/projects/<hash>/run-targets.txt`, UTF-8, one `"…"` target per line with `\` written as `\\` (and `"` as `\"`), passed as `@<absolute path>` where the targets were.
- The window sends only a run id; main never takes a path from it.
- Never run `npm test` and `npm run e2e` at the same time.
- Exact copy:
  - tab labels `Console`, `History (<n>)`; empty tab `No runs yet`; header action `Clear history`
  - row actions `Report`, `Re-run`, `Re-run failed`; toolbar `Re-run failed (<n>)`
  - result `<p> passed, <f> failed of <t>`; cancelled `Cancelled, <p> passed, <f> failed of <t>`; error `Error (exit <code>)` or `Error`
  - what ran `All`, a folder or feature path, a test name, or `Failed tests of <when>`
  - when `Today HH:mm`, `Yesterday HH:mm`, else `yyyy-MM-dd HH:mm`
  - confirm `Delete the <n> runs of this project and their reports?` with `Delete` and `Cancel`
  - notices `The tests of this run no longer exist`; `<n> failed tests no longer exist` (`1 failed test no longer exists`); `<n> failed rows of a dynamic Examples table can't be run alone. Run its outline.` (`1 failed row …`); `Nothing to run`; `This run is no longer in the history`; `Some runs could not be deleted`
  - Properties note gets ` Run history keeps the reports of the last 20 runs; Clear history deletes them.` appended

## Review Focus

1. **Two runs started in the same millisecond, or a clock set back** (folder name taken): the id gets `-2`, and the list stays sorted by `startedAt`, not by name. Test in Task 2 (`runId appends a suffix when taken`, `listHistory sorts by startedAt`).
2. **A forged id from the window** (`..\..\x`, `20261025-143205-123\..`, an id of another project): refused with "Invalid request" in main.ts, or "This run is no longer in the history" in the Runner; nothing is read or deleted. Tests in Task 5 (`rerun refuses an unknown id`) and the main.ts id check.
3. **A `run.json` from a newer version or hand-edited** (`version: 2`, `tests` not an array, `env` `"../x"`, a 2 MB file): not listed, never throws, other runs still listed. Test in Task 1 (`parseRunRecord rejects bad shapes`) and Task 2.
4. **Quitting the app during a run**: the run is killed and its `run.json` says `cancelled`; a folder left without one is deleted at the next start. Tests in Task 4 (`dispose records the killed run as cancelled`) and Task 2 (`removeLeftoverRuns`).
5. **A Re-run whose environment switch is cancelled or hits a parse error**: nothing runs, no history entry is added, and the environment follows environment switcher R1. Tests in Task 5 (`cancel during a re-run's environment switch runs nothing`, `a re-run whose environment has a parse error switches and runs nothing`).

---

### Task 1: Shared history model and labels

**Files:**
- Create: `src/shared/history.ts`, `src/shared/history.test.ts`

**Interfaces:**
- Produces (no Node imports):
  - `HISTORY_LIMIT = 20`, `RUN_ID = /^\d{8}-\d{6}-\d{3}(-\d+)?$/`
  - `type TestStatusFinal = "passed" | "failed" | "notRun" | "interrupted"`
  - `interface StoredTest { kind: "scenario" | "row"; file: string; refId: string; name: string; line: number | null; key: string; status: TestStatusFinal; durationMillis?: number }`
  - `interface StoredSelection { kind: NodeKind; file: string; name: string; line: number | null; key: string }`
  - `interface RunRecord { version: 1; startedAt: number; durationMillis: number; env: string | null; filter: FilterState; selection: StoredSelection | null; failedOf: number | null; outcome: RunOutcome; exitCode: number | null; tests: StoredTest[] }`
  - `interface HistoryEntry { id: string; startedAt: number; durationMillis: number; env: string | null; filter: FilterState; selection: StoredSelection | null; failedOf: number | null; outcome: RunOutcome; exitCode: number | null; passed: number; failed: number; total: number; reportAvailable: boolean }`: what the window gets; no `tests`.
  - `parseRunRecord(x: unknown): RunRecord | null`: every field checked. `env` must be `null` or `validEnvName`, `filter` must pass `parseFilterState`, and `version` must be `1`. A bad test row rejects the whole record.
  - `toEntry(id: string, r: RunRecord, reportAvailable: boolean): HistoryEntry`: counts come from `tests`.
  - `whenLabel(startedAt: number, now: number): string`, `whatRanLabel(e: HistoryEntry, now: number): string`, `resultLabel(e: HistoryEntry): string`, using the exact copy in Global Constraints.

- [ ] **Step 1: Write the failing tests** in `src/shared/history.test.ts`:
  - `RUN_ID accepts ids and rejects traversal`: `20261025-143205-123` and `20261025-143205-123-2` pass; `..`, `20261025-143205-123\..`, `x/20261025-143205-123` and `` fail
  - `parseRunRecord accepts the spec example`: the R2 JSON parses to an equal object
  - `parseRunRecord rejects bad shapes`: `null`, `version: 2`, `tests: {}`, a test with status `pending`, `env: "../x"`, `filter.include: ["a b"]`, a missing `outcome`
  - `toEntry counts from tests`: 3 passed, 2 failed, 1 notRun and 1 interrupted give `passed 3, failed 2, total 7`
  - `whenLabel`: same day gives `Today 14:32`; the day before, across midnight (23:59 run, 00:01 now), gives `Yesterday 23:59`; older gives `2026-10-21 09:05`
  - `whatRanLabel`: root folder (`file ""`) gives `All`; a feature gives its path; a row gives its name; `selection: null` gives `Failed tests of Today 14:32`
  - `resultLabel`: finished `41 passed, 2 failed of 43`; cancelled `Cancelled, 3 passed, 1 failed of 43`; failed with exit 3 `Error (exit 3)`; failed with `null` `Error`
- [ ] **Step 2: Run** `npx vitest run src/shared/history.test.ts`. Expected: FAIL, module not found.
- [ ] **Step 3: Implement** `src/shared/history.ts`. Format times in local time with `getHours`/`getMinutes`, padded.
- [ ] **Step 4: Run** `npx vitest run src/shared/history.test.ts`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(history): add the run record model and labels`

### Task 2: History on disk and the targets file

**Files:**
- Create: `src/main/history.ts`, `src/main/history.test.ts`
- Modify: `src/main/karate/argfile.ts` (`writeTargetFile`; `removeLeftoverArgFiles` also deletes `run-targets.txt`), `src/main/karate/argfile.test.ts`

**Interfaces:**
- Consumes: Task 1 (`RunRecord`, `HistoryEntry`, `parseRunRecord`, `toEntry`, `HISTORY_LIMIT`, `RUN_ID`).
- Produces:
  - `runId(now: Date, taken: (id: string) => boolean): string`
  - `createRunDir(projectDir: string, now: Date): { id: string; dir: string }`: creates `runs/<id>`
  - `writeRunRecord(dir: string, r: RunRecord): void`
  - `readRunRecord(projectDir: string, id: string): RunRecord | null`: `null` for an id not matching `RUN_ID`, a missing folder or an invalid file. Files over 5 MB are not read.
  - `listHistory(projectDir: string): HistoryEntry[]`: valid records only, newest `startedAt` first, at most `HISTORY_LIMIT`; `reportAvailable` = `karate-summary.html` exists. Never throws.
  - `pruneHistory(projectDir: string): void`: deletes every folder past the 20 newest names; a failure is skipped silently.
  - `clearHistory(projectDir: string): boolean`: deletes every run folder; `false` if any stayed.
  - `removeLeftoverRuns(dataDir: string): void`: deletes, under each `projects/<hash>/`, run folders without `run.json` and the 1.0 `run/` folder. Never throws.
  - `writeTargetFile(file: string, targets: string[]): void` in `argfile.ts`: one quoted line per target, CRLF, UTF-8, same `quote()` as the argument file.

- [ ] **Step 1: Write the failing tests:**
  - `history.test.ts` (temp folders):
    - `runId formats local time` (`new Date(2026, 9, 25, 14, 32, 5, 7)` gives `20261025-143205-007`)
    - `runId appends a suffix when taken` (`-2`, then `-3`)
    - `write then list round-trips`
    - `listHistory sorts by startedAt` (folder names in the opposite order)
    - `listHistory skips invalid and unknown-version files`
    - `listHistory caps at 20`
    - `reportAvailable follows karate-summary.html`
    - `pruneHistory keeps the 20 newest names, invalid ones included`
    - `clearHistory removes all and reports a held folder` (hold a file open with `openSync` on Windows)
    - `removeLeftoverRuns deletes folders without run.json and 1.0 run folders, keeps valid runs`
    - `readRunRecord refuses an id with ..`
  - `argfile.test.ts`:
    - `writeTargetFile quotes, doubles backslashes and writes UTF-8` (a path with a space and `é` reads back byte-exact as UTF-8)
    - `removeLeftoverArgFiles also deletes run-targets.txt`
    - a Node test with the bundled Java: targets for `fixture("edge")` in a temp copy whose path holds a space and `é`, passed as `@<file>`, run exactly the two listed tests (read `SCENARIO_EXIT` refIds from the events file). Copy the spawn helper style of `runner.test.ts`.
- [ ] **Step 2: Run** `npx vitest run src/main/history.test.ts src/main/karate/argfile.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** both files.
- [ ] **Step 4: Run** the same tests, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(history): store runs on disk with retention and cleanup`

### Task 3: Finding stored tests again, line targets, run state per key list

**Files:**
- Modify: `src/main/karate/tree.ts` (`resolveNode` takes `Pick<TestNode, "key" | "kind" | "file" | "name">`; add `resolveTests`), `src/main/karate/tree.test.ts`
- Modify: `src/main/karate/command.ts` (add `lineTargets`), `src/main/karate/command.test.ts`
- Modify: `src/main/karate/run-state.ts`, `src/main/karate/run-state.test.ts`, and the `new RunState` call in `src/main/runner.ts`

**Interfaces:**
- Produces:
  - `resolveTests(tree: TestNode, tests: StoredTest[]): { found: TestNode[]; missing: number; dynamic: number }`: each test goes through `resolveNode`; one that is found but has `line === null` counts as `dynamic`; `found` has no duplicates and is in tree order.
  - `lineTargets(nodes: TestNode[], root: string): string[]`: one `<abs file>:L1:L2…` per feature file, lines ascending, files in first-seen order.
  - `new RunState(tree: TestNode, keys: string[])`: the caller passes the keys. Today's caller passes `matchingLeaves(node, filter).map((n) => n.key)`.
  - `RunState.results(): Map<string, { status: TestStatusFinal; durationMillis?: number }>`: valid after `finish`. A key with no status, or still `pending`/`running`, becomes the status `finish` gave it, else `notRun`.

- [ ] **Step 1: Write the failing tests:**
  - `resolveTests finds moved and same-key tests, counts missing and dynamic` (hand-built tree: a scenario whose key shifted keeps its name; a deleted one; a dynamic row with `line: null`; two same-name scenarios in one feature count as missing)
  - `lineTargets groups by file with ascending lines`
  - `run state tracks exactly the given keys`
  - `results after a finished run: notRun for unexited keys`
  - `results after a cancel: interrupted for running, notRun for never started`
  - `results keep durations`
- [ ] **Step 2: Run** `npx vitest run src/main/karate`. Expected: FAIL.
- [ ] **Step 3: Implement**, and update the one `RunState` call site in `runner.ts`.
- [ ] **Step 4: Run** `npx vitest run src/main` and `npm run typecheck`. Expected: PASS. The existing run-state and runner tests change only in how they construct `RunState`.
- [ ] **Step 5: Commit** `feat(runner): resolve stored tests and build line targets`

### Task 4: Each run gets its own folder and record

**Files:**
- Modify: `src/main/runner.ts`, `src/main/runner.test.ts`, `src/main.ts`, `src/shared/events.ts`
- Create: `test/fixtures/projects/history/` (see the spec's acceptance criteria): `karate-config.js` (`function fn() { return {}; }`), `karate-config-qa.js` (same), `flag.json` (`{ "fixed": false }`), `tests/fix.feature` (`fixable` reads `classpath:flag.json` and matches `flag.fixed == true`; `@smoke` `stays green`; outline `row <n>` with rows 1 and 2 matching `<n> == 1`; outline `dynamic <n>` reading `rows.json`, matching `n == 1`), `tests/rows.json` (`[{ "n": 1 }, { "n": 2 }]`)
- Modify: `test/fixtures/copy.ts` (`"history"` in the name union, with a doc line)

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces:
  - `AppEvent` gains `{ type: "history"; runs: HistoryEntry[] }`; `runStarted` gains `runId: string`.
  - `Runner.replay()` adds the `history` event after `environment`.
  - The Runner emits `history` at the end of every `rediscover` (success or cancel), after each run's record is written, and after a clear.
  - `Runner.dispose()` waits up to 10 s for the running operation to finish after the kill, so the run's record is written.
  - `main.ts` calls `removeLeftoverRuns(dataDir)` next to `removeLeftoverArgFiles`.

- [ ] **Step 1: Write the failing tests** in a new `describe("Runner history")` block:
  - `a run writes run.json in its own folder`: run `d:` on `clean`; assert one folder under `runs/`, its record has `env: null`, `selection.key: "d:"`, `outcome: "finished"`, `exitCode: 1`, and a `failing match` test with status `failed`; the `history` event lists it with `reportAvailable: true`
  - `run.json holds no secret` (env fixture, secret `s3cret-token-123`; the file text does not contain it)
  - `two runs keep two folders, newest first`
  - `retention keeps 20` (seed 20 records with `writeRunRecord`, run once, 20 folders remain and the oldest is gone)
  - `a refused run creates no folder` (an unreadable secret)
  - `a cancelled run is recorded as cancelled with notRun for tests that never started`
  - `dispose records the killed run as cancelled`
  - `replay includes history`
  - no existing test reads `projects/<hash>/run/`; `does not modify the project` must still pass unchanged
- [ ] **Step 2: Run** `npx vitest run src/main/runner.test.ts -t "Runner history"`. Expected: FAIL.
- [ ] **Step 3: Implement:** `outDir("run")` becomes `createRunDir(this.projectDir(), new Date())`. When the run ends, after `state.finish`, build the `RunRecord`: `selection` from the node, `failedOf: null`, the filter as used, and `tests` from `state.results()` joined with the tree nodes. Then `writeRunRecord`, `pruneHistory`, and emit `history`. Track the running operation's promise for `dispose`.
- [ ] **Step 4: Run** `npx vitest run src/main` and `npm run typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): keep each run in its own folder with a run record`

### Task 5: Re-run, re-run failed, clear, past reports; IPC

**Files:**
- Modify: `src/main/runner.ts`, `src/main/runner.test.ts`, `src/main.ts`, `src/shared/ipc.ts`, `src/preload.ts`

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces:
  - `CH.rerun = "history:rerun"`, `CH.rerunFailed = "history:rerunFailed"`, `CH.runReport = "history:report"`, `CH.clearHistory = "history:clear"`
  - `Api.rerun(id: string)`, `Api.rerunFailed(id: string)`, `Api.openRunReport(id: string)`, `Api.clearHistory()`, all `Promise<void>`
  - `Runner.rerun(id: string): Promise<void>`, `Runner.rerunFailed(id: string): Promise<void>`, `Runner.runReportPath(id: string): string | null`, `Runner.clearHistory(): void` (refused while busy with the busy notice)
  - In `main.ts`, the history handlers reject an id that is not a string matching `RUN_ID` with "Invalid request". `CH.runReport` opens the path with `shell.openPath`.

- [ ] **Step 1: Write the failing tests** (history fixture unless noted):
  - `rerunFailed runs only the failed tests that can run`: Run all, write `{ "fixed": true }` to `flag.json`, then `rerunFailed`. Only `fixable` and `row 2` go pending; `fixable` passes and `row 2` fails. The notice names 1 failed row of a dynamic Examples table. The new record has `selection: null`, `failedOf` = the first run's `startedAt`, and 2 tests.
  - `rerunFailed passes its targets in a file` (the java command line contains `@` and `run-targets.txt`, not the feature path; the file is gone after the run)
  - `rerun switches environment and filter back`: run `f:tests/fix.feature` with env `qa` and include `smoke`, then `setEnv(null)` and an empty filter, then `rerun`. The environment is `qa` (saved), the filter is `include: ["smoke"]` (saved), and only `stays green` ran.
  - `rerun with a vanished included tag runs nothing, never more` (`clean`: the record's filter includes a tag no longer in the files)
  - `rerun of a deleted test says the tests of this run no longer exist`
  - `rerun of a rerun-failed run runs all its tests and keeps failedOf`
  - `cancel during a re-run's environment switch runs nothing` (env fixture: no new record; environment restored)
  - `a re-run whose environment has a parse error switches and runs nothing` (`broken` fixture, a seeded record with env `qa`: environment `qa` saved, no new record)
  - `rerun checks the target environment's secrets first` (unreadable secret in `qa`, current env `null`: environment unchanged, refusal notice, no record)
  - `rerun refuses an unknown id` ("This run is no longer in the history")
  - `clearHistory deletes the runs and emits an empty history`
  - `runReportPath is null for a cancelled run and for an unknown id`
- [ ] **Step 2: Run** `npx vitest run src/main/runner.test.ts -t "Runner history"`. Expected: FAIL.
- [ ] **Step 3: Implement.** Refactor `runGuarded(key)` into `runTarget(target, filter)`, where `target` is `{ old: Pick<TestNode, "key" | "kind" | "file" | "name"> }` or `{ tests: StoredTest[]; failedOf: number }`. `run(key)` passes the clicked node and `this.filter`. Make `runProperties` take the environment.
  - `rerun` and `rerunFailed`, inside `guarded`:
    1. read the record;
    2. check `runProperties(record.env)`;
    3. if the environment differs, `rediscover` with `{ ...this.envs, env: record.env }` and the record's filter, then save the environment as `setEnv` does; else store `pruneFilter(record.filter, projectTags(tree))` and send it;
    4. call `runTarget` with `record.filter` as written.
  - The tests target uses `resolveTests`, the notices from Global Constraints, `lineTargets` and `writeTargetFile`. It passes `@<file>` as the only target, deletes the file in the same `finally` as the argument file, and stores `failedOf`.
- [ ] **Step 4: Run** `npx vitest run src/main`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): re-run, re-run failed and clear history`

### Task 6: History tab, toolbar, Properties note

**Files:**
- Create: `src/renderer/BottomPanel.tsx`, `src/renderer/History.tsx`, `src/renderer/History.test.tsx`
- Modify: `src/renderer/reduce.ts`, `src/renderer/reduce.test.ts`, `src/renderer/App.tsx`, `src/renderer/Console.tsx` (drop its own height; the panel sets it), `src/renderer/Toolbar.tsx`, `src/renderer/PropertiesDialog.tsx`

**Interfaces:**
- Consumes: Task 1 labels, Task 4 events, Task 5 `Api`.
- Produces:
  - `AppState` gains `history: HistoryEntry[]` and `panel: "console" | "history"`; `RunView` gains `runId: string | null`.
  - In `reduce`:
    - `history` replaces the list, and sets `run.reportAvailable` to false unless the list still has `run.runId` with `reportAvailable`;
    - `runStarted` sets `panel = "console"` and `run.runId`;
    - a `project` event with another root empties `history`.
  - The store gains `showPanel(p: "console" | "history"): void`.

- [ ] **Step 1: Write the failing tests:**
  - `reduce.test.ts`:
    - `history event replaces the list`
    - `runStarted switches to the console`
    - `clearing the history disables Report`
    - `another project empties the history`
  - `History.test.tsx`:
    - `lists runs with labels` (one row's When, Env `—`, Tags chips including `not @slow`, What ran, Result and Duration)
    - `No runs yet`
    - `Report disabled without a report`
    - `Re-run failed disabled with no failure`
    - `actions call the api with the id`
    - `all actions disabled while busy`
    - `Clear history confirms with the count, Cancel does nothing, Delete calls clearHistory`
  - In the existing Toolbar/App tests (or new ones in `History.test.tsx`): `toolbar Re-run failed shows the newest run's failed count and calls rerunFailed with its id`, `disabled with no history or while busy`.
- [ ] **Step 2: Run** `npx vitest run src/renderer`. Expected: FAIL.
- [ ] **Step 3: Implement:**
  - `BottomPanel` keeps the console's `h-48` and has two tab buttons: `Console`, and `History (n)`.
  - `History` is a table with the columns of spec R3. The full selection path goes in a `title`.
  - Clear history uses a `<dialog>` with `showModal()`, as `PropertiesDialog` does.
  - The toolbar's `Re-run failed (n)` sits after Run all.
  - Append the exact sentence to the Properties note constant.
- [ ] **Step 4: Run** `npx vitest run src/renderer src/shared`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add the history tab and re-run failed`

### Task 7: End-to-end scenarios and ship

**Files:**
- Create: `e2e/history.spec.ts`
- Modify: `e2e/helpers.ts` (`project()` accepts `"history"`; add `seedRuns(userData: string, root: string, n: number, opts?: { report?: boolean }): string[]`, which writes `n` valid records with `writeRunRecord`, one minute apart, plus a `karate-summary.html` when `report` is true, and returns their ids), `docs/features.md` (F10 `done`)

- [ ] **Step 1: Write** one Playwright test per Gherkin scenario of the spec, in its order, on fresh copies and fresh userData:
  - Use `seedRuns` where the scenario says "has N runs in its history".
  - Use `stubOpenPath` for Report, as `e2e/report.spec.ts` does.
  - For restarts, close the app before relaunching, as `e2e/tags.spec.ts` does.
  - For "A secret never reaches the history files", read the newest `runs/*/run.json` under userData.
- [ ] **Step 2: Run** `npm run e2e`. If packaging hits `EBUSY` on `out\`, package with Forge's `api.package` into a scratch `outDir`, then run `npx playwright test`. Expected: all tests pass, including the existing ones.
- [ ] **Step 3: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 4: Update** `docs/features.md`: set F10 to `done`.
- [ ] **Step 5: Commit** `test(e2e): cover run history` and `docs: mark f10 done`
