# App check: home screen (F27)

Date: 2026-10-27
Branch: `feature/home-screen` at c7d5c59 (the code under test; this record is the commit after it)
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24.15.0, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [Home screen](../specs/2026-10-27-home-screen.md).

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 702/702 passed in 43 files, run with a temporary config that adds `.claude/**` to the excludes. Plain `npm test` on this machine also loads the tests of the other branches' worktrees under `.claude/worktrees/`: 10 of their renderer test files fail there (`__KK_VERSIONS__ is not defined`), and the run takes 18 minutes instead of 4. None of the repo's own files fails |
| `npm run e2e` | 114/114 passed, including the new `e2e/home.spec.ts` (13). `e2e/panel.spec.ts` (12) changed: its helper now opens a sample project, because the bottom panel and splitter belong to the project view and a first start now shows the home screen. `e2e/open.spec.ts` (1 line) expects the home screen on a first start |
| `npm run typecheck`, `npm run lint` | passed |

Not covered by the automated part: the three by-hand checks below (the first paint, the dark theme, the light theme against the mockup), a folder on a network share that is slow or gone (the 1 s cap on the existence check is a unit test with a stub that never answers), and a second screen.

## By hand (to do by a person)

Build with `npm run package`, then start `out\karate-kid-win32-x64\karate-kid.exe`. Write the result under each step: passed, or what you saw instead.

### A. Dark theme

Setup: Windows in dark mode (Settings, Personalisation, Colours). Start the app, open a Karate project folder A, then open a second folder B with `Open…`, and close the app. Rename B's folder in Explorer. Start the app: B was the last project but is gone, so the home screen shows with B greyed out.

1. The heading, the line under it, the `Open folder…` button, the "Recent folders" heading, both rows and the `×` buttons are readable on the dark background.
   - Result: not checked yet.
2. B's row is greyed out and shows "not found" in a colour that stays readable. Clicking it does nothing. A's row is not greyed and opens A.
   - Result: not checked yet.
3. Press Tab. The focus ring shows on `Open folder…`, then on A's row, then on its `×`, and is visible on each.
   - Result: not checked yet.

### B. No home screen flash at start

Setup: close the app with the X button while a project is open, so it is the last project. Pick a project whose dry run takes a moment (a few dozen feature files, or a cold start after a reboot).

1. Start the app five times in a row, closing it each time while the project is still open. Each time the window appears with the project view (its path in the toolbar, the tree discovering or listing tests). The home screen never shows, even for a moment, and the window is never blank white or black.
   - Result: not checked yet.

### C. Light theme against the mockup

Setup: Windows in light mode. Start the app with a saved list (the setup of part A works).

1. Compare the home screen with part 1 of the [mockup](../specs/2026-10-27-home-screen-mockup.png): a centred column about 540 px wide, the heading, the line, the primary button, "Recent folders" and the list, each row with the folder's name and its full path, and `×` at the row's right end. The colours may differ (the mockup is a sketch).
   - Result: not checked yet.
2. Open a project and compare the toolbar with part 2 of the mockup: `Home` is the first item, left of the project path. Click it: the home screen shows, the closed project is the first row, and the window is the same as before the project was opened.
   - Result: not checked yet.
