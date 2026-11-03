# Open the test tree with only its folders open

Date 2026-11-03. Status: draft. Covers [#224](https://github.com/hpagniez/grace/issues/224). Replaces R2.2 of [Big projects: virtual lists and keyboard navigation](2026-10-30-virtual-lists.md) and narrows its R2.3.

## Intent

The left tree starts with every folder, feature and outline open, so a real suite lists every scenario and every example row of every file, and the feature files themselves are hard to find. People find their way by file. The tree should start as a list of folders and feature files, and show a file's tests when the person asks for them: by clicking the file, or by running it. In the owner's words: "only have folder open, and have only the list of files, not all the steps opened by default on the left panel, it opens only when the file is clicked or when the test is run".

## Goals, non-goals, anti-goals

- Goals: folders start open and features start closed; a click on a feature's name opens it; a run of one feature or of a test in it opens that feature; a wider run opens the features with a failed test; a refresh keeps what the person opened or closed.
- Non-goals: an "open all" or "close all" command; remembering open features across a restart or between projects (virtual-lists keeps that out of scope too); a setting for the old behaviour; changing how folders, outlines or tests behave on a click.
- Anti-goals: walking the tree with the arrow keys opening every feature on the way; a Run all that ends with the whole tree open again; rows that move on every status update during a run; any cost while idle.

## Constraints

- The row list (`rows` in `src/renderer/Tree.tsx`) reads no status, so a status update never rebuilds it (virtual-lists R2.3, R6).
- Arrow keys, Home, End, Page Up and Page Down select the row they move to (virtual-lists R3), so selecting a row must not open it.
- Idle costs nothing (ADR 0023): no timer, poll or effect that runs while no run is going.
- No backward compatibility: nothing of the tree's open state is saved, so nothing migrates.

### What exists today

- `Tree.tsx` keeps a `closed` signal: a set of keys, empty when a project opens (everything open), reset when the project's root changes, kept across a refresh. The chevron button, Right and Left toggle a key in it. `treeRows` (`src/renderer/treeRows.ts`) walks the tree and skips the children of closed keys.
- A click on a row's name calls `select(key)`; the chevron opens and closes. Enter and Space on a row select it, through the same path as the arrow keys (`VirtualList`'s `onSelect`).
- A test row is drawn only while its feature is open, so a run started from a test row in the tree starts inside an open feature. A run can still start from a test or feature whose feature is closed: History's Re-run starts from the run's stored selection, which can be a feature, an outline or a test.
- A debugger pause opens every row above the paused test (step-debugger R7.1), from an effect on `state.paused`.
- `VirtualList` keeps the rows in view where they sit when rows above them come or go (virtual-lists R4.6, `anchoredScrollTop`).
- Main's `runStarted` event carries `selection`, the key the run started from (the tree's root for Run all and for a rerun of the failed tests, whose selection is null), and `keys`, the run's tests. The reducer keeps neither; it sets each run key's status to `pending`. `run.failed` counts the tests that failed in the run, and nothing lists which.
- Keys say where a node sits (`src/main/karate/tree.ts`): a feature is `f:<file>`, a scenario or example row `s:<file>#<n>` or `s:<file>#<n>.<m>`, an outline `o:<file>#<n>`, a folder `d:<dir>`.

## Approaches considered

1. **Keep the open state in the tree, where it lives today. Chosen by the owner.** The tree remembers which rows the person turned from their starting state, and opens features itself when a run starts and when a test fails. The reducer only keeps the run's starting key, which main already sends, and the keys of the tests that failed in the run. One component changes, plus two fields in the run's view.
2. **Move the open state into the app's shared state.** The reducer would open features as run events arrive. Easier to test as a pure function, but the tree's own state would move into the whole app and every component's state, which touches more code for no gain to the person.

## Requirements

### R1. How the tree starts

1. When a project opens, folders start open and features start closed: the tree shows the folders and their feature files only. This replaces the first sentence of virtual-lists R2.2.
2. Outlines start open: a feature that opens shows every test in it, its outlines' rows included.
3. Which rows the person opened or closed is remembered by key while the project stays open. A refresh keeps a row as it was if its key is still there; a feature new in the refresh starts closed. Opening another project, or reopening Grace, starts again as in R1.1. This replaces the rest of virtual-lists R2.2.
4. "Hide non-matching" leaves out the rows that do not match, as today; the features it keeps start closed too.

### R2. Opening a feature by hand

1. A click on a feature's name selects it and opens it if it is closed. It never closes it.
2. The chevron, Right and Left open and close a feature as today.
3. The keys that select never open a feature: arrow keys, Home, End, Page Up, Page Down, Enter and Space.
4. A click on the name of a folder, an outline or a test only selects it, as today.

