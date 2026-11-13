---
status: built
issue: 71
summary: "Show a selected Scenario Outline Examples as a table in the details pane, with each row status and time, and open a row on click"
---

# Data-driven view: an outline's Examples as a table

Date 2026-11-03. Covers [#71](https://github.com/hpagniez/grace/issues/71).

## Intent

A Scenario Outline runs once per row of its Examples. Today the tree lists the rows as leaves named after the outline's title with the values filled in ("add 1 and 2"), and the details pane shows one node at a time: for an outline, its name, status, location and counts. To see which inputs failed, a person opens rows one by one, and a title that does not name every column hides the values altogether.

When an outline is selected, the details pane shows its Examples as a table: one line per row, with the row's column values and its status and time from the last run. A click on a line opens that row. The owner asked for sensible defaults; every choice below that is not in the issue is the builder's call (see [Calls made](#calls-made)).

## Goals, non-goals, anti-goals

**Goals:** the requirements below.

**Non-goals:**
- Sorting or filtering the table, and an error column. A failed row's error is one click away, in the row's details.
- Showing a row's values in its own details.
- Scrolling the tree to a row opened from the table. The tree's open state is changing in [#224](https://github.com/hpagniez/grace/issues/224); the outline's row stays the tree's selected row's parent, one click away.
- The tag filter muting or hiding lines in the table. The group counts above it ignore the filter too.
- Past runs: the table shows the statuses the window holds, as the tree does.
- A horizontal scroll for wide tables: columns share the width and cut their values (R2.4).

**Anti-goals:**
- Any cost while idle.
- A run that redraws the table's lines: only the status marks and times change.
- Arrow keys that open rows on the way, so the table vanishes under the keyboard.
- Grace reading Examples tables from the feature text where Karate already gives the values.

## Constraints

- Follow the Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)): the values are Karate's, from its dry run.
- Run lightly ([ADR 0023](../decisions/0023-run-lightly-as-a-release-requirement.md)): only the lines in view are drawn ([virtual lists](2026-10-30-virtual-lists.md)).
- Solid 2 rules and UI conventions from AGENTS.md: tokens, `Icon`, `StatusMark`, `Tooltip`, `Cut`.
- No change to `src/renderer/Tree.tsx` (#224) or `src/renderer/History.tsx` (#70).

### What exists today

- Discovery is a dry run. `buildTree` (`src/main/karate/tree.ts`) reads its depth-0 events: `OUTLINE_ENTER` makes the outline node, each `SCENARIO_ENTER` with `isOutlineExample` a row node keyed `s:<file>#<section>.<n>`, `n` counting across all the outline's Examples tables.
- The dry run's depth-0 `FEATURE_EXIT` lists every scenario and row in `scenarioResults`. A row's entry has its `refId` and `exampleData`: its values by column name, in the table's column order (`{"a":"1","b":"2","expected":"3"}`). Values from a written table are strings; a dynamic table (`| read('rows.json') |`) gives them as its JSON has them. `buildTree` ignores `FEATURE_EXIT` today.
- `TestNode` (`src/shared/model.ts`) crosses to the window in the `project` event.
- The window holds each test's status (`state.statuses`) and time and error (`state.results`), live during a run.
- `Details.tsx` shows a scenario or row as `Leaf` and anything else as `Group`.
- `VirtualList` draws fixed-height rows in view as one tab stop. Its arrow keys, Home, End, Page Up and Page Down move the focus and call `onSelect`, as do Enter, Space and a click on the row.

## Approaches considered

1. **Values on the row nodes (chosen, the builder's call).** `buildTree` reads `exampleData` from the dry run's `FEATURE_EXIT` and puts it on each row node. The window has the values with the tree, synchronously. Cost: a few short strings per row, in main and in the window.
2. **Values on request.** Main keeps the values and the window asks for one outline's when it is selected, as it asks for a test's file steps. Less memory in the window, but a new IPC channel, an async load on every outline click, and main holds the values anyway.
3. **Read the Examples tables from the feature files.** `rowLines` already finds a table's rows. A dynamic table has no rows in its text, and this does again what Karate already did; dropped.

## Requirements

### R1. Row values come from Karate

1. Discovery keeps each outline row's column values from the dry run: the `exampleData` of the row's entry in its feature's depth-0 `FEATURE_EXIT`, matched by `refId`.
2. A value that is not a string is kept as its JSON text (`1`, `{"x":1}`).
3. A row Karate gives no values for has none; its line shows empty cells.

### R2. The table

1. When the selected node is an outline, the details pane shows, under the outline's name, status, location and counts (as today), a table labelled "Examples" with one line per row, in tree order.
2. Its columns are: the status mark, the row's number (1-based across all its Examples tables, as in its key), one column per example column, and the time of the last run. The example columns are those of all the outline's rows, in the order first met; a row without one shows an empty cell there.
3. A line's status and time are those the window holds, as in the tree: they change live during a run, and a row that has not run shows "Not run" and no time.
4. A column name or value too long for its cell is cut with an ellipsis and shown whole in a tooltip, through `Cut`. The columns of values share the width left by the status, number and time columns.
5. The header line stays in place while the lines scroll.
6. Only the lines in view are drawn, 24 px each, so an outline of 10,000 rows opens and scrolls as the tree does.
7. The table fills the height left in the details pane, and is never shorter than 8 lines or its line count, whichever is fewer; when the pane is shorter, the pane scrolls.

### R3. Opening a row

1. A click on a line selects its row, as a click on the row in the tree does: the details pane shows the row, and the tree shows it selected.
2. The table is one tab stop. Up, Down, Home, End, Page Up and Page Down move the focus through every line without opening one. Enter or Space opens the focused line's row.
3. Each line's accessible name gives its number, its values with their column names, its status and its time, for example "Row 2: a 2, b 3, expected 5, Passed, 6 ms".

## Acceptance criteria

```gherkin
Feature: Data-driven view

  Scenario: An outline's Examples before a run
    Given the "clean" project is open
    When I select the outline "add <a> and <b>"
    Then the details pane shows a table "Examples" with the columns "a", "b" and "expected"
    And its lines read "1 2 3", "2 3 5" and "5 5 10"
    And each line shows "Not run" and no time

  Scenario: Per-row results after a run
    Given the "clean" project is open
    And I select the outline "add <a> and <b>"
    When I run the outline and the run finishes
    Then each line of the table shows "Passed" and its time

  Scenario: A click opens a row
    Given the table of the outline "add <a> and <b>" is shown
    When I click its second line
    Then the details pane shows the row "add 2 and 3"
    And the tree shows that row selected

  Scenario: The keyboard moves without opening
    Given the table of the outline "add <a> and <b>" is shown
    And its first line has the focus
    When I press Down
    Then the second line has the focus
    And the details pane still shows the outline
    When I press Enter
    Then the details pane shows the row "add 2 and 3"

  Scenario: A dynamic Examples table
    Given the "edge" project is open
    When I select the outline "dynamic <a>"
    Then the table has the column "a" and the lines "1" and "2"

  Scenario: Examples tables with different columns
    Given an outline with one table of columns "a, b" and one of columns "a, c"
    When I select it
    Then the table's columns are "a", "b" and "c"
    And the second table's lines show an empty cell under "b"

  Scenario: A big outline
    Given an outline with 10,000 rows
    When I select it
    Then only the lines in view and a margin are in the page
    And a scroll to the end shows row 10,000
```

## Cost

- **Idle:** nothing new runs. No timer, poll or listener; the table is drawn only while an outline is selected.
- **Memory:** each outline row node holds its values: one small object of short strings. The bench's big project (2,000 rows of one column) adds about 100 KB to the tree in main and in the window, and to the `project` event's copy; an outline of 10,000 rows of five columns adds about 2 MB to each. Scenarios, features and folders carry nothing new.
- **CPU:** discovery reads one more field per row from events it already parses. Selecting an outline walks its rows once for the columns. During a run, each line in view reads its own status and time, so a status update redraws one mark, as in the tree.
- **Start time:** unchanged. **Size:** about 3 KB of code; no dependency.

## Calls made

The owner asked for sensible defaults. These are the builder's calls:

- The values ride on the tree (approach 1), not on request.
- The table sits under the outline's existing header in the details pane, not in a new tab or panel.
- Columns: status, number, values, time. No error column; no sorting or filtering.
- Several Examples tables make one table with the union of their columns.
- The arrow keys move without opening; Enter, Space or a click opens. This needs a `VirtualList` option to keep a move from calling `onSelect`.
- The tree does not scroll to a row opened from the table (#224 is changing the tree).
