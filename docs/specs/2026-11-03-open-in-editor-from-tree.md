# Open in VS Code from the test tree

Date 2026-11-03. Status: draft. Covers [#222](https://github.com/hpagniez/grace/issues/222). Builds on [Open a test in VS Code](2026-11-03-open-in-editor.md) (#68).

## Intent

Opening a test in VS Code takes two steps today: select it in the tree, then click "Open in VS Code" in the Details pane. A feature cannot be opened at all. The person reaches for this with the mouse while clicking around the tree, so a right-click on a row opens it, and a failed test lands on the step that failed.

## Goals, non-goals, anti-goals

- Goals: a right-click on a feature, outline, scenario or row offers "Open in VS Code"; a failed test opens where it failed; a feature opens at its top.
- Non-goals: other items in the menu (run, debug, copy path); a button on the row; a shortcut of its own; folders; other editors; a menu drawn by Grace.
- Anti-goals: any cost while idle or per drawn row; Grace opening a file outside the open project; a second way of checking what main opens.

## Constraints

- Everything main checks before it opens a file stays as in [open-in-editor R3 and R4](2026-11-03-open-in-editor.md#r3-main-checks-the-request): the request checks, the project check, the VS Code check and the `vscode://` link.
- The menu is Windows' own, so main shows it: the window asks over the typed preload API, never with anything a shell reads.
- `bench/limits.json` leaves about 200 nodes of room in `page.nodes.open`: the change adds no node to a drawn row.

### What exists today

The tree (`src/renderer/Tree.tsx`) draws its rows in a `VirtualList`. Each row element is a `treeitem` with `data-key`. Rows have run and debug buttons, and the keys Ctrl+Enter and Ctrl+Shift+Enter. Grace has no right-click menu anywhere: `main.ts` sets no application menu. The Details pane's button and error link call `api.openInEditor(file, line)`. A failed test's place comes from its error's first line (`FAILED_AT` in `src/renderer/Details.tsx`).

## Approaches considered

1. **Windows' own right-click menu. Chosen by the owner.** The window asks main to show the menu for a path and a line; main shows an Electron `Menu` with one item, and the item opens the file as the Details button does. It adds no DOM node and no dependency, and the menu key works on the focused row. It looks like Windows' menus, not like Grace.
2. **Grace's own right-click menu.** Styled like Grace and drawn only while open. It needs a new menu component (placing, arrow keys, Escape, closing on a click outside), about three to four times the code of 1, since no menu library is installed.
3. **An icon on the row beside Run and Debug.** The easiest to find. On every drawn row it adds about 240 nodes, past the `page.nodes.open` limit, so it could only show on the hovered or selected row.

## Requirements

### R1. The menu

1. A right-click on a feature, outline, scenario or row of the test tree shows Windows' menu with one item, "Open in VS Code", at the pointer. The menu key or Shift+F10 on the focused row shows it at the row.
2. A folder shows no menu.
3. The right-click does not change the selected row.
4. The menu shows whatever the run state, as the Details button does.

### R2. Where it opens

1. A test (scenario or row) whose status is failed and whose error's first line is a place Karate wrote (`<absolute Windows path>:<line>`, as [open-in-editor R2.1](2026-11-03-open-in-editor.md#r2-open-where-it-failed)) opens at that place, even in a called feature.
2. Any other test, and an outline, opens at its own line. A feature opens at line 1.

### R3. What main does

1. The window asks main for the menu with a path, a line and the point to show it at. Main refuses a request that is not a path and a line as [open-in-editor R3.1](2026-11-03-open-in-editor.md#r3-main-checks-the-request) describes, or whose point is not two numbers, with the notice "Invalid request", and shows no menu.
2. Choosing "Open in VS Code" does what the Details button does with that path and line ([open-in-editor R3.2 to R4](2026-11-03-open-in-editor.md#r3-main-checks-the-request)). Closing the menu does nothing.

## Cost

- Idle: nothing. One `contextmenu` listener on the tree's list, none per row; no node is added to a drawn row.
- A right-click: one IPC call and a native menu, built then dropped. Choosing the item costs what the Details button costs.
- Memory, start time: no change. Size: under 1 KB of code; no dependency.

## Design decisions already made

- Right-click, Windows' own menu: the owner's choice (approach 1).
- A failed test opens at the failed step: the owner's choice. It is the error link's place, which may be in a called feature.
- My call: a folder shows no menu, since VS Code would open a folder in a new window, not at a line.
- My call: the right-click does not select the row, so the Details pane stays on what the person was reading.
- My call: a feature opens at line 1, the top of its file.

## Acceptance criteria

Scenarios run in Vitest (jsdom for the window, Node for main) unless marked **(e2e)**, which run in Playwright against the built app with `shell.openExternal` replaced by a recorder and Electron's `Menu.popup` replaced by one that records the menu's items.

```gherkin
Feature: Open in VS Code from the test tree

  # R1
  Scenario: A right-click on a scenario asks for the menu
    Given a scenario "greets" at "tests/basic.feature:12"
    When I right-click its row
    Then Grace asks main for the menu for "tests/basic.feature" at line 12
    And the selected row does not change

  Scenario: A folder has no menu
    When I right-click a folder's row
    Then Grace asks main for no menu

  # R2
  Scenario: A failed test opens at its failed step
    Given a failed scenario whose error starts with "C:\p\tests\auth.feature:9"
    When I right-click its row
    Then Grace asks main for the menu for "C:\p\tests\auth.feature" at line 9

  Scenario: A feature opens at its top
    When I right-click the feature "tests/basic.feature"
    Then Grace asks main for the menu for "tests/basic.feature" at line 1

  # R3
  Scenario Outline: Main refuses a malformed menu request
    When main is asked for the menu with <request>
    Then it shows "Invalid request"
    And it shows no menu

    Examples:
      | request             |
      | line 0              |
      | the point "12", 4   |

  Scenario: Open a scenario from the tree (e2e)
    Given a project with a failing scenario is open
    When I right-click the scenario and choose "Open in VS Code"
    Then the menu's only item is "Open in VS Code"
    And Windows is handed "vscode://file/<project path>/tests/basic.feature:<scenario line>"
    When I run it, right-click it and choose "Open in VS Code"
    Then Windows is handed the link to the failing step's line
```
