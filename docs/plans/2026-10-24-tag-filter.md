# Tag Filter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-project tag filter: chips that cycle off → include → exclude, a tree that dims or hides non-matching tests, and runs that pass the filter to Karate as `-t`.

**Architecture:** One pure module `src/shared/tags.ts` decides matching, used by the window (for display) and by the main process (for runs). The main `Runner` owns the filter: it loads it per project from `settings.json`, prunes vanished tags after Open and Refresh, saves changes, sends it to the window as a new `filter` event, and builds `-t` and the run targets from it. The window renders a filter bar from the tree's tags and the filter, and asks main to change the filter over a new IPC call.

**Tech Stack:** Electron 45, Solid 2 RC, Tailwind v4, TypeScript, Vitest 5, Playwright 1.64, Karate 2.1.4 (bundled).

**Spec:** [docs/specs/2026-10-24-tag-filter.md](../specs/2026-10-24-tag-filter.md). Read it first; requirement ids (R1–R6) below refer to it. It builds on the [walking skeleton spec](../specs/2026-10-24-walking-skeleton.md).

## Global Constraints

- Branch: `feature/tag-filter`, cut from `develop` after the spec branch `feature/tag-filter-spec` is merged. Conventional Commits; rules and Solid 2 rules in [AGENTS.md](../../AGENTS.md).
- No new dependencies.
- Tags are stored and compared **without** `@`, case-sensitive, exactly as `TestNode.tags` holds them. The UI shows them with `@`.
- Matching (R2): a scenario or row matches when (`include` is empty or one of its tags is in `include`) and none of its tags is in `exclude`. A container matches when any leaf under it matches.
- `-t` form: all included tags in one `-t @a,@b`; each excluded tag in its own `-t ~@c`. No `-t` when the filter is off.
- Pruning of vanished tags happens only after a successful Open or Refresh discovery (R5), never in the pre-run discovery.
- Never run unit tests and e2e tests at the same time (AGENTS.md).
- Exact UI copy: counter `"<m> of <n> tests match"`; notices `"This test no longer matches the filter"`, `"Clear the filter to run this outline"`, `"Nothing to run"` (existing), `"Karate ran <n> tests, more than the <total> selected"`; toggle label `"Hide non-matching"`; clear control `"Clear"`.

## Review Focus

1. **Tags differing only in case** (`@Smoke` and `@smoke`): Karate treats them as different; two chips, matched separately. Test in Task 1 (`matching is case-sensitive`).
2. **A malformed or old `settings.json`** (no `projects` key, `include` not an array, non-string entries): reads as no filter, never crashes the open. Tests in Task 1 (`parseFilterState rejects bad shapes`) and Task 3 (`a malformed entry reads as no filter`).
3. **A forged or oversized `setFilter` IPC payload**: main rejects it with "Invalid request" and keeps the old filter. Validation via `parseFilterState` (Task 1), wired in Task 5; test in Task 1 (`parseFilterState rejects bad shapes`).
4. **A filter change while a run is going**: rejected with the busy notice, the run keeps the filter it started with. Test in Task 5 (`rejects a filter change while busy`).
5. **A tag set only on the feature line** (`@feat` on `Feature:`, scenarios untagged): it is a chip and every scenario of the feature matches it. Test in Task 5 (`lists inherited feature and examples tags`).

---

### Task 1: Shared tag matching

**Files:**
- Create: `src/shared/tags.ts`, `src/shared/tags.test.ts`

**Interfaces:**
- Produces (all exported from `src/shared/tags.ts`, no Node imports):
  - `interface TagFilter { include: string[]; exclude: string[] }`
  - `interface FilterState extends TagFilter { hideNonMatching: boolean }`
  - `const NO_FILTER: FilterState` = `{ include: [], exclude: [], hideNonMatching: false }`
  - `filterOn(f: TagFilter): boolean` — any include or exclude
  - `matches(tags: string[], f: TagFilter): boolean` — R2 rule for one leaf
  - `matchingLeaves(n: TestNode, f: TagFilter): TestNode[]` — scenario and row nodes at or under `n` that match, tree order
  - `projectTags(tree: TestNode): string[]` — sorted union of the tags of every scenario and row
  - `chipTags(tree: TestNode | null, f: TagFilter): string[]` — sorted union of `projectTags(tree)`, `f.include` and `f.exclude` (a filtered tag that vanished stays visible until pruned)
  - `cycleTag(f: FilterState, tag: string): FilterState` — off → include → exclude → off, returns a new object
  - `pruneFilter(f: FilterState, tags: string[]): FilterState` — drops include/exclude entries not in `tags`
  - `parseFilterState(x: unknown): FilterState | null` — null unless `include` and `exclude` are arrays of at most 500 non-empty strings of at most 200 chars each, disjoint, and `hideNonMatching` is boolean

