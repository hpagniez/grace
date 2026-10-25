# Run history

Date 2026-10-25. Status: approved. Covers [features](../features.md) F10.

## Intent

Today karate-kid keeps one run per project: the run folder is emptied before each run, and the console, statuses and step logs live only in the window until the next run or a restart. This feature keeps the last runs of each project so the person can see how they went, open their HTML report, run them again, and run again only what failed, on the environment and tags they used.

It serves the local edit-and-rerun loop; long-term history belongs to CI. Karate Labs' paid tools have no clear run history view ([commercial inventory](../inventory/commercial.md#user-needs-a-free-runner-must-cover)), so this is a place where the free runner can do better. What it stores per test can also feed trends and flaky tests later (F15), over the 20 runs it keeps; F15 can widen the storage when it is specced.

It builds on the [walking skeleton](2026-10-24-walking-skeleton.md), the [tag filter](2026-10-24-tag-filter.md) and the [environment switcher](2026-10-25-environment-switcher.md): same toolbar, same one-process-at-a-time runner, same per-project data folder.

## Who and what success looks like

The walking skeleton's user, iterating on a suite: run, read failures, fix, run again.

Success:
- After a run with failures, one click runs only the failed tests, on that run's environment and tag filter.
- After a restart, the last 20 runs of the project are still listed and their reports still open.
- An older run can be repeated without remembering which environment, tags or tests it used.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals for this version:**
- Loading a past run back into the tree, details pane or console. The run's HTML report shows its results, steps and HTTP calls.
- Storing a run's console output, error messages or step logs.
- Trends, pass rates and flaky tests (F15); comparing two runs.
- A setting for how many runs are kept (fixed at 20).
- History of discovery dry runs (Open, Refresh, environment change).
- Keeping the single run of version 1.0: its folder is deleted, the history starts empty.
- Exporting or sharing runs (F19).
- Deleting the history of a project that was moved or deleted. Its run folders stay in the app data folder: a project on a disconnected drive must not lose its history.

**Anti-goals** (failure even if everything else works):
- A re-run that uses another environment, or a wider tag filter, than the history row says.
- A past run's results shown as the current state of the tree.
- A secret value in any file the app itself writes for history.
- History growing on disk without bound.
- A damaged or unreadable history folder blocking the project from opening or hiding the other runs.

## Constraints

- Everything in the walking skeleton's and environment switcher's Constraints sections. No new dependency.
- Storage layout decided in [ADR 0010](../decisions/0010-run-history-one-folder-per-run.md).

### What Karate and the app do today (from the earlier specs and probes)

| Fact | Consequence here |
|---|---|
| The app passes `-o <folder>` and `-B false`; Karate writes `karate-summary.html` and `karate-json/karate-events.jsonl` there | Each run can get its own `-o` folder |
| Karate writes no HTML report after a kill | A cancelled run has no report |
| A `file:LINE` target ignores `-t` and `@ignore`; `file:L1:L2` runs several lines of one file (the app already does this for outline rows) | Re-run failed passes exact lines, one target per file |
| A row of a dynamic Examples table (`read('rows.json')`) has no line | Such a row cannot be run alone |
| A test's tree key shifts when a scenario is inserted above it; `resolveNode` finds a test again by same key and name, else the one same-kind test of that name in the same feature | Stored tests are found again the same way |
| Karate's HTML report and event file can contain secret values unless the project sets `configure logging mask` (environment switcher R5) | Keeping 20 runs keeps up to 20 such copies |
| The app starts Java without a shell; a Windows command line holds at most 32,767 characters | Hundreds of `file:LINE` targets (an outage failing the whole suite) do not fit on the command line |

Probes for this spec (2026-10-25, Karate 2.1.4, bundled Java 21, Windows 11 fr-FR with code page Windows-1252):

