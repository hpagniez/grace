# Walking skeleton: open, discover, run, watch

Date 2026-10-24. Status: draft for approval. Covers [features](../features.md) F01, F02, F03, F05, F06, F07, F09.

## Intent

The first usable version of karate-kid, and the path every later feature builds on. A person opens a Karate project folder, sees every test, runs any part of it, watches each test go green or red live, and understands a failure without leaving the app. Nothing to install: Java and Karate ship inside the app.

It proves the full chain once: window → main process → bundled Java running Karate → event file → main process → window.

## Who and what success looks like

A developer or tester on Windows with a Maven or Gradle Karate project that uses only feature files, JS and data files (no custom Java classes).

Success:
- After opening the folder, the full test tree appears within about 2 s.
- Any node (suite, folder, feature, outline, scenario, outline row) runs with one click.
- Status, counters and console update while the run is going, not at the end.
- A failed scenario shows its error, location and steps (with HTTP log text) in the app.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals for this version** (each is a later feature):
- environment switcher (`-e`): runs use no environment, so `karate.env` is unset
- tag filtering, thread count, run history, re-run failed
- editing files, settings screens, watch mode
- projects with custom Java classes (needs compiled classes and dependency jars on the classpath)
- macOS and Linux, installers, auto-update, code signing
- mock servers, live per-step events (needs our own Java `RunListener`)

**Anti-goals** (failure even if everything else works):
- Showing "passed" when Karate ran nothing. Karate exits 0 for a missing path, an unknown environment or zero matching scenarios.
- Leaving a Java process running after Cancel or after closing the app.
- Writing into the user's project. Run output goes to the app's own data folder. (Karate itself may still create `target/karate-temp` or `build/karate-temp`; accepted.)
- Freezing the window while a run produces a lot of output.

## Constraints

- Windows 11 x64 only.
- Stack per [ADR 0001](../decisions/0001-solid-router-spa-without-tanstack-start.md), scaffolded from the [stack spike](../research/2026-10-24-stack-spike/README.md). Follow the Solid 2 rules in [AGENTS.md](../../AGENTS.md).
- Java and Karate bundling per [ADR 0006](../decisions/0006-bundle-jre-and-karate-jar.md): Temurin JRE `21.0.12.1+1`, `karate-2.1.4.jar`.
- Karate behaviour per the [CLI spike](../research/2026-10-24-karate-cli-spike/README.md). Where this spec and the spike disagree, the spike's observed behaviour wins and this spec gets fixed.

## Approaches considered

Where the run logic lives (spawning Java, reading the event file, building the tree, tracking status):

1. **Main process owns the run.** A runner module in the Electron main process spawns Java, tails `karate-events.jsonl`, and sends a few typed events to the window. The window only applies events to a Solid store and renders. **Chosen.** The parser is plain Node code tested against the spike's real event files, Java and the file system never reach the window, and the module can later move to a utility process without changing the window.
2. **Main forwards raw lines; the window parses.** Less code in main, but parsing and tree logic land in the browser side, and dry-run handling splits across both. No real gain.
3. **Option 1 in a separate Node utility process.** Protects the main process from large `FEATURE_EXIT` parses, at the cost of another process and messaging layer. Not needed at MVP scale.

## Requirements

### R1. Open a project

- The toolbar has **Open…**, which shows a folder picker.
- Layout detection on the chosen folder:
  - `pom.xml` → Maven; `build.gradle` or `build.gradle.kts` → Gradle. Classpath roots are `src/test/java` and `src/test/resources`, whichever exist. Feature roots are the same folders.
  - Neither → standalone. The folder itself is the classpath root and the feature root.
- The toolbar shows the folder path and the detected layout ("Maven", "Gradle", "Standalone").
- The last opened folder is saved and reopened on the next start if it still exists.

### R2. Discover tests (dry run)

- Opening a project, and the **Refresh** button, run a Karate dry run (`-D`) on all feature roots and build the test tree from its events.
- Tree levels: folder → feature → scenario, and for outlines folder → feature → outline → row.
  - Folders mirror the feature paths relative to the project.
  - Only depth-0 events (`callDepth == 0`) make nodes. Called features and `callSingle` features do not appear.
  - `refId` is `[section:line]` for a scenario and `[section.example:line]` for an outline row. Node key: feature path + section (+ example) from `refId`, **without the line**, so a key survives edits that only move lines. The line is stored on the node. Slugs are not unique; do not use them.
  - Rows belong to the outline whose `OUTLINE_ENTER` precedes them in the feature's event order (the event carries no section number). The outline's name and line come from that event. Verify with the two-Examples-table fixture (`extra/outline-sizes.feature`).
  - Tags are shown next to scenario names, read-only.
  - Features tagged `@ignore` are excluded by Karate itself, which is how projects keep helper and mock features out of runs. The app adds no filtering of its own.
  - Dry-run scenarios report both `passed` and `skipped`; treat them as "not run".
