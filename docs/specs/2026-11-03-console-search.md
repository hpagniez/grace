---
status: built
issue: 73
summary: "Find text in the console and step through the matches, and jump from one error line or failed test to the next"
---

# Find in the console, and step through its errors

Date 2026-11-03. Covers [#73](https://github.com/hpagniez/grace/issues/73).

## Intent

On a big project a run's console holds thousands of lines (up to its cap of 10,000), and today the only way to find something in it is to scroll. Two needs:

- Find a piece of text (a URL, a value, a message) and step from one occurrence to the next, with every line still on screen around it.
- Jump from one error to the next: the lines Karate logs at ERROR level, and the failed tests.

The second is less simple than it looks. Measured with the bundled Karate 2.1.4: **a failing test writes no ERROR line while it runs.** The failures show only at the end, in the summary Karate prints after the run:

```
failed features:
  C:\work\api\users.feature
    - fails match
      C:\work\api\users.feature:7
      * match y == { a: 2 }
      match failed: EQUALS
    - line 2
      …
============================================================
```

A failed test is a `    - <name>` entry in that block (an unnamed scenario reads `line <n>`, an outline row its expanded name). The ERROR lines are something else: a Java exception, or a script that logs at error level (`karate.logger.error(…)` prints `… ERROR karate.scenario - [ERROR] …`). Both count as errors here.

Karate's log lines carry no scenario id, and with several threads the lines of different tests interleave, so "this test's lines only" is out of reach (see Non-goals).

The Karate CLI has no find of its own: it prints to a terminal. Grace follows what terminals and editors do for find: VS Code's terminal, Windows Terminal, a browser's Ctrl+F.

## Who and what success looks like

Someone with Grace open all day, after a run of a few hundred to 10,000 tests, looking for why something failed. Success: Ctrl+F, a few letters, Enter, Enter, and they are on the line they wanted, with its neighbours in sight; one click (or F8) takes them to the next error, failed tests included. While nobody searches, the console costs what it costs today.

## Goals, non-goals, anti-goals

- Goals: find text in the console and step through the matches; step through the errors (ERROR lines and failed tests) from the tab row; both while a run is going too.
- Non-goals: hiding the lines that do not match (a filter); regular expressions, a case-sensitive or whole-word switch; this test's lines only (Karate gives no scenario id); stepping through WARN lines; search history; find in the History, HTTP or Variables tabs; keeping the text after the app closes; opening the test of an error from the console.
- Anti-goals: any timer, poll or animation while idle; redrawing the console's rows to paint matches; a search that freezes the window on a 10,000-line console; the view jumping on its own while a run adds lines.

## Constraints

- `AGENTS.md`: Solid 2, UI conventions (semantic colour tokens only, the dark block overrides only colours, `Icon` with `lucide` data, `Tooltip`, shared class strings in `buttons.ts` and `ui.ts`), Performance (idle costs nothing).
- This work stays in the console's own files, `BottomPanel.tsx`, `reduce.ts` (a count of dropped lines, R5) and the shared UI strings and tokens. Nothing in the main process changes.
- The summary block is Karate's text, not a contract, like the HTTP step logs (`src/main/karate/http.ts`). The e2e test runs real Karate with a failing test, so a Karate update that changes the block fails it.

### What exists today

- `src/renderer/Console.tsx`: one scroller with the run banner (its `children`) first, then one `<div>` per line, drawn with `For` (lines wrap, so the list is not virtual). Lines with ` ERROR ` are `text-failed`, with ` WARN ` `text-cancelled`. It follows new lines while the user is within 20 px of the bottom.
- `src/renderer/reduce.ts`: the store keeps at most `CONSOLE_LIMIT` (10,000) lines, dropping the oldest; a new run clears them. Main sends lines in batches, one per event-loop turn (`src/main/batch.ts`).
- `src/renderer/BottomPanel.tsx`: the tab row (Console, History, HTTP, Variables during a debug run), whose right end holds History's Clear button while History shows. The panel stays mounted while a project is open, under the Settings page too (invisible), and unmounts on the home screen. The Console tab's content unmounts when another tab shows.
- No key handler for Ctrl+F or F8. F5, F10 and F11 are the debugger's, only while paused (`Toolbar.tsx`).

## Approaches considered

Where the controls go, shown to the owner as three mockups:

1. **A: both on the tab row**, always: the find field and the error stepping. Always in sight, but takes the row's room when unused.
2. **B: errors on the tab row, find floats.** **Chosen by the owner.** "3 errors ↑ ↓" sits on the tab row; the find bar floats over the console's top right corner after Ctrl+F or a search icon, as in VS Code's terminal and Chrome.
3. **C: one floating bar with an Errors mode** that switches what ↑ ↓ step through. Least on screen, but errors are one click further. The owner picked C first, then B.

How the matches are painted:

1. **The browser's highlight API (CSS Custom Highlight API).** **Chosen by the owner.** Chromium paints ranges of existing text; no row changes. A keystroke reads the lines once and hands the browser one range per match. Its cost: jsdom has no such API, so unit tests check the matching and the ranges handed over, and e2e checks the paint.
2. **A `<mark>` around each match.** Every matching row redraws on every keystroke and the browser lays out thousands of wrapped rows again: heavy on a big run.
3. **Electron's `findInPage`.** Native, but searches the whole window (tree, details, banner), cannot step through errors, and goes through the main process.

Find keeps every line on screen and steps through matches, rather than hiding the lines that do not match: the owner's choice, over a filter.

## Requirements

### R1. What an error is

1. A line that contains ` ERROR ` (space, ERROR, space): the test that colours a line red today.
2. A failed test: a line that starts with exactly four spaces and `- `, after a line that is exactly `failed features:` and before the next line that starts with `=`.
3. Nothing else: WARN lines are not errors.

### R2. Errors on the tab row

1. While the Console tab shows and the console holds at least one error, the tab row's right end shows the errors group: the `CircleX` icon and the count in the failed colour, then Previous error (`ArrowUp`) and Next error (`ArrowDown`) icon buttons. With no error the group is absent.
2. The count reads `1 error`, `3 errors`; once an error is current, `2 of 3 errors`. It is a live region (`role="status"`).
3. Next error moves to the next error below the current one, Previous error to the one above; both wrap round. With no current error, the line at the top of the view stands in for it: Next goes to the first error at or below it, Previous to the last error above it, each wrapping round.
4. The current error's line is scrolled into view, centred, and painted with the failed tint (R4).
5. F8 does Next error and Shift+F8 Previous error, anywhere in the window, while the Console tab shows, the console is visible (not under the Settings page) and no dialog is open. With no error they do nothing.
6. The buttons' tooltips: `Previous error (Shift+F8)`, `Next error (F8)`.
7. The count follows the lines as they arrive: ERROR lines count as they come, failed tests once Karate prints its summary at the end of the run.
8. An error stays current until another step, a new run, or its line leaving the console at the line cap; then none is current.

### R3. The find bar

1. Ctrl+F, anywhere in the window while the project view shows (not under the Settings page) and no dialog is open, shows the Console tab if another tab shows, opens the find bar, focuses its field and selects its text. The Find icon button (`Search`, tooltip `Find (Ctrl+F)`) at the tab row's right end, after the errors group, while the Console tab shows, does the same.
2. The bar floats over the console's top right corner, on the raised surface with the float shadow: the field (placeholder `Find`, accessible name `Find in console`), the count, Previous match (`ArrowUp`, tooltip `Previous match (Shift+Enter)`), Next match (`ArrowDown`, tooltip `Next match (Enter)`) and Close (`X`, tooltip `Close (Escape)`).
3. A match is the field's text found in one line, not case-sensitive, plain text (no pattern). Every occurrence in a line counts; after a match the search goes on from its end, so matches never overlap. The banner above the lines is not searched.
4. Each keystroke repaints: every match in the match colour, the current one in the current-match colour (R4).
5. The count reads `2 of 5`; `No results` when the field has text and nothing matches; nothing when the field is empty. It is a live region (`role="status"`). While there are matches, one is always current.
6. After a keystroke the current match is the first match at or below the line at the top of the view, else the first match. It is scrolled into view, centred, only when it is not in view already.
7. Enter and the Next match button go to the next match, Shift+Enter and Previous match to the one before; both wrap round and scroll the match into view, centred.
8. Escape in the bar, or Close, closes the bar and clears its paint. Focus goes back to what had it before the bar opened, when that is still in the page.
9. The field keeps its text when the bar closes, when another tab shows and across runs, for as long as the project view stays mounted (until the home screen). The bar stays open across tab switches and runs. Reopening it shows the text selected and paints its matches again.
10. While a run adds lines, the count and the paint follow the new lines; the current match stays on its line; nothing scrolls to a match on its own, and the console follows the bottom as today. When there are matches and none is current (the current match's line left at the line cap, a new run emptied the console, the bar reopened), the first match at or below the line at the top of the view becomes current, else the first match, without scrolling. A new run keeps the text.
11. The bar never searches while it is closed.
12. The text, whether the bar is open, and the current match and error live in the bottom panel, so they survive the Console tab's content unmounting while another tab shows.

### R4. Paint

1. Matches are painted with the CSS Custom Highlight API: `console-match` (the matches within a view's height above and below the view, painted again as the console scrolls), `console-match-current` (the current match), `console-error-current` (the whole text of the current error's line). No console row is redrawn or wrapped to paint. The ranges are static (`StaticRange`): a live `Range` is updated on every change of the page.
2. Their colours come from tokens in `src/index.css`: two new colour tokens for a match and the current match, with light and dark values, and `--color-failed-tint` for the current error. The current match differs from the others by more than hue alone (a stronger fill).
3. The ERROR and WARN text colours stay as they are.

### R5. A line keeps its place

1. The store counts the lines dropped at the cap, and those a new run clears, since the project opened (`consoleDropped`), reset when the project closes. A line's number is that count plus its index, so it does not change as older lines drop, and a new run's lines never take a number an earlier run's line had: nothing held from the last run stays current.
2. The current match and the current error are held by line number (and, for a match, its place in the line), so they stay on their line while lines arrive and drop.
3. "The console is visible" (R2.5, R3.1) is checked on the panel's element (`checkVisibility()`), not passed down from `App.tsx`: under the Settings page the project view is `invisible`.

## Cost

- **Idle (no run going):** zero CPU. Two `keydown` listeners on the window while a project is open (Ctrl+F, F8): they run only on a key press. No timer, poll or animation.
- **A keystroke in the find field:** one pass over the console's lines (at most 10,000) with one case-insensitive pattern, then one static range per match near the view (a few dozen) handed to the browser, which paints without layout. Target: under 16 ms of work for 10,000 lines with 10,000 matches. Measured in the built app (e2e `a full console`, 2026-11-03, the reference machine): 4 to 5 ms of work; the next frame is painted 18 to 31 ms after the key, the wait for the display's next frame included.
- **A scroll while the bar is open:** the matches near the view are painted again: a few dozen ranges, at most once a frame.
- **A step (Enter, F8, a button):** a lookup in the match or error list and one `scrollIntoView`.
- **During a run, find bar closed:** while the Console tab shows, the error list is worked out again on each console batch: one pass over the lines with `includes(" ERROR ")` and the summary rule, the same order of work as the store's existing `concat` and `slice` of the same lines and the `For` that redraws them. Nothing while another tab shows.
- **During a run, find bar open:** the error pass, plus the find pass and a new set of ranges on each batch.
- **A 10,000-test run:** the console holds at most 10,000 lines whatever the number of tests, so every pass is bounded by 10,000 lines. With the bar closed and another tab showing, the run costs exactly what it costs today.
- **Memory:** the match positions (three numbers per match) while the bar is open, the static ranges near the view, and the error line numbers. Under 1 MB at 10,000 matches. Nothing kept once the bar closes.
- **Start time and size:** no work at start; a few kilobytes of code and CSS, no dependency.

## Design decisions already made

By the owner:

- Find keeps every line on screen and steps through matches (over a filter that hides lines).
- Stepping through errors covers both the ERROR lines and the failed tests.
- Layout B: errors on the tab row, the find bar floating after Ctrl+F.
- Matches painted with the browser's highlight API.

Proposed with them (calls the owner can change while reading):

- F8 and Shift+F8 for errors, as VS Code's next and previous problem; Enter and Shift+Enter in the bar, as browsers.
- The errors group is absent when there is no error, rather than showing `0 errors`.
- With nothing current, stepping and typing start from the line at the top of the view, as a browser's find does, rather than from the first line.
- The current error is painted with the failed tint over its text, without the left bar of the mockup: the highlight API paints only text.
- Ctrl+F from another tab shows the Console tab; the text lasts until the home screen, not after the app closes.
- Escape closes the bar and keeps the text.
- While there are matches one is always current, so the count always reads `N of M` (R3.5, R3.10).
- The store counts the lines dropped at the cap, so a current match or error keeps its line (R5).

## Left to the builder

- The names and values of the two match colour tokens, within R4.2 and the palette and token tests.
- Where the match and error logic lives: a pure module beside `Console.tsx`, tested in Node, is expected.

## Acceptance criteria

Scenarios run in Vitest (jsdom, with the highlight API stubbed where noted) unless marked **(e2e)**, which run in Playwright against the built app with real Karate.

```gherkin
Feature: Find in the console, and step through its errors

  # R1
  Scenario: ERROR lines and failed tests are errors, WARN lines are not
    Given the console lines
      | 09:46:08.476 [virtual-36] INFO  karate.scenario - hello                 |
      | 09:46:08.502 [virtual-40] WARN  karate.scenario - careful               |
      | 09:46:08.502 [virtual-46] ERROR karate.scenario - [ERROR] custom boom   |
      | failed features:                                                        |
      |   C:\work\a.feature                                                     |
      |     - fails match                                                       |
      |       C:\work\a.feature:7                                               |
      |       ==========                                                        |
      |     - line 2                                                            |
      | ============================================================            |
      |     - after the block                                                   |
    Then the errors are the lines "… ERROR … custom boom", "    - fails match" and "    - line 2"

  # R2
  Scenario: The errors group shows the count and steps through the errors
    Given the Console tab shows a console with 3 errors
    Then the tab row shows "3 errors" with Previous error and Next error
    When I click Next error
    Then the first error at or below the top of the view is current
    And the count reads "1 of 3 errors"
    When I press F8 twice
    Then the count reads "3 of 3 errors"
    When I press F8
    Then the count reads "1 of 3 errors" again
    When I press Shift+F8
    Then the count reads "3 of 3 errors"

  Scenario: No errors, no group
    Given the console has no error
    Then the tab row shows no errors group
    And F8 does nothing

  Scenario: The errors group belongs to the Console tab
    Given the console has 3 errors
    When the History tab shows
    Then the tab row shows no errors group
    And F8 does nothing

  Scenario: A failing run's errors (e2e)
    Given a project with a test that fails and a test that logs an error
    When I run all and the run finishes
    Then the tab row shows "2 errors"
    When I press F8 until the count reads "2 of 2 errors"
    Then the console is scrolled to the failed test's "    - " line
    And that line is painted as the current error

  # R3
  Scenario: Ctrl+F opens the find bar from another tab
    Given the HTTP tab shows
    When I press Ctrl+F
    Then the Console tab shows
    And the find bar is open with its field focused

  Scenario: Matches are counted, case aside, without overlap
    Given the console lines "get user 42", "create USER", "aaaa"
    When I type "user" in the find field
    Then the count reads "1 of 2"
    When I type "aa" in the find field instead
    Then the count reads "1 of 2"

  Scenario: Enter and Shift+Enter step through the matches and wrap
    Given the find field holds "user" with 2 matches, the first current
    When I press Enter
    Then the count reads "2 of 2"
    When I press Enter
    Then the count reads "1 of 2"
    When I press Shift+Enter
    Then the count reads "2 of 2"

  Scenario: No results
    When I type "nothing-like-this" in the find field
    Then the count reads "No results"

  Scenario: Matches are painted without redrawing the rows (highlight API stubbed)
    Given the console lines "get user 42" and "create user"
    When I type "user" in the find field
    Then the "console-match" highlight holds 2 ranges, each covering "user"
    And the "console-match-current" highlight holds the first one
    And the console's row elements are the same elements as before

  Scenario: Escape closes the bar, keeps the text, and gives the focus back
    Given the tree had the focus when I pressed Ctrl+F and typed "user"
    When I press Escape
    Then the find bar is closed and no match is painted
    And the focus is back on the tree
    When I press Ctrl+F
    Then the field holds "user", selected, and its matches are painted again

  Scenario: The text survives a tab switch and a new run
    Given the find field holds "user"
    When the History tab shows and then the Console tab
    Then the find bar is open with "user"
    When a new run starts and prints "user again"
    Then the count reads "1 of 1"

  Scenario: Lines that arrive during a run do not move the view
    Given the find field holds "user" and I stepped to the first match, away from the bottom
    When lines with "user" arrive
    Then the count grows and the current match stays on its line
    And the scroll position does not change

  Scenario: Find in a real run (e2e)
    Given a project that prints "hello" from two tests
    When I run all and press Ctrl+F and type "hello"
    Then the count reads "1 of 2"
    And the page has a "console-match" highlight of 2 ranges
    When I press Enter
    Then the count reads "2 of 2"

  # Cost
  Scenario: The bar does not search while it is closed
    Given the find field holds "user" and the bar is closed
    When lines arrive
    Then no match is worked out and no highlight is set

  Scenario: A line keeps its place when older lines drop at the cap
    Given a full console of 10,000 lines and the current match on the line "user 9000"
    When 100 more lines arrive and the oldest 100 drop
    Then the current match is still on the line "user 9000"

  Scenario: A full console (e2e)
    # The lines are sent to the window as console events, as e2e/panel.spec.ts does: a real run's summary
    # and banner would push matching lines out at the cap.
    Given a console of 10,000 lines that each match once, followed to its bottom
    When I type the search
    Then the count reads "<n> of 10000", the current match being the first in view (R3.6)
    # The keystroke's time (Cost: under 16 ms) is measured by hand on the reference machine and written in the PR:
    # a timing assertion would be flaky on CI.
```
