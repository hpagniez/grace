# Home screen

Date 2026-10-27. Status: approved. Covers [feature](../features.md) F27.

## Intent

karate-kid has no place to start from. On a first start, or when the last project's folder is gone, the window shows an empty project view: "No project" in the toolbar, "Open a Karate project folder" in the tree, and every project control on screen with nothing to act on. Switching between the few suites a person works on means `Open…` and a walk through the folder dialog every time; the app remembers one folder (`lastProject`) and nothing else.

This spec adds a home screen: a button to pick a folder, and the folders opened recently, one click each. A `Home` button in the toolbar closes the open project and goes back to it.

## Who and what success looks like

The walking skeleton's user, iterating on two or three API suites, switching between them during the day.

Success:
- Start the app with a project open last time: it opens that project, as today. The home screen does not show.
- Start it for the first time, or after the last folder was moved: the home screen shows, with the folders opened before.
- Switch to another suite opened before in two clicks: `Home`, then its row.
- Go home and quit: the next start shows the home screen again.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- A recent list or "Open recent" menu in the toolbar. Switching goes through the home screen (decided in the brainstorm).
- Pinning, renaming, sorting or searching the list. More than 10 entries.
- A layout badge (Maven, Gradle, Standalone), last-opened time or test count in a row. Each would read the disk or the project per row at every visit.
- Pruning folders that no longer exist. A USB drive or network share may come back.
- Dropping a folder onto the window, opening a folder given on the command line, or a Windows jump list.
- Seeding the list from the `projects` map or from `lastProject` for people upgrading. The list starts empty and fills as folders open.
- Noticing a folder appear or disappear while the home screen shows. The list is checked when main sends it.

**Anti-goals** (failure even if everything else works):
- The home screen flashing at start while the last project opens.
- Main opening a folder the renderer names that is not in the saved list.
- Going home or removing a row losing a project's saved tag filter, environment, run properties, secrets, thread count or run history.
- A JVM left running after `Home`.

## Constraints

- No new dependency.
- `contextIsolation`, `sandbox` and `nodeIntegration: false` stay. The renderer reaches main only through new typed preload calls.
- Solid 2 rules of `AGENTS.md`.
- Every save of `settings.json` goes through `update()` (atomic since F41) and keeps the other keys.
- The window follows the Windows theme, as the rest of the app: the home screen has a dark variant.

### What the app does today

- `src/main.ts`, startup: `createWindow`, then `loadLastProject(dataDir)`; if it returns a folder, `runner.open(last)`. `loadLastProject` returns null when `lastProject` is missing, not a string, or not an existing folder. Nothing saves `lastProject` after the startup open.
- `CH.open` (`api.openProject()`, the toolbar `Open…`): a folder dialog starting in `path.dirname(lastProject)`, then `runner.open(folder)`, then `saveLastProject` if it returned true. `handle()` turns any throw into a notice.
- `Runner.open(root)` runs inside `guarded`: refused with the notice "A run is already in progress" while busy; emits `busy` true, then false. A folder that does not exist publishes `{ project: null, tree: null, error: "Folder not found: <root>" }` and returns false. Otherwise it runs a dry run (`rediscover`), which publishes `{ project, tree, error }` only when the dry run ends: before that, the window still has the previous project, or none. A dry run that fails (Karate exit 3, a syntax error) still opens the project with `error` set and returns true. Cancel during the dry run puts back the project before (`this.last`, or `project: null`), emits "Cancelled" and returns false.
- `Runner.replay()` answers `CH.ready` (each page load): the last `project` event, `filter`, `threads`, `environment`, `history`, `busy`; only `busy` when no project was ever published.
- Renderer: `App` always shows `Toolbar`, `Progress`, `FilterBar`, the notice bar, `Tree` + `Details`, `BottomPanel`. With no project the toolbar shows "No project" and the tree "Open a Karate project folder" (`tree-message`). `index.html` has a `#startup` screen that `App` removes once settled.
- `settings.json` (`src/main/settings.ts`): `lastProject`, `window`, and `projects` keyed by folder (tag filter, environment and properties, secrets, threads). Loads never throw.
- e2e: `launch()` gives each test a fresh `userData`; `openProject(app, dir)` stubs `dialog.showOpenDialog` and clicks `open-button`; `recordOpenDialog` records the dialog's `defaultPath`. About ten e2e and unit tests use `open-button` or the "Open a Karate project folder" text.

## Approaches considered

