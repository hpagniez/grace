# Console search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Find text in the Console tab and step through its matches, and step through its errors (ERROR lines and failed tests) from the tab row.

**Architecture:** Pure matching and error rules live in `src/renderer/consoleFind.ts`. The search state (text, open, current match, current error) lives in a controller made by `createConsoleSearch` in `Console.tsx`. `BottomPanel` owns it, so it survives the Console tab's content unmounting. The mounted `Console` attaches a small view handle (rows, top line, scrolling, the field) to the controller and paints with the CSS Custom Highlight API, so no row redraws. The store counts lines dropped at the cap, so a current match or error keeps its line.

**Tech Stack:** SolidJS 2 RC, TypeScript, Tailwind v4 tokens, Vitest (Node and jsdom), Playwright `_electron`.

**Spec:** [docs/specs/2026-11-03-console-search.md](../specs/2026-11-03-console-search.md)

## Global Constraints

- Solid 2 only (`AGENTS.md` § Solid 2 rules): `createEffect(compute, apply)`, `flush()`, `onSettled`, no `on`/`batch`/`classList`.
- Colours only through tokens; no raw palette class, no `dark:`, no `title=`. Icons only through `Icon` with `lucide` data (`Search`, `ArrowUp`, `ArrowDown`, `X`, `CircleX`); tooltips only through `Tooltip`.
- Idle costs nothing: no timer, poll or animation. Window `keydown` listeners only.
- No new dependency. No change in the main process, `Tree.tsx`, `Banner.tsx`, `Progress.tsx`.
- Highlight names: `console-match`, `console-match-current`, `console-error-current`.
- Copy, verbatim: placeholder `Find`; field name `Find in console`; counts `2 of 5`, `No results`, `1 error`, `3 errors`, `2 of 3 errors`; tooltips `Find (Ctrl+F)`, `Previous match (Shift+Enter)`, `Next match (Enter)`, `Close (Escape)`, `Previous error (Shift+F8)`, `Next error (F8)`. Button accessible names: `Find`, `Previous match`, `Next match`, `Close find`, `Previous error`, `Next error`.
- Test ids: `find-button` (tab row), `find-bar`, `find-field`, `find-count`, `find-prev`, `find-next`, `find-close`, `console-errors`, `errors-count`, `error-prev`, `error-next`. `console` stays on the scroller, whose first child stays the banner (`e2e/console.spec.ts`).
- Commits: Conventional Commits, scope `ui` (or `renderer`, `docs`), each ending with the session's attribution lines; body `Part of #73`.

## Review Focus

