---
status: built
issue: 224
summary: "The tree starts with folders open and features closed; clicking a feature's name, running it or a failure in it opens it"
---

# Open the test tree with only its folders open: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The tree starts with folders open and features closed; a click on a feature's name, a run of it, or a failure in it opens it (#224).

**Architecture:** The tree keeps one set of keys turned from their starting state (a feature starts closed, every other row open), replacing today's `closed` set. The run's view keeps the run's starting key and its failed keys in order; the tree opens features from them in one effect, at most once per feature per run. `treeRows` takes an `isOpen` predicate instead of the closed set and still reads no status.

**Tech Stack:** SolidJS 2 RC (see AGENTS.md "Solid 2 rules"), Vitest + `@solidjs/testing-library`, Playwright `_electron`, the bench in `bench/`.

**Spec:** [docs/specs/2026-11-03-tree-folders-open.md](../specs/2026-11-03-tree-folders-open.md)

## Global Constraints

- The row list reads no status (spec R4.3): `rows` in `Tree.tsx` reads only `tree`, the open state and `filter`.
- Idle costs nothing: the run effect's compute reads only `run.runId` and `run.failedKeys.length`.
- The feature of a key comes from the key (`f:` + the file part of an `s:`/`o:` key, before its last `#`), never from `node.file` or a tree walk.
- No backward compatibility, no setting for the old behaviour, no "open all" command.
- Single unit test files run any time; a full `npm test`, `npm run e2e` or `npm run bench` waits for the parent session's "go" (SendMessage `local_63543986-0e1a-425b-9422-9d7be6a009bb`).
- Commits: Conventional Commits, scope `ui`, `test`, `bench` or `docs`; body line `Part of #224`.

## Review Focus

