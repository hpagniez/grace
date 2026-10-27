# Console banner

Date 2026-10-27. Status: approved. Covers [feature](../features.md) F24.

## Intent

Before anything has run, the Console tab is an empty black box. Before the bottom tabs existed it was the only thing at the bottom of the window, a black rectangle with no meaning. The tabs made it clearer, but it still says nothing until the first run. The person wants it to feel alive: a console that reads like a terminal waiting for a command.

In dark mode it also melts into the window. The app background and the console are both `zinc-900`, so only a 1 px border and the tab strip separate the console from the tree and the details pane.

## Who and what success looks like

The walking skeleton's user, who opens a project and has not run anything yet, and later copies the console into a bug report.

Success:
- Open a project: the console greets you with the app, Karate and Java versions, the project folder and how many tests were found, then "Ready".
- Press Run all: "Starting Karate…" shows until Karate's first line arrives, then the output streams under the banner.
- Run again: the old output goes, the banner stays.
- In dark mode, the console reads as its own darker panel.

## Goals, non-goals, anti-goals

**Goals:** the requirements below.

**Non-goals:**
- A real prompt you can type into, or any command input.
- Showing the Karate command line the run uses (it is built in main, which F27 is changing).
- Reading the versions from the bundled files at run time, or asking Java or Karate for them.
- ANSI colours, filtering or search in the console (F22).
- Changing the light-mode console, which already stands out as a dark panel on the white window.
- A banner in the History or HTTP tabs.

**Anti-goals** (failure even if everything else works):
- A banner that names a Karate or Java version other than the one a packaged build bundles.
- A banner that pushes Karate's output away: following new lines (the console scrolls to the bottom unless the person scrolled up) keeps working.
- Grey text too faint to read on the dark panel.

## Constraints

- No new dependency. Solid 2 rules (AGENTS.md). No change to the main process, the preload or the IPC files (`src/main.ts`, `src/main/**`, `src/preload.ts`, `src/shared/ipc.ts`, `src/shared/events.ts`): F27 (`feature/home-screen`) is changing them at the same time.
- `scripts/fetch-vendor.mjs` pins the bundled versions: Temurin `21.0.12.1+1` and Karate `2.1.4`. Each version appears today only inside URL and file-name strings. `setupJre` skips the download when `vendor/jre/bin/java.exe` exists, so a dev machine can keep an older JRE after a pin bump. CI builds start from an empty `vendor/`.
- Text contrast on the console background meets WCAG AA (4.5:1).

### What the app does today

- `src/renderer/Console.tsx` renders `props.lines` in one scrolling `div` (`data-testid="console"`): `bg-zinc-900`, `font-mono`, `text-xs`, `text-zinc-200`, in both themes. ERROR lines are `text-red-400`, WARN lines `text-amber-400`. It follows new lines while scrolled within 20 px of the bottom.
- `BottomPanel` passes only `state.console` to it.
- `state.console` is emptied by `runStarted` and filled by `console` events, capped at `CONSOLE_LIMIT` lines (`src/renderer/reduce.ts`). Only a run's output reaches it; discovery dry runs print nothing there.
- `state.busy` is true while the Runner discovers or runs. `state.run.active` is true from `runStarted` to `runFinished`. A run starts with a discovery, so `busy` turns on before `run.active`.
- `state.project.root` is the open folder. `state.tree` is the discovered tree; `leaves(tree)` (`src/renderer/nodes.ts`) are its scenarios and outline rows. `state.error` holds a discovery error, which the tree shows in place of the tree.
- The tree's empty texts: "Open a Karate project folder" with no project, "No feature files found" with no tests.
- `package.json` holds the app version (`1.1.0` today).

## Approaches considered

For the look (shown as mockups, dark mode):
- A. One grey hint line, like History's "No runs yet".
- B. A prompt with a blinking cursor on a darker, sunken panel.
- C. A startup banner (versions, project, tests found, "Ready") on the same sunken panel. **Chosen by the person.**

