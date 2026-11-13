---
status: built
issue: 67
summary: "A toolbar toggle that re-runs the tests of a feature file when it is saved, remembered per project and waiting for a run in progress"
---

# Watch mode: re-run on save

Date 2026-11-03. Covers [#67](https://github.com/hpagniez/grace/issues/67).

## Intent

A person edits a feature in VS Code with Grace open beside it. Today they save, switch to Grace, find the feature in the tree and click Run. With watch mode on, the save is enough: Grace runs that feature's tests and the tree turns green or red while they keep typing.

## Goals, non-goals, anti-goals

- Goals: a save of a `.feature` file runs that feature's tests that match the filter in force; a toolbar toggle turns it on and off, remembered per project; a save during a run waits for it to end.
- Non-goals: re-running on a save of `karate-config.js`, a data file or a Java class; finding which features `read()` or `call` a saved file; re-running the last selection or the failed tests; refreshing the tree on a save without running; any change to `Tree.tsx` (selecting or revealing the saved feature).
- Anti-goals: any cost while watch mode is off; polling, or any CPU while on and no file changes; cancelling a run, or cutting a paused debug run short, because of a save; a run started by a save the person did not make in a feature root of the open project.

## Constraints

- Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)): Karate 2's `run` command has no watch option (checked with `run --help` on the bundled jar); only `karate mock -W` reloads mock files. Grace builds its own watcher, and the run it starts is an ordinary run.
- Idle costs nothing (AGENTS.md, Performance): OS file events, never a poll; a timer only between a save and its run.
- One Karate process at a time: the runner's `busy` flag refuses any second operation with "A run is already in progress".
- Windows keeps a handle on a watched folder: while it is watched, Windows refuses to rename or delete that folder's parent folders. Turning watch mode off releases it.

### What exists today

No file watcher. The tree changes on open, on Refresh, on an environment switch and in the dry run before every run (`Runner.discover`). `detectLayout` (`src/main/karate/command.ts`) gives the feature roots Grace passes to Karate: `src/test/java` and `src/test/resources` for Maven and Gradle, the project folder for a standalone project, or the `paths` of `karate-pom.json`. A feature's tree key is `f:<path>`, the path as Karate writes it; `locate(bases, path)` turns it into the absolute file. Run on a feature runs `[<absolute feature>]` with the filter's `-t`. Per-project choices (filter, threads, environment) live in `settings.json` under `projects[<root>]` (`src/main/settings.ts`).

## Approaches considered

1. **Node's `fs.watch`, recursive, on each feature root. Chosen by the owner.** On Windows it is one `ReadDirectoryChangesW` handle per root, with no walk of the tree at start, so a 2,000-test project costs what a small one does. Grace filters the events (`.feature` only, not under `.git`, `node_modules` or `.gradle`) and waits for 300 ms of quiet, as editors fire several events per save. No dependency.
2. **The `chokidar` library.** Handles event noise and atomic saves, but on Windows it wraps the same `fs.watch`, adds a dependency (about 80 kB) and stats files at start.
3. **Polling a fingerprint walk.** Ruled out: CPU while idle.

## Requirements

### R1. The toggle

1. The toolbar has a Watch toggle after Refresh: an icon button (`btnIcon`, test id `watch-button`, `aria-label` "Watch", `aria-pressed`), tooltip "Watch: run a feature when it is saved". Off, it shows `EyeOff`; on, `Eye` on the selected fill, so on and off differ by shape, not only by colour.
2. It is enabled whenever a project is open, also during a run: it touches only the watcher and `settings.json`. It is disabled while a project is being opened, and main refuses it then with "A run is already in progress", so the choice never lands on a folder whose open is then cancelled.
3. The runner checks the request is a boolean (else the notice "Invalid request"), applies it at once, saves it as `projects[<root>].watch` (`true`, or the key removed for off), and sends a `watch` event. It is applied even if it cannot be saved, with the notice "Watch mode could not be saved: <reason>".
4. Opening a project applies its saved choice once the project is open, also when its dry run failed and the tree shows the error: that is when a fix is saved next. A project with none saved starts off. A project with no feature root keeps the choice on and watches nothing. A window that loads gets the choice in force with the other replayed events.
5. Closing the project (Home), opening another one, and quitting stop the watcher and drop waiting saves. A cancelled open keeps the project before and its watcher.

### R2. What is watched