| Probe | Result |
|---|---|
| `run <abs>\fix.feature:3:15`, line 3 a scenario and line 15 a row of an outline below it | ran exactly that scenario and that row |
| `run @targets.txt` (after `run`, read by picocli), one target per line, unquoted, single `\` | ran the targets |
| same, each target in `"…"` with `\` doubled, project path with a space | ran the targets |
| same, project path with `é`, file written as UTF-8 | ran the targets (picocli reads it as UTF-8, unlike the Java launcher's `@file` of ADR 0008) |
| same, quoted with single `\` | `\t` read as a tab: exit 3 |
| same, absolute `@C:\…\app data\targets.txt` with the working directory elsewhere | ran the targets |
| `run @missing.txt` | ran nothing, exit 0 |

## Approaches considered

How to look back at a past run:
1. **Open its HTML report.** Karate's report already shows a run's results, steps and HTTP calls; nothing new to build or to mask. **Chosen.**
2. Load the past run back into the tree, details and console. Needs the console and step logs stored masked at write time, and handling for tests that changed since. Much more work for what the report already shows.

Where the history tab lives (mockups compared in the brainstorm):
1. **A History tab next to Console in the bottom panel.** Wide rows fit a full table; console and history are both about runs. **Chosen.**
2. A History tab next to Details in the right half. Narrow: each run becomes a card.
3. A History drop-down in the toolbar. No layout change, but history is out of sight.

How runs are stored:
1. **One folder per run, each with its own `run.json`.** Listing reads at most 20 small files; a damaged file loses one run; retention deletes folders. **Chosen.**
2. Karate's backups (`-B true`): Karate renames the previous output folder when the next run starts. The timestamp is that rename's, there is no retention, and the app still needs its own summary file.
3. One `history.json` index per project plus the run folders. Every run rewrites the shared file, so one bad write loses the whole history, and the index can drift from the folders.

## Requirements

### R1. Run folders

- Each run writes to its own folder `<app data>\projects\<hash>\runs\<id>\`, passed as `-o`. `<id>` is the local start time `yyyyMMdd-HHmmss-SSS`, so folder names sort by time; if that folder already exists, `-2`, `-3`… is appended.
- The discovery folder `...\dryrun\` is unchanged: emptied before each dry run.
- A run gets a folder only once every check before spawning has passed (unreadable secret, nothing to run, test changed: no folder, no history entry).
- When a run ends (finished, cancelled, or failed, including a failed start after its folder was created), main writes `run.json` (R2) into its folder, then applies retention (R7).
- Quitting the app during a run kills it (walking skeleton) and still writes its `run.json`, as cancelled, within the same 10 s bound. If that fails, the folder is deleted at the next start.
- At app start, before any run, main deletes:
  - every run folder without a `run.json` (left by a crash of the app during a run);
  - every `projects\<hash>\run\` folder left by version 1.0.

### R2. What a run stores

`run.json`, written by main when the run ends. It never holds a property value, an error message, a step log or a console line.

```json
{
  "version": 1,
  "startedAt": 1792937525123,
  "durationMillis": 72000,
  "env": "qa",
  "filter": { "include": ["smoke"], "exclude": ["slow"], "hideNonMatching": false },
  "selection": { "kind": "feature", "file": "tests/users.feature", "name": "users", "line": null, "key": "f:tests/users.feature" },
  "failedOf": null,
  "outcome": "finished",
  "exitCode": 1,
  "tests": [
    { "kind": "scenario", "file": "tests/users.feature", "refId": "[1:6]", "name": "create user", "line": 6, "key": "s:tests/users.feature#1", "status": "passed", "durationMillis": 412 },
    { "kind": "row", "file": "tests/users.feature", "refId": "[2.1:14]", "name": "age 3", "line": 21, "key": "s:tests/users.feature#2.1", "status": "failed", "durationMillis": 95 }
  ]
}
```

- `startedAt`: epoch milliseconds when the run started (the `runStarted` time). `durationMillis`: wall-clock time to the end of the run.
- `env`: the run's environment, `null` for none. `filter`: the tag filter the run used, as written (not pruned).
- `selection`: the tree node that was run (`kind`, `file`, `name`, `line`, `key`); the root for Run all. `null` for a run started by Re-run failed, whose `failedOf` is then the `startedAt` of the run it came from.
- `outcome`: `finished`, `cancelled` or `failed`, as the app reports today. `exitCode`: Karate's exit code, `null` if killed or never started.
- `tests`: every test the run selected (the run's scenario and row keys), with its final status (`passed`, `failed`, `notRun`, `interrupted`) and duration when Karate reported one. A test that never started in a cancelled or failed run is `notRun`.
- A `run.json` that cannot be parsed, or whose `version` is not 1, is not listed; its folder stays until retention removes it.

### R3. History tab

- The bottom panel gets two tabs: **Console** and **History (n)**, n being the number of listed runs. The panel keeps its height. The app starts on Console; starting any run switches to Console.
- The tab lists the project's runs, newest first, up to 20, one row each:
  - **When**: "Today 14:32", "Yesterday 17:20", else the date and time ("2026-10-21 09:05").
  - **Env**: the environment, "—" for none.
  - **Tags**: included tags as chips, excluded tags as "not @tag" chips, "—" for no filter.
  - **What ran**: "All", a folder path, a feature path, or a scenario, outline or row name (full path in a tooltip); for a Re-run failed run, "Failed tests of <When of the source run>".
  - **Result**: "41 passed, 2 failed of 43" (counted from `tests`). A cancelled run shows "Cancelled" and the counts so far; a failed run shows "Error" and the exit code if any.
  - **Duration**.
  - **Report**, **Re-run**, **Re-run failed** (R4 to R6).
- The tab header has **Clear history** (R7).
- With no runs the tab says "No runs yet".
- The list is read when a project opens, and updated after each run and after Clear history. Runs of every environment are listed, whatever the current environment.

### R4. Report of a past run

- **Report** on a row opens that run's `karate-summary.html` in the default browser, as the toolbar Report does. It is enabled only when that file exists.
- The toolbar **Report** keeps its walking-skeleton meaning (the last finished run of this session, cleared on an environment change or another project).

### R5. Re-run

Re-run repeats a run on its environment and tag filter. It is one busy operation: everything else is disabled, and Cancel works, until it ends.

0. If a secret of the run's environment cannot be read (environment switcher R5), nothing happens: no switch, and the same notice as a refused run.
1. If the run's environment differs from the current one, the app switches to it exactly as if the person chose it in the Env combo (environment switcher R1): discovery with that environment, saved, results cleared. A cancel there puts the previous environment back and runs nothing. A parse error keeps the new environment, as R1 says, and runs nothing.
2. The run's tag filter is put back in the filter bar and saved, as if the person chose it. Tags that no longer exist are dropped from the filter bar as after a Refresh (tag filter R5), but this re-run selects tests with the filter as written: an included tag that vanished shrinks the re-run, never widens it.
3. The run's `selection` is found again in the tree (same key and name, else the one same-kind node of that name in the same feature). If it is not found, nothing runs and the notice says "The tests of this run no longer exist".
4. That node runs as if the person clicked its run button, with today's saved properties for that environment. Scenarios added under it since run too, so the count can grow past the row's.

Re-run of a Re-run failed run (`selection` is `null`) runs its `tests` as Re-run failed does (R6), all of them, not only the failed ones. The new run keeps the same `failedOf`.

### R6. Re-run failed

- **Re-run failed** on a row is enabled when the run has at least one test with status `failed`, whatever the run's outcome. Interrupted and not-run tests do not count.
- The toolbar gets **Re-run failed (n)** next to Run all: the same action on the newest listed run, n being its failed count. It is disabled when there is no run or the newest has no failed test.
- Steps 0 to 2 of R5 apply first (secrets, environment, then filter).
- Each failed test is found again in the new tree (same key and name, else the one test of the same kind and name in the same feature).
  - A test not found is skipped; the notice says "2 failed tests no longer exist".
  - A row with no line (dynamic Examples) is skipped; the notice says "1 failed row of a dynamic Examples table can't be run alone. Run its outline."
  - If no test is left, nothing runs and the notice says "Nothing to run".
- The run has one target per feature file, `<absolute file>:L1:L2…` with the found tests' lines in ascending order, and `-e` as usual. Since `file:LINE` targets ignore `-t`, these tests run whatever their tags are now.
- The targets go in a picocli argument file, not on the command line, so any number of failed tests fits (see the probes):
  - `<app data>\projects\<hash>\run-targets.txt`, UTF-8, one target per line in `"…"` with `\` written as `\\`;
  - the command line gets `@<that file>` where the targets were;
  - it is deleted when the run ends, is cancelled or fails to start, and at app start if one is left over, as `run-args.txt` is. If it cannot be written, the run does not start.
- In the tree, exactly these tests go pending and get statuses, as a run of a node does today; the progress total is their count.
- The new run is stored with `selection: null` and `failedOf` set (R2).

### R7. Retention and Clear history

- After each run's `run.json` is written, main keeps the 20 newest run folders of the project (by folder name) and deletes the others, valid or not. A folder that cannot be deleted (held by an antivirus, say) is tried again after the next run; no notice.
- **Clear history** asks "Delete the 14 runs of this project and their reports?" with **Delete** and **Cancel**. Delete removes every run folder of the project, empties the list, disables the toolbar Report and Re-run failed, and keeps the tree's statuses (they belong to this session, not to history). A folder that cannot be deleted stays listed and the notice says so.
- The Properties dialog's secret note (environment switcher R5) gets one more sentence: "Run history keeps the reports of the last 20 runs; Clear history deletes them."

### R8. While busy

- Report, Re-run, Re-run failed (rows and toolbar) and Clear history are disabled while the runner is busy or a run is active, as Run all is. Switching tabs stays possible.

## Design decisions already made

### Where it lives

- Main owns the history: it creates run folders, writes `run.json`, lists, deletes, and resolves stored tests in the tree. It sends the window a `history` event with the listed runs (the `run.json` content plus the folder id and whether a report exists) after a project opens, after each run and after Clear history, and includes it in the replay a newly loaded window gets.
- The window sends only a run id for Report, Re-run and Re-run failed, and nothing for Clear history. Main accepts an id only if it matches `^\d{8}-\d{6}-\d{3}(-\d+)?$` and is a listed run of the open project; it never takes a path from the window.
- Re-run and Re-run failed reuse the Runner's existing environment switch, filter save and run paths. Running a list of tests instead of one node is new: the run state's selected keys become the found tests.

## Acceptance criteria

Test projects:
- The **clean sample project**: `tests/basic.feature` "failing match" fails, the others pass.
- The **env sample project** (environment switcher).
- A new **history sample project**: a `karate-config-qa.js` overlay; `tests/fix.feature` with a scenario `fixable` that fails until `flag.json` holds `{ "fixed": true }`, a scenario `stays green` tagged `@smoke`, an outline `row <n>` with rows 1 and 2 where row 2 fails, and an outline `dynamic <n>` reading `rows.json` (`[{ "n": 1 }, { "n": 2 }]`) where row 2 fails.

Every scenario is automated with Playwright against the built app, each on a fresh copy of its fixture and a fresh app data folder.

```gherkin
Feature: Run history

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  Scenario: A run appears in the history
    Given the clean sample project is open with no environment
    When I click Run all and the run finishes
    Then the History tab shows 1 run
    And its row shows "Today", "—", "All", "failed" and a duration
    And its Report, Re-run and Re-run failed are enabled

  Scenario: History survives a restart
    Given the clean sample project has 2 runs in its history
    When I close and restart the app
    Then the History tab shows 2 runs, newest first
    And the newest row's Report opens its karate-summary.html

  Scenario: A cancelled run is listed without a report
    Given the clean sample project is open
    When I run "slow loop" and click Cancel
    Then the newest row shows "Cancelled"
    And its Report is disabled and its Re-run is enabled

  Scenario: Re-run failed runs only the failed tests
    Given the history sample project is open
    And Run all has finished with "fixable" and "row 2" failed
    When I write { "fixed": true } to flag.json
    And I click Re-run failed in the toolbar
    Then only "fixable" and "row 2" go pending
    And "fixable" passes and "row 2" fails
    And the newest row shows "Failed tests of Today …" and "1 passed, 1 failed of 2"

  Scenario: A failed dynamic row is skipped
    Given the history sample project is open
    And Run all has finished with "dynamic 2" failed
    When I click Re-run failed on that run
    Then "dynamic 2" does not go pending
    And the notice says a failed row of a dynamic Examples table can't be run alone

  Scenario: Re-run switches back to the run's environment and filter
    Given the history sample project is open with environment "qa" and the filter including @smoke
    And I ran "fix.feature" and it finished
    And I chose environment "(none)" and cleared the filter
    When I click Re-run on that run
    Then the Env combo shows "qa"
    And the filter bar includes @smoke
    And only "stays green" goes pending

  Scenario: Cancelling the environment switch of a re-run runs nothing
    Given the env sample project has a run on "qa" in its history
    And the current environment is "(none)"
    When I click Re-run on that run and click Cancel during discovery
    Then the Env combo shows "(none)"
    And no run starts and the History tab still shows 1 run

  Scenario: A re-run of a test that no longer exists
    Given the clean sample project has a run of "failing match" in its history
    When I delete the "failing match" scenario from basic.feature
    And I click Re-run on that run
    Then nothing runs and the notice says the tests of this run no longer exist

  Scenario: Only the 20 newest runs are kept
    Given the clean sample project has 20 runs in its history
    When I run "passing hello"
    Then the History tab shows 20 runs
    And the oldest run's folder is gone from the app data folder

  Scenario: Clear history
    Given the clean sample project has 3 runs in its history
    When I click Clear history and confirm
    Then the History tab says "No runs yet"
    And the project's runs folder is empty
    And the toolbar Report and Re-run failed are disabled

  Scenario: A damaged run file does not hide the others
    Given the clean sample project has 3 runs in its history
    And the middle run's run.json is not valid JSON
    When I restart the app
    Then the History tab shows 2 runs

  Scenario: A secret never reaches the history files
    Given the env sample project is open with environment "qa"
    And "qa" has the secret "api.token" = "s3cret-token-123"
    When I run "always" and it finishes
    Then the run's run.json does not contain "s3cret-token-123"

  Scenario: History actions wait for the runner
    Given the clean sample project has 1 run in its history
    When a run is in progress
    Then every Report, Re-run, Re-run failed and Clear history is disabled
