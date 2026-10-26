# Thread count

Date 2026-10-26. Status: approved. Covers [feature](../features.md) F12.

## Intent

Almost every run uses one thread today. The app never passes `-T`, so Karate runs one scenario at a time, and a full suite takes as long as all its scenarios added up. The exception is a project whose `karate-pom.json` sets `threads`, which Karate reads on its own.

The person changes the count often. They set it to 1 while chasing a flaky test, so scenarios run one at a time and the console reads in order, and back to 8 for a full run. When a test is flaky only in parallel, the History tab should show it: the run that failed used 8 threads and the run that passed used 1.

## Who and what success looks like

The walking skeleton's user, iterating on an API suite all day.

Success:
- Set Threads to 8 and press Run all: Karate runs up to 8 scenarios at once.
- Set it to 1 and press Re-run failed: the failed tests run one at a time.
- The History tab shows 8 on the first run and 1 on the second.
- Restart the app: the project opens with the count it had. Another project keeps its own count.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- An "auto" or "number of cores" choice, or a suggested count.
- Warnings about memory, shared state between tests, or which tests are unsafe in parallel.
- Thread labels or per-thread grouping in the console (see F22).
- A count per environment.
- A Re-run that reuses the count of the run it repeats. It uses the current count. The History column shows the old one.
- Karate's efficiency metric or its timeline report.
- Discovery dry runs with more than one thread.
- Changing the count of a run that is going.
- Reading `threads` from a project's `karate-pom.json`, as a starting count or otherwise. The box is the only source (R3.1). F13 covers the pom as a project file.

**Anti-goals** (failure even if everything else works):
- A run that uses a different count than the box shows.
- A saved count that stops a project from opening, or a save that loses other settings.
- History runs that vanish because they were written before this feature.

## Constraints

- No new dependency. Solid 2 rules (AGENTS.md). `contextIsolation`, `sandbox` and `nodeIntegration: false` stay. The renderer reaches main only through the typed preload API, and main checks every value it gets from the renderer.
- Karate 2.1.4: `-T, --threads <n>`, default 1 (`RunCommand.java`). Features, scenarios and outline rows all run in parallel. Karate itself handles `@lock=<name>` and `@lock=*`. `SUITE_ENTER.data.threads` reports the count used (see `test/fixtures/events/run-clean.jsonl`). The CLI spike ran `-T 2` cleanly.
- Karate's guidance is 1 to 2 times the CPU cores, about 256 MB per thread ([inventory §5](../inventory/karate-core.md#5-parallelism)).
- Karate loads `karate-pom.json` from its working directory (the project root) unless `--no-pom` is passed, and CLI flags override the pom's values ([inventory](../inventory/karate-core.md)). Probed in the builder check with the bundled Karate 2.1.4 and a pom holding `"threads": 3`: a run with no `-T` reported 3 threads, a run with `-T 1` reported 1, and a dry run reported 3. Karate reads no thread count from an environment variable or a system property, apart from `KARATE_OPTIONS`, which the app already strips.

### What the app does today

- `buildArgs` (`src/main/karate/command.ts`) builds every Karate command line, dry runs and runs alike, and passes no `-T`.
- The tag filter is the model for this feature:
  1. `Api.setFilter` sends the filter on `CH.filter`.
  2. Main's handler in `src/main.ts` checks its shape with `parseFilterState` and sends "Invalid request" if it is wrong.
  3. `Runner.setFilter` refuses while busy: it sends the `BUSY` notice and re-sends the filter. With no project it does nothing. Otherwise it stores the filter through `storeFilter`, then sends a `filter` event.
  4. `storeFilter` saves through `saveProjectFilter`, which calls `updateEntry` and keeps the project's other fields. A failed save is a notice, and the filter still applies.
  5. On open, `loadProjectFilter` reads the filter, and it never throws. `rediscover` applies it only once the discovery was not cancelled.
  6. `replay()` re-sends it to a window that has just loaded.
  7. In the renderer, the `project` event resets the filter only when the project root changes or no project is open, and main's `filter` event follows (`src/renderer/reduce.ts`). A Refresh of the same project keeps it.
