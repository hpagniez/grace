---
status: built
issue: 229
summary: "A test whose karate-config.js failed shows as failed, a banner links the file and line, and failures sharing a message are grouped in Details"
---

# Group failures that share an error: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A test whose `karate-config.js` failed shows as failed; a banner says so once with a link to the file and line; failed tests that share Karate's message are grouped in Details with Previous, Next and Go to first.

**Architecture:** Main marks config-failed tests failed from the depth-0 `FEATURE_EXIT` and sends, with every failed test, the offset where Karate's reason starts in its error. The window groups the failed tests the tree shows in one pure module (`src/renderer/failures.ts`), only while no run goes on, and derives the banner from the last run's failed keys. A new store action `reveal(key)` selects a test and makes the tree open its ancestors and scroll to it.

**Tech Stack:** Electron, SolidJS 2 RC, TypeScript, Tailwind v4, Vitest 5 (jsdom for `*.test.tsx`), Playwright 1.64.

**Spec:** [docs/specs/2026-11-04-group-failures.md](../specs/2026-11-04-group-failures.md)

## Global Constraints

- Solid 2 rules of `AGENTS.md` (`@solidjs/web`, `createEffect(compute, apply)`, `flush()`, no `classList`, `onClick`).
- UI conventions: semantic tokens only, `Icon` with `lucide` data, `Tooltip` for every tooltip, classes from `buttons.ts` and `ui.ts`; `palette.test.ts` and `tokens.test.ts` stay green.
- Idle costs nothing: no timer, poll or listener; no grouping while `state.run.active`.
- Exact copy: "`n` of `N` tests failed with this error", "Previous test with this error", "Next test with this error", "Failed with the same error", "`N` tests", "Go to first", "Go to first of `N` tests", "`<file>` failed for `N` tests" ("1 test" singular), "Open `<file>` in VS Code", "Open in VS Code".
- Conventional Commits, subject at most 72 characters, scope from `runner`, `ui`, `ipc`, `docs`; each commit ends with the Claude Code attribution lines.
- Run single test files only (`npx vitest run <file>`); a full `npm test` or `npm run e2e` waits for the parent session's go.

## Review Focus

1. A failed `SCENARIO_EXIT` in one `apply` batch and its `FEATURE_EXIT` in a later batch: the test must not be counted failed twice (Task 1 test "does not fail a test twice across batches").
2. A failed test with no error (Karate sent none): it joins no group and is no config failure (Task 2 test "leaves out a failed test with no error").
3. A rerun of some failed tests after a config failure: the banner counts only that run's failures (Task 5 test "counts only the last run's failures").
4. Next or Go to first on a test whose feature the person closed: the tree opens it and shows the row (Task 3 test "reveal opens a closed feature").
5. Outline rows of a config failure: `FEATURE_EXIT` gives them their row `refId` (`[2.1:9]`), and they turn failed like scenarios (Task 1 test "fails an outline row whose config failed").

---

### Task 1: Main marks config-failed tests failed and sends Karate's reason

**Files:**
- Modify: `src/shared/events.ts` (`ScenarioUpdate`)
- Modify: `src/main/karate/run-state.ts` (`apply`)
- Modify: `src/main/karate/steps.ts` (`extractSteps`)
- Test: `src/main/karate/run-state.test.ts`, `src/main/karate/steps.test.ts`

**Interfaces:**
- Produces: `ScenarioUpdate.reasonAt?: number`, documented "Where Karate's reason starts in `error` (#229 R2.1); none: the whole error is the reason."

