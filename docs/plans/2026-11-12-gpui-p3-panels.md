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

What slice 4 built: `grace_core::http_list::call_text` (what the pane reads: the status with its group, the test's place, the Request and Response sections that show only with something under them, the error of a call with no response, the cURL text), `Seams::copy` (GPUI's `write_to_clipboard`; the `e2e` build records `{"seam": "copy"}` and leaves the machine's clipboard alone), and `grace::http::call_details`, drawn by `Details` in place of the node while the HTTP tab is up and a call is selected. `Details` works the text out once for a call, until the calls or the tree change (`createMemo` in `CallDetails.tsx`). Headers, bodies, the error and the call's test and place are GPUI Kit's `SelectableText`. The tests, test for test:

#### `CallDetails.test.tsx` (16) in `grace::http::call_details::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. shows the selected call | ported | `shows_the_selected_call`; the text of each part: `grace_core::http_list::tests::call_text::*` |
| 2. indents an XML body and leaves out what the call lacks | ported | `indents_an_xml_body_and_leaves_out_what_the_call_lacks` |
| 3. shows a call with no response | ported | `shows_a_call_with_no_response` |
| 4. has no heading with nothing under it | ported | `has_no_heading_with_nothing_under_it` |
| 5. copies the call as cURL | ported | `copies_the_call_as_curl` (the seam got the text, "Copied" for 1999 ms and not for 2000) |
| 6. says when the copy failed | n/a | A rejected `navigator.clipboard.writeText`. GPUI's `write_to_clipboard` returns nothing: no write can be told to have failed, so no one using Grace can see "Copy failed" (Redesigns, `seams`) |
| 7. styles Copy as cURL as a small button with the copy icon, announced as it changes | ported | `styles_copy_as_curl_as_a_small_button_with_the_copy_icon_announced_as_it_changes` (the small button's height, the icon and words of both states); the live region: `call_details::view`, read by a screen reader only |
| 8. heads the call with its method and URL in the mono, then a tinted badge and the time | by eye | Tailwind classes. The heading is a level-2 heading naming method and URL; the badge's tint is `http::status_tint`'s |
| 9. tints the badge by group, with a code outside 100-599 neutral | ported | `grace::http::tests::maps_each_group_to_the_tint_of_its_colour` for the tint; `grace_core::http_list::tests::call_text::names_the_status_with_its_group_and_a_code_outside_100_599_alone` for the text |
| 10. sets Request and Response as 12 px section titles over sunken boxes | by eye | Tailwind classes |
| 11. shows the no-response error in a failed box titled with the alert icon | ported | `shows_a_call_with_no_response` (the box is drawn); the tint and icon by eye |
| 12. says the copy failed when the window has no clipboard | n/a | As 6 |
| 13. does not carry Copied over to another call | ported | `does_not_carry_copied_over_to_another_call` |
| 14. disables Copy as cURL without a URL | ported | `disables_copy_as_curl_without_a_url` (a press copies nothing); that the button reads as off is `grace-e2e/tests/http.rs`'s, on Windows (AT-SPI offers a disabled button no action) |
| 15. gives the pane back to the node | ported | `gives_the_pane_back_to_the_node` |
| 16. gives the pane back to the node when a new run empties its calls | ported | `gives_the_pane_back_to_the_node_when_a_new_run_empties_its_calls` |

Added: `shows_and_copies_a_masked_secret_never_the_secret` (HI-09: a secret through the runner's `Masker`, then the pane and the cURL text), `works_the_text_out_once_while_the_call_is_the_same`. `grace_core::http_list::tests::call_text` (7 tests) holds the rules the view only draws.

### Slice 5: the History tab

Spec: [run history](../specs/2026-10-25-run-history.md).

1. `grace::history` from `History.tsx`: a row per run, a cancelled run without a report (RH-03), a damaged run file not hiding the others (RH-11), the empty state (PU-31).
2. Re-run and Re-run failed through `Grace`, with the confirmation before switching environment and filter (RH-06, RH-07), a test that no longer exists (RH-08), a failed dynamic row skipped (RH-05). Every action waits for the runner (RH-13).
3. Clear history: GPUI Kit's dialog with Electron Grace's button order, closed by Escape (RH-10, PU-32, E-look-17).
4. `history.rs` in `grace-e2e/tests`: E-history-01 to 13. E-look-16's tab half (what the History and HTTP tabs say before any run) is ticked here, HTTP being merged.

What slice 5 built: `grace_core::history_list` (`History.tsx`'s rules: a row's text, the mark of its result, the path in the "What ran" tooltip, which of its buttons take a press, Clear history's wording), `Grace::{rerun, rerun_failed_of, open_run_report, clear_history}` over the runner's own `rerun`, `rerun_failed`, `run_report_path` and `clear_history` (which P1 ported with their tests), `grace::history` (the tab, drawn by `BottomPanel` only while it shows, and Clear history's button and dialog), and `grace_e2e::history::seed_runs` for a test that starts with a history. The plan's "confirmation before switching environment and filter" is the switch itself, as the spec's R5 has it and Electron Grace did: Re-run switches at once and the Env box and the filter bar show it; no dialog asks first. Export, and the two assertions of `History.test.tsx` tests 16 and 18 that are its half, came with slice 6; the trends strip is slice 7's. The tests, test for test:

#### `History.test.tsx` (30 tests) in `grace_core::history_list::tests` and `grace::history::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. durationLabel: shows Xs under a minute and Xm YYs from a minute | ported | `history_list::tests::duration_label::shows_xs_under_a_minute_and_xm_yys_from_a_minute` |
| 2. bottom panel tabs: starts on Console and switches to History (n) and back | ported | `bottom_panel::tests::tabs::the_history_tab_shows_its_runs_and_clear_history_beside_the_tab_list` (Clear history is in the header beside the tab list, not in it, and only on History), `name_history_and_http_with_their_count`, `a_click_shows_a_tab` |
| 3. starting a run keeps the History tab | ported | `grace_core::state::tests::run_started_keeps_the_bottom_tab_and_records_the_run_id` (P2) |
| 4. history list: lists runs with labels | ported | `history_list::tests::row_text::lists_runs_with_labels`; `grace::history::tests::lists_a_row_per_run_with_its_labels` (each column drawn) |
| 5. shows each run's thread count, or a dash | ported | `history_list::tests::row_text::shows_each_runs_thread_count_or_a_dash`; the header's words by eye |
| 6. What ran: puts where the node sits in a tooltip, not in a title | ported | `history_list::tests::selection_path::says_where_the_node_sits`; `grace::history::tests::puts_where_the_node_sits_in_a_tooltip_over_its_row` (over its row, as `side="over"`) |
| 7. says nothing for a run that has no path (All, Failed tests) | ported | `selection_path::says_nothing_for_a_run_that_has_no_path`; the same view test |
| 8. cuts a long label at 14 rem, inside a wrapper that can shrink | by eye | Tailwind classes. `WHAT` is a fixed 100 px column (136 before slice 6) that cuts its text (`truncate`); Electron Grace's grew to 224 px at most (Redesigns, `history` (rows)) |
| 9. shows a status mark before the words of the result, and the mark is hidden | ported | `result_mark::a_failure_beats_a_cancel`; the mark is `status_mark(.., decorative)`, which has no node: `grace-e2e/tests/history.rs::a_run_appears_in_the_history` (no image in the row) |
| 10. draws a tag by its shape: included with a check, excluded with a minus and a strike | ported | `row_text::lists_runs_with_labels` (the tags and their order), `grace::history::tests::draws_a_tag_by_its_shape_and_a_run_with_no_filter_says_a_dash`; the check, minus and strike are `tag_chip`'s, by eye |
| 11. right-aligns Threads and Duration in tabular figures | by eye | Tailwind classes; `text_right`, the mono face |
| 12. sticks muted sentence-case headers to the top, and gives rows room under them | ported | `grace::history::tests::makes_a_row_28_px_under_a_header_that_stays_above_the_rows`: the header is outside the scrolling list, so nothing sticks and no scroll padding is needed (Redesigns, `history`) |
| 13. sets the table cells in 12 px | by eye | Tailwind classes; `text_xs` |
| 14. uses no raw palette class, no dark: variant and no title | ported | `theme::tests::colours_come_only_from_the_theme`; a tooltip, never a title |
| 15. says No runs yet in an empty state with the history icon | ported | `grace::history::tests::says_no_runs_yet_in_an_empty_state_with_the_history_icon`; `history_list::tests::the_empty_text_is_electron_graces` |
| 16. history actions: Report is disabled without a report | ported | `history_list::tests::actions::report_is_off_without_a_report`; `grace::history::tests::report_and_re_run_failed_do_nothing_where_they_are_off`. Export's half, ported in slice 6: `grace_core::history_list::tests::actions::export_follows_report`, `grace::history::tests::export::follows_report_off_without_a_report_and_while_busy` |
| 17. Re-run failed is disabled with no failure | ported | `actions::re_run_failed_is_off_with_no_failure`; the same view test |
| 18. actions call the api with the id | ported | `grace::history::tests::the_buttons_of_a_row_call_the_runner_with_its_id` (Report opens that run's report through the seam). Export's half, ported in slice 6: `grace::history::tests::export::saves_the_zip_where_the_dialog_says_and_says_so` (the id goes to the runner, the dialog proposes the name, the zip is written where it says) |
| 19. all actions are disabled while busy | ported | `actions::every_action_is_off_while_the_runner_is_busy`, `locked_while_a_run_is_active_or_the_runner_is_busy`; `grace::history::tests::every_action_is_off_while_the_runner_is_busy_or_a_run_is_active` (the rows, Clear history and the toolbar's buttons; switching tabs stays possible) |
| 20. Clear history: confirms with the count, Cancel does nothing, Delete calls clearHistory | ported | `grace::history::tests::clear::confirms_with_the_count_cancel_does_nothing_delete_clears` |
| 21. is a dialog-style box: a 16 px title, Delete (danger) first, then Cancel (focused) | ported | `clear::puts_delete_first_then_cancel_which_has_the_focus` (Enter on the opened dialog closes it and runs nothing) and `cancel_has_the_focus_when_the_dialog_opens` (Kit's own Enter closes a dialog whatever holds the focus, so the focus is read from Cancel's handle); the 16 px title (`text_lg`) and the danger fill by eye; the frame is GPUI Kit's `Dialog` (Redesigns, `history`) |
| 22. words the confirmation for one run in the singular | ported | `history_list::tests::clear_prompt::counts_the_runs_and_words_one_in_the_singular`; `clear::words_one_run_in_the_singular` |
| 23. while it closes: keeps its title and buttons after Cancel and after Escape | ported | `clear::escape_closes_it_and_deletes_nothing`; GPUI Kit's dialog keeps its content through its exit, built from the closure that opened it (its own test `exiting_content_rejects_clicks_focus_and_text_input`) |
| 24. keeps the count it was opened with when main answers Delete with an empty list | ported | `clear::keeps_the_count_it_was_opened_with_and_counts_again_each_time` |
| 25. counts again each time it opens | ported | the same test |
| 26. disables the toolbar Report and Re-run failed once the history is empty | ported | `grace_core::state::tests::clearing_the_history_disables_report`, `toolbar::tests::shows_no_pill_while_there_is_nothing_to_re_run`; `grace-e2e/tests/history.rs::clear_history` |
| 27. is disabled with no runs | ported | `history_list::tests::actions::clear_history_needs_a_run_and_a_quiet_runner`; `grace::history::tests::says_no_runs_yet_in_an_empty_state_with_the_history_icon` (a press does nothing) |
| 28. toolbar Re-run failed: shows the newest run's failed count and calls rerunFailed with its id | ported | `toolbar::tests::re_run_failed_reruns_the_newest_run`, `shows_the_newest_runs_failed_count_on_re_run_failed` (P2) |
| 29. sits between Report and Cancel, before Run all | ported | `toolbar::tests::puts_the_controls_in_the_order_of_the_spec_run_all_last` (P2) |
| 30. is disabled with no history, with no failure in the newest run, and while busy | ported | `toolbar::tests::shows_no_pill_while_there_is_nothing_to_re_run`, `keeps_run_all_cancel_open_and_report_as_before` (P2); `grace::history::tests::every_action_is_off_while_the_runner_is_busy_or_a_run_is_active` |

Added: `works_the_rows_out_when_the_list_changes_not_on_a_redraw` (an idle History tab works nothing out), `history_list::tests::row_text::a_filter_with_no_tag_shows_no_chip`.

### Slice 6: export a run's report

Spec: [export reports](../specs/2026-11-03-export-reports.md).

1. `grace-core::report_zip` from `report-zip.ts`, `report-zip.test.ts` test for test: the zip holds the report (ER-08), any characters in the path (ER-09), nothing left when it cannot be written (ER-10). The spec forbids a new dependency for it; if Rust needs one, the PR states its size and the Redesigns table why.
2. `Seams::pick_save`: the Shell's save dialog on Windows, GPUI's `prompt_for_new_path` on Linux; the `e2e` build answers it.
3. The Export button in a run's row: it follows Report (ER-05), closing the dialog does nothing (ER-07). ER-06 (a malformed id) is `n/a` by type; `valid_run_id` keeps its test.
4. ER-11 and E-history-14 in `history.rs`.

What slice 6 built: `grace_core::report_zip` (`zip_report`, which writes the zip beside its destination and moves it onto it, so a failure leaves nothing and the file it would replace as it was; and `read_zip`, which reads one back, for the tests of `grace-core` and `grace-e2e`), `Backend::export_source` (the runner's own, with its test from P1), `Grace::export_report` (the runner for the folder and the name, the seam for the place, the zip written off the UI thread, then the notice "Report saved to <path>" or "The report could not be saved: <reason>"), `Seams::pick_save` and the Export button of a row, between Report and Re-run, on when Report is (`Actions::export`). The tests, test for test:

#### `report-zip.test.ts` (6) in `grace_core::report_zip::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. holds the report and leaves out run.json and karate-json | ported | `holds_the_report_and_leaves_out_run_json_and_karate_json` |
| 2. writes to the chosen path whatever its characters | ported | `writes_to_the_chosen_path_whatever_its_characters` (a Rust path is the file system's, not a command line's: the reason for the test is gone, the test stays) |
| 3. writes a zip whatever the name | ported | `writes_a_zip_whatever_the_name` |
| 4. replaces an existing file | ported | `replaces_an_existing_file` |
| 5. leaves nothing when the folder is gone | ported | `leaves_nothing_when_the_folder_is_gone` |
| 6. keeps the file it would replace when it fails | ported | `keeps_the_file_it_would_replace_when_it_fails` |

Added: `leaves_nothing_when_the_destination_cannot_be_written`, `a_secret_in_the_files_left_out_never_reaches_the_zip` (porting rule 2: `run.json` and the event file are where a secret could be, and neither is in the zip), `holds_a_big_report_whole`, `windows_own_tar_unpacks_it` (Windows' own unzipper reads the names as UTF-8). The view's: `grace::history::tests::export` (5 tests). No dependency is added to Grace.exe: `flate2` is in it already (Redesigns, `report_zip`).

### Slice 7: trends

Spec: [trends](../specs/2026-11-04-trends.md), [its plan](2026-11-04-trends.md).

1. `trendLabels.ts` in `grace-core::trends`, test for test.
2. `grace::trends`: the strip above the runs, drawn with plain elements. Columns oldest to newest, pass rate and time per test (TR-09 to TR-13), a column's tooltip and click (TR-14, TR-15).
3. It asks the runner's `trend` only while History shows and one question at a time (TR-17, TR-18), and again when a run ends (TR-16).
4. `trends.rs` in `grace-e2e/tests`.

What slice 7 built: `grace_core::trends`' words and columns (`pass_rate_label`, `trend_tip`, `trend_caption`, and `strip`, which works the columns' bar heights out: the pass bar split with its 2 px gap, the time bar scaled to the tallest with its 2 px least, the newest column's two labels and the top of the scale) and `scroll_to_show` (the least scroll that shows a row); `rows::format_millis` (moved from `grace::details`, which re-exports it); `Backend::trend` and `Grace::trend` over the runner's own `trend` (ported with its tests in P1), read off the UI thread; `grace::trends`, the strip, made by `History` when the tab shows and dropped when it does not, which asks one question at a time; and `trends.rs` in `grace-e2e/tests`. In `crates/grace/src/history.rs` the strip, the column titles and the runs are one scroll, and the titles are drawn again at the top once the strip has scrolled away (Redesigns, `history` (scroll)). The tests, test for test:

#### `Trends.test.tsx` (15 tests) in `grace::trends::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. asks for the whole tree and draws a column per point | ported | `asks_for_the_whole_tree_and_draws_a_column_per_point` (the columns' order and spacing; the group's name "Trend of All tests" is the e2e test's, `the_strip_follows_the_tree_selection`) |
| 2. follows the tree selection | ported | `follows_the_tree_selection` |
| 3. asks one at a time and draws only the newest answer | ported | `asks_one_at_a_time_and_draws_only_the_newest_answer` (the runner answers on the background executor, so three selections made before the answer are three redraws: the question for the first is out, the second is never asked) |
| 4. names the selection of the answer it draws | ported | `names_the_selection_of_the_answer_it_draws` |
| 5. drops the old columns when main gives no answer | ported | `drops_the_old_columns_when_main_gives_no_answer` (the runner's `trend` answers an empty list, never an error: Electron Grace's `catch` has nothing to catch) |
| 6. leaves with the history and asks nothing more | ported | `leaves_with_the_history_and_asks_nothing_more` |
| 7. asks again when the environment changes | ported | `asks_again_when_the_environment_changes` |
| 8. asks again when the history changes | ported | `asks_again_when_the_history_changes` |
| 9. shows no strip and asks nothing with no run | ported | `shows_no_strip_and_asks_nothing_with_no_run` |
| 10. asks nothing while History is hidden | ported | `asks_nothing_while_history_is_hidden` (the strip is also dropped with the tab, so nothing is held) |
| 11. splits the pass bar and scales the time bar | ported | `splits_the_pass_bar_and_scales_the_time_bar` (the heights drawn), `grace_core::trends::tests::labels::the_pass_bar_splits_passed_below_and_failed_above_with_a_gap` and `the_time_bars_scale_to_the_tallest_and_the_top_of_the_scale_is_said` (the numbers) |
| 12. labels the newest point | ported | `labels_the_newest_point`, `grace_core::trends::tests::labels::the_labels_are_the_newest_columns` |
| 13. names each column by its run | ported | `names_each_column_by_its_run` (the name is also the column's tooltip: `a_tooltip_names_the_run_a_column_stands_for`) |
| 14. says so when no stored run ran the selection | ported | `says_so_when_no_stored_run_ran_the_selection` |
| 15. a click scrolls to the run's row | ported | `a_click_shows_the_runs_row` (a click, then Enter on the focused column; the row ends in view of the list) |

Added: `a_redraw_asks_nothing` (an idle History tab asks nothing), `is_88_px_tall`, `the_caption_is_the_tree_s_own_label` (the strip shows the tree's label as it is, so a masked name stays masked, and no text of a stored run reaches it, only counts), `the_titles_stay_at_the_top_once_the_strip_has_scrolled_away`, `a_tooltip_names_the_run_a_column_stands_for`, `the_20_columns_are_inside_a_1008_px_window` (CI's window: a press is a click at the centre of the column). `trendLabels.test.ts`'s 7 tests are `grace_core::trends::tests::labels`'s first seven (`pass_rate_*`, `tip_*`, `caption_*`), and `scroll::*` covers `scrollIntoView({ block: "nearest" })`.

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
| `call_details` | The pane's text is worked out by `grace_core::http_list::call_text` into a `CallText`, which `Details` keeps until the calls or the tree change; the view draws it. Electron Grace's `CallDetails` formatted in the component, memoised per call | Logic in `grace-core`, drawing in `grace` (P1 porting rules); a run's events redraw the pane without indenting a body again | `grace_core::http_list::tests::call_text`, `grace::http::call_details::tests::added::works_the_text_out_once_while_the_call_is_the_same` |
| `call_details` | Headers, bodies, the error and the call's test and place are GPUI Kit's `SelectableText`, inside a `Label` node whose value is the whole text (the time and the "via" line are `Label`s too), in a box that scrolls both ways past 320 px with no wrapping, as the `<pre>` did; `CallText` holds each text as an `Arc<str>`, so a redraw copies none | Selectable, copyable text is only where Grace makes it (guide R4); a screen reader reads the box's value | `grace-e2e/tests/http.rs::reading_the_calls_of_one_run_all` (the indented body is one node's text) |
| `call_details` | Copy as cURL's button sits in an always-present `Status` live region that holds it, where Electron Grace set `aria-live` on the button itself. "Copied" is announced as the region's label changes; the "Copy failed" state is gone (CallDetails tests 6, 12) | GPUI's `div` has no live region (`a11y::live`); GPUI's `write_to_clipboard` cannot say it failed | `call_details::tests::copies_the_call_as_curl` (the button draws "Copied" and goes back after two seconds); the announcement itself by ear, in slice 11 |
| `seams` (copy) | `Seams::copy` puts text on the clipboard and answers nothing. The `e2e` build records `{"seam": "copy", "text": …}` in `e2e-seams.jsonl` and does not touch the clipboard of the machine running the tests, where Playwright read the real `clipboard.readText()` | Tests never reach outside (P3 porting rule 4); a run on the owner's machine must not clobber his clipboard | `grace-e2e/tests/http.rs` (`App::copies`) |
| `report_zip` | The zip is written by Rust, with `flate2` (deflate and the CRC), where Electron Grace ran Windows' `tar.exe` and read its output. The format is the same zip, names in UTF-8, `karate-summary.html` at its root, and Windows' own `tar.exe` unpacks it (`windows_own_tar_unpacks_it`). `grace-core` gains `flate2 = "=1.1.11"` as a direct dependency, and the lock and Grace.exe gain nothing: it is already there, as the PNG decoder's of GPUI's SVG renderer. A file is deflated in memory and there is no zip64, so a report of 65,535 files or a file of 4 GiB is refused with a message (a `shortcut:` comment) | The spec forbids a new dependency and `tar.exe` has no Linux twin: a Linux build, not released, has `bsdtar` only if installed. A program to start made the path of the zip the ANSI code page's problem (spec R3.3); Rust writes the file itself | `grace_core::report_zip::tests` |
| `seams` (save) | `Seams::pick_save(name)` is the Shell's `IFileSaveDialog` on Windows (title "Export report", "Zip archive" for `*.zip`, `.zip` added to a name without it, Downloads as the folder, `name` proposed) and GPUI's `prompt_for_new_path` on Linux, which has no filter by type and starts in `~/Downloads`. The `e2e` build records `{"seam": "save", "name": …}` and answers `{"save": "<path>" | null}`; `null` is closing the dialog | Electron's `dialog.showSaveDialog` (guide R4); GPUI's has no file types and no title | `grace::history::tests::export`, `grace-e2e/tests/history.rs::export_a_runs_report`; the Shell's own dialog by hand in slice 11 |
| `history` (Export) | The Export button's zip is written off the UI thread and the notice is the runner's own kind (`AppEvent::notice`), as Open in VS Code's. The "Invalid request" of a malformed id (ER-06) cannot be asked: `n/a` by type (`app` above). The Actions column is 332 px wide where it was 256, for a fourth button, and the columns beside it are cut to what they need so a run's two tags still fit a window of 1200 px (its table is 1160 px wide): "What ran" 100 px (was 136; its tooltip says the rest), Duration 56 (was 64), Threads 48 (was 56). Measured on the screenshots: the header of Threads is 45 px, that of Duration 47, the buttons 330, the widest listed result 198 of its 208 | Four buttons did not fit 256 px; no IPC; the Tags column cut a chip in two | `grace::history::tests::export`, `draws_the_tags_whole_at_1200_px` |
| `grace-e2e` | E-http-03 (a secret in a call) runs on Windows only, like the secret tests of `properties.rs`, whose Properties helpers moved to `grace_e2e::properties` to be shared. E-http-01's Enter and Space step is the headless `selects_a_call_with_the_arrow_keys_and_opens_it`: it needs the list focused with no call open, and the driver can only focus a list by clicking a row, which opens it. A tree row out of view, or one a click could miss while the tree scrolls, is selected with the tree's arrow keys (`select_with`), which select the row they move to. Copy as cURL has its own test with a short bottom panel (`copy_a_call_as_curl`): a UI Automation press clicks the centre of the button, and a button the details pane has scrolled out of its view is not hit | Linux has no secret storage (guide R5.3); the driver has no focus call | `grace::http::tests::selects_a_call_with_the_arrow_keys_and_opens_it` |
| `history` (rows) | The runs are a list named "Run history" of list items named "<when>, <what ran>, <result>", each cell a label and each button a button, where Electron Grace drew a `<table>` with `th` headers. The header is a row above the scrolling list, so it needs neither `sticky` nor the scroll padding that clears it, and the columns are fixed widths, "Tags" taking the rest and "What ran" cut at 100 px (a tooltip says the path), where Electron Grace's table grew to the cells and scrolled sideways | The e2e driver reads lists and their items on both platforms; AccessKit's table roles map unevenly through UI Automation and AT-SPI. A flex row gives the columns without a table layout. A screen reader reads a row as its when, what and result and the cells as labels, but not the column each belongs to (Env, Threads, Tags, Duration), which Electron Grace's `th` gave it | `grace::history::tests::makes_a_row_28_px_under_a_header_that_stays_above_the_rows`, `grace-e2e/tests/history.rs` |
| `history` (rows) | The rows are worked out when the listed runs change, with one clock read for the row's labels ("Today", "Yesterday"), and not on a redraw. Electron Grace's `For` made a row once per run, and so read `Date.now()` once too | An idle History tab works nothing out (ADR 0030) | `grace::history::tests::works_the_rows_out_when_the_list_changes_not_on_a_redraw` |
| `history` (dialog) | Clear history is GPUI Kit's `Dialog` with its own `DialogFooter`: Delete (danger) first, Cancel second and focused, where Kit's default order is Cancel then OK; the title says the count the dialog was opened with. Escape closes it by Kit's own handling, and it has no × button and no click-outside close (`close_button(false)`, `overlay_closable(false)`), as a `<dialog>` has none. Kit's `Dialog` has no way to name itself, so a screen reader reads its question as a label inside it, where Electron Grace's `<dialog>` had `aria-labelledby`. The title and buttons need no `<dialog>` workaround to survive the exit: Kit builds them from the closure that opened it | Guide R4 (no native dialog); Electron Grace's order is the person's habit (PU-32) | `grace::history::tests::clear::*`, `grace-e2e/tests/history.rs::clear_history_keeps_its_button_order_and_closes_on_escape` |
| `grace-e2e` (history) | `seed_runs` writes run folders with `grace_core::history::create_run_dir` and a JSON record, so `grace-e2e` depends on `grace-core` and `chrono`: both already in the workspace and the lock, no new crate. E-history-04 and E-history-05 read which tests the re-run selected from the new run's stored total ("of 2"), where Playwright watched the tree's pending marks with a `MutationObserver`: a mark is gone before a read of the tree sees it; the runner's `rerun_failed_runs_only_the_failed_tests_that_can_run` proves which. The checks of a disabled button and of the secret (E-history-12) run on Windows only | A snapshot read of the accessibility tree cannot catch a state that lasts a moment; Linux has no secret storage (guide R5.3) and its adapter drops the disabled state | `grace-e2e/tests/history.rs` |
| `trends` (ask) | The strip is made when the History tab shows and dropped when it does not (`History::close`); it asks the runner from its own render when what its answer depends on has changed (the selection, the ids of the runs listed, the tree, the environment), one question at a time. Its columns, caption and tooltips are worked out once when the answer comes, with one clock read then, where Electron Grace's `Trends.tsx` formatted them on each render and read `Date.now()` for each. The runner's `trend` answers an empty list for "no answer" and never fails, so the `catch` is gone | Nothing is asked or held for a closed tab (TR-17); an idle History tab works nothing out (ADR 0030); logic in `grace-core`, drawing in `grace` | `grace::trends::tests::asks_nothing_while_history_is_hidden`, `a_redraw_asks_nothing`, `asks_one_at_a_time_and_draws_only_the_newest_answer`; `grace_core::trends::tests::labels` |
| `history` (scroll) | The strip, the column titles and the rows are one scroll area ("history-scroll"), where slice 5 drew the titles above a list that scrolled. GPUI has no `position: sticky`, so once the strip has scrolled away the titles are drawn again over the top of the area, from the scroll's offset as of the last draw (a scroll redraws the view). The list is no longer the scroll: it holds its rows only, as the list "Run history". A click on a column moves the scroll so the run's row shows under the titles, with the least scroll | R1.4 asks for the strip to scroll with the table and the titles to stick as before; History.test.tsx 12 | `grace::trends::tests::the_titles_stay_at_the_top_once_the_strip_has_scrolled_away`, `a_click_shows_the_runs_row`; `grace::history::tests::makes_a_row_28_px_under_a_header_that_stays_above_the_rows` |
| `trends` (columns) | A column is a button made of a `div` with its own focus handle, kept by the run's id, and the focus ring of `elements::ring`; Enter and Space press it as a click does. Its tooltip is `grace::tooltip`'s, below the column, shown too while the keyboard stands on it. The sizes are in px, not Tailwind's `rem`: 12 px bars, 4 px of air, rows of 24 px. The numbers in the labels are in the sans face, not tabular (a fixed 144 px column keeps them from moving anything) | `Tooltip` has no "above"; a `Button` of GPUI Kit pads and sizes its content, which a 12 px bar cannot take | `grace::trends::tests::a_click_shows_the_runs_row`, `a_tooltip_names_the_run_a_column_stands_for`, `is_88_px_tall` |
| `grace-e2e` (trends) | `trends.rs` keeps its own few helpers (`wait_ready`, `show_history`, `open`) where `history.rs` has its copies: a test file cannot reach another's, and moving them would change `history.rs`, which draft PR #386 also edits. E-trends-02 reads where the oldest run's row sits against the tab bar and the status bar, since the tree has no viewport of its own | The tests are separate binaries | `grace-e2e/tests/trends.rs` |

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
