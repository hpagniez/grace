# Flaky Tests Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mark each test that passed in one run and failed in another, with the same project files and environment, in the tree (counts on the nodes above it), and show its proving runs in the Details pane.

**Architecture:** A new `src/main/fingerprint.ts` hashes the project files' paths, sizes and modified times. `Runner.runBody` takes the fingerprint before Java starts and after it exits, and stores it in `run.json` (`fingerprint`, version stays 1) only when both match. A pure `findFlaky` in `src/main/flaky.ts` groups the listed runs by fingerprint and environment and places each flaky test in the current tree with `resolveNode`. The Runner sends the result in a new `flaky` event after every `project` and every `history` event, and in the replay. The renderer keeps `AppState.flaky`, `Tree.tsx` draws the mark and the counts, and a new `FlakyProof` component shows the proof in `Details.tsx`.

**Tech Stack:** Electron 45, TypeScript, SolidJS 2 RC, Tailwind v4, Vitest 5 (jsdom for `*.test.tsx`), Playwright 1.64.

**Spec:** [docs/specs/2026-10-26-flaky-tests.md](../specs/2026-10-26-flaky-tests.md) (R1–R6).

## Global Constraints

- Branch `feature/flaky-tests`. The spec commit `767fa67` is on it. Conventional Commits with lower-case subjects. AGENTS.md rules apply: Solid 2, no `^` ranges, no new dependency (`node:crypto` and `node:fs/promises` are enough).
- Fingerprint values, copied exactly:
  - One line per file: `<path relative to the root, / separators>\t<size in bytes>\t<Math.trunc(mtimeMs)>\n`. The lines are sorted with the default `Array.prototype.sort`. The fingerprint is the SHA-256 of the joined lines, as lowercase hex.
  - Skipped folder names, compared in lower case at any depth: `.git`, `node_modules`, `target`, `build`, `.gradle`. Symbolic links and junctions are neither followed nor listed.
  - Null past `MAX_FILES = 50_000` files, past `WALK_TIMEOUT_MS = 2_000`, or on any read or stat error. Computing it never throws.
- `run.json` keeps `version: 1`. `fingerprint` matches `/^[0-9a-f]{64}$/` or is null, and a missing field reads as null. `HistoryEntry` does not change.
- UI copy, exactly:
  - The mark is `flaky`, with tooltip and accessible name `Flaky: passed and failed with the same project files`.
  - The count is `<n> flaky`.
  - The headings are `Flaky on <env>` and `Flaky with no environment`.
  - The count line is `Passed <p> time(s), failed <f> time(s) with the same project files`, singular for 1.
  - The thread labels are `1 thread`, `<n> threads`, and `—` for null.
- Test ids: `flaky-mark`, `flaky-count`, `flaky-proof` (one per environment), `flaky-run` (one per proof row, with `data-id` and `data-status`), `flaky-run-report`.
- Runner tests start real JVMs, so run `npm run vendor` once first. Never run `npm test` and `npm run e2e` at the same time. Run one e2e file with `npm run package && npx playwright test e2e/<file>.spec.ts`.

## Review Focus

1. **Cancel pressed while the first fingerprint walk runs.** No Java may start, no run folder may be left, and the notice is `Run cancelled`. Test: Task 2, `a cancel during the first walk starts no Java`.
2. **A window that loads after the events were sent.** That covers Forge's dev reload and a reopened window. The replay must carry `flaky`, or the marks vanish until the next run. Test: Task 3, the replay assertion in `sends the flaky tests with the history`.
3. **Outline rows.** The mark goes on the row, and the outline, feature and folder each count it once. Tests: Task 3, `an outline row is marked by its row key`; Task 4, `marks flaky tests and counts them above`.
4. **A test flaky only in an environment that is not the current one.** The mark still shows, and Details lists the current environment's box first. Test: Task 4, `orders the boxes and words them`.
5. **The same name twice in a feature after an edit.** The test gets no mark rather than a mark on the wrong test. Test: Task 3, `placed by resolveNode`.

---

### Task 1: Project fingerprint

**Files:**
- Create: `src/main/fingerprint.ts`, `src/main/fingerprint.test.ts`

