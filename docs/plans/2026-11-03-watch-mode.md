---
status: built
issue: 67
summary: "A toolbar toggle that, while on, runs a .feature file's matching tests each time the file is saved"
---

# Watch mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A toolbar toggle that, while on, runs a `.feature` file's matching tests each time it is saved.

**Architecture:** A small watcher module in the main process (`src/main/watch.ts`) wraps Node's recursive `fs.watch` on each feature root, filters and debounces events, and hands saved absolute paths to the `Runner`. The `Runner` owns the choice (saved per project in `settings.json`), the watcher's life, a set of waiting saves, and a third run target that runs saved features after the usual pre-run discovery. The window gets a `watch` event and sends `setWatch(boolean)`.

**Tech Stack:** Electron main (Node 24 `fs.watch`), TypeScript, SolidJS 2, Vitest, Playwright.

**Spec:** [docs/specs/2026-11-03-watch-mode.md](../specs/2026-11-03-watch-mode.md)

## Global Constraints

- No dependency. No timer, poll or listener while watch mode is off; only OS file events while on; a timer only between a counted event and its batch (spec Cost).
- Quiet time: 300 ms (`QUIET_MS`), one window across all roots (R2.3).
- Counted path: ends in `.feature`, any case; no segment named `.git`, `node_modules` or `.gradle` (`SKIPPED_DIRS` from `src/main/fingerprint.ts`) (R2.2).
- Copy, exact: tooltip "Watch: run a feature when it is saved"; `aria-label` "Watch"; notices "Invalid request", "A run is already in progress", "Watch mode could not be saved: <reason>", "Watch mode stopped: <reason>".
- Settings key: `projects[<root>].watch`, `true` or absent (R1.3).
- Solid 2 rules and UI conventions of AGENTS.md: semantic tokens only, `Icon` with lucide data, `Tooltip`.
- Commits: Conventional Commits, scopes `runner`, `ui`, `ipc`, `docs`; each ends with the attribution lines; body says `Part of #67`.

## Review Focus

1. **The watched root itself is deleted or renamed.** On Windows libuv then reports the root's own absolute path (`\\?\C:\…`) in a loop, about 50,000 events a second, and never an `error` event (probed on this machine). Expect: the watcher closes at the first such event and reports an error; no CPU burn. Test in Task 1.
2. **Karate reading features during a run.** Expect: no event from a read or from an unchanged modified time starts a run. Test in Task 1 (same mtime) and Task 2 (a run does not trigger another).
3. **A save landing during an open of another project.** Expect: after the open, nothing from the old project runs in the new one; a cancelled open keeps the old project's waiting saves and watcher. Test in Task 2.
4. **Many features saved at once (a `git checkout`).** Expect: one run, targets passed through the targets file so the Windows command line limit is never hit. Test in Task 2 (two features use the targets file).
5. **The app quitting while a batch timer is pending.** Expect: `dispose` closes the watcher and clears the timer; no run starts after. Test in Task 2.

---

### Task 1: The watcher module

**Files:**

- Create: `src/main/watch.ts`
- Test: `src/main/watch.test.ts` (Node project: real temp folders, real `fs.watch`)

**Interfaces:**

