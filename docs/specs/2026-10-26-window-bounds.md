# Window bounds

Date 2026-10-26. Status: approved. Covers [feature](../features.md) F41.

## Intent

karate-kid forgets its window. Whatever size and screen a person gives it, the next start opens it at 1200×800, centred on the main screen. A person with two screens who keeps the app on the second one moves and resizes it at every start. The bottom panel's height (F37) already survives a restart; the window around it does not. Seen in the [UI polish check](../checks/2026-10-26-ui-polish.md).

This spec makes the window come back with the size, position, screen and maximized state it had when the person closed it.

## Who and what success looks like

The walking skeleton's user, iterating on an API suite all day, often with the app on a second screen next to an editor.

Success:
- Close the app on the second screen, maximized or not, at any size; start it again: it is back where it was, the same size, maximized if it was.
- Unplug the second screen; start the app: it opens on the main screen, fully visible.
- Nothing about the window's place ever makes the app unreachable.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals:**
- A window state per project. One state for the app.
- Restoring a minimized or full-screen state. The app has no full-screen mode (no menu, no F11), and a window that starts minimized looks like a failed start.
- Remembering the maximized state of a window that was minimized from maximized (see R2.3).
- A "reset window" command or setting.
- Windows with different display scaling per screen. Electron has known bugs restoring a window onto a screen whose scaling differs from the main screen's; the person's screens share one scaling, so this cannot be checked by hand and gets no workaround (see Left to the builder).

**Anti-goals** (failure even if everything else works):
- A window that opens off every screen, or with its title bar out of reach. The app would run where nobody can reach it, and the single-instance lock would turn every new launch into a no-op.
- A white or wrong-theme frame at start (F39). Restoring must not show the window before it is painted.
- A crash, error dialog or lost setting because the state could not be saved or read.

## Constraints

- No new dependency.
- `contextIsolation`, `sandbox` and `nodeIntegration: false` stay as they are. Nothing changes in the renderer or the preload.
- Electron 45.0.0. Bounds are in device-independent pixels, as `BrowserWindow` and `screen` give them.
- e2e tests run on GitHub's `windows-latest` runner, whose single screen is small (1024×768: an 800 px window is cut down there, see `e2e/panel.spec.ts`). Tests must not assume a big screen or a second one.

### What the app does today

- `src/main.ts`, `createWindow`: `new BrowserWindow({ width: 1200, height: 800, minHeight: 480, show: false, backgroundColor, webPreferences })`. No `x`/`y`, so Electron centres the window on the main screen. The window shows through `reveal()` on `ready-to-show`, on `did-fail-load`, or after 5 s, whichever comes first; `reveal()` calls `show()` if the window is not visible yet (F39).
- `createWindow` runs once, after `app.whenReady()` (and on macOS `activate`, which does not apply on Windows), so the `screen` module is usable inside it.
- `second-instance` restores a minimized window, shows and focuses it.
- Quitting: `before-quit` is cancelled once while the runner is disposed, then `app.quit()` runs again and the window gets its `close` event. Closing the window with its X button closes it, then `window-all-closed` quits.
- `src/main/settings.ts` keeps one JSON object in `<userData>/settings.json`: `lastProject` and `projects` (tag filters, environment, run properties and encrypted secrets). Writes go through `update()`, a read-modify-write that keeps the other keys and throws rather than overwrite a file it could not read (a transient `EBUSY`). It writes with a plain `writeFileSync` over the file, so a write cut off midway leaves a broken file; `read()` takes a non-JSON file as empty, and the next save then drops every other key. Loads never throw: a missing or broken value reads as nothing saved.
- Electron API facts this spec relies on (Electron docs):
  - `win.getBounds()` of a maximized window is the maximized rectangle. `win.getNormalBounds()` is always the normal-state rectangle, whatever the state.
  - `win.maximize()` also shows a hidden window.
  - `resized` and `moved` fire once, at the end of a move or resize by the person (Windows, macOS). Moves and resizes done in code do not fire them.
  - `screen.getDisplayMatching(rect)` always returns a display, the nearest one even when the rectangle overlaps none.
  - `display.workArea` is the screen minus the taskbar.

## Approaches considered

