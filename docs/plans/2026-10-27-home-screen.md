# Home Screen Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A home screen to pick a project folder or reopen one of the 10 most recent, shown whenever no project is open or opening, and a toolbar `Home` button that closes the project and goes back to it.

**Architecture:** `src/main/settings.ts` keeps `recentProjects` next to `lastProject`. A new `src/main/recent.ts` checks each saved folder with async `fs` calls capped at 1 s and sends the newest checked list as a `recent` event. `Runner` emits a new `opening` event around each open, saves the folder after a successful open, and gains `openRecent`, `removeRecent`, `close` and `sendRecent`. The renderer adds `opening`, `loaded` and `recent` to its state, draws nothing until main's first `project` or `opening` event (so a start never flashes the home screen), then shows `Home` when no project is open and none is opening.

**Tech Stack:** Electron, SolidJS 2 RC, Tailwind CSS v4, Vitest 5, Playwright 1.64.

**Spec:** [docs/specs/2026-10-27-home-screen.md](../specs/2026-10-27-home-screen.md) (R1–R5). Layout: [the mockup](../specs/2026-10-27-home-screen-mockup.png).

## Global Constraints

- Branch `feature/home-screen` in the worktree `.claude/worktrees/home-screen`; the spec commit `776e0f4` is on it. Before Task 1: `npm install`, then copy `vendor/` from `D:\Repositories\karate-kid\vendor` (or `npm run vendor`).
- No new dependency. `contextIsolation`, `sandbox` and `nodeIntegration: false` stay. Solid 2 rules of `AGENTS.md`. Every `settings.json` write goes through `update()`.
- Copy from the spec, verbatim: "Open a Karate project", "Pick a folder, or reopen one you used before.", "Open folder…", "Recent folders", "not found", "Remove from list", "Home", "Invalid request", "A run is already in progress", "Cancelled", "Folder not found: <root>".
- Builder copy: tree message while a folder opens "Discovering tests…"; save failures "The recent list could not be saved: <error message>" (open, remove) and "The last project could not be forgotten: <error message>" (close).
- Values: key `recentProjects`, at most 10, newest first. Same folder: `path.resolve(a).toLowerCase() === path.resolve(b).toLowerCase()`. Existence check cap 1000 ms. Paths from the renderer: strings of at most 1000 characters.
- Test ids: `home`, `open-button` (on both `Open folder…` and the toolbar `Open…`: only one is ever on screen, so `openProject()` in `e2e/helpers.ts` stays as is), `recent-list`, `recent-row`, `recent-missing`, `recent-remove`, `home-button`.
- One unit file: `npx vitest run <file>` (`runner.test.ts` needs `vendor/`). One e2e file: `npm run package && npx playwright test e2e/<file>.spec.ts`. Never run `npm test` and `npm run e2e` at the same time.

## Review Focus

1. **Two list sends overlap** (the check of a hung share from `CH.ready` still running when a removal or an open sends again): the newest list wins; an older one never lands after it. Test: Task 2 `drops a list whose checks end after a newer call's`.
2. **A double-click on a recent row**: the second `openRecent` of the folder already opening is ignored, with no "A run is already in progress" notice. Test: Task 3 `a second openRecent of the folder being opened is ignored`.
3. **The window reloads during an open from the home screen**: the replay shows that folder's project view, not home. Tests: Task 3 `replay during an open carries the folder being opened`; Task 4 `opening with no project shows the project view, not home`.
4. **A hand-edited `recentProjects`** with a trailing backslash or another case: one entry per folder. Test: Task 1 `same folder with a trailing separator or another case is one entry`.
5. **A recent entry that is a file, or whose `stat` fails** (EPERM): greyed out, not clickable. Test: Task 2 `a file or a failing stat counts as not found`.

---

### Task 1: The saved list

**Files:**
- Modify: `src/main/settings.ts`
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Produces:
  - `RECENT_LIMIT = 10` (exported).
  - `loadRecentProjects(dataDir: string): string[]`: never throws. Not an array, a missing key or an unreadable file: `[]`. Keeps strings that are absolute paths (`path.isAbsolute`) and not the same folder as an earlier kept entry; stops at 10.
  - `saveOpenedProject(dataDir: string, root: string): void`: replaces `saveLastProject` (delete it; rename its calls in `settings.test.ts`). One `update()`: `lastProject = root`, `recentProjects = [root, ...the loaded list without the same folder].slice(0, 10)`. Throws on failure.
  - `removeRecentProject(dataDir: string, root: string): void`: one `update()`. Drops the entry equal to `root` exactly; deletes `lastProject` if it is the same folder as `root`. Throws on failure.
  - `forgetLastProject(dataDir: string): void`: deletes `lastProject` and keeps every other key. Throws on failure.