**Interfaces:**
- Produces:
  - `SKIPPED_DIRS: readonly string[]`
  - `MAX_FILES = 50_000`
  - `WALK_TIMEOUT_MS = 2_000`
  - `projectFingerprint(root: string, limits?: { maxFiles: number; timeoutMs: number }): Promise<string | null>`. The default limits are `{ maxFiles: MAX_FILES, timeoutMs: WALK_TIMEOUT_MS }`.

- [ ] **Step 1: Write the failing tests.** Each test uses a fresh temp root.
  - `matches the spec's line format`: the root holds `a.txt` (`abc`) and `sub/b.txt`. Build the expected hash in the test from `statSync` (`a.txt\t3\t<trunc mtimeMs>\nsub/b.txt\t…\n`, sorted) and compare. The value matches `/^[0-9a-f]{64}$/`. A second call gives the same value.
  - `each change gives a new value`: an `it.each` over five changes, each compared with the value before it:
    - append a byte;
    - `utimesSync` the file +5 s;
    - add a file;
    - remove a file;
    - rename a file.
  - `skips build and tool folders at any depth and in any case`: adding `.git/x`, `node_modules/x`, `Target/x`, `sub/build/x` and `.GRADLE/x` leaves the value unchanged. Adding `sub/targets/x` changes it.
  - `does not follow or list a junction`: `symlinkSync(<another temp dir holding files>, <root>/link, "junction")` leaves the value unchanged.
  - `null past the file cap`: 4 files give null with `maxFiles: 3`, and a string with `maxFiles: 4`.
  - `null past the time bound`: a root with a subfolder gives null with `timeoutMs: 0`.
  - `null for a missing root`: the call resolves, it does not reject.
  - `null when a stat fails`: `vi.mock("node:fs/promises", …)` wraps the real module, and its `lstat` rejects while a flag is set.
- [ ] **Step 2: Run** `npx vitest run src/main/fingerprint.test.ts`. Expected: FAIL, because `./fingerprint` does not exist.
- [ ] **Step 3: Implement** `projectFingerprint`:
  - Walk with `readdir(dir, { withFileTypes: true })`. Take each folder's entries in parallel (`Promise.all`).
  - Skip `isSymbolicLink()` entries. If the junction test still fails, `lstat` each folder before entering it.
  - Recurse into folders whose lower-case name is not in `SKIPPED_DIRS`.
  - `lstat` each file.
  - After each `readdir`, give up with null once `Date.now() >= start + timeoutMs` or the count passes `maxFiles`.
  - Any rejection gives null.
- [ ] **Step 4: Run** the same command, then `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(history): fingerprint the project files`.

### Task 2: Each run stores its fingerprint

**Files:**
- Modify: `src/shared/history.ts`, `src/main/runner.ts`
- Test: `src/shared/history.test.ts`, `src/main/runner.test.ts`
- Fixtures that must gain `fingerprint: null`: `record()` in `src/main/history.test.ts`, `seed` in `src/main/runner.test.ts`, and `seedRuns` in `e2e/helpers.ts`.

**Interfaces:**
- Consumes: `projectFingerprint` (Task 1).
- Produces:
  - `FINGERPRINT = /^[0-9a-f]{64}$/`, in `src/shared/history.ts`.
  - `RunRecord.fingerprint: string | null`, documented as "SHA-256 of the project files' paths, sizes and times, the same before Java started and after it exited (F15). Null otherwise, and for a run stored before F15."
  - `parseRunRecord` reads a missing or null `fingerprint` as null. Any value that fails `FINGERPRINT` makes the record malformed.
  - `toEntry` drops `fingerprint`, so `HistoryEntry` is unchanged.
  - A Runner constructor option `fingerprint?: (root: string) => Promise<string | null>`. It is a test seam whose default is `projectFingerprint`.
  - `private disposing = false` on the Runner, set first thing in `dispose`.
  - `runBody`:
    1. Takes `before = await fingerprint(root)` before `createRunDir`.
    2. If `this.cancelled` is then set, it sends the notice `Run cancelled` and returns: no folder, no Java.
    3. `end(code, cancelled)` sends the end events as today, sets `ended`, and returns the `runFinished` event.
    4. `store(done, fingerprint: string | null)` writes the record.
    5. After `proc.exit`, it takes `after = this.disposing ? null : await fingerprint(root)`, then calls `store(done, before !== null && before === after ? before : null)`.
    6. The pid-file failure path calls `store(end(null, false), null)`.
- `setup()` in `runner.test.ts` takes the fingerprint option as a 4th argument.

