# GPUI migration: parity checklist

Date 2026-11-08. Part of the [GPUI migration guide](2026-11-08-gpui-migration.md). Status: porting, phase P1.

This is the list the cut-over is checked against ([guide R3](2026-11-08-gpui-migration.md#r3-what-ported-means)): 594 spec scenarios, 194 end-to-end tests and 1,773 unit and component tests, taken from `develop` at `32a5ef6`, the commit of the go decision. A row is ported when the behaviour it names works in GPUI Grace and a test proves it: fill **Proof** with the test (`crates/…::test_name`, `e2e-uia/…`, a bench measure, or a dated check record for a by-hand check) and tick the box in the PR that ports it. A row that does not apply any more says why, in words, in **Proof**, with `n/a` in the last column; only the owner can accept an `n/a` on a user-facing row.

A spec or e2e test added on `develop` after this date is added here in the same PR, until the cut-over ([guide R9](2026-11-08-gpui-migration.md#r9-while-both-apps-live)).

Phases: P0 foundation ([#279](https://github.com/hpagniez/grace/issues/279)), P1 core ([#280](https://github.com/hpagniez/grace/issues/280)), P2 main screen ([#281](https://github.com/hpagniez/grace/issues/281)), P3 panels ([#282](https://github.com/hpagniez/grace/issues/282)), P4 project setup, watch and debugger ([#283](https://github.com/hpagniez/grace/issues/283)), P5 updates and release ([#284](https://github.com/hpagniez/grace/issues/284)), P6 cut-over ([#285](https://github.com/hpagniez/grace/issues/285)) ([guide](2026-11-08-gpui-migration.md#phases)).

| Phase | Spec scenarios |
|---|---|
| P0 | 80 |
| P1 | 10 |
| P2 | 241 |
| P3 | 130 |
| P4 | 103 |
| P5 | 26 |
| P6 | 4 |

## Spec scenarios

Every `Scenario` of every spec in `docs/specs/` on `develop` at the go decision, in the spec's order. Where two specs disagree, the later spec, and then the current e2e test, describes Grace as it ships ([guide R3](2026-11-08-gpui-migration.md#r3-what-ported-means)).

### WS · Walking skeleton: open, discover, run, watch

[Spec](../specs/2026-10-24-walking-skeleton.md) · [#52](https://github.com/hpagniez/grace/issues/52) [#53](https://github.com/hpagniez/grace/issues/53) [#54](https://github.com/hpagniez/grace/issues/54) [#56](https://github.com/hpagniez/grace/issues/56) [#57](https://github.com/hpagniez/grace/issues/57) [#58](https://github.com/hpagniez/grace/issues/58) [#60](https://github.com/hpagniez/grace/issues/60) [#83](https://github.com/hpagniez/grace/issues/83) · phase P2 · 12 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| WS-01 | Open a project and see its tests | | ☐ |
| WS-02 | Maven layout with classpath resources | | ☐ |
| WS-03 | Syntax error is reported, not hidden | | ☐ |
| WS-04 | Run all with live progress | | ☐ |
| WS-05 | Run one outline row | | ☐ |
| WS-06 | Failure details | | ☐ |
| WS-07 | A feature edited outside the app | | ☐ |
| WS-08 | Cancel leaves nothing behind | | ☐ |
| WS-09 | Closing the app during a run | | ☐ |
| WS-10 | Nothing to run | | ☐ |
| WS-11 | Open the HTML report | | ☐ |
| WS-12 | The project is not modified | | ☐ |

### TF · Tag filter: run by tag

[Spec](../specs/2026-10-24-tag-filter.md) · [#55](https://github.com/hpagniez/grace/issues/55) · phase P2 · 11 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| TF-01 | Chips list the project's tags | | ☐ |
| TF-02 | Include and exclude dim the tree | | ☐ |
| TF-03 | Third click turns a chip off | | ☐ |
| TF-04 | Hide non-matching | | ☐ |
| TF-05 | Run all runs only matching tests | | ☐ |
| TF-06 | A feature run is filtered | | ☐ |
| TF-07 | A tag removed before a run does not widen it | | ☐ |
| TF-08 | Part of an outline matches | | ☐ |
| TF-09 | Nothing matches | | ☐ |
| TF-10 | The filter survives a restart | | ☐ |
| TF-11 | A tag removed from the project leaves the filter | | ☐ |

### EV · Environment switcher and run properties

[Spec](../specs/2026-10-25-environment-switcher.md) · [#59](https://github.com/hpagniez/grace/issues/59) · phase P2 · 12 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| EV-01 | Suggestions come from overlay files and env tags | | ☐ |
| EV-02 | Changing the environment updates the tree | | ☐ |
| EV-03 | Changing the environment clears results | | ☐ |
| EV-04 | A run uses the environment | | ☐ |
| EV-05 | An environment without an overlay file | | ☐ |
| EV-06 | Shared and environment properties, environment wins | | ☐ |
| EV-07 | A secret is used but never shown | | ☐ |
| EV-08 | A secret stays off the command line and out of the settings file | | ☐ |
| EV-09 | Reserved names are refused | | ☐ |
| EV-10 | An unreadable secret blocks runs | | ☐ |
| EV-11 | Settings survive a restart | | ☐ |
| EV-12 | The machine's KARATE_ENV is ignored | | ☐ |

### HI · HTTP inspector

[Spec](../specs/2026-10-25-http-inspector.md) · [#62](https://github.com/hpagniez/grace/issues/62) · phase P3 · 11 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| HI-01 | Every call of a run is listed and grouped | | ☐ |
| HI-02 | Showing only the calls that went wrong | | ☐ |
| HI-03 | The tree selection narrows the list | | ☐ |
| HI-04 | A call's request and response | | ☐ |
| HI-05 | A call made in a called feature | | ☐ |
| HI-06 | A shared setup call shows under the scenario Karate ran it in | | ☐ |
| HI-07 | A call that got no response | | ☐ |
| HI-08 | Copy as cURL | | ☐ |
| HI-09 | A secret never shows in a call | | ☐ |
| HI-10 | Clicking a test gives the details pane back | | ☐ |
| HI-11 | A run of one test replaces only its calls | | ☐ |

### RH · Run history

[Spec](../specs/2026-10-25-run-history.md) · [#61](https://github.com/hpagniez/grace/issues/61) · phase P3 · 13 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| RH-01 | A run appears in the history | | ☐ |
| RH-02 | History survives a restart | | ☐ |
| RH-03 | A cancelled run is listed without a report | | ☐ |
| RH-04 | Re-run failed runs only the failed tests | | ☐ |
| RH-05 | A failed dynamic row is skipped | | ☐ |
| RH-06 | Re-run switches back to the run's environment and filter | | ☐ |
| RH-07 | Cancelling the environment switch of a re-run runs nothing | | ☐ |
| RH-08 | A re-run of a test that no longer exists | | ☐ |
| RH-09 | Only the 20 newest runs are kept | | ☐ |
| RH-10 | Clear history | | ☐ |
| RH-11 | A damaged run file does not hide the others | | ☐ |
| RH-12 | A secret never reaches the history files | | ☐ |
| RH-13 | History actions wait for the runner | | ☐ |

### FL · Flaky tests

[Spec](../specs/2026-10-26-flaky-tests.md) · [#66](https://github.com/hpagniez/grace/issues/66) · phase P3 · 9 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| FL-01 | A test that flips with no change is marked flaky | | ☐ |
| FL-02 | A fix in the project is not flaky | | ☐ |
| FL-03 | Different environments are not compared | | ☐ |
| FL-04 | The proof opens a run's report | | ☐ |
| FL-05 | The mark stays on its test when the tree changes | | ☐ |
| FL-06 | Marks survive a restart | | ☐ |
| FL-07 | Clear history removes the marks | | ☐ |
| FL-08 | A file saved during a run does not count (unit) | | ☐ |
| FL-09 | Runs without a fingerprint prove nothing (unit) | | ☐ |

### TC · Thread count

[Spec](../specs/2026-10-26-thread-count.md) · [#63](https://github.com/hpagniez/grace/issues/63) · phase P2 · 17 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| TC-01 | A project with no saved count runs with 1 thread | | ☐ |
| TC-02 | Run all uses the count in the box | | ☐ |
| TC-03 | A single test's run uses the count | | ☐ |
| TC-04 | Re-run failed uses the current count, not the run's | | ☐ |
| TC-05 | The box overrides the thread count of a karate-pom.json | | ☐ |
| TC-06 | The box is disabled while a run is going | | ☐ |
| TC-07 | Typing a number sends it once (unit) | | ☐ |
| TC-08 | Discovery passes one thread (unit) | | ☐ |
| TC-09 | The count is saved per project | | ☐ |
| TC-10 | A value out of range is refused | | ☐ |
| TC-11 | Main refuses a count that is not a whole number from 1 to 32 (unit) | | ☐ |
| TC-12 | The count is refused while busy (unit) | | ☐ |
| TC-13 | Saving the count keeps the other settings (unit) | | ☐ |
| TC-14 | A malformed saved count loads as 1 (unit) | | ☐ |
| TC-15 | A reloaded window shows the count (unit) | | ☐ |
| TC-16 | A run stored before F12 has no thread count (unit) | | ☐ |
| TC-17 | A run.json with a malformed thread count is dropped (unit) | | ☐ |

### UP · UI polish

[Spec](../specs/2026-10-26-ui-polish.md) · [#82](https://github.com/hpagniez/grace/issues/82) [#87](https://github.com/hpagniez/grace/issues/87) [#88](https://github.com/hpagniez/grace/issues/88) [#89](https://github.com/hpagniez/grace/issues/89) [#90](https://github.com/hpagniez/grace/issues/90) [#91](https://github.com/hpagniez/grace/issues/91) · phase P2 · 20 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| UP-01 | The bottom panel starts at its default height | | ☐ |
| UP-02 | Dragging the splitter resizes the bottom panel | | ☐ |
| UP-03 | The splitter works from the keyboard | | ☐ |
| UP-04 | The panel stays within its limits | | ☐ |
| UP-05 | The window cannot get too short for the layout | | ☐ |
| UP-06 | The height survives a restart | | ☐ |
| UP-07 | A smaller window shrinks the panel without forgetting its height | | ☐ |
| UP-08 | A bad saved height falls back to the default | | ☐ |
| UP-09 | Open… starts in the last project's parent folder | | ☐ |
| UP-10 | Open… with no last project | | ☐ |
| UP-11 | The last project folder was deleted | | ☐ |
| UP-12 | The Windows folder dialog shows the siblings (by hand) | | ☐ |
| UP-13 | Cursors match what a control does | | ☐ |
| UP-14 | The busy cursor while running | | ☐ |
| UP-15 | Scroll areas get thin themed scrollbars | | ☐ |
| UP-16 | Scrollbars look right and are easy to use (by hand) | | ☐ |
| UP-17 | The window opens painted in the app's theme | | ☐ |
| UP-18 | No white flash (by hand) | | ☐ |
| UP-19 | The window appears even if the page fails (by hand) | | ☐ |
| UP-20 | A second launch brings the window up (by hand) | | ☐ |

### WB · Window bounds

[Spec](../specs/2026-10-26-window-bounds.md) · [#92](https://github.com/hpagniez/grace/issues/92) · phase P2 · 18 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| WB-01 | The window comes back where it was | | ☐ |
| WB-02 | A maximized window comes back maximized | | ☐ |
| WB-03 | A move by the person is saved before the app closes | | ☐ |
| WB-04 | The saved state keeps the other settings (unit) | | ☐ |
| WB-05 | A window closed while minimized opens normal | | ☐ |
| WB-06 | A failed save does not stop the app from closing | | ☐ |
| WB-07 | A cut-off save leaves the old settings whole (unit) | | ☐ |
| WB-08 | A short saved window is held at the minimum height | | ☐ |
| WB-09 | A broken saved state opens the window as on a first start | | ☐ |
| WB-10 | A window partly off the screen is pulled onto it | | ☐ |
| WB-11 | A window bigger than the screen is shrunk to fit | | ☐ |
| WB-12 | A window on a screen that is gone opens on the main screen | | ☐ |
| WB-13 | A maximized window on a screen that is gone opens maximized on the main screen | | ☐ |
| WB-14 | Placement on two screens (unit) | | ☐ |
| WB-15 | Two screens (by hand) | | ☐ |
| WB-16 | The second screen is gone (by hand) | | ☐ |
| WB-17 | Partly off the screen (by hand) | | ☐ |
| WB-18 | A snapped window (by hand) | | ☐ |

### BD · Bundled dependency updates

[Spec](../specs/2026-10-27-bundled-dependency-updates.md) · [#84](https://github.com/hpagniez/grace/issues/84) · phase P0 · 14 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| BD-01 | Nothing new, nothing opened | | ☐ |
| BD-02 | A new Karate release opens a draft PR | | ☐ |
| BD-03 | A new JRE opens its own PR | | ☐ |
| BD-04 | Marking the PR ready runs full CI | | ☐ |
| BD-05 | A handled version is not opened again | | ☐ |
| BD-06 | A pushed branch without a PR gets its PR | | ☐ |
| BD-07 | A newer version supersedes a bot-only draft | | ☐ |
| BD-08 | An older bump PR with a person's work stays open | | ☐ |
| BD-09 | Versions that are never picked (unit) | | ☐ |
| BD-10 | Numbers compare as numbers (unit) | | ☐ |
| BD-11 | Missing data fails and changes nothing (unit) | | ☐ |
| BD-12 | npm run vendor follows a changed pin (by hand) | | ☐ |
| BD-13 | The bump script reproduces today's pins (by hand) | | ☐ |
| BD-14 | First real run after merge (by hand) | | ☐ |

### CB · Console banner

[Spec](../specs/2026-10-27-console-banner.md) · [#75](https://github.com/hpagniez/grace/issues/75) · phase P2 · 13 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| CB-01 | No project open | | ☐ |
| CB-02 | A project is open and nothing has run | | ☐ |
| CB-03 | One scenario in one feature | | ☐ |
| CB-04 | A project with no tests | | ☐ |
| CB-05 | Finding tests for the first time | | ☐ |
| CB-06 | Discovery failed | | ☐ |
| CB-07 | Nothing to run | | ☐ |
| CB-08 | The cursor stays still with reduced motion | | ☐ |
| CB-09 | A run starts and Karate has not printed yet | | ☐ |
| CB-10 | The banner survives the line cap | | ☐ |
| CB-11 | Open, run and run again (e2e) | | ☐ |
| CB-12 | The console is darker than the window in dark mode (e2e) | | ☐ |
| CB-13 | The banner names the pinned versions | | ☐ |

### HS · Home screen

[Spec](../specs/2026-10-27-home-screen.md) · [#78](https://github.com/hpagniez/grace/issues/78) · phase P2 · 24 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| HS-01 | A first start shows the home screen | | ☐ |
| HS-02 | Opening a folder from the home screen | | ☐ |
| HS-03 | The last project opens at start without the home screen | | ☐ |
| HS-04 | Opening with no project open shows the project view, not home (unit) | | ☐ |
| HS-05 | No home screen flash at start (by hand) | | ☐ |
| HS-06 | A slow network folder does not hold up the home screen (unit) | | ☐ |
| HS-07 | Removing the missing last project forgets it (unit) | | ☐ |
| HS-08 | A missing last project shows the home screen | | ☐ |
| HS-09 | A failed open from the home screen goes back to it (unit) | | ☐ |
| HS-10 | A dry-run error still opens the project | | ☐ |
| HS-11 | Reopening a folder from the list | | ☐ |
| HS-12 | The folder dialog starts next to the newest existing folder | | ☐ |
| HS-13 | Removing a row | | ☐ |
| HS-14 | The list works from the keyboard | | ☐ |
| HS-15 | The home screen follows the dark theme (by hand) | | ☐ |
| HS-16 | Home closes the project and survives a restart | | ☐ |
| HS-17 | Home is disabled during a run | | ☐ |
| HS-18 | Main refuses to close a busy project (unit) | | ☐ |
| HS-19 | A reload after Home stays home | | ☐ |
| HS-20 | The list keeps the 10 newest folders (unit) | | ☐ |
| HS-21 | The same folder in another case is one entry (unit) | | ☐ |
| HS-22 | A broken list reads as what can be kept (unit) | | ☐ |
| HS-23 | A failed list save keeps the project open (unit) | | ☐ |
| HS-24 | A path not in the list is refused | | ☐ |

### AU · App updates

[Spec](../specs/2026-10-28-app-updates.md) · [#77](https://github.com/hpagniez/grace/issues/77) · phase P5 · 26 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| AU-01 | The version is always visible | | ☐ |
| AU-02 | Out of date on the pre-release channel | | ☐ |
| AU-03 | Nothing newer on the stable channel | | ☐ |
| AU-04 | Default channel follows the running version | | ☐ |
| AU-05 | Startup modal | | ☐ |
| AU-06 | Skipping a version | | ☐ |
| AU-07 | A newer version after a skip | | ☐ |
| AU-08 | Closing the modal without a choice | | ☐ |
| AU-09 | Auto-update skips the modal | | ☐ |
| AU-10 | No network when the startup check is off | | ☐ |
| AU-11 | A failed check is quiet | | ☐ |
| AU-12 | Invalid releases are ignored | | ☐ |
| AU-13 | Version list | | ☐ |
| AU-14 | No install while a run is active | | ☐ |
| AU-15 | No install from a development build | | ☐ |
| AU-16 | Rollback warning with a saved copy | | ☐ |
| AU-17 | Rollback warning without a saved copy | | ☐ |
| AU-18 | A damaged download changes nothing | | ☐ |
| AU-19 | An unwritable install folder | | ☐ |
| AU-20 | Update in one click | | ☐ |
| AU-21 | Rollback restores the older version's data | | ☐ |
| AU-22 | A failed swap keeps the old version | | ☐ |
| AU-23 | A version that does not start is switched back | | ☐ |
| AU-24 | Updating after a rollback offers the newer data | | ☐ |
| AU-25 | A folder shared with other files is not updated | | ☐ |
| AU-26 | Only three backups are kept | | ☐ |

### FC · Faster CI

[Spec](../specs/2026-10-28-faster-ci.md) · [#95](https://github.com/hpagniez/grace/issues/95) · phase P0 · 8 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| FC-01 | a ready PR runs four jobs | | ☐ |
| FC-02 | a draft PR runs unit tests only | | ☐ |
| FC-03 | every test runs exactly once | | ☐ |
| FC-04 | the e2e shards cover the suite | | ☐ |
| FC-05 | npm test still runs everything locally | | ☐ |
| FC-06 | no assertion changes | | ☐ |
| FC-07 | merge-when-green and the e2e steps | | ☐ |
| FC-08 | the time goal | | ☐ |

### LV · Loading always visible

[Spec](../specs/2026-10-28-loading-always-visible.md) · [#80](https://github.com/hpagniez/grace/issues/80) · phase P2 · 9 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| LV-01 | Finding tests shows the stripe | | ☐ |
| LV-02 | Finding tests for the first time | | ☐ |
| LV-03 | Tests found | | ☐ |
| LV-04 | The stripe never covers a run's results | | ☐ |
| LV-05 | The stripe stays until the first result | | ☐ |
| LV-06 | The bar fills smoothly | | ☐ |
| LV-07 | Cancel stops the stripe | | ☐ |
| LV-08 | Reduced motion | | ☐ |
| LV-09 | Run and finish (e2e) | | ☐ |

### PU · Premium look and feel with a defined graphic chart

[Spec](../specs/2026-10-29-premium-ui.md) · [#107](https://github.com/hpagniez/grace/issues/107) [#75](https://github.com/hpagniez/grace/issues/75) [#158](https://github.com/hpagniez/grace/issues/158) · phase P0 · 33 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| PU-01 | Every token pair meets WCAG 2.2 AA in both themes (unit) | `crates/grace::theme::tests::keeps_text_at_4_5_to_1_on_every_background`, `…keeps_each_status_readable_on_surface_its_tint_and_a_selected_row`, `…keeps_the_filled_buttons_text_readable`, `…keeps_control_borders_and_the_focus_ring_at_3_to_1`, `…paints_console_matches_readably_the_current_one_stronger` | ☑ |
| PU-02 | The dark theme changes colours only (unit) | By construction: `grace::theme::Theme` is one shared `Metrics` and one `Tokens` per theme, so no size can differ; `crates/grace::theme::tests::matches_the_dark_tokens_of_index_css`, `…matches_the_sizes_of_index_css` | ☑ |
| PU-03 | The structure and the primary button do not change with the theme | | ☐ |
| PU-04 | No raw palette class or dark: variant is left (unit) | `crates/grace::theme::tests::colours_come_only_from_the_theme` (no colour literal outside `theme.rs`; GPUI has no `dark:` variant) | ☑ |
| PU-05 | Main and the startup screen use the CSS ground colours (unit) | | ☐ |
| PU-06 | The window follows the Windows theme | | ☐ |
| PU-07 | Grace draws its own typefaces offline | | ☐ |
| PU-08 | Each status has its own icon and name | `crates/grace::elements::tests::marks_each_status_as_electron_grace_does`, `…tells_statuses_apart_by_shape_or_name`; through the tree, `crates/grace-e2e/tests/elements.rs::names_each_status_mark` | ☑ |
| PU-09 | A cancelled run marks its interrupted tests | | ☐ |
| PU-10 | Tag chip states differ by shape, not only colour (unit) | | ☐ |
| PU-11 | An HTTP status shows its code in a badge with its group in the tooltip (unit) | | ☐ |
| PU-12 | Reduced motion stops every animation | | ☐ |
| PU-13 | The logo is the Home button | | ☐ |
| PU-14 | The startup screen shows the logo in the active theme (by hand) | | ☐ |
| PU-15 | Choosing Dark overrides a light Windows | | ☐ |
| PU-16 | Follow Windows is the default | | ☐ |
| PU-17 | A malformed appearance setting reads as Follow Windows (unit) | | ☐ |
| PU-18 | Main rejects an unknown theme (unit) | | ☐ |
| PU-19 | A tooltip shows on keyboard focus and closes with Escape | `crates/grace::tooltip::tests::shows_at_once_on_keyboard_focus_and_once_per_focus`, `…closes_on_escape_and_on_a_press_of_its_target`, `…does_not_show_on_a_focus_from_the_mouse`; through the tree, `crates/grace-e2e/tests/tooltips.rs::shows_on_keyboard_focus_and_closes_with_escape` | ☑ |
| PU-20 | A tooltip shows after a pause on hover, also on a disabled button | `crates/grace::tooltip::tests::shows_after_the_pause_not_before`, `…does_not_show_when_the_pointer_leaves_first_and_hides_when_it_leaves_after`, `…shows_one_at_a_time`; through the tree, `crates/grace-e2e/tests/tooltips.rs::shows_on_hover_also_over_a_disabled_button` | ☑ |
| PU-21 | Escape in a dialog closes the tooltip first | `crates/grace-e2e/tests/tooltips.rs::escape_in_a_dialog_closes_the_tooltip_first` (the tooltip takes Escape before any other handler) | ☑ |
| PU-22 | No native title attribute is left (unit) | | ☐ |
| PU-23 | Every control shows the focus ring | | ☐ |
| PU-24 | Targets are at least 24 by 24 | | ☐ |
| PU-25 | Existing keyboard paths still work | | ☐ |
| PU-26 | First run shows the first-run home screen | | ☐ |
| PU-27 | A missing recent folder says so in words | | ☐ |
| PU-28 | A project with no feature files shows an empty state | | ☐ |
| PU-29 | Before any run the progress line says so | | ☐ |
| PU-30 | Details with nothing selected | | ☐ |
| PU-31 | Empty history and HTTP tabs | | ☐ |
| PU-32 | Dialogs keep their order and keyboard behaviour | | ☐ |
| PU-33 | Before and after screenshots exist (by hand) | | ☐ |

### VL · Big projects: virtual lists and keyboard navigation

[Spec](../specs/2026-10-30-virtual-lists.md) · [#91](https://github.com/hpagniez/grace/issues/91) [#123](https://github.com/hpagniez/grace/issues/123) [#124](https://github.com/hpagniez/grace/issues/124) [#125](https://github.com/hpagniez/grace/issues/125) · phase P2 · 29 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| VL-01 | Only the rows in view are drawn (e2e) | | ☐ |
| VL-02 | A small list draws in jsdom | | ☐ |
| VL-03 | The tree shows the same rows as before | | ☐ |
| VL-04 | Closed folders survive a refresh | | ☐ |
| VL-05 | Statuses do not move rows | | ☐ |
| VL-06 | The tree is one tab stop | | ☐ |
| VL-07 | Arrow keys move and select | | ☐ |
| VL-08 | Right and Left | | ☐ |
| VL-09 | Home, End and pages | | ☐ |
| VL-10 | Ctrl+Enter runs the focused row | | ☐ |
| VL-11 | The row's buttons left the tab order but still click | | ☐ |
| VL-12 | A click makes the row focused | | ☐ |
| VL-13 | The HTTP list is one tab stop and arrows open calls | | ☐ |
| VL-14 | A moved-to row scrolls into view (e2e) | | ☐ |
| VL-15 | Focus survives scrolling away (e2e) | | ☐ |
| VL-16 | Closing a folder takes focus to it | | ☐ |
| VL-17 | Hiding the focused row | | ☐ |
| VL-18 | A new tree keeps the rows in view still (e2e) | | ☐ |
| VL-19 | Calls arriving above the view keep the rows in view still | | ☐ |
| VL-20 | A filter change scrolls to the selected row (e2e) | | ☐ |
| VL-21 | A cut label shows whole on keyboard focus | | ☐ |
| VL-22 | The run shortcut can be found | | ☐ |
| VL-23 | Tree rows carry their level and position | | ☐ |
| VL-24 | A row's name says its status, flakiness and tags | | ☐ |
| VL-25 | Calls are options | | ☐ |
| VL-26 | Opening 10,000 tests (perf) | | ☐ |
| VL-27 | Folders, selection and the HTTP tab at 10,000 tests (perf) | | ☐ |
| VL-28 | Run all at 10,000 tests (perf) | | ☐ |
| VL-29 | 2,000 tests are not slower (perf) | | ☐ |

### LF · Light and fast: numbers on every PR, a benchmark before every release

[Spec](../specs/2026-10-31-light-and-fast.md) · [#147](https://github.com/hpagniez/grace/issues/147) [#148](https://github.com/hpagniez/grace/issues/148) · phase P0 · 25 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| LF-01 | The bench measures the packaged app | | ☐ |
| LF-02 | The bench compares two builds in turns | | ☐ |
| LF-03 | The bench measures a release zip it did not build | | ☐ |
| LF-04 | The bench waits for a quiet machine | | ☐ |
| LF-05 | A worse number does not fail the bench | | ☐ |
| LF-06 | A bench that cannot measure fails | | ☐ |
| LF-07 | Smoothness is measured idle and during a run | | ☐ |
| LF-08 | Idle CPU is measured shown and minimised | | ☐ |
| LF-09 | Numbers on a PR | | ☐ |
| LF-10 | A merge waits for the numbers | | ☐ |
| LF-11 | A worse number never blocks the merge | | ☐ |
| LF-12 | A perf run that could not measure holds the merge | | ☐ |
| LF-13 | A docs-only PR | | ☐ |
| LF-14 | A PR from a fork | | ☐ |
| LF-15 | Raising a limit without a decision record | | ☐ |
| LF-16 | Lowering a limit | | ☐ |
| LF-17 | A release with no performance section | | ☐ |
| LF-18 | A release with no bench JSON | | ☐ |
| LF-19 | A release result typed over a miss | | ☐ |
| LF-20 | A measure the previous release cannot give | | ☐ |
| LF-21 | A release miss with no decision | | ☐ |
| LF-22 | A release miss with a no-go | | ☐ |
| LF-23 | A release miss, analysed and accepted | | ☐ |
| LF-24 | A release that passes | | ☐ |
| LF-25 | Feature PRs skip the release gate | | ☐ |

### JM · Java memory

[Spec](../specs/2026-11-01-java-memory.md) · [#149](https://github.com/hpagniez/grace/issues/149) · phase P1 · 6 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| JM-01 | Every Java Grace starts gets the memory flags | `karate::command::tests::build_args::builds_the_exact_command_line`, `…passes_the_count_on_runs_and_t_1_on_dry_runs`, `…adds_the_debug_listener_and_its_socket_to_a_debug_run_only`; real Java: `…build_args_through_java::java_stops_at_the_first_out_of_memory_error_with_the_line_out_of_memory_finds`. The build tool's Java ports with Maven in P4
| JM-02 | A run that runs out of memory says so | | ☐ |
| JM-03 | A dry run that runs out of memory says so | | ☐ |
| JM-04 | Another out-of-memory error gets no notice | `karate::command::tests::out_of_memory::finds_javas_out_of_memory_line_among_the_others` (Metaspace and native-thread lines)
| JM-05 | A test's output that names the error is not Java running out of memory | `karate::command::tests::out_of_memory::finds_javas_out_of_memory_line_among_the_others` (a response body and a test name that name the error)
| JM-06 | The bench shows a run takes less memory | | ☐ |

### EN · Ship only the en-US locale

[Spec](../specs/2026-11-02-en-us-locale.md) · [#150](https://github.com/hpagniez/grace/issues/150) · phase P6 · 4 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| EN-01 | The package holds one locale | | ☐ |
| EN-02 | The updater contract does not change | | ☐ |
| EN-03 | Grace starts on a machine with another language | | ☐ |
| EN-04 | A file dialog keeps the machine's language | | ☐ |

### JJ · Java's JIT

[Spec](../specs/2026-11-02-java-jit.md) · [#179](https://github.com/hpagniez/grace/issues/179) · phase P1 · 4 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| JJ-01 | The dry run compiles with C1 only | `karate::command::tests::build_args::builds_the_exact_command_line` (the dry run's `-XX:TieredStopAtLevel=1`)
| JJ-02 | A run keeps C2 on fewer threads | `karate::command::tests::build_args::builds_the_exact_command_line` (the run's `-XX:CICompilerCount=2`)
| JJ-03 | Heavy JavaScript runs no slower | | ☐ |
| JJ-04 | The bench shows Java works less | | ☐ |

### PL · Project libraries

[Spec](../specs/2026-11-02-project-libraries.md) · [#64](https://github.com/hpagniez/grace/issues/64) · phase P4 · 13 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| PL-01 | A jar in the project's .karate/ext is on the classpath | | ☐ |
| PL-02 | A Maven project's libraries are on the classpath | | ☐ |
| PL-03 | A Gradle project's libraries are on the classpath | | ☐ |
| PL-04 | The order of the classpath | | ☐ |
| PL-05 | The list is kept until a build file changes | | ☐ |
| PL-06 | Switching the environment and running do not start the tool | | ☐ |
| PL-07 | No tool | | ☐ |
| PL-08 | A failed listing keeps the last list | | ☐ |
| PL-09 | Grace says what it is doing | | ☐ |
| PL-10 | Cancel while listing | | ☐ |
| PL-11 | The project's Java code is not built | | ☐ |
| PL-12 | A JUnit runner needs no build | | ☐ |
| PL-13 | Standalone projects run no tool | | ☐ |

### PS · Project setup

[Spec](../specs/2026-11-02-project-setup.md) · [#64](https://github.com/hpagniez/grace/issues/64) [#208](https://github.com/hpagniez/grace/issues/208) · phase P4 · 9 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| PS-01 | The pom's run settings do not change a run | | ☐ |
| PS-02 | The pom's paths choose the tests | | ☐ |
| PS-03 | The pom's configDir gives the config and the environments | | ☐ |
| PS-04 | A pom Grace cannot read | | ☐ |
| PS-05 | The notice shows only on open | | ☐ |
| PS-06 | Maven's compiled output is on the classpath | | ☐ |
| PS-07 | Compiled copies stay out of the tree and the environments | | ☐ |
| PS-08 | One row of a Maven outline runs | | ☐ |
| PS-09 | Gradle's compiled output is on the classpath | | ☐ |

### SD · Step debugger: breakpoints on steps, pause, step, inspect variables

[Spec](../specs/2026-11-02-step-debugger.md) · [#65](https://github.com/hpagniez/grace/issues/65) · phase P4 · 23 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| SD-01 | variables | | ☐ |
| SD-02 | second | | ☐ |
| SD-03 | A breakpoint pauses before its step, with the variables | | ☐ |
| SD-04 | Step over stays at the same depth | | ☐ |
| SD-05 | Step into enters a called feature | | ☐ |
| SD-06 | Pause stops before the next step | | ☐ |
| SD-07 | A debug run uses one thread and the listener; a normal run does not | | ☐ |
| SD-08 | Only the token's connection is accepted | | ☐ |
| SD-09 | Cancel while paused | | ☐ |
| SD-10 | A broken socket does not stop the run | | ☐ |
| SD-11 | Secrets are masked in variables | | ☐ |
| SD-12 | A long value is cut | | ☐ |
| SD-13 | Debug runs do not prove a test flaky | | ☐ |
| SD-14 | Steps before a run, from the feature file | | ☐ |
| SD-15 | Setting and removing a breakpoint | | ☐ |
| SD-16 | A background breakpoint shows in every scenario | | ☐ |
| SD-17 | Debugging from the tree, paused, stepping, finished (e2e) | | ☐ |
| SD-18 | The keyboard starts a debug run or a normal run | | ☐ |
| SD-19 | Nothing spins while paused | | ☐ |
| SD-20 | Pause while running | | ☐ |
| SD-21 | Paused in a called feature | | ☐ |
| SD-22 | Between pauses the variables wait | | ☐ |
| SD-23 | The debug controls at 760 px (e2e) | | ☐ |

### TB · A toolbar that stays readable with a long project path

[Spec](../specs/2026-11-02-toolbar.md) · [#81](https://github.com/hpagniez/grace/issues/81) · phase P2 · 8 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| TB-01 | The toolbar holds the name and the chip, not the path | | ☐ |
| TB-02 | The name is cut in the middle, keeping its end | | ☐ |
| TB-03 | A cut name explains itself | | ☐ |
| TB-04 | The name shows whole at the default size (e2e) | | ☐ |
| TB-05 | A long path at the minimum width (e2e) | | ☐ |
| TB-06 | The status bar shows the path while a project is open | | ☐ |
| TB-07 | A cut path explains itself | | ☐ |
| TB-08 | Env and Threads below 1280 px | | ☐ |

### CF · A one-line filter bar

[Spec](../specs/2026-11-03-compact-filter-bar.md) · [#127](https://github.com/hpagniez/grace/issues/127) · phase P2 · 7 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| CF-01 | Chips that do not fit fold behind a count | | ☐ |
| CF-02 | A folded tag that is on is counted | | ☐ |
| CF-03 | No fold chip when every chip fits | | ☐ |
| CF-04 | The panel holds every tag and cycles it | | ☐ |
| CF-05 | The panel closes when nothing is folded | | ☐ |
| CF-06 | During a run the panel opens with its chips disabled | | ☐ |
| CF-07 | The bench project keeps a one-line bar (e2e) | | ☐ |

### CS · Find in the console, and step through its errors

[Spec](../specs/2026-11-03-console-search.md) · [#73](https://github.com/hpagniez/grace/issues/73) · phase P2 · 17 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| CS-01 | ERROR lines and failed tests are errors, WARN lines are not | | ☐ |
| CS-02 | The errors group shows the count and steps through the errors | | ☐ |
| CS-03 | No errors, no group | | ☐ |
| CS-04 | The errors group belongs to the Console tab | | ☐ |
| CS-05 | A failing run's errors (e2e) | | ☐ |
| CS-06 | Ctrl+F opens the find bar from another tab | | ☐ |
| CS-07 | Matches are counted, case aside, without overlap | | ☐ |
| CS-08 | Enter and Shift+Enter step through the matches and wrap | | ☐ |
| CS-09 | No results | | ☐ |
| CS-10 | Matches are painted without redrawing the rows (highlight API stubbed) | | ☐ |
| CS-11 | Escape closes the bar, keeps the text, and gives the focus back | | ☐ |
| CS-12 | The text survives a tab switch and a new run | | ☐ |
| CS-13 | Lines that arrive during a run do not move the view | | ☐ |
| CS-14 | Find in a real run (e2e) | | ☐ |
| CS-15 | The bar does not search while it is closed | | ☐ |
| CS-16 | A line keeps its place when older lines drop at the cap | | ☐ |
| CS-17 | A full console (e2e) | | ☐ |

### DD · Data-driven view: an outline's Examples as a table

[Spec](../specs/2026-11-03-data-driven-view.md) · [#71](https://github.com/hpagniez/grace/issues/71) · phase P3 · 7 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| DD-01 | An outline's Examples before a run | | ☐ |
| DD-02 | Per-row results after a run | | ☐ |
| DD-03 | A click opens a row | | ☐ |
| DD-04 | The keyboard moves without opening | | ☐ |
| DD-05 | A dynamic Examples table | | ☐ |
| DD-06 | Examples tables with different columns | | ☐ |
| DD-07 | A big outline | | ☐ |

### ER · Export a run's report

[Spec](../specs/2026-11-03-export-reports.md) · [#70](https://github.com/hpagniez/grace/issues/70) · phase P3 · 11 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| ER-01 | The project asks for JUnit XML | | ☐ |
| ER-02 | The project does not ask for JUnit XML | | ☐ |
| ER-03 | A wrong value makes the file unreadable | | ☐ |
| ER-04 | The other output keys are listed as unused | | ☐ |
| ER-05 | Export follows Report | | ☐ |
| ER-06 | Main refuses a malformed id | | ☐ |
| ER-07 | Closing the dialog does nothing | | ☐ |
| ER-08 | The zip holds the report | | ☐ |
| ER-09 | Any characters in the chosen path | | ☐ |
| ER-10 | A zip that cannot be written leaves nothing | | ☐ |
| ER-11 | Export a run's report (e2e) | | ☐ |

### FB · Find Maven and Gradle when they are not on PATH

[Spec](../specs/2026-11-03-find-build-tools.md) · [#231](https://github.com/hpagniez/grace/issues/231) · phase P4 · 6 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| FB-01 | Maven in MAVEN_HOME | | ☐ |
| FB-02 | M2_HOME when MAVEN_HOME has no Maven | | ☐ |
| FB-03 | Gradle in GRADLE_HOME | | ☐ |
| FB-04 | PATH comes first | | ☐ |
| FB-05 | IntelliJ IDEA's Maven, the newest first | | ☐ |
| FB-06 | No Maven anywhere | | ☐ |

### OT · Open in VS Code from the test tree

[Spec](../specs/2026-11-03-open-in-editor-from-tree.md) · [#222](https://github.com/hpagniez/grace/issues/222) · phase P3 · 6 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| OT-01 | A right-click on a scenario asks for the menu | | ☐ |
| OT-02 | A folder has no menu | | ☐ |
| OT-03 | A failed test opens at its failed step | | ☐ |
| OT-04 | A feature opens at its top | | ☐ |
| OT-05 | Main refuses a malformed menu request | | ☐ |
| OT-06 | Open a scenario from the tree (e2e) | | ☐ |

### OE · Open a test in VS Code

[Spec](../specs/2026-11-03-open-in-editor.md) · [#68](https://github.com/hpagniez/grace/issues/68) · phase P3 · 11 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| OE-01 | The button opens the test at its line | | ☐ |
| OE-02 | The button shows during a run | | ☐ |
| OE-03 | The error's place is a link | | ☐ |
| OE-04 | An error without a place has no link | | ☐ |
| OE-05 | A feature of the tree resolves under the classpath | | ☐ |
| OE-06 | Main opens only files of the open project | | ☐ |
| OE-07 | Main refuses a malformed request (e2e) | | ☐ |
| OE-08 | The link VS Code opens | | ☐ |
| OE-09 | The link to a file on a share | | ☐ |
| OE-10 | Open a scenario in VS Code (e2e) | | ☐ |
| OE-11 | VS Code is not installed (e2e) | | ☐ |

### SC · Step comments in Details

[Spec](../specs/2026-11-03-step-comments.md) · [#243](https://github.com/hpagniez/grace/issues/243) · phase P3 · 8 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| SC-01 | A run's step keeps Karate's comments | | ☐ |
| SC-02 | The file reader attaches comments as Karate does | | ☐ |
| SC-03 | Background steps keep their comments | | ☐ |
| SC-04 | A comment attached to no step is not shown | | ☐ |
| SC-05 | Comments show above their step | | ☐ |
| SC-06 | The paused step shows its comments | | ☐ |
| SC-07 | Comments are masked | | ☐ |
| SC-08 | Comments before and after a run (e2e) | | ☐ |

### TO · Open the test tree with only its folders open

[Spec](../specs/2026-11-03-tree-folders-open.md) · [#224](https://github.com/hpagniez/grace/issues/224) · phase P2 · 18 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| TO-01 | A project opens with its features closed | | ☐ |
| TO-02 | A refresh keeps what I opened and closed | | ☐ |
| TO-03 | A feature new in a refresh starts closed | | ☐ |
| TO-04 | Another project starts closed again | | ☐ |
| TO-05 | A click on a closed feature's name opens and selects it | | ☐ |
| TO-06 | A click on an open feature's name keeps it open | | ☐ |
| TO-07 | The chevron still closes a feature | | ☐ |
| TO-08 | Moving with the keys does not open features | | ☐ |
| TO-09 | An outline opens with its feature | | ☐ |
| TO-10 | A run of one feature opens it as it starts | | ☐ |
| TO-11 | Run all opens only the features with a failure | | ☐ |
| TO-12 | A run never reopens a feature I closed during it | | ☐ |
| TO-13 | A run opens a feature inside a closed folder, and leaves the folder closed | | ☐ |
| TO-14 | A failure leaves a closed outline closed | | ☐ |
| TO-15 | What a run opened stays open | | ☐ |
| TO-16 | A pause still opens the way to the paused test | | ☐ |
| TO-17 | Statuses do not move rows | | ☐ |
| TO-18 | A feature opening above the view does not move it | | ☐ |

### WM · Watch mode: re-run on save

[Spec](../specs/2026-11-03-watch-mode.md) · [#67](https://github.com/hpagniez/grace/issues/67) · phase P4 · 17 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| WM-01 | The toggle turns watch mode on and is remembered | | ☐ |
| WM-02 | Main refuses a malformed request | | ☐ |
| WM-03 | The toggle works during a run | | ☐ |
| WM-04 | A saved feature runs | | ☐ |
| WM-05 | Nothing runs while off | | ☐ |
| WM-06 | Only features count | | ☐ |
| WM-07 | An event with no new modified time does not count | | ☐ |
| WM-08 | A broken project is still watched | | ☐ |
| WM-09 | Skipped folders do not count | | ☐ |
| WM-10 | Save all is one run | | ☐ |
| WM-11 | The filter applies | | ☐ |
| WM-12 | A called feature is skipped quietly | | ☐ |
| WM-13 | A save during a run waits | | ☐ |
| WM-14 | Cancel drops waiting saves | | ☐ |
| WM-15 | A watcher error stops watch mode | | ☐ |
| WM-16 | Closing the project stops watching | | ☐ |
| WM-17 | Re-run on save in the app (e2e) | | ☐ |

### BN · Build the project's Java code from the not-built notice

[Spec](../specs/2026-11-04-build-from-notice.md) · [#233](https://github.com/hpagniez/grace/issues/233) · phase P4 · 10 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| BN-01 | The notice offers to build | | ☐ |
| BN-02 | No tool, no button | | ☐ |
| BN-03 | Building, then refreshing | | ☐ |
| BN-04 | A build that rewrites no class still counts | | ☐ |
| BN-05 | A Gradle project | | ☐ |
| BN-06 | The build fails | | ☐ |
| BN-07 | No JDK | | ☐ |
| BN-08 | Cancel while building | | ☐ |
| BN-09 | Busy | | ☐ |
| BN-10 | Grace builds only on a click | | ☐ |

### EP · Run properties as environment variables, imported from a secrets file

[Spec](../specs/2026-11-04-env-var-properties.md) · [#230](https://github.com/hpagniez/grace/issues/230) · phase P2 · 26 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| EP-01 | The name rules follow the kind | | ☐ |
| EP-02 | Stripped variables cannot be set | | ☐ |
| EP-03 | Two variables that differ only in case | | ☐ |
| EP-04 | Switching the kind keeps a saved secret | | ☐ |
| EP-05 | An environment variable reaches the run and is masked | | ☐ |
| EP-06 | Both kinds in one run | | ☐ |
| EP-07 | The variable replaces the machine's in any letter case | | ☐ |
| EP-08 | The environment's variable wins over a shared one in another case | | ☐ |
| EP-09 | Dry runs get no environment variables | | ☐ |
| EP-10 | Old rows load as system properties | | ☐ |
| EP-11 | A malformed kind is kept, not loaded | | ☐ |
| EP-12 | Main parses a .properties file as Java does | | ☐ |
| EP-13 | A byte order mark is dropped | | ☐ |
| EP-14 | A malformed escape refuses the file | | ☐ |
| EP-15 | A file over 1 MB is refused | | ☐ |
| EP-16 | Closing the Open dialog does nothing | | ☐ |
| EP-17 | The import step lists adds, replaces and skips | | ☐ |
| EP-18 | A file that is no .properties file shows no line of it | | ☐ |
| EP-19 | Two keys with one name | | ☐ |
| EP-20 | An invalid prefix blocks the import | | ☐ |
| EP-21 | An import past the list limit is blocked | | ☐ |
| EP-22 | Import adds to the draft only | | ☐ |
| EP-23 | Back changes nothing | | ☐ |
| EP-24 | Import into an environment with no list yet | | ☐ |
| EP-25 | Import needs secret storage | | ☐ |
| EP-26 | Import a secrets file and run with it (e2e) | | ☐ |

### GF · Say once when the config fails, and group failures that share an error

[Spec](../specs/2026-11-04-group-failures.md) · [#229](https://github.com/hpagniez/grace/issues/229) · phase P3 · 22 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| GF-01 | A test whose config failed is failed | | ☐ |
| GF-02 | A test with its own SCENARIO_EXIT keeps its status | | ☐ |
| GF-03 | Karate's synthetic step shows without "undefined" | | ☐ |
| GF-04 | Karate's synthetic step has no breakpoint dot | | ☐ |
| GF-05 | The reason is Karate's errorReason | | ☐ |
| GF-06 | Two karate.fail calls at different lines share an error | | ☐ |
| GF-07 | A thrown string's position is left out | | ☐ |
| GF-08 | A config failure's reason is its whole error | | ☐ |
| GF-09 | Failures at different places with one message form a group | | ☐ |
| GF-10 | A test hidden by the tag filter is left out | | ☐ |
| GF-11 | No group while a run goes on | | ☐ |
| GF-12 | Step through a group | | ☐ |
| GF-13 | Stepping wraps round | | ☐ |
| GF-14 | A lone failure has no stepping | | ☐ |
| GF-15 | A folder lists its failures by error, largest first | | ☐ |
| GF-16 | Go to first | | ☐ |
| GF-17 | A folder without a group shows no list | | ☐ |
| GF-18 | The banner names the config file and its line | | ☐ |
| GF-19 | A config message with no line opens line 1 | | ☐ |
| GF-20 | The banner goes when the next run starts | | ☐ |
| GF-21 | No banner for ordinary failures | | ☐ |
| GF-22 | A broken karate-config.js, end to end (e2e) | | ☐ |

### PD · Start from the project's pom

[Spec](../specs/2026-11-04-pom-defaults.md) · [#228](https://github.com/hpagniez/grace/issues/228) · phase P4 · 17 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| PD-01 | karate-pom.json starts the controls (e2e) | | ☐ |
| PD-02 | What the person saved wins | | ☐ |
| PD-03 | Each control on its own | | ☐ |
| PD-04 | Picking no environment is saved | | ☐ |
| PD-05 | Saving run properties does not save the pom's environment | | ☐ |
| PD-06 | A pom filter whose tags no test has is not saved | | ☐ |
| PD-07 | Refresh applies an edited pom to the controls with nothing saved | | ☐ |
| PD-08 | Refresh drops a value the pom no longer gives | | ☐ |
| PD-09 | Tag entries the filter can show | | ☐ |
| PD-10 | Tag entries the filter cannot show are named | | ☐ |
| PD-11 | An env or thread count Grace cannot take is named | | ☐ |
| PD-12 | The notice names only what Grace does not use | | ☐ |
| PD-13 | Surefire passes the project's karate.env (e2e) | | ☐ |
| PD-14 | Maven's karate.env | | ☐ |
| PD-15 | karate-pom.json wins over pom.xml | | ☐ |
| PD-16 | An unused karate-pom.json env keeps pom.xml out | | ☐ |
| PD-17 | A Gradle project gets nothing from its build file | | ☐ |

### RP · Reorder the bottom panel tabs

[Spec](../specs/2026-11-04-reorder-panel-tabs.md) · [#94](https://github.com/hpagniez/grace/issues/94) · phase P3 · 14 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| RP-01 | The tabs start in their usual order | | ☐ |
| RP-02 | Variables stays after the three | | ☐ |
| RP-03 | Dragging a tab onto another takes its place | | ☐ |
| RP-04 | A drop marker shows where the tab will land | | ☐ |
| RP-05 | Dragging "HTTP" over "History" marks the left side | | ☐ |
| RP-06 | A drag that goes nowhere changes nothing | | ☐ |
| RP-07 | A drag keeps the shown tab | | ☐ |
| RP-08 | Something dragged in from outside is ignored | | ☐ |
| RP-09 | Alt+Right moves a tab one place right | | ☐ |
| RP-10 | Alt+Left moves a tab one place left, and stops at the first place | | ☐ |
| RP-11 | Moving a tab does not change the shown tab | | ☐ |
| RP-12 | The order is remembered | | ☐ |
| RP-13 | An unusable saved order shows the default | | ☐ |
| RP-14 | Storage that fails costs nothing | | ☐ |

### SO · Offer the sub-folder that holds the Karate project

[Spec](../specs/2026-11-04-subfolder-offer.md) · [#232](https://github.com/hpagniez/grace/issues/232) · phase P4 · 8 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| SO-01 | The empty tree offers the Karate project below | | ☐ |
| SO-02 | Picking the offer opens the sub-folder as a project | | ☐ |
| SO-03 | What looks like a Karate project | | ☐ |
| SO-04 | Grace does not look inside an offered folder | | ☐ |
| SO-05 | A folder with tests gets no offer | | ☐ |
| SO-06 | A failed dry run gets no offer | | ☐ |
| SO-07 | Grace opens only a folder it offered | | ☐ |
| SO-08 | A Maven project with nowhere to look for tests lists no libraries | | ☐ |

### TR · Trends

[Spec](../specs/2026-11-04-trends.md) · [#93](https://github.com/hpagniez/grace/issues/93) · phase P3 · 18 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| TR-01 | No runs, no strip | | ☐ |
| TR-02 | The strip covers every test with nothing selected | | ☐ |
| TR-03 | The strip follows the tree selection | | ☐ |
| TR-04 | A run that did not run the selection has no column | | ☐ |
| TR-05 | A Re-run failed counts only the tests it ran | | ☐ |
| TR-06 | Only runs of the picked environment count | | ☐ |
| TR-07 | A debug run has no column | | ☐ |
| TR-08 | A selection no stored run ran | | ☐ |
| TR-09 | Columns run oldest to newest | | ☐ |
| TR-10 | Pass rate splits passed below and failed above | | ☐ |
| TR-11 | One selected test is all passed or all failed | | ☐ |
| TR-12 | The newest value is rounded down | | ☐ |
| TR-13 | Time per test is the mean of the counted tests | | ☐ |
| TR-14 | A column's tooltip names the run | | ☐ |
| TR-15 | A click on a column finds its run in the table | | ☐ |
| TR-16 | A finished run updates the strip | | ☐ |
| TR-17 | Nothing is worked out while History is closed | | ☐ |
| TR-18 | Selecting fast asks once at a time | | ☐ |

## End-to-end tests

Every Playwright test in `e2e/*.spec.ts` at the go decision. Each one is ported to the UI Automation driver ([guide R6](2026-11-08-gpui-migration.md#r6-tests)) under the same name, or the row says why not.

### e2e/appearance.spec.ts

Phase P2 · 6 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-appearance-01 | Follow Windows is the default | | ☐ |
| E-appearance-02 | A saved Dark is in force at start, whatever Windows is | | ☐ |
| E-appearance-03 | Choosing a theme in Settings applies at once, with a project open, and is still there after a restart | | ☐ |
| E-appearance-04 | A theme change with a dialog up switches the window and the dialog and loses no state | | ☐ |
| E-appearance-05 | Escape closes a shown tooltip first, then the Settings page goes back | | ☐ |
| E-appearance-06 | Main refuses a theme it does not know and keeps the saved one | | ☐ |

### e2e/cancel.spec.ts

Phase P2 · 2 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-cancel-01 | Cancel leaves nothing behind | | ☐ |
| E-cancel-02 | Closing the app during a run | | ☐ |

### e2e/console-find.spec.ts

Phase P2 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-console-find-01 | find in a real run | | ☐ |
| E-console-find-02 | step through a failing run's errors | | ☐ |
| E-console-find-03 | a full console | | ☐ |

### e2e/console.spec.ts

Phase P2 · 2 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-console-01 | open, run and run again | | ☐ |
| E-console-02 | the console is on the sunken colour in both themes | | ☐ |

### e2e/cursor.spec.ts

Phase P2 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-cursor-01 | cursors match what a control does | | ☐ |
| E-cursor-02 | the busy cursor while running | | ☐ |
| E-cursor-03 | the drag cursor wins over the busy cursor | | ☐ |

### e2e/debug.spec.ts

Phase P4 · 2 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-debug-01 | Debugging from the tree: paused, stepping, finished | | ☐ |
| E-debug-02 | The debug controls at 760 px | | ☐ |

### e2e/details.spec.ts

Phase P2 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-details-01 | Failure details | | ☐ |

### e2e/editor.spec.ts

Phase P3 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-editor-01 | Open a scenario in VS Code | | ☐ |
| E-editor-02 | Open a scenario from the tree | | ☐ |
| E-editor-03 | main refuses what it must not open | | ☐ |

### e2e/env.spec.ts

Phase P2 · 13 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-env-01 | Suggestions come from overlay files and env tags | | ☐ |
| E-env-02 | Changing the environment updates the tree | | ☐ |
| E-env-03 | Changing the environment clears results | | ☐ |
| E-env-04 | A run uses the environment | | ☐ |
| E-env-05 | An environment without an overlay file | | ☐ |
| E-env-06 | Shared and environment properties, environment wins | | ☐ |
| E-env-07 | A secret is used but never shown | | ☐ |
| E-env-08 | Import a secrets file and run with it | | ☐ |
| E-env-09 | A secret stays off the command line and out of the settings file | | ☐ |
| E-env-10 | Reserved names are refused | | ☐ |
| E-env-11 | An unreadable secret blocks runs | | ☐ |
| E-env-12 | Settings survive a restart | | ☐ |
| E-env-13 | The machine's KARATE_ENV is ignored | | ☐ |

### e2e/examples.spec.ts

Phase P3 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-examples-01 | An outline's Examples as a table | | ☐ |

### e2e/failures.spec.ts

Phase P3 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-failures-01 | A broken karate-config.js, end to end (#229) | | ☐ |

### e2e/flaky.spec.ts

Phase P3 · 7 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-flaky-01 | A test that flips with no change is marked flaky | | ☐ |
| E-flaky-02 | A fix in the project is not flaky | | ☐ |
| E-flaky-03 | Different environments are not compared | | ☐ |
| E-flaky-04 | The proof opens a run's report | | ☐ |
| E-flaky-05 | The mark stays on its test when the tree changes | | ☐ |
| E-flaky-06 | Marks survive a restart | | ☐ |
| E-flaky-07 | Clear history removes the marks | | ☐ |

### e2e/history.spec.ts

Phase P3 · 14 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-history-01 | A run appears in the history | | ☐ |
| E-history-02 | History survives a restart | | ☐ |
| E-history-03 | A cancelled run is listed without a report | | ☐ |
| E-history-04 | Re-run failed runs only the failed tests | | ☐ |
| E-history-05 | A failed dynamic row is skipped | | ☐ |
| E-history-06 | Re-run switches back to the run's environment and filter | | ☐ |
| E-history-07 | Cancelling the environment switch of a re-run runs nothing | | ☐ |
| E-history-08 | A re-run of a test that no longer exists | | ☐ |
| E-history-09 | Only the 20 newest runs are kept | | ☐ |
| E-history-10 | Clear history | | ☐ |
| E-history-11 | A damaged run file does not hide the others | | ☐ |
| E-history-12 | A secret never reaches the history files | | ☐ |
| E-history-13 | History actions wait for the runner | | ☐ |
| E-history-14 | Export a run's report | | ☐ |

### e2e/home.spec.ts

Phase P2 · 14 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-home-01 | A first start shows the home screen | | ☐ |
| E-home-02 | Opening a folder from the home screen | | ☐ |
| E-home-03 | The last project opens at start without the home screen | | ☐ |
| E-home-04 | A missing last project shows the home screen | | ☐ |
| E-home-05 | A dry-run error still opens the project | | ☐ |
| E-home-06 | The home screen is on the ground in light and in dark | | ☐ |
| E-home-07 | Reopening a folder from the list | | ☐ |
| E-home-08 | The folder dialog starts next to the newest existing folder | | ☐ |
| E-home-09 | Removing a row | | ☐ |
| E-home-10 | The list works from the keyboard | | ☐ |
| E-home-11 | Home closes the project and survives a restart | | ☐ |
| E-home-12 | Home is disabled during a run | | ☐ |
| E-home-13 | A reload after Home stays home | | ☐ |
| E-home-14 | A path not in the list is refused | | ☐ |

### e2e/http.spec.ts

Phase P3 · 4 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-http-01 | Reading the calls of one Run all | | ☐ |
| E-http-02 | A call made in a called feature | | ☐ |
| E-http-03 | A secret never shows in a call | | ☐ |
| E-http-04 | A run of one test replaces only its calls | | ☐ |

### e2e/look.spec.ts

Phase P0 · 17 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-look-01 | The window follows Windows in ${scheme} mode | | ☐ |
| E-look-02 | Grace draws its own typefaces without any network request | | ☐ |
| E-look-03 | Each status has its own mark and name | | ☐ |
| E-look-04 | A cancelled run marks its interrupted test | | ☐ |
| E-look-05 | Reduced motion stops every animation | | ☐ |
| E-look-06 | The logo is the Home button | | ☐ |
| E-look-07 | A tooltip shows on keyboard focus and closes with Escape | | ☐ |
| E-look-08 | A tooltip shows after a pause on hover, also on a disabled button | | ☐ |
| E-look-09 | A row's tooltip leaves the next row's run button to the pointer | | ☐ |
| E-look-10 | Escape in a dialog closes a shown tooltip first, then the dialog | | ☐ |
| E-look-11 | Every control shows the focus ring in ${scheme} mode | | ☐ |
| E-look-12 | Targets are at least 24 by 24 | | ☐ |
| E-look-13 | A first start shows the first-run home screen | | ☐ |
| E-look-14 | A missing recent folder says so in words | | ☐ |
| E-look-15 | A folder with no feature files shows an empty state | | ☐ |
| E-look-16 | Before any run the progress line, the details and the tabs say so | | ☐ |
| E-look-17 | Clear history keeps its button order and closes on Escape | | ☐ |

### e2e/open.spec.ts

Phase P2 · 11 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-open-01 | Open a project and see its tests | | ☐ |
| E-open-02 | A feature opens on a click on its name, and as a run in it starts (#224) | | ☐ |
| E-open-03 | A narrow window keeps every toolbar button on screen, with its name | | ☐ |
| E-open-04 | A long project path stays readable at the minimum width | | ☐ |
| E-open-05 | The Env and Threads boxes show their longest likely values whole | | ☐ |
| E-open-06 | Maven layout with classpath resources | | ☐ |
| E-open-07 | Syntax error is reported, not hidden | | ☐ |
| E-open-08 | Nothing to run | | ☐ |
| E-open-09 | Open… starts in the last project's parent folder | | ☐ |
| E-open-10 | Open… with no last project | | ☐ |
| E-open-11 | the last project folder was deleted | | ☐ |

### e2e/panel.spec.ts

Phase P2 · 13 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-panel-01 | the bottom panel starts at its default height | | ☐ |
| E-panel-02 | dragging the splitter resizes the bottom panel | | ☐ |
| E-panel-03 | the splitter works from the keyboard | | ☐ |
| E-panel-04 | the panel stays within its limits | | ☐ |
| E-panel-05 | the window cannot get too short for the layout | | ☐ |
| E-panel-06 | the height survives a restart | | ☐ |
| E-panel-07 | a smaller window shrinks the panel without forgetting its height | | ☐ |
| E-panel-08 | a bad saved height falls back to the default | | ☐ |
| E-panel-09 | a saved height taller than the window is clamped at start | | ☐ |
| E-panel-10 | releasing a drag above the window keeps the maximum and ends the drag | | ☐ |
| E-panel-11 | focus cannot scroll the toolbar out of a cut-off layout | | ☐ |
| E-panel-12 | Tab moves focus past the splitter | | ☐ |
| E-panel-13 | the tabs can be dragged or moved with Alt+Left and Alt+Right, and their order survives a restart | | ☐ |

### e2e/project-setup.spec.ts

Phase P4 · 8 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-project-setup-01 | karate-pom.json gives the tests and the config, and a notice names the rest | | ☐ |
| E-project-setup-02 | karate-pom.json starts the environment, filter and thread count | | ☐ |
| E-project-setup-03 | Maven's karate.env starts the environment | | ☐ |
| E-project-setup-04 | Maven's compiled output is on the classpath | | ☐ |
| E-project-setup-05 | a Maven project's libraries come from its build | | ☐ |
| E-project-setup-06 | jars in .karate/ext are on the classpath | | ☐ |
| E-project-setup-07 | an empty folder offers the Karate project below it | | ☐ |
| E-project-setup-08 | the not-built notice builds the project's Java code | | ☐ |

### e2e/report.spec.ts

Phase P3 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-report-01 | Open the HTML report | | ☐ |

### e2e/run.spec.ts

Phase P2 · 4 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-run-01 | Run all with live progress | | ☐ |
| E-run-02 | Run one outline row | | ☐ |
| E-run-03 | A feature edited outside the app | | ☐ |
| E-run-04 | The project is not modified | | ☐ |

### e2e/scrollbar.spec.ts

Phase P2 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-scrollbar-01 | scroll areas get thin themed scrollbars | | ☐ |

### e2e/startup.spec.ts

Phase P2 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-startup-01 | the window opens painted in the app's theme | | ☐ |
| E-startup-02 | the page starts with a static startup screen | | ☐ |
| E-startup-03 | a reload shows the app once | | ☐ |

### e2e/tags.spec.ts

Phase P2 · 11 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-tags-01 | Chips list the project's tags | | ☐ |
| E-tags-02 | Include and exclude dim the tree | | ☐ |
| E-tags-03 | Third click turns a chip off | | ☐ |
| E-tags-04 | Hide non-matching | | ☐ |
| E-tags-05 | Run all runs only matching tests | | ☐ |
| E-tags-06 | A feature run is filtered | | ☐ |
| E-tags-07 | A tag removed before a run does not widen it | | ☐ |
| E-tags-08 | Part of an outline matches | | ☐ |
| E-tags-09 | Nothing matches | | ☐ |
| E-tags-10 | The filter survives a restart | | ☐ |
| E-tags-11 | A tag removed from the project leaves the filter | | ☐ |

### e2e/threads.spec.ts

Phase P2 · 8 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-threads-01 | A project with no saved count runs with 1 thread | | ☐ |
| E-threads-02 | Run all uses the count in the box | | ☐ |
| E-threads-03 | A single test's run uses the count | | ☐ |
| E-threads-04 | Re-run failed uses the current count | | ☐ |
| E-threads-05 | The box starts from karate-pom.json and the person's count wins | | ☐ |
| E-threads-06 | The box is disabled while a run is going | | ☐ |
| E-threads-07 | The count is saved per project | | ☐ |
| E-threads-08 | A value out of range is refused | | ☐ |

### e2e/trends.spec.ts

Phase P3 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-trends-01 | the strip follows the tree selection | | ☐ |
| E-trends-02 | a column finds its run in the table | | ☐ |
| E-trends-03 | a run on another environment has no column | | ☐ |

### e2e/updates.spec.ts

Phase P5 · 16 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-updates-01 | the status bar shows the version on every view (R3.1) | | ☐ |
| E-updates-02 | out of date on the pre-release channel shows the marker (R3.2) | | ☐ |
| E-updates-03 | the startup check reads an http feed once, through main (R2.1) | | ☐ |
| E-updates-04 | the last switch's result shows as a notice once the window is ready (R10.7) | | ☐ |
| E-updates-05 | the last switch's result outlasts the last project's reopening (R10.7) | | ☐ |
| E-updates-06 | main refuses update requests with invalid arguments | | ☐ |
| E-updates-07 | skipping a version keeps the marker but not the modal (R4.3) | | ☐ |
| E-updates-08 | a newer version after a skip shows the modal again | | ☐ |
| E-updates-09 | closing the modal without a choice asks again (R4.5) | | ☐ |
| E-updates-10 | a failed check is quiet (R2.5) | | ☐ |
| E-updates-11 | default channel follows the running version (R1.3) | | ☐ |
| E-updates-12 | version list and dev-build install button (R6.3, R6.4) | | ☐ |
| E-updates-13 | a channel change follows from the last check without a new request (R6.2) | | ☐ |
| E-updates-14 | Back returns to the open project unchanged (R6.1) | | ☐ |
| E-updates-15 | a failed check shows its reason in the settings (R2.5) | | ☐ |
| E-updates-16 | no network when the startup check is off (R2.3) | | ☐ |

### e2e/virtual.spec.ts

Phase P2 · 9 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-virtual-01 | Only the rows in view are drawn | | ☐ |
| E-virtual-02 | A moved-to row scrolls into view | | ☐ |
| E-virtual-03 | Focus survives scrolling away | | ☐ |
| E-virtual-04 | A new tree keeps the rows in view still | | ☐ |
| E-virtual-05 | The filter bar keeps one line with 54 tags | | ☐ |
| E-virtual-06 | A filter change scrolls to the selected row | | ☐ |
| E-virtual-07 | A click shows neither the focus ring nor a tooltip | | ☐ |
| E-virtual-08 | A click on an HTTP call shows no focus ring | | ☐ |
| E-virtual-09 | The keyboard shows the focus ring and a cut label's tooltip | | ☐ |

### e2e/watch.spec.ts

Phase P4 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-watch-01 | Re-run on save (docs/specs/2026-11-03-watch-mode.md) | | ☐ |

### e2e/window.spec.ts

Phase P2 · 12 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-window-01 | the window comes back where it was | | ☐ |
| E-window-02 | a maximized window comes back maximized | | ☐ |
| E-window-03 | a move by the person is saved before the app closes | | ☐ |
| E-window-04 | a window closed while minimized opens normal | | ☐ |
| E-window-05 | a failed save does not stop the app from closing | | ☐ |
| E-window-06 | a short saved window is held at the minimum height | | ☐ |
| E-window-07 | a broken saved state opens the window as on a first start | | ☐ |
| E-window-08 | a window partly off the screen is pulled onto it | | ☐ |
| E-window-09 | a window flush with the screen edges keeps its size and place | | ☐ |
| E-window-10 | a window bigger than the screen is shrunk to fit | | ☐ |
| E-window-11 | a window on a screen that is gone opens on the main screen | | ☐ |
| E-window-12 | a maximized window on a screen that is gone opens maximized on the main screen | | ☐ |

## Unit and component tests

Every Vitest file under `src/` at the go decision, with its number of tests. Logic tests move to `cargo test` in `grace-core` test for test, on the same fixtures; component tests become headless GPUI tests (`#[gpui::test]`) of the view that replaces the component ([guide R6](2026-11-08-gpui-migration.md#r6-tests)). A test that only checks something Electron, the DOM or Solid did (IPC plumbing, a Solid warning, a CSS class) is `n/a` with the reason. Tooling tests (`bench/`, `scripts/`) stay in Node and are not listed.

| File | Tests | Moves to | Ported |
|---|---|---|---|
| `src/main/batch.test.ts` | 2 | `grace-core::batch::tests` (4 tests, every Vitest test and the added ones) | ☑ |
| `src/main/editor.test.ts` | 10 | `grace-core::editor` | ☐ |
| `src/main/fingerprint.test.ts` | 10 | `grace-core::fingerprint::tests` (20 tests, every Vitest test and the added ones) | ☑ |
| `src/main/flaky.test.ts` | 19 | `grace-core::flaky::tests` (20 tests) | ☑ |
| `src/main/history.test.ts` | 19 | `grace-core::history::tests` (37 tests) | ☑ |
| `src/main/karate/argfile.test.ts` | 9 | `grace-core::karate::argfile::tests` (12 tests, every Vitest test and the added ones) | ☑ |
| `src/main/karate/command.test.ts` | 46 | `grace-core::karate::command::tests` (51 tests: the 46 Vitest tests on the same strings, and 5 for paths and the Linux classpath) | ☑ |
| `src/main/karate/compiled.test.ts` | 11 | `grace-core::karate::compiled` | ☐ |
| `src/main/karate/debug.test.ts` | 8 | `grace-core::karate::debug` | ☐ |
| `src/main/karate/environments.test.ts` | 10 | `grace-core::karate::environments::tests` (11 tests) | ☑ |
| `src/main/karate/events.test.ts` | 7 | `grace-core::karate::events::tests` (13 tests, every Vitest test and the added ones) | ☑ |
| `src/main/karate/http.test.ts` | 19 | `grace-core::karate::http::tests` (20 tests, every Vitest test and the added ones) | ☑ |
| `src/main/karate/libraries.test.ts` | 29 | `grace-core::karate::libraries` | ☐ |
| `src/main/karate/maven.test.ts` | 19 | `grace-core::karate::maven` | ☐ |
| `src/main/karate/pom.test.ts` | 19 | `grace-core::karate::pom` | ☐ |
| `src/main/karate/process.test.ts` | 9 | `grace-core::karate::process::tests` (11 tests; the job object, Windows priority and `taskkill` bodies run on Windows CI) | ☑ |
| `src/main/karate/run-state.test.ts` | 24 | `grace-core::karate::run_state::tests` (26 tests, every Vitest test and the added ones) | ☑ |
| `src/main/karate/steps.test.ts` | 8 | `grace-core::karate::steps::tests` (9 tests, every Vitest test and the added ones) | ☑ |
| `src/main/karate/subfolders.test.ts` | 10 | `grace-core::karate::subfolders` | ☐ |
| `src/main/karate/tree.test.ts` | 17 | `grace-core::karate::tree::tests` (20 tests, every Vitest test and the added ones) | ☑ |
| `src/main/mask.test.ts` | 15 | `grace-core::mask::tests` (26 tests; two gaps of Electron Grace fixed, #293) | ☑ |
| `src/main/propertiesFile.test.ts` | 12 | `grace-core::properties_file::tests` (13 tests, every Vitest test and the added ones) | ☑ |
| `src/main/recent.test.ts` | 6 | `grace-core::recent::tests` (7 tests) | ☑ |
| `src/main/report-zip.test.ts` | 6 | `grace-core::report-zip` | ☐ |
| `src/main/runner.debug.test.ts` | 11 | `grace-core::runner` (debug) | ☐ |
| `src/main/runner.discovery.test.ts` | 11 | `grace-core::runner` (discovery) | ☐ |
| `src/main/runner.editor.test.ts` | 2 | `grace-core::runner` (editor) | ☐ |
| `src/main/runner.env.test.ts` | 12 | `grace-core::runner` (env) | ☐ |
| `src/main/runner.fingerprint.test.ts` | 10 | `grace-core::runner` (fingerprint) | ☐ |
| `src/main/runner.flaky.test.ts` | 3 | `grace-core::runner` (flaky) | ☐ |
| `src/main/runner.history.test.ts` | 15 | `grace-core::runner` (history) | ☐ |
| `src/main/runner.home.test.ts` | 13 | `grace-core::runner` (home) | ☐ |
| `src/main/runner.libraries.test.ts` | 31 | `grace-core::runner` (libraries) | ☐ |
| `src/main/runner.pom.test.ts` | 12 | `grace-core::runner` (pom) | ☐ |
| `src/main/runner.properties.test.ts` | 18 | `grace-core::runner` (properties) | ☐ |
| `src/main/runner.rerun.test.ts` | 11 | `grace-core::runner` (rerun) | ☐ |
| `src/main/runner.runs.test.ts` | 13 | `grace-core::runner` (runs) | ☐ |
| `src/main/runner.setup.test.ts` | 9 | `grace-core::runner` (setup) | ☐ |
| `src/main/runner.subfolders.test.ts` | 7 | `grace-core::runner` (subfolders) | ☐ |
| `src/main/runner.tags.test.ts` | 13 | `grace-core::runner` (tags) | ☐ |
| `src/main/runner.threads.test.ts` | 8 | `grace-core::runner` (threads) | ☐ |
| `src/main/runner.trends.test.ts` | 2 | `grace-core::runner` (trends) | ☐ |
| `src/main/runner.watch.test.ts` | 18 | `grace-core::runner` (watch) | ☐ |
| `src/main/secrets.test.ts` | 6 | `grace-core::secrets::tests` (10 tests, every Vitest test and the added ones) | ☑ |
| `src/main/settings.test.ts` | 62 | `grace-core::settings::tests` (80 tests: every Vitest case, `it.each` rows one test each, and the bytes Node writes) | ☑ |
| `src/main/trends.test.ts` | 10 | `grace-core::trends::tests` (11 tests) | ☑ |
| `src/main/updates/check.test.ts` | 11 | `grace-core::updates::check` | ☐ |
| `src/main/updates/fetchZip.test.ts` | 11 | `grace-core::updates::fetchZip` | ☐ |
| `src/main/updates/original-fs.test.ts` | 1 | `grace-core::updates::original-fs` | ☐ |
| `src/main/updates/preflight.test.ts` | 14 | `grace-core::updates::preflight` | ☐ |
| `src/main/updates/startup.test.ts` | 32 | `grace-core::updates::startup` | ☐ |
| `src/main/updates/switch.test.ts` | 22 | `grace-core::updates::switch` | ☐ |
| `src/main/updates/updater.test.ts` | 36 | `grace-core::updates::updater` | ☐ |
| `src/main/watch.test.ts` | 13 | `grace-core::watch` | ☐ |
| `src/main/window.test.ts` | 5 | `grace-core::window::tests` (13 tests, every Vitest test and the added ones) | ☑ |
| `src/renderer/Banner.test.tsx` | 10 | view `Banner` (`grace::ui`), headless | ☐ |
| `src/renderer/BottomPanel.test.tsx` | 17 | view `BottomPanel` (`grace::ui`), headless | ☐ |
| `src/renderer/CallDetails.test.tsx` | 16 | view `CallDetails` (`grace::ui`), headless | ☐ |
| `src/renderer/ConfigBanner.test.tsx` | 6 | view `ConfigBanner` (`grace::ui`), headless | ☐ |
| `src/renderer/Console.test.tsx` | 30 | view `Console` (`grace::ui`), headless | ☐ |
| `src/renderer/Details.test.tsx` | 33 | view `Details` (`grace::ui`), headless | ☐ |
| `src/renderer/EmptyState.test.tsx` | 3 | view `EmptyState` (`grace::ui`), headless | ☐ |
| `src/renderer/EnvPicker.test.tsx` | 18 | view `EnvPicker` (`grace::ui`), headless | ☐ |
| `src/renderer/Examples.test.tsx` | 10 | view `Examples` (`grace::ui`), headless | ☐ |
| `src/renderer/FieldMessage.test.tsx` | 2 | view `FieldMessage` (`grace::ui`), headless | ☐ |
| `src/renderer/FilterBar.test.tsx` | 21 | view `FilterBar` (`grace::ui`), headless | ☐ |
| `src/renderer/FlakyProof.test.tsx` | 8 | view `FlakyProof` (`grace::ui`), headless | ☐ |
| `src/renderer/History.test.tsx` | 30 | view `History` (`grace::ui`), headless | ☐ |
| `src/renderer/Home.test.tsx` | 26 | view `Home` (`grace::ui`), headless | ☐ |
| `src/renderer/HttpCalls.test.tsx` | 26 | view `HttpCalls` (`grace::ui`), headless | ☐ |
| `src/renderer/Icon.test.tsx` | 3 | view `Icon` (`grace::ui`), headless | ☐ |
| `src/renderer/Notice.test.tsx` | 6 | view `Notice` (`grace::ui`), headless | ☐ |
| `src/renderer/Progress.test.tsx` | 18 | view `Progress` (`grace::ui`), headless | ☐ |
| `src/renderer/PropertiesDialog.test.tsx` | 54 | view `PropertiesDialog` (`grace::ui`), headless | ☐ |
| `src/renderer/Settings.test.tsx` | 43 | view `Settings` (`grace::ui`), headless | ☐ |
| `src/renderer/StatusBar.test.tsx` | 18 | view `StatusBar` (`grace::ui`), headless | ☐ |
| `src/renderer/StatusMark.test.tsx` | 8 | view `StatusMark` (`grace::ui`), headless | ☐ |
| `src/renderer/SwitchDialog.test.tsx` | 12 | view `SwitchDialog` (`grace::ui`), headless | ☐ |
| `src/renderer/Threads.test.tsx` | 6 | view `Threads` (`grace::ui`), headless | ☐ |
| `src/renderer/Toolbar.test.tsx` | 36 | view `Toolbar` (`grace::ui`), headless | ☐ |
| `src/renderer/Tooltip.test.tsx` | 46 | view `Tooltip` (`grace::ui`), headless | ☐ |
| `src/renderer/Tree.test.tsx` | 86 | view `Tree` (`grace::ui`), headless | ☐ |
| `src/renderer/Trends.test.tsx` | 15 | view `Trends` (`grace::ui`), headless | ☐ |
| `src/renderer/UpdateModal.test.tsx` | 13 | view `UpdateModal` (`grace::ui`), headless | ☐ |
| `src/renderer/UpdateProgress.test.tsx` | 13 | view `UpdateProgress` (`grace::ui`), headless | ☐ |
| `src/renderer/Variables.test.tsx` | 4 | view `Variables` (`grace::ui`), headless | ☐ |
| `src/renderer/VirtualList.test.tsx` | 28 | view `VirtualList` (`grace::ui`), headless | ☐ |
| `src/renderer/bannerText.test.ts` | 15 | `grace::ui::bannerText` | ☐ |
| `src/renderer/buttons.test.ts` | 2 | `grace::ui::buttons` | ☐ |
| `src/renderer/consoleFind.test.ts` | 9 | `grace::ui::consoleFind` | ☐ |
| `src/renderer/failures.test.ts` | 10 | `grace::ui::failures` | ☐ |
| `src/renderer/http.test.ts` | 9 | `grace::ui::http` | ☐ |
| `src/renderer/importPlan.test.ts` | 6 | `grace::ui::importPlan` | ☐ |
| `src/renderer/lastChecked.test.ts` | 4 | `grace::ui::lastChecked` | ☐ |
| `src/renderer/nodes.test.ts` | 3 | `grace::ui::nodes` | ☐ |
| `src/renderer/palette.test.ts` | 6 | `crates/grace::theme::tests::colours_come_only_from_the_theme`, which scans every file but `theme.rs` and knows its patterns. Its `dark:` and `title=` checks have nothing to find in GPUI, which has neither | ☑ |
| `src/renderer/panelHeight.test.ts` | 8 | `grace::ui::panelHeight` | ☐ |
| `src/renderer/panelTabs.test.ts` | 7 | `grace::ui::panelTabs` | ☐ |
| `src/renderer/placeTip.test.ts` | 16 | `grace::ui::placeTip` | ☐ |
| `src/renderer/reduce.test.ts` | 45 | `grace::ui::reduce` | ☐ |
| `src/renderer/store.test.tsx` | 8 | view `store` (`grace::ui`), headless | ☐ |
| `src/renderer/tokens.test.ts` | 19 | `crates/grace::theme::tests`: every token in both themes and equal to `src/index.css` until the cut-over, the contrast rules, the primary's green, the focus ring, the sheet and the shadow's geometry. The CSS-only checks (the `::highlight` rules, `color-scheme`, Tailwind's palette reset) have no GPUI counterpart; the console's highlights port with the console (CS-*) | ☑ |
| `src/renderer/treeRows.test.ts` | 5 | `grace::ui::treeRows` | ☐ |
| `src/renderer/trendLabels.test.ts` | 7 | `grace::ui::trendLabels` | ☐ |
| `src/renderer/ui.test.ts` | 1 | `grace::ui::ui` | ☐ |
| `src/renderer/virtual.test.ts` | 15 | `grace::ui::virtual` | ☐ |
| `src/shared/env.test.ts` | 29 | `grace-core::model::env::tests` (54 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/flaky.test.ts` | 6 | `grace-core::model::flaky::tests` (7 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/history.test.ts` | 18 | `grace-core::model::history::tests` (22 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/http.test.ts` | 19 | `grace-core::model::http::tests` (20 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/sanity.test.ts` | 1 | Checked that Vitest runs; `cargo test` needs no such check | n/a |
| `src/shared/status.test.ts` | 9 | `grace-core::model::status::tests` (10 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/tags.test.ts` | 14 | `grace-core::model::tags::tests` (15 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/theme.test.ts` | 4 | `grace-core::model::theme::tests` (5 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/threads.test.ts` | 3 | `grace-core::model::threads::tests` (14 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/updates.test.ts` | 18 | `grace-core::model::updates::tests` (21 tests, every Vitest test and the added ones) | ☑ |
| `src/shared/version.test.ts` | 6 | `grace-core::model::version::tests` (7 tests, every Vitest test and the added ones) | ☑ |