1. While on, Grace watches each feature root from `detectLayout` with a recursive `fs.watch`; a feature root that is a file is watched as a file. Each folder root's parent folder is watched too, without its subfolders, only to see the root renamed or moved away, which the root's own watcher never reports. Nothing else is watched, and nothing is watched while off.
2. An event counts only for a path ending in `.feature` (any case) with no folder named `.git`, `node_modules` or `.gradle` (`SKIPPED_DIRS`) in its path below the root.
3. A feature counts as saved once 300 ms passed with no counted event in any root (one quiet window for all, so a "Save all" is one batch). It is then checked once: a path that is no longer a file (deleted or renamed away) is dropped, and so is one whose modified time is not later than both 2 s before the watcher's start (Windows stamps files from a clock that ticks every 15.6 ms, 2 s on FAT) and the modified time Grace last counted for it. Windows can report a read or an attribute change; Karate reads every feature in each dry run and run, so without this check a run could start the next one for ever.
4. Refresh starts the watcher again on the feature roots of the new discovery (`karate-pom.json` may have changed them); waiting saves and a running quiet time are kept. The dry run before a run does not touch the watcher.
5. If a root cannot be watched (`fs.watch` throws, the watcher reports an error later, or the root itself is deleted, renamed or moved away: on a delete Windows reports the root's own path in a loop, about 50,000 events a second, and never an error; a rename or a move to the Recycle Bin shows only in the parent folder), Grace closes every watcher, turns watch mode off in memory without saving it, sends `watch` off and the notice "Watch mode stopped: <reason>". The next open of the project tries again.

### R3. What a save runs

1. Saved features run as one run once Grace is not busy: the dry run before it, the environment, threads and properties in force, the console, the history entry, exactly as a click on Run does.
2. Each saved feature that the dry run lists and that has tests matching the filter in force adds them: `<absolute feature>` as a target, with the filter's `-t`. A saved feature the tree does not list (a called feature, a file outside the feature roots' Karate paths) or with no matching test is skipped without a notice, since saving a called feature is routine. If no saved feature is left, nothing runs and nothing is said.
3. A run of one feature records that feature as its selection, so Re-run runs it again. A run of several records no selection, and Re-run runs the same tests by line, as it does for a Re-run failed run.
4. A failed dry run shows its error in the tree as Refresh does, and nothing runs.

### R4. A save while Grace is busy

1. A save while Grace is busy (a run, a debug run, paused or not, a discovery, a library listing) waits. Saves add up with no duplicates. When the busy operation ends, the waiting features run as R3 says. If a click of the person's starts an operation first, the saves keep waiting, with no notice, until that one ends too.
2. Cancel drops the saves waiting when it is clicked, along with the run it stops: Cancel means stop. A save after the click waits as any other.
3. A run started by a save is an ordinary run: Cancel stops it, and other actions are refused while it runs.
4. Turning watch mode off drops the waiting saves.

## Cost

- **Off:** nothing. No handle, no listener, no timer.
- **On, no file changes:** one OS directory handle per feature root (two for a Maven or Gradle project, one for a standalone one) and libuv's change buffer per handle, 4 KB each, plus one handle on each folder root's parent, without its subfolders. No walk of the project at start on Windows, whatever its size: the 2,000-test bench project costs what a small one does. Zero CPU: the OS wakes Grace only when a file changes.
- **On, a file changes:** one string check per event, and one `stat` per saved feature when its batch is handed over. A burst bigger than the 4 KB buffer (a `git checkout`, a "Save all" of many files) loses its file names: that batch then walks the folder roots for features and keeps those whose modified time moved, so no saved feature is missed; the walk happens only then; a map of the modified time last counted per saved feature (a few saved files, not the project). Events outside `.feature` files (an IDE's index, a `git` command in a standalone project's `.git`) cost that check only. A 300 ms timer lives from the first counted event until its batch is handed over; none otherwise. A `git checkout` that rewrites many features is one batch and one run.
- **During a run:** a set of waiting paths; nothing else.
- **Start:** nothing while off; one `fs.watch` call per root after an open's discovery while on (under 1 ms each).
- **Size:** about 2 KB of code; no dependency.
- **Checked by hand before the PR is ready:** on the bench's 2,000-test project, idle CPU and the main process's memory with watch mode on and off.

## Design decisions already made