- [ ] **Step 1: Write the failing tests** in `src/shared/tags.test.ts`, building a small tree with the same `node()` helper shape as `src/renderer/Tree.test.tsx`, leaves tagged like the clean fixture (`["smoke"]`, `["wip"]`, `[]`, `["smoke","wip"]`, `["slow"]`, `["smoke","slow"]`, 3 untagged rows):
  - `empty filter matches everything` — `matchingLeaves(tree, NO_FILTER).length === 9`, `filterOn(NO_FILTER) === false`
  - `include smoke exclude wip` — names of `matchingLeaves` equal `["passing hello", "smoke and slow"]`
  - `exclude only keeps untagged tests` — `{include:[], exclude:["wip"]}` gives 7
  - `included tags combine with OR` — `{include:["wip","slow"]}` gives `failing match`, `smoke but wip`, `slow loop`, `smoke and slow`
  - `matching is case-sensitive` — `matches(["Smoke"], {include:["smoke"], exclude:[]}) === false`
  - `projectTags is sorted and unique` — `["slow","smoke","wip"]`
  - `chipTags keeps a filtered tag the tree lost` — `chipTags(tree, {include:["gone"], exclude:[]})` contains `"gone"`
  - `cycleTag goes off, include, exclude, off` — three calls from `NO_FILTER` on `"smoke"`
  - `pruneFilter drops unknown tags and keeps hideNonMatching`
  - `parseFilterState rejects bad shapes` — `null` for `undefined`, `{}`, `{include:"a",exclude:[],hideNonMatching:false}`, `{include:[1],...}`, `{include:[""],...}`, a 501-entry array, a tag in both lists, `hideNonMatching: "yes"`; returns the value for a valid object

- [ ] **Step 2: Run** `npx vitest run src/shared/tags.test.ts` — expected: FAIL, module not found.

- [ ] **Step 3: Implement `src/shared/tags.ts`** with the interfaces above. `matchingLeaves` uses the same leaf rule as `leafKeys` in `src/main/karate/tree.ts` (`kind` is `scenario` or `row`).

- [ ] **Step 4: Run** `npx vitest run src/shared/tags.test.ts` — expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): add shared tag filter matching`

### Task 2: `-t` arguments and filtered run targets

**Files:**
- Modify: `src/main/karate/command.ts` (`selectionTargets`, `buildArgs`)
- Test: `src/main/karate/command.test.ts`

**Interfaces:**
- Consumes: `TagFilter`, `filterOn`, `matchingLeaves` (Task 1)
- Produces:
  - `tagArgs(f: TagFilter): string[]` — `[]` when off; else `["-t", "@a,@b", "-t", "~@c", "-t", "~@d"]` (include group first, omitted when `include` is empty)
  - `selectionTargets(node: TestNode, root: string, featureRoots: string[], f: TagFilter): string[] | null` — as today when the filter is off or the node is a suite, folder, feature, scenario or row. For an outline under a filter: all rows match → `[abs:outlineLine]`; some match → `[abs:L1:L2…]` with the matching rows' lines in tree order; a matching row with `line === null` → `null`.
  - `buildArgs` gains `tags: string[]`, inserted after `-f html,karate:jsonl` and `-D`, before the targets.

- [ ] **Step 1: Write the failing tests:**
  - `tagArgs` — off → `[]`; `{include:["smoke","api"], exclude:["wip","slow"]}` → `["-t","@smoke,@api","-t","~@wip","-t","~@slow"]`; `{include:[], exclude:["wip"]}` → `["-t","~@wip"]`
  - `outline targets under a filter` — an outline with rows at lines 9, 10, 11 tagged `fast`, `fast`, `slow`: `{include:["fast"]}` → `["<abs>:9:10"]`; `{include:["fast","slow"]}` → `["<abs>:<outlineLine>"]`; the same with row 9's `line: null` → `null`
  - extend `builds the exact command line` with `tags: ["-t","@smoke"]` and assert they sit just before the targets
  - existing `maps each node kind` passes `NO_FILTER` and still gets today's values

- [ ] **Step 2: Run** `npx vitest run src/main/karate/command.test.ts` — expected: FAIL.

- [ ] **Step 3: Implement** `tagArgs`, the new `selectionTargets` parameter and return type, and the `buildArgs` `tags` field. Fix the one caller in `src/main/runner.ts` so it compiles (`NO_FILTER`, `[]`); Task 5 wires the real filter.

- [ ] **Step 4: Run** `npx vitest run src/main/karate/command.test.ts` and `npm run typecheck` — expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): build tag arguments and filtered run targets`