1. **A pure placement function plus two settings helpers. No new dependency.** `settings.ts` loads and saves the state with its existing `update()`. A pure function takes the saved state and the screens' work areas and returns where the window goes; Vitest covers every screen layout with fake work areas. `createWindow` wires it in. **Chosen.**
2. The `electron-window-state` package. Resets a window that is not fully inside a screen instead of pulling it on (the opposite of R3.2), writes its own file instead of `settings.json`, and has not been updated in years.
3. The same logic inline in `main.ts`. The screen logic would be testable only end to end, and CI has one small screen, so no second-screen case could ever be automated.

## Requirements

### R1. What is saved

1. One window state for the app, under the key `window` in `settings.json`: `{ x, y, width, height, maximized }`. `x`, `y`, `width` and `height` are the window's normal-state bounds, never the maximized rectangle: `getNormalBounds()` while the window is maximized or minimized, `getBounds()` otherwise. The two are equal in the normal state (Electron docs); `getBounds()` keeps a window snapped with Windows Snap from being saved at its size before the snap. `maximized` is `isMaximized() && !isMinimized()`, so R2.3 holds whatever Windows reports for a minimized window.
2. The state is saved when the window closes (its `close` event), and also at the end of each move or resize by the person (`moved`, `resized`) and when it is maximized or unmaximized (`maximize`, `unmaximize`), so a crash or a Windows shutdown keeps the last state.
3. Saving goes through `update()` and keeps every other key of `settings.json`.
4. A failed save never stops the window from closing, never shows an error dialog, and never throws out of an event handler. It is logged to the console and the previous saved state stays.
5. Every save of `settings.json` (all callers of `update()`, not only the window) is atomic: it writes the whole file to `settings.json.tmp` with `flush: true` (fsync), then renames it over `settings.json`. A save cut off by a crash, a shutdown or a power loss leaves the previous `settings.json` whole, so it can never be read back as empty and then overwritten without `lastProject`, the tag filters, the properties and the secrets. A failed write or rename throws, as a failed write does today, and leaves `settings.json` as it was. A leftover `settings.json.tmp` is overwritten by the next save and never read.

### R2. What is restored

1. The state is read once, in `createWindow`, before the window is created. The window is created at the placed bounds (R3), hidden as today.
2. If the state says maximized, the window is maximized when it is revealed: `reveal()` calls `maximize()` and then `focus()` (`maximize()` shows a hidden window but does not focus it; `show()` does) instead of `show()`. It never shows before `ready-to-show` (or the F39 fallbacks), so it never shows unpainted. Un-maximizing it later gives back the saved normal bounds.
3. A window that was minimized when it closed opens normal, not minimized, at its saved normal bounds. If it was maximized before it was minimized, it also opens normal (R1.1 saves `maximized` false while minimized).
4. `second-instance` reveals a still-hidden window through the same `reveal()`, so a second launch during startup brings the window up maximized if the state says so, not normal. It still restores a minimized window and focuses it, as today.
5. A missing state, or one that is not an object of the shape of R1.1 (any of `x`, `y`, `width`, `height` not a finite number, `width` or `height` not above 0, `maximized` not a boolean), reads as nothing saved: the window opens as on a first start, 1200×800 centred on the main screen. A broken state never throws.
6. `minHeight: 480` stays. A saved height below 480 comes back as 480.

### R3. Where the window goes

The placement uses each connected screen's work area.

1. **On a screen.** If the saved bounds overlap at least one work area, the window goes to the work area it overlaps most (by overlapping area).
2. **Pulled fully onto it.** If the saved bounds stick out of that work area on any side, the window is moved, keeping its size, until it lies fully inside. If it is wider or taller than the work area, it is first shrunk to the work area's width or height.
3. **No screen.** If the saved bounds overlap no work area (the screen they were on is unplugged or switched off), the window opens as on a first start: 1200×800, centred on the main screen. If the state says maximized, it is maximized there, on the main screen.
4. Maximized windows: the window is created at its placed normal bounds, on its screen, and then maximized; Windows maximizes it on the screen it is on.
5. The placement is a pure function with no Electron import, so it can be tested with any screen layout.

## Design decisions already made

