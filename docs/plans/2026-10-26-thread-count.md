# Thread Count Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Changed during Task 5:** discovery dry runs pass `-T 1`, not no `-T`: a `karate-pom.json` with `threads` above 1 made discovery multi-threaded and broke the tree. See spec R3.2 and commit 09ebb64. Where the plan below says `-T` goes on runs only and never on a dry run, read "runs get the count, dry runs get 1".

**Goal:** A per-project Threads box in the toolbar whose count every run passes to Karate as `-T <n>`, recorded in each run's `run.json` and shown in a History column.

**Architecture:** Shared rules (`MAX_THREADS`, `validThreads`) live in a new `src/shared/threads.ts`. `settings.ts` loads and saves `projects[root].threads`. The `Runner` holds the open project's count like the tag filter. It checks and saves it in `setThreads`, applies it in `rediscover`, sends a `threads` event, and passes it through `spawn` to `buildArgs`, which adds `-T` on runs only. `run.json` and `HistoryEntry` gain `threads: number | null`. The renderer keeps `threads: { count } | null` (a fresh object per event) and a `Threads` box between Env and Properties….

**Tech Stack:** Electron 45, TypeScript, SolidJS 2 RC, Tailwind v4, Vitest 5 (jsdom for `*.test.tsx`), Playwright 1.64.

**Spec:** [docs/specs/2026-10-26-thread-count.md](../specs/2026-10-26-thread-count.md) (R1–R4). Mirrors the tag filter's flow (tag filter spec) and the EnvPicker's reset-on-every-view pattern.

## Global Constraints

- Branch `feature/thread-count`. The spec commit `91e00cb` is on it. Conventional Commits, lower-case subjects, AGENTS.md rules (Solid 2, no `^` ranges, no new dependency).
- Values, copied exactly:
  - `MAX_THREADS = 32`. A valid count is an integer from 1 to 32.
  - Refusal notice: `Threads must be a whole number from 1 to 32`.
  - Save failure notice: `The thread count could not be saved: <message>`.
  - Busy notice: the existing `BUSY` ("A run is already in progress").
  - The settings field is `projects[root].threads`. The default count is 1.
- UI:
  - The label is "Threads", on `data-testid="threads-input"`, an `<input type="number" min="1" max="32" step="1">`. It sits between Env and Properties… in the toolbar.
  - The History column is "Threads", right after Env, on `data-testid="history-threads"`. It shows the number, or `—` for null.
- `-T <n>` goes on every run, never on a dry run (`-D`). `run.json` keeps `version: 1`.
- Runner tests start real JVMs. Run `npm run vendor` once first. Never run `npm test` and `npm run e2e` at the same time. Run one e2e file with `npm run package && npx playwright test e2e/<file>.spec.ts`.

## Review Focus

1. **A typed count that was never committed when Run all is clicked.** Clicking the button blurs the box, `change` fires and `setThreads` reaches main before `run`, so the run uses the typed count. Test: the Task 5 scenario "Run all uses the count in the box" types `2` and clicks Run all without Enter.
2. **Stepping with the arrow keys or the spin buttons.** Each step is one `change` and one save. Test: Task 5 "The count is saved per project" presses ArrowUp and polls `settings.json`.
3. **A refusal of a value equal to what main held.** The box must show main's count again, even though the count did not change. Test: Task 4 `a refused value resets the box`.
4. **Switching projects.** The box must never show the previous project's count for the new one. It is disabled until the new project's `threads` event arrives, and a Refresh of the same project keeps it. Test: Task 4 `a new project shows its own count`.
5. **`settings.json` cannot be written.** The count still applies, with a notice. Test: Task 2 `applies a count it cannot save`.

---

### Task 1: Thread count rules and settings

**Files:**
- Create: `src/shared/threads.ts`, `src/shared/threads.test.ts`
- Modify: `src/main/settings.ts`
- Test: `src/main/settings.test.ts`