### Task 3: Per-project filter in settings

**Files:**
- Modify: `src/main/settings.ts`
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Consumes: `FilterState`, `NO_FILTER`, `parseFilterState` (Task 1)
- Produces:
  - `loadProjectFilter(dataDir: string, root: string): FilterState` — `NO_FILTER` when the file, the `projects` map or the entry is missing or fails `parseFilterState`
  - `saveProjectFilter(dataDir: string, root: string, f: FilterState): void`
  - `saveLastProject` keeps the rest of the file. File shape exactly as the spec's "Settings file" section; keys are the absolute project path as given.

- [ ] **Step 1: Write the failing tests:**
  - `filter round-trips per project` — save for two roots, load each back
  - `saveLastProject keeps saved filters` and `saveProjectFilter keeps lastProject`
  - `a malformed entry reads as no filter` — write `{"projects":{"<root>":{"include":"smoke"}}}` and a file that is not JSON; both load `NO_FILTER`

- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts` — expected: FAIL.

- [ ] **Step 3: Implement** with one private read-modify-write helper; a read failure starts from `{}`.

- [ ] **Step 4: Run** `npx vitest run src/main/settings.test.ts` — expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): save the tag filter per project`

### Task 4: Filtered run selection and over-run notice

**Files:**
- Modify: `src/main/karate/run-state.ts`
- Test: `src/main/karate/run-state.test.ts`

**Interfaces:**
- Consumes: `TagFilter`, `matchingLeaves` (Task 1)
- Produces: `new RunState(tree: TestNode, selection: TestNode, filter: TagFilter)`; `keys` = keys of `matchingLeaves(selection, filter)`. `finish()` adds, when the outcome is `finished` and `summary.scenarioCount > keys.length`, the notice `Karate ran ${scenarioCount} tests, more than the ${keys.length} selected`.

- [ ] **Step 1: Write the failing tests** (existing `newState` passes `NO_FILTER`):
  - `keys are the matching leaves` — `new RunState(tree, tree, {include:["smoke"], exclude:["wip"], ...})` has 2 keys
  - `warns when Karate ran more than selected` — state with `{include:["smoke"], exclude:["wip"]}` fed `run-clean.jsonl` (9 scenarios) then `finish(1, false, true)` contains the notice `Karate ran 9 tests, more than the 2 selected`
  - `no over-run notice for a matching count` — the unfiltered state fed `run-clean.jsonl` has no notice

- [ ] **Step 2: Run** `npx vitest run src/main/karate/run-state.test.ts` — expected: FAIL.

- [ ] **Step 3: Implement**, and pass `NO_FILTER` at the caller in `runner.ts` until Task 5.

