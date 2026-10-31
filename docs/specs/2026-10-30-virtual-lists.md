# Big projects: virtual lists and keyboard navigation

Date 2026-10-30. Status: approved. Covers [#125](https://github.com/hpagniez/grace/issues/125) and [#91](https://github.com/hpagniez/grace/issues/91).

## Intent

Grace should feel the same with 10,000 tests as with ten. Today the test tree and the HTTP list put one DOM row in the page for every test and every call. At 10,000 tests that is over 500k DOM nodes and over 400 MB of JS heap. Opening the project keeps the renderer busy for 3 to 6 s, expanding a folder takes seconds, and a run freezes the window for up to 5 s at a time ([big-project performance note](../research/2026-10-29-big-project-perf/README.md), section 3). Batching run events (#123) and the cheap HTTP tab (#124) fixed 2,000 tests but not 10,000.

The fix is to draw only the rows on screen. Once off-screen rows are not in the page, Tab can no longer reach them. So the keyboard navigation of #91 comes in the same change: each list is one tab stop, and the arrow keys move through every row, drawn or not.

## Who and what success looks like

People who test big Karate projects: hundreds of feature files, thousands of scenarios and outline rows, often run from one top folder. Success: opening, expanding, selecting, scrolling and watching Run all stay smooth at 10,000 tests (R6). A keyboard or screen-reader user can reach and run every test, as they can today.

## Goals, non-goals, anti-goals

**Goals:** the requirements below.

**Non-goals:**
- Projects beyond 10,000 tests. The folder status roll-up ([#126](https://github.com/hpagniez/grace/issues/126)) is the next limit there and stays its own issue.
- Type-to-jump (typing a name to move to it) and `*` to open all siblings.
- Keeping closed folders across a restart or between projects.
- Find in page. Grace has none, so rows that are not drawn hide nothing from it.
- Any change to how a row looks (premium UI R16, R18.5).
- The History list: it holds at most `HISTORY_LIMIT` runs.

**Anti-goals:**
- A test or call that a keyboard user can reach today and cannot reach after.
- Focus falling back to the page when its row scrolls out, is closed away or is hidden by the filter.
- The list jumping while a run updates statuses.
- A screen reader reading wrong levels, positions or counts.
- Rows that flicker, shift by a pixel or lose their hover while scrolling.

## Constraints

- Renderer only: no change to `src/main/**`, the preload or the IPC files.
- No new dependency (approach 1 below). Solid 2 rules and UI conventions from AGENTS.md: tokens, `Icon`, `StatusMark`, `Tooltip`, `Cut`.
- Rows keep a fixed height: 24 px in the tree (R16.1), 28 px in the HTTP list (R18.5). Labels already truncate to one line.
- jsdom has no layout and no `ResizeObserver`: a list that measures 0 px must still draw its first rows, so the unit tests can see them.

### What the app does today

- `src/renderer/Tree.tsx`: nested `<ul role="tree">` / `<li role="treeitem">` / `<ul role="group">`. Each `Node` keeps its own `open` signal (all open at start). A refresh replaces the tree objects, so every folder opens again. Each row holds three buttons in the tab order: the chevron, the label (selects) and the run button (R16.3: "always in the tab order"). Rows use `content-visibility: auto`. Non-matching rows are muted, or removed with "Hide non-matching".
- `src/renderer/HttpCalls.tsx`: one `<button aria-pressed>` per call (`data-testid="http-call"`), in a scrolling `div` (`data-testid="http-calls"`). A click selects the call; the details pane shows it (`state.selectedCall`).
- Selection changes only by a click today (`store.select`, `store.selectCall`). `reduce` keeps the selection across a refresh when the test is unchanged.
- The prototype in [`virtual.diff`](../research/2026-10-29-big-project-perf/virtual.diff) (about 200 lines) draws visible rows only: a flat list of the rows a person could scroll to, closed folders as a `Set` of keys, rows placed absolutely at `index × height`, 10 rows of margin. It has no keyboard, ARIA or test work.

## Approaches considered

1. **Our own helper (chosen by the person).** Grow the prototype into one small piece both lists share: given a row count, a row height, the scroll position and the viewport height, it gives the range of rows to draw. The tree flattens itself into the rows a person can scroll to. No dependency, written for Solid 2, measured at 10,000 tests. Cost: about 300 lines that Grace owns and tests.
2. **`@tanstack/solid-virtual`.** Less own code, but its Solid adapter targets Solid 1 (as testing-library 0.8 did), it needs a decision record, and its main strength, measuring rows of varying height, is not needed here.
3. **CSS only (`content-visibility: auto`, on the tree rows already).** Skips paint but keeps every node: the note measured 527k nodes and 5 s freezes with it. Dropped.

## Requirements

### R1. Only the rows on screen are drawn

1. The tree and the HTTP list each put in the page only the rows inside their visible area, plus a margin above and below, plus the focused row (R4.3).
2. The scroll area is as tall as all the rows together, so the scrollbar's size and position tell the truth, and a drag of the scrollbar lands where it would today.
3. Scrolling, resizing the window or the bottom panel, opening or closing a folder, a filter change and new calls all update which rows are drawn, with no gap or blank band visible at rest.
4. A row that is drawn looks and behaves exactly as today: same height, indent (16 px per level), content, tooltips, hover, selection and focus styles.
5. A tooltip open on a row closes when that row stops being drawn.
6. A list that measures no height (jsdom) draws its first rows as if it had a 600 px viewport.

### R2. The tree as a flat list

1. The tree shows the same rows in the same order as today: a folder's children follow it while it is open; "Hide non-matching" leaves out rows that do not match.
2. Folders, features and outlines start open, as today. Which are closed is remembered by key while the project stays open: a refresh keeps them closed if their key is still there. Opening another project, or reopening Grace, starts with everything open.
3. Status updates during a run never change which rows are drawn or where they sit.

### R3. Keyboard

#### The tree

1. The tree is one tab stop. Tabbing in focuses the row that was last focused; if none or if it is gone, the selected row; otherwise the first row.
2. The chevron, label and run buttons of a row leave the tab order (they still work with the mouse). The run button is reached with Ctrl+Enter (R3.4). This replaces premium UI R16.3's "always in the tab order".
3. Keys on the focused row:

   | Key | Does |
   |---|---|
   | Down / Up | Moves to the next / previous row and selects it |
   | Right | On a closed folder: opens it. On an open folder: moves to its first child and selects it. On a test: nothing |
   | Left | On an open folder: closes it. Otherwise: moves to its parent and selects it. On a top-level row: nothing |
   | Home / End | Moves to the first / last row and selects it |
   | Page Down / Page Up | Moves by one screen of rows (the number of whole rows in view) and selects |
   | Enter, Space | Selects the focused row |
   | Ctrl+Enter | Runs the focused row, as its run button does; nothing when that button is not there or is disabled |

4. A row is "selected" exactly as a click on its label selects it today: the details pane, the HTTP tab and the run button's colour follow (the person's choice: selection follows the arrow keys).
5. A click anywhere on a row makes it the focused row, so the next key moves from there.
6. Keys that do not move (at the first row, Up; at the last, Down) do nothing and do not scroll the page.

#### The HTTP list

7. The HTTP list is one tab stop, after the status group chips. Tabbing in focuses the last focused call; if none or if it is gone, the selected call; otherwise the first call.
8. Down, Up, Home, End, Page Down and Page Up move between calls and select the call moved to, which opens it in the details pane. Enter and Space select the focused call.
9. A click on a call makes it the focused call.

### R4. Focus and scrolling

1. A key that moves focus scrolls the list just enough to show the new row (nothing when it is fully in view).
2. When the focused row is scrolled out with the mouse wheel or the scrollbar, the next key first brings the focused row back into view, then moves from it.
3. The focused row stays in the page while it is scrolled out, so focus never drops to the page.
4. Closing a folder (by click or Left) that holds the focused row moves focus to the folder. The selection stays where it was, as today.
5. When "Hide non-matching" hides the focused row, focus moves to the nearest row above it that is still shown (the first row if none). The selection stays where it was, as today.
6. When rows are added or removed above the view, the rows in view stay where they are on screen, as the browser's scroll anchoring keeps them today. This covers calls that arrive above the view during a run (calls are sorted by start time, so with several threads a slow feature's calls land above shown ones) and a new tree, from Refresh or from the discovery main runs before every run. At the very top of a list, the list stays at the top.
7. After a filter change or a change of HTTP group chips, which the person makes, the selected row is scrolled into view if it is still shown (R4.1's minimum scroll). A new tree never scrolls to the selection: the discovery before each run sends one, and a jump at each run start is what the anti-goals rule out.
8. Status updates and the arrival of the details never scroll a list.
9. A focused row has the inset focus ring (R10.1) and is never hidden under a sticky header (R10.2).
10. When a tree row gets focus from the keyboard and its label is cut, the label's tooltip opens over it, as on hover, and closes when focus moves (premium UI R9.4 and R10.5: a keyboard user reads a cut name today by tabbing to it). HTTP rows show no tooltip on focus, as today.

### R5. Screen readers

1. The tree is `role="tree"` with the accessible name "Tests". Each row is a `role="treeitem"` with `aria-level` (1 at the top), `aria-setsize` and `aria-posinset` among its shown siblings, `aria-expanded` on folders, features and outlines, and `aria-selected`.
2. A tree row's accessible name is its label and its status word, then what its flaky chip says ("flaky" on a test, "2 flaky" on a folder or feature holding two), then its tags as its chips show them, for example "login.feature, failed, 2 flaky, smoke".
6. Tree rows carry `aria-keyshortcuts="Control+Enter"`, and the run button's tooltip reads "Run <name> (Ctrl+Enter)", so the shortcut can be found by ear and by mouse.
3. The HTTP list is `role="listbox"` with the accessible name "HTTP calls". Each call is a `role="option"` with `aria-selected`, `aria-setsize` (the calls shown) and `aria-posinset`.
4. A call's accessible name is its status, method, URL and test, in that order, as the row reads.
5. Focus sits on the row itself (roving `tabindex`: 0 on the focused row, −1 on the others), so a screen reader announces the row on each move.

### R6. Speed at 10,000 tests

Measured on the packaged app with the performance note's harness and generated project (25 features per module: 10,000 tests, 3,000 HTTP calls), window 1400 × 900, on the developer machine. Each figure is the median of 3 rounds; the note's prototype figures are in brackets.

1. Open the project: longest renderer task under 500 ms (290 ms); DOM nodes after open under 10,000 (3.7k).
2. Close then open the top folder (12,049 rows): under 100 ms each (49–52 ms).
3. Select a test with the mouse, or move with Down: under 50 ms (11 ms).
4. Run all, Console tab or HTTP tab open: "Finished" within 2 s of Karate's last test (0.2–1.2 s); no frame longer than 500 ms (342–383 ms, the rest is #126).
5. Open the HTTP tab with 3,000 calls: under 100 ms (23 ms).
6. Scroll the tree during Run all: no frame longer than 100 ms (25–33 ms).
7. At 2,000 tests nothing is slower than on `develop` before this change (note section 2, B–E column).

## Design decisions already made

By the person:
- Keyboard navigation (#91) in the same change.
- Selection follows the arrow keys, in the tree and the HTTP list.
- 10,000 tests is the size to stay smooth at.
- Approach 1, our own helper.

The spec author's calls, for the person to check:
- Ctrl+Enter runs the focused row. Plain Enter would start runs by accident; a letter would clash with a later type-to-jump.
- Page Up and Page Down, which #91 did not list, because at 10,000 rows Home and End are not enough.
- Closed folders survive a refresh but not a restart or another project.
- A row's accessible name (R5.2, R5.4), and the tree and list names "Tests" and "HTTP calls".
- Where focus goes when its row is closed away or hidden (R4.4, R4.5).
- Rows added above the view keep the rows in view still (R4.6), and only the person's own filter and chip changes scroll to the selection (R4.7). The renderer cannot tell Refresh from the discovery before a run, so a new tree never scrolls.
- The label's tooltip on keyboard focus (R4.10), and the shortcut named on the run button's tooltip and in `aria-keyshortcuts` (R5.6).
- The speed limits of R6: about twice the prototype's figures, so they hold on a busier machine.

Raised by the builder check (2026-10-30) and settled above as the spec author's calls: the discovery before each run (R4.6, R4.7), calls arriving above the view (R4.6), tooltips from the keyboard (R4.10), and the flaky words in a row's name (R5.2).

## Left to the builder

- The helper's name, file and shape, and how the tree's flat rows are computed and memoized.
- The margin of extra rows (the prototype used 10).
- How Tab-in finds "last focused" (R3.1, R3.7), and whether the focused row is kept by key or by position.
- The `data-testid`s of the list containers and any new attributes the e2e tests need, for example the total row count.
- How the unit tests drive scrolling in jsdom.
- Whether the generated project for the e2e tests is written by the test (from `generate.mjs`) or checked in.
- Splitting the build into PRs (for example: the helper and the HTTP list, then the tree).

Tests known to change: `e2e/look.spec.ts` counts more than 30 tab stops (51 today) and will see far fewer; `e2e/http.spec.ts` and the unit tests check `aria-pressed` on calls, which becomes `aria-selected`; tests that select through the nested `li` structure; the performance note's harness, which uses both.

## Acceptance criteria

Scenarios run in Vitest (jsdom) unless marked **(e2e)**, which run in Playwright against the built app, or **(perf)**, which run by hand with the performance note's harness and are written down in a check record under `docs/checks/`.

```gherkin
Feature: Big projects: virtual lists and keyboard navigation

  # R1, R2: drawing
  Scenario: Only the rows in view are drawn (e2e)
    Given a generated project of 2,000 tests is open
    Then the tree holds fewer than 100 rows in the page
    And the tree's scroll area is as tall as all its rows
    When I scroll the tree to the bottom
    Then the last test of the project is drawn

  Scenario: A small list draws in jsdom
    Given a project of 30 tests is open in jsdom
    Then the first 25 tree rows are in the page

  Scenario: The tree shows the same rows as before
    Given a project with a folder, a feature and an outline
    Then the rows, their order and their indents are as they are today
    When I turn on "Hide non-matching" with a tag that matches one scenario
    Then only that scenario and its parents are rows

  Scenario: Closed folders survive a refresh
    Given I closed the feature "basic.feature"
    When I refresh the project
    Then "basic.feature" is still closed
    When I open another project and come back
    Then "basic.feature" is open

  Scenario: Statuses do not move rows
    Given the tree is scrolled half way
    When a run updates the statuses of 500 tests
    Then the scroll position and the drawn rows are unchanged

  # R3: tree keys
  Scenario: The tree is one tab stop
    Given a project is open and nothing is selected
    When I press Tab until the tree has focus
    Then the first row is focused
    When I press Tab
    Then focus has left the tree

  Scenario: Arrow keys move and select
    Given the first row of the tree is focused
    When I press Down
    Then the second row is focused and selected
    And the details pane shows the second row

  Scenario Outline: Right and Left
    Given the focused row is <row>
    When I press <key>
    Then <result>
    Examples:
      | row                        | key   | result                                          |
      | a closed feature           | Right | the feature opens and focus stays on it         |
      | an open feature            | Right | its first scenario is focused and selected      |
      | an open feature            | Left  | the feature closes and focus stays on it        |
      | a scenario                 | Left  | its feature is focused and selected             |
      | a top-level closed folder  | Left  | nothing changes                                 |

  Scenario: Home, End and pages
    Given a project of 2,000 tests is open with the first row focused
    When I press End
    Then the last row is focused, selected and in view
    When I press Page Up
    Then the row one screen above is focused
    When I press Home
    Then the first row is focused and in view

  Scenario: Ctrl+Enter runs the focused row
    Given the scenario "passing hello" is focused
    When I press Ctrl+Enter
    Then a run of "passing hello" starts
    When I press Ctrl+Enter during the run
    Then no second run starts

  Scenario: The row's buttons left the tab order but still click
    Then no chevron, label or run button in the tree is in the tab order
    When I click the run button of "passing hello"
    Then a run of "passing hello" starts

  Scenario: A click makes the row focused
    Given the first row is focused
    When I click the label of the fifth row
    And I press Down
    Then the sixth row is focused and selected

  # R3: HTTP keys
  Scenario: The HTTP list is one tab stop and arrows open calls
    Given a run made 3 HTTP calls and the HTTP tab is open
    When I press Tab until the HTTP list has focus
    Then the first call is focused
    When I press Down
    Then the second call is focused and selected
    And the details pane shows the second call

  # R4: focus and scrolling
  Scenario: A moved-to row scrolls into view (e2e)
    Given a project of 2,000 tests is open with the first row focused
    When I hold Down for 40 rows
    Then the focused row is in view

  Scenario: Focus survives scrolling away (e2e)
    Given a project of 2,000 tests is open with the tenth row focused
    When I scroll the tree to the bottom with the wheel
    Then the tenth row is still in the page and still focused
    When I press Down
    Then the tree scrolls back and the eleventh row is focused

  Scenario: Closing a folder takes focus to it
    Given the scenario "passing hello" in "basic.feature" is focused and selected
    When I click the chevron of "basic.feature"
    Then "basic.feature" is focused
    And "passing hello" is still selected

  Scenario: Hiding the focused row
    Given a scenario that does not match the tag "@smoke" is focused
    When I turn on "Hide non-matching" with "@smoke"
    Then the nearest shown row above it is focused

  Scenario: A new tree keeps the rows in view still (e2e)
    Given a project of 2,000 tests is open, scrolled so its 1,500th row is the top row in view
    And its 10th row is selected
    When I run the 1,500th row with its run button
    Then the 1,500th row is still the top row in view when the run starts

  Scenario: Calls arriving above the view keep the rows in view still
    Given the HTTP list is scrolled so the call at 2 s is the top row in view
    When a call that started at 1 s arrives
    Then the call at 2 s is still the top row in view, at the same place on screen

  Scenario: A filter change scrolls to the selected row (e2e)
    Given a project of 2,000 tests is open with its 1,500th row selected and scrolled out of view
    When I turn on "Hide non-matching" with a tag that the selected row has
    Then the selected row is in view

  Scenario: A cut label shows whole on keyboard focus
    Given a tree row whose label is cut
    When I move to it with Down
    Then its label's tooltip shows the whole label over the row
    When I press Down again
    Then that tooltip closes

  Scenario: The run shortcut can be found
    Then every tree row has aria-keyshortcuts "Control+Enter"
    And the run button of "passing hello" has the tooltip "Run passing hello (Ctrl+Enter)"

  # R5: screen readers
  Scenario: Tree rows carry their level and position
    Given a project with one folder holding two features, the second with three scenarios
    Then the second feature is a treeitem at level 2, position 2 of 2, expanded
    And its third scenario is a treeitem at level 3, position 3 of 3
    And the tree is named "Tests"

  Scenario: A row's name says its status, flakiness and tags
    Given the feature "login.feature" shows the tag chip "smoke" and failed
    And two of its scenarios are marked flaky, one of them "login works", which failed and shows no tag chip
    Then the feature's treeitem is named "login.feature, failed, 2 flaky, smoke"
    And the scenario's treeitem is named "login works, failed, flaky"

  Scenario: Calls are options
    Given a run made 3 HTTP calls and the HTTP tab is open
    Then the HTTP list is a listbox named "HTTP calls"
    And each call is an option with its position of 3
    And the selected call is the only one with aria-selected true

  # R6: speed
  Scenario: Opening 10,000 tests (perf)
    When I open the generated 10,000-test project
    Then the longest renderer task is under 500 ms
    And the page holds fewer than 10,000 DOM nodes

  Scenario: Folders, selection and the HTTP tab at 10,000 tests (perf)
    Given the generated 10,000-test project is open and has run once
    Then closing and opening the top folder each take under 100 ms
    And selecting a test takes under 50 ms
    And opening the HTTP tab takes under 100 ms

  Scenario: Run all at 10,000 tests (perf)
    Given the generated 10,000-test project is open
    When I run all, once with the Console tab and once with the HTTP tab open
    Then "Finished" shows within 2 s of Karate's last test
    And no frame takes longer than 500 ms
    And while I scroll the tree during the run no frame takes longer than 100 ms

  Scenario: 2,000 tests are not slower (perf)
    Given the generated 2,000-test project
    Then every figure of the note's section 2 is no worse than its B–E column
```
