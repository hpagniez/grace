# Say once when the config fails, and group failures that share an error

Date 2026-11-04. Status: approved. Covers [#229](https://github.com/hpagniez/grace/issues/229).

## Intent

When many tests fail for one reason, Grace shows the reason once per test, and the person has to open the tests one by one to see that they all say the same thing. A failed `karate-config.js` is the worst case: it fails every test of the run before its first step. On a real Maven suite, a missing environment, a missing secret or no VPN made all 24 scenarios fail with one message, and nothing said the cause was the config, not the tests.

Grace should say a config failure once, after the run, with Karate's message and a link to the file and line. For any run, Grace should show which failures share an error, and let the person step through them.

## Who and what success looks like

Someone running a whole suite who gets a wall of red. Success: they see at once that the config failed and jump to the line, or they see "24 tests failed with this error" on the first failure they open and step through the others without hunting in the tree.

## Goals, non-goals, anti-goals

- Goals: a test whose config failed shows as failed, with Karate's error; a banner after the run when the config failed, with a link to the file and line; in a failed test's details, how many tests share its error, with Previous and Next; in a folder's details, its failures grouped by error, each with Go to first.
- Non-goals: a tree filter that shows only one group's tests (stepping covers it, the owner's choice); masking numbers or ids to group near-identical errors; groups in the history of an earlier run; keyboard shortcuts for Previous and Next; a link to the config file from the test's own error box.
- Anti-goals: any cost while idle; any grouping work while a run goes on; two errors merged that differ in a status code, a value or a path.

## Constraints

- Follow the Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)). Karate already splits a failure's message from its place: `SCENARIO_EXIT` carries `error` (the place `C:\…\x.feature:9`, the step's comment label if any, the step's text, then the message) and `errorReason`, the message alone (Karate's `ScenarioResult.getFailureReason()`). Grace groups on Karate's `errorReason`, not on its own parsing of `error`.
- When `karate-base.js`, `karate-config.js` or `karate-config-<env>.js` fails, Karate 2.1.4 sends no `SCENARIO_ENTER` or `SCENARIO_EXIT` for any scenario: each one gets a single synthetic failed step in the depth-0 `FEATURE_EXIT` (`ScenarioRuntime.call()`, "a config-error result (below, no ENTER)"). That step has a `prefix` (`*`) and a `text`, but no `keyword`; its `errorMessage`, and the scenario's `error` in `scenarioResults`, read `Config evaluation failed: <file> - <message>`. `<file>` is the config file's path relative to its classpath root (`karate-config.js`), or its prefixed path when it has none. `<message>` is Karate's JS error. A `karate.fail(…)` or a reference to an undefined name gives `js failed:` and `Line: <n>, Col: <m>`, where `<n>` is the line in the file; a `throw new Error('boom')` gives `Error: boom` with no line; a syntax error gives `<n>:<m>` on a line of its own (checked with Karate 2.1.4 in a scratch folder). `test/fixtures/events/run-http-config-fail.jsonl` records a real one: a `callSingle` in `karate-config.js` line 4 that failed, for two scenarios of `tests/shared.feature`.
- Open in VS Code exists ([open-in-editor](2026-11-03-open-in-editor.md)): `api.openInEditor(path, line)`. Main resolves a relative path against the project root, then its classpath roots, and opens only files inside the project.
- Performance rules (`AGENTS.md`): idle costs nothing.

### What exists today

- `RunState` (`src/main/karate/run-state.ts`) sets a test's status only from depth-0 `SCENARIO_ENTER` and `SCENARIO_EXIT`. From a depth-0 `FEATURE_EXIT` it sends only steps and HTTP calls. So when the config fails, every test of the run ends "Not run", the notice says "Karate ran 0 of 24 selected tests", and Karate's own summary says 24 failed.
- `extractSteps` (`src/main/karate/steps.ts`) writes a step's text as `${keyword} ${text}`: the synthetic step shows as `* undefined Scenario execution failed: …`.
- The window keeps each test's `error` in `state.results[key]`, and the last run's failed keys in `state.run.failedKeys`. A failed test's details show the error in a red box (`details-error`). A folder's details show its status counts.
- The notice under the tag bar shows one text from main at a time (`Notice`).
- The tree opens the ancestors of a paused test and scrolls to it (step debugger R7.1); nothing else selects a test the tree may not show.