- What a save runs: the saved feature's tests that match the filter (owner).
- Only `.feature` saves count; config, data and code files are ignored (owner).
- A toolbar toggle, remembered per project (owner).
- A save during a run waits and runs once the run ends (owner).
- `fs.watch` recursive, no dependency (owner, approach 1).
- A run started by a save goes into the history like any run, and counts toward its 20 runs (owner). Flaky detection is safe: a save changes the project fingerprint, so these runs are never compared with one another.
- My call: several saved features run as one run, not one run each, so a "Save all" makes one history entry.
- My call: 300 ms of quiet across all roots before a batch counts.
- My call: a saved feature with nothing to run is skipped without a notice (R3.2).
- My call: Cancel drops waiting saves (R4.2).
- My call: a watcher error turns watch mode off in memory, not in `settings.json` (R2.5).
- My call: the toggle sits after Refresh, an icon button like it, to fit the 760 px window.
- My call: a save counts only if the file's modified time moved (R2.3), so reads by Karate, an antivirus or an indexer never start a run.
- My call: a project whose dry run failed at open is watched (R1.4); the toggle is disabled during an open (R1.2).

## Left to the builder

- The module split: a watcher module in `src/main/` (roots in, saved absolute paths out, `close()`), owned by `Runner`.
- How a saved absolute path finds its tree feature (`locate` each feature's path, or `path.relative` to each base), as long as case and separators match on Windows.
- The run target shape for several features inside `runTarget`.

## Acceptance criteria

Scenarios run in Vitest (Node for main with a real temporary folder and real `fs.watch`; jsdom for the window) unless marked **(e2e)**, which run in Playwright against the built app.

```gherkin
Feature: Watch mode

  Background:
    Given a standalone project with "a.feature" and "b.feature", each with a passing scenario

  # R1
  Scenario: The toggle turns watch mode on and is remembered
    Given the project is open with watch mode off
    When I turn on Watch
    Then main saves watch mode on for the project
    And the Watch button is pressed
    When I open the project again
    Then watch mode is on

  Scenario: Main refuses a malformed request
    When main is asked to set watch mode to "yes"
    Then it shows "Invalid request"
    And watch mode does not change

  Scenario: The toggle works during a run
    Given a run is going
    Then the Watch button is enabled

  # R2, R3
  Scenario: A saved feature runs
    Given watch mode is on
    When I save "a.feature"
    Then after the quiet time one run starts with "a.feature" as its selection
    And "b.feature" does not run

  Scenario: Nothing runs while off
    Given watch mode is off
    When I save "a.feature"
    Then nothing runs

  Scenario: Only features count
    Given watch mode is on
    When I save "karate-config.js" and "data.json"
    Then nothing runs

  Scenario: An event with no new modified time does not count
    Given watch mode is on
    And "a.feature" was saved and its run ended
    When the watcher reports "a.feature" again with the same modified time
    Then nothing runs

  Scenario: A broken project is still watched
    Given watch mode is saved on
    And "a.feature" has a parse error
    When I open the project
    Then the tree shows the error
    When I fix and save "a.feature"
    Then a run of "a.feature" starts

  Scenario: Skipped folders do not count
    Given watch mode is on
    When a file "node_modules/x/c.feature" is written
    Then nothing runs

  Scenario: Save all is one run
    Given watch mode is on
    When I save "a.feature" and "b.feature" within 100 ms
    Then one run starts with the tests of both and no selection

  Scenario: The filter applies
    Given watch mode is on
    And the filter includes only "@smoke"
    When I save a feature with one "@smoke" scenario and one untagged scenario
    Then only the "@smoke" scenario runs

  Scenario: A called feature is skipped quietly
    Given watch mode is on
    When I save a feature whose only scenario is tagged "@ignore"
    Then nothing runs
    And no notice shows

  # R4
  Scenario: A save during a run waits
    Given watch mode is on
    And a run of "b.feature" is going
    When I save "a.feature"
    Then no second run is refused
    And when the run ends, a run of "a.feature" starts

  Scenario: Cancel drops waiting saves
    Given watch mode is on
    And a run is going
    And "a.feature" was saved during it
    When I cancel
    Then no run starts after it

  # R2.5
  Scenario: A watcher error stops watch mode
    Given watch mode is on
    When the watcher reports an error "EPERM"
    Then the notice "Watch mode stopped: EPERM" shows
    And the Watch button is not pressed
    And the project's saved choice is still on

  Scenario: Closing the project stops watching
    Given watch mode is on
    When I go Home
    And I save "a.feature"
    Then nothing runs

  Scenario: Re-run on save in the app (e2e)
    Given the project is open
    When I turn on Watch
    And a test writes a failing step into "a.feature"
    Then the tree shows "a.feature" failed
    And the history has a run of "a.feature"
```
