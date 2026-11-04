# Sub-folder offer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a discovery finds no test, offer the folders up to 3 levels down that look like a Karate project, and open the one picked like any project; skip the library listing for a project with no feature roots.

**Architecture:** A bounded async walk in `src/main/karate/subfolders.ts`. `Runner.discover` calls it only for an empty tree with no error and adds the result to the `project` event as an optional `subfolders` field (replayed for free through `this.last`). A new IPC call `openSubfolder(folder)` is checked against that field and calls `open`. The tree's empty state shows one button per folder.

**Tech Stack:** Electron main (Node `fs/promises`), SolidJS 2 renderer, Vitest, Playwright.

**Spec:** `docs/specs/2026-11-04-subfolder-offer.md`

## Global Constraints

- Depth 3 below the project folder; the project folder itself is never offered (R2.1).
- Markers, file names compared lower-case: `karate-pom.json`, `karate-config.js`, or `pom.xml` / `build.gradle` / `build.gradle.kts` beside `src/test` holding a `.feature` anywhere under it (R2.2).
- Skipped folder names, lower-case, at every level and under `src/test`: `SKIPPED_DIRS` (`.git`, `node_modules`, `.gradle`) plus `target`, `build`. No link or junction followed (R2.4).
- Limits: 10 found, 2,000 folders read, 2 seconds; return what was found by then (R2.5). Order: by level, then by path (R2.6).
- Button copy: `Open <folder>/`, `/` between names (R3.1).
- Refused folder notice: `Invalid request` (R4.2).
- Solid 2 rules and UI conventions of `AGENTS.md`: classes from `src/renderer/buttons.ts`, no raw palette class, no `title=`.
- Commits: Conventional Commits, subject ≤ 72 chars, body `Part of #232`, attribution footer.

## Review Focus

- A Maven or Gradle folder whose `karate-pom.json` `paths` point outside `src/test` still lists its libraries (R5 narrows only "no feature roots"): Task 2 test.
- A `.feature` only under `src/test/target/` or `src/test/build/` does not make its folder a project: Task 1 test.
- A file named `karate-config.js` that is a folder, or `src/test` that is a file, is not a marker: Task 1 test.
- A second `project` event without `subfolders` (Refresh that found tests, another project) clears the offer in the window: Task 3 test.
- The window asks for an offered folder after another project opened: refused, since only the last tree's offer counts: Task 2 test.

---

### Task 1: The walk

**Files:**
- Create: `src/main/karate/subfolders.ts`
- Create: `src/main/karate/subfolders.test.ts`
- Modify: `src/main/fingerprint.ts:10` (export `OUTPUT_DIRS`)

**Interfaces:**
- Produces: `export const SUBFOLDER_LIMITS = { depth: 3, found: 10, folders: 2_000, timeoutMs: 2_000 }` and `export async function karateFolders(root: string, limits = SUBFOLDER_LIMITS): Promise<string[]>`: relative paths with `/`, no trailing slash. Never throws.