For where the versions come from:
1. **The pins in `scripts/fetch-vendor.mjs`, built into the renderer at build time.** No main-process or IPC change, so no conflict with F27, and F33 gets one place to bump each version. Limit: in dev only, an old JRE can survive a pin bump. **Chosen by the person.**
2. Main reads `vendor/jre/release` and the version inside the Karate jar at start, and sends them over a new IPC call. Always true to the files, but it touches the files F27 changes and needs a small zip reader.
3. Ask Java and Karate at start. Dropped: an extra JVM at every launch, seen by the Java leak checks.

## Requirements

### R1. The banner

1. The console shows a banner above Karate's output, inside the same scrolling area, in the console's monospace font. It is always there, with or without a project.
2. Line 1: `karate-kid <app version> · Karate <karate version> · Java <java version>`, for example `karate-kid 1.1.0 · Karate 2.1.4 · Java 21.0.12.1+1`.
3. Line 2: `project  <state.project.root>`, or `project  none` with no project open.
4. Line 3, only with a project open:
   - `tests    <n> scenarios in <m> features`, where n is the number of `leaves(state.tree)` and m the number of feature nodes in the tree. One of each reads `1 scenario`, `1 feature`. The counts are of the whole discovered tree, not of the tag filter.
   - `tests    No feature files found` when the tree has no leaves.
   - `tests    finding tests…` while no tree has been found yet and the Runner is busy.
   - `tests    could not be listed: see the message in the tree` when `state.error` is set.
5. The labels (`project`, `tests`) are padded so the values line up.

### R2. The state line

1. Under the banner, while the console has no output lines, one state line says what the app is doing:
   - no project: `Open a Karate project folder`
   - busy and no run active (opening, refreshing, or the discovery before a run): `Finding tests…`
   - a run active: `Starting Karate…`
   - otherwise: `Ready. Press Run all, or ▶ on a test.`
2. The state line ends with a block cursor that blinks. It does not blink when the system asks for reduced motion. The cursor is hidden from screen readers.
3. Once the console has output lines, the state line is gone and the lines follow the banner.

### R3. Runs

1. A new run clears the previous run's output, as today, and keeps the banner.
2. The banner is never counted in `CONSOLE_LIMIT` and never trimmed by it.
3. Following new lines works as today, with the banner's height included in the scroll area.

### R4. The panel

1. In dark mode the console background is `zinc-950`, a shade darker than the window's `zinc-900`, with an inset shadow along its top edge, so it reads as a sunken panel.
2. Light mode keeps today's `zinc-900` console.
3. Line 1's app name is `sky-400`, `Ready.` is `emerald-400`, labels and hint text are `zinc-400`, values are `zinc-200`. ERROR and WARN lines keep their colours.

### R5. Versions at build time

1. `scripts/fetch-vendor.mjs` holds each bundled version once (Java `21.0.12.1+1`, Karate `2.1.4`), exports them, and builds its download URLs and file names from them. Its SHA-256 pins stay where they are.
2. The renderer build gets the app version (from `package.json`) and these two versions as build-time constants. The Vitest jsdom project gets the same constants, so component tests see the real values.
3. A packaged build's banner names exactly the versions `npm run vendor` downloaded for it.

## Design decisions already made

- The banner (option C) and its sunken dark panel (the person, from the mockups).
- The banner stays at the top; a new run clears only the output below it (the person).
- The versions are the pins in `scripts/fetch-vendor.mjs`, built in at build time (the person, approach 1).

The spec author's calls:
- The app version on line 1, next to Karate's and Java's: the banner is most useful in a bug report.
- The full Temurin version (`21.0.12.1+1`), not just `21`.
- The state line texts and when each shows (R2.1), including "Finding tests…" during the discovery before a run.
- The tests line counts the whole tree, not the tag filter (R1.4).
- The texts reuse the tree's wording ("Open a Karate project folder", "No feature files found").
- The colours in R4.3, picked for 4.5:1 or more on `zinc-950` and `zinc-900`.

