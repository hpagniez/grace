# App check: premium look and feel (#107)

Date: 2026-10-29
Branch: `feature/premium-ui` at bba0935b (the code is the same as at ae2567fe; the commits since then only retake screenshots)
Machine: Windows 11 Pro 10.0.26300 (x64), AMD Ryzen 7 5800X, 32 GB, NVIDIA GeForce RTX 4060 Ti, Windows in dark mode, Node 24.15.0, npm 11.16.0, Electron 45.0.0, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [Premium look and feel](../specs/2026-10-29-premium-ui.md), with the [mockup](../specs/2026-10-29-premium-ui-mockup.png) and the [plan](../plans/2026-10-29-premium-ui.md).

The branch restyles every screen to design tokens (IBM Plex, Lucide, the light "parchment sheet" and the dark "logbook" themes), adds styled tooltips and Settings › Appearance (Follow Windows, Light, Dark), and restyles the app-updates screens that came from `develop` (#110). This record holds the before and after screenshots of R22, the automated results, and what was checked by hand on the packaged app.

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm run screens` (`e2e/screens.shots.ts`, 1280 × 800, light and dark) | `2 passed (37.4s)`. Wrote the 34 PNGs of `2026-10-29-premium-ui/after/` (17 screens × 2 themes). |
| `npm test` | First run: `Test Files  1 failed \| 87 passed (88)` and `Tests  2 failed \| 1266 passed (1268)`. The 2 failures are `src/main/updates/switch.test.ts`, EPERM on `rmSync` of its temp folder under the parallel run (the updater flake of `develop`). That file with `startup.test.ts` alone: `Test Files  2 passed (2)`, `Tests  63 passed (63)`. |
| `npm run typecheck` | `tsc --noEmit`, no output, passed. |
| `npm run lint` | oxlint reported nothing; oxfmt: `All matched files use the correct format.` |
| `npm run e2e` | `159 passed (8.0m)`. Nothing on stderr. |
| `npm run make` | Passed. `Grace-win32-x64-1.0.0-rc.4.zip`, 218,717,090 bytes (208.6 MB). Unzipped: `Grace.exe` (product and file version `1.0.0-rc.4`), `resources/jre/bin/java.exe`, `resources/karate.jar`, `resources/app-files.json`, `resources/licenses/` (181 files). |

The after shots were looked at one by one, in both themes, against the spec and the mockup. Nothing was off: tokens, type, icons, the sheet (light) and flush panels (dark), the filter chips, the status marks, the dialogs and the update screens match R1 to R21. Differences a reader may notice are in the deviations below, not defects.

How the shots were taken: `SHOTS_DIR=<dir> npm run screens` packages the app and drives it with Playwright, one window of 1280 × 800 per state, the pointer parked off every control. Paths in the shots are temp folders of the git-ignored `out/`, so they hold no user name. Their random suffixes, the clock times and the counts of the `running` shot differ between before and after.

- 26 of the before shots (13 screens) were taken from the code of `develop` before the first build commit (`b26768bb`).
- The 8 before shots of the app-updates screens (Settings, the update, roll back and progress dialogs) were taken from `develop` itself: a packaged build of `60c1e1b5` (`develop` at the merge), in a temporary worktree outside this one, with this branch's harness copied in and nothing committed there. An earlier take on this branch showed develop's raw palette classes compiled to nothing, so it was not develop's real look.

## Before and after

Click a pair to compare. Every shot is 1280 × 800. The 1280 px width is where the toolbar still shows its button labels (see the deviations).

| Screen | Light before | Light after | Dark before | Dark after |
|---|---|---|---|---|
| Home, first run | [before](2026-10-29-premium-ui/before/light-home-first-run.png) | [after](2026-10-29-premium-ui/after/light-home-first-run.png) | [before](2026-10-29-premium-ui/before/dark-home-first-run.png) | [after](2026-10-29-premium-ui/after/dark-home-first-run.png) |
| Home, recent folders (one missing) | [before](2026-10-29-premium-ui/before/light-home-recent.png) | [after](2026-10-29-premium-ui/after/light-home-recent.png) | [before](2026-10-29-premium-ui/before/dark-home-recent.png) | [after](2026-10-29-premium-ui/after/dark-home-recent.png) |
| A notice (folder not found) | [before](2026-10-29-premium-ui/before/light-notice.png) | [after](2026-10-29-premium-ui/after/light-notice.png) | [before](2026-10-29-premium-ui/before/dark-notice.png) | [after](2026-10-29-premium-ui/after/dark-notice.png) |
| Project, idle | [before](2026-10-29-premium-ui/before/light-project-idle.png) | [after](2026-10-29-premium-ui/after/light-project-idle.png) | [before](2026-10-29-premium-ui/before/dark-project-idle.png) | [after](2026-10-29-premium-ui/after/dark-project-idle.png) |
| Filter bar on (an included and an excluded tag) | [before](2026-10-29-premium-ui/before/light-filter-on.png) | [after](2026-10-29-premium-ui/after/light-filter-on.png) | [before](2026-10-29-premium-ui/before/dark-filter-on.png) | [after](2026-10-29-premium-ui/after/dark-filter-on.png) |
| A run in progress | [before](2026-10-29-premium-ui/before/light-running.png) | [after](2026-10-29-premium-ui/after/light-running.png) | [before](2026-10-29-premium-ui/before/dark-running.png) | [after](2026-10-29-premium-ui/after/dark-running.png) |
| Finished run, a failed test selected | [before](2026-10-29-premium-ui/before/light-finished-failed.png) | [after](2026-10-29-premium-ui/after/light-finished-failed.png) | [before](2026-10-29-premium-ui/before/dark-finished-failed.png) | [after](2026-10-29-premium-ui/after/dark-finished-failed.png) |
| A group selected | [before](2026-10-29-premium-ui/before/light-group-selected.png) | [after](2026-10-29-premium-ui/after/light-group-selected.png) | [before](2026-10-29-premium-ui/before/dark-group-selected.png) | [after](2026-10-29-premium-ui/after/dark-group-selected.png) |
| Console | [before](2026-10-29-premium-ui/before/light-console.png) | [after](2026-10-29-premium-ui/after/light-console.png) | [before](2026-10-29-premium-ui/before/dark-console.png) | [after](2026-10-29-premium-ui/after/dark-console.png) |
| HTTP, call details | [before](2026-10-29-premium-ui/before/light-http-call.png) | [after](2026-10-29-premium-ui/after/light-http-call.png) | [before](2026-10-29-premium-ui/before/dark-http-call.png) | [after](2026-10-29-premium-ui/after/dark-http-call.png) |
| History | [before](2026-10-29-premium-ui/before/light-history.png) | [after](2026-10-29-premium-ui/after/light-history.png) | [before](2026-10-29-premium-ui/before/dark-history.png) | [after](2026-10-29-premium-ui/after/dark-history.png) |
| Run properties dialog | [before](2026-10-29-premium-ui/before/light-properties-dialog.png) | [after](2026-10-29-premium-ui/after/light-properties-dialog.png) | [before](2026-10-29-premium-ui/before/dark-properties-dialog.png) | [after](2026-10-29-premium-ui/after/dark-properties-dialog.png) |
| Clear history dialog | [before](2026-10-29-premium-ui/before/light-clear-history-dialog.png) | [after](2026-10-29-premium-ui/after/light-clear-history-dialog.png) | [before](2026-10-29-premium-ui/before/dark-clear-history-dialog.png) | [after](2026-10-29-premium-ui/after/dark-clear-history-dialog.png) |
| Settings, Updates | [before](2026-10-29-premium-ui/before/light-settings.png) | [after](2026-10-29-premium-ui/after/light-settings.png) | [before](2026-10-29-premium-ui/before/dark-settings.png) | [after](2026-10-29-premium-ui/after/dark-settings.png) |
| Update available dialog | [before](2026-10-29-premium-ui/before/light-update-dialog.png) | [after](2026-10-29-premium-ui/after/light-update-dialog.png) | [before](2026-10-29-premium-ui/before/dark-update-dialog.png) | [after](2026-10-29-premium-ui/after/dark-update-dialog.png) |
| Roll back dialog | [before](2026-10-29-premium-ui/before/light-switch-dialog.png) | [after](2026-10-29-premium-ui/after/light-switch-dialog.png) | [before](2026-10-29-premium-ui/before/dark-switch-dialog.png) | [after](2026-10-29-premium-ui/after/dark-switch-dialog.png) |
| Update progress dialog | [before](2026-10-29-premium-ui/before/light-update-progress.png) | [after](2026-10-29-premium-ui/after/light-update-progress.png) | [before](2026-10-29-premium-ui/before/dark-update-progress.png) | [after](2026-10-29-premium-ui/after/dark-update-progress.png) |

Not in the table:
- Startup screen: there is no before shot (the old one was a spinner on a plain colour). After, in a real window with its title bar: [light](2026-10-29-premium-ui/by-hand/startup-light.png), [dark](2026-10-29-premium-ui/by-hand/startup-dark.png).
- Settings › Appearance did not exist before. After: [Light chosen](2026-10-29-premium-ui/by-hand/appearance-light.png), [Dark chosen](2026-10-29-premium-ui/by-hand/appearance-dark.png) (taken from the packaged app, below).

## By hand (done by an agent on the packaged app)

Method: `npm run make`, the zip unzipped into a folder under `out/`, `Grace.exe` started with `GRACE_USER_DATA` on a fresh profile whose `settings.json` was written first (the saved appearance, the last project a copy of the `clean` fixture). Keys were sent with SendKeys only while Grace held the foreground (the script stops otherwise; it stopped once, nothing was sent elsewhere). The window, title bar included, was captured from the screen with `CopyFromScreen`. Frames taken before the window shows its ground colour were thrown away unsaved: while Windows plays its show animation it blends the desktop behind the window.

1. Startup screen and title bar with a saved appearance that differs from Windows (spec: Dark over a light Windows; R6.3, R8.3).
   - This machine's Windows is in dark mode and the mode is a system setting, so the exact scenario could not be set up. The mirror was run: a saved **Light** over a dark Windows.
   - Packaged `Grace.exe`, 229 frames at about 12 ms: from the first opaque frame the body is `#F3EFE6` (the light ground) and the title bar is light (`#F5F2F0` fading to `#F9F2EA`). No dark frame, no white frame. Result: passed.
   - The startup screen itself lasts less than Windows' show animation here: the window is shown at first paint and the page has mounted before the animation ends, so no frame of the packaged exe shows it (a slow machine may). To see it, the built app (`electron .` on the `.vite` build; the packaged exe refuses a changed asar, its integrity fuse is on) was started with the renderer script renamed, so the page never mounts. In a real window with its title bar: the logo, "Grace" and the spinner turning on `#F3EFE6` with a light title bar (saved Light, Windows dark), and on `#151821` with a dark title bar (saved Dark): [light](2026-10-29-premium-ui/by-hand/startup-light.png), [dark](2026-10-29-premium-ui/by-hand/startup-dark.png). Result: passed.
2. Tab through every screen shows the focus ring (R10.1). Packaged app, Light on Home and Settings, Dark on the project screen and the dialog.
   - Home: Open folder…, each recent row and its remove button (with its tooltip on keyboard focus), the version and Settings buttons of the status bar. [Example](2026-10-29-premium-ui/by-hand/ring-home-light.png).
   - Project screen, forwards from the start: Home, Env, Threads, Properties…, Refresh, Open…, Run all, the three tag chips, Hide non-matching, then each tree chevron, row and run button. Backwards from the start: Settings, the version, the HTTP, History and Console tabs, the splitter, the run buttons and rows. Report, Re-run failed and Cancel are disabled before a run and Tab skips them. [Example](2026-10-29-premium-ui/by-hand/ring-project-dark.png).
   - Run properties dialog: the dialog itself, All environments, the new-environment field, Add, Add property, Save; Escape closes it and focus goes back to Properties…. [Example](2026-10-29-premium-ui/by-hand/ring-dialog-dark.png).
   - Settings: Back, Updates, Appearance, Check for updates, the channel radios and the two checkboxes. [Example](2026-10-29-premium-ui/by-hand/ring-settings-light.png).
   - Every stop shows a 2 px ring, inset on rows. Result: passed for the screens listed. Not walked by hand: the update, roll back and progress dialogs, the Clear history dialog, and the History and HTTP rows (they need an update feed or a finished run). The e2e tests "Every control shows the focus ring in light mode" and "... in dark mode" walk the Tab stops of the project screen and the status bar.
3. Settings › Appearance with the keyboard (R8). Saved Light over a dark Windows: Appearance shows Light checked. Down arrow to Dark: the page, the window background and the title bar turn dark at once, with Settings still open. Up twice to Follow Windows: the window stays dark (Windows is dark) and `settings.json` ends with `"appearance":{"theme":"system"}`. Result: passed.
4. Closing the window: no `Grace.exe` or bundled `java.exe` left; no `running.pid` or `run-args.txt` in the profiles. Result: passed. Cleanup: the unzipped app, the profiles and the project copies were deleted.
5. Seen on the way: the default window is 1200 px wide, so the packaged app opens with the icon-only toolbar (below 1280 px). Hovering or focusing a button shows its tooltip.

### Still needs a person

- Windows "Show animations" off (Settings › Accessibility › Visual effects) stops every motion in the app (R5.3). It is a system setting and was left alone. The CSS rule is covered by the e2e test "Reduced motion stops every animation", which emulates `prefers-reduced-motion`.
- Dark saved over a **light** Windows, start-up (R6.3): the mirror was run here, the exact case needs a machine in light mode.
- A person's eye on the look, in both themes, at the screen's real scale: the shots against the mockup, the first frames of a start on a slower machine, the feel of hover, press, tooltip and dialog motion.
- The Tab walk of the update, roll back, progress and Clear history dialogs, and of the History and HTTP rows after a run.

## Deviations from the spec decided during the build

- Tags in tree rows are 20 px chips, not 24 px: 24 px bordered chips in 24 px rows with no gap touched the rows above and below (R15.1, R16.1).
- The toolbar shows icons only below 1280 px (the buttons keep their accessible names and a tooltip), and the window's minimum width is 760 px so Run all and Cancel stay on screen. The default 1200 px window is therefore icon-only (R13).
- "Not run yet" shows only when the project has no run history (R14.3).
- Tooltips show only for text that is actually cut, and close on a press of their target and on a scroll that moves their target, not on any scroll (R9.4).
- The dialogs of app-updates follow R19.1's button order (the primary or destructive button first, Cancel last), and the roll back confirmation uses the danger style (R19.1, R21).
- The Back button of Settings has the accessible name "Back" (the old name carried the arrow character).
- The console follows the theme (R18.3), which replaces `e2e/console.spec.ts`'s check that it matches the dark background.
- Non-matching tree rows are muted, not 40 % opacity, which dropped their text to 1.9:1 (R1.6, R15.2).