1. **Main owns the list and pushes it as a new `recent` event**, replayed on `CH.ready` like `filter`, `threads` and `history`. The renderer shows the home screen when no project is open and none is opening. New calls `openRecent(root)`, `removeRecent(root)`, `closeProject()`. Every piece of state already reaches the renderer this way, and the list updates itself after an open, a removal or a close. **Chosen.**
2. The renderer pulls the list with `api.recentProjects()` each time the home screen shows. The only pull-style state in the app, and it must re-ask after each open, removal and close.
3. The list in the renderer's `localStorage`, like the panel height. Only main knows when an open succeeded, and main has to check every path anyway; F27's row already names `settings.json`.

## Requirements

### R1. When the home screen shows

1. The window shows the home screen when no project is open and main is not opening one. Otherwise it shows the project view.
2. At start, main opens `lastProject` as today when `loadLastProject` returns a folder. If it returns null, the window shows the home screen.
3. While main opens a folder and no project is open (the startup open, `Open folder…`, a recent row), the window shows the project view for that folder: its path in the toolbar, busy, the tree showing it is discovering, `Cancel` enabled. The home screen never shows in between, including before the first `project` event of a start.
4. If that open ends without a project (the folder is gone, or `Cancel`), the window goes back to the home screen, with the reason in the notice bar ("Folder not found: …", "Cancelled").
5. While a project is open, opening another (`Open…` in the toolbar) behaves as today: the open project stays on screen until the new one is published, and comes back if the open is cancelled.
6. A dry run that fails opens the project with its error, as today (`Runner.open` returns true). The project view shows the error; the home screen does not.

### R2. The home screen

Layout as in the [brainstorm mockup](2026-10-27-home-screen-mockup.png) (part 1; part 2 is the toolbar of R3): a centred column about 540 px wide, no app name or logo beyond the heading, no toolbar, no progress bar, no filter bar, no tree, no details, no bottom panel. The notice bar stays above it. The mockup's colours are a sketch; R2.8 rules.

1. A heading "Open a Karate project", the line "Pick a folder, or reopen one you used before.", and a primary button `Open folder…`.
2. `Open folder…` does what `Open…` does: the same folder dialog, then the same open. The dialog starts in the parent of the newest folder of the recent list that is an existing folder, or with no starting folder if none is. This keeps the F31 rule (ui-polish R3) that a deleted folder gives no starting folder. `Open…` in the toolbar starts there too (it used `lastProject`, which `Home` now deletes).
3. Under it, "Recent folders" and the list, newest first. With an empty list, neither the heading nor the list shows.
4. Each row shows the folder's name (its last path segment) and its full path, as saved. Clicking the row opens that folder (R5).
5. A row whose folder was not an existing folder when main sent the list is greyed out, shows "not found" after the name, and does nothing on click. The check never blocks main's thread (a disconnected network share can take 20 s or more to answer): main checks every entry in parallel with async `fs` calls, each capped at 1 s, and an entry whose check has not answered by then counts as existing (clickable; if it is really gone, opening it says "Folder not found"). Until the checked list arrives, the home screen shows without the list.
6. Each row has a `×` button, "Remove from list", always shown at the row's right end. It removes the row at once, with no confirmation: it forgets the entry only, never touches the folder, the project's saved settings or its run history. If the row's folder is also `lastProject` (same folder, R4.2), removing it deletes `lastProject` too, so the folder does not reopen by itself at a later start once its drive is back.
7. Rows and buttons are real buttons: reachable with Tab, Enter or Space activates them, visible focus ring. The `×` button has an accessible name.
8. Light and dark variants, from the existing Tailwind palette (zinc, as the rest of the app).

### R3. The `Home` button

1. `Home` is the first item of the toolbar, left of the project path.
2. It is disabled while main is busy (a discovery or a run), like `Open…`.
3. Clicking it closes the project: main stops holding it, the window drops the project, its tree, filter, environment, threads, history, run state, selection, console, HTTP calls and report, and shows the home screen. The closed folder stays first in the recent list.
4. Closing deletes `lastProject` from `settings.json`, so the next start shows the home screen. It keeps every other key, the project's entry in `projects`, and its runs on disk.
5. Main refuses `closeProject()` while busy, with the notice "A run is already in progress", whatever the renderer shows. No Java process is started or left by closing.
6. After `Home`, `api.ready()` (a page load) replays the home screen, not the closed project.

### R4. The saved list