- [ ] **Step 1: Write the failing tests** in `run-state.test.ts`, new `describe("RunState: config failures (#229)")`. The config-fail fixture has no dry run: build its tree by hand with `node`-style literals, a feature `f:tests/shared.feature` holding scenarios `s:tests/shared.feature#1` (`file: "tests/shared.feature"`, `refId: "[1:7]"`, name `first`) and `s:tests/shared.feature#2` (`refId: "[2:10]"`, name `second`), and `new RunState(tree, leafKeys(tree))`.
  - `fails the tests whose config failed`: `apply(events("run-http-config-fail.jsonl"))` gives a `scenarios` event with both keys `status: "failed"`, `durationMillis: 0`, `reasonAt: 0`, and `error` starting `"Config evaluation failed: karate-config.js - js failed:"`; then `finish(1, false, true)` has no `notice` event, and `results()` gives both `"failed"`.
  - `does not fail a test twice across batches`: feed `run-clean.jsonl` one event per `apply` call; exactly one `failed` update in total for `s:tests/basic.feature#2`.
  - `fails an outline row whose config failed`: a hand-made depth-0 `FEATURE_EXIT` (`relativePath: "tests/shared.feature"`, `callDepth: 0`, one scenario result `{ refId: "[2.1:9]", failed: true, error: "Config evaluation failed: karate-config.js - boom", durationMillis: 0, stepResults: [] }`) against a tree with a row of that `refId`: the row's update is `failed` with that error.
  - `sends where Karate's reason starts`: in `run-clean.jsonl`, the failed update's `error.slice(reasonAt)` equals the fixture's `errorReason` (`"match failed: EQUALS\n  $ | not equal (STRING:STRING)\n  'hello'\n  'goodbye'\n"`); a hand-made failed `SCENARIO_EXIT` whose `error` does not end with its `errorReason` gets no `reasonAt`.
  - In `steps.test.ts`, `shows a step with no keyword by its text alone`: the first scenario of the depth-0 `FEATURE_EXIT` of `run-http-config-fail.jsonl` has one step, `keyword: "*"`, `line: 0`, `status: "failed"`, and `text` starting `"Scenario execution failed: Config evaluation failed: karate-config.js"`.

- [ ] **Step 2: Run them to see them fail**
  Run: `npx vitest run src/main/karate/run-state.test.ts src/main/karate/steps.test.ts`
  Expected: the five new tests FAIL (no `reasonAt`, statuses `notRun`, text starting `undefined `).

- [ ] **Step 3: Implement**
  - `SCENARIO_EXIT` of a failed test: besides `error`, set `reasonAt = error.length - errorReason.length` when `errorReason` is a string and `error.endsWith(errorReason)`.
  - `FEATURE_EXIT`: a loop over `d.scenarioResults ?? []`; for each with `failed === true` whose `keyOf(d.relativePath, sc.refId)` is tracked and whose `this.status` is neither `"passed"` nor `"failed"`: put `{ key, status: "failed", durationMillis, error, reasonAt: 0 }` (error and `reasonAt` only when `sc.error` is a string) into the same `updates` map, and set `this.status` and `this.duration`.
  - `extractSteps`: `text` is `r.step.text` alone when `r.step.keyword` is missing.

- [ ] **Step 4: Run them to see them pass**
  Run: `npx vitest run src/main/karate/run-state.test.ts src/main/karate/steps.test.ts src/main/karate/http.test.ts`
  Expected: PASS.

- [ ] **Step 5: Commit** `fix(runner): mark tests failed when karate-config.js fails`, body `Part of #229.`

### Task 2: The window keeps the reason and groups failures

**Files:**
- Modify: `src/renderer/reduce.ts` (`AppState.results`, `scenarios` case)
- Create: `src/renderer/failures.ts`
- Test: `src/renderer/failures.test.ts` (Node), `src/renderer/reduce.test.ts`

**Interfaces:**
- Consumes: `ScenarioUpdate.reasonAt` (Task 1).
- Produces, in `failures.ts`:
  - `type Result = AppState["results"][string]` (`{ durationMillis?: number; error?: string; reasonAt?: number }`)
  - `reasonOf(r: Result | undefined): string | null`: `error.slice(reasonAt ?? 0)`, null without an error.
  - `sameError(reason: string): string`: drops every line `^[ \t]*Line: \d+, Col: \d+\r?$` and a last line `at <n>:<m>` (`/\r?\n[ \t]*at \d+:\d+\s*$/`); the comparison key of R2.2.
  - `shownLeaves(n: TestNode, f: FilterState): TestNode[]`: `matchingLeaves(n, f)` when `f.hideNonMatching && filterOn(f)`, else `leaves(n)`; tree order.
  - `interface FailureGroup { keys: string[] }` and `failureGroups(tests: TestNode[], statuses: Record<string, Status>, results: Record<string, Result>): FailureGroup[]`: the failed tests with a reason, by `sameError(reasonOf(...))`, groups of 2 or more, in the tree order of their first tests, keys in tree order.
  - `interface ConfigFailure { file: string; line: number; reason: string; count: number }` and `configFailure(tests: TestNode[], failedKeys: readonly string[], results: Record<string, Result>): ConfigFailure | null`: over `tests` (tree order) that are in `failedKeys` and whose reason starts with `CONFIG = "Config evaluation failed: "`; `reason` of the first, `count` of all; `file` = text after `CONFIG` up to the next `" - "` (to the end of the first line when none), less a leading `classpath:`; `line` = number after the first `Line: `, else 1.

