# Big projects: virtual lists and keyboard navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Draw only the rows on screen in the test tree and the HTTP list, and make each list one tab stop with arrow-key navigation, so Grace stays smooth at 10,000 tests and every row stays reachable.

**Architecture:** Pure window math in `src/renderer/virtual.ts` (which rows to draw, where to scroll, scroll anchoring, where focus goes). One generic Solid component, `VirtualList`, owns the scroller, the drawn rows, roving `tabindex` and the shared keys. The HTTP list uses it directly. The tree first flattens itself (`treeRows.ts`: the rows a person can scroll to, with their ARIA levels and positions), then uses it with its own Left, Right and Ctrl+Enter keys.

**Tech Stack:** Electron, SolidJS 2.0.0-rc.18, Tailwind CSS 4, Vitest 5 + jsdom 30, Playwright 1.64.1 `_electron`.

**Spec:** [docs/specs/2026-10-30-virtual-lists.md](../specs/2026-10-30-virtual-lists.md) (R1 to R6). Read it before any task: requirement numbers below point into it, and its Gherkin scenarios are the tests.

## Global Constraints

- Two PRs, each cut from `develop` once the one before has merged:
  - PR 1, `feature/virtual-http-list`: Tasks 1 to 3. Body `Part of #125` and `Part of #91`. When it opens, move the #125 and #91 cards to `building` (the `gh project item-add` and `item-edit` commands in AGENTS.md).
  - PR 2, `feature/virtual-tree`: Tasks 4 to 8. Body `Closes #125` and `Closes #91`.
- Conventional Commits, scope `ui` (`test(e2e)` for e2e-only commits); each message ends with the Claude Code attribution lines.
- Renderer only. No change to `src/main/**`, `src/preload.ts`, `src/shared/ipc.ts` or `src/shared/events.ts`. No new dependency.
- Solid 2 rules and UI conventions of AGENTS.md: tokens only, `Icon`, `StatusMark`, `Tooltip`, `Cut`; no `title=`, no `classList`.
- Row heights: tree 24 px, HTTP 28 px. Tree indent 16 px per level. The tree keeps its 8 px padding (`p-2` today) as `pad: 8`.
- `OVERSCAN = 10` rows above and below. A scroller that measures 0 px uses `FALLBACK_VIEW = 600` px (R1.6).
- jsdom has no layout, no `ResizeObserver` and no `scrollIntoView`: set `scrollTop` yourself, never call `scrollIntoView`, and guard `ResizeObserver` with `typeof`.
- Keep every `data-testid` and attribute e2e tests use: `tree`, `tree-node` (`data-key`, `data-status`, `data-kind`, `data-match`), `run-node`, `flaky-count`, `flaky-mark`, `http-calls` (the scrolling box), `http-call` (`data-status`), `http-call-*`, `http-empty`. A run button's `aria-label` stays `Run <label>`: `e2e/helpers.ts` `node()` finds rows by it.
- Run `npm run typecheck` and `npm run lint` before each commit. Never run `npm test` and `npm run e2e` at the same time. Run `npm run e2e` before marking a PR ready and paste the summary in it.

## Review Focus

1. **A list that shrinks below its scroll position** (a filter hides most of the tree while it is scrolled to the bottom): the view clamps and shows rows, never a blank box. Test in Task 1 and Task 2.
2. **A click on a button inside a row, then a key:** focus moves to the row, so Down moves on and Space selects instead of pressing that button again (the run button would start a second run). Test in Task 2.
3. **The focused call disappears** when a new run starts and clears its calls: focus moves to the nearest call above, with no error and no focus lost to the page. Test in Task 3.
4. **A refresh removes the focused tree row** (its feature file was deleted) while the tree has focus: focus lands on the nearest row above, and `document.activeElement` is a treeitem. Test in Task 6.
5. **A tiny viewport** (the bottom panel at its 96 px minimum shows 2 or 3 calls): Page Down still moves by at least one row. Test in Task 1.