- `settings.json` → `projects[root]` holds the tag filter (`include`, `exclude`, `hideNonMatching`), `env` and `properties`.
- `run.json` is `version: 1` and is read by `parseRunRecord` (`src/shared/history.ts`), which drops a record with any malformed field. `toEntry` spreads the record's fields into the `HistoryEntry` the window gets.
- The History tab (`src/renderer/History.tsx`) has the columns When, Env, Tags, What ran, Result, Duration, and the actions.
- Toolbar order (`src/renderer/Toolbar.tsx`): project path, layout badge, Env (`EnvPicker`, disabled while busy or with no project), Properties…, Open…, Refresh, Run all, Re-run failed, Cancel, Report.
- None of these depend on the thread count:
  - `RunState` already handles several scenarios marked `running` at once.
  - Progress counts depth-0 `SCENARIO_EXIT`.
  - HTTP calls and step logs come from `FEATURE_EXIT`.
  - Console lines arrive whole. In a parallel run, lines of different scenarios interleave.

## Approaches considered

1. **Its own setting and channel, like the tag filter.** A new Api call sends the count. Main checks it, keeps it, saves it in `projects[root]` and sends it back. Every run reads that one value. **Chosen by the person.**
2. The window sends the count with each run (`run(key, n)`, `rerun(id, n)`, `rerunFailed(id, n)`). Three signatures change and the check runs in three places. The count is saved only on a run, so a change made without a run is lost on restart.
3. A count per environment, inside Properties. The person changes the count to debug, whatever the environment. It would also tie the count to the Properties save flow and its secret checks.

## Requirements

### R1. The control

1. In the toolbar, right after Env and before Properties…: the text "Threads" and a number box (`type="number"`, `min="1"`, `max="32"`, `step="1"`, `data-testid="threads-input"`). The visible text is the box's label, as "Env" is for its box.
2. The box shows the open project's count as main holds it. It changes only when main says so.
3. The box is disabled while the Runner is busy (a discovery or a run) and when no project is open, like Env.
4. A change is sent when the person commits it, on the input's `change` event: Enter, leaving the box, or one step with the arrow keys or the spin buttons. Each keystroke is not a change: typing `16` sends 16 once, not 1 and then 16.
5. If main refuses a value, the box shows the count main holds again (R2.2, R2.3). Main re-sends a count it already held, so the window resets the box on every count event, not only when the count changes.

### R2. Main: checking and saving

1. `MAX_THREADS = 32` lives in `src/shared/`, and the box's `max` uses it. A valid count is a whole number from 1 to `MAX_THREADS`.
2. Main refuses anything else, such as 0, 33, 2.5, `NaN`, a string or null. It sends the notice "Threads must be a whole number from 1 to 32", keeps the count it holds and re-sends it.
3. While busy, the count is refused. Main sends the `BUSY` notice, keeps the count and re-sends it, as `setFilter` does.
4. With no project open, the call is ignored.
5. An accepted count becomes the open project's count and is sent back to the window. It is saved as `projects[root].threads` through `updateEntry`, so the project's other fields and the other projects stay as they are. If the save fails, main sends the notice "The thread count could not be saved: <message>", and the count still applies until the app closes, as the filter does.
6. On open, the count is `projects[root].threads` when that is a valid count (R2.1). Otherwise it is 1: nothing saved, a malformed value (`"8"`, 0, 2.5, 64), or an unreadable file. Loading never throws.
7. The count is applied the way the filter is in `rediscover`. If an open is cancelled, the previous project and its count come back.
8. `replay()` sends the count too, so a window that reloads shows it.
9. Each project has its own count. A project with no saved count opens at 1.

### R3. Runs

1. Every run passes `-T <n>` with the open project's count at the moment the run starts, `-T 1` included. That covers Run all, the ▶ button on any tree node, Re-run and Re-run failed from the toolbar or the History tab. Because `-T` is always passed, it overrides a `threads` in the project's `karate-pom.json`, so the run uses exactly the count the box shows. A pom project that ran with the pom's count before F12 runs at 1 until the person changes the box.
2. Discovery dry runs pass no `-T`. That covers open, Refresh, an environment switch and the discovery before each run.
3. Re-run and Re-run failed use the current count, not the count stored in the run they repeat.

