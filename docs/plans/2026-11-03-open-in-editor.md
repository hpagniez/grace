---
status: built
issue: 68
summary: "One click in Details opens a test's feature file in VS Code at the test's line, or at the line Karate names in a failed test's error"
---

# Open in editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One click in the Details pane opens a test's feature file in VS Code at the test's line, or at the line Karate names in a failed test's error.

**Architecture:** The window sends a path and a line over one new IPC call. Main resolves the path (`Runner.editorFile`: tree path through the classpath, or absolute as is), keeps it only if it is a file inside the open project's real folder (`projectFile` in a new `src/main/editor.ts`), checks Windows has a `vscode://` handler, then hands `vscodeUrl(file, line)` to `shell.openExternal`.

**Tech Stack:** TypeScript, Electron (`shell.openExternal`, `app.getApplicationNameForProtocol`), Node `fs`/`path`, SolidJS 2, Vitest, Playwright.

**Spec:** [docs/specs/2026-11-03-open-in-editor.md](../specs/2026-11-03-open-in-editor.md)

## Global Constraints

- Notice copy, exact: `Invalid request` / `Grace opens only files of the open project` / `Open in VS Code needs VS Code installed`.
- Button copy, exact: `Open in VS Code`; icon `ExternalLink` from `lucide`; class `ghostSmall`.
- Link: `vscode://file/<path, "\" → "/", encodeURI, then "#" → "%23", "?" → "%3F">:<line>`.
- Request limits: path a string of at most 1000 characters; line `Number.isInteger` and at least 1.
- Do not edit `Tree.tsx`, `Banner.tsx`, `Progress.tsx`, `src/main/karate/*`, `src/shared/events.ts` or `e2e/helpers.ts` (PR #215). In `runner.ts`, add only the one method of Task 2.
- No timer, poll or listener; nothing runs until a click.

## Review Focus

- A row with no line (dynamic Examples): the button does not show, rather than sending line null.
- A project path with a space, `%`, `#` or `é`: the link still opens that file.
- An error whose first line has a lowercase drive or `/` separators (`c:/p/a.feature:3`): it is still a link.
- A path that differs from the project's only in letter case: Windows treats it as the same file, so it is inside.
- A folder named `..data` inside the project: inside, not outside; the project folder itself is not a file.

---

### Task 1: `projectFile` and `vscodeUrl`

**Files:**
- Create: `src/main/editor.ts`, `src/main/editor.test.ts`

**Interfaces:**
- Produces:
  ```ts
  /** `file`'s real path if it is an existing file inside `root`'s real folder (R3.3); null otherwise. */
  export function projectFile(root: string, file: string): string | null;
  /** VS Code's link to `file` at `line` (R4.2). */
  export function vscodeUrl(file: string, line: number): string;
  ```

- [ ] **Step 1: Write the failing tests** in `src/main/editor.test.ts` (Node), on a `mkdtempSync` project with `tests/a.feature`, removed in `afterEach`:
  - `projectFile(root, join(root, "tests/a.feature"))` equals `realpathSync.native` of it; so does the same path with its letters upper-cased.
  - `join(root, "..data/b.feature")` (created) is kept.
  - `null` for: `join(root, "../outside.feature")` (created next to the project), a file in another `mkdtempSync` folder, `tests/gone.feature` (never created), `root` itself, and `join(root, "link/out.feature")` where `link` is `symlinkSync(otherDir, join(root, "link"), "junction")` and `otherDir/out.feature` exists.
  - `vscodeUrl("C:\\My Project\\a#b.feature", 7)` is `"vscode://file/C:/My%20Project/a%23b.feature:7"`.
  - `vscodeUrl("C:\\p\\100%\\é?.feature", 2)` is `"vscode://file/C:/p/100%25/%C3%A9%3F.feature:2"`.

- [ ] **Step 2: Run** `npx vitest run src/main/editor.test.ts`. Expected: FAIL, module not found.

- [ ] **Step 3: Implement** both in `src/main/editor.ts`. `projectFile`: `realpathSync.native` both, `statSync(real).isFile()`, then the `path.relative` test (`rel !== ".." && !rel.startsWith(`..${path.sep}`) && !path.isAbsolute(rel)`), inside a `try` that returns null on any error. `vscodeUrl`: the Global Constraints rule.

- [ ] **Step 4: Run** `npx vitest run src/main/editor.test.ts`. Expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): check and link a file to open in VS Code`.

### Task 2: `Runner.editorFile`, the IPC call and main's handler

**Files:**
- Modify: `src/main/runner.ts` (one method after `steps()`), `src/shared/ipc.ts`, `src/preload.ts`, `src/main.ts`
- Create: `src/main/runner.editor.test.ts`

**Interfaces:**
- Consumes: `projectFile`, `vscodeUrl` (Task 1); `locate`, `detectLayout` (already imported in `runner.ts`).
- Produces:
  ```ts
  // Runner
  /** A tree feature path or an absolute path, as a file of the open project to open in an editor (open-in-editor R3); null otherwise. */
  editorFile(file: string): string | null;
  // ipc.ts
  CH.openInEditor = "editor:open"
  // Api
  /** Opens the file at the line in VS Code; main refuses any file outside the open project (docs/specs/2026-11-03-open-in-editor.md). */
  openInEditor(file: string, line: number): Promise<void>;
  ```

- [ ] **Step 1: Write the failing tests** in `src/main/runner.editor.test.ts`, with `setup`, `fixture`, `cleanUp`, `T` from `test/runner.ts`:
  - after `runner.open(fixture("maven"))`, `editorFile("examples/users.feature")` equals `realpathSync.native(join(root, "src/test/java/examples/users.feature"))`, and `editorFile(<that absolute path>)` equals the same.
  - `editorFile("../x.feature")` is null.
  - a fresh `setup().runner.editorFile("examples/users.feature")` (no project) is null.

- [ ] **Step 2: Run** `npx vitest run src/main/runner.editor.test.ts`. Expected: FAIL, `editorFile` is not a function.

- [ ] **Step 3: Implement** `editorFile`: null without `this.project`; otherwise `projectFile(root, path.isAbsolute(file) ? file : locate([root, ...detectLayout(root).classpath], file))`.

- [ ] **Step 4: Run** `npx vitest run src/main/runner.editor.test.ts`. Expected: PASS.

- [ ] **Step 5: Wire the call.** `CH.openInEditor` and `Api.openInEditor` in `ipc.ts`; `openInEditor: (file, line) => ipcRenderer.invoke(CH.openInEditor, file, line)` in `preload.ts`. In `main.ts`, `handle(CH.openInEditor, async (file, line) => …)`: the request limits, else notice `Invalid request`; `runner.editorFile(file)` null → notice `Grace opens only files of the open project`; `!app.getApplicationNameForProtocol("vscode://")` → notice `Open in VS Code needs VS Code installed`; else `await shell.openExternal(vscodeUrl(abs, line))`.

- [ ] **Step 6: Run** `npm run typecheck` and `npm run lint`. Expected: both exit 0.

- [ ] **Step 7: Commit** `feat(ipc): open a project file in VS Code from the window`.

### Task 3: The button and the error link

**Files:**
- Modify: `src/renderer/Details.tsx` (`Leaf`), `src/renderer/Details.test.tsx`

**Interfaces:**
- Consumes: `Api.openInEditor(file, line)` (Task 2).

- [ ] **Step 1: Write the failing tests** in a new `describe("details: open in VS Code (open-in-editor R1, R2)")`, rendering `Details` with a store as the step-debugger block does, `api = { steps: vi.fn(async () => null), openInEditor: vi.fn(async () => {}) }`:
  - a scenario at `tests/x.feature` line 6: clicking the button named `Open in VS Code` calls `openInEditor("tests/x.feature", 6)`; the button also shows while `run.active`.
  - a row with `line: null`: no `Open in VS Code` button.
  - a failed scenario whose error is `"C:\\p\\tests\\auth.feature:9\nThen status 200\nexpected status: 200"`: `details-error` text is unchanged; clicking the button named `C:\p\tests\auth.feature:9` calls `openInEditor("C:\\p\\tests\\auth.feature", 9)`.
  - error `"c:/p/a.feature:3\nboom"`: a link calling `openInEditor("c:/p/a.feature", 3)`.
  - error `"Config evaluation failed: karate-config.js - js failed:\nLine: 4"`, and error `"boom at x.feature:7"`: no button inside `details-error`.

- [ ] **Step 2: Run** `npx vitest run src/renderer/Details.test.tsx`. Expected: the new tests FAIL.

- [ ] **Step 3: Implement** in `Leaf`. The button: `ghostSmall`, `data-testid="open-in-editor"`, `ExternalLink` icon size 14, text `Open in VS Code`, after the location `Cut`, inside `<Show when={props.node.line}>`. The link: the error's first line (`error.split("\n", 1)[0]`) matched against `/^([A-Za-z]:[\\/].*):(\d+)$/`; when it matches, a `button` with class `link` and `data-testid="error-place"` showing that line, then the rest of the error (`error.slice(firstLine.length)`, which keeps its `\n`); otherwise the error as today.

- [ ] **Step 4: Run** `npx vitest run src/renderer/Details.test.tsx src/renderer/palette.test.ts`. Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): open a test or its failure in VS Code from the details`.