- [ ] **Step 4: Run** `npx vitest run src/main/karate/run-state.test.ts` — expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): select matching tests and warn on extra ones`

### Task 5: Runner owns the filter; IPC

**Files:**
- Create: `test/fixtures/projects/tags/tests/tags.feature`
- Modify: `test/fixtures/copy.ts` (name union adds `"tags"`), `src/main/runner.ts`, `src/shared/events.ts`, `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`
- Test: `src/main/runner.test.ts` (Java), extend its `fixture()` union with `"tags"`

**Interfaces:**
- Consumes: Tasks 1–4
- Produces:
  - `AppEvent` gains `{ type: "filter"; filter: FilterState }`
  - `CH.filter = "filter:set"`; `Api.setFilter(f: FilterState): Promise<void>`
  - `Runner.setFilter(f: FilterState): void` — busy → the existing busy notice, filter unchanged; no project → ignored; else store, `saveProjectFilter`, emit `filter`
  - `Runner.replay()` also returns the current `filter` event
- Behaviour in `runner.ts`:
  - `open`: load the filter with `loadProjectFilter` for the new root before discovery; a cancelled open restores the previous filter. `rediscover` (Open and Refresh) after a discovery that produced a tree: `pruneFilter(filter, projectTags(tree))`, save if it changed, emit `filter`.
  - `runGuarded`: after resolving the node, if `filterOn` and the node is a scenario or row that does not match → notice `This test no longer matches the filter`, no run. `selectionTargets(..., filter)` returning `null` → notice `Clear the filter to run this outline`. `matchingLeaves(node, filter).length === 0` → `Nothing to run`. Spawn with `tags: tagArgs(filter)`, `new RunState(tree, node, filter)`.
  - `main.ts`: `handle(CH.filter, (f) => …)` rejects with "Invalid request" unless `parseFilterState(f)` is non-null; the `CH.open` handler is unchanged.

Fixture `test/fixtures/projects/tags/tests/tags.feature` (4 tests, all pass):

```gherkin
@feat
Feature: tagged outline

@other
Scenario: other
  * match 1 == 1

@ol
Scenario Outline: speed <n>
  * match <n> == <n>

  @fast
  Examples:
    | n |
    | 1 |
    | 2 |

  @slow
  Examples:
    | n |
    | 3 |
```

- [ ] **Step 1: Write the failing tests** in `src/main/runner.test.ts`, new `describe("Runner tag filter")`:
  - `lists inherited feature and examples tags` — open `tags`; `projectTags(tree)` equals `["fast","feat","ol","other","slow"]`
  - `runs all with a filter` — open `clean`, `setFilter({include:["smoke"], exclude:["wip"], hideNonMatching:false})`, `run("d:")` → `runStarted.total === 2`, summary `scenarioCount: 2, scenariosPassed: 2`
  - `filters a feature run` — exclude `wip`, run `f:tests/basic.feature` → total 2, `scenarioCount: 2`
  - `runs the matching rows of an outline` — `tags`, include `fast`, run the outline node → total 2, `scenarioCount: 2` (proves `file:L1:L2` on rows)
  - `refuses a clicked test that no longer matches` — include `smoke`, remove `@smoke` from `passing hello` in the file, run its key → notice, no `runStarted`
  - `a vanished include tag runs nothing` — include only `wip`, remove every `@wip` line from `basic.feature`, `run("d:")` → notice `Nothing to run`, no `runStarted`, filter still includes `wip`
  - `refresh prunes vanished tags and saves` — same edit, `refresh()` → last `filter` event has `include: []`, and `loadProjectFilter` agrees
  - `restores the filter on open` — set a filter, `dispose`, new `Runner` on the same `dataDir` opens the same folder → its `filter` event equals the saved one
  - `rejects a filter change while busy` — start `run("f:tests/slow.feature")` without awaiting, wait for `runStarted`, `setFilter(...)` → busy notice, `filter` unchanged
  - `Karate agrees with the matching function` — on `tags`, for `{include:["fast"]}`, `{include:["fast","other"]}`, `{exclude:["slow"]}`: a dry run with `tagArgs(f)` on the feature root (spawned through the same `buildArgs`/`spawnKarate` path, or a `Runner` test helper) selects exactly the keys of `matchingLeaves(tree, f)`

- [ ] **Step 2: Run** `npx vitest run src/main/runner.test.ts -t "tag filter"` — expected: FAIL.

- [ ] **Step 3: Implement** the Runner changes, the event, channel, preload method and `main.ts` handler.

- [ ] **Step 4: Run** `npx vitest run src/main` and `npm run typecheck` — expected: PASS, including the existing Runner tests unchanged.

- [ ] **Step 5: Commit** `feat(runner): own the per-project tag filter and apply it to runs`

### Task 6: Filter bar and filtered tree

**Files:**
- Create: `src/renderer/FilterBar.tsx`, `src/renderer/FilterBar.test.tsx`
- Modify: `src/renderer/reduce.ts` (`AppState.filter: FilterState`, initial `NO_FILTER`, `case "filter"`; a `project` event for another root resets it to `NO_FILTER` until main's `filter` event arrives), `src/renderer/App.tsx` (bar between `Progress` and the notice), `src/renderer/Tree.tsx`, `src/renderer/Toolbar.tsx`
- Test: `src/renderer/store.test.tsx` (reducer case), `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: Task 1 helpers; `Api.setFilter`, `AppEvent` `filter` (Task 5)
- Produces (test ids used by Task 7):
  - bar `data-testid="filter-bar"`, absent when `chipTags(tree, filter)` is empty
  - chip `data-testid="tag-chip"`, `data-tag="<tag>"`, `data-state="off|include|exclude"`, text `@<tag>`; click → `api.setFilter(cycleTag(filter, tag))`
  - `data-testid="tag-clear"` only when `filterOn`; click → `api.setFilter({...NO_FILTER, hideNonMatching})`
  - `data-testid="tag-counter"` only when `filterOn`, text `"<m> of <n> tests match"`
  - checkbox `data-testid="hide-non-matching"`, label `Hide non-matching`; change → `api.setFilter({...filter, hideNonMatching})`
  - every `tree-node` gets `data-match="true|false"`; non-matching: `opacity-40`, no `run-node` button, still selectable (the details pane works as today); hidden entirely when `hideNonMatching && filterOn`
  - chips, Clear and the checkbox are disabled while `state.run.active || state.busy`
  - Toolbar: Run all also disabled when `matchingLeaves(tree, filter).length === 0`