- Empty and error states:
  - No feature files: the tree area says so.
  - Dry run exits 3 (a feature has a syntax error): the tree area shows the error message from Karate's output, and Run is disabled until a Refresh succeeds.

### R3. Run

- **Run all** in the toolbar, and a ▶ button on every tree node.
- What each node runs:
  - suite: all feature roots
  - folder or feature: its absolute path
  - scenario: `feature.feature:LINE` with the scenario's line
  - outline: `feature.feature:LINE` with the outline's line (runs all its rows)
  - outline row: `feature.feature:LINE` where LINE = the line of the row in the feature file (see "Outline row lines" below). A row whose line cannot be found (dynamic Examples such as `| read('rows.json') |`) does not run: show "Run the outline to run its rows".
- One run at a time. While a run is going, Run buttons are disabled and **Cancel** is enabled.
- Every run first repeats the full discovery dry run of R2 (users edit features in their own editor, so the tree may be stale). Then:
  - exit 3 → show the error, do not run
  - the tree is rebuilt. Nodes whose key still exists keep their status; new nodes start as "not run"; removed nodes disappear.
  - the clicked node is found in the new tree: first by key if the name is unchanged, else by feature path + name if exactly one node matches (scenarios inserted above shift section numbers). If neither finds it, do not run: show "This test changed; the tree was refreshed" so the user can click again. This prevents running a different scenario than the one clicked.
  - the run's command line uses the lines from the new tree
  - `total` = scenarios and rows under the node in the new tree. Zero → show "Nothing to run", do not run.
- Nodes in the selection reset to "pending" when the run starts. Nodes outside it keep their last status.

### R4. Live status and progress

- Each scenario and row shows one of: not run, pending, running, passed, failed, interrupted.
- Feature, outline and folder status is derived from children: any failed → failed, else any running → running, else all passed → passed.
- A progress section under the toolbar shows a bar split into passed and failed, `done / total`, passed count, failed count, and elapsed time.
  - `done` counts depth-0 `SCENARIO_EXIT` events (Karate emits no progress event). `total` comes from the pre-run discovery (R3).
- Status changes reach the window within about 0.5 s of Karate writing them.
- End of run:
  - exit 0 or 1 with a `SUITE_EXIT` event → "Finished" with Karate's summary
  - Cancel → "Cancelled"; scenarios still running become "interrupted"
  - exit 2 or 3, or the process ended without `SUITE_EXIT` → "Run failed", and the console keeps Karate's output for diagnosis

### R5. Details pane

- Clicking a scenario or row shows: name, status, relative `file:line`, duration, tags, and the error text if it failed.
- Below that, the step list: keyword + text, a status icon and duration for each step; background steps are marked. Expanding a step shows its log text (HTTP request and response, `print` output) as preformatted text.
- Steps come from the depth-0 `FEATURE_EXIT` event (`scenarioResults[].stepResults`), matched to scenarios by `refId`. Until the feature finishes, the pane says the steps are not available yet.
- Steps of called features (`callResults`) are not expanded in this version.
- Clicking a feature, outline or folder shows its name, path and child counts by status.

### R6. Console

- One console pane below the tree and details. It shows Karate's stdout and stderr lines merged in arrival order. It is cleared when a run starts.
- Lines containing ` ERROR ` are shown in red, ` WARN ` in amber. No other parsing of console text.
- It follows new output, unless the user has scrolled up.
- It keeps the last 10,000 lines.

### R7. Cancel and shutdown

- Cancel runs `taskkill /PID <pid> /T /F` and marks the run "Cancelled" in the app (Karate exits 1 after a kill, the same as a test failure).
- Closing the app during a run kills the process tree first.
- If the app itself crashes or is force-killed, the JVM survives. Partial cover for this version: write the Java PID to `<app data>\running.pid` when a run starts and delete it when the run ends. On the next start, if the file exists and that PID is still a `java.exe` whose path is inside the app's own `jre` folder, kill its tree. Never kill a process by PID alone (PIDs are reused). Full cover (a Windows Job Object) needs a native module and is out of scope.
- After Cancel, Open report stays disabled (Karate writes no HTML after a kill).