1. A test that failed in an earlier run, outside this run, must never open its feature: only this run's failed keys count. Test in Task 3.
2. A rerun of the failed tests and Run all start from the root key (`d:` or the root's key): `featureOf` gives null, nothing opens at start. Test in Task 1.
3. A file path holding `#` (`s:tests/a#b/x.feature#2`): the feature is `f:tests/a#b/x.feature`. Test in Task 1.
4. A feature that fails twice in a run after the person closed it stays closed (R3.5). Test in Task 3.
5. Another project resets the open state; a refresh keeps it, and a feature new in the refresh starts closed. Test in Task 2.

---

### Task 1: The run's starting key, its failed keys, and `featureOf`

**Files:**
- Modify: `src/renderer/reduce.ts` (`RunView`, `idleRun`, the `runStarted` and `scenarios` cases)
- Modify: `src/renderer/nodes.ts` (add `featureOf`)
- Test: `src/renderer/reduce.test.ts`, `src/renderer/nodes.test.ts` (create)

**Interfaces:**
- Produces: `RunView.selection: string | null` (from `runStarted.selection`; null when idle), `RunView.failedKeys: string[]` (keys in the order they first failed in this run; `[]` when idle and at each `runStarted`).
- Produces: `featureOf(key: string | null): string | null` in `nodes.ts`.

- [ ] **Step 1: Write the failing tests**
  - `reduce.test.ts`: "keeps the run's starting key": after `runStarted` with `selection: "s:tests/a.feature#2"`, `run.selection` is that key.
  - `reduce.test.ts`: "lists the tests that fail in the run, once each, in order": `scenarios` updates fail `k2`, then pass `k1`, then fail `k3`, then fail `k2` again → `run.failedKeys` equals `["k2", "k3"]`; a new `runStarted` resets it to `[]`.
  - `nodes.test.ts`: `featureOf("f:tests/a.feature")` → `"f:tests/a.feature"`; `featureOf("s:tests/a.feature#3")` → `"f:tests/a.feature"`; `featureOf("s:tests/a.feature#3.2")` → `"f:tests/a.feature"`; `featureOf("o:tests/a.feature#3")` → `"f:tests/a.feature"`; `featureOf("s:tests/a#b/x.feature#2")` → `"f:tests/a#b/x.feature"`; `featureOf("d:tests")`, `featureOf("d:")`, `featureOf(null)` → `null`.
- [ ] **Step 2: Run them and see them fail:** `npx vitest run src/renderer/reduce.test.ts src/renderer/nodes.test.ts`
- [ ] **Step 3: Implement.** Push in `scenarios` under the existing `!final` branch, only when `u.status === "failed"`. Fix any `toEqual` on a whole `RunView` in `reduce.test.ts` for the two new fields.
- [ ] **Step 4: Run the two files again: PASS.**
- [ ] **Step 5: Commit** `feat(ui): keep a run's starting key and the tests that failed in it`

### Task 2: Features start closed; a click on a feature's name opens it (R1, R2)

**Files:**
- Modify: `src/renderer/treeRows.ts` (`treeRows(root, isOpen: (key: string) => boolean, filter)` replaces the `closed` set)
- Modify: `src/renderer/Tree.tsx`
- Create: `test/openFeatures.ts`
- Test: `src/renderer/treeRows.test.ts`, `src/renderer/Tree.test.tsx`, and every other `src/renderer/*.test.tsx` that renders `App` with a tree

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces in `Tree.tsx` (local): `turned` signal (`ReadonlySet<string>`, reset when `root()` changes, as `closed` is today); `startsOpen(key) = !key.startsWith("f:")`; `isOpen(key) = startsOpen(key) !== turned().has(key)`; `setOpen(key: string, open: boolean)`; `toggle(key)` = `setOpen(key, !isOpen(key))`. Task 3 uses `isOpen` and `setOpen`.
- Produces: `openFeatures(): void` in `test/openFeatures.ts`: clicks every button named `/^Expand .+\.feature$/`, the last first (so opening one never pushes another out of the drawn rows), with `flush()` after each, until none is left.

- [ ] **Step 1: Write the failing tests in `Tree.test.tsx`** (a new `describe("folders open, features closed (#224)")`), with the file's `setup()`/`load()`:
  - "starts with folders open and features closed": row keys equal `["d:tests", "f:tests/basic.feature", "f:tests/outline.feature"]`; `d:tests` has `aria-expanded="true"`, both features `"false"`.
  - "a click on a closed feature's name selects and opens it": click the label button of `f:tests/basic.feature` → `aria-selected="true"`, `aria-expanded="true"`, rows include `s:tests/basic.feature#1` and `#2`.
  - "a click on an open feature's name keeps it open": click it twice → still `aria-expanded="true"`.
  - "the chevron closes a feature": `Expand basic.feature`, then `Collapse basic.feature` → `"false"`, its scenarios gone.
  - "keys that select do not open": focus `d:tests`, ArrowDown twice → `f:tests/outline.feature` selected and closed; Enter → still closed; ArrowRight → open, with the outline and its 2 rows (outlines start open).
  - "a refresh keeps what was opened and closed, and a new feature starts closed": open `basic.feature`, close `tests`; send a `project` event with the same root and a tree that adds `f:tests/new.feature` → `tests` closed; open `tests` → `basic.feature` open, `new.feature` closed.
  - "another project starts closed again": open `basic.feature`; load a tree with root `D:/q`, then `D:/p` again → `basic.feature` closed.
- [ ] **Step 2: Run `npx vitest run src/renderer/Tree.test.tsx`: the new tests FAIL.**
- [ ] **Step 3: Implement.**
  - `treeRows`: skip a node's children unless `isOpen(node.key)`. Update its doc comment and the callers in `treeRows.test.ts` (`new Set()` → `() => true`; `new Set([k])` → `(key) => key !== k`) and `Tree.test.tsx:828`.
  - `Tree.tsx`: replace `closed` with the Interfaces above; `aria-expanded` and `Row`'s `open` read `isOpen`; `onKey` reads `isOpen`; the pause effect opens each ancestor with `setOpen(a, true)` when `!isOpen(a)`. The comment on the signal cites spec R1.3.
  - `Row`: the label button's click selects, then, on a feature that is closed, calls `toggle()` (R2.1).
- [ ] **Step 4: Run `Tree.test.tsx` and `treeRows.test.ts`.** The new tests PASS. Existing tests that read or click a scenario, outline or row now fail: call `openFeatures()` after `load()` in each, and fix row counts only where the test meant the closed tree. The first test (`shows a failed scenario's status on its feature`) keeps its 8 rows only after `openFeatures()`.
- [ ] **Step 5: Run each other renderer test file that renders `App`** (`CallDetails`, `Details`, `EnvPicker`, `FilterBar`, `FlakyProof`, `History`, `Home`, `HttpCalls`, `PropertiesDialog`, `Threads`), one `npx vitest run <file>` at a time. In each failing test, call `openFeatures()` once the tree is loaded. Change nothing else.
- [ ] **Step 6: `npm run typecheck` and `npm run lint`: clean.**
- [ ] **Step 7: Commit** `feat(ui)!: open the test tree with only its folders open`, footer `BREAKING CHANGE: the tree no longer starts with every feature open.`

### Task 3: A run opens features (R3, R4)

**Files:**
- Modify: `src/renderer/Tree.tsx`
- Test: `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: `run.selection`, `run.failedKeys`, `featureOf` (Task 1); `isOpen`, `setOpen`, `toggle` (Task 2).

- [ ] **Step 1: Write the failing tests** (a `describe("a run opens features (#224 R3)")`; send `runStarted` and `scenarios` as the file's first test does):
  - "a run of a feature opens it as it starts", for `selection` `f:tests/basic.feature`, `s:tests/basic.feature#2` (History's Re-run) and with `debug: true` → `basic.feature` open, `outline.feature` closed.
  - "Run all opens nothing as it starts": `selection: "d:"` → both closed.
  - "Run all opens a feature when one of its tests fails": fail `s:tests/basic.feature#2` → `basic.feature` open; pass `s:tests/outline.feature#1.1` → `outline.feature` closed.
  - "a run never reopens a feature closed during it", three cases: opened on its failure; open before the run; opened by a name click during the run. Close it, fail another of its tests → still closed.
  - "a failure from an earlier run opens nothing": run 1 fails `s:tests/basic.feature#2`; close `basic.feature`; run 2 (`selection: "f:tests/outline.feature"`) fails `s:tests/outline.feature#1.1` → `basic.feature` closed.
  - "a run opens only the feature": close `d:tests`, fail `s:tests/basic.feature#1` → `d:tests` still `aria-expanded="false"`; open `tests` → `basic.feature` open. Open `outline.feature`, close its outline, fail a row → outline still closed.
  - "what a run opened stays open after it": `runFinished` → still open.
  - "statuses that open nothing leave the rows alone": with `treeRows` spied (as the file spies `deriveStatus`), passing 2 tests of an open feature calls it 0 more times.
- [ ] **Step 2: Run `Tree.test.tsx`: the new tests FAIL.**
- [ ] **Step 3: Implement one effect in `Tree`.** Compute: `[props.state.run.runId, props.state.run.failedKeys.length]`. Apply, with a local `let run = { id: null as string | null, done: new Set<string>(), read: 0 }`:
  - when the id is null, do nothing;
  - when it differs from `run.id`, start a new `run` and open `featureOf(props.state.run.selection)`;
  - then open `featureOf(failedKeys[i])` for each key from `run.read` to the length, and move `run.read` along.
  - "Open" means: skip when null or in `run.done`; add to `run.done`; `setOpen(f, true)` when `!isOpen(f)`.
  - `toggle`, when it closes a feature while `props.state.run.active`, adds the key to `run.done` (R3.5).
  - Comment the effect with R3 and R4.3.
- [ ] **Step 4: Run `Tree.test.tsx`: PASS.** Run `reduce.test.ts` too.
- [ ] **Step 5: Commit** `feat(ui): open a feature when a run starts in it or a test of it fails`

### Task 4: e2e specs and the screenshot scripts

**Files:**
- Modify: `e2e/helpers.ts` (add `openFeatures(win: Page): Promise<void>`; `openEnv` calls it before waiting for `always`)
- Modify: the specs that click or read a scenario, outline or row before its feature is open: `virtual`, `open`, `history`, `tags`, `env`, `flaky`, `debug`, `look`, `editor`, `http`, `run`, `threads`, `project-setup`, `details`, `cancel`, `cursor`, `home`, and `e2e/readme.shots.ts`, `e2e/screens.shots.ts`
- Test: one new test in `e2e/open.spec.ts`

**Interfaces:**
- Produces: `openFeatures(win)`: the Playwright twin of `test/openFeatures.ts`; clicks the last button named `/^Expand .+\.feature$/` until there is none.

- [ ] **Step 1: Write the new test in `open.spec.ts`**, "the tree opens with only its folders open (#224)": after the project loads, no `[data-kind=scenario]`, `[data-kind=outline]` or `[data-kind=row]` row; a click on a feature's name opens it (`aria-expanded="true"`); a closed second feature's run button opens it as the run starts (`aria-expanded="true"` while `run-outcome` reads `Running…` or once it reads `Finished`).
- [ ] **Step 2: In each spec's open helper or test, call `openFeatures(win)` once the tree is loaded**, where the spec then uses test rows. The specs' local open helpers (`openFix`, `openTags`, `coin`, `startSlowRun`, `openBig`, `open` in `screens.shots.ts`) call it themselves, `openBig` excepted (Step 3). A test that checks the closed tree on purpose does not.
- [ ] **Step 3: Retarget `virtual.spec.ts` to the tree as it now opens:**
  - `ROWS = 249`.
  - The last row is `f:tests/area-8/module-5/feature-5.feature`, text `feature-5.feature`, kind `feature`.
  - The tests that use `ROW_1500` first open `f:tests/area-5/module-5/feature-3.feature` by its name, then use that scenario's new index, read from the list, not hard-coded.
  - The filter test expects `3 + 5 + 11` rows: area-5, module-5-5's folder and `tests`; its 5 features; the open feature's 11 children.
  - The `CUT` and keyboard tests first open `FEATURE_2` by its name.
- [ ] **Step 4: Typecheck and lint.** Then ask the parent for the go, run `npm run e2e`, and fix what fails. Tell the parent when it is done.
- [ ] **Step 5: Commit** `test(e2e): open features before using their tests in the tree`

### Task 5: The bench

**Files:**
- Modify: `e2e/big-project.ts` (`writeBigProject` returns `{ tests, calls, rows }`, `rows` = folders + features: `1 + AREAS + AREAS * MODULES + AREAS * MODULES * featuresPerModule`)
- Modify: `bench/bench.mjs` (`makeProject` keeps `rows`; `treeRows(page, rows)` waits for `rows`; `interactive` opens the first feature by a click on its label, then reads the row count it passes to `smoothProbes` as `total`)

- [ ] **Step 1: Make the changes; the first feature is the first `[data-kind=feature]` row; its label is its second button.** A click there never closes (R2.1), so the base build, whose features start open, gets the same probes.
- [ ] **Step 2: `node --check bench/bench.mjs`, then `bench/judge.test.ts`, `report.test.ts`, `stats.test.ts` and `system.test.ts`, one file at a time.**
- [ ] **Step 3: Ask the parent for the go, run `npm run bench -- --rounds 1`, and check every Smooth figure in `bench-out/bench.md` reads a number, not n/a.** Tell the parent when it is done.
- [ ] **Step 4: Commit** `test(bench): measure the tree as it opens, with features closed`

### Task 6: Finish

- [ ] **Step 1:** After the parent's go, run a full `npm test`, then `npm run typecheck` and `npm run lint`.
- [ ] **Step 2:** Push `feature/tree-folders-open`. Open a draft PR into `develop` with `Closes #224`, the e2e summary and the bench note. Move #224's card to building.
- [ ] **Step 3:** Read the `perf` comment. Explain every measure that is worse than base beyond its noise margin, and say which tree figures changed because the tree now opens with features closed.
- [ ] **Step 4:** After merge, point #224's `Spec:` line at `docs/specs/2026-11-03-tree-folders-open.md` on `develop`.