### R3. A run opens features

1. A run started from a feature, or from an outline, scenario or example row inside one, opens that feature as it starts. This holds for a debug run, and for every way of starting one: the run and debug buttons, Ctrl+Enter, Ctrl+Shift+Enter, and History's Re-run of such a run.
2. A run started from a folder, Run all, or a rerun of the failed tests opens nothing as it starts.
3. During any run, a feature opens when one of its tests fails in that run.
4. A run opens only features: an outline the person closed stays closed and shows the failed mark, and a folder the person closed stays closed, with the feature open inside it.
5. A run never reopens a feature the person closed while it was going, whatever fails in it later in the run: one the run opened, one open before the run, or one opened by hand or by a pause during it.
6. A feature a run opened stays open after the run, like one the person opened, until the person closes it. A refresh keeps it open (R1.3).
7. A debugger pause still opens every row above the paused test, folders included (step-debugger R7.1).

### R4. Rows during a run

This narrows virtual-lists R2.3.

1. Status updates change which rows are shown only through R3: when a run starts, and when a feature's first test fails in the run. Any other status update leaves the rows and where they sit unchanged.
2. When a feature opens above the rows in view, the rows in view stay where they are on screen (virtual-lists R4.6).
3. The row list still reads no status: a run changes it only by opening a feature, at most once per feature.

## Cost

- Idle: no new timer, poll or listener. A project opens with fewer rows: at 2,000 tests the tree lists 249 rows instead of 2,449, and at 10,000 tests 1,049 instead of 12,049, so building the row list on open, on a filter change and on every open or close walks less of the tree. The rows drawn stay the ones in view, so the DOM is the same size.
- A run: as it starts, the tree reads the feature of the run's starting key from the key itself. Each time tests fail, the tree reads the features of the newly failed tests the same way: work in proportion to the new failures, never a walk of the tree. Each feature that opens rebuilds the row list once. At 10,000 tests with 1 test in 50 failing, that is at most 200 rebuilds over a run of several minutes, against none today.
- Memory: one set of keys the person turned from their start; per run, the keys of the tests that failed in it and a set of the features the run may no longer open: a few kilobytes, a few hundred keys at 10,000 tests with 1 in 50 failing.
- Start time: unchanged, or slightly faster for a big project (a shorter row list to build). Size: under 1 KB of code; no dependency.

## Design decisions already made

By the owner:

- A click on a feature's name selects it and opens it if it is closed, and never closes it (R2.1).
- A run of one feature, or one of its tests, opens that feature when it starts; a folder run, Run all or a rerun of the failed tests opens only the features with a failed test (R3.1 to R3.3).
- A feature a run opened stays open after the run (R3.6).
- The open state stays in the tree (approach 1).

The builder's calls, open to the owner's correction:

- Outlines start open (R1.2): one click shows a file's every test, and an outline is rarely long.
- A run opens the feature only, never an outline or a folder the person closed (R3.4): only the person closes those, so the tree keeps their choice. A pause still opens everything above the paused test, since the pause selects it and must show it.
- A run never reopens a feature the person closed during it (R3.5): a run never fights the person, and rows change at most once per feature per run.
- The keys that select never open a feature (R2.3), Enter and Space included; Right still does.
- History's Re-run of a feature or test run opens that feature (R3.1): it is a run of one feature, started another way.
- A failure opens its feature as it happens, not at the end of the run: the view stays still (R4.2), and the person sees failures while the run goes on.

## Left to the builder

- How the run's view keeps the run's starting key (a field filled from `runStarted.selection`), and how the tree learns which tests failed since it last looked (for example the run's failed keys, in order, filled by the reducer's `scenarios` case and read from where the tree stopped).
- The feature of a key comes from the key, as main builds it: `f:` and the file part of an `s:` or `o:` key, before its last `#`. Not from `node.file`, and not from a walk of the tree.
- How the tree stores its open state: for example one set of keys turned from their starting state, a feature's start being closed and every other row's open.

### Tests and the bench

- Unit tests in `src/renderer/Tree.test.tsx` cover R1 to R4. Unit tests of other components that click a test row in the tree open its feature first.
- e2e specs that click a scenario, outline or example row open its feature first, through a shared helper in `e2e/helpers.ts`. One e2e test covers R1.1, R2.1 and R3.1 on the packaged app.
- `e2e/virtual.spec.ts` measures the tree as it now opens: a list of folders and features (249 rows at 2,000 tests), its last row a feature. A test that needs test rows opens their feature through the UI first.
- The screenshot scripts (`e2e/readme.shots.ts`, `e2e/screens.shots.ts`) open the features they show before they select a test in them.
- The bench measures the tree as people now get it:
  - It waits for the tree to show its folders and features, not every test. `writeBigProject` gives that row count with the test count.
  - Before the Smooth probes it opens the first feature by a click on its name, so "select a test" still selects a test. A name click never closes (R2.1), so the same script drives the base build of a PR, whose features start open.
  - "Expand" closes and opens the top folder as today, which now draws folders and features.
  - Its figures then differ from the base's for reasons this spec gives; the PR says so. Limits are not raised.