- [ ] **Step 1: Write the failing tests.**
  - `src/shared/history.test.ts`:
    - `specExample` gets `fingerprint: "ab".repeat(32)` and still round trips.
    - New `reads a record stored before F15 as no fingerprint`: the example without the field parses with `fingerprint: null`.
    - `rejects bad shapes` gains `"AB".repeat(32)`, `"ab"`, `1` and `"g".repeat(64)`.
    - The `toEntry` result has no `fingerprint` key.
  - `src/main/runner.test.ts`, in "Runner history":
    - `a run of an unchanged project stores its fingerprint`: open `clean` and run `passing hello`. The record's `fingerprint` equals `await projectFingerprint(root)` and is not null.
    - `a file saved during a run stores no fingerprint`: run `f:tests/slow.feature`. After `runStarted`, write `<root>/note.txt`. The record's `fingerprint` is null.
    - `different walks store no fingerprint`: the fake returns `"a".repeat(64)`, then `"b".repeat(64)`. The record stores null.
    - `a cancel during the first walk starts no Java`: the fake calls `await runner.cancel()`, then returns `"a".repeat(64)`. Running `passing hello` gives no `runStarted`, the last notice `Run cancelled`, and no run folder.
    - `dispose skips the second walk`: a counting fake, then dispose during a run of `slow.feature`. The fake was called once, and the record matches `{ outcome: "cancelled", fingerprint: null }`.
    - `runFinished comes before the run's history`: the last `runFinished` sits before the last `history` in `events`.