- [ ] **Step 1: Write the failing tests** in `failures.test.ts` (plain `TestNode` literals, keys `a`, `b`, `c`, `d` in tree order):
  - `groups failures at different places with one message`: `a` and `b` reason `"expected status: 200, actual: 503"` (errors with different `C:\p\x.feature:5` / `C:\p\y.feature:9` heads and `reasonAt`), `c` reason `"expected status: 200, actual: 404"` → `[{ keys: ["a", "b"] }]`.
  - `leaves out a failed test with no error`, and a passed test with an error: neither is in any group.
  - `groups karate.fail at different lines`: reasons `"js failed:\n==========\n  Line: 10, Col: 0\n  Code: karate.fail('no vpn')\n  Error: no vpn\n=========="` and the same with `Line: 7, Col: 0` → one group.
  - `groups a thrown string at different positions`: `"bad\n    at 15:1"` and `"bad\n    at 3:5"` → one group.
  - `orders groups by their first test`: `a`, `c` share one reason, `b`, `d` another → `[["a","c"], ["b","d"]]`.
  - `shownLeaves` with `hideNonMatching` on and a filter matching `a`, `b` only → `[a, b]`; with it off → all four.
  - `configFailure`: the fixture's reason (`"Config evaluation failed: karate-config.js - js failed:\n==========\n  Line: 4, Col: 2\n…"`) for `a`, `b` in `failedKeys` → `{ file: "karate-config.js", line: 4, count: 2, reason }`; `"Config evaluation failed: classpath:karate-config-dev.js - boom"` → `file: "karate-config-dev.js"`, `line: 1`; `count` leaves out failed tests not in `failedKeys`; null with no such reason.
  - In `reduce.test.ts`: a `scenarios` update with `reasonAt: 7` puts `reasonAt: 7` in `results[key]`, and `runStarted` for that key clears it.

- [ ] **Step 2: Run them to see them fail**
  Run: `npx vitest run src/renderer/failures.test.ts src/renderer/reduce.test.ts`
  Expected: FAIL (module missing; `reasonAt` not kept).

- [ ] **Step 3: Implement** `failures.ts` with the signatures above, and in `reduce.ts` add `reasonAt?: number` to the results type and copy `u.reasonAt` as `durationMillis` is copied.

- [ ] **Step 4: Run them to see them pass** (same command). Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): group failed tests by Karate's reason`, body `Part of #229.`

### Task 3: Step through a group from a failed test