1. `settings.json` key `recentProjects`: an array of absolute folder paths, newest first, at most 10.
2. Every open that succeeds (`Runner.open` returns true) puts the folder first and saves `lastProject`, whatever started it: the startup open, `Open folder…`, `Open…`, a recent row. An entry for the same folder is removed first. The same folder means the same path compared case-insensitively (Windows paths); the path saved is the one just opened. An 11th entry is dropped from the end.
3. An open that fails or is cancelled changes neither the list nor `lastProject`.
4. Loading never throws. A missing key, or a value that is not an array, reads as empty. Entries that are not strings, not absolute paths, or repeat an earlier entry (case-insensitively) are dropped; past 10 entries the rest are dropped. A broken `settings.json` reads as empty, as for the other keys.
5. A save of the list or of `lastProject` that fails never undoes the open: the project stays open and a notice says the recent list could not be saved.
6. Removing a row saves the list without it. A save that fails shows a notice and keeps the row.

### R5. Calls from the renderer

The renderer is not trusted with paths.

1. `openRecent(root)`: main opens `root` only if it is a string of at most 1000 characters that is in the saved list, exactly as saved. Otherwise it emits the notice "Invalid request" and opens nothing. While busy, `Runner.open` refuses it as today.
2. `removeRecent(root)`: the same check. A root not in the list emits "Invalid request" and changes nothing.
3. `closeProject()` takes no argument.
4. The `recent` event carries, per entry, the path and whether it is an existing folder (R2.5). Main sends it after the `CH.ready` replay, once the checks of R2.5 settle, and after every change of the list or of the open project that changes what the home screen shows.
5. Opening a row still checks the folder with `Runner.open`'s synchronous check, as `Open…` does today; a row on a share that hangs can block main while it opens. Making that check async is out of scope.

## Design decisions already made

- The home screen shows only when no project is open; the last project still opens at start (the person).
- Switching projects goes through `Home`, not a toolbar list (the person).
- `Home` closes the project, and closing deletes `lastProject`, so a quit on the home screen starts on it (the person).
- Approach 1: main owns the list and pushes a `recent` event (the person).
- `Home` disabled while busy and refused by main while busy; 10 entries, newest first; a folder joins the list only once it opened; missing folders greyed out, kept, not clickable; `×` removes without confirmation; folder name and path per row, no badge; the list shared across the app as `recentProjects` (the spec author, accepted by the person with the mockup).

The spec author's calls: the project view, not the home screen, while a folder opens with no project open (R1.3), so `Cancel` stays reachable and a start never flashes the home screen; the dialog starting in the newest existing recent folder's parent (R2.2); no list heading when empty (R2.3); case-insensitive matching (R4.2); every successful open moving its folder first, the startup open included (R4.2); "Invalid request" for a path not in the list (R5).

Resolved by the builder check (2026-10-27), all as the spec author's calls: existence checks async and capped at 1 s, an unanswered check counting as existing (R2.5, R5.4); the dialog skipping missing folders, per F31 (R2.2); removing a row also deleting a matching `lastProject` (R2.6); the mockup committed next to this spec and `×` always shown (R2); the no-flash check moved to a unit test plus a by-hand check, since Playwright may not see the first paint.

## Left to the builder

- How main tells the window that it is opening a folder (R1.3), as long as cancel and failure restore what was there (R1.4, R1.5) and `replay()` stays correct after a page reload during the open.
- Where the list logic lives in `settings.ts` and how `Runner` resets on close (a `close()` method, say), as long as `rediscover`'s cancel restore still works.
- Test ids, as long as `openProject()` in `e2e/helpers.ts` works from both the home screen and the toolbar. The existing tests that expect "Open a Karate project folder" or click `open-button` with no project open change with it.
- Copy of the "could not be saved" notices.

## Acceptance criteria

Every scenario is automated with Playwright against the built app, each with a fresh app data folder, unless it is marked **(unit)** or **(by hand)**. Unit scenarios run in Vitest against `settings.ts` with a temp folder, `Runner` with the test doubles of `runner.test.ts`, or the renderer with `@solidjs/testing-library`. By-hand scenarios run on the packaged app and go into a dated record in `docs/checks/`. Where a scenario starts from a saved list, the test writes `settings.json` into the app data folder before launch.