- [ ] **Step 1: Write the failing tests** in `subfolders.test.ts` (Node project). A `write(root, files)` helper makes files under a fresh `mkdtempSync` root (removed in `afterEach`). Tests:
  - `offers each kind of Karate project`: `api/karate-pom.json`, `cfg/karate-config.js`, `g/build.gradle` + `g/src/test/java/a/b.feature`, `k/build.gradle.kts` + `k/src/test/x.feature`, `m/pom.xml` + `m/src/test/resources/c.feature` → `["api", "cfg", "g", "k", "m"]`.
  - `skips a build file whose src/test has no feature`: `tool/pom.xml` + `tool/src/test/java/Helper.java`, and `t2/pom.xml` + `t2/src/test/target/x.feature` → `[]`.
  - `looks 3 levels down`: `a/b/c/karate-config.js` → `["a/b/c"]`; another root with `a/b/c/d/karate-config.js` → `[]`.
  - `skips tool and output folders in any case`: `node_modules/x/`, `NODE_MODULES/y/`, `target/test-classes/`, `build/z/`, `.git/w/`, `.gradle/v/`, each holding `karate-config.js` → `[]`.
  - `never offers the project folder and does not look inside an offered one`: root `karate-config.js`, `karate/karate-config.js`, `karate/nested/karate-pom.json` → `["karate"]`.
  - `ignores a marker that is a folder`: folders `x/karate-config.js/` and `y/src/test` as a file next to `y/pom.xml` → `[]`.
  - `follows no junction`: `real/` outside root holding `karate-config.js`, `symlinkSync(real, path.join(root, "link"), "junction")` → `[]`.
  - `orders by level, then by path`: `z/karate-config.js`, `a/x/karate-config.js`, `b/karate-pom.json` → `["b", "z", "a/x"]`.
  - `stops at the limits`: three level-1 projects with `{ ...SUBFOLDER_LIMITS, found: 2 }` → first two; `folders: 1` with `a/karate-config.js` → `[]` (only the root was read); `timeoutMs: 0` → `[]`; a missing root → `[]`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/karate/subfolders.test.ts`
Expected: FAIL, cannot resolve `./subfolders`.

- [ ] **Step 3: Implement `karateFolders`**

Breadth-first, level by level, folders of a level in path order (`localeCompare`), children sorted by name. Each `readdir(dir, { withFileTypes: true })` counts one against `folders`, including those of the `src/test` search; a failed `readdir` is skipped. A folder is a project if an entry `isFile()` whose lower-case name is `karate-pom.json` or `karate-config.js`, or a build file `isFile()` and a `src` entry `isDirectory()` holding a `test` entry `isDirectory()` with a `.feature` file found by a depth-first search that stops at the first hit and skips the same names. Skip `isSymbolicLink()` entries and the names `[...SKIPPED_DIRS, ...OUTPUT_DIRS]`. Stop when `found` reached, `folders` spent or the deadline passed; a `setTimeout` race returns the found list even if a `readdir` never returns, and is cleared at the end.

- [ ] **Step 4: Run the tests to see them pass**

Run: `npx vitest run src/main/karate/subfolders.test.ts src/main/fingerprint.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): find karate projects below a folder`

---

### Task 2: Runner, IPC and the library listing

**Files:**
- Modify: `src/shared/events.ts:92` (project event)
- Modify: `src/shared/ipc.ts` (`CH.openSubfolder`, `Api.openSubfolder`)
- Modify: `src/preload.ts`, `src/main.ts:267`
- Modify: `src/main/runner.ts` (`discover`, `listLibraries`, new `openSubfolder`)
- Create: `src/main/runner.subfolders.test.ts`

**Interfaces:**
- Consumes: `karateFolders(root)` from Task 1.
- Produces: project event `{ type: "project"; project; tree; error; subfolders?: string[] }`, the field present only when non-empty. `CH.openSubfolder = "project:openSubfolder"`. `Api.openSubfolder(folder: string): Promise<void>`. `Runner.openSubfolder(folder: string): Promise<void>`.

- [ ] **Step 1: Write the failing tests** in `runner.subfolders.test.ts` with `setup`, `cleanUp`, `stubTool`, `calls`, `temps`, `T` from `test/runner.ts`. The fixture `repo()`: a temp root with `pom.xml` (`<project/>`), `mvnw.cmd` stubbed by `stubTool(root, "mvnw.cmd", "")`, `karate/karate-config.js` (`function fn() { return {}; }`) and `karate/tests/basic.feature` with 2 scenarios. Tests:
  - `offers the Karate project below a folder with no test, without listing libraries`: `open(root)` → `last("project").subfolders` equals `["karate"]`, `leafKeys(tree)` empty, `calls(log)` equals `[]`, no `notice` event.
  - `skips the listing on Refresh too`: then `refresh()` → `calls(log)` still `[]`.
  - `lists libraries when karate-pom.json paths give feature roots`: `pom.xml`, `karate-pom.json` `{ "paths": ["tests"] }`, `tests/t.feature`, stub `mvnw.cmd` → `calls(log)` has length 1.
  - `offers nothing when the tree has tests`: standalone root with `t.feature` and `sub/karate-config.js` → `last("project")` has no `subfolders` property.
  - `offers nothing after a failed dry run`: `copyFixture("broken")` (from `test/fixtures/copy.ts`) plus `sub/karate-pom.json` → `error` contains `syntax.feature`, no `subfolders`.
  - `keeps the offer in the replay`: after `open(root)`, `runner.replay()[0]` has `subfolders` `["karate"]`.
  - `opens an offered folder as any project`: `openSubfolder("karate")` → `last("project").project.root` equals `path.join(root, "karate")`, 2 leaves, `loadRecentProjects(dataDir)[0]` equals `path.join(root, "karate")`.
  - `refuses a folder it did not offer`: after `open(root)`, `openSubfolder("../elsewhere")` and `openSubfolder("karate/")` → `last("notice").message` `Invalid request`, project root still `root`. After `openSubfolder("karate")`, `openSubfolder("karate")` again → `Invalid request` (karate's own tree offers nothing).

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/runner.subfolders.test.ts`
Expected: FAIL (`openSubfolder` is not a function; `subfolders` undefined; `calls(log)` length 1).