**Interfaces:**
- Produces:
  - `MAX_THREADS = 32`
  - `validThreads(n: unknown): n is number`: true only for an integer from 1 to `MAX_THREADS`.
  - `THREADS_ERROR`: the refusal notice, built from `MAX_THREADS`.
  - `loadProjectThreads(dataDir: string, root: string): number`: the saved count if `validThreads`, else 1. Never throws: missing, malformed or unreadable all read as 1.
  - `saveProjectThreads(dataDir: string, root: string, n: number): void`: through `updateEntry`, sets `threads` only. Throws on failure, as the other saves do.

- [ ] **Step 1: Write the failing tests.**
  - `threads.test.ts`, `validThreads`: true for 1, 8, 32. False for 0, 33, 2.5, -1, `NaN`, `Infinity`, `"4"`, null, undefined. `THREADS_ERROR` equals the exact notice above.
  - `settings.test.ts`:
    - `save then load returns the count`: 8 round trips.
    - `each project has its own count`: 3 for root A. Root B loads 1.
    - `saving the count keeps the other settings`: after `saveLastProject`, `saveWindowState`, `saveProjectFilter` and `saveProjectEnv` (one plain row), `saveProjectThreads(…, 4)` leaves all four loadable and unchanged.
    - `a malformed saved count loads as 1`: `it.each` over raw `threads` values `"8"`, 0, 2.5, 64, null.
    - `an unreadable file loads as 1`: use the existing read-failure mock.
- [ ] **Step 2: Run** `npx vitest run src/shared/threads.test.ts src/main/settings.test.ts`. Expected: FAIL, because `./threads` does not exist and the settings exports are missing.
- [ ] **Step 3: Implement** the Interfaces above. The load reads through `savedEntry` inside a `try`, as `loadProjectEnv` does.
- [ ] **Step 4: Run** the same command, then `npm run typecheck`. Expected: PASS, the existing settings tests included.
- [ ] **Step 5: Commit** `feat(settings): load and save the project's thread count`.

### Task 2: The Runner holds the count and every run passes `-T`

**Files:**
- Modify: `src/main/karate/command.ts`, `src/main/runner.ts`, `src/main.ts`, `src/preload.ts`, `src/shared/ipc.ts`, `src/shared/events.ts`
- Test: `src/main/karate/command.test.ts`, `src/main/runner.test.ts`. Add `threads: 1` to the `buildArgs` calls in `src/main/karate/argfile.test.ts`, `src/main/karate/process.test.ts` and `runner.test.ts`'s `dryRunTree`.

**Interfaces:**
- Consumes: `validThreads`, `THREADS_ERROR`, `loadProjectThreads` and `saveProjectThreads` (Task 1).
- Produces:
  - `buildArgs({ …, threads: number })`: `["-T", String(threads)]` in the place `-D` takes on a dry run, so a run gets `-T`, a dry run gets `-D` and never `-T`.
  - `AppEvent` `| { type: "threads"; count: number }`, documented as "The open project's thread count, as main holds it and passes to every run."
  - `CH.threads = "threads:set"`, `Api.setThreads(n: number): Promise<void>`. The preload is `setThreads: (n) => ipcRenderer.invoke(CH.threads, n)`.
  - `main.ts`: `handle(CH.threads, (n) => runner.setThreads(n))`. It takes any value through, and the Runner checks it, as `setEnv` does with names.
  - `Runner.setThreads(n: unknown): void`:
    - While busy: the `BUSY` notice, then the count is re-sent.
    - No project: nothing happens.
    - `!validThreads(n)`: the `THREADS_ERROR` notice, then the count is re-sent.
    - Otherwise: the count is stored and saved through `saveProjectThreads`. A failed save is the "could not be saved" notice and the count still applies. Then a `threads` event is sent.
  - `rediscover(project, filter, envs, threads: number)`:
    - `open` passes `loadProjectThreads(dataDir, root)`. `refresh` and `switchEnv` pass `this.threads`.
    - On success, it sets `this.threads` and sends `threads` right after `filter`.
    - On cancel, it sends the unchanged `threads` right after `filter`.
  - `replay()` becomes `[last, filter, threads, environment, history, busy]`.
  - `spawn(…, filter, threads: number)`. `runTarget` captures `threads: this.threads` into its `run` object, and `runBody` takes `threads` and passes it to `spawn`. `discover` passes `this.threads`, which `buildArgs` drops on a dry run.