```

Also automated (Vitest):
- `run.json` writing and reading: every field, unknown version and malformed files not listed, counts derived from `tests`.
- Run folder ids: format, sorting, the `-2` suffix.
- Retention: keeps the 20 newest by name, deletes invalid folders beyond them, retries a folder that failed to delete.
- Start-up cleanup: folders without `run.json` and 1.0 `run\` folders deleted, valid runs kept.
- Finding stored tests again after a re-discovery: same key, moved key with the same name, ambiguous names refused, vanished tests, dynamic rows.
- Re-run failed targets: one per file, lines ascending; the targets file's quoting, `\` doubling and UTF-8 encoding; and a Node test with the bundled Java that targets in a path with a space and an `é` run exactly the listed tests.
- Re-run with a vanished included tag selects fewer tests, never more.
- Id validation of the window's requests.
- "When" labels: today, yesterday, older, across midnight.

## Left to the builder

- Component structure, styling, exact IPC names, how the tabs are built with the project's existing components.
- Whether Re-run reuses the discovery of the environment switch instead of the run's own pre-run discovery.
- Exact wording of notices and labels beyond the ones quoted above.

Resolved by the builder check (2026-10-25). The person decided that Re-run runs the node as it is now, so added scenarios run too, and that whole run folders are kept 20 deep, so F15 sees at most 20 runs. The spec author's calls: Re-run failed targets go in a picocli argument file (probed above) instead of being refused past the command-line limit; a moved or deleted project's history is not cleaned up; never-started tests are stored as `notRun`; a re-run of a Re-run failed run keeps its `failedOf`; the target environment's secrets are checked before switching; quitting during a run still writes its `run.json`.