- [ ] **Step 1: Write the failing tests** in `settings.test.ts`. Write raw `settings.json` text where a test starts from a hand-edited file.
  - `saveOpenedProject puts the folder first and saves lastProject`: with two temp folders `a`, `b`, open `a`, `b`, `a`. Expect `loadRecentProjects` to be `[a, b]` and `loadLastProject` to be `a`.
  - `keeps the 10 newest folders`: open `D:\p0` … `D:\p10`. Expect `["D:\\p10", …, "D:\\p1"]` (10 entries).
  - `same folder with a trailing separator or another case is one entry`: raw `recentProjects: ["D:\\work\\api", "d:\\WORK\\API\\"]` loads as `["D:\\work\\api"]`. Then `saveOpenedProject(data, "d:\\WORK\\api")` gives `["d:\\WORK\\api"]`.
  - `a broken list reads as what can be kept`: raw `[42, "rel\\path", "D:\\a", "d:\\A", "D:\\v0", …, "D:\\v11"]` loads as `["D:\\a", "D:\\v0", …, "D:\\v8"]`. `"x"` loads as `[]`, and so do no file and non-JSON text. Nothing throws.
  - `removeRecentProject drops the row and a matching lastProject`: raw `{ lastProject: "D:\\gone", recentProjects: ["D:\\gone", "D:\\b"], projects: { "D:\\gone": { threads: 2 } } }`. After `removeRecentProject(data, "D:\\gone")`, the parsed file has no `lastProject`, `recentProjects` is `["D:\\b"]`, and the `projects` entry is unchanged.
  - `removeRecentProject keeps a lastProject that is another folder`.
  - `forgetLastProject keeps the other keys`: start from `lastProject`, `recentProjects`, `window` and `projects`. Afterwards only `lastProject` is gone.
  - The existing EBUSY test now calls `saveOpenedProject` and expects a throw.
- [ ] **Step 2: Run** `npx vitest run src/main/settings.test.ts`. Expected: FAIL (`saveOpenedProject` is not exported).
- [ ] **Step 3: Implement** the Interfaces above in `settings.ts`, with one private `sameFolder(a, b)` and one private list parser shared by load, save and remove.
- [ ] **Step 4: Run** `npx vitest run src/main/settings.test.ts`, then `npm run typecheck`. Expected: PASS. Typecheck fails only on `src/main.ts` (still imports `saveLastProject`). Swap that import and its call to `saveOpenedProject` there for now: Task 5 rewrites that handler.
- [ ] **Step 5: Commit** `feat(settings): keep the 10 most recent project folders`

### Task 2: Checking and sending the list

**Files:**
- Create: `src/main/recent.ts`, `src/main/recent.test.ts`
- Modify: `src/shared/events.ts`

**Interfaces:**
- Produces:
  - `src/shared/events.ts`:
    - `export interface RecentEntry { path: string; exists: boolean }`.
    - New `AppEvent` members:
      - `{ type: "opening"; root: string | null }`: the folder main is opening, or null. Sent around each open and in every replay.
      - `{ type: "recent"; entries: RecentEntry[] }`: the saved recent folders, newest first, each checked.
  - `src/main/recent.ts`:
    - `type Stat = (p: string) => Promise<{ isDirectory(): boolean }>`.
    - `checkRecent(paths: string[], stat: Stat = fs.promises.stat, ms = 1000): Promise<RecentEntry[]>`: every path is checked in parallel. A folder gives `exists: true`; a file or a rejected `stat` gives `false`. A check not settled within `ms` gives `true`. Use `within` from `./karate/process`.
    - `recentSender(load: () => string[], emit: (e: AppEvent) => void, stat?: Stat, ms?: number): () => Promise<void>`: each call loads, checks, then emits `{ type: "recent", entries }`. It emits nothing if a later call started meanwhile (a call counter).

