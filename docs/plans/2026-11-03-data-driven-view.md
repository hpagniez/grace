# Data-driven view Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When an outline is selected, the details pane shows its Examples as a table of rows with values, status and time; a click or Enter opens a row.

**Architecture:** `buildTree` copies Karate's `exampleData` from the dry run's depth-0 `FEATURE_EXIT` onto each row node. A new `Examples` component under the outline's `Group` header draws the rows through `VirtualList`, which gains an option so arrow keys move without selecting.

**Tech Stack:** Electron, SolidJS 2 RC, Tailwind v4, Vitest + @solidjs/testing-library, Playwright.

**Spec:** `docs/specs/2026-11-03-data-driven-view.md`

## Global Constraints

- Solid 2 rules and UI conventions of AGENTS.md: semantic tokens only, `Icon`, `StatusMark`, `Tooltip`, `Cut`; no `title=`, no raw palette class.
- No change to `src/renderer/Tree.tsx` or `src/renderer/History.tsx`.
- Idle costs nothing: no timer, poll or effect while no run is going.
- Rows are 24 px; the table draws only the lines in view (`VirtualList`).
- Accessible name of a line: `Row <n>: <col> <value>, …, <Status label>[, <time>]`, e.g. `Row 2: a 2, b 3, expected 5, Passed, 6 ms`.

## Review Focus

- A refresh while an outline is selected: the table shows the new tree's rows and values, not the old ones (Task 3 test).
- A row with no `exampleData` or an outline whose rows have different columns: empty cells, columns in first-met order (Tasks 1 and 3 tests).
- A non-string value from a dynamic table (`1`, `{"x":1}`): shown as JSON text (Task 1 test).
- Keyboard: Down then Enter in the table keeps the outline until Enter, then opens the row (Task 3 test).
- A run updating statuses while the table is shown: the marks change, the lines are not rebuilt (Task 3 test).

---

### Task 1: Row values on the tree

**Files:**
- Modify: `src/shared/model.ts` (TestNode)
- Modify: `src/main/karate/tree.ts` (`buildTree`)
- Test: `src/main/karate/tree.test.ts`, `src/main/runner.runs.test.ts`

**Interfaces:**
- Produces: `TestNode.example?: Record<string, string>` — rows only, the row's values by Examples column from Karate's dry run.

- [ ] **Step 1: Write the failing tests** in `tree.test.ts`, `describe("buildTree")`:
  - `"keeps each row's Examples values from the dry run"`: `findNode(buildTree(clean(), "clean"), "s:tests/outline.feature#1.2")!.example` equals `{ a: "2", b: "3", expected: "5" }`; the outline and scenarios have no `example`.
  - `"keeps the values of rows in two Examples tables"`: on `dryrun-outline-sizes.jsonl`, rows of `o:extra/outline-sizes.feature#2` map to `{ a: "1" }` … `{ a: "7" }`.
  - `"keeps a non-string value as JSON and leaves a row without values bare"`: `mutate` the clean events so the `FEATURE_EXIT` of `tests/outline.feature` has row 1 `exampleData: { a: 1, b: { x: 1 } }` and row 3 without `exampleData`; expect `{ a: "1", b: '{"x":1}' }` and `undefined`. (`mutate` matches `data.feature`; FEATURE_EXIT has `relativePath`, so write a direct `structuredClone` edit.)
  - In `runner.runs.test.ts`, `"refuses a row of a dynamic Examples table"`: add `expect(rows.map((r) => r.example)).toEqual([{ a: "1" }, { a: "2" }])`.
- [ ] **Step 2: Run** `npx vitest run src/main/karate/tree.test.ts` — the three new tests fail.
- [ ] **Step 3: Implement.** In `buildTree`, on a depth-0 `FEATURE_EXIT`, collect `d.scenarioResults[].exampleData` by `${d.relativePath}|${r.refId}`; after the loop, set them on the nodes through `leafIndex(root)`, each value `typeof v === "string" ? v : JSON.stringify(v)`. Update the `buildTree` doc comment.
- [ ] **Step 4: Run** `npx vitest run src/main/karate/tree.test.ts` — pass. Then `npx vitest run src/main/runner.runs.test.ts -t "dynamic"` (real Karate, one file) — pass.
- [ ] **Step 5: Commit** `feat(runner): keep each outline row's Examples values`.

### Task 2: VirtualList moves without selecting

**Files:**
- Modify: `src/renderer/VirtualList.tsx`
- Test: `src/renderer/VirtualList.test.tsx`

**Interfaces:**
- Produces: `VirtualList` prop `selectOnMove?: boolean` (default `true`). When `false`, arrow keys, Home, End, Page Up and Page Down move the focus without calling `onSelect`; Enter, Space and a click on the row still call it.

