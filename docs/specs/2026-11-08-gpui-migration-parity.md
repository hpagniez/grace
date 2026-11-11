# GPUI migration: parity checklist

Date 2026-11-08. Part of the [GPUI migration guide](2026-11-08-gpui-migration.md). Status: porting, phase P2.

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
| WS-01 | Open a project and see its tests | `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` | ☑ |
| WS-02 | Maven layout with classpath resources | `grace-e2e/tests/open.rs::maven_layout_with_classpath_resources` | ☑ |
| WS-03 | Syntax error is reported, not hidden | `grace-e2e/tests/open.rs::syntax_error_is_reported_not_hidden` | ☑ |
| WS-04 | Run all with live progress | `grace-e2e/tests/run.rs::run_all_with_live_progress` (the rows turning running, passed and failed) | ☑ |
| WS-05 | Run one outline row | `grace-e2e/tests/run.rs::run_one_outline_row` | ☑ |
| WS-06 | Failure details | `grace-e2e/tests/details.rs::failure_details` (the place, the error with Karate's comment, the 5 steps and a step's log after a run) | ☑ |
| WS-07 | A feature edited outside the app | `grace-e2e/tests/run.rs::a_feature_edited_outside_the_app` | ☑ |
| WS-08 | Cancel leaves nothing behind | `grace-e2e/tests/run.rs::cancel_leaves_nothing_behind` ("slow loop" interrupted) | ☑ |
| WS-09 | Closing the app during a run | `grace-e2e/tests/run.rs::closing_the_app_during_a_run` | ☑ |
| WS-10 | Nothing to run | `grace-e2e/tests/run.rs::nothing_to_run` (the toolbar and the tree's "No feature files found"), `grace::toolbar::tests::the_actions::run_all_is_disabled_with_nothing_to_run` | ☑ |
| WS-11 | Open the HTML report | `grace-e2e/tests/run.rs::report_opens_the_runs_html_report` | ☑ |
| WS-12 | The project is not modified | `grace-e2e/tests/run.rs::the_project_is_not_modified` | ☑ |

### TF · Tag filter: run by tag

[Spec](../specs/2026-10-24-tag-filter.md) · [#55](https://github.com/hpagniez/grace/issues/55) · phase P2 · 11 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| TF-01 | Chips list the project's tags | `grace-e2e/tests/tags.rs::chips_list_the_projects_tags` | ☑ |
| TF-02 | Include and exclude dim the tree | `grace-e2e/tests/tags.rs::include_and_exclude_dim_the_tree` | ☑ |
| TF-03 | Third click turns a chip off | `grace-e2e/tests/tags.rs::a_third_click_turns_a_chip_off` | ☑ |
| TF-04 | Hide non-matching | `grace-e2e/tests/tags.rs::hide_non_matching_hides_the_other_rows` | ☑ |
| TF-05 | Run all runs only matching tests | `grace-e2e/tests/tags.rs::run_all_runs_only_matching_tests` | ☑ |
| TF-06 | A feature run is filtered | `grace-e2e/tests/tags.rs::a_feature_run_is_filtered` | ☑ |
| TF-07 | A tag removed before a run does not widen it | `grace-e2e/tests/tags.rs::a_tag_removed_before_a_run_does_not_widen_it` | ☑ |
| TF-08 | Part of an outline matches | `grace-e2e/tests/tags.rs::part_of_an_outline_matches` | ☑ |
| TF-09 | Nothing matches | `grace-e2e/tests/tags.rs::nothing_matches` (the counter); Run all disabled: `grace::toolbar::tests` (WS-10) | ☑ |
| TF-10 | The filter survives a restart | `grace-e2e/tests/tags.rs::the_filter_survives_a_restart` | ☑ |
| TF-11 | A tag removed from the project leaves the filter | `grace-e2e/tests/tags.rs::a_tag_removed_from_the_project_leaves_the_filter` | ☑ |

### EV · Environment switcher and run properties

[Spec](../specs/2026-10-25-environment-switcher.md) · [#59](https://github.com/hpagniez/grace/issues/59) · phase P2 · 12 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| EV-01 | Suggestions come from overlay files and env tags | `grace-e2e/tests/env.rs::suggestions_come_from_overlay_files_and_env_tags`; `grace::env_picker::tests::words::suggests_none_then_the_environments_that_hold_the_text` | ☑ |
| EV-02 | Changing the environment updates the tree | `grace-e2e/tests/env.rs::changing_the_environment_updates_the_tree` | ☑ |
| EV-03 | Changing the environment clears results | `grace-e2e/tests/env.rs::changing_the_environment_clears_results` | ☑ |
| EV-04 | A run uses the environment | `grace-e2e/tests/env.rs::a_run_uses_the_environment` | ☑ |
| EV-05 | An environment without an overlay file | `grace-e2e/tests/env.rs::an_environment_without_an_overlay_file`; `grace::env_picker::tests::says_when_the_environment_has_no_overlay_file` | ☑ |
| EV-06 | Shared and environment properties, environment wins | `grace-e2e/tests/properties.rs::shared_and_environment_properties_environment_wins`; `grace-core::runner::tests::properties::shared_and_env_properties_env_wins` | ☑ |
| EV-07 | A secret is used but never shown | `grace-e2e/tests/properties.rs::a_secret_is_used_but_never_shown` (Windows); `grace-core::runner::tests::properties::a_secret_reaches_the_run_and_is_masked` | ☑ |
| EV-08 | A secret stays off the command line and out of the settings file | `grace-e2e/tests/properties.rs::a_secret_stays_off_the_command_line_and_out_of_the_settings_file` (Windows); `grace-core::runner::tests::properties::the_secret_is_not_on_the_command_line`, `grace-core::runner::tests::properties::settings_never_hold_the_plain_secret` | ☑ |
| EV-09 | Reserved names are refused | `grace-e2e/tests/properties.rs::reserved_names_are_refused`; `grace-core::properties_draft::tests::a_reserved_name_is_an_error_in_any_letter_case` | ☑ |
| EV-10 | An unreadable secret blocks runs | `grace-e2e/tests/properties.rs::an_unreadable_secret_blocks_runs`; `grace-core::runner::tests::properties::an_unreadable_secret_blocks_the_run` | ☑ |
| EV-11 | Settings survive a restart | `grace-e2e/tests/env.rs::the_environment_survives_a_restart`; `grace-e2e/tests/properties.rs::the_properties_survive_a_restart` (Windows) | ☑ |
| EV-12 | The machine's KARATE_ENV is ignored | `grace-e2e/tests/env.rs::the_machines_karate_env_is_ignored` | ☑ |

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
| TC-01 | A project with no saved count runs with 1 thread | `grace-e2e/tests/threads.rs::a_project_with_no_saved_count_runs_with_1_thread` | ☑ |
| TC-02 | Run all uses the count in the box | `grace-e2e/tests/threads.rs::run_all_uses_the_count_in_the_box` | ☑ |
| TC-03 | A single test's run uses the count | `grace-e2e/tests/threads.rs::a_single_tests_run_uses_the_count` | ☑ |
| TC-04 | Re-run failed uses the current count, not the run's | `grace-e2e/tests/threads.rs::rerun_failed_uses_the_current_count`; `grace-core::runner::tests::threads::re_run_uses_the_current_count_not_the_runs` | ☑ |
| TC-05 | The box overrides the thread count of a karate-pom.json | `grace-e2e/tests/threads.rs::the_box_starts_from_karate_pom_json_and_the_persons_count_wins` | ☑ |
| TC-06 | The box is disabled while a run is going | `grace-e2e/tests/threads.rs::the_box_is_disabled_while_a_run_is_going`; `grace::threads_box::tests::disabled_while_busy_and_sends_nothing` | ☑ |
| TC-07 | Typing a number sends it once (unit) | `grace::threads_box::tests::sends_a_committed_value_once` | ☑ |
| TC-08 | Discovery passes one thread (unit) | `grace-core::runner::tests::threads::a_run_passes_the_count_and_discovery_passes_1` | ☑ |
| TC-09 | The count is saved per project | `grace-e2e/tests/threads.rs::the_count_is_saved_per_project`; `grace-core::runner::tests::threads::restores_the_count_on_open_per_project` | ☑ |
| TC-10 | A value out of range is refused | `grace-e2e/tests/threads.rs::a_value_out_of_range_is_refused`; `grace::threads_box::tests::a_refused_value_stays_until_the_runner_resends_its_count` | ☑ |
| TC-11 | Main refuses a count that is not a whole number from 1 to 32 (unit) | `grace-core::runner::tests::threads::refuses_a_count_that_is_not_a_whole_number_from_1_to_32` | ☑ |
| TC-12 | The count is refused while busy (unit) | `grace-core::runner::tests::threads::rejects_a_change_while_busy` | ☑ |
| TC-13 | Saving the count keeps the other settings (unit) | `grace-core::settings::tests::thread_count::saving_the_count_keeps_the_other_settings` | ☑ |
| TC-14 | A malformed saved count loads as 1 (unit) | `grace-core::settings::tests::thread_count::a_malformed_saved_count_*_loads_as_1` (5 values), `an_unreadable_file_loads_as_1` | ☑ |
| TC-15 | A reloaded window shows the count (unit) | `grace::threads_box::tests::a_new_box_shows_the_count_in_force` (no reload in GPUI Grace: a new box reads the state the runner left) | ☑ |
| TC-16 | A run stored before F12 has no thread count (unit) | `grace-core::model::history::tests::parse_run_record::reads_a_record_stored_before_f12_as_no_thread_count` | ☑ |
| TC-17 | A run.json with a malformed thread count is dropped (unit) | `grace-core::model::history::tests::parse_run_record::rejects_bad_shapes` (`threads` 0, "2", 1.5) | ☑ |

### UP · UI polish

[Spec](../specs/2026-10-26-ui-polish.md) · [#82](https://github.com/hpagniez/grace/issues/82) [#87](https://github.com/hpagniez/grace/issues/87) [#88](https://github.com/hpagniez/grace/issues/88) [#89](https://github.com/hpagniez/grace/issues/89) [#90](https://github.com/hpagniez/grace/issues/90) [#91](https://github.com/hpagniez/grace/issues/91) · phase P2 · 20 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| UP-01 | The bottom panel starts at its default height | `grace-e2e/tests/panel.rs::the_bottom_panel_starts_at_its_default_height`; `bottom_panel::tests::splitter::starts_at_the_default_height` | ☑ |
| UP-02 | Dragging the splitter resizes the bottom panel | `bottom_panel::tests::splitter::dragging_resizes_within_the_limits_and_saves_on_release` (GPUI's simulated pointer; a real drag by hand, PR 6) | ☑ |
| UP-03 | The splitter works from the keyboard | `grace-e2e/tests/panel.rs::the_splitter_works_from_the_keyboard`; `bottom_panel::tests::splitter::the_keys_step_and_jump_and_save` | ☑ |
| UP-04 | The panel stays within its limits | `panel_height::tests`; `bottom_panel::tests::splitter::dragging_resizes_within_the_limits_and_saves_on_release`; `grace-e2e/tests/panel.rs::a_saved_height_taller_than_the_window_is_clamped_at_start` | ☑ |
| UP-05 | The window cannot get too short for the layout | `grace::window::MIN_SIZE` (760 × 480) and the main area's `min_h(MAIN_MIN)`; `bottom_panel::tests::splitter::a_smaller_window_shrinks_the_panel_without_forgetting_its_height` | ☑ |
| UP-06 | The height survives a restart | `grace-e2e/tests/panel.rs::the_height_survives_a_restart`; `settings::tests::panel_height` | ☑ |
| UP-07 | A smaller window shrinks the panel without forgetting its height | `bottom_panel::tests::splitter::a_smaller_window_shrinks_the_panel_without_forgetting_its_height` | ☑ |
| UP-08 | A bad saved height falls back to the default | `grace-e2e/tests/panel.rs::a_bad_saved_height_falls_back_to_the_default`; `settings::tests::panel_height` | ☑ |
| UP-09 | Open… starts in the last project's parent folder | `grace-e2e/tests/open.rs::open_starts_in_the_last_projects_parent_folder` | ☑ |
| UP-10 | Open… with no last project | `grace-e2e/tests/open.rs::open_with_no_last_project` | ☑ |
| UP-11 | The last project folder was deleted | `grace-e2e/tests/open.rs::the_last_project_folder_was_deleted` | ☑ |
| UP-12 | The Windows folder dialog shows the siblings (by hand) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): opens in the parent folder with the siblings listed, modal | ☑ |
| UP-13 | Cursors match what a control does | `grace::elements::tests::a_button_shows_the_hand_and_a_disabled_one_the_not_allowed_sign`, `grace::tree::tests::a_rows_run_button_shows_the_hand`; by hand in the P2 exit check and [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md): the hand on buttons, labels and a row's Run button, not-allowed on a control turned off, the text cursor in the boxes | ☑ |
| UP-14 | The busy cursor while running | No busy cursor in GPUI Grace: GPUI has none, and adding one to the Windows patch is more stack to carry; the progress bar, the status bar and the running marks show the run ([P2 plan, Redesigns](../plans/2026-11-09-gpui-p2-main-screen.md#redesigns)) | n/a |
| UP-15 | Scroll areas get thin themed scrollbars | `grace::theme::tests::scrollbars_are_thin_themed_and_always_shown` (the tree, details, console and lists; History and HTTP with phase P3) | ☑ |
| UP-16 | Scrollbars look right and are easy to use (by hand) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): the tree in dark, thin thumb, hover and drag | ☑ |
| UP-17 | The window opens painted in the app's theme | `grace-e2e/tests/appearance.rs::follow_windows_is_the_default`, `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start` (the ground in each theme); the window shows once its first frame is drawn (#90) | ☑ |
| UP-18 | No white flash (by hand) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): 10 starts, 5 dark and 5 light, no white or wrong-theme frame (Windows' own theme switch is for a person) | ☑ |
| UP-19 | The window appears even if the page fails (by hand) | n/a: GPUI Grace draws its window itself and has no page that could fail ([the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md)) | ☑ |
| UP-20 | A second launch brings the window up (by hand) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): while the first window starts and while it is minimised | ☑ |

### WB · Window bounds

[Spec](../specs/2026-10-26-window-bounds.md) · [#92](https://github.com/hpagniez/grace/issues/92) · phase P2 · 18 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| WB-01 | The window comes back where it was | `grace-e2e/tests/window.rs::the_window_comes_back_where_it_was` | ☑ |
| WB-02 | A maximized window comes back maximized | `grace-e2e/tests/window.rs::a_maximized_window_comes_back_maximized` (Windows only: Xvfb has no window manager to maximize) | ☑ |
| WB-03 | A move by the person is saved before the app closes | `grace-e2e/tests/window.rs::a_move_by_the_person_is_saved_before_the_app_closes` | ☑ |
| WB-04 | The saved state keeps the other settings (unit) | `grace-core::settings::tests::window_state::saving_the_window_keeps_the_other_settings` | ☑ |
| WB-05 | A window closed while minimized opens normal | `grace-e2e/tests/window.rs::a_window_closed_while_minimized_opens_normal` (Windows only: Xvfb has no window manager to maximize) | ☑ |
| WB-06 | A failed save does not stop the app from closing | `grace-e2e/tests/window.rs::a_failed_save_does_not_stop_the_app_from_closing` | ☑ |
| WB-07 | A cut-off save leaves the old settings whole (unit) | `grace-core::settings::tests::window_state::a_failed_rename_leaves_the_file_as_it_was` | ☑ |
| WB-08 | A short saved window is held at the minimum height | `grace::window::tests::a_short_saved_window_is_held_at_the_minimum_height` | ☑ |
| WB-09 | A broken saved state opens the window as on a first start | `grace-core::settings::tests::window_state::load_window_state_reads_a_broken_state_as_nothing_saved`, `grace::window::tests::a_broken_or_missing_state_opens_as_on_a_first_start` | ☑ |
| WB-10 | A window partly off the screen is pulled onto it | `grace-e2e/tests/window.rs::a_window_partly_off_the_screen_is_pulled_onto_it` | ☑ |
| WB-11 | A window bigger than the screen is shrunk to fit | `grace-e2e/tests/window.rs::a_window_bigger_than_the_screen_is_shrunk_to_fit` | ☑ |
| WB-12 | A window on a screen that is gone opens on the main screen | `grace::window::tests::a_window_on_a_screen_that_is_gone_opens_on_the_main_screen` | ☑ |
| WB-13 | A maximized window on a screen that is gone opens maximized on the main screen | `grace::window::tests::a_window_on_a_screen_that_is_gone_opens_on_the_main_screen` (the maximized case) | ☑ |
| WB-14 | Placement on two screens (unit) | `grace-core::window::tests` (13 tests: two screens, edges, flush, shrink) | ☑ |
| WB-15 | Two screens (by hand) | #317; [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): bounds and maximized on screen 2 | ☑ |
| WB-16 | The second screen is gone (by hand) | #317; [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): passes with saved bounds where no screen is. By hand on the owner's PC on 2026-11-11, the release build of `develop` at 029a401: with the screen Grace was on unplugged, Grace came back on another screen | ☑ |
| WB-17 | Partly off the screen (by hand) | #317; [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md) | ☑ |
| WB-18 | A snapped window (by hand) | `grace::window::tests::a_snapped_window_saves_where_it_is_and_a_maximized_or_minimized_one_its_normal_bounds` (#318); [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md): a Win+Left snap on the main screen and a Win+Right snap on the second reopen exactly where they were over 3 restarts, and a normal window does not drift | ☑ |
| WB-19 | A first window on a small screen fits it whole, title bar included, as Electron's `BrowserWindow` fits its window to the work area (#346) | `grace-e2e/tests/window.rs::a_first_window_keeps_its_title_bar_on_the_screen` (Windows: CI's screen is 1024×768) | ☑ |

### BD · Bundled dependency updates

[Spec](../specs/2026-10-27-bundled-dependency-updates.md) · [#84](https://github.com/hpagniez/grace/issues/84) · phase P0 · 14 scenarios

The vendor pipeline is tooling and stays in Node ([guide](2026-11-08-gpui-migration.md#goals-non-goals-anti-goals), non-goal). It serves both apps: GPUI Grace's tests, e2e and package take the same pins from `scripts/vendor.json`.

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| BD-01 | Nothing new, nothing opened | `scripts/bump-vendor.test.ts` pickKarate "keeps 2.1.4 when v2.1.4 is the newest", pickJre "keeps the pin when Adoptium's latest is the pinned build", bump "leaves an up-to-date JRE alone" | ☑ |
| BD-02 | A new Karate release opens a draft PR | `scripts/bump-vendor.test.ts` bump "bumps Karate and its LICENSE, and nothing else"; `scripts/bump-pr.test.ts` openBumpPr "commits, pushes, opens the draft and supersedes the older bot-only draft", titleFor, runCommandStatus, prBody "lists what a Karate reviewer must check" | ☑ |
| BD-03 | A new JRE opens its own PR | `scripts/bump-vendor.test.ts` pickJre "picks a newer build with Adoptium's link and checksum"; `scripts/bump-pr.test.ts` prBody "links the Adoptium release for a JRE bump" | ☑ |
| BD-04 | Marking the PR ready runs full CI | `test.yml`'s `rust` job keys its `vendor` cache on `scripts/vendor.json` and fetches the new pins with `scripts/fetch-vendor.mjs`, so a bump PR runs `grace-core`'s real-Java tests and GPUI Grace's Linux e2e on them; Windows e2e on a run by hand. Electron Grace's e2e on a bump PR follows ADR 0017 (by hand or into `main`) | ☑ |
| BD-05 | A handled version is not opened again | `scripts/bump-pr.test.ts` openBumpPr "stops when the version's branch already has a PR" (`--state all`: open or closed) | ☑ |
| BD-06 | A pushed branch without a PR gets its PR | `scripts/bump-pr.test.ts` openBumpPr "opens the missing PR of a branch already pushed, without committing" | ☑ |
| BD-07 | A newer version supersedes a bot-only draft | `scripts/bump-pr.test.ts` supersedePlan "closes only older bot-only drafts of the same dependency", openBumpPr "commits, pushes, opens the draft and supersedes the older bot-only draft" | ☑ |
| BD-08 | An older bump PR with a person's work stays open | `scripts/bump-pr.test.ts` supersedePlan "closes only older bot-only drafts of the same dependency", prBody "says unknown, check by hand, and names kept PRs" | ☑ |
| BD-09 | Versions that are never picked (unit) | `scripts/bump-vendor.test.ts` pickKarate "keeps 2.1.4 when …" (the newest, an older one, a new major, a pre-release, a draft, a release candidate tag) | ☑ |
| BD-10 | Numbers compare as numbers (unit) | `scripts/bump-vendor.test.ts` compareVersions "compares part by part as numbers", pickKarate "picks the highest by number, not by text" | ☑ |
| BD-11 | Missing data fails and changes nothing (unit) | `scripts/bump-vendor.test.ts` pickKarate "fails when the jar asset is missing", "fails when the jar has no sha256 digest", pickJre "fails without a checksum", bump "fails and changes nothing when the releases API answers 500" | ☑ |
| BD-12 | npm run vendor follows a changed pin (by hand) | [Check record](../checks/2026-10-27-bundled-dependency-updates.md), by hand 1; `scripts/fetch-vendor.test.ts` verifySha256. GPUI Grace's package takes the same `vendor/`: `scripts/package-gpui.test.ts` layout "refuses without the vendored files, and says how to get them" | ☑ |
| BD-13 | The bump script reproduces today's pins (by hand) | [Check record](../checks/2026-10-27-bundled-dependency-updates.md), by hand 2 | ☑ |
| BD-14 | First real run after merge (by hand) | [Check record](../checks/2026-10-27-bundled-dependency-updates.md), Still needs a person 2: passed on 2026-10-28 (PR #35). The workflow has not changed since; its bump PRs now also run the `rust` job (BD-04) | ☑ |

### CB · Console banner

[Spec](../specs/2026-10-27-console-banner.md) · [#75](https://github.com/hpagniez/grace/issues/75) · phase P2 · 13 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| CB-01 | No project open | `banner::tests::asks_for_a_project_with_none_open`; `console::tests::banner::says_what_the_app_is_doing` | ☑ |
| CB-02 | A project is open and nothing has run | `banner::tests::is_ready_with_a_project_and_nothing_running`; `grace-e2e/tests/console.rs::open_run_and_run_again` ("Ready.") | ☑ |
| CB-03 | One scenario in one feature | `banner::tests::uses_the_singular_for_one_of_each` | ☑ |
| CB-04 | A project with no tests | `banner::tests::says_when_there_are_no_tests` | ☑ |
| CB-05 | Finding tests for the first time | `banner::tests::says_it_is_finding_tests_while_busy_with_no_tree`, `is_finding_while_busy_and_no_run_is_active` | ☑ |
| CB-06 | Discovery failed | `banner::tests::points_at_the_tree_after_a_discovery_error` | ☑ |
| CB-07 | Nothing to run | `banner::tests::has_no_state_line_when_run_all_has_nothing_to_run` | ☑ |
| CB-08 | The cursor stays still with reduced motion | `console::tests::banner::the_cursor_blinks_only_while_grace_works_and_never_with_reduced_motion` | ☑ |
| CB-09 | A run starts and Karate has not printed yet | `banner::tests::is_starting_once_a_run_is_active_and_karate_has_not_printed` | ☑ |
| CB-10 | The banner survives the line cap | `console::tests::lines::keeps_the_banner_above_the_lines_past_the_line_cap` | ☑ |
| CB-11 | Open, run and run again (e2e) | `grace-e2e/tests/console.rs::open_run_and_run_again` | ☑ |
| CB-12 | The console is darker than the window in dark mode (e2e) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): console `#0b0d11` on `#101217` in dark, as in Electron Grace | ☑ |
| CB-13 | The banner names the pinned versions | `console::tests::banner::names_the_pinned_versions`; `grace-e2e/tests/console.rs::open_run_and_run_again` | ☑ |

### HS · Home screen

[Spec](../specs/2026-10-27-home-screen.md) · [#78](https://github.com/hpagniez/grace/issues/78) · phase P2 · 24 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| HS-01 | A first start shows the home screen | `grace-e2e/tests/home.rs::a_first_start_shows_the_home_screen`, `grace::home::tests::a_first_start_shows_the_home_screen_without_a_list` | ☑ |
| HS-02 | Opening a folder from the home screen | `grace-e2e/tests/home.rs::opening_a_folder_from_the_home_screen` | ☑ |
| HS-03 | The last project opens at start without the home screen | `grace-e2e/tests/home.rs::the_last_project_opens_at_start_without_the_home_screen` | ☑ |
| HS-04 | Opening with no project open shows the project view, not home (unit) | `grace-core::state::tests::reduce::home_screen::an_opening_event_sets_opening_and_loaded_and_null_clears_opening`, `grace-core::runner::tests::home::open_emits_opening_around_the_discovery` | ☑ |
| HS-05 | No home screen flash at start (by hand) | `grace-e2e/tests/home.rs::the_last_project_opens_at_start_without_the_home_screen` (no frame of the home screen on the way; the window draws the ground alone until the runner says what is open) | ☑ |
| HS-06 | A slow network folder does not hold up the home screen (unit) | `grace-core::recent::tests::check_recent::a_folder_that_does_not_answer_within_the_time_bound_counts_as_existing` | ☑ |
| HS-07 | Removing the missing last project forgets it (unit) | `grace-core::settings::tests::recent_projects::remove_recent_project_drops_the_row_and_a_matching_last_project` | ☑ |
| HS-08 | A missing last project shows the home screen | `grace-e2e/tests/home.rs::a_missing_last_project_shows_the_home_screen` | ☑ |
| HS-09 | A failed open from the home screen goes back to it (unit) | `grace-core::runner::tests::home::a_failed_open_from_the_home_screen_changes_nothing` | ☑ |
| HS-10 | A dry-run error still opens the project | `grace-e2e/tests/home.rs::a_dry_run_error_still_opens_the_project` | ☑ |
| HS-11 | Reopening a folder from the list | `grace-e2e/tests/home.rs::reopening_a_folder_from_the_list` | ☑ |
| HS-12 | The folder dialog starts next to the newest existing folder | `grace-e2e/tests/home.rs::the_folder_dialog_starts_next_to_the_newest_existing_folder`; on Windows the Shell dialog's `SetFolder` (`grace::seams`), checked by hand in PR 6 | ☑ |
| HS-13 | Removing a row | `grace-e2e/tests/home.rs::removing_a_row` | ☑ |
| HS-14 | The list works from the keyboard | `grace-e2e/tests/home.rs::the_list_works_from_the_keyboard` | ☑ |
| HS-15 | The home screen follows the dark theme (by hand) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md) | ☑ |
| HS-16 | Home closes the project and survives a restart | `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` | ☑ |
| HS-17 | Home is disabled during a run | `grace-e2e/tests/home.rs::home_is_disabled_during_a_run`, `grace::console::tests::lines::a_closed_project_clears_the_lines_with_none_dropped` | ☑ |
| HS-18 | Main refuses to close a busy project (unit) | `grace-core::runner::tests::home::close_is_refused_while_busy` | ☑ |
| HS-19 | A reload after Home stays home | GPUI Grace has no page to reload: the window is drawn from the state, so Home stays until the state changes. Home surviving a restart: `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` | n/a |
| HS-20 | The list keeps the 10 newest folders (unit) | `grace-core::settings::tests::recent_projects::keeps_the_10_newest_folders` | ☑ |
| HS-21 | The same folder in another case is one entry (unit) | `grace-core::settings::tests::recent_projects::same_folder_with_a_trailing_separator_or_another_case_is_one_entry` | ☑ |
| HS-22 | A broken list reads as what can be kept (unit) | `grace-core::settings::tests::recent_projects::a_broken_list_reads_as_what_can_be_kept` | ☑ |
| HS-23 | A failed list save keeps the project open (unit) | `grace-core::runner::tests::home::a_failed_list_save_keeps_the_project_open` | ☑ |
| HS-24 | A path not in the list is refused | No renderer and no IPC: the view calls the project code itself, and only the home list's own rows open a recent project | n/a |

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

GPUI Grace's CI is `test.yml`'s `rust` job ([guide R6.5](2026-11-08-gpui-migration.md#r6-tests)): Cargo's `target/` and registry cached, Linux for every PR, Windows e2e for PRs into `main` and runs by hand. The Electron jobs stay until the cut-over.

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| FC-01 | a ready PR runs four jobs | `test.yml`'s `rust` job, one VM per OS (`ubuntu-latest`, `windows-latest`), beside the four Electron jobs. Its step named `e2e` runs on Linux on every PR, and on Windows on a PR into `main` or a run by hand (guide R6.5, ADR 0017) | ☑ |
| FC-02 | a draft PR runs unit tests only | GPUI Grace's job has no e2e step to skip on a draft: the `rust` job runs clippy, `cargo test` and the Linux e2e on every PR, drafts included (guide R6.5) | n/a |
| FC-03 | every test runs exactly once | `cargo test --workspace --locked` runs every test of the workspace in one step per OS: nothing is split, so no test runs twice or not at all | ☑ |
| FC-04 | the e2e shards cover the suite | `sh scripts/e2e-gpui.sh` runs every `crates/grace-e2e` test in one step per OS: no shards | ☑ |
| FC-05 | npm test still runs everything locally | `cargo test --workspace` runs the same tests on a PC as in CI; nothing is left out by a variable | ☑ |
| FC-06 | no assertion changes | Was about one PR's split of `runner.test.ts`; `grace-core`'s runner tests port test for test (R3.4) under the P1 rows of `src/main/runner.*.test.ts` | n/a |
| FC-07 | merge-when-green and the e2e steps | `merge-when-green.yml` takes every step named `e2e` in the run, the `rust` job's on both OSes included: a back-merge waits for GPUI Grace's e2e too. The jq cases: [check record](../checks/2026-10-28-faster-ci.md) | ☑ |
| FC-08 | the time goal | The time goal of the faster-CI PR (#45), met in its [check record](../checks/2026-10-28-faster-ci.md). The `rust` job runs beside the Electron jobs, on its own VMs, with its `target/` cached | n/a |

### LV · Loading always visible

[Spec](../specs/2026-10-28-loading-always-visible.md) · [#80](https://github.com/hpagniez/grace/issues/80) · phase P2 · 9 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| LV-01 | Finding tests shows the stripe | `grace::progress::tests::the_bar_r14_1::finding_tests_shows_the_stripe_until_the_runner_is_no_longer_busy`, `grace::progress::tests::after_a_run_r14_2::shows_finding_tests_with_the_turning_icon_while_the_runner_discovers` | ☑ |
| LV-02 | Finding tests for the first time | `grace-e2e/tests/open.rs::finding_tests_for_the_first_time` | ☑ |
| LV-03 | Tests found | `grace::progress::tests::the_bar_r14_1::finding_tests_shows_the_stripe_until_the_runner_is_no_longer_busy` | ☑ |
| LV-04 | The stripe never covers a run's results | `grace::progress::tests::the_bar_r14_1::the_stripe_hides_the_last_runs_segments_and_they_show_again_after` | ☑ |
| LV-05 | The stripe stays until the first result | `grace::progress::tests::the_bar_r14_1::the_stripe_stays_until_the_first_result_and_a_cancel_stops_it` | ☑ |
| LV-06 | The bar fills smoothly | `grace::progress::tests::the_bar_r14_1::is_6_px_with_2_px_gaps_between_passed_failed_and_cancelled` (a spring, not a CSS transition: P2 plan, Redesigns) | ☑ |
| LV-07 | Cancel stops the stripe | `grace::progress::tests::the_bar_r14_1::the_stripe_stays_until_the_first_result_and_a_cancel_stops_it` | ☑ |
| LV-08 | Reduced motion | `grace::progress::tests::the_bar_r14_1::reduced_motion_keeps_the_stripe_still` | ☑ |
| LV-09 | Run and finish (e2e) | `grace-e2e/tests/run.rs::run_all_with_live_progress` | ☑ |

### PU · Premium look and feel with a defined graphic chart

[Spec](../specs/2026-10-29-premium-ui.md) · [#107](https://github.com/hpagniez/grace/issues/107) [#75](https://github.com/hpagniez/grace/issues/75) [#158](https://github.com/hpagniez/grace/issues/158) · phase P0 · 33 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| PU-01 | Every token pair meets WCAG 2.2 AA in both themes (unit) | `crates/grace::theme::tests::keeps_text_at_4_5_to_1_on_every_background`, `…keeps_each_status_readable_on_surface_its_tint_and_a_selected_row`, `…keeps_the_filled_buttons_text_readable`, `…keeps_control_borders_and_the_focus_ring_at_3_to_1`, `…paints_console_matches_readably_the_current_one_stronger` | ☑ |
| PU-02 | The dark theme changes colours only (unit) | By construction: `grace::theme::Theme` is one shared `Metrics` and one `Tokens` per theme, so no size can differ; `crates/grace::theme::tests::matches_the_dark_tokens_of_index_css`, `…matches_the_sizes_of_index_css` | ☑ |
| PU-03 | The structure and the primary button do not change with the theme | `grace::theme::tests::matches_the_sizes_of_index_css` (the sheet's 8 px inset, 16 px corners and 1 px side, one set of sizes for both themes, PU-02), `grace::window::tests::the_project_view_sits_on_the_sheet_under_the_toolbar`; Run all is the toolbar's one primary button (`grace::toolbar::tests::order_and_shape_r13_1::puts_the_controls_in_the_order_of_the_spec_run_all_last`), filled with the passed green and its text readable in each theme (`grace::theme::tests::fills_the_primary_button_with_the_passed_green`, `…keeps_the_filled_buttons_text_readable`, the tokens equal to `src/index.css`'s) | ☑ |
| PU-04 | No raw palette class or dark: variant is left (unit) | `crates/grace::theme::tests::colours_come_only_from_the_theme` (no colour literal outside `theme.rs`; GPUI has no `dark:` variant) | ☑ |
| PU-05 | Main and the startup screen use the CSS ground colours (unit) | `grace::theme::tests::matches_the_light_tokens_of_index_css`, `…matches_the_dark_tokens_of_index_css` (the ground in each theme); GPUI Grace has no main process and no startup screen of its own: its first frame is the window on the ground (`grace::window::tests::opens_an_empty_grace_window_in_the_theme`, E-startup-02) | ☑ |
| PU-06 | The window follows the Windows theme | Light: `grace-e2e/tests/appearance.rs::follow_windows_is_the_default` (the ground's pixels; Xvfb has no dark preference), `grace::theme::tests::picks_the_theme_from_the_system_unless_pinned`. Dark: [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md), with Windows in dark mode and no Appearance saved: the ground is `#151821` and the title bar dark. Windows switched while Grace is open is UP-18's, for a person | ☑ |
| PU-07 | Grace draws its own typefaces offline | `grace::assets::tests::embeds_ibm_plex`, `…the_theme_asks_for_the_families_it_embeds` (the window in IBM Plex Sans, the console and the details' code in IBM Plex Mono), `grace::assets::tests::serves_every_icon` (the icons are in the exe); neither `grace` nor `grace-core` has an HTTP client until the updater (P5), so nothing leaves the app for a font or an icon | ☑ |
| PU-08 | Each status has its own icon and name | `crates/grace::elements::tests::marks_each_status_as_electron_grace_does`, `…tells_statuses_apart_by_shape_or_name`; through the tree, `crates/grace-e2e/tests/elements.rs::names_each_status_mark` | ☑ |
| PU-09 | A cancelled run marks its interrupted tests | `grace-e2e/tests/run.rs::cancel_leaves_nothing_behind` ("slow loop" named interrupted, the progress line's "Cancelled"), `grace::elements::tests::marks_each_status_as_electron_grace_does` (Interrupted: circle-slash), `grace::progress::tests::after_a_run_r14_2::names_the_outcome_with_its_icon` | ☑ |
| PU-10 | Tag chip states differ by shape, not only colour (unit) | `grace::filter_bar::tests::words::a_chips_state_shows_in_its_shape_and_its_name_not_only_its_colour` | ☑ |
| PU-11 | An HTTP status shows its code in a badge with its group in the tooltip (unit) | Waits for the HTTP tab, phase P3 | ☐ |
| PU-12 | Reduced motion stops every animation | `grace::window::tests::reduced_motion_stops_every_animation_during_a_run` (the whole window during a run: nothing asks for a frame); each one: `grace::elements::tests::turns_only_while_running_and_never_with_reduced_motion`, `grace::progress::tests::the_bar_r14_1::reduced_motion_keeps_the_stripe_still`, `grace::console::tests::banner::the_cursor_blinks_only_while_grace_works_and_never_with_reduced_motion`, `grace::theme::tests::the_caret_stops_blinking_when_idle_and_never_blinks_with_reduced_motion`; GPUI's `with_animation` and springs draw at rest with it set | ☑ |
| PU-13 | The logo is the Home button | `grace::toolbar::tests::home_and_refresh_the_icon_only_buttons::home_holds_the_logo_and_shows_home_in_a_tooltip`, `…home_goes_home_and_is_off_while_busy`, `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` | ☑ |
| PU-14 | The startup screen shows the logo in the active theme (by hand) | GPUI Grace has no startup screen (E-startup-02): its first frame is the window in the theme (UP-17, UP-18). Its title bar takes the theme too, as Electron Grace's follows `nativeTheme.themeSource`: `grace::theme::apply` sets the window's `DWMWA_USE_IMMERSIVE_DARK_MODE` from Grace's theme, which GPUI sets only from Windows' (#337, found in [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md)). On Windows, `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start` and `…choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart` read the attribute and the colour the screen shows in the title bar: dark with Dark, light with Light, at once and after a restart; `…a_switch_of_windows_theme_leaves_graces_on_the_title_bar` switches CI's Windows to the other theme under Grace and reads Grace's still, then Follow Windows takes Windows' new one. By hand on the owner's PC on 2026-11-11, the release build of `develop` at 420c6d8: with Grace on Light, switching Windows to dark left the title bar light, and with Windows dark, Follow Windows, Light and Dark each drew the title bar in their own theme | ☑ |
| PU-15 | Choosing Dark overrides a light Windows | `grace-e2e/tests/appearance.rs::choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart`, `…a_saved_dark_is_in_force_at_start` (under Xvfb's light preference the ground turns #151821 at once, and a saved Dark is in force at the next start) | ☑ |
| PU-16 | Follow Windows is the default | `grace-e2e/tests/appearance.rs::follow_windows_is_the_default` | ☑ |
| PU-17 | A malformed appearance setting reads as Follow Windows (unit) | `grace_core::settings` tests `appearance` | ☑ |
| PU-18 | Main rejects an unknown theme (unit) | Nothing to refuse: no IPC, and a theme is an enum (plan, Redesigns; E-appearance-06) | ☑ |
| PU-19 | A tooltip shows on keyboard focus and closes with Escape | `crates/grace::tooltip::tests::shows_at_once_on_keyboard_focus_and_once_per_focus`, `…closes_on_escape_and_on_a_press_of_its_target`, `…does_not_show_on_a_focus_from_the_mouse`; through the tree, `crates/grace-e2e/tests/tooltips.rs::shows_on_keyboard_focus_and_closes_with_escape` | ☑ |
| PU-20 | A tooltip shows after a pause on hover, also on a disabled button | `crates/grace::tooltip::tests::shows_after_the_pause_not_before`, `…does_not_show_when_the_pointer_leaves_first_and_hides_when_it_leaves_after`, `…shows_one_at_a_time`; through the tree, `crates/grace-e2e/tests/tooltips.rs::shows_on_hover_also_over_a_disabled_button` | ☑ |
| PU-21 | Escape in a dialog closes the tooltip first | `crates/grace-e2e/tests/tooltips.rs::escape_in_a_dialog_closes_the_tooltip_first` (the tooltip takes Escape before any other handler) | ☑ |
| PU-22 | No native title attribute is left (unit) | `grace::tooltip::tests::tooltips_come_only_from_this_module` (no tooltip of GPUI's or GPUI Kit's own anywhere else) | ☑ |
| PU-23 | Every control shows the focus ring | `grace-e2e/tests/look.rs::every_control_shows_the_focus_ring_in_light_mode`, `…_in_dark_mode` (Tab once round the window after a run, more than 17 stops, the tree one of them, each stop's ring read off the pixels on Linux; on Windows the stops only), `grace::theme::tests::kits_half_strength_focus_ring_keeps_3_to_1`. The ring is GPUI Kit's (plan, Redesigns "Kit's focus ring") | ☑ |
| PU-24 | Targets are at least 24 by 24 | `grace-e2e/tests/look.rs::targets_are_at_least_24_by_24` (the recent list's remove buttons, the tree's run buttons, the tabs and the tag chips, by their places); a chip's button covers its frame, border included (`filter_bar::Target`) | ☑ |
| PU-25 | Existing keyboard paths still work | The e2e suite, ported to the driver (the E-* rows), passes in `test.yml`: on Linux on every PR that changes code, on Windows on every PR into `main` and every run by hand | ☑ |
| PU-26 | First run shows the first-run home screen | `grace-e2e/tests/home.rs::a_first_start_shows_the_home_screen`, `grace::home::tests::a_first_start_shows_the_home_screen_without_a_list`, `…the_mascot_is_decorative`; Open folder… is the home screen's one primary button | ☑ |
| PU-27 | A missing recent folder says so in words | `grace::home::tests::says_not_found_in_a_cancelled_badge_with_the_warning_icon_and_the_row_is_disabled`, `…a_missing_folder_shows_its_reason_on_home` | ☑ |
| PU-28 | A project with no feature files shows an empty state | `grace-e2e/tests/run.rs::nothing_to_run` ("No feature files found" and where Grace looks, both in the tree) | ☑ |
| PU-29 | Before any run the progress line says so | `grace::progress::tests::before_any_run_r14_3::counts_the_projects_tests_and_says_not_run_yet_with_no_zero_counts`; through the tree, `grace-e2e/tests/details.rs::failure_details` ("9 tests · Not run yet") | ☑ |
| PU-30 | Details with nothing selected | `details::tests::nothing_selected`; the e2e test reads its words before a run | ☑ |
| PU-31 | Empty history and HTTP tabs | Waits for the History and HTTP tabs, phase P3 | ☐ |
| PU-32 | Dialogs keep their order and keyboard behaviour | Waits for Clear history, phase P3 | ☐ |
| PU-33 | Before and after screenshots exist (by hand) | P2's screens, GPUI beside Electron in light and dark, in [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md); the panels' with phase P3 | ☐ |

### VL · Big projects: virtual lists and keyboard navigation

[Spec](../specs/2026-10-30-virtual-lists.md) · [#91](https://github.com/hpagniez/grace/issues/91) [#123](https://github.com/hpagniez/grace/issues/123) [#124](https://github.com/hpagniez/grace/issues/124) [#125](https://github.com/hpagniez/grace/issues/125) · phase P2 · 29 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| VL-01 | Only the rows in view are drawn (e2e) | `grace-e2e/tests/tree.rs::only_the_rows_in_view_are_drawn` | ☑ |
| VL-02 | A small list draws in jsdom | `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` (every row of a small project drawn); GPUI Grace has no jsdom | ☑ |
| VL-03 | The tree shows the same rows as before | `grace_core::rows::tests::lists_every_node_but_the_root_in_tree_order_with_its_level_and_place_among_its_siblings`, `grace::tree::tests::hide_non_matching_leaves_the_matching_rows` | ☑ |
| VL-04 | Closed folders survive a refresh | `grace::tree::tests::a_refresh_keeps_what_was_opened_and_closed_and_a_new_feature_starts_closed` | ☑ |
| VL-05 | Statuses do not move rows | `grace::tree::tests::statuses_do_not_build_the_rows_again` | ☑ |
| VL-06 | The tree is one tab stop | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` (the tree is a tab stop, the focus a tree item) | ☑ |
| VL-07 | Arrow keys move and select | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` | ☑ |
| VL-08 | Right and Left | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` | ☑ |
| VL-09 | Home, End and pages | `grace::tree::tests::home_end_and_pages`, `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` (Home and End) | ☑ |
| VL-10 | Ctrl+Enter runs the focused row | `grace-e2e/tests/tree.rs::ctrl_enter_runs_the_focused_row` | ☑ |
| VL-11 | The row's buttons left the tab order but still click | `grace-e2e/tests/tree.rs::the_rows_buttons_are_out_of_the_tab_order_but_still_click` | ☑ |
| VL-12 | A click makes the row focused | `grace::tree::tests::a_click_makes_the_row_focused` | ☑ |
| VL-13 | The HTTP list is one tab stop and arrows open calls | Waits for the HTTP tab, phase P3 | ☐ |
| VL-14 | A moved-to row scrolls into view (e2e) | `grace-e2e/tests/tree.rs::only_the_rows_in_view_are_drawn` (End) | ☑ |
| VL-15 | Focus survives scrolling away (e2e) | `grace::tree::tests::focus_survives_scrolling_away` (headless: plan, Redesigns) | ☑ |
| VL-16 | Closing a folder takes focus to it | `grace::tree::tests::closing_a_feature_with_its_chevron_takes_the_focus_to_it` | ☑ |
| VL-17 | Hiding the focused row | `grace::tree::tests::closing_the_feature_of_the_focus_row_hands_the_focus_to_the_nearest_row_above` | ☑ |
| VL-18 | A new tree keeps the rows in view still (e2e) | `grace::tree::tests::a_feature_opening_above_the_view_does_not_move_it`, `grace-core::rows::tests::anchored_scroll_top` (headless: plan, Redesigns) | ☑ |
| VL-19 | Calls arriving above the view keep the rows in view still | Waits for the HTTP tab, phase P3 | ☐ |
| VL-20 | A filter change scrolls to the selected row (e2e) | `grace-e2e/tests/tags.rs::a_filter_change_scrolls_to_the_selected_row` | ☑ |
| VL-21 | A cut label shows whole on keyboard focus | `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus` | ☑ |
| VL-22 | The run shortcut can be found | `grace-e2e/tests/tree.rs::the_rows_buttons_are_out_of_the_tab_order_but_still_click` (the tooltip "Run passing hello (Ctrl+Enter)"); every row carries `tree::SHORTCUTS`, "Control+Enter", which AT-SPI does not pass on | ☑ |
| VL-23 | Tree rows carry their level and position | `grace_core::rows::tests::lists_every_node_but_the_root_in_tree_order_with_its_level_and_place_among_its_siblings` (each tree item takes its level, position and set size from its row); the tree named "Tests" in `grace-e2e/tests/tree.rs` | ☑ |
| VL-24 | A row's name says its status, flakiness and tags | `grace-e2e/tests/run.rs::run_all_with_live_progress` ("slow loop, running"); `grace_core::rows::tests::row_name_joins_the_label_the_status_word_in_lower_case_the_flaky_words_and_the_tags` | ☑ |
| VL-25 | Calls are options | Waits for the HTTP tab, phase P3 | ☐ |
| VL-26 | Opening 10,000 tests (perf) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): 4.4 s to the tree (Electron 4.8 s), longest stall 17 ms, 57 MB (Electron 278 MB) | ☑ |
| VL-27 | Folders, selection and the HTTP tab at 10,000 tests (perf) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): folders 35 ms, selection 34 ms, Down 50 ms; the HTTP tab with phase P3 | ☐ |
| VL-28 | Run all at 10,000 tests (perf) | [the P2 exit check](../checks/2026-11-10-gpui-p2-exit.md): longest stall 27 ms, Finished 0.4 s after Karate, peak 267 MB (Electron 698 MB) | ☑ |
| VL-29 | 2,000 tests are not slower (perf) | Misses in [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md) on one measure: with no UI Automation client, app CPU 5,438 ms against Electron's 4,156 ms during Run all (+31%, from +58% before #327); run time, Java's CPU and memory are as good or better (#320) | ☐ |

### LF · Light and fast: numbers on every PR, a benchmark before every release

[Spec](../specs/2026-10-31-light-and-fast.md) · [#147](https://github.com/hpagniez/grace/issues/147) [#148](https://github.com/hpagniez/grace/issues/148) · phase P0 · 25 scenarios

`bench/`, `perf` and `check-pr` are tooling and stay in Node ([guide](2026-11-08-gpui-migration.md#goals-non-goals-anti-goals), non-goal). The bench measures a GPUI build without Playwright ([R7](2026-11-08-gpui-migration.md#r7-performance)); what it cannot measure there yet is #334.

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| LF-01 | The bench measures the packaged app | Partly: `npm run bench:gpui` writes the JSON and the table for a GPUI build's size, start, idle and wake-ups ([P0 record](../checks/2026-11-09-gpui-p0-windows.md), [P2 record](../checks/2026-11-10-gpui-p2-exit.md)). The tree, run and smooth measures: #334 | ☐ |
| LF-02 | The bench compares two builds in turns | `bench/stats.test.ts` order "runs one warm-up per build, then the rounds in turns: a, b, b, a, a, b"; [P2 record](../checks/2026-11-10-gpui-p2-exit.md): Electron Grace against GPUI Grace in turns, the table with A, B and the change | ☑ |
| LF-03 | The bench measures a release zip it did not build | [P2 record](../checks/2026-11-10-gpui-p2-exit.md): `--b-zip` on GPUI Grace's release zip. GPUI Grace has no CDP: the bench reads its probes (`bench/gpui.test.ts` parseProbe) and its processes | ☑ |
| LF-04 | The bench waits for a quiet machine | `bench/system.test.ts` waitForQuiet; the same rounds for both apps (`bench/bench.mjs`) | ☑ |
| LF-05 | A worse number does not fail the bench | `bench/judge.test.ts` judge "calls a move beyond the noise worse or better, and one within it a pass"; the bench exits 0 on a worse number whatever the build | ☑ |
| LF-06 | A bench that cannot measure fails | `startGpui` rejects when GPUI Grace cannot start, dies first or draws nothing in 60 s (`bench/gpui.mjs`), with no test yet: #334 | ☐ |
| LF-07 | Smoothness is measured idle and during a run | Not yet on GPUI Grace: the frame probe and the driven actions of R7.2, #334. Meanwhile the UI thread's stalls are measured from outside ([P2 record](../checks/2026-11-10-gpui-p2-exit.md), VL-26 to VL-29) | ☐ |
| LF-08 | Idle CPU is measured shown and minimised | `idle.cpu.shown` and `idle.cpu.minimised` of `npm run bench:gpui` ([P0 record](../checks/2026-11-09-gpui-p0-windows.md), [P2 record](../checks/2026-11-10-gpui-p2-exit.md)) | ☑ |
| LF-09 | Numbers on a PR | `perf` measures Electron Grace until it builds GPUI Grace's tree (guide, Cost: CI; P6), after #334 | ☐ |
| LF-10 | A merge waits for the numbers | Unchanged tooling (guide, non-goal): `merge-when-green.yml` waits for `perf` on the head, whichever build the bench measures | ☑ |
| LF-11 | A worse number never blocks the merge | Unchanged tooling: `merge-when-green.yml` reads `perf`'s conclusion, never its numbers | ☑ |
| LF-12 | A perf run that could not measure holds the merge | Unchanged tooling: `merge-when-green.yml` needs `perf` to finish with success | ☑ |
| LF-13 | A docs-only PR | Unchanged tooling: `perf.yml`'s `paths-ignore` (`docs/**`, `**/*.md`) | ☑ |
| LF-14 | A PR from a fork | Unchanged tooling: `perf.yml` writes the job summary and comments only from a branch of this repo | ☑ |
| LF-15 | Raising a limit without a decision record | `scripts/check-pr.test.ts` limitRaiseError "names a raised limit that adds no decision record"; check-pr CLI "fails a PR that raises a limit without a decision record" | ☑ |
| LF-16 | Lowering a limit | `scripts/check-pr.test.ts` limitRaiseError "accepts a lowered limit" | ☑ |
| LF-17 | A release with no performance section | `scripts/check-pr.test.ts` performanceError "rejects a record with no Performance section"; check-pr CLI "fails a release into main whose check record has no Performance section" | ☑ |
| LF-18 | A release with no bench JSON | `scripts/check-pr.test.ts` performanceError "rejects a record with no bench JSON beside it" | ☑ |
| LF-19 | A release result typed over a miss | `scripts/check-pr.test.ts` performanceError "rejects Result: pass over a JSON that misses, and names the measure" | ☑ |
| LF-20 | A measure the previous release cannot give | `bench/judge.test.ts` judge "judges on the limit alone when build a has no value"; `bench/report.test.ts` markdown "writes n/a for a measure with no number" | ☑ |
| LF-21 | A release miss with no decision | `scripts/check-pr.test.ts` performanceError "rejects Result: miss with no Decision: go" | ☑ |
| LF-22 | A release miss with a no-go | `scripts/check-pr.test.ts` performanceError "rejects Decision: no-go" | ☑ |
| LF-23 | A release miss, analysed and accepted | `scripts/check-pr.test.ts` performanceError "accepts Result: miss with an analysis and Decision: go" | ☑ |
| LF-24 | A release that passes | `scripts/check-pr.test.ts` performanceError "accepts Result: pass over a JSON that passes" | ☑ |
| LF-25 | Feature PRs skip the release gate | `scripts/check-pr.test.ts` performanceError "skips a feature PR into develop" | ☑ |

### JM · Java memory

[Spec](../specs/2026-11-01-java-memory.md) · [#149](https://github.com/hpagniez/grace/issues/149) · phase P1 · 6 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| JM-01 | Every Java Grace starts gets the memory flags | `karate::command::tests::build_args::builds_the_exact_command_line`, `…passes_the_count_on_runs_and_t_1_on_dry_runs`, `…adds_the_debug_listener_and_its_socket_to_a_debug_run_only`; real Java: `…build_args_through_java::java_stops_at_the_first_out_of_memory_error_with_the_line_out_of_memory_finds`. The build tool's Java ports with Maven in P4 | ☑ |
| JM-02 | A run that runs out of memory says so | `runner::tests::runs::says_so_when_java_runs_out_of_memory_and_the_next_run_does_not` (real Java); on screen: `grace_core::state::tests::notice_sets_the_notice_a_warning_unless_main_says_it_reports_a_success`, `grace::window::tests::the_notice_the_main_area_and_the_panel_are_on_the_sheet` | ☑ |
| JM-03 | A dry run that runs out of memory says so | `runner::tests::discovery::a_dry_run_that_ran_out_of_heap_says_so_one_that_only_names_the_error_does_not`; on screen: `grace_core::state::tests::notice_sets_the_notice_a_warning_unless_main_says_it_reports_a_success`, `grace::window::tests::the_notice_the_main_area_and_the_panel_are_on_the_sheet` | ☑ |
| JM-04 | Another out-of-memory error gets no notice | `karate::command::tests::out_of_memory::finds_javas_out_of_memory_line_among_the_others` (Metaspace and native-thread lines) | ☑ |
| JM-05 | A test's output that names the error is not Java running out of memory | `karate::command::tests::out_of_memory::finds_javas_out_of_memory_line_among_the_others` (a response body and a test name that name the error) | ☑ |
| JM-06 | The bench shows a run takes less memory | The same command line as JJ-04. GPUI Grace's run keeps Java's peak within the limits Electron Grace's flags set: 247 MB on 10,000 tests (Electron Grace 252, limit 281.3) and 238 on 2,000 (Electron Grace 189, limit 249.9; Java's peak varies by about 75 MB from round to round, [JIT record](../checks/2026-11-02-java-jit.md)) ([P2 record](../checks/2026-11-10-gpui-p2-exit.md), measured from outside). From the bench itself: #334 | ☑ |

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
| JJ-01 | The dry run compiles with C1 only | `karate::command::tests::build_args::builds_the_exact_command_line` (the dry run's `-XX:TieredStopAtLevel=1`) | ☑ |
| JJ-02 | A run keeps C2 on fewer threads | `karate::command::tests::build_args::builds_the_exact_command_line` (the run's `-XX:CICompilerCount=2`) | ☑ |
| JJ-03 | Heavy JavaScript runs no slower | Java gets the same flags (JJ-01, JJ-02), and the [research](../research/2026-11-02-java-jit/README.md) ran the heavy JavaScript on Java itself, not through an app: it holds for GPUI Grace unchanged | ☑ |
| JJ-04 | The bench shows Java works less | The flags' gain was measured on Java, which GPUI Grace starts with the same command line. GPUI Grace's run gives Java the same CPU as Electron Grace's: 30,406 against 32,250 ms on 10,000 tests, 10,141 against 10,578 on 2,000 ([P2 record](../checks/2026-11-10-gpui-p2-exit.md), measured from outside). From the bench itself: #334 | ☑ |

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
| TB-01 | The toolbar holds the name and the chip, not the path | `grace::toolbar::tests::the_project::shows_the_folder_name_and_the_layout_and_leaves_the_path_to_the_status_bar_r1_1`, `grace::toolbar::tests::the_project::draws_the_name_and_the_chip` | ☑ |
| TB-02 | The name is cut in the middle, keeping its end | `grace::toolbar::tests::the_project::cuts_a_long_name_in_the_middle_its_start_gives_way_its_last_12_characters_stay_r1_2`, `grace::toolbar::tests::the_project::lets_the_chip_go_whole_before_the_name_is_cut_r1_2` | ☑ |
| TB-03 | A cut name explains itself | `grace::toolbar::tests::the_project::explains_a_cut_name_in_a_tooltip_and_says_nothing_while_it_fits_r1_3` | ☑ |
| TB-04 | The name shows whole at the default size (e2e) | `grace::window::tests::the_name_shows_whole_at_the_default_size` (headless: plan, Redesigns) | ☑ |
| TB-05 | A long path at the minimum width (e2e) | `grace::window::tests::a_long_path_at_the_minimum_width` (headless: plan, Redesigns); the path's tooltip TB-07, the words TB-08 | ☑ |
| TB-06 | The status bar shows the path while a project is open | `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` | ☑ |
| TB-07 | A cut path explains itself | `grace::status_bar::tests::explains_a_cut_path_in_a_tooltip_and_says_nothing_while_it_fits` | ☑ |
| TB-08 | Env and Threads below 1280 px | `grace::env_picker::tests::below_1280_px_the_word_env_goes_from_sight_and_says_itself_in_a_tooltip`, `grace::threads_box::tests::below_1280_px_the_word_threads_goes_from_sight_and_says_itself_in_a_tooltip` | ☑ |

### CF · A one-line filter bar

[Spec](../specs/2026-11-03-compact-filter-bar.md) · [#127](https://github.com/hpagniez/grace/issues/127) · phase P2 · 7 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| CF-01 | Chips that do not fit fold behind a count | `grace::filter_bar::tests::folding::folds_the_chips_past_the_line_and_keeps_the_bar_one_line`, `words::the_fold_chip_counts_the_folded_tags_and_those_that_are_on` | ☑ |
| CF-02 | A folded tag that is on is counted | `grace::filter_bar::tests::folding::counts_the_folded_tags_that_are_on`; `grace-e2e/tests/tags.rs::many_tags_keep_a_one_line_bar` (", 1 on") | ☑ |
| CF-03 | No fold chip when every chip fits | `grace::filter_bar::tests::folding::has_no_fold_chip_when_every_chip_fits` | ☑ |
| CF-04 | The panel holds every tag and cycles it | `grace::filter_bar::tests::folding::opens_a_panel_of_every_tag_that_cycles_them_and_stays_open` | ☑ |
| CF-05 | The panel closes when nothing is folded | `grace::filter_bar::tests::folding::the_panel_closes_when_nothing_is_folded` | ☑ |
| CF-06 | During a run the panel opens with its chips disabled | `grace::filter_bar::tests::folding::during_a_run_the_panel_opens_with_its_chips_disabled` | ☑ |
| CF-07 | The bench project keeps a one-line bar (e2e) | `grace-e2e/tests/tags.rs::many_tags_keep_a_one_line_bar` (54 generated tags) | ☑ |

### CS · Find in the console, and step through its errors

[Spec](../specs/2026-11-03-console-search.md) · [#73](https://github.com/hpagniez/grace/issues/73) · phase P2 · 17 scenarios

| ID | Scenario | Proof in GPUI Grace | Ported |
|---|---|---|---|
| CS-01 | ERROR lines and failed tests are errors, WARN lines are not | `console_find::tests` (`takes_error_lines_and_failed_tests_not_warn_lines`); `console::tests::lines::styles_error_and_warn_lines` | ☑ |
| CS-02 | The errors group shows the count and steps through the errors | `console::tests::errors::shows_the_count_and_steps_through_the_errors_wrapping`, `counts_one_error_in_the_singular`, `paints_the_current_errors_line` | ☑ |
| CS-03 | No errors, no group | `console::tests::errors::has_no_group_and_steps_nowhere_without_errors` | ☑ |
| CS-04 | The errors group belongs to the Console tab | `console::tests::errors::belongs_to_the_console_tab` | ☑ |
| CS-05 | A failing run's errors (e2e) | `grace-e2e/tests/console.rs::step_through_a_failing_runs_errors` | ☑ |
| CS-06 | Ctrl+F opens the find bar from another tab | `console::tests::find::opening_the_bar_shows_the_console_tab_and_focuses_its_field`; `grace-e2e/tests/console.rs::ctrl_f_opens_the_bar_from_another_tab` | ☑ |
| CS-07 | Matches are counted, case aside, without overlap | `console_find::tests::counts_every_occurrence_case_aside_without_overlap` | ☑ |
| CS-08 | Enter and Shift+Enter step through the matches and wrap | `console::tests::find::counts_matches_and_steps_with_enter_and_shift_enter_wrapping`; `grace-e2e/tests/console.rs::find_in_a_real_run` | ☑ |
| CS-09 | No results | `console::tests::find::shows_no_results_and_nothing_for_no_text` | ☑ |
| CS-10 | Matches are painted without redrawing the rows (highlight API stubbed) | `console::tests::find::paints_matches_and_the_current_one_brighter` (only the lines in view are drawn: plan, Redesigns) | ☑ |
| CS-11 | Escape closes the bar, keeps the text, and gives the focus back | `console::tests::find::escape_closes_the_bar_keeps_the_text_and_gives_the_focus_back`; `grace-e2e/tests/console.rs::ctrl_f_opens_the_bar_from_another_tab` | ☑ |
| CS-12 | The text survives a tab switch and a new run | `console::tests::find::keeps_the_text_across_a_tab_switch_and_a_new_run` | ☑ |
| CS-13 | Lines that arrive during a run do not move the view | `console::tests::find::a_step_stops_following_the_bottom`, `keeps_the_current_match_on_its_line_as_lines_arrive` | ☑ |
| CS-14 | Find in a real run (e2e) | `grace-e2e/tests/console.rs::find_in_a_real_run` | ☑ |
| CS-15 | The bar does not search while it is closed | `console::tests::find::does_not_search_while_another_tab_shows_or_the_bar_is_closed` | ☑ |
| CS-16 | A line keeps its place when older lines drop at the cap | `console::tests::find::keeps_the_current_matchs_line_when_older_lines_drop_at_the_cap` | ☑ |
| CS-17 | A full console (e2e) | `console::tests::lines::keeps_following_once_the_store_holds_the_line_cap`, `find::keeps_the_current_matchs_line_when_older_lines_drop_at_the_cap` (the driver cannot send events: plan, Redesigns) | ☑ |

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
| TO-01 | A project opens with its features closed | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one`, `grace::tree::tests::a_project_opens_with_its_features_closed` | ☑ |
| TO-02 | A refresh keeps what I opened and closed | `grace::tree::tests::a_refresh_keeps_what_was_opened_and_closed_and_a_new_feature_starts_closed` | ☑ |
| TO-03 | A feature new in a refresh starts closed | `grace::tree::tests::a_refresh_keeps_what_was_opened_and_closed_and_a_new_feature_starts_closed` | ☑ |
| TO-04 | Another project starts closed again | `grace::tree::tests::another_project_starts_closed_again` | ☑ |
| TO-05 | A click on a closed feature's name opens and selects it | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` | ☑ |
| TO-06 | A click on an open feature's name keeps it open | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` (the second press) | ☑ |
| TO-07 | The chevron still closes a feature | `grace-e2e/tests/tree.rs::the_chevron_opens_and_closes_a_feature` | ☑ |
| TO-08 | Moving with the keys does not open features | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` | ☑ |
| TO-09 | An outline opens with its feature | `grace::tree::tests::an_outline_opens_with_its_feature`, `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` (the outline and its rows) | ☑ |
| TO-10 | A run of one feature opens it as it starts | `grace-e2e/tests/tree.rs::ctrl_enter_runs_the_focused_row`, `grace::tree::tests::a_run_of_one_feature_opens_it_as_it_starts` | ☑ |
| TO-11 | Run all opens only the features with a failure | `grace::tree::tests::run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it` | ☑ |
| TO-12 | A run never reopens a feature I closed during it | `grace::tree::tests::run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it` | ☑ |
| TO-13 | A run opens a feature inside a closed folder, and leaves the folder closed | `grace::tree::tests::a_run_opens_a_feature_inside_a_closed_folder_and_leaves_the_folder_closed` | ☑ |
| TO-14 | A failure leaves a closed outline closed | `grace::tree::tests::a_failure_leaves_a_closed_outline_closed` | ☑ |
| TO-15 | What a run opened stays open | `grace::tree::tests::run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it` | ☑ |
| TO-16 | A pause still opens the way to the paused test | Waits for the step debugger, phase P4: a pause needs it | ☐ |
| TO-17 | Statuses do not move rows | `grace::tree::tests::statuses_do_not_build_the_rows_again` | ☑ |
| TO-18 | A feature opening above the view does not move it | `grace::tree::tests::a_feature_opening_above_the_view_does_not_move_it` | ☑ |

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
| EP-01 | The name rules follow the kind | `grace-core::model::env::tests::environment_variables::the_kind_picks_the_rule`; `grace-core::properties_draft::tests::env_variable_picks_the_name_rule_and_save_sends_it` | ☑ |
| EP-02 | Stripped variables cannot be set | `grace-core::model::env::tests::environment_variables::cannot_be_one_grace_strips_from_karates_environment_in_any_case`; `grace-core::import_plan::tests::skips_a_value_the_value_rules_refuse_and_a_reserved_variable` | ☑ |
| EP-03 | Two variables that differ only in case | `grace-core::properties_draft::tests::two_variables_that_differ_only_in_case_are_duplicates_a_property_beside_them_is_not` | ☑ |
| EP-04 | Switching the kind keeps a saved secret | `grace-core::properties_draft::tests::ticking_env_variable_on_a_saved_secret_keeps_it`; `grace-core::runner::tests::properties::the_kind_is_saved_and_switching_it_keeps_the_secret` | ☑ |
| EP-05 | An environment variable reaches the run and is masked | `grace-core::runner::tests::properties::an_environment_variable_reaches_the_run_masked_and_stays_out_of_run_args_txt` | ☑ |
| EP-06 | Both kinds in one run | `grace-core::runner::tests::properties::both_kinds_in_one_run` | ☑ |
| EP-07 | The variable replaces the machine's in any letter case | `grace-core::runner::tests::properties::the_machines_variable_is_replaced_in_any_case` | ☑ |
| EP-08 | The environment's variable wins over a shared one in another case | `grace-core::runner::tests::properties::the_environments_variable_wins_over_a_shared_one_in_another_case` | ☑ |
| EP-09 | Dry runs get no environment variables | `grace-core::runner::tests::properties::dry_runs_get_no_environment_variables` | ☑ |
| EP-10 | Old rows load as system properties | `grace-core::settings::tests::environment_and_run_properties::an_environment_variable_row_keeps_its_kind_a_row_without_one_is_a_system_property` | ☑ |
| EP-11 | A malformed kind is kept, not loaded | `grace-core::settings::tests::environment_and_run_properties::a_malformed_kind_or_a_name_its_kind_refuses_is_named_and_kept_not_loaded` | ☑ |
| EP-12 | Main parses a .properties file as Java does | `grace-core::properties_file::tests::parse_properties::reads_keys_the_way_java_util_properties_load_does` | ☑ |
| EP-13 | A byte order mark is dropped | `grace-core::properties_file::tests::read_properties_file::drops_a_utf_8_byte_order_mark_and_reads_the_rest_as_iso_8859_1` | ☑ |
| EP-14 | A malformed escape refuses the file | `grace-core::properties_file::tests::read_properties_file::refuses_a_malformed_escape_without_a_value_in_the_message` | ☑ |
| EP-15 | A file over 1 MB is refused | `grace-core::properties_file::tests::read_properties_file::refuses_a_file_over_1_mb` | ☑ |
| EP-16 | Closing the Open dialog does nothing | `grace::properties::tests::back_changes_nothing_and_no_file_picked_shows_no_step` | ☑ |
| EP-17 | The import step lists adds, replaces and skips | `grace::properties::tests::import_shows_what_it_adds_and_puts_secret_variables_in_the_draft`; `grace-core::import_plan::tests::adds_replaces_and_skips_as_variables_with_no_prefix` | ☑ |
| EP-18 | A file that is no .properties file shows no line of it | `grace::properties::tests::import_shows_what_it_adds_and_puts_secret_variables_in_the_draft` (the empty key counted, not listed) | ☑ |
| EP-19 | Two keys with one name | `grace-core::import_plan::tests::skips_a_key_that_maps_to_the_name_of_an_earlier_one` | ☑ |
| EP-20 | An invalid prefix blocks the import | `grace::properties::tests::an_invalid_prefix_blocks_the_import` | ☑ |
| EP-21 | An import past the list limit is blocked | `grace::properties::tests::an_import_past_the_list_limit_is_blocked` | ☑ |
| EP-22 | Import adds to the draft only | `grace::properties::tests::import_shows_what_it_adds_and_puts_secret_variables_in_the_draft`; `grace-core::properties_draft::tests::import_puts_secret_variables_in_the_draft_replacing_in_place` | ☑ |
| EP-23 | Back changes nothing | `grace::properties::tests::back_changes_nothing_and_no_file_picked_shows_no_step` | ☑ |
| EP-24 | Import into an environment with no list yet | `grace-core::properties_draft::tests::imports_into_the_chosen_environments_new_list` | ☑ |
| EP-25 | Import needs secret storage | `grace::properties::tests::without_secret_storage_nothing_can_become_a_secret`; `grace-e2e/tests/properties.rs::without_secret_storage_no_secret_can_be_saved` (Linux) | ☑ |
| EP-26 | Import a secrets file and run with it (e2e) | `grace-e2e/tests/properties.rs::import_a_secrets_file_and_run_with_it` (Windows) | ☑ |

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
| E-appearance-01 | Follow Windows is the default | `grace-e2e/tests/appearance.rs::follow_windows_is_the_default` | ☑ |
| E-appearance-02 | A saved Dark is in force at start, whatever Windows is | `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start` (the ground's pixels on Linux) | ☑ |
| E-appearance-03 | Choosing a theme in Settings applies at once, with a project open, and is still there after a restart | `grace-e2e/tests/appearance.rs::choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart` | ☑ |
| E-appearance-04 | A theme change with a dialog up switches the window and the dialog and loses no state | `properties::tests::a_theme_change_keeps_the_draft_and_the_focus` (plan, Redesigns) | ☑ |
| E-appearance-05 | Escape closes a shown tooltip first, then the Settings page goes back | Going back: `grace-e2e/tests/appearance.rs::escape_goes_back_from_the_settings_page`; the tooltip half with P5's update button (plan, Redesigns) | ☐ |
| E-appearance-06 | Main refuses a theme it does not know and keeps the saved one | Nothing to refuse: no IPC, and a theme is an enum (plan, Redesigns) | ☑ |

### e2e/cancel.spec.ts

Phase P2 · 2 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-cancel-01 | Cancel leaves nothing behind | `grace-e2e/tests/run.rs::cancel_leaves_nothing_behind` (the slow feature's own run, "slow loop" interrupted) | ☑ |
| E-cancel-02 | Closing the app during a run | `grace-e2e/tests/run.rs::closing_the_app_during_a_run` (Run all in place of the slow feature's own button) | ☑ |

### e2e/console-find.spec.ts

Phase P2 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-console-find-01 | find in a real run | `grace-e2e/tests/console.rs::find_in_a_real_run` | ☑ |
| E-console-find-02 | step through a failing run's errors | `grace-e2e/tests/console.rs::step_through_a_failing_runs_errors` | ☑ |
| E-console-find-03 | a full console | `console::tests::lines::keeps_following_once_the_store_holds_the_line_cap` (headless: plan, Redesigns) | ☑ |

### e2e/console.spec.ts

Phase P2 · 2 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-console-01 | open, run and run again | `grace-e2e/tests/console.rs::open_run_and_run_again` | ☑ |
| E-console-02 | the console is on the sunken colour in both themes | `grace-e2e/tests/console.rs::the_console_is_on_the_sunken_colour_in_both_themes` (`#f8f6f1` light, `#0b0d11` dark, read off the window's pixels on Linux) | ☑ |

### e2e/cursor.spec.ts

Phase P2 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-cursor-01 | cursors match what a control does | `grace::elements::tests::a_button_shows_the_hand_and_a_disabled_one_the_not_allowed_sign`, `grace::tree::tests::a_rows_run_button_shows_the_hand`; by hand in the P2 exit check and [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md): the hand on buttons, labels and a row's Run button, not-allowed on a control turned off, the text cursor in the boxes | ☑ |
| E-cursor-02 | the busy cursor while running | No busy cursor in GPUI Grace: GPUI has none, and adding one to the Windows patch is more stack to carry; the progress bar, the status bar and the running marks show the run ([P2 plan, Redesigns](../plans/2026-11-09-gpui-p2-main-screen.md#redesigns)) | n/a |
| E-cursor-03 | the drag cursor wins over the busy cursor | No busy cursor to win over (UP-14); the splitter keeps its own resize cursor, `bottom_panel`'s `CursorStyle::ResizeUpDown` | n/a |

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
| E-details-01 | Failure details | Run half: `grace-e2e/tests/details.rs::failure_details`; the steps from the file before a run come with P4 (plan, Redesigns) | ☐ |

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
| E-env-01 | Suggestions come from overlay files and env tags | `grace-e2e/tests/env.rs::suggestions_come_from_overlay_files_and_env_tags` | ☑ |
| E-env-02 | Changing the environment updates the tree | `grace-e2e/tests/env.rs::changing_the_environment_updates_the_tree` | ☑ |
| E-env-03 | Changing the environment clears results | `grace-e2e/tests/env.rs::changing_the_environment_clears_results` | ☑ |
| E-env-04 | A run uses the environment | `grace-e2e/tests/env.rs::a_run_uses_the_environment` | ☑ |
| E-env-05 | An environment without an overlay file | `grace-e2e/tests/env.rs::an_environment_without_an_overlay_file` | ☑ |
| E-env-06 | Shared and environment properties, environment wins | `grace-e2e/tests/properties.rs::shared_and_environment_properties_environment_wins`; `grace-core::runner::tests::properties::shared_and_env_properties_env_wins` | ☑ |
| E-env-07 | A secret is used but never shown | `grace-e2e/tests/properties.rs::a_secret_is_used_but_never_shown` (Windows); `grace-core::runner::tests::properties::a_secret_reaches_the_run_and_is_masked` | ☑ |
| E-env-08 | Import a secrets file and run with it | `grace-e2e/tests/properties.rs::import_a_secrets_file_and_run_with_it` (Windows) | ☑ |
| E-env-09 | A secret stays off the command line and out of the settings file | `grace-e2e/tests/properties.rs::a_secret_stays_off_the_command_line_and_out_of_the_settings_file` (Windows); `grace-core::runner::tests::properties::the_secret_is_not_on_the_command_line`, `grace-core::runner::tests::properties::settings_never_hold_the_plain_secret` | ☑ |
| E-env-10 | Reserved names are refused | `grace-e2e/tests/properties.rs::reserved_names_are_refused`; `grace-core::properties_draft::tests::a_reserved_name_is_an_error_in_any_letter_case` | ☑ |
| E-env-11 | An unreadable secret blocks runs | `grace-e2e/tests/properties.rs::an_unreadable_secret_blocks_runs`; `grace-core::runner::tests::properties::an_unreadable_secret_blocks_the_run` | ☑ |
| E-env-12 | Settings survive a restart | `grace-e2e/tests/env.rs::the_environment_survives_a_restart`; `grace-e2e/tests/properties.rs::the_properties_survive_a_restart` (Windows) | ☑ |
| E-env-13 | The machine's KARATE_ENV is ignored | `grace-e2e/tests/env.rs::the_machines_karate_env_is_ignored` | ☑ |

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
| E-home-01 | A first start shows the home screen | `grace-e2e/tests/home.rs::a_first_start_shows_the_home_screen` | ☑ |
| E-home-02 | Opening a folder from the home screen | `grace-e2e/tests/home.rs::opening_a_folder_from_the_home_screen` | ☑ |
| E-home-03 | The last project opens at start without the home screen | `grace-e2e/tests/home.rs::the_last_project_opens_at_start_without_the_home_screen` | ☑ |
| E-home-04 | A missing last project shows the home screen | `grace-e2e/tests/home.rs::a_missing_last_project_shows_the_home_screen` | ☑ |
| E-home-05 | A dry-run error still opens the project | `grace-e2e/tests/home.rs::a_dry_run_error_still_opens_the_project` | ☑ |
| E-home-06 | The home screen is on the ground in light and in dark | `grace-e2e/tests/appearance.rs::follow_windows_is_the_default`, `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start` (the home screen's ground, light and dark) | ☑ |
| E-home-07 | Reopening a folder from the list | `grace-e2e/tests/home.rs::reopening_a_folder_from_the_list` | ☑ |
| E-home-08 | The folder dialog starts next to the newest existing folder | `grace-e2e/tests/home.rs::the_folder_dialog_starts_next_to_the_newest_existing_folder` | ☑ |
| E-home-09 | Removing a row | `grace-e2e/tests/home.rs::removing_a_row` | ☑ |
| E-home-10 | The list works from the keyboard | `grace-e2e/tests/home.rs::the_list_works_from_the_keyboard` | ☑ |
| E-home-11 | Home closes the project and survives a restart | `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` | ☑ |
| E-home-12 | Home is disabled during a run | `grace-e2e/tests/home.rs::home_is_disabled_during_a_run` | ☑ |
| E-home-13 | A reload after Home stays home | GPUI Grace has no page to reload: the window is drawn from the state, so Home stays until the state changes. Home surviving a restart: `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` | n/a |
| E-home-14 | A path not in the list is refused | No renderer and no IPC: the view calls the project code itself, and only the home list's own rows open a recent project | n/a |

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
| E-look-01 | The window follows Windows in ${scheme} mode | As PU-06: light by `grace-e2e/tests/appearance.rs::follow_windows_is_the_default`; dark: [the P2 recheck](../checks/2026-11-10-gpui-p2-recheck.md), with Windows in dark mode and no Appearance saved: the ground is `#151821` and the title bar dark. Windows switched while Grace is open is UP-18's, for a person | ☑ |
| E-look-02 | Grace draws its own typefaces without any network request | As PU-07: `grace::assets::tests::the_theme_asks_for_the_families_it_embeds`, `…embeds_ibm_plex`, `…serves_every_icon` | ☑ |
| E-look-03 | Each status has its own mark and name | `grace-e2e/tests/elements.rs::names_each_status_mark`, `grace::elements::tests::marks_each_status_as_electron_grace_does` (PU-08) | ☑ |
| E-look-04 | A cancelled run marks its interrupted test | `grace-e2e/tests/run.rs::cancel_leaves_nothing_behind` (PU-09) | ☑ |
| E-look-05 | Reduced motion stops every animation | `grace::window::tests::reduced_motion_stops_every_animation_during_a_run` (PU-12) | ☑ |
| E-look-06 | The logo is the Home button | `grace::toolbar::tests::home_and_refresh_the_icon_only_buttons::home_holds_the_logo_and_shows_home_in_a_tooltip`, `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` (PU-13) | ☑ |
| E-look-07 | A tooltip shows on keyboard focus and closes with Escape | `grace-e2e/tests/tooltips.rs::shows_on_keyboard_focus_and_closes_with_escape` (PU-19) | ☑ |
| E-look-08 | A tooltip shows after a pause on hover, also on a disabled button | `grace-e2e/tests/tooltips.rs::shows_on_hover_also_over_a_disabled_button` (PU-20) | ☑ |
| E-look-09 | A row's tooltip leaves the next row's run button to the pointer | `grace::tree::tests::a_rows_tooltips_stay_on_its_row` (the run button's tooltip to its left, the mark's to its right, the label's over it: none covers the next row) | ☑ |
| E-look-10 | Escape in a dialog closes a shown tooltip first, then the dialog | `grace-e2e/tests/tooltips.rs::escape_in_a_dialog_closes_the_tooltip_first` (PU-21) | ☑ |
| E-look-11 | Every control shows the focus ring in ${scheme} mode | `grace-e2e/tests/look.rs::every_control_shows_the_focus_ring_in_light_mode`, `…_in_dark_mode` (PU-23) | ☑ |
| E-look-12 | Targets are at least 24 by 24 | `grace-e2e/tests/look.rs::targets_are_at_least_24_by_24` (PU-24) | ☑ |
| E-look-13 | A first start shows the first-run home screen | `grace-e2e/tests/home.rs::a_first_start_shows_the_home_screen` (PU-26) | ☑ |
| E-look-14 | A missing recent folder says so in words | `grace::home::tests::says_not_found_in_a_cancelled_badge_with_the_warning_icon_and_the_row_is_disabled` (PU-27) | ☑ |
| E-look-15 | A folder with no feature files shows an empty state | `grace-e2e/tests/run.rs::nothing_to_run` (PU-28) | ☑ |
| E-look-16 | Before any run the progress line, the details and the tabs say so | The progress line and the details: `grace::progress::tests::before_any_run_r14_3::counts_the_projects_tests_and_says_not_run_yet_with_no_zero_counts`, `grace-e2e/tests/details.rs::failure_details` (PU-29, PU-30); the History and HTTP tabs with phase P3 | ☐ |
| E-look-17 | Clear history keeps its button order and closes on Escape | Waits for Clear history, phase P3 | ☐ |

### e2e/open.spec.ts

Phase P2 · 11 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-open-01 | Open a project and see its tests | `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` | ☑ |
| E-open-02 | A feature opens on a click on its name, and as a run in it starts (#224) | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one`, `grace-e2e/tests/tree.rs::ctrl_enter_runs_the_focused_row` (a run opens its feature) | ☑ |
| E-open-03 | A narrow window keeps every toolbar button on screen, with its name | `grace-e2e/tests/open.rs::a_narrow_window_keeps_every_toolbar_button_on_screen_with_its_name` (the floor on Windows only: Xvfb has no window manager to hold the minimum) | ☑ |
| E-open-04 | A long project path stays readable at the minimum width | `grace::window::tests::a_long_path_at_the_minimum_width`, `grace::window::tests::the_name_shows_whole_at_the_default_size` (headless: plan, Redesigns); the words and tooltips: TB-07, TB-08 | ☑ |
| E-open-05 | The Env and Threads boxes show their longest likely values whole | `grace::env_picker::tests::the_env_and_threads_boxes_show_their_longest_likely_values_whole` (headless: plan, Redesigns) | ☑ |
| E-open-06 | Maven layout with classpath resources | `grace-e2e/tests/open.rs::maven_layout_with_classpath_resources` | ☑ |
| E-open-07 | Syntax error is reported, not hidden | `grace-e2e/tests/open.rs::syntax_error_is_reported_not_hidden` | ☑ |
| E-open-08 | Nothing to run | `grace-e2e/tests/run.rs::nothing_to_run` | ☑ |
| E-open-09 | Open… starts in the last project's parent folder | `grace-e2e/tests/open.rs::open_starts_in_the_last_projects_parent_folder` | ☑ |
| E-open-10 | Open… with no last project | `grace-e2e/tests/open.rs::open_with_no_last_project` | ☑ |
| E-open-11 | the last project folder was deleted | `grace-e2e/tests/open.rs::the_last_project_folder_was_deleted` | ☑ |

### e2e/panel.spec.ts

Phase P2 · 13 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-panel-01 | the bottom panel starts at its default height | `grace-e2e/tests/panel.rs::the_bottom_panel_starts_at_its_default_height` | ☑ |
| E-panel-02 | dragging the splitter resizes the bottom panel | `bottom_panel::tests::splitter::dragging_resizes_within_the_limits_and_saves_on_release` (headless: plan, Redesigns) | ☑ |
| E-panel-03 | the splitter works from the keyboard | `grace-e2e/tests/panel.rs::the_splitter_works_from_the_keyboard` | ☑ |
| E-panel-04 | the panel stays within its limits | `bottom_panel::tests::splitter::dragging_resizes_within_the_limits_and_saves_on_release`, `the_keys_step_and_jump_and_save` | ☑ |
| E-panel-05 | the window cannot get too short for the layout | `bottom_panel::tests::splitter::a_smaller_window_shrinks_the_panel_without_forgetting_its_height` (headless: plan, Redesigns) | ☑ |
| E-panel-06 | the height survives a restart | `grace-e2e/tests/panel.rs::the_height_survives_a_restart` | ☑ |
| E-panel-07 | a smaller window shrinks the panel without forgetting its height | `bottom_panel::tests::splitter::a_smaller_window_shrinks_the_panel_without_forgetting_its_height` | ☑ |
| E-panel-08 | a bad saved height falls back to the default | `grace-e2e/tests/panel.rs::a_bad_saved_height_falls_back_to_the_default` | ☑ |
| E-panel-09 | a saved height taller than the window is clamped at start | `grace-e2e/tests/panel.rs::a_saved_height_taller_than_the_window_is_clamped_at_start`; `bottom_panel::tests::splitter::a_saved_height_taller_than_the_window_is_clamped` | ☑ |
| E-panel-10 | releasing a drag above the window keeps the maximum and ends the drag | `bottom_panel::tests::splitter::dragging_resizes_within_the_limits_and_saves_on_release` (released far above the window) | ☑ |
| E-panel-11 | focus cannot scroll the toolbar out of a cut-off layout | GPUI never scrolls the layout to the focus (plan, Redesigns) | ☑ |
| E-panel-12 | Tab moves focus past the splitter | `grace-e2e/tests/panel.rs::the_splitter_works_from_the_keyboard` | ☑ |
| E-panel-13 | the tabs can be dragged or moved with Alt+Left and Alt+Right, and their order survives a restart | `grace-e2e/tests/panel.rs::the_tabs_move_with_alt_left_and_alt_right_and_their_order_survives_a_restart`; `bottom_panel::tests::tabs::a_dragged_tab_takes_the_place_of_the_tab_it_is_dropped_on_and_is_saved` | ☑ |

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
| E-report-01 | Open the HTML report | `grace-e2e/tests/run.rs::report_opens_the_runs_html_report` (Run all in place of the outline feature's own button) | ☑ |

### e2e/run.spec.ts

Phase P2 · 4 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-run-01 | Run all with live progress | `grace-e2e/tests/run.rs::run_all_with_live_progress` | ☑ |
| E-run-02 | Run one outline row | `grace-e2e/tests/run.rs::run_one_outline_row` | ☑ |
| E-run-03 | A feature edited outside the app | `grace-e2e/tests/run.rs::a_feature_edited_outside_the_app` | ☑ |
| E-run-04 | The project is not modified | `grace-e2e/tests/run.rs::the_project_is_not_modified` | ☑ |

### e2e/scrollbar.spec.ts

Phase P2 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-scrollbar-01 | scroll areas get thin themed scrollbars | `grace::theme::tests::scrollbars_are_thin_themed_and_always_shown` | ☑ |

### e2e/startup.spec.ts

Phase P2 · 3 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-startup-01 | the window opens painted in the app's theme | `grace-e2e/tests/appearance.rs::follow_windows_is_the_default`, `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start` | ☑ |
| E-startup-02 | the page starts with a static startup screen | GPUI Grace has no page and no startup screen: its first frame is its own, in the theme (plan, Redesigns) | n/a |
| E-startup-03 | a reload shows the app once | GPUI Grace has no page to reload (plan, Redesigns) | n/a |

### e2e/tags.spec.ts

Phase P2 · 11 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-tags-01 | Chips list the project's tags | `grace-e2e/tests/tags.rs::chips_list_the_projects_tags` | ☑ |
| E-tags-02 | Include and exclude dim the tree | `grace-e2e/tests/tags.rs::include_and_exclude_dim_the_tree` | ☑ |
| E-tags-03 | Third click turns a chip off | `grace-e2e/tests/tags.rs::a_third_click_turns_a_chip_off` | ☑ |
| E-tags-04 | Hide non-matching | `grace-e2e/tests/tags.rs::hide_non_matching_hides_the_other_rows` | ☑ |
| E-tags-05 | Run all runs only matching tests | `grace-e2e/tests/tags.rs::run_all_runs_only_matching_tests` | ☑ |
| E-tags-06 | A feature run is filtered | `grace-e2e/tests/tags.rs::a_feature_run_is_filtered` | ☑ |
| E-tags-07 | A tag removed before a run does not widen it | `grace-e2e/tests/tags.rs::a_tag_removed_before_a_run_does_not_widen_it` | ☑ |
| E-tags-08 | Part of an outline matches | `grace-e2e/tests/tags.rs::part_of_an_outline_matches` | ☑ |
| E-tags-09 | Nothing matches | `grace-e2e/tests/tags.rs::nothing_matches` (the counter); Run all disabled: `grace::toolbar::tests` (WS-10) | ☑ |
| E-tags-10 | The filter survives a restart | `grace-e2e/tests/tags.rs::the_filter_survives_a_restart` | ☑ |
| E-tags-11 | A tag removed from the project leaves the filter | `grace-e2e/tests/tags.rs::a_tag_removed_from_the_project_leaves_the_filter` | ☑ |

### e2e/threads.spec.ts

Phase P2 · 8 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-threads-01 | A project with no saved count runs with 1 thread | `grace-e2e/tests/threads.rs::a_project_with_no_saved_count_runs_with_1_thread` | ☑ |
| E-threads-02 | Run all uses the count in the box | `grace-e2e/tests/threads.rs::run_all_uses_the_count_in_the_box` | ☑ |
| E-threads-03 | A single test's run uses the count | `grace-e2e/tests/threads.rs::a_single_tests_run_uses_the_count` | ☑ |
| E-threads-04 | Re-run failed uses the current count | `grace-e2e/tests/threads.rs::rerun_failed_uses_the_current_count` | ☑ |
| E-threads-05 | The box starts from karate-pom.json and the person's count wins | `grace-e2e/tests/threads.rs::the_box_starts_from_karate_pom_json_and_the_persons_count_wins` | ☑ |
| E-threads-06 | The box is disabled while a run is going | `grace-e2e/tests/threads.rs::the_box_is_disabled_while_a_run_is_going` | ☑ |
| E-threads-07 | The count is saved per project | `grace-e2e/tests/threads.rs::the_count_is_saved_per_project` | ☑ |
| E-threads-08 | A value out of range is refused | `grace-e2e/tests/threads.rs::a_value_out_of_range_is_refused` | ☑ |

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
| E-virtual-01 | Only the rows in view are drawn | `grace-e2e/tests/tree.rs::only_the_rows_in_view_are_drawn` (300 features, generated) | ☑ |
| E-virtual-02 | A moved-to row scrolls into view | `grace-e2e/tests/tree.rs::only_the_rows_in_view_are_drawn` (End, in place of 40 Down presses) | ☑ |
| E-virtual-03 | Focus survives scrolling away | `grace::tree::tests::focus_survives_scrolling_away` (headless: plan, Redesigns) | ☑ |
| E-virtual-04 | A new tree keeps the rows in view still | `grace::tree::tests::a_feature_opening_above_the_view_does_not_move_it` (headless: plan, Redesigns) | ☑ |
| E-virtual-05 | The filter bar keeps one line with 54 tags | `grace-e2e/tests/tags.rs::many_tags_keep_a_one_line_bar` (54 generated tags) | ☑ |
| E-virtual-06 | A filter change scrolls to the selected row | `grace-e2e/tests/tags.rs::a_filter_change_scrolls_to_the_selected_row` | ☑ |
| E-virtual-07 | A click shows neither the focus ring nor a tooltip | `grace::tree::tests::a_click_makes_the_row_focused`, `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus` (a click shows no tooltip) | ☑ |
| E-virtual-08 | A click on an HTTP call shows no focus ring | Waits for the HTTP tab, phase P3 | ☐ |
| E-virtual-09 | The keyboard shows the focus ring and a cut label's tooltip | `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus`, `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` | ☑ |

### e2e/watch.spec.ts

Phase P4 · 1 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-watch-01 | Re-run on save (docs/specs/2026-11-03-watch-mode.md) | | ☐ |

### e2e/window.spec.ts

Phase P2 · 12 tests

| ID | Test | Ported as | Ported |
|---|---|---|---|
| E-window-01 | the window comes back where it was | `grace-e2e/tests/window.rs::the_window_comes_back_where_it_was` | ☑ |
| E-window-02 | a maximized window comes back maximized | `grace-e2e/tests/window.rs::a_maximized_window_comes_back_maximized` (Windows only: Xvfb has no window manager to maximize) | ☑ |
| E-window-03 | a move by the person is saved before the app closes | `grace-e2e/tests/window.rs::a_move_by_the_person_is_saved_before_the_app_closes` | ☑ |
| E-window-04 | a window closed while minimized opens normal | `grace-e2e/tests/window.rs::a_window_closed_while_minimized_opens_normal` (Windows only: Xvfb has no window manager to maximize) | ☑ |
| E-window-05 | a failed save does not stop the app from closing | `grace-e2e/tests/window.rs::a_failed_save_does_not_stop_the_app_from_closing` | ☑ |
| E-window-06 | a short saved window is held at the minimum height | `grace-e2e/tests/window.rs::a_short_saved_window_is_held_at_the_minimum_height` | ☑ |
| E-window-07 | a broken saved state opens the window as on a first start | `grace-e2e/tests/window.rs::a_broken_saved_state_opens_the_window_as_on_a_first_start` | ☑ |
| E-window-08 | a window partly off the screen is pulled onto it | `grace-e2e/tests/window.rs::a_window_partly_off_the_screen_is_pulled_onto_it` | ☑ |
| E-window-09 | a window flush with the screen edges keeps its size and place | `grace-e2e/tests/window.rs::a_window_flush_with_the_screen_edges_keeps_its_size_and_place` | ☑ |
| E-window-10 | a window bigger than the screen is shrunk to fit | `grace-e2e/tests/window.rs::a_window_bigger_than_the_screen_is_shrunk_to_fit` | ☑ |
| E-window-11 | a window on a screen that is gone opens on the main screen | `grace-e2e/tests/window.rs::a_window_on_a_screen_that_is_gone_opens_on_the_main_screen` | ☑ |
| E-window-12 | a maximized window on a screen that is gone opens maximized on the main screen | `grace-e2e/tests/window.rs::a_maximized_window_on_a_screen_that_is_gone_opens_maximized_on_the_main_screen` (Windows only: Xvfb has no window manager to maximize) | ☑ |

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
| `src/main/karate/pom.test.ts` | 19 | Partly, in P1: `grace-core::karate::pom::tests` (`read_pom`, 23 tests). The 5 `pomStart` tests port with `maven.rs` in P4 | ☐ |
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
| `src/main/runner.discovery.test.ts` | 11 | `grace-core::runner::tests::discovery` (11 tests, real Java and Karate) | ☑ |
| `src/main/runner.editor.test.ts` | 2 | `grace-core::runner` (editor) | ☐ |
| `src/main/runner.env.test.ts` | 12 | `grace-core::runner::tests::env` (12 tests, real Java and Karate) | ☑ |
| `src/main/runner.fingerprint.test.ts` | 10 | `grace-core::runner::tests::fingerprint` (10 tests, real Java and Karate) | ☑ |
| `src/main/runner.flaky.test.ts` | 3 | `grace-core::runner::tests::flaky` (3 tests, real Java and Karate) | ☑ |
| `src/main/runner.history.test.ts` | 15 | `grace-core::runner::tests::history` (16 tests, real Java and Karate) | ☑ |
| `src/main/runner.home.test.ts` | 13 | `grace-core::runner::tests::home` (13 tests, real Java and Karate) | ☑ |
| `src/main/runner.libraries.test.ts` | 31 | `grace-core::runner` (libraries) | ☐ |
| `src/main/runner.pom.test.ts` | 12 | `grace-core::runner` (pom) | ☐ |
| `src/main/runner.properties.test.ts` | 18 | `grace-core::runner::tests::properties` (19 tests, real Java and Karate) (one, a file another process holds, runs on Windows only) | ☑ |
| `src/main/runner.rerun.test.ts` | 11 | `grace-core::runner::tests::rerun` (11 tests, real Java and Karate) | ☑ |
| `src/main/runner.runs.test.ts` | 13 | `grace-core::runner::tests::runs` (14 tests, real Java and Karate) (and one new: a cancel just before Java starts) | ☑ |
| `src/main/runner.setup.test.ts` | 9 | `grace-core::runner` (setup) | ☐ |
| `src/main/runner.subfolders.test.ts` | 7 | `grace-core::runner` (subfolders) | ☐ |
| `src/main/runner.tags.test.ts` | 13 | `grace-core::runner::tests::tags` (13 tests, real Java and Karate) | ☑ |
| `src/main/runner.threads.test.ts` | 8 | `grace-core::runner::tests::threads` (8 tests, real Java and Karate) | ☑ |
| `src/main/runner.trends.test.ts` | 2 | `grace-core::runner::tests::trends` (2 tests, real Java and Karate) | ☑ |
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
| `src/renderer/Banner.test.tsx` | 10 | view `Banner` (`grace::ui`), headless: `console::tests::banner`; `banner::tests` (the states) | ☑ |
| `src/renderer/BottomPanel.test.tsx` | 17 | view `BottomPanel` (`grace::ui`), headless: `bottom_panel::tests` (the tabs, the drag, the keys, the saved order); the look tests with PR 6 | ☑ |
| `src/renderer/CallDetails.test.tsx` | 16 | view `CallDetails` (`grace::ui`), headless | ☐ |
| `src/renderer/ConfigBanner.test.tsx` | 6 | view `ConfigBanner` (`grace::ui`), headless | ☐ |
| `src/renderer/Console.test.tsx` | 30 | view `Console` (`grace::ui`), headless: `console::tests` (25 tests: the lines, find, errors); the highlight-API tests go (plan, Redesigns) | ☑ |
| `src/renderer/Details.test.tsx` | 33 | `grace::details::tests` and `grace-e2e/tests/details.rs`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (35 runs with the `it.each` cases: 12 ported, 1 `n/a` that counts Solid memo calls; 17 wait for P3, Open in VS Code and the failure groups, and 5 for P4, the steps before a run and breakpoints) | ☑ |
| `src/renderer/EmptyState.test.tsx` | 3 | `grace::ui::tests` (2 tests; "has no test id unless given one" is `n/a`: a DOM test id) | ☑ |
| `src/renderer/EnvPicker.test.tsx` | 18 | view `EnvPicker` (`grace::ui`), headless; ported in `grace::env_picker::tests` (18: the 3 `words` tests and 15 views) | ☑ |
| `src/renderer/Examples.test.tsx` | 10 | view `Examples` (`grace::ui`), headless | ☐ |
| `src/renderer/FieldMessage.test.tsx` | 2 | `grace::ui::tests`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (5 runs with the `it.each` cases, all ported) | ☑ |
| `src/renderer/FilterBar.test.tsx` | 21 | `grace::filter_bar::tests` (20; the dimming and hiding of rows are `grace_core::rows`' and `grace::tree`'s tests, the plain-data test has no IPC to cross) | ☑ |
| `src/renderer/FlakyProof.test.tsx` | 8 | view `FlakyProof` (`grace::ui`), headless | ☐ |
| `src/renderer/History.test.tsx` | 30 | view `History` (`grace::ui`), headless | ☐ |
| `src/renderer/Home.test.tsx` | 26 | `grace::home::tests`, `grace::window::tests`, `grace::toolbar::tests`, `grace_core::state::tests` and `grace-e2e/tests/home.rs`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (all 26 ported) | ☑ |
| `src/renderer/HttpCalls.test.tsx` | 26 | view `HttpCalls` (`grace::ui`), headless | ☐ |
| `src/renderer/Icon.test.tsx` | 3 | `grace::elements::tests::draws_buttons_icons_and_marks_at_their_sizes`, `grace::assets::tests` ("one child per node" is `n/a`: GPUI draws the Lucide file); [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) | ☑ |
| `src/renderer/Notice.test.tsx` | 6 | `grace::ui::tests` (6 tests) | ☑ |
| `src/renderer/Progress.test.tsx` | 18 | `grace::progress::tests` (all 18; the Chromium hold test is replaced by the spring, the palette test by `theme::tests::colours_come_only_from_the_theme`) | ☑ |
| `src/renderer/PropertiesDialog.test.tsx` | 54 | `grace-core::properties_draft::tests` (20) and `grace::properties::tests` (18), headless; the fade-out group goes (P2 plan, Redesigns); the look tests (spec R19) wait for P2 PR 6's check by eye | ☐ |
| `src/renderer/Settings.test.tsx` | 43 | `grace::settings::tests` and `grace-e2e/tests/appearance.rs`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (6 ported, the radios' accent by eye in PR 6d; 36 wait for P5's Updates section) | ☑ |
| `src/renderer/StatusBar.test.tsx` | 18 | `grace::status_bar::tests`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (6 ported; 12 wait for P5's update marker and version) | ☑ |
| `src/renderer/StatusMark.test.tsx` | 8 | `grace::elements::tests` (the statuses, shapes, colours, turning and sizes); [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) | ☑ |
| `src/renderer/SwitchDialog.test.tsx` | 12 | view `SwitchDialog` (`grace::ui`), headless | ☐ |
| `src/renderer/Threads.test.tsx` | 6 | view `Threads` (`grace::ui`), headless; ported in `grace::threads_box::tests` (14) | ☑ |
| `src/renderer/Toolbar.test.tsx` | 36 | `grace::toolbar::tests` and `grace::window::tests`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (26 ported, 2 by eye in PR 6d, 2 `n/a`: a CSS class and `title=`; 6 wait for P4, Watch and the step debugger) | ☑ |
| `src/renderer/Tooltip.test.tsx` | 46 | `grace::tooltip::tests` (23 tests) and the toolbar's and tags' tests, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests); 12 are `n/a` (the DOM's `popover`, CSS classes, Solid, describedby ids) | ☑ |
| `src/renderer/Tree.test.tsx` | 86 | `grace::tree::tests`, `grace_core::rows::tests` and `grace-e2e/tests/tree.rs`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (92 runs with the `it.each` cases: 73 ported, the look of the selected row by eye in PR 6d, 3 `n/a` that count Solid memo calls; 15 wait for P3, the VS Code menu, and P4, the debugger, Build and sub-folders) | ☑ |
| `src/renderer/Trends.test.tsx` | 15 | view `Trends` (`grace::ui`), headless | ☐ |
| `src/renderer/UpdateModal.test.tsx` | 13 | view `UpdateModal` (`grace::ui`), headless | ☐ |
| `src/renderer/UpdateProgress.test.tsx` | 13 | view `UpdateProgress` (`grace::ui`), headless | ☐ |
| `src/renderer/Variables.test.tsx` | 4 | view `Variables` (`grace::ui`), headless | ☐ |
| `src/renderer/VirtualList.test.tsx` | 28 | `grace::tree::tests` and `grace-e2e/tests/tree.rs` (the tree is the list), test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests); 3 `n/a` (a Solid prop and memo, DOM node identity); 2 wait for P3's lists (the data-driven view, the HTTP tab) | ☑ |
| `src/renderer/bannerText.test.ts` | 15 | `grace::ui::bannerText`: `grace-core::banner::tests` | ☑ |
| `src/renderer/buttons.test.ts` | 2 | `grace::elements::tests::draws_buttons_icons_and_marks_at_their_sizes` | ☑ |
| `src/renderer/consoleFind.test.ts` | 9 | `grace::ui::consoleFind`: `grace-core::console_find::tests` | ☑ |
| `src/renderer/failures.test.ts` | 10 | `grace::ui::failures` | ☐ |
| `src/renderer/http.test.ts` | 9 | `grace::ui::http` | ☐ |
| `src/renderer/importPlan.test.ts` | 6 | `grace-core::import_plan::tests` (6 tests) | ☑ |
| `src/renderer/lastChecked.test.ts` | 4 | `grace::ui::lastChecked` | ☐ |
| `src/renderer/nodes.test.ts` | 3 | `grace_core::rows::tests::feature_of` (all 3) | ☑ |
| `src/renderer/palette.test.ts` | 6 | `crates/grace::theme::tests::colours_come_only_from_the_theme`, which scans every file but `theme.rs` and knows its patterns. Its `dark:` and `title=` checks have nothing to find in GPUI, which has neither | ☑ |
| `src/renderer/panelHeight.test.ts` | 8 | `grace::ui::panelHeight`: `grace-core::panel_height::tests`; `settings::tests::panel_height` | ☑ |
| `src/renderer/panelTabs.test.ts` | 7 | `grace::ui::panelTabs`: `grace-core::panel_tabs::tests`; `settings::tests::panel_tabs` | ☑ |
| `src/renderer/placeTip.test.ts` | 16 | `grace::tooltip::tests::goes_beside_its_target_on_the_side_asked_and_below_by_default`, `stays_in_the_window_flipping_where_it_has_no_room`, `a_click_on_an_over_tooltip_within_its_target_reaches_the_target` (GPUI Kit's `Positioner` places it); [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) | ☑ |
| `src/renderer/reduce.test.ts` | 45 | `grace-core::state::tests` (45 tests, every Vitest test; the state lives in `grace-core`, P2 plan, Redesigns) | ☑ |
| `src/renderer/store.test.tsx` | 8 | `grace-core::state::tests::create_app_store`, test for test in [P2 plan, The component tests](../plans/2026-11-09-gpui-p2-main-screen.md#the-component-tests) (7 ported; `toggleGroup` ports with the HTTP tab in P3) | ☑ |
| `src/renderer/tokens.test.ts` | 19 | `crates/grace::theme::tests`: every token in both themes and equal to `src/index.css` until the cut-over, the contrast rules, the primary's green, the focus ring, the sheet and the shadow's geometry. The CSS-only checks (the `::highlight` rules, `color-scheme`, Tailwind's palette reset) have no GPUI counterpart; the console's highlights port with the console (CS-*) | ☑ |
| `src/renderer/treeRows.test.ts` | 5 | `grace_core::rows::tests::tree_rows`, `row_name_…` (all 5) | ☑ |
| `src/renderer/trendLabels.test.ts` | 7 | `grace::ui::trendLabels` | ☐ |
| `src/renderer/ui.test.ts` | 1 | `n/a`: a closed GPUI dialog is not drawn, so it takes no pointer events | n/a |
| `src/renderer/virtual.test.ts` | 15 | `grace_core::rows::tests` (10; `drawnRange`'s 5 are `uniform_list`'s) | ☑ |
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

