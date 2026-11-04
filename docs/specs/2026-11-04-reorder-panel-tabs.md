# Reorder the bottom panel tabs

Date 2026-11-04. Status: draft. Covers [#94](https://github.com/hpagniez/grace/issues/94).

## Intent

The bottom panel has three tabs, Console, History and HTTP, always in that order. People who live in the HTTP tab, or who never read the console, want their tab first. A tab can be dragged to a new place, or moved with the keyboard, and the order is remembered like the panel's height ([panelHeight.ts](../../src/renderer/panelHeight.ts), spec UI polish R1).

## Requirements

### R1. The tabs

1. The three tabs start in the order Console, History, HTTP.
2. The Variables tab (step debugger R7.4) is not part of the order. It shows only during a debug run, always after the three, and it cannot be moved, dropped on or dragged.

### R2. Dragging

1. A tab can be dragged onto another of the three. It takes that tab's place, and the tabs between them shift one place the other way.
2. While a tab is dragged over another, a 2 px accent bar shows on the side of the target where the dragged tab will land: the right side when it comes from the left, the left side when it comes from the right. Nothing shows over the dragged tab itself.
3. A drop anywhere else, on the dragged tab itself, or a drag cancelled with Escape, changes nothing and saves nothing.
4. Dragging never changes the shown tab; a click on a tab still shows it.
5. Only a tab dragged from the tab list reacts: a file or text dragged in from outside does neither mark nor move anything.

### R3. Keyboard

1. Alt+Left and Alt+Right on a focused tab move it one place left or right. At the first place, Alt+Left does nothing, and at the last, Alt+Right does nothing.
2. The tab keeps the focus after it moves, so the keys can be pressed again, and a screen reader reads its new place.
3. The shown tab does not change.

### R4. Remembered

1. Each move saves the order in the renderer's `localStorage` under `grace.panelTabs`, as a JSON list of tab ids, for all projects.
2. The panel reads it before it first paints. A saved value that is not a JSON list of exactly Console, History and HTTP, once each, is unusable: the tabs show in the default order, and the value stays as it is until the next move replaces it.
3. Storage that throws on a read or a write is no error: the default order shows, a move is only forgotten.

## Cost

- Idle: nothing runs. The tabs get `dragstart`, `dragover`, `drop`, `dragend` and `keydown` handlers, which run only while a person drags or presses a key. No timer, poll, observer or animation; the drop bar has no transition.
- A move: one array of three ids, a re-ordering of three buttons, and one `localStorage` write. During a drag, `dragover` fires every few tens of milliseconds, and its handler writes a signal only when the target changes.
- Memory: a three-element list and two signals. Start: one `localStorage` read, as for the height. Size: under 1 KB of code, no dependency.

## Design decisions already made

The builder's calls, open to the owner's correction:

- Native HTML drag and drop on the tab buttons, not pointer events: the app is Chromium only, it gives the drag image, Escape and the cursor for free, and the code is smaller. The e2e test drives it with the real mouse.
- A drop marker shows (R2.2): without it a drag over three small tabs gives no sign of where the tab will land.
- A dragged tab takes the target's place (R2.1) rather than choosing a side by where the pointer is in the target: with three tabs the result is the same, and there is no half-tab to aim at.
- A saved order that names an unknown tab or misses one is dropped whole (R4.2): there are three tabs and no older format to keep ([No backward compatibility](../../AGENTS.md)).
- No spoken announcement beyond the refocus (R3.2): the tab list already reads "tab 2 of 3".
- Variables stays last and fixed (R1.2): it comes and goes with a debug run, so a place for it in a saved order would be a place for a tab that is mostly absent.

## Acceptance criteria

```gherkin
Feature: Reorder the bottom panel tabs

  Background:
    Given a project is open in Grace
    And the bottom panel shows the tabs "Console", "History" and "HTTP"

  # R1
  Scenario: The tabs start in their usual order
    Given nothing is saved
    Then the tabs read "Console", "History", "HTTP" from left to right

  Scenario: Variables stays after the three
    Given the order is "HTTP", "Console", "History"
    When a debug run starts
    Then the tabs read "HTTP", "Console", "History", "Variables"
    And "Variables" cannot be dragged
    And Alt+Left on "Variables" does nothing

  # R2
  Scenario Outline: Dragging a tab onto another takes its place
    When I drag "<from>" onto "<onto>"
    Then the tabs read <order>
    Examples:
      | from    | onto    | order                      |
      | Console | HTTP    | "History", "HTTP", "Console" |
      | HTTP    | Console | "HTTP", "Console", "History" |
      | History | Console | "History", "Console", "HTTP" |

  Scenario: A drop marker shows where the tab will land
    When I drag "Console" over "History"
    Then "History" shows an accent bar on its right side
    When I drag it over "Console"
    Then no bar shows

  Scenario: Dragging "HTTP" over "History" marks the left side
    When I drag "HTTP" over "History"
    Then "History" shows an accent bar on its left side

  Scenario Outline: A drag that goes nowhere changes nothing
    When I drag "Console" and <end>
    Then the tabs read "Console", "History", "HTTP"
    And nothing is saved
    Examples:
      | end                                  |
      | drop it on itself                    |
      | drop it outside the tab list         |
      | press Escape                         |

  Scenario: A drag keeps the shown tab
    Given the "Console" tab is shown
    When I drag "HTTP" onto "Console"
    Then the "Console" tab is still shown

  Scenario: Something dragged in from outside is ignored
    When I drag a file over "History"
    Then no bar shows
    And dropping it moves no tab

  # R3
  Scenario: Alt+Right moves a tab one place right
    Given the "Console" tab has the focus
    When I press Alt+Right
    Then the tabs read "History", "Console", "HTTP"
    And "Console" has the focus
    When I press Alt+Right
    Then the tabs read "History", "HTTP", "Console"
    When I press Alt+Right
    Then the tabs still read "History", "HTTP", "Console"

  Scenario: Alt+Left moves a tab one place left, and stops at the first place
    Given the order is "History", "HTTP", "Console" and "Console" has the focus
    When I press Alt+Left
    Then the tabs read "History", "Console", "HTTP"
    And "Console" has the focus
    Given the "History" tab has the focus
    When I press Alt+Left
    Then the tabs read "History", "Console", "HTTP"

  Scenario: Moving a tab does not change the shown tab
    Given the "HTTP" tab is shown and the "Console" tab has the focus
    When I press Alt+Right
    Then the "HTTP" tab is still shown

  # R4
  Scenario: The order is remembered
    When I drag "Console" onto "HTTP"
    And I reload the window
    Then the tabs read "History", "HTTP", "Console"

  Scenario Outline: An unusable saved order shows the default
    Given <saved> is saved as the order
    When the panel shows
    Then the tabs read "Console", "History", "HTTP"
    Examples:
      | saved                              |
      | not JSON                           |
      | ["history","console"]              |
      | ["history","console","http","x"]   |
      | ["history","history","http"]       |
      | ["history","console","logs"]       |
      | {"console":0}                      |

  Scenario: Storage that fails costs nothing
    Given reading and writing storage throws
    Then the tabs read "Console", "History", "HTTP"
    When I press Alt+Right on "Console"
    Then the tabs read "History", "Console", "HTTP"
```