- One state for the app, in `settings.json` under `window`, through `settings.ts`'s `update()`.
- A window partly off its screen is pulled fully onto that screen, not left where it was.
- A window whose screen is gone opens as on a first start, on the main screen.
- No new dependency; a pure placement function.
- No workaround for mixed display scaling.
- Every save of `settings.json` becomes atomic (temp file, fsync, rename), because the window adds frequent writes to a file that holds secrets (R1.5).

The spec author's calls: saving on `moved`, `resized`, `maximize` and `unmaximize` as well as on `close` (R1.2); opening maximized on the main screen when the saved screen is gone (R3.3); opening normal after a minimized close, with an explicit `!isMinimized()` guard (R1.1, R2.3); `getBounds()` in the normal state so snapped windows keep their snapped size (R1.1); `focus()` after `maximize()` and `second-instance` going through `reveal()` (R2.2, R2.4); choosing the screen by largest overlap with work areas rather than `screen.getDisplayMatching` (R3.1), because `getDisplayMatching` never says "no screen" and matches full screen bounds, not work areas.

Resolved by the builder check (2026-10-26): the person chose atomic saves (R1.5) over saving only on close or a separate file; the calls above on `isMinimized`, snapped windows, `focus()` and `second-instance`; scenarios added for a minimized close, a failed save and a snapped window; the CI screen size stated as the runner's 1024×768 less its taskbar.

## Acceptance criteria

Every scenario is automated with Playwright against the built app, each with a fresh app data folder, unless it is marked **(by hand)** or **(unit)**. Unit scenarios run in Vitest: against `settings.ts` with a temp folder, or against the placement function with fake work areas. By-hand scenarios run on the packaged app on the person's two-screen machine and go into a dated record in `docs/checks/`.

"Work area" in the automated scenarios is the main screen's work area on the machine running the test, read from `screen.getPrimaryDisplay().workArea`. Every rectangle is placed relative to it and fits on a 1024×768 screen less its taskbar, the size of the CI runner's screen.