### R8. Open report

- **Report** in the toolbar opens `karate-summary.html` of the last finished run in the default browser. It is enabled only when that file exists.

## Design decisions already made

### Spawning Karate

All runs and dry runs go through the main process, never through a shell:

```
<resources>\jre\bin\java.exe
  -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8
  -cp "<resources>\karate.jar;<classpath roots>"
  io.karatelabs.Main run
  --no-color -w <project> -o <output dir> -B false
  -f html,karate:jsonl [-D]
  <absolute feature paths, or file.feature:LINE>
```

- `cwd` = the project folder. Feature paths are absolute.
- Remove `KARATE_OPTIONS`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS` and `_JAVA_OPTIONS` from the child's environment.
- Output dir: `<app data>\projects\<hash of project path>\run` for runs and `...\dryrun` for dry runs. Delete its contents before each spawn, so no report or event file from an earlier run is ever read or opened. `-B false` turns off report backups, since there is no run history yet.
- `<resources>` is `vendor\` in the repo during development and `process.resourcesPath` in the packaged app.

### Outline row lines

A row's events give its `Examples:` line (in `refId` and `line`) and its index within that table, not its own line. After each discovery the app reads the feature file: from the line after `Examples:`, it skips blank lines and `#` lines, takes the first `|` line as the header, then collects the following `|` lines (still skipping blank and `#` lines) until the first other line. The row's line is the entry at its index in that table. If there is no such entry (a dynamic table, or fewer table lines than row events), the row's line is null and the row cannot be run on its own. Adding 2 + the index to the `Examples:` line is wrong as soon as a table holds a blank or commented-out line.

### Reading the event file

- Poll `karate-json\karate-events.jsonl` every 100 ms, reading from the last byte offset, and keep any partial last line for the next read. Do not rely on file-change notifications.
- Translate Karate events into a few app events and send them to the window. Batch console lines and status updates per poll so a busy run does not flood the IPC channel.

### Main ↔ window contract

Typed, in one shared module, exposed through the preload with `contextBridge`. Shape (names may change):

- Window → main: open project (shows the dialog), refresh, start run (a node key, or all), cancel, open report.
- Main → window: project loaded (path, layout, tree, or an error), run started (total), scenario updates (key, status, duration, error), steps for a scenario (key, steps), console lines, run finished (outcome, summary, exit code).

### Bundled Java and Karate

- `scripts/fetch-vendor.mjs` downloads, checks and unpacks into `vendor\` (git-ignored):
  - JRE: `https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jre_x64_windows_hotspot_21.0.12.1_1.zip`, SHA-256 `d35f31e712f0fcf6ac5a093edc90204fbff22f720ba3950bd09d331d5e621636`
  - Karate: `https://github.com/karatelabs/karate/releases/download/v2.1.4/karate-2.1.4.jar`, SHA-256 `1f743f4acc56526b39bd88c84bfe4491c46d5e94f4ab5a38c437c9400dbc2488`
  - It fails loudly on a checksum mismatch.
- Forge `packagerConfig.extraResource` copies `vendor\jre`, `vendor\karate.jar` and both licence files into the packaged app.
- Distribution for this version: the `electron-forge package` folder, zipped. No installer.

### Repository

- `.gitattributes` with `* text=auto eol=lf`, so event-file fixtures keep LF line endings on Windows checkouts.
- Unit tests with Vitest per [ADR 0005](../decisions/0005-vitest-for-unit-tests.md); end-to-end tests with Playwright per [ADR 0004](../decisions/0004-playwright-for-end-to-end-tests.md).
- `features.md` rows F06, F07 and F09 are reworded to what this spec delivers (no ANSI or console filter, no jump to line, report in the default browser). Console filtering becomes its own `could` row.

## Acceptance criteria

