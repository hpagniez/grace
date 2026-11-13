---
status: built
issue: 222
summary: "A right-click on a tree row shows Windows' menu with Open in VS Code, which opens the row's file where Details would"
---

# Open in VS Code from the Test Tree Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A right-click on a tree row shows Windows' menu with "Open in VS Code", which opens the row's file where the Details button or error link would.

**Architecture:** The tree's list gets one `contextmenu` listener. It works out the row's path and line in the window and asks main over a new IPC call, `editorMenu`. Main checks the request, shows an Electron `Menu` with one item, and the item runs the same code as the `openInEditor` handler.

**Tech Stack:** Electron `Menu`, SolidJS 2, Vitest (jsdom and Node), Playwright `_electron`.

**Spec:** [docs/specs/2026-11-03-open-in-editor-from-tree.md](../specs/2026-11-03-open-in-editor-from-tree.md), building on [docs/specs/2026-11-03-open-in-editor.md](../specs/2026-11-03-open-in-editor.md).

## Global Constraints

- The menu has one item, labelled exactly `Open in VS Code`. A folder shows no menu (R1.2).
- A right-click does not change the selected row (R1.3). The menu shows during a run (R1.4).
- A malformed request shows the notice `Invalid request` and no menu (R3.1). The request checks are the ones `openInEditor` makes today: the path is a string of at most 1000 characters, and the line is an integer of 1 or more. The point is two finite numbers.
- Choosing the item does exactly what `openInEditor` does (R3.2): one code path in `src/main.ts`, not a copy.
- No node is added to a drawn row, and there is no listener per row: one listener on the list element (spec Cost).
- AGENTS.md: the typed preload API only; Solid 2 rules; no `title=`; conventional commits ending with the attribution lines.

## Review Focus

- A right-click on a row's run or debug button, or on its chevron, still shows that row's menu: the event's target is inside the row, not the row itself. Task 2 tests the run button.
- A right-click on the list's padding, below the last row, shows no menu and throws nothing. Task 2 tests it.
- A failed test whose error names a called feature opens that file, not the row's own. Task 2 uses a place whose path differs from the row's file.
- A right-click during a run still asks for the menu. Task 2 tests it.
- The menu chosen after the project was closed or changed. The item goes through `runner.editorFile` at click time, so the current project decides. No test: the item's code is the handler's, which is already tested.

---

### Task 1: Main shows the menu

**Files:**
- Modify: `src/main/editor.ts`
- Modify: `src/main/editor.test.ts`
- Modify: `src/shared/ipc.ts` (`CH`, `Api`)
- Modify: `src/preload.ts`
- Modify: `src/main.ts` (the `CH.openInEditor` handler, about line 293)

**Interfaces:**
- Produces: `validRequest(file: unknown, line: unknown): boolean` and `validPoint(x: unknown, y: unknown): boolean` in `src/main/editor.ts`.
- Produces: `CH.editorMenu = "editor:menu"`, and `Api.editorMenu(file: string, line: number, x: number, y: number): Promise<void>`. Its doc comment says: shows Windows' menu with "Open in VS Code" at the point, in window pixels; choosing it does what `openInEditor` does (docs/specs/2026-11-03-open-in-editor-from-tree.md).

- [ ] **Step 1: Write the failing tests** in `src/main/editor.test.ts`, under a new `describe("validRequest and validPoint (open-in-editor-from-tree R3.1)")`:

```ts
it("takes a path of at most 1000 characters and a line of 1 or more", () => {
  expect(validRequest("tests/a.feature", 1)).toBe(true);
  expect(validRequest("a".repeat(1000), 12)).toBe(true);
  for (const [file, line] of [["tests/a.feature", 0], ["tests/a.feature", 1.5], ["tests/a.feature", "12"], ["a".repeat(1001), 1], [null, 1]])
    expect(validRequest(file, line)).toBe(false);
});

it("takes a point of two finite numbers", () => {
  expect(validPoint(0, 0)).toBe(true);
  expect(validPoint(30.5, 40)).toBe(true);
  for (const [x, y] of [["12", 4], [1, Number.NaN], [Infinity, 1], [1, undefined]])
    expect(validPoint(x, y)).toBe(false);
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `npx vitest run src/main/editor.test.ts`
Expected: FAIL, `validRequest` and `validPoint` are not exported.

- [ ] **Step 3: Implement** `validRequest` and `validPoint` in `src/main/editor.ts`. `validRequest` holds the exact check now inline in the `CH.openInEditor` handler.

- [ ] **Step 4: Run them to verify they pass**

Run: `npx vitest run src/main/editor.test.ts`
Expected: PASS.

- [ ] **Step 5: Wire main.** In `src/main.ts`:
  - Move the body of the `CH.openInEditor` handler after its check into `async function openInEditor(file: string, line: number): Promise<void>`.
  - The handler then shows `Invalid request` unless `validRequest(file, line)`, and otherwise calls it.
  - Add `handle(CH.editorMenu, (file, line, x, y) => …)`. It shows `Invalid request` unless `validRequest(file, line) && validPoint(x, y)`. Otherwise it calls `Menu.buildFromTemplate([{ label: "Open in VS Code", click: … }]).popup({ x: Math.round(x), y: Math.round(y) })`.
  - The `click` calls `openInEditor(file, line)` and turns a rejection into a notice, as `handle` does.
  - Add the channel to `CH`, the method to `Api` and the call to `src/preload.ts`, next to `openInEditor`.

- [ ] **Step 6: Check types and lint**

Run: `npm run typecheck && npm run lint`
Expected: no errors. Every `Api` mock that lists methods compiles: they are cast with `as unknown as Api`.

- [ ] **Step 7: Commit**

```bash
git add src/main/editor.ts src/main/editor.test.ts src/shared/ipc.ts src/preload.ts src/main.ts
git commit -m "feat(ipc): show Windows' menu to open a file in VS Code"
```

### Task 2: The tree asks for the menu

**Files:**
- Modify: `src/renderer/nodes.ts` (add `failedAt`)
- Modify: `src/renderer/Details.tsx` (`FAILED_AT` and `ErrorText` use `failedAt`)
- Modify: `src/renderer/VirtualList.tsx` (an `onContextMenu` prop)
- Modify: `src/renderer/Tree.tsx` (`editorPlace` and the listener)
- Test: `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: `Api.editorMenu(file, line, x, y)` from Task 1.
- Produces: `failedAt(error: string): { file: string; line: number } | null` in `src/renderer/nodes.ts`. It reads the error's first line with today's `FAILED_AT` (`/^([A-Za-z]:[\\/].*):(\d+)$/`).
- Produces: a `VirtualList` prop `onContextMenu?(e: MouseEvent, item: T): void`. It is called for a `contextmenu` event whose target is a drawn row or inside one, with that row's item. It is not called for the list's padding.
- Produces: `editorPlace(node: TestNode, state: AppState): { file: string; line: number } | null` in `src/renderer/Tree.tsx`. A folder gives null. A scenario or row with `deriveStatus(node, state.statuses) === "failed"` and `failedAt(state.results[node.key]?.error ?? "")` gives that place. Anything else gives `{ file: node.file, line: node.line ?? 1 }`.

- [ ] **Step 1: Write the failing tests** in `src/renderer/Tree.test.tsx`:
  - Add `editorMenu: vi.fn(async () => {})` to `setup()`'s `api`.
  - Add a `describe("tree: open in VS Code (open-in-editor-from-tree R1, R2)")`.
  - `label(name)` below means `screen.getByRole("button", { name, exact: true })`, the row's label button.

```ts
it("asks for the menu at the pointer, for a test at its line, and keeps the selection", () => {
  const { api, load } = setup();
  load();
  fireEvent.contextMenu(label("failing match"), { clientX: 30, clientY: 40 });
  expect(api.editorMenu).toHaveBeenCalledWith("tests/basic.feature", 14, 30, 40);
  expect(rowByKey("s:tests/basic.feature#2").parentElement!.getAttribute("aria-selected")).toBe("false");
});

it("asks from a row's run button and from the focused row itself, as the menu key does", () => {
  const { api, load } = setup();
  load();
  fireEvent.contextMenu(screen.getByRole("button", { name: "Run add 2 and 3" }));
  fireEvent.contextMenu(rowByKey("s:tests/outline.feature#1.2").parentElement!);
  expect(api.editorMenu).toHaveBeenNthCalledWith(1, "tests/outline.feature", 7, 0, 0);
  expect(api.editorMenu).toHaveBeenNthCalledWith(2, "tests/outline.feature", 7, 0, 0);
});

it("opens a feature at line 1, and asks nothing for a folder or the list's padding", () => {
  const { api, load } = setup();
  load();
  fireEvent.contextMenu(rowByKey("f:tests/basic.feature"));
  expect(api.editorMenu).toHaveBeenCalledWith("tests/basic.feature", 1, 0, 0);
  api.editorMenu.mockClear();
  fireEvent.contextMenu(rowByKey("d:tests"));
  fireEvent.contextMenu(screen.getByTestId("tree"));
  expect(api.editorMenu).not.toHaveBeenCalled();
});

it("opens a failed test at its failed step during a run, even in a called feature", () => {
  const { api, send, load } = setup();
  load();
  send({ type: "runStarted", selection: "d:", runId: "20261025-120000-000", keys: ["s:tests/basic.feature#2"], total: 1, startedAt: Date.now(), env: null, debug: false });
  send({ type: "scenarios", updates: [{ key: "s:tests/basic.feature#2", status: "failed", error: String.raw`C:\p\tests\common.feature:20` + "\nmatch failed" }] });
  fireEvent.contextMenu(label("failing match"));
  expect(api.editorMenu).toHaveBeenCalledWith(String.raw`C:\p\tests\common.feature`, 20, 0, 0);
});

it("opens a failed test without a place at its own line", () => {
  // the same runStarted as above, then the update with error: "boom"
  fireEvent.contextMenu(label("failing match"));
  expect(api.editorMenu).toHaveBeenCalledWith("tests/basic.feature", 14, 0, 0);
});
```