### R4. History

1. `run.json` gets a `threads` field: the count the run passed with `-T`. `version` stays 1. App versions from before F12 ignore the field. [ADR 0010](../decisions/0010-run-history-one-folder-per-run.md) says a change to `run.json` needs a version bump, or old runs drop out of the list. An added field that old records lack and old app versions ignore drops no run, so there is no bump and no new decision record.
2. `parseRunRecord` reads a missing `threads` as null: unknown. A run stored before F12 used Karate's default of 1, or the `threads` of the project's `karate-pom.json`, and the app recorded neither. A `threads` that is present but not a whole number of at least 1 makes the record malformed, like any other bad field. There is no upper bound, so raising `MAX_THREADS` later never hides stored runs.
3. `RunRecord.threads` and `HistoryEntry.threads` are `number | null`. Every run written from F12 on stores a number.
4. The History tab gets a "Threads" column right after Env (`data-testid="history-threads"`). It shows the number, or "—" for null.

## Design decisions already made

- The thread count is in the toolbar, right after Env, because the person changes it often. They chose option A in the mockup.
- The count is saved per project as soon as it changes (backlog row and the person).
- The History tab shows each run's count (the person).
- Approach 1: its own setting and channel, like the tag filter (the person).
- The box starts at 1 for a project with no saved count, even when its `karate-pom.json` sets `threads`. The pom is never read (the person, after the builder check).
- History rows written before F12 show "—", not 1 (the person, after the builder check).

The spec author's calls:
- A range of 1 to 32, default 1, because 1 is what every run uses today (R1.1, R2.1).
- Re-runs use the current count. Setting 1 and pressing Re-run failed is then the person's debug step (R3.3).
- The box is disabled while busy (R1.3).
- A value is sent on `change`, not on each keystroke (R1.4).
- One check, in main, with a plain-words notice and the box reset, instead of a second check in the renderer (R2.2).
- The column goes after Env (R4.4).
- `run.json` keeps `version: 1` and gets no new decision record (R4.1).
- `-T 1` is passed explicitly (R3.1), and dry runs pass no `-T` (R3.2).
- The count is not added to the `runStarted` event.

Resolved by the builder check (2026-10-26):
- The `karate-pom.json` finding. The person chose to start at 1 and to show old rows as "—".
- The renderer's filter reset happens only on a change of root (step 7 above).
- The box is reset on every count event (R1.5).
- Keeping `version: 1` is squared with ADR 0010 (R4.1).
- The builder check also ran 300 scenarios at `-T 16`. Every event line parsed, and the run had 300 depth-0 `SCENARIO_EXIT` events.

## Left to the builder

- The names of the Api method, the channel and the event (for example `setThreads`, `threads:set`, `{ type: "threads"; threads: number }`), and where the count lives in `Runner` and in the renderer's `AppState`.
- How the main handler and `Runner` share the check of R2.2: main's handler takes any value through, and `Runner` checks it, as `setEnv` does with `validEnvName`.
- The box's width, which must fit two digits and the spin buttons.
- How the end-to-end tests read `SUITE_ENTER.data.threads` from a run's `karate-json/karate-events.jsonl` (the `projectDataDir` helper in `e2e/helpers.ts` finds the run folders).

## Known risks

- Tests that share state can fail with more than one thread. The app cannot tell. The History column is how the person sees it.
- Memory: 32 threads at about 256 MB each can pass the JVM's default maximum heap (a quarter of the machine's memory). Karate then fails, and the run shows as failed, with the error in the console. F12 adds no heap setting.
- Console lines of parallel scenarios interleave (F22 covers filtering).

## Acceptance criteria

Every scenario is automated with Playwright against the built app, each with a fresh app data folder, unless it is marked **(unit)**. Unit scenarios run in Vitest. "Karate reports N threads" means the run's `karate-events.jsonl` has a `SUITE_ENTER` event whose `data.threads` is N.