---

### Task 1: Window math

**Files:**
- Create: `src/renderer/virtual.ts`, `src/renderer/virtual.test.ts`

**Interfaces:**
- Produces:

```ts
export const OVERSCAN = 10;
export const FALLBACK_VIEW = 600; // px, a scroller that measures 0 (jsdom, R1.6)
export interface View {
  scrollTop: number;
  height: number; // the scroller's clientHeight; 0 means FALLBACK_VIEW
  rowHeight: number;
  pad: number; // px before the first row
}
/** The rows to draw: first inclusive, last exclusive, with OVERSCAN either side. scrollTop is clamped to the content first. */
export function drawnRange(count: number, v: View): { first: number; last: number };
/** The scrollTop that shows row `index` with the least scroll (R4.1); v.scrollTop when it is in view. */
export function scrollToShow(index: number, v: View): number;
/** Whole rows in view, at least 1 (Page Up, Page Down). */
export function pageRows(v: View): number;
/** The scrollTop that keeps the first row in view still on screen when the rows change (R4.6). 0 stays 0; an anchor that is gone keeps v.scrollTop. */
export function anchoredScrollTop(v: View, before: readonly string[], after: readonly string[]): number;
/** Where focus goes when `key` leaves the list: the nearest row before it that is still there, else the first row, else null (R4.4, R4.5). */
export function survivor(key: string, before: readonly string[], after: readonly string[]): string | null;
```

- [ ] **Step 1: Write the failing tests** in `src/renderer/virtual.test.ts`, with `v = (scrollTop, height = 600, rowHeight = 24, pad = 0) => ({ scrollTop, height, rowHeight, pad })`:

```ts
expect(drawnRange(1000, v(0))).toEqual({ first: 0, last: 35 });
expect(drawnRange(1000, v(2400))).toEqual({ first: 90, last: 135 });
expect(drawnRange(1000, v(2408, 600, 24, 8))).toEqual({ first: 90, last: 135 });
expect(drawnRange(1000, v(0, 0))).toEqual({ first: 0, last: 35 }); // fallback 600
expect(drawnRange(5, v(4000))).toEqual({ first: 0, last: 5 }); // Review Focus 1
expect(drawnRange(0, v(0))).toEqual({ first: 0, last: 0 });

expect(scrollToShow(50, v(0))).toBe(624); // row 50 is 1200..1224
expect(scrollToShow(10, v(624))).toBe(240);
expect(scrollToShow(30, v(624))).toBe(624); // in view: no scroll
expect(scrollToShow(0, v(100, 600, 24, 8))).toBe(8);

expect(pageRows(v(0))).toBe(25);
expect(pageRows(v(0, 84, 28))).toBe(3);
expect(pageRows(v(0, 20, 28))).toBe(1); // Review Focus 5

const keys = ["a", "b", "c", "d", "e"];
expect(anchoredScrollTop(v(48), keys, ["x", "y", ...keys])).toBe(96);
expect(anchoredScrollTop(v(60), keys, ["x", "y", ...keys])).toBe(108); // keeps the 12 px offset
expect(anchoredScrollTop(v(0), keys, ["x", ...keys])).toBe(0);
expect(anchoredScrollTop(v(48), keys, ["a", "b"])).toBe(48); // anchor "c" gone

expect(survivor("c", ["a", "b", "c", "d"], ["a", "d"])).toBe("a");
expect(survivor("a", ["a", "b"], ["b"])).toBe("b");
expect(survivor("a", ["a"], [])).toBeNull();
```

