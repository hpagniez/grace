# Flaky tests

Date 2026-10-26. Status: approved. Covers [feature](../features.md) F15 (flaky tests). The pass rate over time, the other half of the old F15 row, moves to its own row, F42.

## Intent

When a project has been run many times, the person wants to know which tests they cannot trust: tests that passed in one run and failed in another although nothing in the project changed. Today the History tab (F10) lists each run with its counts, but nothing points at a test that flips. The person has to remember which run failed what.

This feature marks those tests in the tree and shows, for each one, the runs that prove it. The person decides what to do about a flaky test; the app only shows it.

No Karate Labs tool shows flaky tests ([commercial inventory](../inventory/commercial.md#user-needs-a-free-runner-must-cover)). The data is already on disk: every run's `run.json` stores each test's final status (F10). What is missing is knowing whether the project changed between two runs. A test that fails, then passes after the person fixed it, is not flaky.

It builds on the [run history](2026-10-25-run-history.md), the [environment switcher](2026-10-25-environment-switcher.md) and the [thread count](2026-10-26-thread-count.md).

## Who and what success looks like

The walking skeleton's user, iterating on an API suite all day.

Success:
- A test fails in a run of the whole suite. The person changes nothing and clicks Re-run failed, and the test passes. The tree now marks it **flaky**, and its feature and folder show **1 flaky**.
- Selecting it shows "Flaky with no environment: passed 1 time, failed 1 time with the same project files", then the two runs, each with its time, result, thread count and a Report link.
- A test that failed, was fixed in the project, then passed, is not marked.
- A test that passes on `dev` and fails on `qa` is not marked.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- Pass rate or duration over time, charts, suite-level trends. That is F42.
- Actions on a flaky test: re-run it several times to confirm, tag or quarantine it, skip it. The person handles it themselves.
- Keeping more than F10's 20 runs per project. The person decided 20 runs is enough.
- Flakiness across environments: a pass on one and a fail on another is an environment difference.
- Telling the person which file changed between two runs.
- A setting for which folders the fingerprint skips.
- Marks from runs stored before this feature: they have no fingerprint.
- Anything new in the History tab, the console or the HTML report.
- Notices or alerts when a test becomes flaky.

**Anti-goals** (failure even if everything else works):
- A test marked flaky when its pass and fail came from runs whose checked project files differed.
- A mark, or proof, on a different test than the one that flipped, after the tree changed.
- A run that fails, is refused, or waits noticeably because of the fingerprint.
- A secret value in `run.json`.
- History runs that vanish because they were written before this feature.

## Constraints

- No new dependency. `node:crypto` and `node:fs` are enough. Solid 2 rules (AGENTS.md). `contextIsolation`, `sandbox` and `nodeIntegration: false` stay. The renderer reaches main only through the typed preload API.
- Storage stays as [ADR 0010](../decisions/0010-run-history-one-folder-per-run.md) and F10 decided it: one folder per run, the 20 newest kept per project.

### What the app does today

- `run.json` (`RunRecord` in `src/shared/history.ts`) holds per run: `startedAt`, `env`, `threads`, `filter`, `selection`, `failedOf`, `outcome`, `exitCode`, and `tests`. Each test has `kind`, `file`, `refId`, `name`, `line`, `key` and its final `status` (`passed`, `failed`, `notRun`, `interrupted`).
- `parseRunRecord` drops a record with any malformed field. F12 added `threads` without a version bump. A missing `threads` reads as null, because old records lack it and old app versions ignore it (thread count spec, R4). This feature adds its field the same way.
- `listHistory` (`src/main/history.ts`) reads the valid records, newest first, at most 20. Main sends them in the `history` event after a project opens, after each run, after Clear history, and in the replay. The window gets `HistoryEntry`, which has counts but no `tests`.
- Every discovery, including the one before each run, publishes a new tree in the `project` event (`Runner.discover`). A test's key can shift when a scenario is inserted above it. `resolveNode` (`src/main/karate/tree.ts`) finds a stored test again: same key and name, else the one node of the same kind and name in the same feature. It refuses to guess.
- `Runner.runBody` (`src/main/runner.ts`) creates the run folder, writes the argument files, spawns Java, then writes `run.json` in `store` when the run ends. Quitting during a run still writes it, within the 10 s bound of `dispose`.
- The window opens a past run's report with `Api.openRunReport(id)` (`CH.runReport`). Main checks the id against the listed runs and never takes a path from the window.
- Karate reads more than the classpath roots: `karate-config*.js` overlays (the app looks for them in the project root and the classpath roots), `karate-pom.json` in the project root, and `file:` paths relative to its working directory, which is the project root.

Probe for this spec (2026-10-26, Karate 2.1.4, bundled Java 21, Windows 11): a run of a copy of the history sample project, the way the app runs it (`-o` outside the project, `-T 2`, exit 1), left every file in the project folder with the same path, size and modified time, and added none. So Karate itself does not change the fingerprint of a run.

Walk timing (same machine, Node 24, warm disk cache): `readdir` with `withFileTypes` plus `lstat` of each file over this repository's `node_modules` (14,956 files) took 1.5 to 1.7 s one entry at a time and 0.55 to 0.67 s with each folder's entries in parallel. A test project of a few hundred files takes milliseconds; the 50,000-file cap is at about the 2 s bound.

## Approaches considered

How "the project files did not change" is checked:
1. **A fingerprint from file details.** Hash each file's path, size and modified time at the start and at the end of each run, and store it in `run.json`. No file contents are read, so it is fast on a large folder. A save that changes nothing, or a switch of git branch and back, looks like a change. That can hide a flaky signal, never invent one. **Chosen by the person.**
2. A fingerprint from file contents. Exact, but it reads every byte of the project twice per run.
3. Ask git: the commit plus a hash of `git status`. It needs git and a repository, it misses ignored local files, and it starts another process each run.

Where the mark and its proof show (mockups compared in the brainstorm):
1. **A mark on the test's row in the tree, counts on the folders, features and outlines above it, and the proof in the Details pane.** No new tab. **Chosen.**
2. A Flaky tab in the bottom panel listing every flaky test, with the proof in each row. The tree gets only the mark.
3. Both: the tree and Details as in 1, plus a Flaky tab as an index.

How many runs to judge from:
1. **F10's 20 runs per project, unchanged.** **Chosen by the person.**
2. Keep the small per-test results for more runs than the report folders. That needs a new decision record.

## Requirements

### R1. Project fingerprint

1. The fingerprint is the SHA-256, as 64 lowercase hex characters, of one line per file under the project root: `<path relative to the root, with / separators>\t<size in bytes>\t<modified time in whole milliseconds>\n`. The lines are sorted by path, comparing UTF-16 code units, which is JavaScript's default sort.
2. Every file under the project root counts, at any depth, except what is inside a folder named `.git`, `node_modules` or `.gradle`, and inside a folder named `target` or `build` that sits next to a `pom.xml`, `build.gradle` or `build.gradle.kts` (a Maven or Gradle output folder). A test folder named `build` or `target` counts. The names are compared ignoring case. Symbolic links and junctions are neither followed nor listed. The whole root counts, not only the classpath roots, because Karate also reads config overlays, `karate-pom.json` and `file:` paths relative to the project root (spec author's call).
3. The fingerprint is null when:
   - the project holds more than 50,000 files after the skips;
   - any folder or file under the root cannot be read or stat'ed;
   - it is not done within 2 s.
   Computing it never throws.
4. It never blocks the main process: the folder walk is asynchronous, with the stats of a folder's entries taken in parallel (a sequential walk took 1.5 s for 15,000 files in the probe below, a parallel one 0.6 s).
5. A run takes it twice:
   - just before Java starts, after every pre-run check has passed;
   - after Java has exited, before `run.json` is written. The window gets `runFinished` once this walk is done, so it never shows an idle runner that still refuses a run; on a few hundred files that is milliseconds.
   The record stores it only if both are non-null and equal. Otherwise it stores null: a file was saved during the run, or the walk failed.
6. A cancelled run takes the second walk like any other. A run ended by quitting the app stores null, without the second walk, so the quit stays within its 10 s bound.
7. A run that never started Java (the pid file could not be written) stores null.
8. Discovery dry runs take no fingerprint.

### R2. What a run stores

1. `run.json` gets a `fingerprint` field: the value of R1, or null. `version` stays 1. A record without the field reads as null. A record whose `fingerprint` is present but neither null nor 64 lowercase hex characters is malformed and is not listed, like any other bad field. As with `threads` (F12), an added field that old records lack and old app versions ignore drops no run, so there is no version bump and no new decision record.
2. The fingerprint holds only paths, sizes and times hashed together. It holds no property value and no file content.
3. The window does not need the fingerprint. `HistoryEntry` may carry it or not (builder).

### R3. Which tests are flaky

Worked out by main from the listed runs (the same up to 20 valid records the History tab shows):

1. Runs whose `fingerprint` is null are left out.
2. The other runs are grouped by fingerprint and environment (`env`, null being its own value). The thread count, tag filter, selection, outcome and exit code play no part.
3. Within a group, the tests are matched by their stored `key`. Keys are stable while the files are the same. A test is **flaky in that group** when at least one run of the group has it `passed` and at least one has it `failed`. `notRun` and `interrupted` do not count.
4. The **proof** of a test flaky in a group is every run of that group where the test is `passed` or `failed`.
5. Each test flaky in a group is placed in the current tree with `resolveNode`, from the stored test of the group's newest run in its proof. If it is not found, or the name is ambiguous, that proof marks nothing.
6. Per tree test and environment, the proofs of every group that resolve to it are merged: the count of runs where it passed, the count where it failed, and the runs, newest first. A test is **flaky** when it has at least one such environment.
7. Main sends the result to the window with every `history` event (open, each run, Clear history, replay) and again with every new tree (the `project` event of any discovery). The marks then always follow the tree that is shown. The window gets, per flaky test of the current tree, its current key and, per environment, the counts and the proof runs: `id`, `startedAt`, `status`, `threads` and whether the report exists. It gets nothing about tests that are not flaky.
8. With no project, or no tree (a parse error), nothing is flaky.

### R4. In the tree

1. A flaky scenario or outline row shows a **flaky** mark after its name, before its ▶ run button: amber, readable in light and dark mode, `data-testid="flaky-mark"`. Its tooltip and accessible name are "Flaky: passed and failed with the same project files".
2. A folder, feature or outline with at least one flaky test under it shows **n flaky**, with `data-testid="flaky-count"`. n counts each flaky test once, whatever its number of environments, and includes tests hidden by the tag filter. Flakiness is not about tags.
3. Marks show whatever the current environment is.
4. A dimmed node keeps its mark. A node hidden by "hide non-matching" is hidden with its mark.

### R5. In the Details pane

1. When the selected node is a flaky test, the Details pane shows one box per environment where it is flaky (`data-testid="flaky-proof"`). The box goes after the location, duration and tags and before the error. The current environment comes first, then the others: no environment first, then by name.
2. Box heading: "Flaky on qa", or "Flaky with no environment" for null. Then: "Passed 3 times, failed 2 times with the same project files", with "1 time" for one.
3. Then the proof runs, newest first, one row each:
   - "When", as the History tab writes it ("Today 14:32");
   - "passed" or "failed";
   - the thread count ("1 thread", "8 threads", "—" for null);
   - a **Report** button that opens that run's report with `openRunReport`. It is enabled only when the report exists and the runner is not busy, as in the History tab.
4. A test that is not flaky, and a folder, feature or outline, show nothing new.

### R6. Lifetime of a mark

1. A mark lasts while its proof is among the listed runs. Fixing the test does not clear it. It goes when retention deletes the proving runs (keeping the 20 newest), or when Clear history deletes them all. The proof rows show the dates, so the person can judge.
2. A damaged or unreadable `run.json` is not listed, so it proves nothing.

## Design decisions already made

- Definition (the person): only flips where the project files did not change count as flaky. Same environment too (the person accepted the proposal).
- Fingerprint from path, size and modified time (the person, option 1 above).
- Placement A: tree mark with counts, proof in Details (the person, from the mockups).
- 20 runs, F10's storage unchanged (the person).
- No actions on flaky tests (the person).
- Main owns the computation, as it owns the history (F10, "Where it lives"). The window never reads `run.json`.

### Known limits (accepted)

- A save that changes nothing, a git branch switch and back, or a tool that touches files changes the fingerprint. Such a flip is not marked.
- Saved run properties (F08) are not part of the fingerprint. A flip caused by editing a property between two runs is marked flaky.
- A project whose tests write into its own folder, outside the skipped folders, changes its files during every run. Its runs store null, so it never gets marks.
- Files outside the project folder are not checked: a shared data folder elsewhere, the server under test, a database. By design, a flip caused by those is what flaky means here.
- Twenty runs can be one morning of work. A flip older than the 20 newest runs is gone.

## Acceptance criteria

Test projects:
- The **history sample project** (F10): `tests/fix.feature` has `fixable`, which fails until `flag.json` holds `{ "fixed": true }`, and `stays green`.
- A new **flaky sample project**: a `karate-config-qa.js` overlay and `tests/coin.feature` with:
  - a scenario `coin`, which passes when a `coin.json` outside the project folder holds `{ "heads": true }` and fails otherwise;
  - a scenario `steady`, which always passes.
  Each test gets its own `coin.json`.

Every scenario is automated with Playwright against the built app, each on a fresh copy of its fixture and a fresh app data folder, unless it is marked **(unit)**. Unit scenarios run in Vitest.

```gherkin
Feature: Flaky tests

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  Scenario: A test that flips with no change is marked flaky
    Given the flaky sample project is open with no environment and 2 threads
    And coin.json holds { "heads": false }
    When I click Run all and the run finishes with "coin" failed
    And I write { "heads": true } to coin.json
    And I set Threads to 1 and click Re-run failed in the toolbar
    Then "coin" passes and shows the flaky mark
    And "coin.feature" and "tests" show "1 flaky"
    And "steady" shows no flaky mark
    When I select "coin"
    Then Details shows "Flaky with no environment"
    And "Passed 1 time, failed 1 time with the same project files"
    And 2 proof rows: the newest "passed" with "1 thread", the other "failed" with "2 threads"

  Scenario: A fix in the project is not flaky
    Given the history sample project is open
    And Run all has finished with "fixable" failed
    When I write { "fixed": true } to flag.json
    And I click Re-run failed in the toolbar and "fixable" passes
    Then "fixable" shows no flaky mark

  Scenario: Different environments are not compared
    Given the flaky sample project is open with no environment
    And coin.json holds { "heads": false }
    And I ran "coin" and it failed
    When I write { "heads": true } to coin.json
    And I choose environment "qa" and run "coin" and it passes
    Then "coin" shows no flaky mark

  Scenario: The proof opens a run's report
    Given "coin" is marked flaky as in the first scenario
    When I select "coin" and click Report on the "failed" proof row
    Then that run's karate-summary.html opens

  Scenario: The mark stays on its test when the tree changes
    Given "coin" is marked flaky as in the first scenario
    When I add a scenario "added" above "coin" in coin.feature and click Refresh
    Then "coin" shows the flaky mark
    And "added" shows no flaky mark

  Scenario: Marks survive a restart
    Given "coin" is marked flaky as in the first scenario
    When I close and restart the app
    Then "coin" shows the flaky mark

  Scenario: Clear history removes the marks
    Given "coin" is marked flaky as in the first scenario
    When I click Clear history and confirm
    Then no node shows a flaky mark or a flaky count

  Scenario: A file saved during a run does not count (unit)
    Given a run whose fingerprint before Java differs from the one after
    Then its run.json stores fingerprint null

  Scenario: Runs without a fingerprint prove nothing (unit)
    Given 2 listed runs of the same environment, one passed and one failed for a test
    And one of them has fingerprint null
    Then the test is not flaky
```

Also automated (Vitest):
- Fingerprint:
  - the same tree of files gives the same value;
  - a changed size, a changed modified time, an added file, a removed file and a renamed file each give a new value;
  - files under `.git`, `node_modules` and `.gradle`, and under `target` and `build` next to a `pom.xml`, `build.gradle` or `build.gradle.kts`, at any depth and in any case, do not count; a test folder named `build` or `target` does;
  - a symbolic link or junction is not followed;
  - more than 50,000 files, an unreadable folder, or the 2 s bound give null;
  - it never throws.
- `run.json`: a missing `fingerprint` reads as null; null and a 64-character lowercase hex value round-trip; any other value makes the record malformed.
- Flakiness:
  - grouping by fingerprint and environment;
  - the thread count, filter and outcome play no part;
  - `notRun` and `interrupted` do not count;
  - one passed and one failed run make a test flaky, two passed do not;
  - proofs of two fingerprints of one environment merge;
  - two environments give two entries;
  - placement through `resolveNode`: a moved key with the same name, an ambiguous name, a vanished test;
  - the counts on folders, features and outlines.
- A run ended by quitting stores fingerprint null and stays within the 10 s bound.
- The Details proof box: the order of environments, "1 time" and "2 times", thread labels, Report enabled only with a report and while not busy.

## Left to the builder

- Whether the flaky result rides in the `history` and `project` events or in its own event, and its exact shape and names.
- Component structure and styling, using the existing tree row and Details components.
- How the folder walk is written (`fs.promises.readdir` with `withFileTypes`, `lstat`), and how the 2 s bound and the 50,000-file cap are enforced.
- Whether `HistoryEntry` carries the fingerprint.
- Where the e2e tests keep each test's `coin.json`, so that it is outside the project folder and not shared between tests. The e2e helpers can seed `run.json` records directly for the scenarios that only need marks (`e2e/helpers.ts`). The first scenario must run Karate for real, to prove the fingerprint is stable across runs of an unchanged project.
- Exact wording of tooltips and labels beyond the ones quoted above.

Builder check (2026-10-26): nothing important left. The reviewer confirmed against the code that `fingerprint` can be added as `threads` was, that `resolveNode` and the keys behave as R3 assumes, and that only completed scenarios are stored `passed` or `failed` (crashed, cancelled or never-run ones are `interrupted` or `notRun`). The spec author's calls: the whole project root is fingerprinted, not only the classpath roots (R1.2); the walk is bounded to 2 s and 50,000 files, after timing it (R1.3, R1.4); the second walk comes before `runFinished`, a Cancel clicked during it does not change the run's outcome, and a quit skips it (R1.5, R1.6); saved run properties stay out of the fingerprint (Known limits); counts on folders include tests hidden by the tag filter (R4.2); the order of environments in Details (R5.1).