- Produces:
  - `export const QUIET_MS = 300;`
  - `export function counts(rel: string): boolean`: `rel` as `fs.watch` gives it (relative, `\` or `/`).
  - `export interface FeatureWatcher { close(): void }`
  - `export function watchFeatures(roots: string[], onSaved: (files: string[]) => void, onError: (err: Error) => void, quietMs = QUIET_MS): FeatureWatcher`: throws synchronously (after closing what it opened) if a root cannot be watched. `onSaved` gets absolute paths. After `onError` is called once, every handle and the timer are closed and nothing more is called. `close()` is idempotent.

- [ ] **Step 1: Write the failing tests** in `src/main/watch.test.ts`, each with its own `mkdtempSync` folder, removed in `afterEach` after `close()`. Use `quietMs` 50 and a helper `settle = () => sleep(400)`.
  - `counts`: true for `a.feature`, `x\\B.FEATURE`, `x/y/c.feature`; false for `karate-config.js`, `a.feature.bak`, `node_modules\\x\\c.feature`, `x\\.git\\a.feature`, `.gradle/b.feature`, `NODE_MODULES/c.feature`.
  - "a saved feature is handed over once, as an absolute path": write `sub/a.feature` → after settle, `onSaved` called once with `[path.join(root, "sub", "a.feature")]`.
  - "two saves within the quiet time are one batch": write `a.feature`, then 20 ms later `b.feature` → one call, both paths (any order).
  - "other files do not count": write `karate-config.js`, `data.json`, `node_modules/x/c.feature` → no call.
  - "a feature deleted before the batch is dropped": write then `rmSync` within 10 ms → no call.
  - "an event with no new modified time does not count": write `a.feature`, settle (one call), then `utimesSync` with its own atime and mtime → no second call. Also: a feature that existed before `watchFeatures` started, touched by `utimesSync` to its old mtime → no call.
  - "a root that is a file is watched as a file": root = `<dir>/one.feature` → writing it gives one call with that path.
  - "a missing root throws": `expect(() => watchFeatures([missing], …)).toThrow()`.
  - "deleting a watched root reports one error and stops": `rmSync(root, { recursive: true })` → after settle, `onError` called exactly once, `onSaved` never; `process.cpuUsage` over a further 300 ms below 100 ms user time.
  - "close stops it and frees the parent folder": after `close()`, writing a feature gives no call, and `renameSync` of the root's parent folder succeeds.

- [ ] **Step 2: Run the tests to see them fail**

Run: `npx vitest run src/main/watch.test.ts`
Expected: FAIL, `Cannot find module './watch'`.

- [ ] **Step 3: Implement `src/main/watch.ts`**
  - One `fs.watch(root, { recursive: true })` per directory root; a file root gets `fs.watch(root)` and its events always mean that file.
  - An event whose `filename` is null is ignored; one whose `filename` is absolute (`path.isAbsolute`) means the root itself went away: call `fail(new Error(\`${root} was removed or renamed\`))`. The watcher's `error` event also calls `fail`.
  - A counted event adds `path.join(root, filename)` to a pending set and restarts one `setTimeout(quietMs)`.
  - When the timer fires: for each pending path, `statSync` (missing or not a file: drop); keep it only if `mtimeMs > Math.max(started, seen.get(p) ?? 0)`, then `seen.set(p, mtimeMs)`; `started` is `Date.now()` at the call. Call `onSaved` only with a non-empty list.
  - `fail` and `close` close every handle and clear the timer; `fail` calls `onError` once.

- [ ] **Step 4: Run the tests to see them pass**

Run: `npx vitest run src/main/watch.test.ts`
Expected: PASS, all tests.

- [ ] **Step 5: Commit** `feat(runner): watch feature roots for saved features`

### Task 2: Watch mode in the runner

**Files:**

- Modify: `src/shared/events.ts` (add `| { type: "watch"; on: boolean }` to `AppEvent`)
- Modify: `src/main/settings.ts` (load and save the choice)
- Modify: `src/main/runner.ts`
- Test: `src/main/settings.test.ts`, `src/main/runner.watch.test.ts` (new, real Karate, helpers from `test/runner.ts`)

**Interfaces:**

- Consumes: `watchFeatures`, `FeatureWatcher`, `QUIET_MS` from Task 1.
- Produces:
  - `export function loadProjectWatch(dataDir: string, root: string): boolean`: true only for a saved `true`; never throws.
  - `export function saveProjectWatch(dataDir: string, root: string, on: boolean): void`: `entry.watch = true`, or `delete entry.watch`; through `updateEntry`.
  - `Runner.setWatch(on: unknown): void`
  - `Runner.replay()` includes `{ type: "watch", on }` after the threads event when a project is open.

- [ ] **Step 1: Write the failing tests.**
  - `settings.test.ts`: `loadProjectWatch` false with nothing saved, true after `saveProjectWatch(…, true)`, false after `saveProjectWatch(…, false)` (key absent from the file), false for `"watch": "yes"`; other fields of the entry are kept.
  - `runner.watch.test.ts`, on `fixture("tags")`, each feature written by the test (`* match 1 == 1`):
    - "turning on saves it, sends it and is replayed": `setWatch(true)` → `last("watch")` is `{ type: "watch", on: true }`, `loadProjectWatch` true, `replay()` contains it; a second runner on the same `dataDir` opening the project sends `on: true`.
    - "refuses a value that is not a boolean": `setWatch("yes")` → notice "Invalid request", `last("watch").on` unchanged.
    - "a saved feature runs, alone, as its selection": on, write `tests/w1.feature` → `until(() => all("runStarted").length === 1)`; `selection` is `f:tests/w1.feature`; `keys` are only `w1`'s scenarios; once it ends, the history's newest run has that selection.
    - "nothing runs while off": write a feature, wait `QUIET_MS + 1000` → no `runStarted`.
    - "two features saved together are one run with no selection": write `w1` and `w2` within 50 ms → one `runStarted`, `selection` the tree's root key, `keys` of both.
    - "the filter applies": filter include `smoke`; write a feature with an `@smoke` scenario and an untagged one → `keys` has only the `@smoke` one.
    - "a feature with nothing to run is skipped quietly": write a feature whose only scenario is `@ignore` → no `runStarted`, no new `notice` after the write.
    - "a save during a run waits, then runs": start `runner.run("d:")`, write `w1` while busy → no notice "A run is already in progress"; after it ends, a second `runStarted` with selection `f:tests/w1.feature`.
    - "a run does not start another": after the run of `w1` ends, wait `QUIET_MS + 1500` → still one `runStarted` (Karate read the files).
    - "Cancel drops waiting saves": run `d:`, write `w1`, `cancel()` → after `QUIET_MS + 1500`, one `runStarted` only.
    - "closing stops watching" and "opening another project stops the first": after `close()` (or `open(fixture("clean"))`), write into the first project → no run.
    - "a cancelled open keeps the watcher": on, `open(fixture("clean"))`, `cancel()`, write `w1` in the first → it runs.
    - "a broken project is watched": write a feature with a parse error before `open`, saved `watch` true → tree has `error`; fix the file → a run of it starts.
    - "the toggle is refused during an open": `onEvent` on `opening` with a root → `setWatch(false)` → notice "A run is already in progress", choice unchanged.
    - "a watcher error stops watch mode in memory only": write `karate-pom.json` `{ "paths": ["tests"] }` into the fixture so the feature root is `<root>/tests`; open, turn on, `rmSync` the `tests` folder → notice starts with "Watch mode stopped: ", `last("watch").on` false, `loadProjectWatch` still true.
    - "turning off drops waiting saves": run `d:`, write `w1`, `setWatch(false)` → after the run, one `runStarted` only.
    - "dispose stops it": write `w1`, `dispose()` before `QUIET_MS` → no `runStarted`.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/main/settings.test.ts src/main/runner.watch.test.ts`
Expected: FAIL, missing exports and `setWatch`.

- [ ] **Step 3: Implement**
  - `settings.ts`: the two functions above, beside `loadProjectThreads`.
  - `runner.ts` fields: `watching = false`, `watcher: FeatureWatcher | null = null`, `waiting = new Set<string>()`.
  - `setWatch(on)`: not a boolean → "Invalid request" and re-send; `this.opening` → `BUSY` and re-send; no project → return. Else apply, save (catch → "Watch mode could not be saved: <message>"), `arm()`, send.
  - `private arm()`: close the watcher; if off or no project, clear `waiting` and stop; else `watchFeatures(detectLayout(root).featureRoots, onSaved, onError)` in a try, and on a throw or `onError`: close, `watching = false`, clear `waiting`, send `watch` off and "Watch mode stopped: <message>"; do not save.
  - `onSaved(files)`: add to `waiting`, then `drain()`.
  - `private drain()`: return if `busy`, `disposing`, no project or nothing waiting; take and clear `waiting`; `void this.guarded(() => this.runTarget({ files }, this.filter))`.
  - `guarded`'s `finally`, after `busy` is false: `this.drain()`.
  - `open`: when `opened`, clear `waiting`, `watching = loadProjectWatch(...)`, `arm()`, send `watch`. The folder-not-found branch: `watching = false`, `arm()`. `close()`: `watching = false`, `arm()`, send `watch` off. `refresh()`: after a discovery that did not cancel, `arm()`. `cancel()`: clear `waiting` first. `dispose()`: close the watcher and clear `waiting` first.
  - `RunTarget` gets `| { files: string[] }`. In `runTarget` after the discovery: each file's feature node is the first `findNode(tree, \`f:${rel}\`)`where`rel = path.relative(base, file).split(path.sep).join("/")`over`bases`and`rel`does not start with`..`; keep nodes with `matchingLeaves(node, filter).length > 0`; none → return with no notice. One → as a feature click (`selection: node`, `targets: selectionTargets(...)`, `targetFile: false`). Several → `selection: null`, `failedOf: null`, `targets` the concatenated `selectionTargets`, `targetFile: true`.

- [ ] **Step 4: Run them to see them pass**

Run: `npx vitest run src/main/settings.test.ts src/main/runner.watch.test.ts src/main/runner.runs.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit** `feat(runner): run saved features while watch mode is on`

### Task 3: The toggle in the window

**Files:**

- Modify: `src/shared/ipc.ts` (`CH.watch: "watch:set"`, `Api.setWatch(on: boolean): Promise<void>` with a one-line doc pointing at the spec)
- Modify: `src/preload.ts`, `src/main.ts` (`handle(CH.watch, (on) => runner.setWatch(on))`; the runner checks the type)
- Modify: `src/renderer/reduce.ts` (`watch: boolean` in `AppState`, `false` at start, set by the `watch` event)
- Modify: `src/renderer/Toolbar.tsx`
- Test: `src/renderer/reduce.test.ts`, `src/renderer/Toolbar.test.tsx`

**Interfaces:**

- Consumes: the `watch` event and `Runner.setWatch` from Task 2.
- Produces: `data-testid="watch-button"` for Task 4.

- [ ] **Step 1: Write the failing tests.**
  - `reduce.test.ts`: state starts with `watch: false`; a `watch` event with `on: true` sets it.
  - `Toolbar.test.tsx`: the order test gets `"watch-button"` after `"refresh-button"`. New tests: the button has `aria-label` "Watch" and `aria-pressed` "false", shows the `EyeOff` icon's path; after a `watch` event `on: true`, `aria-pressed` "true", the class has `bg-selected`, and the icon differs; a click calls `api.setWatch(true)` (then `false` when on); hovering shows "Watch: run a feature when it is saved"; enabled during a run (`busy` true); disabled with `opening` set and with no project.

- [ ] **Step 2: Run them to see them fail**

Run: `npx vitest run src/renderer/reduce.test.ts src/renderer/Toolbar.test.tsx`
Expected: FAIL.

- [ ] **Step 3: Implement** the channel, preload line, handler, state and button: `btnIcon`, plus `bg-selected` while on, `<Icon icon={on ? Eye : EyeOff} />`, wrapped in `Tooltip`, placed right after the Refresh tooltip, `disabled={!props.state.project || !!props.state.opening}`.

- [ ] **Step 4: Run them, then the static checks**

Run: `npx vitest run src/renderer/reduce.test.ts src/renderer/Toolbar.test.tsx src/renderer/palette.test.ts`, then `npm run typecheck` and `npm run lint`.
Expected: PASS, no errors.

- [ ] **Step 5: Commit** `feat(ui): add the Watch toggle to the toolbar`

### Task 4: End to end, cost check, docs

**Files:**

- Create: `e2e/watch.spec.ts`
- Modify: `docs/specs/2026-11-03-watch-mode.md` (status approved; R2.5 names the deleted-root case)

- [ ] **Step 1: Write `e2e/watch.spec.ts`**: `project("clean")`, `launch()`, `openProject`; click `watch-button`, expect `aria-pressed` "true"; overwrite `tests/basic.feature` with one failing scenario (`* match 1 == 2`); expect its tree row's status failed and the History tab count to go up by one with the newest run's selection that feature; click `watch-button` again, write the file again, wait 2 s, the History count is unchanged.

- [ ] **Step 2: Ask the parent session for a go, then run `npm run e2e`.** Expected: all pass. Tell the parent when done.

- [ ] **Step 3: Cost check by hand, with the parent's go**: launch the built app on `bigProject()`'s folder, watch on and off: main-process CPU over 60 s idle stays 0, and its private memory differs by under 1 MB. Write the numbers into the PR body.

- [ ] **Step 4: Update the spec** (status, R2.5 note), commit `docs(specs): mark the watch mode spec approved`.

- [ ] **Step 5: Push, open the draft PR** into `develop` from `feature/watch-mode` with `Closes #67`, the e2e summary and the cost numbers; read the `perf` comment when it lands and explain any worse measure.