- [ ] **Step 1: Write the failing test** `"with selectOnMove false, keys move the focus and only Enter selects"`: render 5 items, focus the first row, press ArrowDown: `onSelect` not called, row 2 focused; press Enter: `onSelect` called once with item 2.
- [ ] **Step 2: Run** `npx vitest run src/renderer/VirtualList.test.tsx` — fails.
- [ ] **Step 3: Implement**: in `moveTo`, call `props.onSelect(item)` only when `props.selectOnMove !== false`.
- [ ] **Step 4: Run** the file — pass.
- [ ] **Step 5: Commit** `feat(ui): let a virtual list move its focus without selecting`.

### Task 3: The Examples table in Details

**Files:**
- Create: `src/renderer/Examples.tsx`
- Modify: `src/renderer/Details.tsx` (outline branch, `select` prop, aside layout), `src/renderer/App.tsx` (pass `select`)
- Test: `src/renderer/Examples.test.tsx`

**Interfaces:**
- Consumes: `TestNode.example` (Task 1), `selectOnMove` (Task 2).
- Produces: `exampleColumns(rows: TestNode[]): string[]` — the union of the rows' columns in first-met order; `Examples(props: { outline: TestNode; state: AppState; select(key: string): void })`; `Details(props: { state; api; select(key: string): void })`.

- [ ] **Step 1: Write the failing tests** in `Examples.test.tsx`, rendering `App` with a fake api as `Details.test.tsx` does, tree: feature `tests/o.feature` > outline `o:tests/o.feature#1` "add <a> and <b>" > rows `#1.1..#1.3` "add 1 and 2" … with `example` `{a,b,expected}`:
  - `exampleColumns` gives `["a","b","c"]` for rows `{a,b}`, `{a,c}`, `{}`.
  - Selecting the outline shows `getByRole("listbox", { name: "Examples" })`, header cells `a`, `b`, `expected`, 3 options; option 2's accessible name is `Row 2: a 2, b 3, expected 5, Not run`.
  - After a `scenarios` event (row 2 passed, 6 ms), option 2's name is `Row 2: a 2, b 3, expected 5, Passed, 6 ms` and its `data-status` is `passed`; the option element is the same node as before the event.
  - A click on option 3's cell selects row 3: the details heading reads `add 5 and 5`.
  - Focus option 1, ArrowDown: the outline's heading stays; Enter: the heading reads `add 2 and 3`.
  - A row without `example` shows empty cells and its name `Row 3: Not run`.
  - A new `project` event with row 1's `a` changed to `9` while the outline is selected: option 1's name starts `Row 1: a 9`.
  - A 10,000-row outline: the listbox has `data-rows="10000"` and fewer than 60 options in the DOM.
  - A scenario selected shows no Examples listbox.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Examples.test.tsx` — fails.
- [ ] **Step 3: Implement.** `Examples`: a box (`rounded-lg border border-line`) of a header row and a `VirtualList` (`role="listbox"`, `itemRole="option"`, `label="Examples"`, `rowHeight={24}`, `selectOnMove={false}`, `selectedKey={null}`, `onSelect={(r) => props.select(r.key)}`, items = `props.outline.children`). Header and lines share one inline `grid-template-columns`: `1.5rem 2.5rem repeat(<n>, minmax(0, 1fr)) 4.5rem` (no value columns: drop the `repeat`). Line: `StatusMark` in a `Tooltip` with `STATUS_LABEL` (`side="right"`), the number, each value through `Cut` (`side="over"`), the time `formatMillis` right-aligned; the inner div's `onClick` selects, as `HttpCalls`' `Row` does. Attributes: `data-testid="example"`, `data-status`, `aria-label` per Global Constraints, `aria-selected="false"`. Height: the box is a flex column with `min-height` of `(min(8, rows) + 1) × 24 px + 2 px` and `flex: 0 1 auto`; the `Details` aside changes from `space-y-3` to `flex flex-col gap-3` so the box fills what is left and the list scrolls inside it. In `Details`, an outline renders `Group` then `Examples`; `App` passes `select`.
- [ ] **Step 4: Run** `npx vitest run src/renderer/Examples.test.tsx src/renderer/Details.test.tsx src/renderer/palette.test.ts` — pass; `npm run typecheck` and `npm run lint` — clean.
- [ ] **Step 5: Commit** `feat(ui): show an outline's Examples as a table in the details pane`.

### Task 4: End-to-end test and spec status

**Files:**
- Create: `e2e/examples.spec.ts`
- Modify: `docs/specs/2026-11-03-data-driven-view.md` (Status: approved)

- [ ] **Step 1: Write the test** `"An outline's Examples as a table"`: open `project("clean")`, click the outline in the tree (`node(win, "add <a> and <b>")`), expect 3 `example` lines in `details` reading `1 2 3`, `2 3 5`, `5 5 10` and each `data-status="notRun"`; run the outline with its `run-node` button; wait for `run-outcome` "Finished"; each line `data-status="passed"`; click line 2; `details` heading reads `add 2 and 3` and `nodeByKey(win, "s:tests/outline.feature#1.2")` has `aria-selected="true"` on its treeitem.
- [ ] **Step 2: Run** `npm run e2e` after the parent session's go — all pass. Paste the summary in the PR body.
- [ ] **Step 3: Commit** `test(e2e): show an outline's Examples as a table`.
