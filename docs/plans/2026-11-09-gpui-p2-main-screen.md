# Plan: GPUI migration, phase P2 main screen

Date 2026-11-09. Issue [#281](https://github.com/hpagniez/grace/issues/281), under [#278](https://github.com/hpagniez/grace/issues/278). Spec: the [migration guide](../specs/2026-11-08-gpui-migration.md) (Architecture, R1 to R8, feature map P2), [ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md). Checklist: [parity](../specs/2026-11-08-gpui-migration-parity.md), the P2 rows: 241 spec scenarios, the P2 e2e files and the renderer tests of P2's views.

P2 puts Grace's main screen on `grace-core`: the window, the home screen, the toolbar, the tree, the filter bar, environments and properties, threads, run and cancel, progress, details, the console and the bottom panel. It ends when the P2 rows are ticked and `npm run bench:gpui` on the GPUI build is within every limit it shares with Electron Grace. The Electron app does not change.

## Scope

| Electron Grace | GPUI Grace | PR |
|---|---|---|
| `reduce.ts`, `store.ts` | `grace-core::state` | 1 |
| `main.ts` (window, single instance, startup, close, dialogs), `App.tsx`'s frame | `grace::app` (the `Grace` entity), `grace::window`, `grace::instance`, `grace::seams` | 1 |
| `Home.tsx`, `EmptyState.tsx`, `Notice.tsx` | `grace::home`, `grace::ui::{empty_state, notice}` | 1 |
| `Toolbar.tsx`, `Progress.tsx`, `StatusBar.tsx`, `narrow.ts` | `grace::{toolbar, progress, status_bar}` | 2 |
| `Tree.tsx`, `treeRows.ts`, `nodes.ts`, `VirtualList.tsx`, `virtual.ts`, `FilterBar.tsx` | `grace-core::rows`, `grace::{tree, filter_bar}` on GPUI's `uniform_list` | 3 |
| `EnvPicker.tsx`, `PropertiesDialog.tsx`, `ImportStep.tsx`, `importPlan.ts`, `FieldMessage.tsx`, `Threads.tsx` | `grace::{env_picker, properties, threads}`, `grace-core::import_plan` | 4 |
| `Details.tsx` (P2's part), `Console.tsx`, `Banner.tsx`, `bannerText.ts`, `consoleFind.ts`, `BottomPanel.tsx`, `Splitter.tsx`, `panelHeight.ts`, `Settings.tsx` (appearance) | `grace::{details, console, bottom_panel, settings}`, `grace-core::{banner, console_find, panel_height}` | 5 |
| The P2 e2e files not yet ported, the exit check | `crates/grace-e2e/tests/*`, `docs/checks/` | 6 |

What P2 leaves to later phases, as the feature map says: the HTTP tab, history, trends, flaky proof, reports, the data-driven view, failures and step comments (P3); Build, the pom, sub-folders, watch, the debugger and the Variables tab (P4); updates and the Updates section of Settings (P5). A P2 view that shows one of those leaves its place empty until then, never a control that does nothing.

## Porting rules

P1's rules hold ([P1 plan](2026-11-09-gpui-p1-core.md#porting-rules)): same fixtures, same strings, same files, platform seams, light, checks. And for views:

1. **Logic in `grace-core`, drawing in `grace`.** A view's rules that need no window (rows of the tree, the console's find, the banner's text, the panel's height) are plain Rust in `grace-core`, their Vitest tests ported test for test. A view in `grace` draws the state and calls `Grace`'s methods; its component tests become headless `#[gpui::test]`s on a fake runner.
2. **Same words, same names.** Every text, tooltip and accessible name stays word for word (guide R1.2): the e2e tests find controls by them.
3. **Accessibility by hand.** Every control has `.id`, `.role` and a name; the tree check (`Node::unnamed_controls`) runs on every view an e2e test opens.
4. **Idle.** No timer or animation while no run is going (ADR 0030, #170). Run updates arrive on a channel the window waits on: nothing wakes while idle.
5. **Seams.** Dialogs, opening a path and the clipboard go through `grace::seams`; the `e2e` build answers them from `GRACE_E2E_SEAMS` and records what was asked, as the Playwright tests stubbed `dialog.showOpenDialog`.
6. **Port, do not transcribe** (P1 rule 8): every change from Electron Grace's design goes in [Redesigns](#redesigns), in the PR that makes it.

## Redesigns

| Module | Change from Electron Grace | Why | Parity kept by |
|---|---|---|---|
| `state` | The renderer's store is `grace-core::state::AppState`, owned by the `Grace` entity; `reduce` is a method; `Record<string, …>` maps are `HashMap`s, the console a `VecDeque` | No IPC and no store proxy: the window's state lives with the runner that feeds it; the console drops its oldest lines without copying 10,000 | The 45 `reduce` tests and the 8 store tests, ported in `state::tests` |
| `state` | The parity list put `reduce` in `grace::ui::reduce`; it is in `grace-core::state` | Guide, Architecture ("State"): logic that needs no window lives in `grace-core`, tested by plain `cargo test` | As above |
| `app` | The 38 IPC calls are methods of the `Grace` entity; each runs the runner's blocking call on GPUI's background executor. The `main.ts` checks on what the renderer sent (`Invalid request` for a bad key, run id, version or path) go: a method takes typed values only the views make | No untrusted renderer: one process, typed calls | Each method's view test; `E-home-14` ("A path not in the list is refused") keeps the runner's own check |
| `app` | The runner's events reach the window through a channel; a foreground task waits on it, applies a batch with `reduce` and notifies once | `batch.ts`'s `setImmediate` tick and IPC are gone; nothing wakes while no event comes (ADR 0030) | `state` tests; the run e2e tests |
| `state` | The thread count is a `ThreadsView { count, generation }`; `generation` grows with each `threads` event | `reduce.ts` gave the box a fresh object per event so it reset even on the count it held; a Rust value has no identity to compare | `added::each_threads_event_is_a_new_generation_even_with_the_same_count` |
| `state` | `AppState::shown_notice` and `home` hold `App.tsx`'s two rules (home shows when nothing is open or opening; the open's error shows on home as a notice) | Testable without a window | `added::the_home_screen_shows_the_opens_error_as_a_notice`, `grace::home::tests::a_missing_folder_shows_its_reason_on_home` |
| `app` | At start the window asks the runner once: the last project's open, or `replay()` when there is none. No `ready` handshake | Electron's window could miss events sent before it subscribed; here the channel holds them until the window's task reads them | `app::tests::a_first_start_says_nothing_is_open_and_sends_the_recent_folders`, `E-home-03` |
| `window` | The bounds are saved 500 ms after the window last moved or changed size, and on close | GPUI tells every change of bounds, not the end of a move (`moved`, `resized`); one write per move, not one per pixel | `window::tests` (placement); the save by hand on Windows, PR 6 |
| `window` | `startup.spec.ts`'s static startup screen is gone: the window draws the ground alone until the runner says what is open | No page to load: the first frame is GPUI's, in the theme (#90) | `E-home-03` sees no frame of the home screen before the project |
| `theme` | The saved Appearance is a GPUI global read before the window opens; `GRACE_THEME` still pins the theme for tests and screenshots | Electron set `nativeTheme.themeSource` before the window existed; the same order here | `theme::tests::picks_the_theme_from_the_system_unless_pinned` (the saved Appearance cases added) |
| `instance` | One instance per data folder is a lock on `<data folder>/instance.lock` (`File::try_lock`), and the second start tells the first through a loopback socket whose port the lock file holds | Electron's `requestSingleInstanceLock`; the same rule on Windows and Linux with only `std` | `instance::tests` (two locks on one folder, a message reaches the first) |
| `seams` | Open… on Windows uses the Shell's `IFileOpenDialog` with `SetFolder`, not GPUI's `prompt_for_paths`, which has no starting folder | HS-12 and `E-open-09`: the dialog starts next to the newest existing recent folder | `seams::tests`; `E-home-08`, `E-open-09` through the `e2e` seam. On Linux GPUI's portal dialog opens where the desktop chooses (Linux is not released, guide R5) |
| `app` | `Grace` holds its `AppState` in an `Rc`: a view takes a clone of the `Rc` to draw, and an event batch copies the state only while a view still holds the old one, which none does between frames | Each redraw would otherwise copy the whole state, the 10,000 console lines and the tree included | Every view test; `grace::app::tests` |
| `tooltip` | `cut_middle` cuts a line's start and keeps its end whole: the head is an ellipsized block beside the tail, the tooltip only while the head is cut. GPUI's own `TruncateMiddle` is not used | `Cut.tsx`'s `Middle` keeps an exact end (the name's last 12 characters, the path's project folder); `TruncateMiddle` balances both ends | `toolbar::tests::the_project`, `status_bar::tests` |
| `progress` | The segments ease to their width on a spring (GPUI's `with_spring`), not a 300 ms CSS transition; the hold Chromium needed beside a starting spinner is gone | A spring keeps its speed when a new result moves its end, so a fast run never restarts the ease; GPUI starts every animation in the frame it is asked for | `progress::tests::the_bar_r14_1` (LV-06 by its spring, the 2 px gaps) |
| `progress` | The stripe and the turning icon run only while they show, and the clock ticks once a second only during a run that is not paused | ADR 0030: nothing animates or ticks while idle; `setInterval` and CSS animations ran the same way | `progress::tests::the_clock` |
| `progress` | The bar has no `aria-busy`: while it waits it has no values, as Electron's did, which a screen reader reads as busy | GPUI 0.3.11 writes no busy state to AccessKit | `run_all_with_live_progress` (no value at `0 / 9`) |
| `progress` | The counts are one label each, "8 passed", with the number inside it, not a number node beside a word | A GPUI text child is not in the accessibility tree; the DOM read the span and the word as one line | `run_all_with_live_progress` |
| `a11y` | Reduced motion is the system's (`SPI_GETCLIENTAREAANIMATION` on Windows) given to GPUI's `set_reduce_motion`, read at start and each time the window becomes active; GPUI's animations then draw their end or start and ask for no frame | Electron read CSS's `prefers-reduced-motion`, which follows that setting; GPUI reads nothing itself, and a re-read on activation follows a change without polling | `progress::tests::the_bar_r14_1::reduced_motion_keeps_the_stripe_still` |
| `toolbar` | The Watch toggle and the step debugger's buttons wait for phase P4, which ports the runner's watch mode and debug runs; Env, Threads and Properties for PR 4 | The runner has no watch or debug run yet (P1 plan) | Their `Toolbar.test.tsx` tests are listed with P4 and PR 4 |
| `status_bar` | The update marker, the version and Settings wait for PR 5 (Settings) and phase P5 (the updater) | The Settings page and the updater are not ported yet | `StatusBar.test.tsx`'s update tests, with P5 |

## PR 1: the window, the state and the home screen

1. **`grace-core::state`.** `reduce.ts` and `store.ts`: `AppState`, `RunView`, `AppState::reduce(&AppEvent)`, and the store's `select`, `reveal`, `show_panel`, `select_call` and `dismiss_prompt`. The HTTP tab's `toggle_group` waits for P3 with `http.ts`. Tests: `reduce.test.ts` test for test on the same fixtures, `store.test.tsx`'s tests of P2.
2. **`grace::app::Grace`.** The entity that holds the `Runner` and the `AppState`. The runner's `emit` sends to a channel; the window's task applies each batch. Methods for P1's runner calls: open (through the dialog seam), open recent, remove recent, close, refresh, run, cancel, set filter, set threads, set env, save properties. Notices from failed calls as `main.ts`'s handler made them.
3. **Startup and close** (`main.ts`): the data folder (`GRACE_USER_DATA`), `kill_orphan`, leftover argument files and run folders removed, the last project reopened, the window shown once it has drawn its first frame (#90). Closing the window disposes of the runner, which kills Java, before Grace quits (WS-09).
4. **`grace::window`.** Bounds from `window.json` placed on the screens there are (`grace_core::window::place_window` on each display's work area), maximized when saved so, saved at the end of each move or resize and on close; a failed save never blocks the close.
5. **`grace::instance`.** One Grace per data folder; a second start brings the first window forward and quits.
6. **Appearance.** The saved theme (`settings.json` `appearance`) in force before the first frame; `system` follows the window's appearance.
7. **`grace::home`.** The home screen (spec R12): the mascot, the heading and its line, Open folder…, the recent folders with Not found and Remove from list, keyboard; the notice above it. `grace::ui::{notice, empty_state}`.
8. **`grace::seams`.** The folder dialog (Windows: `IFileOpenDialog` from the newest existing folder's parent; Linux: GPUI's), behind a trait; the `e2e` build answers from `GRACE_E2E_SEAMS`.
9. **e2e.** `home.spec.ts`'s tests that need neither the toolbar nor the tree, on the AT-SPI driver with the e2e seams (`GRACE_E2E_SEAMS`, a file of answers the app reads at each ask, and `e2e-seams.jsonl`, where it records each ask); the tree check on the home screen; a second start on the same data folder. `window.spec.ts` needs a window moved and closed by the driver, and the Windows Shell dialog needs a look by hand: both in PR 6.
10. **Parity.** Tick each row with its proof.

## PR 2: toolbar, run and cancel, progress, status bar

`Toolbar.tsx` (Home, Open…, the project path that stays readable, Run all, Cancel, Refresh, Report, the narrow layout below 1280 px), `Progress.tsx` (the bar, the passed part, the waiting stripe only during a run, #144, #263), `StatusBar.tsx` (busy, run summary, loading always visible). Rows: WS-04, WS-08, WS-09, WS-10, TB-*, LV-*, part of UP-*; `run.spec.ts`, `cancel.spec.ts`.

Done in this PR: the toolbar without its boxes, Watch and the debugger's buttons; the progress section; the status bar's path; Report through a new `open_path` seam; the system's reduced motion; `Driver::close` (`WM_DELETE_WINDOW` through `x11rb`, the window pattern's Close through UI Automation) and `App::java_pids` in the e2e driver. The checks that look at the tree's rows (a scenario running, passed or interrupted; a feature run from its own button) wait for PR 3, so WS-04, WS-08, `E-run-01` and `E-cancel-01` are ticked there. TB-04, TB-05 and TB-08 need the boxes or a resize from the driver: PR 4 and PR 6.

## PR 3: the tree and the filter bar

`treeRows.ts` and `nodes.ts` to `grace-core::rows`; the tree on `uniform_list` with keyboard navigation (VL-*), folders open first (TO-*), the focus ring at the row's indent (#153), run one test of a Maven or Gradle project (#208, #223); `FilterBar.tsx` with tag chips (TF-*, CF-*). `virtual.spec.ts`, `tags.spec.ts`.

## PR 4: environments, properties, threads

`EnvPicker.tsx`, `PropertiesDialog.tsx`, `ImportStep.tsx` and `importPlan.ts`, `Threads.tsx`: EV-*, EP-*, TC-*; `env.spec.ts`, `threads.spec.ts`.

## PR 5: details, console, bottom panel, appearance

`Details.tsx` (WS-06, its P2 part), `Console.tsx` with the banner, colours, find and error stepping (CB-*, CS-*), `BottomPanel.tsx`, `Splitter.tsx`, `panelHeight.ts` (UP-*), and the Appearance section of Settings (`appearance.spec.ts`). `console*.spec.ts`, `details.spec.ts`, `panel.spec.ts`.

## PR 6: the exit check

The P2 e2e tests left, the regressions of P2 (#144, #153, #170, #208, #223, #263), and `npm run bench:gpui` on the GPUI build against Electron Grace's limits, recorded in `docs/checks/`.

## Exit check (from #281)

The parity rows of phase P2 ticked; `npm run bench` on the GPUI build within every limit it shares with Electron Grace, reported in the PR.

## Cost

- **Memory.** The window's state is what Electron Grace's renderer held, without a JavaScript heap or a DOM: the tree once (shared with the runner through an `Arc`), the console's last 10,000 lines, the run's steps.
- **Idle.** No timer, poll or animation while no run is going. The window waits on the runner's channel; the single-instance listener is one thread blocked in `accept`, which wakes only when a second Grace starts.
- **During a run.** One update of the state and one redraw per batch of events, as `batch.ts` gives today; only the views whose data changed redraw.
- **Start.** The window shows after its first frame (#90); the last project's dry run starts after the window, as today.
- **Size.** No new dependency in PR 1: the Shell dialog uses the `windows` crate already in the lock, with two more features.