- [ ] **Step 1: Write the failing tests** in `recent.test.ts` (Node project):
  - `checks each folder`: a temp folder gives `true`; a missing path gives `false`; the order of `paths` is kept.
  - `a file or a failing stat counts as not found`: a temp file gives `false`; a `stat` rejecting with an `EPERM` error gives `false`.
  - `a folder that does not answer within 1 s counts as existing`: use `vi.useFakeTimers()` and two paths whose `stat` never settles. After `vi.advanceTimersByTimeAsync(1000)`, both resolve as `exists: true`.
  - `drops a list whose checks end after a newer call's`: `load` returns `["D:\\a"]`, then `[]`. The first call's `stat` waits on a promise the test releases; the second call's checks nothing. Start both calls, let the second finish, then release the first. Expect the emitted events to be exactly `[{ type: "recent", entries: [] }]`.
- [ ] **Step 2: Run** `npx vitest run src/main/recent.test.ts`. Expected: FAIL (module not found).
- [ ] **Step 3: Implement** `recent.ts` and the event types.
- [ ] **Step 4: Run** `npx vitest run src/main/recent.test.ts`, then `npm run typecheck`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(main): check the recent folders without blocking`

### Task 3: The Runner opens, remembers and closes

**Files:**
- Modify: `src/main/runner.ts`
- Test: `src/main/runner.test.ts`

**Interfaces:**
- Consumes: Task 1's `loadRecentProjects`, `saveOpenedProject`, `removeRecentProject`, `forgetLastProject`; Task 2's `recentSender` and the `opening` and `recent` events.
- Produces:
  - `private opening: string | null = null`.
  - `open(root)`: same signature.
    - Once the folder exists, it sets `opening` to the resolved root and emits `{ type: "opening", root }`.
    - It resets `opening` and emits `{ type: "opening", root: null }` in a `finally` inside the guarded work, so this comes before `busy: false` on every path. A missing folder emits no `opening`.
    - After `rediscover` returns true, still inside the work: `saveOpenedProject(dataDir, root)`. A throw becomes the notice "The recent list could not be saved: …", and `open` still returns true.
    - After the guarded work, whatever the outcome: `void this.sendRecent()`.
  - `openRecent(root: string): Promise<void>`:
    - Returns at once, with no event, while busy with `this.opening === root`.
    - Otherwise, a `root` not in `loadRecentProjects(dataDir)` gets the notice "Invalid request".
    - Otherwise: `await this.open(root)`.
  - `removeRecent(root: string): void`:
    - A `root` not in the list gets "Invalid request".
    - Otherwise `removeRecentProject`. A throw becomes "The recent list could not be saved: …", and nothing else happens.
    - Otherwise: `void this.sendRecent()`.
    - Not refused while busy: it touches only `settings.json`.
  - `close(): void`:
    - While busy: the notice `BUSY` only.
    - Otherwise it clears `project`, `tree`, `last` and `report`, and resets `filter` (no filter), `threads` (1), `envs` (`emptyEnvSettings()`), `overlays` and `tagEnvs`.
    - Then `forgetLastProject`. A throw becomes "The last project could not be forgotten: …".
    - Then it emits `{ type: "project", project: null, tree: null, error: null }` directly, not through `publish`, so `last` stays null.
    - Then `void this.sendRecent()`. It starts no process.
  - `sendRecent(): Promise<void>`: one `recentSender(() => loadRecentProjects(this.o.dataDir), this.o.emit)`, made in the constructor.
  - `replay()`: `{ type: "opening", root: this.opening }` just before the `busy` event, in both branches.

- [ ] **Step 1: Write the failing tests** in `runner.test.ts`, with the existing `setup()`, `fixture()`, `until()` and `T`:
  - `open emits opening around the discovery`: open `clean`. In `events`, expect this order: `busy` true, `opening` with `path.resolve(root)`, the `project` event, `opening` null, `busy` false.
  - `a successful open saves the folder first and as lastProject`: open `clean`, then `tags`.
    - `loadRecentProjects` is `[tags, clean]` and `loadLastProject` is `tags`.
    - `until` a `recent` event holds both, each `exists: true`.
  - `a failed open from the home screen changes nothing` (spec unit): open `clean` and cancel during the dry run (pattern of the existing cancel test).
    - The last `project` event has `project: null`, the notice is "Cancelled", and the last `opening` is null.
    - `loadRecentProjects` is `[]` and `loadLastProject` is null.
    - Also, a missing folder emits no `opening` event.
  - `a failed list save keeps the project open` (spec unit): `mkdirSync(path.join(dataDir, "settings.json"))`, then open `clean`.
    - `open` returns true and the last `project` event has the `clean` root.
    - A notice starts with "The recent list could not be saved: ".
  - `openRecent opens only a saved folder`:
    - After `saveOpenedProject(dataDir, clean)`, `openRecent(tags)` emits "Invalid request" and no `busy` event.
    - Then `openRecent(clean)` opens it.
  - `a second openRecent of the folder being opened is ignored`: with `clean` saved, start `openRecent(clean)`, wait `until` an `opening` event, then call `openRecent(clean)` again. Expect no `notice` at all.
  - `removeRecent drops a saved folder only`:
    - With `[clean, tags]` saved, `removeRecent(tags)` leaves `[clean]`, and a later `recent` event holds `clean` only.
    - `removeRecent("D:\\not-saved")` emits "Invalid request".
  - `close forgets the project and lastProject, keeps the rest`: open `clean`, `setThreads(2)`, then `close()`.
    - The last `project` event is null.
    - `loadLastProject` is null, `loadRecentProjects` is `[clean]` and `loadProjectThreads(dataDir, clean)` is 2.
    - `replay()` is `[{ type: "opening", root: null }, { type: "busy", busy: false }]`.
    - Opening `clean` again sends `threads` 2.
  - `close is refused while busy` (spec unit): start `open(clean)`, wait `until` busy, then call `close()`. Await the open. A discovery holds the same busy flag as a run.
    - The notice is "A run is already in progress".
    - The last `project` event has the `clean` root, and `loadLastProject` is `clean`.
  - `replay during an open carries the folder being opened`: during `open(clean)`, `replay()` is `[{ type: "opening", root: path.resolve(clean) }, { type: "busy", busy: true }]`.
- [ ] **Step 2: Run** `npx vitest run src/main/runner.test.ts`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npx vitest run src/main/runner.test.ts`, then `npm run typecheck`. Expected: PASS. Existing tests that compare whole event lists may now see `opening` or `recent` events. Filter those types out in such a test; never change what it checks.
- [ ] **Step 5: Commit** `feat(runner): open, remember and close projects for the home screen`