- [ ] **Step 3: Implement**
  - `events.ts`: add `subfolders?: string[]` to the project event, with a one-line doc naming the spec.
  - `listLibraries`: return null also when `detectLayout(root).featureRoots.length === 0` (R5.1).
  - `discover`: after the tree is built, when `tree && !error && leafKeys(tree).length === 0`, `await karateFolders(project.root)`; then `if (this.cancelled) return null;` before `this.tree = tree`; publish with `...(found.length ? { subfolders: found } : {})`.
  - `openSubfolder(folder)`: if `this.last?.project` and `this.last.subfolders?.includes(folder)`, `await this.open(path.join(this.last.project.root, folder))`; otherwise emit notice `Invalid request`.
  - `ipc.ts`, `preload.ts`: the channel and the method. `main.ts`: `handlePath(CH.openSubfolder, (f) => runner.openSubfolder(f));`.

- [ ] **Step 4: Run the tests to see them pass**

Run: `npx vitest run src/main/runner.subfolders.test.ts src/main/runner.libraries.test.ts src/main/runner.setup.test.ts` then `npm run typecheck`
Expected: PASS, no type error.

- [ ] **Step 5: Commit** `feat(runner): offer the karate projects below an empty folder`

---

### Task 3: The offer in the empty tree

**Files:**
- Modify: `src/renderer/reduce.ts` (`AppState.subfolders: string[]`, `initialState`, `case "project"`)
- Modify: `src/renderer/Tree.tsx:412-419` (empty state)
- Modify: `src/renderer/Tree.test.tsx` (api mock gets `openSubfolder: vi.fn(async () => {})`)

**Interfaces:**
- Consumes: the project event's `subfolders`, `Api.openSubfolder` from Task 2.

- [ ] **Step 1: Write the failing test** `offers the folders below an empty project` in `Tree.test.tsx`: send a project with an empty folder tree and `subfolders: ["karate", "tests/api"]` → buttons by role named `Open karate/` and `Open tests/api/`, inside the empty state below "Grace looks for .feature files under the project folder."; click `Open tests/api/` → `api.openSubfolder` called with `"tests/api"`; send `busy: true` → both disabled; send the same project without `subfolders` → no button named `/^Open /` left. The existing empty-state test keeps passing unchanged (no button without the field).

- [ ] **Step 2: Run it to see it fail**

Run: `npx vitest run src/renderer/Tree.test.tsx -t "offers the folders"`
Expected: FAIL, no button `Open karate/`.

- [ ] **Step 3: Implement**: `reduce` sets `s.subfolders = e.subfolders ?? []` in the project case; the empty state renders a `<For each={props.state.subfolders}>` of `<button type="button" class={btnSmall} disabled={props.state.busy} onClick={() => void props.api.openSubfolder(f)}>Open {f}/</button>` in a wrapping flex row.

- [ ] **Step 4: Run the renderer tests**

Run: `npx vitest run src/renderer/Tree.test.tsx src/renderer/palette.test.ts src/renderer/reduce.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit** `feat(ui): offer the karate projects below an empty tree`

---

### Task 4: e2e and README

**Files:**
- Modify: `e2e/project-setup.spec.ts` (one test)
- Modify: `README.md:44` (R6.1)

- [ ] **Step 1: Write the e2e test** `an empty folder offers the Karate project below it`, using the file's `write` helper: `pom.xml` `<project/>`, `karate/karate-config.js`, `karate/tests/basic.feature` with scenarios `one` and `two`. `launch()`, `openProject(app, dir)`; expect `tree-message` `No feature files found`; click the button named `Open karate/`; expect `project-name` `karate`; `openFeatures(win)`; expect `node(win, "one")` and `node(win, "two")` visible.

- [ ] **Step 2: README**: add to the project line (line 44) one sentence: when the opened folder has no test, Grace offers the folders up to 3 levels below it that look like a Karate project.

- [ ] **Step 3: Run lint and the spec** (after the parent session's go for e2e)

Run: `npm run lint`, then `npm run e2e -- project-setup.spec.ts`
Expected: lint clean; the new test passes.

- [ ] **Step 4: Commit** `test(e2e): open the karate project offered below a folder` and `docs: say grace offers a karate project below the folder`