1. A query with regex characters (`a.b(`, `*`, `\`): matched as plain text. Test in Task 1.
2. A line where lower-casing changes length (`İstanbul user`): the range must sit on `user` in the original line. Task 1 matches with an escaped case-insensitive `RegExp`, never `toLowerCase` offsets.
3. Spaces in the query (` ERROR `) are kept, not trimmed. Test in Task 1.
4. A user's own printed line ending in `failed features:` must not start the block (the rule is the exact line). Test in Task 1.
5. Ctrl+F and F8 while a dialog is open, or under the Settings page: nothing happens. Tests in Tasks 4 and 5.

---

### Task 1: Matching and error rules

**Files:**
- Create: `src/renderer/consoleFind.ts`
- Test: `src/renderer/consoleFind.test.ts`

**Interfaces:**
- Produces:
  - `export interface Match { line: number; start: number; end: number }` (`line` is an index into the lines given)
  - `export const isErrorLine = (line: string): boolean` (` ERROR ` in the line)
  - `export function findMatches(lines: readonly string[], query: string): Match[]` (`[]` for `""`)
  - `export function errorLines(lines: readonly string[]): number[]` (indexes, ascending)
  - `export function startIndex(lineOf: readonly number[], top: number, dir: 1 | -1): number`: `dir 1` gives the first `i` with `lineOf[i] >= top`, else `0`; `dir -1` gives the last `i` with `lineOf[i] < top`, else the last index. `-1` for an empty list.
  - `export function stepIndex(count: number, current: number, dir: 1 | -1): number` wraps round.

- [ ] **Step 1: Write the failing tests**

```ts
describe("findMatches (spec R3.3)", () => {
  it("counts every occurrence, case aside, without overlap", () => {
    expect(findMatches(["get user 42", "create USER", "aaaa"], "user")).toEqual([
      { line: 0, start: 4, end: 8 },
      { line: 1, start: 7, end: 11 },
    ]);
    expect(findMatches(["aaaa"], "aa")).toEqual([
      { line: 0, start: 0, end: 2 },
      { line: 0, start: 2, end: 4 },
    ]);
  });
  it("takes the text as plain text", () => {
    expect(findMatches(["a.b(c", "axb(c", "1*2\\3"], "a.b(")).toEqual([{ line: 0, start: 0, end: 4 }]);
    expect(findMatches(["1*2\\3"], "*2\\")).toEqual([{ line: 0, start: 1, end: 4 }]);
  });
  it("keeps offsets on the line as written when case folding changes its length", () => {
    expect(findMatches(["İstanbul user"], "user")).toEqual([{ line: 0, start: 9, end: 13 }]);
  });
  it("keeps the spaces of the text", () => {
    expect(findMatches(["a ERROR b", "aERRORb"], " ERROR ")).toEqual([{ line: 0, start: 1, end: 8 }]);
  });
  it("finds nothing for no text", () => expect(findMatches(["a"], "")).toEqual([]));
});

describe("errorLines (spec R1)", () => {
  it("takes ERROR lines and failed tests, not WARN lines", () => {
    // the spec's R1 scenario, verbatim lines (Karate 2.1.4)
    const lines = [
      "09:46:08.476 [virtual-36] INFO  karate.scenario - hello",
      "09:46:08.502 [virtual-40] WARN  karate.scenario - careful",
      "09:46:08.502 [virtual-46] ERROR karate.scenario - [ERROR] custom boom",
      "failed features:",
      "  C:\\work\\a.feature",
      "    - fails match",
      "      C:\\work\\a.feature:7",
      "      ==========",
      "    - line 2",
      "============================================================",
      "    - after the block",
    ];
    expect(errorLines(lines)).toEqual([2, 5, 8]);
  });
  it("starts the block only at the exact line", () => {
    expect(errorLines(["12:00 [main] INFO  karate.scenario - failed features:", "    - not a test"])).toEqual([]);
  });
});

describe("stepping", () => {
  it("starts from the top of the view, wrapping", () => {
    expect(startIndex([3, 7, 9], 5, 1)).toBe(1);
    expect(startIndex([3, 7, 9], 5, -1)).toBe(0);
    expect(startIndex([3, 7, 9], 10, 1)).toBe(0);
    expect(startIndex([3, 7, 9], 0, -1)).toBe(2);
    expect(startIndex([], 0, 1)).toBe(-1);
  });
  it("steps round", () => {
    expect(stepIndex(3, 2, 1)).toBe(0);
    expect(stepIndex(3, 0, -1)).toBe(2);
  });
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/renderer/consoleFind.test.ts`
Expected: FAIL, `consoleFind` not found.

- [ ] **Step 3: Implement `src/renderer/consoleFind.ts`**

`findMatches`: one `RegExp` per call from the escaped text with flags `gi`, then `matchAll` on each line (offsets are on the line as written). `errorLines`: one pass; inside the block (after a line `=== "failed features:"`, until a line that starts with `=`), a line that starts with `"    - "` and not a fifth space is an error; outside it, `isErrorLine`.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/renderer/consoleFind.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): match console text and find its errors`

---

### Task 2: The store counts dropped lines

**Files:**
- Modify: `src/renderer/reduce.ts` (`AppState`, `initialState`, cases `project`, `runStarted`, `console`)
- Test: `src/renderer/reduce.test.ts`

**Interfaces:**
- Produces: `AppState.consoleDropped: number`: the lines dropped at `CONSOLE_LIMIT` since the run started. A line's number is `consoleDropped + index`.

- [ ] **Step 1: Write the failing test**

```ts
it("counts the console lines dropped at the cap, from 0 at each run", () => {
  const s = initialState();
  reduce(s, { type: "console", lines: Array.from({ length: CONSOLE_LIMIT + 5 }, (_, i) => `l${i}`) });
  reduce(s, { type: "console", lines: ["a", "b"] });
  expect(s.consoleDropped).toBe(7);
  expect(s.console[0]).toBe("l7");
  reduce(s, { type: "runStarted", /* fields as the existing runStarted test builds them */ });
  expect(s.consoleDropped).toBe(0);
});
```

Also assert `consoleDropped` is `0` after `{ type: "project", project: null, … }` in the existing "a closed project leaves no console" path.

- [ ] **Step 2: Run it to see it fail**: `npx vitest run src/renderer/reduce.test.ts`, FAIL on `consoleDropped`.
- [ ] **Step 3: Implement**: the field in `AppState` and `initialState` (`0`); in `console`, add the overflow (`max(0, old + new − CONSOLE_LIMIT)`) before slicing; reset to `0` where `s.console = []`.
- [ ] **Step 4: Run it to see it pass**, and `npm run typecheck`.
- [ ] **Step 5: Commit** `feat(ui): count the console lines dropped at the cap`

---

### Task 3: Match colours and the test stubs

**Files:**
- Modify: `src/index.css` (`@theme` colour tokens, the dark block, `@layer base`)
- Modify: `src/renderer/tokens.test.ts` (`TOKENS`, a new test)
- Modify: `test/jsdom-setup.ts`

**Interfaces:**
- Produces: tokens `--color-found`, `--color-found-current`; the three `::highlight()` rules; in jsdom, `CSS.highlights` (a `Map`), a `Highlight` class (a `Set` of ranges built from its arguments) and `Element.prototype.checkVisibility` (returns `true`).

- [ ] **Step 1: Write the failing test** in `tokens.test.ts`: add `"found"` and `"found-current"` to `TOKENS`, and inside the per-theme `describe`:

```ts
it("paints console matches readably, the current one stronger (console search R4.2)", () => {
  expect(contrast(t.ink, t.found), "ink on found").toBeGreaterThanOrEqual(4.5);
  expect(contrast(t.ink, t["found-current"]), "ink on found-current").toBeGreaterThanOrEqual(4.5);
  expect(contrast(t["found-current"], t.sunken)).toBeGreaterThan(contrast(t.found, t.sunken));
});
```

And a CSS test: `index.css` holds `::highlight(console-match)`, `::highlight(console-match-current)` and `::highlight(console-error-current)` with `background-color: var(--color-found)`, `var(--color-found-current)` and `var(--color-failed-tint)`.

- [ ] **Step 2: Run it to see it fail**: `npx vitest run src/renderer/tokens.test.ts`.
- [ ] **Step 3: Implement**: start from light `#f2e3a6` / `#e8b84a`, dark `#3d3519` / `#6b5216`, and adjust until the test passes. Put the rules in `@layer base`. Add the three stubs to `test/jsdom-setup.ts`, each with a one-line why, like the `ResizeObserver` one.
- [ ] **Step 4: Run it to see it pass**: `npx vitest run src/renderer/tokens.test.ts src/renderer/palette.test.ts`.
- [ ] **Step 5: Commit** `feat(ui): add the console match colours`

---

### Task 4: The find bar

**Files:**
- Modify: `src/renderer/Console.tsx`
- Modify: `src/renderer/BottomPanel.tsx`
- Modify: `src/renderer/ui.ts` (the bar's class string, `findBar`)
- Test: `src/renderer/Console.test.tsx` (new `describe("find (console search R3)")`)

**Interfaces:**
- Consumes: Task 1 (`findMatches`, `startIndex`, `stepIndex`, `Match`), Task 2 (`consoleDropped`), Task 3 (stubs, colours).
- Produces, in `Console.tsx`:

```ts
/** What the mounted Console lends the search: its rows and its scroller. */
export interface ConsoleView {
  row(index: number): HTMLElement | undefined;   // the line's element
  topIndex(): number;                             // the first line in view (binary search on offsetTop)
  show(index: number, onlyIfHidden: boolean): void; // centres the line in the scroller
  focusField(): void;                             // focuses the find field and selects its text
}
export interface ConsoleSearch {
  query(): string; type(text: string): void;
  open(): boolean; openBar(): void; close(): void;
  matches(): readonly Match[]; current(): number; step(dir: 1 | -1): void;
  errors(): readonly number[]; currentError(): number; stepError(dir: 1 | -1): void; // Task 5
  attach(view: ConsoleView): () => void;          // returns the detach
}
export function createConsoleSearch(o: {
  lines: () => readonly string[];
  dropped: () => number;
  shown: () => boolean;        // the Console tab is the panel's tab
  visible: () => boolean;      // the panel is on screen (checkVisibility), spec R5.3
  showConsole: () => void;
}): ConsoleSearch;
export function Console(props: { lines: readonly string[]; search: ConsoleSearch; children?: JSX.Element }): JSX.Element;
```

The current match and error are signals of `{ line: number /* dropped + index */; start: number }` and `number | null`. `current()` and `currentError()` are indexes worked out from them; when a held one is gone, an effect's apply half re-picks per R3.10 (`startIndex` from `view.topIndex()`), without scrolling. `matches()` is `[]` while the bar is closed (R3.11).

- [ ] **Step 1: Write the failing tests.** Render `BottomPanel` with a store from `createAppStore(fakeApi)`, wired as `App.tsx` wires it (`state`, `showPanel`). Emit console events as `Console.test.tsx` already does, and `flush()` after each event or key. Reset `CSS.highlights` in `afterEach`. Tests:
  - `Ctrl+F opens the find bar from another tab`: panel `http`, `keydown` Ctrl+F on `window`. Then `tab-console` is selected, `find-bar` exists and `find-field` is `document.activeElement`.
  - `counts matches and steps with Enter and Shift+Enter, wrapping`: lines `get user 42`, `create user`; type `user`. `find-count` reads `1 of 2`; Enter gives `2 of 2`, Enter `1 of 2`, Shift+Enter `2 of 2`. The buttons `find-next` and `find-prev` do the same.
  - `shows No results, and nothing for no text`. `find-count` has `role="status"`; `find-field` has the accessible name `Find in console` and the placeholder `Find`; `find-prev`, `find-next` and `find-close` are named `Previous match`, `Next match` and `Close find`, with the tooltips in Global Constraints.
  - `paints matches without redrawing the rows`: keep the console's row elements, then type `user`. `CSS.highlights.get("console-match")` holds 2 ranges, each `toString() === "user"`; `console-match-current` holds the first; the row elements are the same objects.
  - `Escape closes the bar, keeps the text and gives the focus back`: focus `tab-history`, Ctrl+F, type `user`, Escape. `find-bar` is gone, no highlight is set, focus is on `tab-history`. Ctrl+F again: the field holds `user` with `selectionStart` 0 and `selectionEnd` 4, and `console-match` is set again.
  - `keeps the text across a tab switch and a new run`: show History, then Console: the bar is open with `user`. Emit `runStarted`, then lines `user again`: `1 of 1`.
  - `keeps the current match on its line as lines arrive, without scrolling`: fake `scrollHeight` and `clientHeight` as the existing tests do. Step to a match, note `scrollTop`, emit `user later`. The count is `… of 3`, `console-match-current`'s range is in the same line element, and `scrollTop` has not changed.
  - `keeps the current match's line when older lines drop at the cap`: `CONSOLE_LIMIT` lines `l<i>`, line 9000 `user 9000`; type `user`, then emit 100 lines. The current range's `startContainer.textContent` is `user 9000`.
  - `does not search while the bar is closed`: type `user`, Escape, emit `user more`: no highlight is set.
  - `ignores Ctrl+F while a dialog is open, and under the Settings page`: append `<dialog open>` to `document.body`, then Ctrl+F: no `find-bar`. Remove it, stub the panel section's `checkVisibility` to `false`, then Ctrl+F: no `find-bar`.
  - `the Find button opens the bar`: `find-button` (accessible name `Find`, tooltip `Find (Ctrl+F)`) is on the tab row only while the Console tab shows.
- [ ] **Step 2: Run them to see them fail**: `npx vitest run src/renderer/Console.test.tsx`.
- [ ] **Step 3: Implement.**
  - `Console`: wrap the scroller in `<div class="relative flex min-h-0 flex-1 flex-col">`, with the bar (`<Show when={search.open()}>`, absolutely placed at the top right, `findBar` from `ui.ts`: `bg-raised shadow-float` with a border and padding) beside the scroller, never inside it. The scroller gets `relative`, so a row's `offsetTop` is relative to it.
  - Rows: `el.children[el.childElementCount - lines.length + index]`.
  - Painting: one `createEffect`, whose compute reads `matches()`, `current()` and `open()`. Its apply calls `CSS.highlights.set(name, new Highlight(...ranges))`, or `delete` when there are none, and its cleanup deletes the highlights.
  - The field: `onInput`, then `search.type`; `onKeyDown`: Enter steps 1, Shift+Enter steps −1, Escape closes.
  - `createConsoleSearch` registers one window `keydown` listener (with `onCleanup`) for Ctrl+F. It does nothing while a `dialog[open]` exists or while `visible()` is false. `openBar` remembers `document.activeElement`, calls `showConsole()`, sets open, calls `flush()`, then `view.focusField()`. `close` restores the focus when the element `isConnected`.
  - `BottomPanel`: `const search = createConsoleSearch({...})`, using `section.checkVisibility()` for `visible`. It passes `search` to `Console` and, while the Console tab shows, puts `find-button` at the tab row's right end (`btnIconSmall`, `Icon` `Search`, size 14).
- [ ] **Step 4: Run them to see them pass**: `npx vitest run src/renderer/Console.test.tsx src/renderer/BottomPanel.test.tsx`, then `npm run typecheck && npm run lint`.
- [ ] **Step 5: Commit** `feat(ui): find text in the console`

---

### Task 5: Errors on the tab row

**Files:**
- Modify: `src/renderer/Console.tsx` (`ConsoleErrors` component, the `errors` part of `createConsoleSearch`, F8)
- Modify: `src/renderer/BottomPanel.tsx` (renders `ConsoleErrors` before `find-button`)
- Test: `src/renderer/Console.test.tsx` (new `describe("errors (console search R2)")`)

**Interfaces:**
- Consumes: Task 1 (`errorLines`, `startIndex`, `stepIndex`), Task 4 (`ConsoleSearch`, `ConsoleView`, the painting effect).
- Produces: `export function ConsoleErrors(props: { search: ConsoleSearch }): JSX.Element`. `errors()` is `[]` while `shown()` is false.

- [ ] **Step 1: Write the failing tests** (same harness as Task 4):
  - `shows the count and steps through the errors with the buttons and F8, wrapping`: 3 errors, `errors-count` reads `3 errors`. `error-next` gives `1 of 3 errors`; F8, F8 give `3 of 3 errors`; F8 gives `1 of 3 errors`; Shift+F8 gives `3 of 3 errors`. The count has `role="status"`.
  - `paints the current error's line`: `CSS.highlights.get("console-error-current")` holds one range whose `toString()` is the whole line.
  - `has no group and no F8 without errors`.
  - `belongs to the Console tab`: with History shown, there is no `console-errors` and F8 changes nothing.
  - `names its buttons`: `Previous error` and `Next error`, with tooltips `Previous error (Shift+F8)` and `Next error (F8)`. F8 is ignored with a `dialog[open]` or when `visible()` is false.
- [ ] **Step 2: Run them to see them fail.**
- [ ] **Step 3: Implement**: the `CircleX` icon and count in `text-failed`, and two `btnIconSmall` buttons (`ArrowUp`, `ArrowDown`) in `<div role="group" aria-label="Errors">`. F8 joins the Ctrl+F listener. `stepError` shows the line centred (`show(index, false)`). The painting effect also sets `console-error-current` with `range.selectNodeContents(row)`.
- [ ] **Step 4: Run them to see them pass**, plus `npm run typecheck && npm run lint`.
- [ ] **Step 5: Commit** `feat(ui): step through the console's errors`

---

### Task 6: End-to-end

**Files:**
- Create: `e2e/console-find.spec.ts`

**Interfaces:**
- Consumes: `launch`, `openProject`, `tempDir`, `expect`, `test` from `e2e/helpers.ts`; the test ids above.

- [ ] **Step 1: Write the tests.** Write each project into a `tempDir("kk-find-")` with `writeFileSync`, as one feature file; no fixture changes.
  - `find in a real run`: two scenarios that each `* print 'zebra-42'`, named without that text. Run all, wait for `Finished`, press Ctrl+F, type `zebra-42`. `find-count` reads `1 of 2` and `await win.evaluate(() => CSS.highlights.get("console-match")?.size)` is `2`. Enter gives `2 of 2`.
  - `step through a failing run's errors`: `Scenario: fails` with `* match 1 == 2`, and `Scenario: logs` with `* karate.logger.error('custom boom')`. After the run, `errors-count` reads `2 errors`. F8 gives `1 of 2 errors`, F8 gives `2 of 2 errors`, and the `console-error-current` range text is `    - fails`.
  - `a full console`: send `{ type: "console", lines }` with 10,000 lines `line <i> match-me` through `app:event`, as `e2e/panel.spec.ts` does, on an open empty project. Ctrl+F, type `match-me`: `find-count` matches `/^\d+ of 10000$/`. Log the keystroke's time, `performance.now()` around the last `type` until the count changes, with `test.info().annotations`, for the PR's Cost line. Do not assert it.
- [ ] **Step 2: Ask the parent session for a go, then run** `npm run e2e -- console-find`. Expected: 3 passed.
- [ ] **Step 3: Commit** `test(e2e): find and step through errors in the console`

---

## After the tasks

- Full `npm test` and `npm run e2e`, each after the parent session's go. Paste the e2e summary and the keystroke time in the PR.
- Draft PR into `develop`, `Closes #73`. Read the `perf` comment and explain any worse measure.
- After the merge: point #73's `Spec:` line at `docs/specs/2026-11-03-console-search.md` on `develop` (AGENTS.md § Doc rules).