```gherkin
Feature: Window bounds

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  # R1, R2: save and restore

  Scenario: The window comes back where it was
    Given the app is open
    When I move the window to 40 px right and 30 px down from the work area's corner and size it 700×500
    And I close the app and start it again with the same app data folder
    Then the window is at 40 px right and 30 px down from the work area's corner, 700×500
    And it is not maximized

  Scenario: A maximized window comes back maximized
    Given the app is open with the window at 700×500
    When I maximize the window
    And I close the app and start it again with the same app data folder
    Then the window is maximized
    When I unmaximize it
    Then it is 700×500 at the place it had before

  Scenario: A move by the person is saved before the app closes
    Given the app is open
    When the window gets a "moved" event at 700×500
    Then settings.json holds that window state, maximized false

  Scenario: The saved state keeps the other settings (unit)
    Given settings.json holds a last project and a project's tag filter
    When the window state is saved
    Then settings.json still holds the last project and the tag filter

  Scenario: A window closed while minimized opens normal
    Given the app is open with the window at 700×500
    When I maximize the window, then minimize it
    And I close the app and start it again with the same app data folder
    Then the window is visible, not minimized and not maximized, at 700×500

  Scenario: A failed save does not stop the app from closing
    Given settings.json is a folder, so it cannot be read or written
    When the app starts
    Then the window opens as on a first start
    When I move the window and close the app
    Then the app exits with no error dialog

  Scenario: A cut-off save leaves the old settings whole (unit)
    Given settings.json holds a last project and a project's tag filter
    When a save writes its temp file and then fails before the rename
    Then settings.json still holds exactly what it held before
    And the next save succeeds and keeps the last project and the tag filter

  Scenario: A short saved window is held at the minimum height
    Given settings.json holds a window state 700×300 inside the work area
    When the app starts
    Then the window is 480 px tall

  Scenario Outline: A broken saved state opens the window as on a first start
    Given settings.json holds the window state <state>
    When the app starts
    Then the window has the bounds of a first start with a fresh app data folder
    And it is not maximized

    Examples:
      | state                                                          |
      | "abc"                                                          |
      | { x: 10, y: 10, width: -5, height: 500, maximized: false }     |
      | { x: "a", y: 10, width: 700, height: 500, maximized: false }   |
      | { x: 10, y: 10, width: 700, height: 500 }                      |

  # R3: where the window goes

  Scenario: A window partly off the screen is pulled onto it
    Given settings.json holds a 700×500 window whose right half sticks out past the work area's right edge
    When the app starts
    Then the window is 700×500 and its right edge is the work area's right edge
    And its top is where it was saved

  Scenario: A window bigger than the screen is shrunk to fit
    Given settings.json holds a window 500 px wider than the work area, at the work area's corner
    When the app starts
    Then the window is as wide as the work area and lies fully inside it

  Scenario: A window on a screen that is gone opens on the main screen
    Given settings.json holds a 700×500 window at x -30000, y -30000
    When the app starts
    Then the window has the bounds of a first start with a fresh app data folder

  Scenario: A maximized window on a screen that is gone opens maximized on the main screen
    Given settings.json holds a maximized window at x -30000, y -30000
    When the app starts
    Then the window is maximized on the main screen

  Scenario Outline: Placement on two screens (unit)
    Given a main screen with work area 0,0 1920×1040 and a second screen with work area 1920,0 2560×1400
    When the saved bounds are <saved>
    Then the window goes to <placed>

    Examples:
      | saved                       | placed                         |
      | 2000,100 1200×800           | 2000,100 1200×800              |
      | 3900,100 1200×800           | 3280,100 1200×800              |
      | 1800,100 1200×800           | 1920,100 1200×800              |
      | 2000,900 1200×800           | 2000,600 1200×800              |
      | 2000,100 3000×1600          | 1920,0 2560×1400               |
      | 5000,100 1200×800           | first-start bounds              |
      | 100,-900 1200×800           | first-start bounds              |

  # By hand, on the packaged app

  Scenario: Two screens (by hand)
    Given the packaged app on a machine with two screens
    When I move the window to the second screen, resize it and close the app
    And I start it again
    Then it opens on the second screen at the same place and size, without a white or wrong-theme frame
    When I maximize it on the second screen, close the app and start it again
    Then it opens maximized on the second screen
    When I unmaximize it
    Then it goes back to the size and place it had before I maximized it

  Scenario: The second screen is gone (by hand)
    Given the packaged app was closed on the second screen, maximized, then again not maximized
    When I disconnect the second screen (or set Windows to show only on screen 1) and start the app each time
    Then it opens fully visible on the main screen, maximized the first time, 1200×800 centred the second time

  Scenario: Partly off the screen (by hand)
    Given the packaged app's window is dragged so half of it sticks out past the second screen's outer edge
    When I close the app and start it again
    Then it opens fully on the second screen, the same size

  Scenario: A snapped window (by hand)
    Given the packaged app's window is snapped to the left half of a screen with Win+Left
    When I close the app and start it again
    Then it opens at the size and place it had while snapped
```

"First-start bounds" in the unit table means the function returns no position, so `createWindow` passes only `width: 1200, height: 800` and Electron centres the window on the main screen, as today.

## Left to the builder

- File and function names (for example `src/main/window.ts` with a `placeWindow` function and `src/main/window.test.ts`), and the names of the `settings.ts` helpers.
- How the e2e tests get the main work area (`screen.getPrimaryDisplay().workArea` through `app.evaluate`), how they write `settings.json` before a start, and how they fire `moved` (`BrowserWindow.emit("moved")` after `setBounds`, since moves in code fire no `moved`).
- Whether the `close` save uses `close` or `closed` (the bounds must still be readable: `close`).
- Rounding: Electron's bounds are whole numbers; the placement keeps them whole. e2e checks of bounds may allow 1 px, like `e2e/panel.spec.ts`.
- How `createWindow` gets the app data folder (it is a `const` inside the `whenReady` callback today).
- Existing e2e specs that restart the app with the same app data folder (`panel`, `env`, `history`, `open`, `tags`) now get their window back too. They may be adjusted where the restored window changes what they measure, keeping what each one checks.
- Mixed display scaling: if the person ever reports a wrong size after restoring onto a screen with a different scaling, the usual Electron workaround is a second `setBounds` after creation. Not built now.