- [ ] **Step 1: Write the failing tests.**
  - `command.test.ts`: add `threads: 1` to `o`. Runs now expect `[...base, "-T", "1", "t1"]`, and with env and the argument file `["@A.txt", ...base, "-e", "qa", "-T", "1", "t1"]`. Dry runs stay as they are. New test `passes -T on runs only`: `{ ...o, threads: 8, dryRun: false }` contains `"-T", "8"` right before the targets, and `{ ...o, threads: 8, dryRun: true }` has no `"-T"`.
  - `runner.test.ts`:
    - Move the `projectDir` helper from "Runner history" to module scope. Add `suiteThreads(file: string): number`: the `data.threads` of the first `SUITE_ENTER` in `readAllEvents(file)`.
    - Insert the `threads` event into every existing exact event list: `{ type: "threads", count: 1 }` after the filter in the `replay()` `toEqual`s at about lines 200, 573, 707 and 1437, and `"threads"` after `"filter"` in the `typesAfter` lists at about lines 654 and 665.
    - New `describe("Runner threads")`:
      - `a run passes the count and discovery does not`: open `clean`, then `setThreads(2)`. `last("threads")` is `{ type: "threads", count: 2 }`, and `loadProjectThreads` gives 2. `run("d:")`. Then `suiteThreads` of `<projectDir>/runs/<runStarted.runId>/karate-json/karate-events.jsonl` is 2, and `suiteThreads` of `<projectDir>/dryrun/karate-json/karate-events.jsonl` is 1.
      - `restores the count on open, per project`: `setThreads(3)` on `clean`, then dispose. A second runner on the same data folder opens `clean`: `last("threads").count` is 3 and the replay holds `{ type: "threads", count: 3 }`. Opening `maven` gives count 1.
      - `a cancelled open restores the count before`: `setThreads(3)` on `clean`, open `maven`, then cancel. `last("threads").count` is 3.
      - `refuses a count that is not a whole number from 1 to 32`: after `setThreads(2)`, for each of 0, 33, 2.5, `NaN`, `"4"`, null and -1, `last("notice").message` is `THREADS_ERROR`, `last("threads").count` is 2, and `loadProjectThreads` gives 2.
      - `rejects a change while busy`: during a run of `f:tests/slow.feature`, `setThreads(4)` gives the `BUSY` notice and `last("threads").count` stays 1. Then cancel.
      - `applies a count it cannot save`: make `settings.json` a folder, then `setThreads(4)`. The notice starts with `The thread count could not be saved`, and `last("threads").count` is 4.
      - `ignores a change with no project`: `setThreads(4)` on a fresh runner emits no event.
- [ ] **Step 2: Run** `npx vitest run src/main/karate/command.test.ts src/main/runner.test.ts -t "buildArgs|Runner threads"`. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npx vitest run src/main`, then `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(runner): pass the project's thread count to every run`.

### Task 3: Each run records its count and History shows it