- [ ] **Step 2: Run** `npx vitest run src/renderer/virtual.test.ts`. Expected: FAIL, `virtual` has no exports.
- [ ] **Step 3: Implement** the five functions in `src/renderer/virtual.ts`. `anchoredScrollTop` finds the anchor as `before[floor((scrollTop - pad) / rowHeight)]` and its new index with one `indexOf`.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add the window math for virtual lists`.

### Task 2: `VirtualList`

**Files:**
- Create: `src/renderer/VirtualList.tsx`, `src/renderer/VirtualList.test.tsx`
- Modify: `src/renderer/Tooltip.tsx` (export `keyboardFocus`)

**Interfaces:**
- Consumes: Task 1's functions.
- Produces:

```ts
export interface VirtualListHandle {
  /** Scrolls the least to show the row with this key (R4.1); nothing when it is not in the list. */
  reveal(key: string): void;
}
export function VirtualList<T>(props: {
  items: readonly T[];
  keyOf(item: T): string;
  rowHeight: number;
  pad?: number; // default 0
  role: "tree" | "listbox";
  itemRole: "treeitem" | "option";
  label: string; // the list's aria-label
  testid?: string; // on the role element
  scrollerTestid?: string; // on the scrolling element
  itemClass?: string; // on every row element
  attrs(item: T, index: number): Record<string, string | number | undefined>; // spread on the row element
  selectedKey: string | null;
  onSelect(item: T): void;
  onKey?(e: KeyboardEvent, item: T, index: number): boolean; // the list's own keys; true when handled
  ref?(handle: VirtualListHandle): void;
  children(item: T, index: number, keyboardFocused: () => boolean): JSX.Element;
}): JSX.Element;
```

- DOM: `<div data-testid={scrollerTestid} class="min-h-0 flex-1 overflow-auto">` holds `<div role aria-label data-testid={testid} data-rows={items.length} style="position:relative;height:{count × rowHeight + 2 × pad}px">`. Each drawn row is `<div role={itemRole} data-key tabindex class={itemClass} style="position:absolute;left:0;right:0;top:{pad + index × rowHeight}px;height:{rowHeight}px" {...attrs}>` around `children`. The role element's parent is the scroller; the performance harness relies on that and on `data-rows`.
- The focus row is the active key if it is in `items`, else `selectedKey` if it is in `items`, else the first item. It alone has `tabindex="0"` (the others `-1`) and is always drawn (R4.3).

- [ ] **Step 1: Write the failing tests** in `src/renderer/VirtualList.test.tsx`. Render `VirtualList` over `items = Array.from({ length: n }, (_, i) => \`r${i}\`)`, `rowHeight: 24`, a `children` that renders the key and a `<button>` with a spy, and `onSelect`/`onKey` spies. Scroll with `scroller.scrollTop = x; scroller.dispatchEvent(new Event("scroll"))`, then `flush()`. One `it` per line:
  - "draws only the rows in view, plus the margin": n = 1000 → 35 `[role=option]` elements; `data-rows` is `"1000"`; the role element's height is `24000px`.
  - "draws the rows at the bottom after a scroll, and keeps the focus row": `scrollTop = 23400` → rows `r965` to `r999` are drawn, and so is `r0` (`tabindex="0"`).
  - "has one tab stop: the selected row, else the first": `selectedKey: "r5"` → only `r5` has `tabindex="0"`; with `selectedKey: null`, only `r0`.
  - "moves with Down, Up, Home, End, Page Down and Page Up, selecting as it goes": from focused `r0`, `ArrowDown` → `document.activeElement` is `r1` and `onSelect` got `"r1"`; `End` → `r999`, and `scrollTop` is `scrollToShow(999, …)` (23400); `PageUp` → `r974`; `Home` → `r0`, `scrollTop` 0.
  - "does nothing at the ends, and keeps the page still": `ArrowUp` on `r0` → `onSelect` not called, and the event's `defaultPrevented` is true.
  - "selects with Enter and Space": both call `onSelect` with the focused item.
  - "passes other keys to onKey, and stops what it handled": `ArrowRight` → `onKey` called with `(event, "r0", 0)`; when it returns true, `defaultPrevented` is true.
  - "brings a scrolled-away focus row back before moving" (R4.2): focused `r10`, `scrollTop = 20000`, `ArrowDown` → `r11` focused and `scrollTop` is `scrollToShow(11, …)`.
  - "takes focus to the row clicked, even through a button in it" (Review Focus 2): click the button of `r3` → its spy was called once, `document.activeElement` is the `r3` row; `Space` → `onSelect("r3")` and the button spy still at 1.
  - "keeps the rows in view still when rows are added above" (R4.6): `scrollTop = 240` (top row `r10`); prepend 5 items → `scrollTop` is 360.
  - "moves focus to the nearest row above when the focus row leaves" (R4.4, R4.5): DOM focus on `r5`; remove `r3` to `r5` → `r2` has `tabindex="0"` and is `document.activeElement`.
  - "draws rows after the list shrinks below the scroll position" (Review Focus 1): `scrollTop = 20000`, items cut to 5 → 5 rows drawn.
  - "reveals a row by key": `handle.reveal("r500")` → `scrollTop` is `scrollToShow(500, …)`; `reveal("nope")` changes nothing.
  - "tells a row when it has keyboard focus": `children`'s third argument is true for the focused row after `ArrowDown`, false for the others.
- [ ] **Step 2: Run** `npx vitest run src/renderer/VirtualList.test.tsx`. Expected: FAIL, no `VirtualList`.
- [ ] **Step 3: Implement** `VirtualList` in `src/renderer/VirtualList.tsx` and export `keyboardFocus` from `src/renderer/Tooltip.tsx`.
  - Height comes from the scroller's `clientHeight`, updated by a `ResizeObserver` when there is one.
  - Rows go through `<For each={drawn} keyed={(r) => r.key}>`, where `drawn` is the `drawnRange` slice plus the focus row.
  - On an `items` change, the scroller first gets `anchoredScrollTop`, then a missing active key becomes its `survivor`. Track whether the list holds DOM focus with `focusin` and `focusout` on the role element; when it does, focus the new focus row with `{ preventScroll: true }`.
  - A click on a row sets the active key and focuses the row.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add a virtual list with roving focus`.

### Task 3: The HTTP list on `VirtualList`

**Files:**
- Modify: `src/renderer/HttpCalls.tsx`, `src/renderer/HttpCalls.test.tsx`, `e2e/http.spec.ts:95,154,166`

**Interfaces:**
- Consumes: `VirtualList`, `VirtualListHandle` (Task 2).
- Produces: `export function callName(l: ListedCall): string` in `HttpCalls.tsx`: the status (`"no status"` when null), the method, the URL (or the error's first line), the test's `nodeName`, joined with `", "` (R5.4).

- [ ] **Step 1: Write the failing tests** in `HttpCalls.test.tsx`:
  - "is a listbox of options named HTTP calls" (R5.3): `getByRole("listbox", { name: "HTTP calls" })`; with 3 calls, each `[role=option]` has `aria-setsize="3"` and `aria-posinset` 1, 2, 3; it carries `data-testid="http-call"`.
  - "names a call by status, method, URL and test" (R5.4): `callName` of a 404 GET `http://localhost/x` from "list users" is `"404, GET, http://localhost/x, list users"`; a null status reads `"no status, …"`.
  - "selects a call with the arrow keys and opens it": Tab focus on the list (the first option has `tabindex="0"`), `ArrowDown` → the second option has `aria-selected="true"` and the details pane shows its URL.
  - "scrolls to the selected call when the chips change" (R4.7): 200 calls, the 150th selected, `scrollTop` 0; toggle the `2xx` chip (all calls are 2xx) → the list's `scrollTop` is `scrollToShow` of its new index.
  - "keeps the calls in view still when an earlier call arrives" (R4.6): the call at 2 s is the top row; a `calls` event adds one that started at 1 s → `scrollTop` grew by 28.
  - "moves focus up when the focused call is cleared by a new run" (Review Focus 3): focus the third call; send `runStarted` with the keys of the third call's test → focus is on a remaining option, and no error.
  - Change "selects a row and marks it pressed" to `aria-selected`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/HttpCalls.test.tsx`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement.** In `HttpCalls.tsx`, `Row` becomes a `div` with today's classes, minus the button-only ones. Its `aria-pressed` and `onClick` move to `VirtualList`:
  - `role: "listbox"`, `itemRole: "option"`, `label: "HTTP calls"`, `scrollerTestid: "http-calls"`, `testid: "http-list"`, `rowHeight: 28`, `itemClass: "focus-visible:-outline-offset-2"`.
  - `keyOf` is `` `${ref.key}\n${ref.index}` ``; `selectedKey` is built from `state.selectedCall` the same way; `onSelect` is `props.selectCall(l.ref)`.
  - `attrs`: `data-testid`, `data-status`, `aria-selected`, `aria-label: callName(l)`, `aria-setsize`, `aria-posinset`.
  - The empty state keeps its own `div data-testid="http-calls"`.
  - An effect on `state.httpGroups` calls `reveal` for the selected call.
- [ ] **Step 4: Run** `npx vitest run src/renderer/HttpCalls.test.tsx`, then `npm test`. Expected: all PASS.
- [ ] **Step 5: Change** `aria-pressed` to `aria-selected` on calls in `e2e/http.spec.ts` (lines 95, 154 and 166). The chips keep `aria-pressed`.
- [ ] **Step 6: Run** `npm run e2e`. Expected: all pass. Paste the summary in PR 1.
- [ ] **Step 7: Commit** `feat(ui): draw only the http calls in view`, push, and open PR 1 as described in Global Constraints.

### Task 4: The tree's rows

**Files:**
- Create: `src/renderer/treeRows.ts`, `src/renderer/treeRows.test.ts`

**Interfaces:**
- Produces:

```ts
export interface TreeRow {
  node: TestNode;
  level: number; // aria-level: 1 for the root's children
  setSize: number; // shown siblings
  posInSet: number; // 1-based among shown siblings
  parentKey: string | null; // null at level 1
}
/** The rows a person can scroll to, in order: children of closed nodes left out, and non-matching rows left out under "Hide non-matching" (R2.1). The root itself is not a row. */
export function treeRows(root: TestNode, closed: ReadonlySet<string>, filter: FilterState): TreeRow[];
/** A row's accessible name (R5.2): label, status word in lower case, the flaky chip's words if any, then tags. */
export function rowName(label: string, status: Status, flaky: string | null, tags: readonly string[]): string;
```

- [ ] **Step 1: Write the failing tests** in `src/renderer/treeRows.test.ts`. Use the tree of `Tree.test.tsx` (`d:` › `d:tests` › `basic.feature` (2 scenarios, the first tagged `smoke`) and `outline.feature` › outline › 2 rows):
  - All open, no filter: the keys in order are `d:tests`, `f:tests/basic.feature`, `s:…#1`, `s:…#2`, `f:tests/outline.feature`, `o:…#1`, `s:…#1.1`, `s:…#1.2`; the levels are 1, 2, 3, 3, 2, 3, 4, 4; `outline.feature` has `posInSet` 2, `setSize` 2, `parentKey` `d:tests`.
  - `closed = new Set(["f:tests/basic.feature"])`: its two scenarios are gone and `basic.feature` is still there.
  - `{ include: ["smoke"], exclude: [], hideNonMatching: true }`: the rows are `d:tests`, `basic.feature` (`posInSet` 1, `setSize` 1) and `s:…#1`.
  - The same filter with `hideNonMatching: false`: all 8 rows.
  - `rowName("login.feature", "failed", "2 flaky", ["smoke"])` is `"login.feature, failed, 2 flaky, smoke"`; `rowName("login works", "failed", "flaky", [])` is `"login works, failed, flaky"`; `rowName("x", "notRun", null, [])` is `"x, not run"`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/treeRows.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** both functions in `src/renderer/treeRows.ts`. `treeRows` reads no statuses, so a run never changes the rows (R2.3). It uses `filterOn` and `matchingLeaves` from `src/shared/tags.ts` as `Tree.tsx` does today. `rowName` takes the status word from `STATUS_LABEL`.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): flatten the tree into the rows on show`.

### Task 5: A tooltip that a row can open

**Files:**
- Modify: `src/renderer/Tooltip.tsx`, `src/renderer/Tooltip.test.tsx`

**Interfaces:**
- Produces: `Tooltip` prop `pinned?: boolean`. While true and `text` is set, the tooltip shows as on hover; when it turns false, the tooltip hides. Hover and focus keep working as today.

- [ ] **Step 1: Write the failing tests**: with a `pinned` signal, `true` → `getByRole("tooltip")` has the text; `false` → no tooltip; `pinned` true with `text` undefined → no tooltip.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Tooltip.test.tsx`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement** with a `createEffect` on `props.pinned` that calls `show` or `hide`.
- [ ] **Step 4: Run** the same command. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): let a tooltip be opened by its owner`.

### Task 6: The tree on `VirtualList`

**Files:**
- Modify: `src/renderer/Tree.tsx`, `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: `VirtualList`, `VirtualListHandle` (Task 2), `treeRows`, `rowName`, `TreeRow` (Task 4), `Tooltip` `pinned` (Task 5).