- [ ] **Step 2: Run** `npx vitest run src/shared/history.test.ts`, then `npx vitest run src/main/runner.test.ts -t "Runner history"`. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above, plus the fixture fields.
- [ ] **Step 4: Run** `npx vitest run src/shared src/main/history.test.ts`, then `npx vitest run src/main/runner.test.ts -t "Runner history"`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(history): store the project fingerprint of each run`.

### Task 3: Find flaky tests and send them to the window

**Files:**
- Create: `src/shared/flaky.ts`, `src/main/flaky.ts`, `src/main/flaky.test.ts`
- Modify: `src/main/history.ts`, `src/shared/events.ts`, `src/main/runner.ts`
- Test: `src/main/history.test.ts`, `src/main/runner.test.ts`

**Interfaces:**
- Consumes: `RunRecord.fingerprint` (Task 2), and `resolveNode` from `src/main/karate/tree.ts`.
- Produces:
  - In `src/shared/flaky.ts`, with no Node imports:
    ```ts
    export interface FlakyRun { id: string; startedAt: number; status: "passed" | "failed"; threads: number | null; reportAvailable: boolean }
    export interface FlakyEnv { env: string | null; passed: number; failed: number; runs: FlakyRun[] } // runs newest first
    export interface FlakyTest { key: string; envs: FlakyEnv[] }
    ```
  - In `src/main/history.ts`: `interface ListedRun { id: string; record: RunRecord; reportAvailable: boolean }`. `listRuns(projectDir: string): ListedRun[]` returns the valid records, newest `startedAt` first, at most `HISTORY_LIMIT`, and never throws. `listHistory` becomes `listRuns(...).map((r) => toEntry(r.id, r.record, r.reportAvailable))`.
  - In `src/main/flaky.ts`: `findFlaky(runs: ListedRun[], tree: TestNode | null): FlakyTest[]`:
    1. With no tree, return `[]`.
    2. Drop the runs whose `fingerprint` is null. Group the rest by `JSON.stringify([fingerprint, env])`.
    3. In each group, for each stored test `key`, collect the runs where its status is `passed` or `failed`. The test is flaky in the group when that set holds both statuses.
    4. Resolve it with `resolveNode(tree, <its stored test in the newest of those runs>)`. Skip it if the result is null.
    5. Merge by the resolved node key, then by `env`: add up `passed` and `failed`, and concatenate the runs. Sort each run list newest first.
  - In `src/shared/events.ts`: `| { type: "flaky"; tests: FlakyTest[] }`, documented as "The flaky tests of the tree last sent, by their key in it (F15). Follows every `project` and every `history` event."
  - On the Runner:
    - A private `sendHistory()` reads `listRuns` once and emits `history`, then `flaky` (`findFlaky(runs, this.tree)`). It replaces every `this.o.emit(this.historyEvent())`.
    - `publish(e)` emits `flaky` right after the `project` event.
    - `replay()` becomes `[last, filter, threads, environment, history, flaky, busy]`.
    - With no project, `flaky` carries `[]`.

- [ ] **Step 1: Write the failing tests.**
  - `src/main/flaky.test.ts` uses a hand-built tree, as `src/renderer/Tree.test.tsx` builds one. Feature `tests/a.feature` holds scenarios `one` (`s:tests/a.feature#1`) and `two` (`#2`), and an outline `rows` (`o:tests/a.feature#3`) with rows `rows 1` (`#3.1`) and `rows 2` (`#3.2`). A `run(i, over)` helper builds a `ListedRun` with `startedAt = i`.
    - `a pass and a fail with the same files and environment make a test flaky`: gives `[{ key: "s:tests/a.feature#1", envs: [{ env: null, passed: 1, failed: 1, runs: <newest first, with threads and reportAvailable copied> }] }]`.
    - `two passes are not flaky`.
    - `a null fingerprint proves nothing`.
    - `different fingerprints are not compared`.
    - `different environments are not compared`, and the same pair in two environments gives two `envs`.
    - `threads, filter and outcome play no part`: a fail in a cancelled run with 8 threads plus a pass with 1 thread and another filter make the test flaky.
    - `notRun and interrupted do not count`, and such runs are not in `runs`.
    - `proofs of two fingerprints of one environment merge`: gives `passed: 2, failed: 2` and 4 runs, newest first.
    - `placed by resolveNode`:
      - a stored `one` at `#1`, with `one` now at `#2`, gives key `#2`;
      - a name that appears twice in the feature, with its stored key now held by another name, gives nothing;
      - a vanished test gives nothing.
    - `an outline row is marked by its row key`: `#3.2`.
    - `no tree, no flaky`.
  - `src/main/history.test.ts`: `listRuns returns what listHistory lists`. It has the same ids in the same order, and each record equals what was written.
  - `src/main/runner.test.ts`:
    - Add `last("flaky")` after `last("history")` in the `replay()` `toEqual`s (about lines 209, 584, 842 and 1567).
    - Insert `"flaky"` after each `"project"` and each `"history"` in the `typesAfter` lists (about lines 777, 794 and 909).
    - New `describe("Runner flaky")`:
      - `sends the flaky tests with the history`: `seed` two `clean` records with fingerprint `"a".repeat(64)` and `passing hello` (kind `scenario`, file `tests/basic.feature`, refId `[1:7]`, line 7, key `s:tests/basic.feature#1`), `passed` in one and `failed` in the other. After `open`, `last("flaky").tests` is that key with `{ env: null, passed: 1, failed: 1 }`, and `runner.replay()` holds the same event.
      - `follows a new tree`: after `open`, prepend a scenario `added` (`  * match 1 == 1`) to `tests/basic.feature` and call `refresh()`. The flaky key is `byName(last("project").tree!, "passing hello").key`, and it is not `s:tests/basic.feature#1`.
      - `clear history leaves no flaky tests`: after `clearHistory()`, `last("flaky").tests` is `[]`.