**Files:**
- Modify: `src/shared/history.ts`, `src/main/runner.ts` (the record in `runBody`'s `store`), `src/renderer/History.tsx`
- Test: `src/shared/history.test.ts`, `src/renderer/History.test.tsx`, `src/main/runner.test.ts`
- Fixtures that must gain `threads: 1`: the records in `src/main/history.test.ts`, `runner.test.ts`'s `seed`, `e2e/helpers.ts`'s `seedRuns`, and the `HistoryEntry` fixtures in `src/shared/history.test.ts`, `src/renderer/History.test.tsx` and `src/renderer/reduce.test.ts`.

**Interfaces:**
- Consumes: `runBody`'s `threads` (Task 2).
- Produces:
  - `RunRecord.threads: number | null` and `HistoryEntry.threads: number | null`. `toEntry` carries it through its spread unchanged. Document it as "The count passed with `-T`; null for a run stored before F12."
  - `parseRunRecord`:
    - A missing `threads`, or `null`, reads as `null`. The type allows null, so a record must survive a write and read round trip.
    - Any other value must be an integer of at least 1, or the record is malformed (null). There is no upper bound.
  - Each `run.json` the Runner writes holds the run's `threads`.

- [ ] **Step 1: Write the failing tests.**
  - `history.test.ts`:
    - Give `specExample` `threads: 4`. `accepts the spec example` then still round trips.
    - New `reads a record stored before F12 as no thread count`: `specExample` without `threads` parses to `{ ...specExample(), threads: null }`, and so does `{ ...specExample(), threads: null }`.
    - `rejects bad shapes` gains `{ ...specExample(), threads: 0 }`, `threads: "2"` and `threads: 1.5`.
    - The `toEntry` test expects `threads` copied.
  - `History.test.tsx`, new test `shows each run's thread count, or a dash`: two entries with `threads: 8` and `threads: null` give `history-threads` cells `8` and `—`. The header cells read `When, Env, Threads, Tags, What ran, Result, Duration`.
  - `runner.test.ts`:
    - `a run writes run.json in its own folder` expects `threads: 1` in the record.
    - In "Runner threads", the first test also expects `threads` 2 in `<projectDir>/runs/<runId>/run.json`, and 2 in the newest `history` entry.
- [ ] **Step 2: Run** `npx vitest run src/shared/history.test.ts src/renderer/History.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** the Interfaces above, the Threads column, and the fixture fields.
- [ ] **Step 4: Run** `npx vitest run src/shared src/renderer src/main/history.test.ts`, then `npx vitest run src/main/runner.test.ts -t "run.json|Runner threads"`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(history): record and show each run's thread count`.

### Task 4: The Threads box

**Files:**
- Create: `src/renderer/Threads.tsx`, `src/renderer/Threads.test.tsx`
- Modify: `src/renderer/reduce.ts`, `src/renderer/Toolbar.tsx`, `src/index.css`

**Interfaces:**
- Consumes: the `threads` event, `Api.setThreads` (Task 2), `MAX_THREADS` (Task 1).
- Produces:
  - `AppState.threads: { count: number } | null`. Document it as "The open project's thread count, as main holds it. A fresh object per `threads` event, so the box resets even when main re-sends the count it held. Null with no project." `initialState` sets it to null.
  - The `project` case sets it to null in the root-changed branch, with the comment "main's `threads` event follows".
  - The new `case "threads"` does nothing with no project. Otherwise it sets `s.threads = { count: e.count }`.
  - `Threads(props: { state: AppState; api: Api })`, a `<label class="flex items-center gap-1 text-sm">Threads <input …></label>`:
    - The input uses the EnvPicker input's classes with `w-16`, and `min`, `max={MAX_THREADS}` and `step` as in the Global Constraints.
    - It is disabled while `run.active`, while `busy`, or when `threads` is null.
    - `onChange` calls `void props.api.setThreads(e.currentTarget.valueAsNumber)`. Main checks every value, so the window sends `NaN` for an empty box.
    - A `createEffect` tracks `props.state.threads` and sets the input's `.value` to `String(t?.count ?? 1)` through a `ref={(e) => (box = e)}`, the house ref style. A `value=` binding would not reset after a refusal of the same count.
  - Toolbar: `<Threads …/>` goes between `<EnvPicker …/>` and `<PropertiesDialog …/>`.
  - `src/index.css`: `input[type="number"]` joins the `cursor: text` rule.

- [ ] **Step 1: Write the failing tests** in `Threads.test.tsx`. Render `App` with a fake api, as `EnvPicker.test.tsx` does (add `setThreads: vi.fn(async () => {})`). `load(count)` sends `project` (root `D:/p`) and then `{ type: "threads", count }`.
  - `shows main's count and is disabled with no project`: before any event, the box is disabled and shows `"1"`. After `load(3)`, it is enabled and shows `"3"`.
  - `sends a committed value once`: after `load(1)`, `fireEvent.input` with `"16"` makes no call. A `fireEvent.change` then calls `setThreads` exactly once, with 16.
  - `a refused value resets the box`: after `load(2)`, input `"0"` and change call `setThreads(0)`. Sending `{ type: "threads", count: 2 }` makes the box show `"2"`.
  - `disabled while busy`: `{ type: "busy", busy: true }` disables the box, and `busy: false` enables it again.
  - `a new project shows its own count`: after `load(3)`, a `project` event for root `D:/q` disables the box. `{ type: "threads", count: 1 }` then shows `"1"`. Next, a `project` event for the same root `D:/q` (a Refresh) keeps it enabled at `"1"`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/Threads.test.tsx`. Expected: FAIL, because `threads-input` is not found.
- [ ] **Step 3: Implement** the Interfaces above.
- [ ] **Step 4: Run** `npx vitest run src/renderer`, then `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): add the threads box to the toolbar`.

### Task 5: End-to-end and the backlog row

**Files:**
- Create: `e2e/threads.spec.ts`
- Modify: `e2e/helpers.ts`, `e2e/cursor.spec.ts`, `docs/features.md` (F12 row: `specced` becomes `done`)

**Interfaces:**
- Consumes: everything above.
- Produces: `ranThreads(userData: string, root: string, id: string): number` in `e2e/helpers.ts`. It returns `data.threads` of the first `SUITE_ENTER` line of `<projectDataDir>/runs/<id>/karate-json/karate-events.jsonl`. Get the id from the newest History row's `data-id`.

- [ ] **Step 1: Write the tests.**
  - `threads.spec.ts` follows `history.spec.ts`'s style. Use local `openClean` and `runAll` helpers that wait for the History count. Give each test `test.setTimeout(120_000)`. "Karate reports N" means `ranThreads(…) === N`.
    - `A project with no saved count runs with 1 thread`: the box shows `1`. After Run all, the newest row's `history-threads` is `1` and Karate reports 1.
    - `Run all uses the count in the box`: `fill("2")` with no Enter, then click Run all. The newest row shows `2` and Karate reports 2.
    - `A single test's run uses the count`: after `fill("2")` and Enter, `passing hello`'s `run-node`. Karate reports 2.
    - `Re-run failed uses the current count`:
      - Run all at 2. `failing match` fails.
      - `fill("1")` and Enter, then Re-run failed.
      - The newest row shows `1` and Karate reports 1. The second row still shows `2`.
    - `The box overrides a karate-pom.json`: write `karate-pom.json` `{"threads":3}` into the `clean` copy before opening it. The box shows `1`, and after Run all Karate reports 1.
    - `The box is disabled while a run is going`: once Run all is clicked, the box is disabled. It is enabled again after the run.
    - `The count is saved per project`:
      - `fill("3")` and Enter. Poll `saved(userData, dir).threads` until it is 3.
      - Restart on the same `userData`: the box shows `3`.
      - `press("ArrowUp")`: the box shows `4`. Poll the saved count until it is 4.
      - Open `project("tags")`: the box shows `1`. Reopen the first project: it shows `4`.
    - `A value out of range is refused`: start from a saved 2, then loop over `"0"`, `"33"`, `"2.5"` and `""`. Each `fill` and Enter gives the notice `Threads must be a whole number from 1 to 32`, the box shows `2`, and the saved count stays 2.
  - `cursor.spec.ts`: `cursor(win.getByTestId("threads-input"))` is `"text"`.
- [ ] **Step 2: Run** `npm run package && npx playwright test e2e/threads.spec.ts e2e/cursor.spec.ts`. Expected: PASS. A failure here is a bug in Tasks 1–4: fix it there and keep these tests as written.
- [ ] **Step 3: Run** the full suites one after the other, never together: `npm test`, then `npm run e2e`. Expected: PASS.
- [ ] **Step 4: Set** the F12 row in `docs/features.md` to `done`.
- [ ] **Step 5: Commit** `test(e2e): cover the thread count and mark f12 done`.