- [ ] **Step 1: Write the failing tests** (jsdom, with a fake `Api` like `Tree.test.tsx`, emitting a `project` event with the clean-like tree then a `filter` event):
  - `shows a chip per tag` — `@slow`, `@smoke`, `@wip`, all `data-state="off"`, no counter
  - `a chip click asks main for the next state` — click `@smoke` → `setFilter` called with `include: ["smoke"]`
  - `dims and hides non-matching tests` — `filter` include `smoke` exclude `wip`: counter `2 of 9 tests match`; `failing match` node has `data-match="false"` and no `run-node`; after a `filter` event with `hideNonMatching: true` it is not in the DOM and `outline.feature` is gone
  - `Run all is disabled when nothing matches`
  - `controls are disabled while busy`
  - reducer: `filter event replaces the filter`; `opening another project resets the filter`

- [ ] **Step 2: Run** `npx vitest run src/renderer` — expected: FAIL.

- [ ] **Step 3: Implement.** Chip colours: include green (`border-green-600 bg-green-100 text-green-800`), exclude red and `line-through` (`border-red-600 bg-red-100 text-red-800`), off neutral zinc, like the toolbar's tag pills.

- [ ] **Step 4: Run** `npx vitest run src/renderer`, `npm run typecheck`, `npm run lint` — expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): add tag filter bar and dim or hide non-matching tests`

### Task 7: End-to-end scenarios and ship

**Files:**
- Create: `e2e/tags.spec.ts`
- Modify: `e2e/helpers.ts` (`project()` accepts `"tags"`; `launch(userData?: string)` reuses a given folder instead of making a new one), `docs/features.md` (F04 status `done`)

- [ ] **Step 1: Write one Playwright test per Gherkin scenario** of the spec's "Acceptance criteria", same names, using the Task 6 test ids and `node()` / `nodeByKey()` helpers. "The filter survives a restart" closes the app and calls `launch(userData)` with the first launch's folder. "A tag removed from the project leaves the filter" and "A tag removed before a run does not widen it" edit the copied fixture with `writeFileSync`.

- [ ] **Step 2: Run** `npm run e2e` (not alongside `npm test`) — expected: all tests pass, the walking-skeleton tests unchanged.

- [ ] **Step 3: Run** `npm test`, `npm run typecheck`, `npm run lint` — expected: PASS.

- [ ] **Step 4: Update** `docs/features.md` F04 status `specced` → `done`.

- [ ] **Step 5: Commit** `test(e2e): cover the tag filter` and `docs: mark f04 done`