### Task 4: The home screen and the Home button

**Files:**
- Create: `src/renderer/Home.tsx`, `src/renderer/Home.test.tsx`
- Modify: `src/shared/ipc.ts`, `src/preload.ts`, `src/renderer/reduce.ts`, `src/renderer/App.tsx`, `src/renderer/Toolbar.tsx`, `src/renderer/Tree.tsx`
- Test: `src/renderer/reduce.test.ts`, `src/renderer/Tree.test.tsx`

**Interfaces:**
- Consumes: Task 2's `RecentEntry`, `opening` and `recent` events.
- Produces:
  - `CH.openRecent = "recent:open"`, `CH.removeRecent = "recent:remove"`, `CH.close = "project:close"`.
  - `Api`: `openRecent(root: string): Promise<void>`, `removeRecent(root: string): Promise<void>`, `closeProject(): Promise<void>`. The preload invokes the channels; `closeProject` passes no argument.
  - `AppState`:
    - `opening: string | null` (initial null).
    - `loaded: boolean` (initial false): set by the first `project` or `opening` event; until then the window cannot tell home from a folder that is opening.
    - `recent: RecentEntry[] | null` (initial null): null until main sends it.
  - `reduce`:
    - `opening`: sets `opening` and `loaded`.
    - `recent`: sets `recent`.
    - `project`: also sets `loaded`. With `!e.project` the reset branch also empties `console`.
  - `Home(props: { state: AppState; api: Api })`.
  - `App`:
    - `home = () => !state.project && !state.opening`.
    - Nothing is drawn in `#root` until `state.loaded`. `#startup` is removed once `state.loaded` is true (a `createEffect` on it, in place of `onSettled`).
    - On home: the notice bar, then `Home`. Otherwise the project view as today.
    - The notice bar text is `state.notice ?? (home() ? state.error : null)`, so "Folder not found: …" shows on home.
  - `Toolbar`:
    - `Home` (`home-button`, `btn`) is the first item. It is disabled while `busy()` and calls `api.closeProject()`.
    - The path is `project?.root ?? opening ?? "No project"` (also its `title`).
  - `Tree`: `!project || !tree` without an error gives "Discovering tests…".
  - `Home` layout, per the mockup:
    - A centred column, `max-w-[540px]`, holding the `h1` and its line, then the `primary` button `Open folder…` (`open-button`, calls `api.openProject()`).
    - When `recent` has entries: an `h2` "Recent folders" and `ul` `recent-list`.
    - Each `li` holds two buttons. The row `recent-row` (`disabled` when `!exists`) calls `api.openRecent(path)`; the `×` `recent-remove` (`aria-label` and `title` "Remove from list") calls `api.removeRecent(path)`.
    - The row shows the name (the last segment of `path` split on `\` and `/`, or `path` itself) in bold, `recent-missing` "not found" after it when `!exists`, and the path in mono below.
    - Both buttons get a visible `focus-visible` ring. Every colour has a zinc `dark:` variant.

- [ ] **Step 1: Write the failing tests.**
  - In `Home.test.tsx`, render `App` with a `vi.fn` api, as in `Tree.test.tsx`. The api includes `openRecent`, `removeRecent` and `closeProject`.
    - `draws nothing until main says what is open`: add `<div id="startup">` to `document.body` and render. There is no `home` and no `project-path`, and `#startup` is still there. After `{ type: "opening", root: null }`: `home` shows and `#startup` is gone.
    - `a first start shows the home screen without a list`: the heading, the line and `open-button` show the spec's copy. There is no `recent-list`, `project-path` or `tree-message`, also after `recent` with `[]`.
    - `lists the recent folders`: send `[{ path: "D:\\work\\api", exists: true }, { path: "E:\\usb\\old", exists: false }]`.
      - The names are "api" and "old". The second row is disabled with `recent-missing` "not found".
      - Clicking the first row calls `openRecent("D:\\work\\api")`.
      - The second row's "Remove from list" calls `removeRecent("E:\\usb\\old")`.
    - `opening with no project shows the project view, not home` (spec unit): send `opening` null, `busy` true, then `opening` `"D:\\p"`. There is no `home`, `project-path` is `"D:\\p"`, `cancel-button` is enabled, `home-button` is disabled and `tree-message` is "Discovering tests…".
    - `opening another folder keeps the open project on screen`: with project `D:/p` open, send `opening` `"D:\\q"`. `project-path` stays `D:/p`.
    - `a cancelled open goes back home with the notice`: from an opening `"D:\\p"`, send a `project` null, the notice "Cancelled", `opening` null and `busy` false. `home` shows with `notice` "Cancelled".
    - `a missing folder shows its reason on home`: a `project` null with `error` "Folder not found: D:\\x" shows `home` with `notice` "Folder not found: D:\\x".
    - `Home closes the project`: with a project open, clicking `home-button` calls `closeProject`. After `busy` true, the button is disabled.
  - In `reduce.test.ts`: `opening` sets `opening` and `loaded`; `recent` stores the entries; a `project` event sets `loaded`; a null project empties `console`.
  - In `Tree.test.tsx`: the "disables Run all without a tree" test sends `{ type: "opening", root: "D:/p" }` first and expects "Discovering tests…".
- [ ] **Step 2: Run** `npx vitest run src/renderer/Home.test.tsx src/renderer/reduce.test.ts src/renderer/Tree.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npm test`, `npm run typecheck`, `npm run lint`. Expected: PASS.
  - Another test that renders `App` and checks the screen before its first event now sees nothing. Send `{ type: "opening", root: "D:/p" }` first to get today's empty project view.
- [ ] **Step 5: Commit** `feat(ui): add the home screen and the Home button`

### Task 5: Main wiring, end-to-end tests and the check record

**Files:**
- Modify: `src/main.ts`, `e2e/helpers.ts`, `e2e/open.spec.ts`, `docs/features.md`
- Create: `e2e/home.spec.ts`, `docs/checks/2026-10-27-home-screen.md`

**Interfaces:**
- Consumes: Task 2's `checkRecent`, Task 3's Runner methods, Task 4's channels.
- Produces:
  - `src/main.ts`:
    - `CH.ready`: after the replay loop, `void runner.sendRecent()`.
    - `CH.open`: `defaultPath` is the `path.dirname` of the first entry with `exists` in `await checkRecent(loadRecentProjects(dataDir))`; with none, no `defaultPath`. Then `await runner.open(folder)`, which saves now.
    - `handlePath(channel, fn: (root: string) => unknown)`: like `handleRunId`. It accepts only a string of at most 1000 characters; anything else gets the notice "Invalid request". `handlePath(CH.openRecent, …)` calls `runner.openRecent`; `handlePath(CH.removeRecent, …)` calls `runner.removeRecent`. `handle(CH.close, () => runner.close())`.
    - The startup open is unchanged.
  - `e2e/helpers.ts`:
    - `launchWithSettings(settings: Record<string, unknown>)`: a fresh userData folder with `settings.json` written, then `launch()`. Returns what `launch` returns.
    - `settingsOf(userData: string): Record<string, unknown>`: the parsed `settings.json`.

- [ ] **Step 1: Wire `src/main.ts`** as above, and remove the `saveOpenedProject` call that Task 1 left in the `CH.open` handler. Run `npm run typecheck`. Expected: PASS.
- [ ] **Step 2: Write `e2e/home.spec.ts`**: one test per automated scenario of the spec's Acceptance criteria, named after it.
  - Start a scenario from a saved list with `launchWithSettings`.
  - Write paths as `path.resolve(dir)` so they match what the Runner saves.
  - For the slow run, follow `e2e/cancel.spec.ts`.
  - Points the spec leaves to the test:
    - First start: `home` shows with the copy. `recent-list`, `project-path`, `tree` and `tree-message` have count 0.
    - Last project at start: open `clean`, close the app and launch again with the same userData. `project-path` shows the path while `tree-message` is "Discovering tests…". Then the tests show.
    - Missing last project: `{ lastProject: gone, recentProjects: [gone] }`, where `gone` is a path that was deleted. `home` shows, the row is disabled (`toBeDisabled`) and `recent-missing` is visible.
    - Dry-run error: open `broken`. `tree-message` contains `syntax.feature`. After `home-button`, the first `recent-row` contains the broken path.
    - Dialog start: saved `[gone, a, b]`. Use `recordOpenDialog`, then click `open-button`. `defaultPath` is `path.dirname(a)`.
    - Removing a row: saved `{ recentProjects: [clean], projects: { [clean]: { threads: 2 } } }`.
      - After `recent-remove`, `recent-list` is gone and `settingsOf(userData).recentProjects` is `[]`.
      - `projects[clean].threads` is still 2, and the folder still exists.
    - Keyboard: saved `[clean]`. Press Tab until `recent-row` is focused (at most 5 presses), then Enter. The tests show.
    - Home and a restart: after `home-button`, the first row is `clean` and `settingsOf` has no `lastProject`. After a relaunch, `home` shows.
    - Home during a run: `home-button` is disabled while the slow feature runs and enabled once it ends.
    - Reload after Home: `win.reload()` shows `home`.
    - Not in the list: saved `[clean]`. `win.evaluate((d) => window.api.openRecent(d), tagsDir)` shows `notice` "Invalid request", and `home` stays.
- [ ] **Step 3: Update `e2e/open.spec.ts`**: the first test expects `home` to be visible in place of the "Open a Karate project folder" message.
- [ ] **Step 4: Run** `npm run e2e`, not alongside `npm test`. Expected: PASS. Fix any other test that relied on the empty project view at start: it now gets the home screen.
- [ ] **Step 5: Commit** `test(e2e): cover the home screen and mark f27 done`, including `src/main.ts`, `e2e/helpers.ts`, both specs, and the F27 row of `docs/features.md` set to `done`.
- [ ] **Step 6: Write `docs/checks/2026-10-27-home-screen.md`** in the format of `docs/checks/2026-10-26-window-bounds.md`.
  - The automated part: `npm test`, `npm run e2e`, typecheck and lint, with their counts.
  - By hand, results left for the person:
    - The dark theme scenario.
    - No home flash over five starts of the packaged app with a last project.
    - The light theme against the mockup.
- [ ] **Step 7: Commit** `docs(checks): add the home screen check record`, then push and open a draft PR to `develop`.