**Files:**
- Modify: `src/renderer/reduce.ts` (`AppState.reveals: number`, 0 in `initialState`)
- Modify: `src/renderer/store.ts` (`reveal(key: string): void`)
- Modify: `src/renderer/App.tsx` (pass `reveal` to `Details`)
- Modify: `src/renderer/Tree.tsx` (show the revealed test)
- Modify: `src/renderer/Details.tsx` (`Details`, `Leaf`, `StepRow`)
- Test: `src/renderer/Details.test.tsx`, `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: `shownLeaves`, `failureGroups`, `FailureGroup` (Task 2).
- Produces: `reveal(key)` in the store: sets `selected = key`, `selectedCall = null`, and `reveals += 1`. `Details` props gain `reveal(key: string): void`. `Details` holds `groups = createMemo(() => state.run.active || !state.tree ? [] : failureGroups(shownLeaves(state.tree, state.filter), state.statuses, state.results))` and hands it to `Leaf`; `Group` gets `reveal` (Task 4).

- [ ] **Step 1: Write the failing tests.** In `Details.test.tsx`, a new `describe("details: step through a group (#229 R4)")` using `setup()` with a tree of tests `a`, `b`, `c`, `d` (one feature), a finished run where `a`, `b`, `c` failed with error `"C:\\p\\x.feature:N\nThen status 200\nexpected status: 200, actual: 503"` (N differs) and `reasonAt` at the message, and `d` passed:
  - `says where the test sits in its group`: select `a` → `getByTestId("error-group")` text `"1 of 3 tests failed with this error"`; `details-error` text is still the whole error.
  - `Next selects the next test and keeps the focus`: focus and click "Next test with this error" → `b`'s row has `aria-selected="true"`, the text reads `"2 of 3 …"`, and `document.activeElement` is the button named "Next test with this error".
  - `Previous wraps round`: from `a`, "Previous test with this error" selects `c`.
  - `a lone failure has no stepping`: `d` fails alone with another reason → no `error-group`.
  - `no stepping while a run goes on`: a new `runStarted` of `d` → selecting `a` shows no `error-group`.
  - `has no breakpoint dot on a step below line 1`: steps event for `a` with a step `line: 0` → that step has no `breakpoint` button.
  In `Tree.test.tsx`: `reveal opens a closed feature`: with the feature of `b` closed (a fresh load, no `openFeatures()`), pressing Next from a selected failed test in another, open feature makes `rowByKey(b)` present with `aria-selected="true"` (use a two-feature tree; select the first test by clicking it after opening only its feature by its chevron).

- [ ] **Step 2: Run them to see them fail**
  Run: `npx vitest run src/renderer/Details.test.tsx src/renderer/Tree.test.tsx`
  Expected: the new tests FAIL.

- [ ] **Step 3: Implement**
  - Store `reveal`; `App` passes it to `Details` as `reveal`.
  - `Tree`: a function `show(key)` that opens every closed ancestor (`ancestors(tree, key)`) and bumps the reveal cue (today's `pauses` signal, renamed `reveals`); the pause effect calls it with the paused key, and a new effect on `state.reveals` calls it with `state.selected`.
  - `Leaf`: under the `pre` in the red box, when `groups()` has a group holding the key: a `div data-testid="error-group"` (`mt-2 flex items-center gap-1 text-xs text-ink`): the count text, then two `btnIconSmall` buttons with `ChevronUp` and `ChevronDown` (size 14), `aria-label`s "Previous test with this error" and "Next test with this error", each in a `Tooltip` with the same text; they call `reveal(keys[(i + dir + n) % n])`.
  - `StepRow`: the breakpoint button only when `step.line >= 1`; otherwise a `size-4 shrink-0` spacer keeps the row aligned.

- [ ] **Step 4: Run them to see them pass** (same command, plus `src/renderer/palette.test.ts`). Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): step through tests that failed with one error`, body `Part of #229.`

### Task 4: A folder's failures by error

**Files:**
- Modify: `src/renderer/Details.tsx` (`Group`)
- Test: `src/renderer/Details.test.tsx`

**Interfaces:**
- Consumes: `failureGroups`, `shownLeaves`, `reasonOf` (Task 2); `reveal` (Task 3).

- [ ] **Step 1: Write the failing tests** in a new `describe("details: a folder's failures by error (#229 R5)")`, tree: folder `api` with features holding 6 tests and one test outside `api`; a finished run where in `api` 3 failed with reason `"timeout"`, 2 with `"503"`, 1 with `"404"`, and the test outside `api` failed with `"timeout"`:
  - `lists the folder's groups, largest first`: select `api` → heading "Failed with the same error"; `getAllByTestId("failure-group")` texts in order contain `"3 tests"` + `"timeout"`, then `"2 tests"` + `"503"`; no `"404"`.
  - `Go to first selects the group's first test`: click "Go to first of 3 tests" → the first `timeout` test of `api` in tree order is selected (its row `aria-selected="true"`, its name in the `h2`).
  - `shows no list without a group`: a folder whose failures all differ → no "Failed with the same error".

- [ ] **Step 2: Run them to see them fail**
  Run: `npx vitest run src/renderer/Details.test.tsx`
  Expected: the new tests FAIL.

- [ ] **Step 3: Implement** in `Group` (props gain `reveal`): a memo of `failureGroups(shownLeaves(node, filter), …)` sorted by size, largest first (a stable sort keeps tree order), empty while a run goes on. Under the counts: `h3` "Failed with the same error" (`text-sm font-semibold`), then a `ul` of `li data-testid="failure-group"`: `"N tests"` (`tabular-nums`), the first test's reason in a `pre` (`box`, `line-clamp-4 whitespace-pre-wrap break-words`), and a `ghostSmall` "Go to first" button with `aria-label` "Go to first of N tests" calling `reveal(keys[0])`.

- [ ] **Step 4: Run them to see them pass** (same command). Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): list a folder's failures by error`, body `Part of #229.`