- [ ] **Step 2: Run** `npx vitest run src/main/flaky.test.ts src/main/history.test.ts`, then `npx vitest run src/main/runner.test.ts -t "Runner flaky"`. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npx vitest run src/main`, then `npm run typecheck` and `npm run lint`. Expected: PASS, the updated event lists included.
- [ ] **Step 5: Commit** `feat(history): find flaky tests and send them to the window`.

### Task 4: Marks in the tree and proof in Details

**Files:**
- Create: `src/shared/flaky.test.ts`, `src/renderer/FlakyProof.tsx`, `src/renderer/FlakyProof.test.tsx`
- Modify: `src/shared/flaky.ts`, `src/renderer/reduce.ts`, `src/renderer/Tree.tsx`, `src/renderer/Details.tsx`, `src/renderer/App.tsx`
- Test: `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: the `flaky` event and the `FlakyEnv` and `FlakyRun` types (Task 3), `whenLabel` (`src/shared/history.ts`), `Api.openRunReport`, and `leaves` (`src/renderer/nodes`).
- Produces:
  - In `src/shared/flaky.ts`:
    - `FLAKY_TITLE`
    - `flakyHeading(env: string | null): string`
    - `flakyCounts(passed: number, failed: number): string`
    - `threadsLabel(n: number | null): string`
    - `orderEnvs(envs: FlakyEnv[], current: string | null): FlakyEnv[]`: the current environment first, then null, then the rest by name.
    - The copy is in the Global Constraints.
  - `AppState.flaky: Record<string, FlakyEnv[]>`, documented as "Proof per flaky test key of the tree shown (F15). Main's `flaky` event follows every tree and every history."
    - `initialState` sets `{}`, and so does the root-changed branch of `project`.
    - `case "flaky"` sets `s.flaky = Object.fromEntries(e.tests.map((t) => [t.key, t.envs]))`.
  - `Tree.tsx` `Node` draws its mark after the tags and before ▶:
    - A scenario or row whose key is in `flaky` shows `<span data-testid="flaky-mark" title={FLAKY_TITLE} aria-label={FLAKY_TITLE}>flaky</span>`.
    - Any other node with `n = leaves(node).filter((l) => l.key in flaky).length > 0` shows `<span data-testid="flaky-count">{n} flaky</span>`.
    - Use amber, with `text-xs` in light and dark mode: `border-amber-500 bg-amber-50 text-amber-800`, and `dark:border-amber-600 dark:bg-amber-950 dark:text-amber-200`.
  - `FlakyProof(props: { envs: FlakyEnv[]; current: string | null; locked: boolean; api: Api })`:
    - One amber `flaky-proof` box per entry of `orderEnvs`, holding the heading, the count line, then one `flaky-run` row per run.
    - Each row shows `whenLabel(startedAt, Date.now())`, the status, `threadsLabel`, and a `flaky-run-report` button.
    - The button is disabled when `locked || !reportAvailable`, and calls `openRunReport(id)`.
  - `Details(props: { state: AppState; api: Api })`. `Leaf` renders `FlakyProof` after the tags and before the error, when `state.flaky[node.key]` exists. It passes `current = state.env?.env ?? null` and `locked = state.run.active || state.busy`. `App.tsx` passes `api`.

- [ ] **Step 1: Write the failing tests.**
  - `src/shared/flaky.test.ts`: the exact strings.
    - `flakyHeading("qa")` and `flakyHeading(null)`.
    - `flakyCounts(1, 1)`, `flakyCounts(3, 2)` and `flakyCounts(1, 2)`.
    - `threadsLabel(1)`, `threadsLabel(8)` and `threadsLabel(null)`.
    - `orderEnvs` of `[dev, null, qa]` with current `"qa"` gives `[qa, null, dev]`, and with current null gives `[null, dev, qa]`.
  - `src/renderer/Tree.test.tsx`:
    - `marks flaky tests and counts them above`: after `flaky` for one scenario and one outline row:
      - those two rows have `flaky-mark`, whose `title` is `FLAKY_TITLE`;
      - the outline shows `1 flaky`;
      - the feature and the root show `2 flaky`;
      - an unflagged scenario shows neither.
    - `counts a flaky test the tag filter hides`: with `hideNonMatching` hiding the flaky scenario, the feature still shows `1 flaky`.
    - `a new project clears the marks`.
  - `src/renderer/FlakyProof.test.tsx` renders `App` with a fake api, as `History.test.tsx` does, plus `openRunReport: vi.fn(async () => {})`. It loads a project, an `environment` view with env `qa`, selects a test, then sends `flaky`.
    - `orders the boxes and words them`: envs `dev`, `null` and `qa` give headings in the order `Flaky on qa`, `Flaky with no environment`, `Flaky on dev`, each with its count line.
    - `lists the proof runs`: rows newest first, with `data-status` and the labels `1 thread`, `8 threads` and `—`.
    - `Report opens the run's report, only when it has one and the runner is idle`:
      - with `reportAvailable: false`, the button is disabled;
      - with `{ type: "busy", busy: true }`, all are disabled;
      - a click calls `openRunReport(id)`.
    - `a test that is not flaky shows no proof`.
- [ ] **Step 2: Run** `npx vitest run src/shared/flaky.test.ts src/renderer/Tree.test.tsx src/renderer/FlakyProof.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npx vitest run src/shared src/renderer`, then `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): mark flaky tests in the tree and show their proof`.

### Task 5: Flaky sample project, end-to-end, backlog row