- [ ] **Step 1: Write the failing tests** in `Tree.test.tsx`. Write one `it` per tree scenario of the spec that is not marked (e2e) or (perf), with the spec's values. The HTTP scenarios belong to Task 3. Add a `bigTree(tests: number)` helper (one folder, features of 10 scenarios) for the 2,000-test scenarios: jsdom draws 35 rows of 2,000. Add to them:
  - "keeps focus in the tree when a refresh removes the focused row" (Review Focus 4): focus `s:tests/basic.feature#2`; send a `project` event whose tree has no `basic.feature` → `document.activeElement` has `role="treeitem"` and is `d:tests`.
  - "starts everything open in another project": close `basic.feature`, send a `project` event with another `root` and the same tree → `basic.feature` has `aria-expanded="true"`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Tree.test.tsx`. Expected: the new tests FAIL; the existing ones still pass.
- [ ] **Step 3: Implement** in `Tree.tsx`:
  - `closed` is a `Set<string>` signal, emptied when `state.project?.root` changes (R2.2).
  - `rows` is a memo of `treeRows(tree, closed(), state.filter)`.
  - `VirtualList` gets `role: "tree"`, `itemRole: "treeitem"`, `label: "Tests"`, `testid: "tree"`, `rowHeight: 24`, `pad: 8` and `itemClass: "group rounded-sm focus-visible:-outline-offset-2"`. Also `selectedKey: state.selected`, `onSelect` is `props.select(row.node.key)`, and `keyOf` gives `row.node.key`.
  - `attrs`: `aria-level`, `aria-setsize`, `aria-posinset`, `aria-expanded` (rows with children), `aria-selected`, `aria-label` from `rowName`, `aria-keyshortcuts: "Control+Enter"`.
  - `onKey` does R3.3's Left, Right and Ctrl+Enter. Ctrl+Enter runs only when the row matches the filter and neither `run.active` nor `busy` is set.
  - The row content is today's `tree-node` div, indented by `(level − 1) × 16` px, without `content-visibility`. Its chevron, label and run buttons get `tabIndex={-1}`. The run tooltip reads `Run <label> (Ctrl+Enter)`; its `aria-label` stays `Run <label>`. The label's `Tooltip` gets `pinned={keyboardFocused() && cut()}` (R4.10).
  - An effect on `state.filter` calls `reveal(state.selected)` (R4.7).
- [ ] **Step 4: Fix existing tests** that depended on the nested `li`/`ul` or on the run tooltip text "Run X" ("keeps a row's tooltips on its row"). Change only their selectors and that text.
- [ ] **Step 5: Run** `npx vitest run src/renderer/Tree.test.tsx`, then `npm test`. Expected: all PASS.
- [ ] **Step 6: Commit** `feat(ui): draw only the tree rows in view, with arrow keys`.

### Task 7: e2e on a generated project

**Files:**
- Create: `e2e/big-project.ts`, `e2e/virtual.spec.ts`
- Modify: `e2e/helpers.ts`, `e2e/look.spec.ts:365`, any e2e test the new tree breaks

**Interfaces:**
- Produces: `writeBigProject(dir: string, featuresPerModule: number): { tests: number; calls: number }` in `e2e/big-project.ts`. It ports `docs/research/2026-10-29-big-project-perf/generate.mjs`: 5 per module gives 2,000 tests. It also produces `bigProject(featuresPerModule = 5): string` in `e2e/helpers.ts`, a temp folder cleaned up like `project()`.

- [ ] **Step 1: Write** `e2e/virtual.spec.ts`, one test per (e2e) scenario of the spec, on `bigProject()`:
  - "Only the rows in view are drawn": fewer than 100 `tree-node`; the tree's height is `data-rows × 24 + 16` px; after scrolling to the bottom, the last test's row is drawn.
  - "A moved-to row scrolls into view": hold Down for 40 rows.
  - "Focus survives scrolling away": scroll with `win.mouse.wheel`.
  - "A new tree keeps the rows in view still": run the 1,500th row with its run button; it is still the top row in view when `run-outcome` reads "Running…".
  - "A filter change scrolls to the selected row".

  "In view" means the row's box lies inside the scroller's box. Open the project with `openProject` and wait for `data-rows` to reach its total.
- [ ] **Step 2: Run** `npm run e2e`. Expected: `virtual.spec.ts` passes. `look.spec.ts`'s tab-stop count fails, since the tree is now one stop. Fix any other failure at its root, never by loosening the check.
- [ ] **Step 3: Update** `e2e/look.spec.ts:365`: set the bound a few under the new count, and keep the comment's "N at the time of writing" with the new N. The `area` set must still include `tree`.
- [ ] **Step 4: Run** `npm run e2e`. Expected: all pass.
- [ ] **Step 5: Commit** `test(e2e): check virtual lists on a 2,000-test project`.

### Task 8: Speed check and PR 2

**Files:**
- Create: `docs/checks/<date>-virtual-lists-perf.md`, `docs/checks/<date>-virtual-lists-perf/` (copies of `perf.spec.ts`, `scroll.spec.ts` and `playwright.config.ts` from the research note, with their selectors updated; the research note stays as it is)

- [ ] **Step 1: Copy and update the harness.** Tree rows are `[data-testid=tree-node]` (unchanged), calls are `[role=option]` with `aria-selected` (was `aria-pressed`), and the HTTP list's `data-rows` sits on `[data-testid=http-list]`. The tree's scroller is still `[data-testid=tree]`'s parent.
- [ ] **Step 2: Run** the note's section 8 steps against `npm run package` of `feature/virtual-tree`, 3 rounds at 2,000 tests and 3 at 10,000, with the copied config.
- [ ] **Step 3: Write the check record** in the format of `docs/checks/`: the machine, the commit, each R6 figure as a median against its limit and the note's figure, the result, and what still needs a person.
  - If a limit fails, stop and report it. Do not change the limit.
- [ ] **Step 4: Commit** `docs(checks): record virtual list speed at 10,000 tests`, push, and open PR 2 with `Closes #125`, `Closes #91`, the e2e summary and a link to the check record.