- [ ] **Step 2: Run them to verify they fail**

Run: `npx vitest run src/renderer/Tree.test.tsx`
Expected: the new tests FAIL; `editorMenu` is never called. The others pass.

- [ ] **Step 3: Implement**
  - `failedAt` in `nodes.ts`; `ErrorText` in `Details.tsx` uses it in place of its own `FAILED_AT`.
  - The `VirtualList` prop: one `onContextMenu` on the role element. It finds the row as the target or its ancestor whose `parentElement` is the list, and the item by that row's `data-key`.
  - `editorPlace` and, in `Tree.tsx`'s `VirtualList`, `onContextMenu={(e, row) => …}`. When `editorPlace` gives a place, the listener calls `e.preventDefault()` and `props.api.editorMenu(place.file, place.line, e.clientX, e.clientY)`.

- [ ] **Step 4: Run them to verify they pass**

Run: `npx vitest run src/renderer/Tree.test.tsx src/renderer/Details.test.tsx src/renderer/VirtualList.test.tsx`
Expected: PASS, the Details error-link tests included.

- [ ] **Step 5: Commit**

```bash
git add src/renderer/nodes.ts src/renderer/Details.tsx src/renderer/VirtualList.tsx src/renderer/Tree.tsx src/renderer/Tree.test.tsx
git commit -m "feat(ui): open a tree row in VS Code from its right-click menu"
```

### Task 3: End to end

**Files:**
- Modify: `e2e/editor.spec.ts`
- Modify: `docs/specs/2026-11-03-open-in-editor-from-tree.md` (only if a ruling changes what it says)

**Interfaces:**
- Consumes: `CH.editorMenu` wired in Task 1, the tree listener from Task 2, `stubVsCode` in `e2e/editor.spec.ts`.
- Produces: `stubMenu(app)` in `e2e/editor.spec.ts`. It replaces `Menu.prototype.popup` in main with one that keeps each menu in `globalThis.menus`. It returns `{ labels(): Promise<string[] | undefined>; choose(): Promise<void> }` for the last menu: its items' labels, and a call to its first item's `click()`.

- [ ] **Step 1: Write the failing test** "Open a scenario from the tree", on the `clean` fixture, as "Open a scenario in VS Code" does:
  1. Right-click the label of "failing match": `node(win, "failing match").getByRole("button", { name: "failing match", exact: true }).click({ button: "right" })`.
  2. Expect `labels()` to poll to `["Open in VS Code"]`, and the row's `aria-selected` to be `"false"`.
  3. `choose()`, then expect `opened` to poll to `[`${link}:15`]`.
  4. Run the feature and wait for `Finished`. Right-click again, `choose()`, and expect `[`${link}:15`, `${link}:20`]`.
  5. Focus the row and press `Shift+F10`. Expect a third menu.

  In "main refuses what it must not open", call `stubMenu`. Before each of `editorMenu("tests/basic.feature", 0, 1, 1)` and `editorMenu("tests/basic.feature", 1, "12", 4)`, ask `openInEditor` for an outside path, as the loop there does. Expect `Invalid request` each time, and `globalThis.menus` to stay empty.

- [ ] **Step 2: Run it**

Run: `npm run e2e -- e2e/editor.spec.ts` (ask the parent session for the machine first).
Expected: PASS. This test is written after the code, since each e2e run packages the app; the unit tests of Tasks 1 and 2 are the failing-first gate.

If Chromium sends no `contextmenu` for `Shift+F10` from Playwright's keys, keep the jsdom test of the row-itself path. Drop step 1.5 with a ledgered ruling.

- [ ] **Step 3: Commit**

```bash
git add e2e/editor.spec.ts
git commit -m "test(e2e): open a test in VS Code from the tree's menu"
```