Test project: a fixture built from the CLI spike's [sample project](../research/2026-10-24-karate-cli-spike/project).
- The **clean sample project** keeps `tests/`, `karate-config*.js`, `start-mock.feature` and `mocks/`, and drops `broken/` and `extra/`. `start-mock.feature` and the mock features get an `@ignore` tag so they are not discovered as tests (`tests/common/create-user.feature` already has one). Expected: 9 scenarios (4 in `basic.feature`, 3 outline rows, 2 in `slow.feature`); a full run gives 8 passed, 1 failed. The builder verifies that `@ignore` does not stop `callSingle` or `karate.start` from using those files.
- The **broken sample project** is the clean one plus `broken/syntax.feature`.

How they are checked:
- Every scenario below is automated with Playwright against the built app (`_electron.launch`, per ADR 0004), each test on a fresh copy of its fixture project.
- Playwright cannot drive the packaged exe, so a short manual check on the packaged zip covers what only packaging can break: on a Windows 11 machine without Java, unzip, start, open the clean sample project, Run all, Cancel a run, open the report.

```gherkin
Feature: Walking skeleton

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  Scenario: Open a project and see its tests
    When I open the clean sample project
    Then within 2 seconds the tree shows the "tests" folder
    And it contains "basic.feature", "outline.feature" and "slow.feature"
    And "outline.feature" contains the outline "add <a> and <b>" with 3 rows
    And the called feature "common/create-user.feature" is not shown as a test
    And the toolbar shows the project path and "Standalone"

  Scenario: Maven layout with classpath resources
    Given a Maven project whose features read "classpath:" files from src/test/resources
    When I open it and click Run all
    Then the toolbar shows "Maven"
    And the run finishes with every scenario passed

  Scenario: Syntax error is reported, not hidden
    When I open the broken sample project
    Then the tree area shows Karate's parse error for "broken/syntax.feature"
    And Run all is disabled

  Scenario: Run all with live progress
    Given the clean sample project is open
    When I click Run all
    Then the progress section shows "0 / 9" before any scenario finishes
    And "slow loop" shows as running while it runs
    And scenarios turn passed or failed while the run is still going
    And the run ends "Finished" with 8 passed and 1 failed
    And "basic.feature" shows as failed

  Scenario: Run one outline row
    Given the clean sample project is open
    When I click run on the row "add 2 and 3"
    Then only that row runs and the progress shows "1 / 1"
    And the other rows keep their previous status

  Scenario: Failure details
    Given a finished run where "failing match" failed
    When I click "failing match"
    Then the details pane shows "tests/basic.feature:20" and the match error
    And it lists the scenario's steps with their status
    And expanding the HTTP step shows the request and response text

  Scenario: A feature edited outside the app
    Given the clean sample project is open
    And I insert a new scenario at the top of "basic.feature" in another editor
    When I click run on "failing match"
    Then the tree shows the new scenario without me clicking Refresh
    And "failing match" runs, not the scenario now on its old line

  Scenario: Cancel leaves nothing behind
    Given the clean sample project is open
    When I run "slow.feature" and click Cancel while "slow loop" is running
    Then the run ends "Cancelled"
    And "slow loop" shows as interrupted
    And no java.exe process from karate-kid is left running
    And Report is disabled

  Scenario: Closing the app during a run
    Given a run of "slow.feature" is in progress
    When I close the app
    Then no java.exe process from karate-kid is left running

  Scenario: Nothing to run
    Given an empty folder is open
    Then the tree area says no feature files were found
    And Run all is disabled

  Scenario: Open the HTML report
    Given a run finished
    When I click Report
    Then karate-summary.html opens in the default browser

  Scenario: The project is not modified
    Given the clean sample project is open
    When I run all tests
    Then no file under the project folder was created or changed, except a karate-temp folder
```

Also automated:
- Vitest unit tests for the event-file reader (including a partial last line), tree builder, outline grouping, node keys, row line computation and status logic, fed by the spike's JSONL samples.
- A Vitest test in Node that runs the bundled Java on the clean sample project and checks the dry-run tree (9 scenarios, 3 outline rows) and a full run's counts (8 passed, 1 failed).

The Maven fixture for "Maven layout with classpath resources" is a minimal Maven-layout project: `pom.xml`, `karate-config.js` in `src/test/java`, a feature that does `read('classpath:data/user.json')` from `src/test/resources` and `call read('classpath:common/helper.feature')`.

## Left to the builder

- Component structure, store shape, styling, icons, exact IPC names.
- How to recognise background steps in `FEATURE_EXIT` data, and how to show long step logs.
- Whether to keep the template's oxlint/oxfmt (its oxfmt pulls dev-only audit advisories).
- Exact empty-state and error wording.