```gherkin
Feature: Thread count

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11
    And a sample project with at least two scenarios, one of which fails, is open

  # R1, R3: the control and the runs

  Scenario: A project with no saved count runs with 1 thread
    Then the Threads box shows 1
    When I press Run all and the run finishes
    Then Karate reports 1 thread
    And the newest History row shows 1 thread

  Scenario: Run all uses the count in the box
    When I set the Threads box to 2
    And I press Run all and the run finishes
    Then Karate reports 2 threads
    And the newest History row shows 2 threads

  Scenario: A single test's run uses the count
    Given the Threads box shows 2
    When I press the run button of one scenario in the tree and the run finishes
    Then Karate reports 2 threads

  Scenario: Re-run failed uses the current count, not the run's
    Given a run with 2 threads has finished with a failed test
    When I set the Threads box to 1
    And I press Re-run failed and the run finishes
    Then Karate reports 1 thread
    And the newest History row shows 1 thread
    And the row before it still shows 2 threads

  Scenario: The box overrides the thread count of a karate-pom.json
    Given a project whose karate-pom.json holds "threads": 3 is open, with no saved count
    Then the Threads box shows 1
    When I press Run all and the run finishes
    Then Karate reports 1 thread

  Scenario: The box is disabled while a run is going
    When I press Run all
    Then the Threads box is disabled until the run finishes

  Scenario: Typing a number sends it once (unit)
    Given the Threads box shows 1
    When I type "16" into the Threads box and press Enter
    Then the window sends the count 16 once and nothing else

  Scenario: Discovery passes no thread count (unit)
    When the command line of a dry run is built with a count of 8
    Then it has no "-T"
    And the command line of a run with a count of 8 has "-T" "8"

  # R2: checking and saving

  Scenario: The count is saved per project
    When I set the Threads box to 3
    And I close the app and start it again with the same app data folder
    Then the Threads box shows 3
    When I open another project with no saved count
    Then the Threads box shows 1
    When I open the first project again
    Then the Threads box shows 3

  Scenario Outline: A value out of range is refused
    Given the Threads box shows 2
    When I type "<value>" into the Threads box and press Enter
    Then a notice says "Threads must be a whole number from 1 to 32"
    And the Threads box shows 2
    And settings.json still holds 2 threads for the project

    Examples:
      | value |
      | 0     |
      | 33    |
      | 2.5   |
      |       |

  Scenario Outline: Main refuses a count that is not a whole number from 1 to 32 (unit)
    Given the open project's count is 2
    When the window sends <value>
    Then a notice says "Threads must be a whole number from 1 to 32"
    And the count is still 2 and is sent back to the window

    Examples:
      | value |
      | NaN   |
      | "4"   |
      | null  |
      | -1    |

  Scenario: The count is refused while busy (unit)
    Given a run is going and the count is 2
    When the window sends 4
    Then the busy notice is sent
    And the count is still 2 and is sent back to the window

  Scenario: Saving the count keeps the other settings (unit)
    Given settings.json holds a last project, a window state, and for the project a tag filter, an environment and run properties
    When the project's count 4 is saved
    Then settings.json still holds all of them, and 4 threads for the project

  Scenario Outline: A malformed saved count loads as 1 (unit)
    Given settings.json holds <value> as the project's threads
    When the project's count is loaded
    Then it is 1

    Examples:
      | value |
      | "8"   |
      | 0     |
      | 2.5   |
      | 64    |

  Scenario: A reloaded window shows the count (unit)
    Given the open project's count is 5
    When a window that has just loaded asks for the replay
    Then the replay holds the count 5

  # R4: history

  Scenario: A run stored before F12 has no thread count (unit)
    Given a run.json with version 1 and no threads field
    When it is read
    Then it is kept and its threads is null
    And its History row shows "—" in the Threads column

  Scenario Outline: A run.json with a malformed thread count is dropped (unit)
    Given a run.json with version 1 and threads <value>
    When it is read
    Then it is malformed

    Examples:
      | value |
      | 0     |
      | "2"   |
      | 1.5   |
```