## Acceptance criteria

```gherkin
Feature: The tree opens with only its folders open

  Background:
    Given a project with a folder "tests" holding "basic.feature" with 2 scenarios
    And "outline.feature" with an outline of 2 rows

  # R1
  Scenario: A project opens with its features closed
    When I open the project
    Then the tree shows "tests", "basic.feature" and "outline.feature"
    And it shows no scenario, outline or example row
    And "tests" is open and both features are closed

  Scenario: A refresh keeps what I opened and closed
    Given I opened "basic.feature" and closed "tests"
    When I refresh the project
    Then "basic.feature" is open and "tests" is closed

  Scenario: A feature new in a refresh starts closed
    When a feature file "new.feature" is added and I refresh the project
    Then "new.feature" is closed

  Scenario: Another project starts closed again
    Given I opened "basic.feature"
    When I open another project and come back
    Then "basic.feature" is closed

  # R2
  Scenario: A click on a closed feature's name opens and selects it
    When I click the name "basic.feature"
    Then "basic.feature" is selected and open
    And its 2 scenarios are shown

  Scenario: A click on an open feature's name keeps it open
    Given "basic.feature" is open
    When I click the name "basic.feature"
    Then "basic.feature" is selected and still open

  Scenario: The chevron still closes a feature
    Given "basic.feature" is open
    When I click its chevron
    Then "basic.feature" is closed

  Scenario: Moving with the keys does not open features
    Given the tree has focus on "tests"
    When I press Down twice
    Then "outline.feature" is selected
    And both features are still closed
    When I press Enter
    Then "outline.feature" is still closed
    When I press Right
    Then "outline.feature" is open

  Scenario: An outline opens with its feature
    When I click the name "outline.feature"
    Then the outline and its 2 rows are shown

  # R3
  Scenario Outline: A run of one feature opens it as it starts
    Given both features are closed
    When I start <run>
    Then "basic.feature" opens as the run starts
    And "outline.feature" stays closed
    Examples:
      | run                                                        |
      | a run of "basic.feature" from its run button               |
      | a debug run of "basic.feature" from its debug button       |
      | a run of "basic.feature" with Ctrl+Enter on its row        |
      | History's Re-run of a run of the scenario "passing hello"  |

  Scenario: Run all opens only the features with a failure
    Given "basic.feature" has a failing scenario and "outline.feature" passes
    When I click Run all
    Then "basic.feature" opens when its scenario fails
    And "outline.feature" stays closed, with a passed mark

  Scenario Outline: A run never reopens a feature I closed during it
    Given "basic.feature" is <before> and Run all is going
    And "basic.feature" is open, <how>
    When I close "basic.feature"
    And a scenario of "basic.feature" fails
    Then "basic.feature" stays closed
    Examples:
      | before | how                                  |
      | closed | as its first scenario failed         |
      | open   | as it was before the run             |
      | closed | as I clicked its name during the run |

  Scenario: A run opens a feature inside a closed folder, and leaves the folder closed
    Given I closed "tests"
    When Run all fails a scenario of "basic.feature"
    Then "tests" is still closed
    When I open "tests"
    Then "basic.feature" is open

  Scenario: A failure leaves a closed outline closed
    Given "outline.feature" is open and I closed its outline
    When Run all fails one of the outline's rows
    Then the outline stays closed with a failed mark

  Scenario: What a run opened stays open
    Given a run opened "basic.feature"
    When the run ends
    Then "basic.feature" is still open
    When I refresh the project
    Then "basic.feature" is still open

  Scenario: A pause still opens the way to the paused test
    Given both features are closed
    When a debug run of "tests" pauses in "outline.feature"'s second row
    Then "outline.feature" and its outline are open and the row is selected

  # R4
  Scenario: Statuses do not move rows
    Given the tree is scrolled half way through a project of 2,000 tests
    When a run passes 500 tests
    Then the scroll position and the drawn rows are unchanged

  Scenario: A feature opening above the view does not move it
    Given the tree is scrolled so that "basic.feature" is above the rows in view
    When Run all fails a scenario of "basic.feature"
    Then the rows in view stay where they were on screen
```