## Left to the builder

- Whether the banner is its own component or part of `Console`, and how `BottomPanel` passes the state it needs.
- The constants' name and type declaration, and how the two configs share them.
- The blink keyframes and timing (the mockup used 1.1 s, stepped).
- `data-testid`s for the banner lines and the state line.

## Known risks

- In dev, after a JRE pin bump, the banner names the new Java while an older `vendor/jre` still runs, until `vendor/jre` is deleted and `npm run vendor` runs again. Packaged builds are not affected.
- When F27 merges, its home screen may change what the window shows with no project open, and so whether the "no project" banner is ever seen. Its texts may then need a pass.
- The labels are aligned with spaces in a monospace font; a font fallback without fixed widths would misalign them.

## Acceptance criteria

Scenarios run in Vitest (jsdom) unless marked **(e2e)**, which run in Playwright against the built app with a fresh app data folder.

```gherkin
Feature: Console banner

  # R1, R2: the banner and the state line

  Scenario: No project open
    Given no project is open
    Then the console shows "karate-kid <the version in package.json> · Karate 2.1.4 · Java 21.0.12.1+1"
    And the console shows "project  none"
    And the console shows no tests line
    And the state line says "Open a Karate project folder"

  Scenario: A project is open and nothing has run
    Given a project at "D:\work\orders-api" is open with 3 scenarios in 2 features
    And the Runner is not busy
    Then the console shows "project  D:\work\orders-api"
    And the console shows "tests    3 scenarios in 2 features"
    And the state line says "Ready. Press Run all, or ▶ on a test."
    And the state line ends with a blinking cursor

  Scenario: One scenario in one feature
    Given a project is open with 1 scenario in 1 feature
    Then the console shows "tests    1 scenario in 1 feature"

  Scenario: A project with no tests
    Given a project is open whose tree has no scenarios
    Then the console shows "tests    No feature files found"

  Scenario: Finding tests for the first time
    Given a project is being opened and no tree has been found yet
    Then the console shows "tests    finding tests…"
    And the state line says "Finding tests…"

  Scenario: Discovery failed
    Given a project is open and its discovery failed with an error
    Then the console shows "tests    could not be listed: see the message in the tree"

  Scenario: The cursor stays still with reduced motion
    Given the system asks for reduced motion
    Then the state line's cursor does not blink

  # R3: runs

  Scenario: A run starts and Karate has not printed yet
    Given a project is open
    When a run starts
    Then the state line says "Starting Karate…"
    When the first console line arrives
    Then the state line is gone
    And the line shows under the banner

  Scenario: The banner survives the line cap
    Given a project is open
    When more than CONSOLE_LIMIT console lines arrive
    Then the banner is still the first thing in the console
    And the console still follows the newest line

  Scenario: Open, run and run again (e2e)
    Given the sample project is open
    Then the console shows "Karate 2.1.4" and "Java 21.0.12.1+1"
    And the console shows the project folder and the sample project's scenario and feature counts
    And the console shows "Ready."
    When I press Run all and the run finishes
    Then the banner is above the run's output
    When I press Run all again and the run finishes
    Then the console shows the banner once
    And the first run's output is gone

  # R4: the panel

  Scenario: The console is darker than the window in dark mode (e2e)
    Given the app is in dark mode
    Then the console's background is darker than the window's background
    Given the app is in light mode
    Then the console's background is zinc-900

  # R5: versions at build time

  Scenario: The banner names the pinned versions
    Given scripts/fetch-vendor.mjs pins Java "21.0.12.1+1" and Karate "2.1.4"
    Then the banner's line 1 names those versions
    And fetch-vendor's download URLs and file names contain those versions
```