```gherkin
Feature: Home screen

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  # R1: when it shows

  Scenario: A first start shows the home screen
    Given a fresh app data folder
    When I start the app
    Then I see the home screen with "Open a Karate project" and "Open folder…"
    And I see no "Recent folders" list
    And I see no toolbar, tree, details or bottom panel

  Scenario: Opening a folder from the home screen
    Given the home screen shows
    When I click "Open folder…" and pick the clean project
    Then the project view shows the clean project's tests
    And the toolbar shows its path

  Scenario: The last project opens at start without the home screen
    Given I opened the clean project and quit the app
    When I start the app again
    Then the toolbar shows the clean project's path before its tests are listed
    And the project view shows its tests

  Scenario: Opening with no project open shows the project view, not home (unit)
    Given no project is open
    When main reports it is opening a folder
    Then the window renders the project view for that folder, busy, and not the home screen

  Scenario: No home screen flash at start (by hand)
    Given the packaged app last had the clean project open
    When I start it five times
    Then I never see the home screen, even for a moment

  Scenario: A slow network folder does not hold up the home screen (unit)
    Given the saved list holds a folder whose check never answers
    When main sends the list
    Then it sends it within about 1 s, with that folder not greyed out

  Scenario: Removing the missing last project forgets it (unit)
    Given lastProject and the list's first row are the same missing folder
    When the row is removed
    Then settings.json has neither the row nor lastProject

  Scenario: A missing last project shows the home screen
    Given the saved list holds a folder that was deleted, which is also the last project
    When I start the app
    Then I see the home screen
    And its row is greyed out with "not found"
    And clicking the row opens nothing

  Scenario: A failed open from the home screen goes back to it (unit)
    Given no project is open
    When main opens a folder and the open is cancelled during the dry run
    Then the window shows the home screen with the notice "Cancelled"
    And the recent list and lastProject are unchanged

  Scenario: A dry-run error still opens the project
    Given the home screen shows
    When I open the broken project
    Then the project view shows its syntax error
    And the broken project is first in the recent list

  # R2: the home screen

  Scenario: Reopening a folder from the list
    Given I opened the clean project, then the tags project with "Open…"
    When I click "Home"
    Then the list shows the tags project, then the clean project
    When I click the clean project's row
    Then the project view shows the clean project's tests
    And clicking "Home" shows the clean project, then the tags project

  Scenario: The folder dialog starts next to the newest existing folder
    Given the saved list holds a deleted folder, then two existing folders
    When I start the app and click "Open folder…"
    Then the folder dialog starts in the parent of the first existing folder

  Scenario: Removing a row
    Given I opened the clean project and clicked "Home"
    When I click "Remove from list" on its row
    Then the row is gone and settings.json's recentProjects no longer holds it
    And settings.json still holds the project's entry under projects
    And the project's folder and its runs are untouched

  Scenario: The list works from the keyboard
    Given the saved list holds the clean project
    When I start the app, press Tab until its row has focus and press Enter
    Then the project view shows the clean project's tests

  Scenario: The home screen follows the dark theme (by hand)
    Given Windows is in dark mode
    When I start the packaged app with a saved list holding an existing and a missing folder
    Then the home screen, its rows, the greyed row and the focus ring are readable

  # R3: the Home button

  Scenario: Home closes the project and survives a restart
    Given I opened the clean project
    When I click "Home"
    Then I see the home screen with the clean project first
    And settings.json has no lastProject
    When I quit and start the app again
    Then I see the home screen

  Scenario: Home is disabled during a run
    Given I opened the clean project
    When I run the slow feature
    Then "Home" is disabled until the run finishes

  Scenario: Main refuses to close a busy project (unit)
    Given a run is in progress
    When the renderer calls closeProject
    Then the notice "A run is already in progress" shows and the project stays open

  Scenario: A reload after Home stays home
    Given I opened the clean project and clicked "Home"
    When the window reloads
    Then I see the home screen

  # R4: the saved list

  Scenario: The list keeps the 10 newest folders (unit)
    Given 10 folders were opened
    When an 11th folder opens
    Then the list holds the 11th first and drops the oldest

  Scenario: The same folder in another case is one entry (unit)
    Given the list holds "D:\work\api"
    When "d:\WORK\api" opens
    Then the list holds "d:\WORK\api" once, first

  Scenario: A broken list reads as what can be kept (unit)
    Given settings.json's recentProjects holds a number, a relative path, a duplicate and 12 valid paths
    When the list is loaded
    Then it holds the first 10 valid, distinct paths in order
    And nothing throws

  Scenario: A failed list save keeps the project open (unit)
    Given settings.json cannot be written
    When a folder opens
    Then the project is open and a notice says the recent list could not be saved

  # R5: calls from the renderer

  Scenario: A path not in the list is refused
    Given the saved list holds the clean project
    When the renderer calls openRecent with another existing folder
    Then the notice "Invalid request" shows and no project opens
```