**Files:**
- Create:
  - `test/fixtures/projects/flaky/karate-config.js`
  - `test/fixtures/projects/flaky/karate-config-qa.js`
  - `test/fixtures/projects/flaky/tests/coin.feature`
  - `e2e/flaky.spec.ts`
- Modify: `test/fixtures/copy.ts`, `e2e/helpers.ts`, `docs/features.md` (the F15 row goes from `specced` to `done`)

**Interfaces:**
- Consumes: everything above.
- Produces:
  - Both config files are `function fn() {\n  return {};\n}`.
  - `coin.feature` is exactly the block below, so `coin` is `[1:3]` (key `s:tests/coin.feature#1`) and `steady` is `[2:7]`:
    ```gherkin
    Feature: coin

    Scenario: coin
      * def coin = read('file:../coin.json')
      * match coin.heads == true

    Scenario: steady
      * match 1 == 1
    ```
    Probed on 2026-10-27 with Karate 2.1.4: `file:../` resolves from the working directory, the project root.
  - `copyFixture("flaky")` copies into `<mkdtemp>/project` and returns that path, so `coin.json` can sit next to it, outside the project. Document it with the other fixtures.
  - `flakyProject(): { dir: string; coin(heads: boolean): void }` in `e2e/helpers.ts`:
    - It pushes the parent folder to `dirs`, for cleanup.
    - `coin` writes `{ "heads": <bool> }` to `path.join(path.dirname(dir), "coin.json")`.
    - Add `"flaky"` to `project()`'s name union only if a test needs it.

- [ ] **Step 1: Write the tests** in `e2e/flaky.spec.ts`, in `history.spec.ts`'s style, with `test.setTimeout(120_000)` each. A local `seedCoin(userData, dir, { report })` writes two records with `createRunDir` and `writeRunRecord`, both with fingerprint `"f".repeat(64)` and env null, and returns their ids `{ failed, passed }`:
  - the older one has `coin` `failed` and 2 threads;
  - the newer one has `coin` `passed` and 1 thread;
  - `steady` passes in both.

  The tests are the spec's Gherkin:
  - `A test that flips with no change is marked flaky`. This test runs Karate for real:
    1. Set Threads to 2 and call `coin(false)`, then Run all. `coin` fails.
    2. Call `coin(true)`, set Threads to 1, then click the toolbar Re-run failed.
    3. `coin` has `flaky-mark`. `coin.feature` and `tests` show `1 flaky`. `steady` has no mark.
    4. Select `coin`. One `flaky-proof` holds `Flaky with no environment` and `Passed 1 time, failed 1 time with the same project files`.
    5. The two `flaky-run` rows read `passed` with `1 thread` (newest), then `failed` with `2 threads`.
  - `A fix in the project is not flaky`. Real runs on the `history` project:
    1. Run all. `fixable` fails.
    2. Write `{ "fixed": true }` to `flag.json`, then Re-run failed. `fixable` passes.
    3. `fixable` has no mark.
  - `Different environments are not compared`. Real runs:
    1. Call `coin(false)` and run `coin` with no environment. It fails.
    2. Call `coin(true)`, then `chooseEnv(…, "qa")` and run `coin`. It passes.
    3. `coin` has no mark.
  - `The proof opens a run's report`: `seedCoin` with `report: true`, open the project, and `stubOpenPath`. Select `coin`, click Report on the `failed` row. The recorded path is `<projectDataDir>/runs/<failed>/karate-summary.html`.
  - `The mark stays on its test when the tree changes`: `seedCoin`, open, then prepend `Scenario: added\n  * match 1 == 1\n\n` above `Scenario: coin` and click Refresh. `coin` has the mark, and `added` has none.
  - `Marks survive a restart`: `seedCoin`, open, close, then launch again on the same `userData`. `coin` has the mark.
  - `Clear history removes the marks`: `seedCoin`, open, then Clear history and confirm. There are 0 `flaky-mark` and 0 `flaky-count`.
- [ ] **Step 2: Run** `npm run package && npx playwright test e2e/flaky.spec.ts`. Expected: PASS. A failure here is a bug in Tasks 1–4: fix it there and keep these tests as written.
- [ ] **Step 3: Run** the full suites one after the other, never together: `npm test`, then `npm run e2e`. Expected: PASS.
- [ ] **Step 4: Set** the F15 row in `docs/features.md` to `done`.
- [ ] **Step 5: Commit** `test(e2e): cover flaky tests and mark f15 done`.