### Task 4: End-to-end

**Files:**
- Create: `e2e/editor.spec.ts`

**Interfaces:**
- Consumes: `launch`, `openProject`, `project`, `node`, `nodeByKey`, `expect`, `test` from `e2e/helpers.ts` (not modified).

- [ ] **Step 1: Write the tests.** A local `stubVsCode(app, name)` does `app.evaluate` to replace `shell.openExternal` with a recorder (as `e2e/updates.spec.ts` does) and `app.getApplicationNameForProtocol` with `() => name`, and returns a reader of the recorded URLs. Expected URLs are built from `realpathSync.native(dir)` with `\` → `/`.
  - `Open a scenario in VS Code`: `project("clean")`, stub `"Visual Studio Code"`, select `failing match`, click `open-in-editor` → `[…/tests/basic.feature:15]`; run `f:tests/basic.feature` as `e2e/details.spec.ts` does, select `failing match`, click `error-place` → the second URL ends `tests/basic.feature:20`.
  - `main refuses what it must not open`: stub `""`; click `open-in-editor` → notice `Open in VS Code needs VS Code installed`. Then through `window.api.openInEditor` on `win.evaluate`: `("tests/basic.feature", 0)`, `(…, 1.5)`, `(…, "12")` → notice `Invalid request`; `("C:\\Windows\\win.ini", 1)` → `Grace opens only files of the open project`. Recorded URLs stay `[]`.

- [ ] **Step 2: Ask the parent session for a go**, then run `npm run e2e -- e2e/editor.spec.ts`. Expected: 2 passed. Then, with its go, the full `npm test` and `npm run e2e`.

- [ ] **Step 3: Commit** `test(e2e): open a test and its failure in VS Code`.