## Approaches considered

1. **Group in the window. Chosen by the owner.** Main sends Karate's reason with each failed test (R2). The window groups the failed tests the tree shows once a run ends, and finds the config banner in the same groups. No new event; nothing runs while idle or during a run.
2. **Group in main.** At the end of a run, main groups the failures and sends the groups and the config failure as a new event. Same work, one more event type, and the groups go stale when the person reruns one test unless main sends them again.
3. **Dropped: a tree filter for a group.** A third button would show only the group's tests in the tree. It needs its own chip to clear it and rules against the tag filter and "Hide non-matching"; stepping covers the need (the owner's choice).

## Requirements

### R1. A test whose config failed is failed

1. When a depth-0 `FEATURE_EXIT` lists a scenario with `failed: true`, and this run has given its test no passed or failed status, Grace marks the test failed, with that scenario's `error` and `durationMillis`.
2. Such a test counts as any failed test: in the progress, in the run's history entry and so in flaky tests (F15), in the tree opening its feature (#224 R3), and not in the notice "Karate ran X of N selected tests".
3. A step that has no `keyword` shows its `text` alone, after its `prefix`, as any step: `* Scenario execution failed: Config evaluation failed: …`.
4. A step whose line is below 1 (Karate's synthetic step has line 0) has no breakpoint dot: main refuses a breakpoint there.

### R2. Karate's reason

1. With a failed test, main sends where Karate's reason starts in its `error`: from `SCENARIO_EXIT`, the length of `error` less that of `errorReason` when `error` ends with `errorReason`; for a test of R1, 0. Otherwise it sends none, and the whole error is the reason.
2. Two failed tests share an error when their reasons are the same text, character for character, once Karate's position in a JS failure is left out: a line `Line: <n>, Col: <m>` (with its leading spaces) and a last line `at <n>:<m>` (a `throw` in a function). That position is the place: `karate.fail('no vpn')` at line 10 of one feature and at line 7 of another share an error.

### R3. Groups

1. A group is 2 or more failed tests that share an error, among the tests the tree shows: every test, or, with "Hide non-matching" on and a tag filter in force, the tests it matches. A group's tests are in tree order.
2. The window finds the groups when a run ends, and again when the tree, the tag filter or the statuses change while no run goes on. While a run goes on, no group shows and none is computed.
3. A group holds tests of any run, not only the last: a test failed in an earlier run and not run since still counts.

### R4. Step through a group from a failed test

1. When the selected test is failed and in a group, its error box shows a line under the error: "3 of 24 tests failed with this error", then a Previous and a Next button (ghost, small, icon only: `ChevronUp`, `ChevronDown`), named "Previous test with this error" and "Next test with this error", each with its name as a tooltip.
2. Next selects the group's next test in tree order, Previous the one before. Both wrap round: Next on the last selects the first.
3. The tree opens the folders, feature and outline above the selected test and scrolls to it, as it does for a paused test.
4. The focus stays on the pressed button, so Enter steps again.

### R5. A folder's failures by error

1. The details of the project root, a folder, a feature or an outline show, under its counts, a heading "Failed with the same error" and the groups of its own failed tests (2 or more sharing an error), largest first; groups of the same size keep the tree order of their first tests.
2. Each group shows its count ("24 tests"), the reason of its first test as Karate wrote it in monospace, cut after 4 lines, and a small ghost button "Go to first" that selects the group's first test in tree order, as R4.3. Its accessible name adds the count: "Go to first of 24 tests".
3. A folder with no group shows neither the heading nor a list.
4. The count in a folder counts only its own tests; the "3 of 24" of R4.1 counts the whole tree.

### R6. Say once that the config failed

1. When no run goes on and one or more of the last run's failed tests have a reason that starts with `Config evaluation failed: `, a banner shows under the tag bar, where notices show, and under the notice when both show. It has the error tint and the `CircleAlert` icon, and is an alert.
2. Its first line reads "`<file>` failed for 24 tests": `<file>` is the text between `Config evaluation failed: ` and the next ` - `, without a `classpath:` prefix, and the count is the last run's failed tests with such a reason. A small ghost button "Open in VS Code" with the `ExternalLink` icon follows, named "Open `<file>` in VS Code".
3. Under it, Karate's reason as written, in monospace, in a box that scrolls past 15 lines.
4. The button asks main to open `<file>` at the number after the first `Line: ` in the reason, or at line 1 when there is none (a thrown `Error`, a syntax error's `<n>:<m>`). Main resolves and checks it as for any Open in VS Code (open-in-editor R3, R4).
5. When those tests have different reasons, the banner names the file and shows the reason of the first of them in tree order; the count covers them all. Details shows each reason as its own group.
6. The banner goes when the next run starts, another project opens, or the environment changes (which clears the results). It shows on the project view only, not on the Settings page.

## Cost

- Idle: nothing. No timer, poll or listener is added; the groups and the banner are derived from state that changes only when a run or a refresh changes it.
- During a run: main reads one more field of each failed `SCENARIO_EXIT`, and for each scenario of a depth-0 `FEATURE_EXIT` it already walks for steps, one status lookup. The window computes no group.
- When a run ends: one pass over the tree's tests, putting each failed test's reason in a `Map`; one more pass over the last run's failed keys for the banner. Both again only on a refresh, a filter change or an environment change while no run goes on.
- Memory: one number per failed test (where its reason starts); groups hold keys, and a reason is read from the error already kept, not copied.
- Start time, size: no change; under 3 KB of code; no dependency.

## Design decisions already made

- Same error = same message at any place, no masking: the owner's choice. A JS failure's `Line: <n>, Col: <m>` and `at <n>:<m>` are its place and are left out: the owner's choice. Karate's `errorReason` is that message, so Grace takes Karate's split.
- Groups show in Details (the failed test's error box, and the folder's details): the owner's choice.
- Step with Previous and Next, no tree filter: the owner's choice.
- Group in the window: the owner's choice.
- My call: Previous and Next wrap round, as the console's search does.
- My call: only groups of 2 or more show; a lone failure is a failure.
- My call: a group counts every failed test the tree shows, not only the last run's; the banner counts only the last run's, since it reports that run.
- My call: one banner even when the config failures differ; Details shows each reason.
- My call: the reason travels as an offset into the error, so a failed test's message is not kept twice.
- My call: a test whose config failed counts as failed in flaky tests (F15). A run that failed on the config for an outside cause (no VPN), then a good run of the same files, marks those tests flaky, as a test that fails on its own for an outside cause does today.
- My call: the banner's line comes from `Line: <n>` only; other positions open line 1.

## Acceptance criteria

Scenarios run in Vitest (jsdom for the window, Node for main) unless marked **(e2e)**, which run in Playwright against the built app with real Karate and `shell.openExternal` replaced by a recorder.

```gherkin
Feature: Say once when the config fails, and group failures that share an error

  # R1
  Scenario: A test whose config failed is failed
    Given the events of "run-http-config-fail.jsonl", where karate-config.js failed for "first" and "second"
    When the run ends
    Then "first" and "second" are failed
    And the error of "first" starts with "Config evaluation failed: karate-config.js - js failed:"
    And no notice says Karate ran fewer tests than selected

  Scenario: A test with its own SCENARIO_EXIT keeps its status
    Given the events of "1-normal-run-failure.jsonl"
    When the run ends
    Then every test has the status its SCENARIO_EXIT gave

  Scenario: Karate's synthetic step shows without "undefined"
    Given the depth-0 FEATURE_EXIT of "run-http-config-fail.jsonl"
    Then the step of "first" reads "Scenario execution failed: Config evaluation failed: karate-config.js - js failed:" at its start
    And its keyword is "*"

  Scenario: Karate's synthetic step has no breakpoint dot
    Given the failed test "first" of a config failure is selected
    Then its step at line 0 shows no breakpoint dot

  # R2
  Scenario: The reason is Karate's errorReason
    Given the events of "1-normal-run-failure.jsonl"
    Then the failed test's reason is "match failed: EQUALS" and the lines Karate wrote under it
    And its error still starts with "C:\spike\project\tests\basic.feature:20"

  Scenario: Two karate.fail calls at different lines share an error
    Given test "a" failed with reason "js failed:
==========
  Line: 10, Col: 0
  Code: karate.fail('no vpn')
  Error: no vpn
=========="
    And test "b" failed with the same reason, but "Line: 7, Col: 0"
    Then "a" and "b" form a group of 2

  Scenario: A thrown string's position is left out
    Given tests "a" and "b" failed with reasons "bad
    at 15:1" and "bad
    at 3:5"
    Then "a" and "b" form a group of 2

  Scenario: A config failure's reason is its whole error
    Given the events of "run-http-config-fail.jsonl"
    Then the reason of "first" is its whole error

  # R3
  Scenario: Failures at different places with one message form a group
    Given tests "a" at "x.feature:5" and "b" at "y.feature:9" failed with reason "expected status: 200, actual: 503"
    And test "c" failed with reason "expected status: 200, actual: 404"
    Then "a" and "b" form a group of 2 in tree order
    And "c" is in no group

  Scenario: A test hidden by the tag filter is left out
    Given "a", "b" and "c" failed with one reason
    And "Hide non-matching" is on and the filter matches only "a" and "b"
    Then the group holds "a" and "b"

  Scenario: No group while a run goes on
    Given "a" and "b" failed with one reason
    When a run starts
    Then the error box of "a" shows no count and no Previous or Next

  # R4
  Scenario: Step through a group
    Given "a", "b" and "c" failed with one reason, in that tree order
    And "a" is selected
    Then its error box says "1 of 3 tests failed with this error"
    When I press "Next test with this error"
    Then "b" is selected
    And its error box says "2 of 3 tests failed with this error"
    And the focus is on "Next test with this error"

  Scenario: Stepping wraps round
    Given "a", "b" and "c" failed with one reason, and "a" is selected
    When I press "Previous test with this error"
    Then "c" is selected

  Scenario: A lone failure has no stepping
    Given "a" failed with a reason no other test has
    When I select "a"
    Then its error box shows no count and no Previous or Next

  # R5
  Scenario: A folder lists its failures by error, largest first
    Given in folder "api", 3 tests failed with "timeout" and 2 with "503"
    And 1 test failed with "404"
    When I select "api"
    Then its details show "Failed with the same error"
    And "3 tests" with "timeout" comes before "2 tests" with "503"
    And "404" is not listed

  Scenario: Go to first
    Given folder "api" lists 3 tests failed with "timeout"
    When I press "Go to first of 3 tests"
    Then the first of them in tree order is selected

  Scenario: A folder without a group shows no list
    Given every failure in folder "api" has its own reason
    When I select "api"
    Then "Failed with the same error" does not show

  # R6
  Scenario: The banner names the config file and its line
    Given the last run's tests "first" and "second" failed with reason "Config evaluation failed: karate-config.js - js failed:\n==========\n  Line: 4, Col: 2 …"
    Then a banner says "karate-config.js failed for 2 tests"
    And shows Karate's reason
    When I press "Open karate-config.js in VS Code"
    Then Grace asks main to open "karate-config.js" at line 4

  Scenario: A config message with no line opens line 1
    Given the last run failed with reason "Config evaluation failed: classpath:karate-config-dev.js - boom"
    Then the banner says "karate-config-dev.js failed for 1 test"
    When I press "Open karate-config-dev.js in VS Code"
    Then Grace asks main to open "karate-config-dev.js" at line 1

  Scenario: The banner goes when the next run starts
    Given the banner shows
    When a run starts
    Then the banner does not show

  Scenario: No banner for ordinary failures
    Given the last run's failures have no reason starting with "Config evaluation failed: "
    Then no banner shows

  Scenario: A broken karate-config.js, end to end (e2e)
    Given a project whose karate-config.js calls karate.fail('no vpn') on line 2, with 2 scenarios in "a.feature" and 1 in "b.feature"
    When I run all
    Then the 3 tests are failed
    And a banner says "karate-config.js failed for 3 tests"
    When I press "Open karate-config.js in VS Code"
    Then VS Code is asked to open "<project>/karate-config.js:2"
    When I select the first test of "a.feature"
    Then its error box says "1 of 3 tests failed with this error"
    When I press "Previous test with this error"
    Then the test of "b.feature" is selected, and the tree shows it
```