### Task 5: Say once that the config failed

**Files:**
- Create: `src/renderer/ConfigBanner.tsx`
- Modify: `src/renderer/App.tsx` (under the notice box in the sheet)
- Test: `src/renderer/ConfigBanner.test.tsx`

**Interfaces:**
- Consumes: `configFailure` (Task 2); `api.openInEditor(path, line)`.
- Produces: `ConfigBanner(props: { state: AppState; api: Api })`.

- [ ] **Step 1: Write the failing tests** with `App` and a fake `api` (as `Details.test.tsx`'s `setup()`, plus `openInEditor: vi.fn(async () => {})`):
  - `names the file and its line`: a finished run where `first` and `second` failed with the fixture's reason and `reasonAt: 0` → `getByRole("alert")` has `"karate-config.js failed for 2 tests"` and the reason in `config-failure-reason`; clicking "Open karate-config.js in VS Code" calls `openInEditor("karate-config.js", 4)`.
  - `opens line 1 when the message has none`: reason `"Config evaluation failed: classpath:karate-config-dev.js - boom"`, one test → `"karate-config-dev.js failed for 1 test"`, `openInEditor("karate-config-dev.js", 1)`.
  - `goes when the next run starts`: `runStarted` → no `config-failure`.
  - `counts only the last run's failures`: after the 2-test config failure, a run of `first` alone that fails the same way → `"failed for 1 test"`.
  - `shows nothing for ordinary failures`.
  - `sits under the notice`: with a `notice` event too, `notice` comes before `config-failure` in document order.

- [ ] **Step 2: Run them to see them fail**
  Run: `npx vitest run src/renderer/ConfigBanner.test.tsx`
  Expected: FAIL (module missing).

- [ ] **Step 3: Implement** `ConfigBanner`: a memo `state.run.active || !state.tree ? null : configFailure(leaves(state.tree), state.run.failedKeys, state.results)`; when set, `div class="px-3 py-2"` holding `div role="alert" data-testid="config-failure"` with `rounded-lg p-3 ${TINT.failed}`: a first line (`flex items-center gap-1.5 text-sm font-semibold`) with `Icon CircleAlert` size 14, the text in `text-ink`, and a `ghostSmall` button (`ExternalLink` size 14, "Open in VS Code", `aria-label` "Open `<file>` in VS Code"); then `pre data-testid="config-failure-reason"` (`mt-1 max-h-60 overflow-auto font-mono text-xs whitespace-pre-wrap break-words text-ink`). In `App`, render it right after `<Show when={!settingsOpen()}>{noticeBox()}</Show>` in the sheet.

- [ ] **Step 4: Run them to see them pass**
  Run: `npx vitest run src/renderer/ConfigBanner.test.tsx src/renderer/palette.test.ts`
  Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): say once when karate-config.js fails`, body `Part of #229.`

### Task 6: End to end, and the pull request

**Files:**
- Create: `e2e/failures.spec.ts`

- [ ] **Step 1: Write the e2e test** `A broken karate-config.js, end to end`: `tempDir("kk-config-")` with `karate-config.js` = `"function fn() {\n  karate.fail('no vpn');\n  return {};\n}\n"`, `a.feature` with two scenarios and `b.feature` with one, each `* print 'x'`. `launch()`, stub VS Code as `editor.spec.ts` does (`stubVsCode`), `openProject`, click `run-all-button`, wait for `run-outcome` "Finished" (30 s). Expect the three rows failed (`data-status` or the run's counts), the alert text `"karate-config.js failed for 3 tests"`; click "Open karate-config.js in VS Code" → `vscode://file/<dir with />/karate-config.js:2` recorded. Select the first test of `a.feature`; `error-group` reads `"1 of 3 tests failed with this error"`; click "Previous test with this error" → the test of `b.feature` has `aria-selected="true"` and is visible.

- [ ] **Step 2: Ask the parent session for a go, then run** `npm run typecheck`, `npm run lint`, `npm test`, `npm run e2e`. Expected: all pass; tell the parent when done.

- [ ] **Step 3: Commit** `test(e2e): cover a broken karate-config.js`, body `Part of #229.`

- [ ] **Step 4: Push and open a draft PR** into `develop` from `feature/group-failures`, body with `Closes #229`, the spec link, the e2e summary, and the attribution line. Read the `perf` comment when it posts and explain any measure worse than base beyond its noise margin.
