---
status: approved
issue: 282
summary: "GPUI phase 3: the History and HTTP tabs and detail views in GPUI: HTTP inspector, run history, trends, flaky proof, report export, grouped failures"
---

# Plan: GPUI migration, phase P3 panels

Date 2026-11-12. Issue [#282](https://github.com/hpagniez/grace/issues/282), under [#278](https://github.com/hpagniez/grace/issues/278). Spec: the [migration guide](../specs/2026-11-08-gpui-migration.md) (R1 to R9, feature map P3, regressions #219 and #220), [ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md). Checklist: [parity](../specs/2026-11-08-gpui-migration-parity.md). Model: the [P2 plan](2026-11-09-gpui-p2-main-screen.md).

P3 fills the bottom panel's History and HTTP tabs and the detail views that P2 left empty: the HTTP inspector with Copy as cURL, run history with re-run and clear, trends, the flaky proof, export of a run's report, the data-driven view, the config banner and grouped failures, step comments, Open in VS Code from Details and the tree, and the panel tabs' order. It ends when the rows below are ticked and its exit check is recorded. The Electron app does not change.

## Rows to tick

Counted from the parity file on `develop` at `3487164`, by its Ported column.

| Part of the list | Open | Of which |
|---|---|---|
| Spec scenarios | 138 | 130 of P3's own sections (HI 11, RH 13, FL 9, DD 7, ER 11, OT 6, OE 11, SC 8, GF 22, RP 14, TR 18) and 8 handed on: PU-11, PU-31, PU-32, PU-33's panel screenshots from P0; VL-13, VL-19, VL-25, VL-27's HTTP half from P2 |
| End-to-end tests | 36 | 33 of P3's files (`editor` 3, `examples` 1, `failures` 1, `flaky` 7, `history` 14, `http` 4, `trends` 3) and 3 handed on: E-look-16's tab half, E-look-17, E-virtual-08. E-report-01 is ticked already |
| Unit and component files | 13 files, 155 tests | `editor.test.ts` 10, `runner.editor.test.ts` 2, `report-zip.test.ts` 6, `http.test.ts` 9, `failures.test.ts` 10, `trendLabels.test.ts` 7, `HttpCalls.test.tsx` 26, `CallDetails.test.tsx` 16, `History.test.tsx` 30, `Trends.test.tsx` 15, `FlakyProof.test.tsx` 8, `Examples.test.tsx` 10, `ConfigBanner.test.tsx` 6 |
| Tests waiting in files P2 ticked | 27 | `Details.test.tsx` 17 (tests 18 to 28 and 30 to 34 of the [P2 plan's table](2026-11-09-gpui-p2-main-screen.md#detailstesttsx-34-tests-one-an-iteach-of-2)), `Tree.test.tsx` 7 (28, 86 to 91), `VirtualList.test.tsx` 2 (9, 23), `store.test.tsx` 1 (`toggleGroup`) |

Three of P3's rows need the step debugger and are handed on to P4 by their Proof text, in slice 1: SC-06 (the paused step's comments), SC-08's half before a run (the file's steps, as E-details-01), GF-04 (no breakpoint dot on Karate's synthetic step).

## What is already there

P1 ported the logic and P2 the frame, so most of P3 is views:

- `grace-core`: `history`, `flaky`, `fingerprint`, `trends`, `karate::http`, `mask`, `karate::steps` (comments), `karate::pom` and `karate::command` (JUnit XML), `panel_tabs`. The runner has `rerun`, `rerun_failed`, `clear_history`, `trend`, `run_report_path` and `export_source`. `AppState` holds `calls`, `http_groups`, `selected_call`, `history`, `flaky` and each result's `reason_at`.
- `grace`: the bottom panel with its three tabs, their drag, Alt+Left and Alt+Right and the saved order; History's count; Details with a step's comments above it; the tree's flaky marks; the `pick_folder`, `pick_file` and `open_path` seams.
- Not there: `grace-core::editor`, `grace-core::report_zip`, `AppState::toggle_group`, the views `grace::{http, history, trends, flaky_proof, examples, failures}`, a context menu, and seams for a URL, the clipboard and a save dialog.

## Porting rules

The rules of [P1](2026-11-09-gpui-p1-core.md#porting-rules) and [P2](2026-11-09-gpui-p2-main-screen.md#porting-rules) hold: logic in `grace-core` and drawing in `grace`, the same words and accessible names, accessibility by hand, nothing that wakes while idle, the outside world behind `grace::seams`, port and do not transcribe. And for P3:

1. **A list is the tree's kind of list.** The HTTP list and the Examples table draw only the rows in view, are one tab stop, and tell assistive technology their focused row as the tree does (P2 Redesigns, `tree`). Their scroll rules are `grace-core::rows`'s.
2. **A secret never shows.** Calls, cURL text, history files, comments and the exported zip hold masked text only (HI-09, RH-12, SC-07). Each view test that shows text from a run has a case with a secret in it.
3. **Nothing is worked out for a closed tab.** Trends asks the runner only while History shows (TR-17); the HTTP list builds rows only while its tab shows, and a run's calls cost nothing more than storing them (#124).
4. **Tests never reach outside.** VS Code, the clipboard and the save dialog are asked through the seams, which the `e2e` build answers and records in `e2e-seams.jsonl`.
5. **A row is ticked in the slice that proves it.** A row that two slices share (PU-31, E-look-16) is ticked by the later one.

## Slices

One PR each, in this order, each `Part of #282`, each from the merged `develop`. A slice ends when its rows are ticked with their proofs, its Redesigns rows are written, and the checks of [GPUI Grace's rules](../../.claude/rules/gpui.md) pass: format, clippy, `cargo test --workspace`, the whole e2e suite with its summary in the PR, before and after screenshots, and the Windows run by hand.

| Slice | What it does | Rows that end it | Model |
|---|---|---|---|
| 1 | Tick what P1 and P2 already built, each row with the test that proves it; write the test where none does | RP-01 to RP-05, RP-07 to RP-14; SC-01 to SC-05, SC-07; FL-08, FL-09; ER-01 to ER-04; GF-01 to GF-03, GF-05, GF-08. SC-06, SC-08 and GF-04 handed on to P4 | `sonnet` |
| 2 | Open in VS Code: `grace-core::editor`, the URL seam, the button and the error's link in Details, the tree's context menu | OE-01 to OE-11, OT-01 to OT-06; E-editor-01 to 03; `editor.test.ts`, `runner.editor.test.ts`; `Details.test.tsx` 18 to 22; `Tree.test.tsx` 86 to 91; #219, #220 | `sonnet` |
| 3 | The HTTP tab's list: groups, only the calls that went wrong, the tree selection, the count, the keys | HI-01, 02, 03, 05, 06, 11; VL-13, VL-19, VL-25; PU-11; E-http-02, E-http-04, E-virtual-08; `http.test.ts`, `HttpCalls.test.tsx`; `store.test.tsx` `toggleGroup`; `VirtualList.test.tsx` 23 | `opus` |
| 4 | A call's details and Copy as cURL | HI-04, 07, 08, 09, 10; E-http-01, E-http-03; `CallDetails.test.tsx` | `sonnet` |
| 5 | The History tab: the runs, Re-run, Re-run failed, the environment switch, Clear history | RH-01 to RH-13; PU-31, PU-32; E-history-01 to 13; E-look-16, E-look-17; `History.test.tsx` | `sonnet` |
| 6 | Export a run's report: `grace-core::report_zip`, the save dialog seam, the Export button | ER-05 to ER-11; E-history-14; `report-zip.test.ts` | `sonnet` |
| 7 | Trends: the strip above the runs | TR-01 to TR-18; E-trends-01 to 03; `Trends.test.tsx`, `trendLabels.test.ts` | `sonnet` |
| 8 | The flaky proof in Details | FL-01 to FL-07; E-flaky-01 to 07; `FlakyProof.test.tsx`; `Tree.test.tsx` 28 | `sonnet` |
| 9 | The data-driven view: an outline's Examples as a table | DD-01 to DD-07; E-examples-01; `Examples.test.tsx`; `VirtualList.test.tsx` 9 | `sonnet` |
| 10 | The config banner and failures grouped by error | GF-06, GF-07, GF-09 to GF-22; E-failures-01; `failures.test.ts`, `ConfigBanner.test.tsx`; `Details.test.tsx` 23 to 28, 30 to 34 | `sonnet` |
| 11 | The exit check | PU-33, VL-27, RP-06; the regressions table; the check record | `sonnet` |

The HTTP list is marked hard: it is P3's one virtual list with its own grouping, keyboard and anchoring, it must stay cheap during a run of thousands of calls (#124, VL-27), and four later rows lean on it. Spec rows: 33 + 17 + 10 + 5 + 15 + 7 + 18 + 7 + 7 + 16 + 3 = 138 (slice 1 counts the 3 handed on to P4; RP-06, GF-06 and GF-07 moved, see slice 1). E2e rows: 0 + 3 + 3 + 2 + 15 + 1 + 3 + 7 + 1 + 1 + 0 = 36.

### Slice 1: what is already built

No new behaviour is expected. For each row, find the Rust test that proves it and tick the row with it:

- RP-*: `grace::bottom_panel::tests::tabs`, `grace-core::panel_tabs::tests`, `settings::tests::panel_tabs`, `grace-e2e/tests/panel.rs`. RP-02 (Variables stays last) is `panel_tabs`'s rule; the tab itself is P4's.
- SC-01 to SC-05, SC-07: `grace-core::karate::steps`, `mask`, `grace::details::tests::a_test::shows_a_steps_comments_above_it`.
- FL-08, FL-09: `grace-core::flaky::tests`, `fingerprint::tests`.
- ER-01 to ER-04: `grace-core::karate::pom::tests`, `karate::command::tests` (`junit:xml` in `-f`).
- GF-01 to GF-03, GF-05 to GF-08: the run's statuses and reasons in `grace-core::karate::{events, run_state}` and `state` (`reason_at`).

What slice 1 found: RP-06, GF-06 and GF-07 were not ready to tick. GF-06 and GF-07 (two failures share an error once Karate's position is left out) are `failures.ts`'s rule, which is not ported: they go to slice 10. RP-06 has three ends of a drag: dropped on itself and outside the tabs are proven; Escape is not, because GPUI has no Escape handler for a drag and in a headless window any key ends it, so only a real window can tell. It goes to slice 11's by-hand run, with the tab drag; if Escape then does not cancel the drag, that is a bug to fix in a PR of its own. The other 30 rows are ticked. The tests written for them (the drop bar, a drop outside the tabs, a file dragged in, Alt+Right at the last place with the shown tab, an unusable saved order, the dry run without `junit:xml`, the two pom notices) passed at once, the behaviour being there. RP-02 is ticked for its order half: the Variables tab itself is P4's.

A row with no test gets one, written first and seen failing where the behaviour is missing. A row whose behaviour turns out not to be built goes to its feature's slice (RP to this one, SC and GF to slice 10, ER to slice 6, FL to slice 8), said in the PR. SC-06, SC-08 and GF-04 get their Proof text "Waits for the step debugger, phase P4".

### Slice 2: Open in VS Code

Specs: [open in editor](../specs/2026-11-03-open-in-editor.md), [from the tree](../specs/2026-11-03-open-in-editor-from-tree.md).

1. `grace-core::editor` from `editor.ts`: the `vscode://file/…` link for a path and a line, a share and a mapped drive included (#219, OE-08, OE-09). `editor.test.ts` test for test.
2. The runner's open-in-editor call from `runner.editor.test.ts`: only a file of the open project (OE-06), a tree feature resolved under the classpath (OE-05).
3. `Seams::open_url`, with GPUI's `open_url`; the `e2e` build records the link and can answer that VS Code is not installed (OE-11).
4. Details: the Open in VS Code button, shown during a run too (OE-01, OE-02), and the error's place as a link (OE-03, OE-04). A long place that wraps starts at the left (#220).
5. The tree: GPUI Kit's `PopupMenu`, opened by the Tree on a right-click and, from the focused row, on the menu key or Shift+F10, with "Open in VS Code" for a test and a feature, none for a folder (OT-01 to OT-04). Built as Redesigns, `tree`, says: Kit's `ContextMenu` has no way to open from a key.
6. OE-07, OT-05 and E-editor-03's malformed requests: there is no IPC to send one through. The rows are `n/a` by the type of the call, as E-appearance-06 was; the runner's own refusal (item 2) keeps its test. Nobody using Grace could notice these.

### Slice 3: the HTTP list

Spec: [HTTP inspector](../specs/2026-10-25-http-inspector.md), [virtual lists](../specs/2026-10-30-virtual-lists.md).

1. `http.ts`'s rules in `grace-core` (the rows of the list from `calls`, `http_groups` and the tree selection; the status groups), with `http.test.ts` test for test, and `AppState::toggle_group` with its store test.
2. `grace::http`: the list on `uniform_list`, grouped by test with a called feature's and a shared setup's calls under the scenario that ran them (HI-05, HI-06), "only the calls that went wrong" (HI-02), narrowed by the tree selection (HI-03), a single test's run replacing only its calls (HI-11).
3. A status badge with its code and its group in the tooltip (PU-11); the empty state (PU-31's HTTP half); the tab's count pill.
4. Keys and accessibility: one tab stop, arrows open calls (VL-13), calls are options of a list box (VL-25), no focus ring after a click (E-virtual-08), rows in view stay still when calls arrive above (VL-19).
5. Cost during a run (#124): a headless test that a run's calls build no rows while the tab is closed.

What slice 3 built: `grace_core::http_list` (`http.ts`'s rules, and the tab's count read without listing the calls), `AppState::toggle_group` and `calls_changes`, and `grace::http`, drawn by the bottom panel only while its tab shows. The list is flat in run order, each row naming its test, as Electron Grace's: "grouped by test" above means the calls belong to the test that made them, a called feature's and a shared setup's included (HI-05, HI-06). Selecting a call sets `selected_call`; the details pane still shows the node until slice 4. The tests, test for test:

#### `http.test.ts` (9) in `grace_core::http_list::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. narrowedCalls: lists every call in start-time order | ported | `narrowed_calls::lists_every_call_in_start_time_order` |
| 2. narrows to the selected node | ported | `narrowed_calls::narrows_to_the_selected_node` |
| 3. keeps the rows of a test whose calls did not change | n/a | A `WeakMap`'s object identity, so unchanged DOM rows were not drawn again. `uniform_list` draws the rows in view each frame; the list builds its rows again only when what they show changes: `grace::http::tests::rows_are_built_again_only_when_what_they_show_changes` |
| 4. lists kept calls under the nodes of a refreshed tree | ported | `narrowed_calls::lists_kept_calls_under_the_nodes_of_a_refreshed_tree` |
| 5. groupCounts: counts every group, zeros included | ported | `group_counts_counts_every_group_zeros_included` |
| 6. shownCalls: with no group on shows all; with 4xx and none on shows only those | ported | `shown_calls_with_no_group_on_shows_all_with_4xx_and_none_on_shows_only_those` |
| 7. toggledGroups: turns a group on and off; All turns every group off | ported | `toggled_groups_turns_a_group_on_and_off_all_turns_every_group_off` |
| 8. totalCalls: adds up the calls of every test | ported | `total_calls_adds_up_the_calls_of_every_test` |
| 9. findCall: resolves the selected call and is null once it is gone | ported | `find_call_resolves_the_selected_call_and_is_none_once_it_is_gone` |

Added: `shown_count_is_the_length_of_the_shown_list` (#124), `a_call_is_named_by_status_method_url_and_test` (`callName`), `a_status_is_named_by_its_group` (`groupLabel`), `the_empty_text_says_why_the_list_is_empty` (`empty`).

#### `HttpCalls.test.tsx` (26) in `grace::http::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. adds the HTTP tab after History | ported | `adds_the_http_tab_after_history` (its name with its count, kept open through a run's start); the tab list's name is `bottom_panel::TABS` |
| 2. shows every group chip with its count | ported | `shows_every_group_chip_with_its_count` |
| 3. shows a row per call | ported | `shows_a_row_per_call`; each row an option: `grace-e2e/tests/http.rs::reading_the_calls_of_one_run_all` |
| 4. maps each group to the tint of its colour | ported | `maps_each_group_to_the_tint_of_its_colour` |
| 5. status badge: shows the code tinted by its group, with the group in a tooltip | ported | `the_badge_shows_the_code_tinted_by_its_group_with_the_group_in_a_tooltip` (right of the badge, on its row) |
| 6. status badge: is a rounded pill with the code in tabular mono, never white on a fill | by eye | Tailwind classes. `HttpCalls::row` draws it `rounded_full`, 20 px, in `MONO`, the status colour on its tint (`Tint::colours`, read in test 5) |
| 7. status badge: keeps a code outside 100-599 neutral, named in the tooltip | ported | `keeps_a_code_outside_100_599_neutral_named_in_the_tooltip` |
| 8. filters by the chosen groups | ported | `filters_by_the_chosen_groups` |
| 9. narrows to the tree selection | ported | `narrows_to_the_tree_selection` |
| 10. shows the three empty states | ported | `shows_the_three_empty_states` |
| 11. keeps a row and its focus when more calls arrive | ported | `keeps_a_row_and_its_focus_when_more_calls_arrive` |
| 12. shows the new calls in place when a test's calls are replaced | ported | `shows_the_new_calls_in_place_when_a_tests_calls_are_replaced` |
| 13. selects a call and marks it selected | ported | `selects_a_call_and_marks_it_selected`; the option's selected state in `grace-e2e/tests/http.rs` |
| 14. marks the pressed chips with a check and the selected fill | by eye | Tailwind classes. The pressed state: tests 2 and 8; `HttpCalls::chip` draws the check and the fill |
| 15. makes a row 28 px, with the selected one on the selected fill and the accent bar | ported | `makes_a_row_28_px` (the bar 2 px down the selected row's left side, on that row only); the fill by eye |
| 16. sets the method, URL and time in the mono and the rest in muted | by eye | Tailwind classes |
| 17. is a listbox of options named HTTP calls | ported | `grace-e2e/tests/http.rs::reading_the_calls_of_one_run_all` (headless windows build no accessibility tree) |
| 18. names a call by status, method, URL and test | ported | `grace_core::http_list::tests::added::a_call_is_named_by_status_method_url_and_test` |
| 19. keyboard: selects a call with the arrow keys and opens it | ported | `selects_a_call_with_the_arrow_keys_and_opens_it`; `tabindex` is DOM: one focus handle |
| 20. keyboard: moves focus up when the focused call is cleared by a new run | ported | `moves_focus_up_when_the_focused_call_is_cleared_by_a_new_run` |
| 21. long lists: scrolls to the selected call when the chips change | ported | `scrolls_to_the_selected_call_when_the_chips_change` |
| 22. long lists: shows the selected call after the chips take rows away above the view | ported | `shows_the_selected_call_after_the_chips_take_rows_away_above_the_view` |
| 23. long lists: keeps the calls in view still when an earlier call arrives | ported | `keeps_the_calls_in_view_still_when_an_earlier_call_arrives` (VL-19) |
| 24. cut text: shows a cut URL, test name and via whole in a tooltip | ported | `shows_a_cut_url_test_name_and_via_whole_in_a_tooltip` |
| 25. cut text: shows nothing for text that fits | ported | `shows_nothing_for_text_that_fits` |
| 26. shows an empty list as an EmptyState with the globe | ported | `shows_the_three_empty_states` (`ui::empty_state` with `Icon::Globe`, at its 24 px) |

Added: `a_click_on_a_call_shows_no_focus_ring` (E-virtual-08), `rows_are_built_again_only_when_what_they_show_changes`, `a_runs_calls_build_no_rows_while_the_tab_is_closed` (#124).

### Slice 4: a call's details and Copy as cURL

1. `grace::http`'s details pane from `CallDetails.tsx`: request and response, headers and bodies as selectable text (guide R4), a call with no response (HI-07), and a click on a test giving Details back (HI-10).
2. `Seams::copy`, with GPUI's clipboard, recorded by the `e2e` build; Copy as cURL through it (HI-08), on masked text (HI-09).
3. `http.rs` in `grace-e2e/tests`: E-http-01 whole and E-http-03.

### Slice 5: the History tab

Spec: [run history](../specs/2026-10-25-run-history.md).

1. `grace::history` from `History.tsx`: a row per run, a cancelled run without a report (RH-03), a damaged run file not hiding the others (RH-11), the empty state (PU-31).
2. Re-run and Re-run failed through `Grace`, with the confirmation before switching environment and filter (RH-06, RH-07), a test that no longer exists (RH-08), a failed dynamic row skipped (RH-05). Every action waits for the runner (RH-13).
3. Clear history: GPUI Kit's dialog with Electron Grace's button order, closed by Escape (RH-10, PU-32, E-look-17).
4. `history.rs` in `grace-e2e/tests`: E-history-01 to 13. E-look-16's tab half (what the History and HTTP tabs say before any run) is ticked here, HTTP being merged.

### Slice 6: export a run's report

Spec: [export reports](../specs/2026-11-03-export-reports.md).

1. `grace-core::report_zip` from `report-zip.ts`, `report-zip.test.ts` test for test: the zip holds the report (ER-08), any characters in the path (ER-09), nothing left when it cannot be written (ER-10). The spec forbids a new dependency for it; if Rust needs one, the PR states its size and the Redesigns table why.
2. `Seams::pick_save`: the Shell's save dialog on Windows, GPUI's `prompt_for_new_path` on Linux; the `e2e` build answers it.
3. The Export button in a run's row: it follows Report (ER-05), closing the dialog does nothing (ER-07). ER-06 (a malformed id) is `n/a` by type; `valid_run_id` keeps its test.
4. ER-11 and E-history-14 in `history.rs`.

### Slice 7: trends

Spec: [trends](../specs/2026-11-04-trends.md), [its plan](2026-11-04-trends.md).

1. `trendLabels.ts` in `grace-core::trends`, test for test.
2. `grace::trends`: the strip above the runs, drawn with plain elements. Columns oldest to newest, pass rate and time per test (TR-09 to TR-13), a column's tooltip and click (TR-14, TR-15).
3. It asks the runner's `trend` only while History shows and one question at a time (TR-17, TR-18), and again when a run ends (TR-16).
4. `trends.rs` in `grace-e2e/tests`.

### Slice 8: the flaky proof

Spec: [flaky tests](../specs/2026-10-26-flaky-tests.md).

1. `grace::flaky_proof` from `FlakyProof.tsx`, in Details: the runs that prove the mark, per environment, each opening its run's report through `run_report_path` and `open_path` (FL-04).
2. `flaky.rs` in `grace-e2e/tests`: the seven tests on real runs, the mark's name read through the accessibility tree (`Tree.test.tsx` 28), marks after a restart (FL-06) and after Clear history (FL-07).

### Slice 9: the data-driven view

Spec: [data-driven view](../specs/2026-11-03-data-driven-view.md).

1. `grace::examples` from `Examples.tsx`, in Details for an outline: the table before a run (DD-01), each row's result after (DD-02), a click opens a row (DD-03), the keyboard moves without opening and Enter opens (DD-04, `VirtualList.test.tsx` 9), a dynamic table (DD-05), tables with different columns (DD-06).
2. A big outline draws only the rows in view (DD-07).
3. `examples.rs` in `grace-e2e/tests`.

### Slice 10: the config banner and grouped failures

Spec: [group failures](../specs/2026-11-04-group-failures.md).

1. `failures.ts` in `grace-core::failures`, test for test: two failures share an error once Karate's `Line: n, Col: m` and a thrown string's `at n:m` are left out (GF-06, GF-07), groups by error, a test hidden by the tag filter left out (GF-10), none while a run goes on (GF-11).
2. Details: where a test sits in its group, Next and Previous wrapping round and keeping the focus, announced (GF-12 to GF-14); a folder's groups largest first with Go to first, which moves the focus to the test's heading (GF-15 to GF-17).
3. `grace::failures`' banner from `ConfigBanner.tsx`: the config file and its line, opened through slice 2's editor call, line 1 when the message has none (GF-18, GF-19); gone when the next run starts (GF-20); none for ordinary failures (GF-21).
4. `failures.rs` in `grace-e2e/tests` (GF-22).

### Slice 11: the exit check

1. The regressions of P3, each with its test:

   | Bug | What must hold | Test |
   |---|---|---|
   | #219 | Open in VS Code works for a share or a mapped drive | `grace_core::editor::tests::project_file::names_the_file_under_the_project_folder_as_it_was_opened_a_mapped_drive_keeps_its_letter`, `grace_core::editor::tests::vscode_url::keeps_a_share_paths_two_leading_slashes_as_vs_code_reads_them` |
   | #220 | A long wrapped error place is not centred | `grace::details::tests::open_in_vs_code::a_long_place_that_wraps_starts_at_the_left` |

2. PU-33: GPUI beside Electron for each panel and detail view of P3, light and dark.
3. VL-27's HTTP half: the HTTP tab at 10,000 tests, on the owner's machine with his go ([owner-machine](../process/owner-machine.md)).
4. By hand on the owner's machine: a real tab drag, also ended with Escape (RP-06), a link opening in a real VS Code, the Shell's save dialog, a paste of the copied cURL.
5. `npm run bench:gpui` on the GPUI build, reported not judged (guide R7.1).
6. The record in `docs/checks/<date>-gpui-p3-exit.md`; every row of the tables above ticked or handed on by its Proof text; #278's State edited. The owner closes #282.

## Redesigns

Changes from Electron Grace's design (guide R3.6). The rows below are decided by this plan; each slice adds its own in its PR.

| Module | Change from Electron Grace | Why | Parity kept by |
|---|---|---|---|
| `app` | The requests `main.ts` refused as malformed (OE-07, OT-05, ER-06, half of E-editor-03) cannot be made: the calls are typed methods of `Grace`. `validRequest` and `validPoint` of `editor.ts` are not ported (2 of `editor.test.ts`'s 10 tests, `n/a`); a place whose line is 0 is no place, where Electron Grace showed its link and then refused it with "Invalid request" | One process, no IPC (P2 Redesigns, `app`) | The runner's own refusals: a file outside the project (OE-06), `valid_run_id` |
| `seams` | `open_url`, `copy` and `pick_save` join the seams. `open_url` answers whether a program took the link: GPUI's `open_url` does not say, so the Windows build first asks the Shell (`AssocQueryString` for the scheme) as Electron's `app.getApplicationNameForProtocol` did, and Grace says "Open in VS Code needs VS Code installed" when none is registered | Electron's `shell.openExternal`, `navigator.clipboard` and `dialog.showSaveDialog` (guide R4); without the check Windows offers to find an app for the link | The `e2e` build's record of each link and its answer `{"vscode": false}`; `grace::seams::windows_url_tests`; the real ones by hand in slice 11 |
| `tree` | The row's menu is GPUI Kit's `PopupMenu`, which the Tree opens and draws itself on a right press and on the menu key or Shift+F10 at the focus row, where Electron Grace asked main for a native `Menu.popup`. Not Kit's `ContextMenu` wrapper: it opens only on a right press, with no way to open it from a key | Guide R4; no native menu in GPUI; the spec asks for the menu key (OT-01, R1.1). No patch: only the Kit's public API | `tree::tests::open_in_vs_code` (tests 86 to 91), `grace-e2e/tests/editor.rs` |
| `details` (link) | The error's place is `elements::link`, a GPUI Kit `Button` whose text is its own child, left-aligned and wrapping, where a browser's `<button>` centred a wrapped place (#220) and Kit's button content never wraps at all | Bug #220; a long place must not run past the pane | `details::tests::open_in_vs_code::a_long_place_that_wraps_starts_at_the_left` |
| `http`, `examples` | The lists are `uniform_list`s with the tree's focus rule, where Electron Grace shared `VirtualList.tsx` | P2 Redesigns, `tree` | VL-13, VL-19, VL-25, DD-04, DD-07 |
| `http` (cost) | The tab's count is `http_list::shown_count`, which counts the calls under the selection without listing them; the list is built only while the tab shows, and again only when `AppState::calls_changes`, the tree, the selection or the chips move. Electron Grace listed the narrowed calls on every change to count them, and kept each test's listed calls per calls array (`WeakMap`) so unchanged DOM rows were not drawn again | #124: `uniform_list` draws the rows in view each frame, so the saving is in not listing at all; a count needs no list | `grace::http::tests::a_runs_calls_build_no_rows_while_the_tab_is_closed`, `rows_are_built_again_only_when_what_they_show_changes`; `grace_core::http_list::tests::added::shown_count_is_the_length_of_the_shown_list` |
| `http` (options) | An option's description says what its row shows beyond its name: "attempt 2 of 3", "via tests/common/create-user.feature:8". Electron Grace's `aria-label` (status, method, URL, test) hid them from a screen reader. A chip's pressed state is AccessKit's `toggled`, where Electron Grace set `aria-pressed`: UI Automation reads it as a toggle that is on, AT-SPI as a toggle button that is pressed, and the e2e driver reads both as `checked` | A screen reader hears what a sighted person reads; the e2e driver reads them too | `grace-e2e/tests/http.rs::reading_the_calls_of_one_run_all`, `a_call_made_in_a_called_feature` |
| `http` (selected row) | The selected row's accent bar and the keyboard's ring are children drawn over the row, where Electron Grace drew an inset shadow and an outline | GPUI gives a box one border colour for its four sides: a left border would turn the row's separator to the accent, and a ring border would hide the bar and move the text | `grace::http::tests::makes_a_row_28_px` (the bar), `a_click_on_a_call_shows_no_focus_ring` (the ring) |

## Traps

- A cached view adds no accessibility nodes: cache only while `!window.is_a11y_active()`.
- On Linux the AT-SPI adapter drops a row's level and expanded state and a button's disabled state: an e2e test of a group's fold or a waiting History button checks what the control did.
- `bottom_panel` and `details` are touched by most slices: one slice at a time, each from the merged `develop`.
- A `Closes #282` in a commit would close the issue: write `Part of #282`.

## Exit check (from #282)

The parity rows of phase P3 ticked.

## Cost

- **Memory.** A run's HTTP calls are held once, masked, as P1's state already holds them; the views add rows only for what is in view. History and trends hold the 20 runs' entries.
- **Idle.** No timer, poll or animation. Trends and the HTTP rows are worked out only while their tab shows.
- **During a run.** The HTTP tab adds its count, and its rows only while it shows (#124).
- **Start.** Nothing: History's entries arrive with the project, as today.
- **Size.** No dependency is planned; slice 6 says so if the zip needs one.
