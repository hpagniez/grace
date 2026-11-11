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
| `tooltip` | A press on a tooltip closes it; Electron Grace closed it on the click that the press starts. A tooltip whose target moved is checked once per frame against where its target was laid out, not through scroll listeners on the boxes that hold it | GPUI has no `popover` element and no scroll events of a box: the tooltip is one deferred element of the window, and a frame is when GPUI knows where every target went. Closing on the press needs no tracking of the click across the press and release | `tooltip::tests` (the clicks, the scrolls, a target no longer drawn); the click on an `over` tooltip still reaches its target |
| `tree` | A click on a row's chevron or run button gives the focus to the tree, so no control in a row ever holds it and a key after the click is the tree's; Electron Grace left a key on a focused button to the button | One tab stop (VL-11): the buttons are out of the tab order, and the tree's keys work from the row the click made the focus row | `tree::tests::a_click_makes_the_row_focused`, `closing_a_feature_with_its_chevron_takes_the_focus_to_it` |
| `tree` | The tree draws a test's flaky mark, its feature's count and the mark's tooltip in P2, ahead of the proof they lead to (P3) | `grace-core` already sends the marks with the tree; the rows read them for their names | `tree::tests::a_flaky_test_has_its_mark_counted_above_and_explained_on_its_left`, `grace_core::state::tests::reduce`'s two flaky tests |
| `progress` | The segments ease to their width on a spring (GPUI's `with_spring`), not a 300 ms CSS transition; the hold Chromium needed beside a starting spinner is gone | A spring keeps its speed when a new result moves its end, so a fast run never restarts the ease; GPUI starts every animation in the frame it is asked for | `progress::tests::the_bar_r14_1` (LV-06 by its spring, the 2 px gaps) |
| `progress` | The stripe and the turning icon run only while they show, and the clock ticks once a second only during a run that is not paused | ADR 0030: nothing animates or ticks while idle; `setInterval` and CSS animations ran the same way | `progress::tests::the_clock` |
| `progress` | The bar has no `aria-busy`: while it waits it has no values, as Electron's did, which a screen reader reads as busy | GPUI 0.3.11 writes no busy state to AccessKit | `run_all_with_live_progress` (no value at `0 / 9`) |
| `progress` | The counts are one label each, "8 passed", with the number inside it, not a number node beside a word | A GPUI text child is not in the accessibility tree; the DOM read the span and the word as one line | `run_all_with_live_progress` |
| `a11y` | Reduced motion is the system's (`SPI_GETCLIENTAREAANIMATION` on Windows) given to GPUI's `set_reduce_motion`, read at start and each time the window becomes active; GPUI's animations then draw their end or start and ask for no frame | Electron read CSS's `prefers-reduced-motion`, which follows that setting; GPUI reads nothing itself, and a re-read on activation follows a change without polling | `progress::tests::the_bar_r14_1::reduced_motion_keeps_the_stripe_still` |
| `toolbar` | The Watch toggle and the step debugger's buttons wait for phase P4, which ports the runner's watch mode and debug runs; Env, Threads and Properties for PR 4 | The runner has no watch or debug run yet (P1 plan) | Their `Toolbar.test.tsx` tests are listed with P4 and PR 4 |
| `status_bar` | The update marker, the version and Settings wait for PR 5 (Settings) and phase P5 (the updater) | The Settings page and the updater are not ported yet | `StatusBar.test.tsx`'s update tests, with P5 |
| `tree` | The keyboard focus stays on the tree, one tab stop; the row it is on reports itself as the focused node to assistive technology (GPUI's `aria_active_descendant`). Electron Grace moved the DOM focus from row to row and kept the focus row first in the DOM so it never lost the focus | GPUI's `uniform_list` draws rows afresh each frame; a focus handle per drawn row would be dropped when its row scrolls away, which is what VL-15 forbids | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` (the focused tree item follows the keys) |
| `tree` | The rows are drawn by GPUI's `uniform_list`; `virtual.ts`'s `drawnRange` is gone, its `scrollToShow` is `uniform_list`'s `ScrollStrategy::Nearest`. `anchoredScrollTop`, `pageRows` and `survivor` are `grace-core::rows`'s | `uniform_list` draws only the rows in view and scrolls the least to a row; its scroll offset is read and set for the anchoring | `rows::tests` (`virtual.test.ts`'s tests but `drawnRange`'s); `only_the_rows_in_view_are_drawn` (VL-01, VL-14) |
| `tree` | A row is one clickable line: a click anywhere on it selects it and opens a closed feature. Electron Grace had a label button inside the row and selected only from it or the row's indent | One target per row reads the same to a screen reader (a tree item it can press) and saves a node per row; the chevron and the run button still stop the click | `features_start_closed_and_a_click_opens_one` (TO-05, TO-06), `the_chevron_opens_and_closes_a_feature` (TO-07) |
| `tree` | A row's `TreeRow` holds its node's path of child indexes, not the node; the rows are built again only when the state's tree count (`AppState::trees`, new), the filter or a turn changes | A row cannot borrow from the state it outlives; the count saves comparing trees on each event of a run | `tree::tests::statuses_do_not_build_the_rows_again` (VL-05, TO-17) |
| `tree` | A feature's title shows beside its file name, as its own text that gives way first, then the file name is cut; the cut file name's tooltip says the file name and the title, on hover and on the keyboard's focus row (`Tip::keyboard`). Each row sits in a wrapper as wide as the list | `Tree.tsx` cut one line holding both; GPUI's `uniform_list` lays each row out alone, as wide as its content, so without the wrapper no label was ever cut | `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus` (VL-21) |
| `tree` | The debug button and the breakpoint dot (P4), the row's "Open in VS Code" menu (P3) and the sub-folder buttons of an empty project (P4) come with their phases | Their runner calls are not ported yet | Their rows stay with P3 and P4 |
| `filter_bar` | The filter bar moves to its own PR, PR 3b, after the tree | The tree was a PR's worth on its own; the bar's folding of chips past one line (CF-*) needs its own measuring | TF-*, CF-* with PR 3b |
| `filter_bar` | The chips past the line are found from their places after each layout (`Div::on_children_prepainted`: a chip whose top is below the first chip's has folded), where Electron Grace asked an `IntersectionObserver` | GPUI has no observer; it hands a `div` its children's places after layout, and comparing them costs nothing while idle, when no frame is drawn. The chips wrap in the strip as in Electron Grace, so the spec's approach 2 (the layout wraps, Grace counts) holds | `filter_bar::tests::folding` (CF-01 to CF-06), `grace-e2e/tests/tags.rs::many_tags_keep_a_one_line_bar` (CF-07) |
| `filter_bar` | A folded chip keeps its place drawn hidden, without its accessible node (GPUI Kit's `RoleOverride::Presentational`) and out of the Tab order; the strip's clip gets 4 px of padding inside a 4 px negative margin, for the focus ring, where CSS had `overflow-clip-margin` | GPUI builds a node for every element with a role, hidden or not; it has no clip margin | `folds_the_chips_past_the_line_and_keeps_the_bar_one_line`; `many_tags_keep_a_one_line_bar` counts the chips a screen reader sees |
| `filter_bar` | The panel is a `deferred` element under the bar that closes on a press outside it, Escape and a second press on the fold chip; Tab from the fold chip goes into the panel, Tab past its last chip on after the fold chip, and Shift+Tab before its first back to the fold chip, by hand. Electron Grace had a native popover, which gave these | GPUI has no popover; a deferred element is drawn last, so its chips would come last in the Tab order | `the_panel_closes_on_a_click_outside_and_on_escape`, `opens_a_panel_of_every_tag_that_cycles_them_and_stays_open`; `many_tags_keep_a_one_line_bar` (Escape gives the focus back, Tab goes into the panel) |
| `filter_bar` | A chip is GPUI Kit's `Button` in a frame that draws its border and fill, and its next click is its tooltip and its accessible description, set through `a11y::aria` (new), which reaches GPUI's AccessKit setters on controls that are not a stateful `div` | GPUI Kit's custom button colours its own border and keeps no caller's border colour; its `Button` has no `aria_description` or `aria_expanded` of its own | `a_chip_says_its_state_in_its_name_and_its_next_click_in_its_hint`, `shows_the_next_click_in_a_tooltip` |
| `filter_bar` | Hide non-matching is GPUI Kit's `Checkbox`, named by its label | Grace's controls are GPUI Kit's | `toggles_hide_non_matching_through_the_runner`; `hide_non_matching_hides_the_other_rows` (TF-04) |
| `env_picker` | Env is GPUI Kit's `Input` with its own list of suggestions under it (a `deferred` list box, role `EditableComboBox`), filtered by the text it holds, any letter case, with "(none)" first; the arrow keys and Enter pick, a click picks, Escape puts the environment back | Electron Grace's box was an `<input list>` and Chromium drew the `<datalist>` popup; GPUI has no datalist | `grace::env_picker::tests` (the suggestions, the keys, the click); EV-01 |
| `threads_box` | Threads is GPUI Kit's `Input` read as a spin button: it takes only a number box's characters, Enter or leaving sends a changed value, and Up and Down step it by one between 1 and 32 and send at once. It has no `-` and `+` buttons | Electron Grace's was an `<input type=number>`, whose stepping and `change` event Chromium gave; GPUI Kit's `NumberInput` adds `-` and `+` buttons Electron Grace did not have | `grace::threads_box::tests`; TC-07, TC-10 |
| `env_picker`, `threads_box` | A press anywhere outside a box that has the focus commits what it holds before that press's click | GPUI tells a box it lost the focus at the next frame, after the click; the DOM's `blur` came before it, so Run all ran with the count just typed | `a_press_elsewhere_commits_before_its_click`, `a_press_elsewhere_commits_what_was_typed_before_its_click`; TC-02 |
| `app` | The runner calls run in the order they were made: a quick call (set threads, set filter, remove recent) holds the next until it ends, a call that starts work (open, refresh, run, set env) until it starts; Cancel is never held | Each call runs on GPUI's background executor, where two could swap; Electron's main process took IPC messages in order | `app::tests::a_setting_is_in_force_before_a_run_sent_after_it` |
| `theme`, `a11y` | The text caret blinks every 500 ms only for 5 s after the last key, and not at all with reduced motion (GPUI Kit's `cursor_blink`), as the Windows caret of Electron Grace's boxes | GPUI Kit blinks forever by default, a redraw twice a second while a box has the focus, which breaks the idle rule (ADR 0030) | `theme::tests::the_caret_stops_blinking_when_idle_and_never_blinks_with_reduced_motion` |
| `ui` | A field's hint, warning or error is a `Role::Label` under the box, read by a screen reader as the next node, not the box's `aria-describedby` | GPUI Kit's `Input` has no described-by setter | `says_when_the_environment_has_no_overlay_file`, `a_bad_name_says_why_and_sends_nothing`; EV-05 |
| `grace-e2e` | On Linux the e2e tests cannot read the text in the Env and Threads boxes: AccessKit gives a combo box or spin button no text through AT-SPI. They check what the box did (the runner's settings, the run's thread count) instead; on Windows UI Automation's Value pattern reads it | The pinned AccessKit adapter exposes AT-SPI's Text only for text runs, which GPUI Kit's `Input` does not give | `App::wait_for_text`; `env.rs`, `threads.rs` |
| `properties_draft` | The dialog's draft is `grace-core::properties_draft::Draft`: its rows, lists, chosen scope, errors, Replace, the import and the edit Save sends. The view holds only the boxes and what they show | Logic that needs no window lives in `grace-core` (guide, Architecture); `PropertiesDialog.tsx` kept it in component signals | `properties_draft::tests` (20 tests, the dialog's Vitest tests of rows, lists, secrets and Save) |
| `properties` | The dialog is GPUI Kit's, opened with `open_dialog`; its view asks to close by an event (`Close`) that the window answers with `close_dialog`. A view and its draft live as long as the dialog, and each open builds them from the runner's view. The "while it fades out" group of `PropertiesDialog.test.tsx` goes | Electron Grace kept the content through a 100 ms exit and guarded a reopen within it; GPUI Kit's dialog drops its view with it, so no old draft can show or outlive a close | `cancel_closes_and_sends_nothing`, `save_sends_the_draft_and_closes_once_applied`, `a_refused_save_keeps_the_dialog_and_its_draft` |
| `properties` | The scopes are buttons with the tab role in a tab list named Environments, the chosen one selected; a scope with errors adds an icon and "has errors" to its name. Each row is a list item named by its property's name, or "New property" | GPUI Kit has no list box with a selected item; a tab list tells a screen reader which list shows. A named row lets one find a row's controls by its name | `lists_scopes_and_rows_and_shows_the_chosen_ones`, `an_error_blocks_save`; `grace-e2e/tests/properties.rs` finds rows and scopes by name |
| `properties` | The import step's environment is GPUI Kit's `Select`, its kind a `Checkbox` | Grace's controls are GPUI Kit's; there is no native `<select>` | `import_shows_what_it_adds_and_puts_secret_variables_in_the_draft`, `an_invalid_prefix_blocks_the_import` |
| `seams` | The file to import comes through a `pick_file` seam: on Windows the Shell's `IFileOpenDialog` titled Import properties, with a Properties files (`*.properties`) and an All files filter; on Linux GPUI's dialog. The `e2e` build answers from its `file` key | The folder seam's reasons (P2 PR 1); Electron used `dialog.showOpenDialog` with the same filters | `back_changes_nothing_and_no_file_picked_shows_no_step`; EP-26 through the seam. The Shell dialog by hand, PR 6 |
| `ui` | `field_message_marked` takes another icon than its kind's (the secrets note's lock) | `FieldMessage.tsx`'s `icon` prop | `the_secrets_note_shows_with_a_secret_and_a_short_one_has_a_hint` |
| `grace-e2e` | The secret e2e tests (EV-07, EV-08, EV-11's properties, EP-26) run on Windows only; on Linux a test checks that no secret can be saved | Linux has no secret storage yet (guide R5.3); `OsProtector` is not available there | `without_secret_storage_no_secret_can_be_saved`; the Windows CI run |
| `grace-e2e` | On Linux the driver clicks a new row's name box before it types, and a disabled button is told by its lack of an action | Under Xvfb a window that only had actions pressed through the tree is not the active one, so it gives no focus; AT-SPI drops a button's disabled state but offers no action for it | `add_property` in `properties.rs`; `reserved_names_are_refused` |
| `console` | The console draws only the lines in view (GPUI's `list`, which measures wrapped lines) and paints a match by drawing its line's text runs again with a background (`StyledText::with_highlights`). Electron Grace drew every line and painted matches over them with the CSS highlight API, so no row redrew (R4.1); the "paints more matches than one call can take", "paints only the matches in and near the view" and "paints with static ranges" tests go | GPUI has no highlight API and no DOM; drawing only the lines in view makes the cost of painting the view's, never the console's 10,000 lines | `console::tests::find::paints_matches_and_the_current_one_brighter`, `errors::paints_the_current_errors_line` |
| `console` | A match's place is a byte range of its line, not a UTF-16 one; `grace-core::console_find` folds case char by char and keeps the offsets on the line as written | Rust strings are UTF-8, and `StyledText` takes byte ranges | `console_find::tests` (`consoleFind.test.ts`'s 9 tests) |
| `console` | The list follows the bottom with GPUI's `FollowMode::Tail`: a scroll up stops it, coming back to the bottom starts it again. Electron Grace measured whether the scroller was within 20 px of its bottom | The list keeps that state itself, so no scroll event is read | `console::tests::lines::follows_new_lines_unless_the_user_scrolled_up`, `keeps_following_once_the_store_holds_the_line_cap` |
| `console` | A step to a match or an error brings its line into view at the nearest edge (`scroll_to_reveal_item`); a match already in view does not move the view. Electron Grace centred the line | GPUI's list reveals an item; centring would need each line's height before it is drawn | `a_step_stops_following_the_bottom`; `grace-e2e/tests/console.rs` |
| `console` | GPUI Kit's box tells of a change on Enter too: a change that leaves the text as it was picks no match again | Without it, Enter would step and then lose the current match | `counts_matches_and_steps_with_enter_and_shift_enter_wrapping` |
| `console` | Ctrl+F, F8 and Shift+F8 are actions bound in the project view's key context, which the window's root takes while the project view shows, so they work when the focus is on nothing. A dialog is drawn by GPUI Kit's `Root` outside that context, and the Settings page replaces the project view, so neither takes them (R1, R3.1) | GPUI dispatches keys along the focused element's path; Electron Grace listened on the document and checked for an open dialog | `grace-e2e/tests/console.rs::ctrl_f_opens_the_bar_from_another_tab`, `step_through_a_failing_runs_errors` |
| `console` | The console's text cannot be selected or copied yet | GPUI Kit's text selection works in its `Root` on plain text, not yet on a `list` of styled lines | A gap, to close before the cut-over; PR 6 lists it in its check |
| `console` | The banner's versions come from `package.json` and `scripts/vendor.json` at build time (`build.rs`), where Vite's `__KK_VERSIONS__` gave them | The same pins, read once by the build | `console::tests::banner::names_the_pinned_versions`; `open_run_and_run_again` (CB-13) |
| `grace-e2e` | The e2e tests read the find count and the error count from their status nodes, not the painted ranges: only the lines in view are in the tree, and a paint is not in it at all. "A full console" (E-console-find-03) needs events sent into the app, which the driver cannot; the headless tests send them | GPUI builds the tree from what it draws | `console::tests::lines::keeps_the_banner_above_the_lines_past_the_line_cap`, `find::keeps_the_current_matchs_line_when_older_lines_drop_at_the_cap` |
| `bottom_panel` | The panel's height and its tabs' order are `panelHeight` and `panelTabs` in `settings.json`, where Electron Grace kept them in the renderer's `localStorage` | GPUI Grace has no `localStorage`; settings already live in that file | `settings::tests::panel_height`, `settings::tests::panel_tabs`; `grace-e2e/tests/panel.rs` (the restarts) |
| `bottom_panel` | The panel reads its height and the room it shares with the main area from the last layout (`canvas`), where Electron Grace asked `getBoundingClientRect` and a `ResizeObserver`. The main area keeps 120 px by its `min_h`, and flexbox shrinks the panel in a short window | GPUI tells an element its bounds after each layout, at no cost while idle | `bottom_panel::tests::splitter::*` |
| `bottom_panel` | A tab's drag is GPUI's `on_drag` and `on_drop`, with the drop side drawn as a 2 px accent border on the tab it is over. Electron Grace used the HTML drag and drop API and a `data-drop` attribute | GPUI has no HTML drag and drop | `bottom_panel::tests::tabs::a_dragged_tab_takes_the_place_of_the_tab_it_is_dropped_on_and_is_saved`, `a_drag_that_ends_on_itself_changes_and_saves_nothing` |
| `grace-e2e` | The splitter and tab drags, a window resized under the panel (UP-05, UP-07, E-panel-02, -05, -07, -10) and the drag half of E-panel-13 are the headless tests', with GPUI's simulated pointer and window. A real drag and a real resize are looked at by hand in PR 6 | The driver has no drag and does not resize the window | `bottom_panel::tests` |
| `bottom_panel` | Focus never scrolls the layout: GPUI does not scroll an element into view when it takes the focus, so a cut-off layout cannot lose its toolbar (E-panel-11) | Electron's DOM scrolled the focused element into view | Nothing to test: GPUI has no such scroll |
| `bottom_panel` | The History and HTTP tabs show no content yet, and HTTP no count; the Variables tab comes with P4 | Their panels are P3's and P4's (P2 plan) | Their rows stay with P3 and P4 |
| `console`, `bottom_panel` | The console's colours on the sunken token in both themes (CB-12, E-console-02) are looked at in PR 6's check by eye | The tree has no colours | PR 6 |
| `details` | A step's keyword and text show as one line in the mono face, cut as one, where Electron Grace set the keyword in the sans at 500 | `tooltip::cut` cuts one run of text, and the tooltip names the whole step, keyword included, as before | `details::tests::long_text` |
| `details` | Details reads the state where the `Grace` entity keeps it and builds only the part shown: no copy of the state, no memo, and a group's status is worked out once per frame it is drawn | GPUI draws the view again on a change; Solid needed a memo to share one `deriveStatus` walk between the badge and the counts | `details::tests::a_group` (the counts and the status from one walk each) |
| `details` | The open logs are kept by step index in the `Details` entity and cleared when the selection changes | Solid made the rows afresh for a new test, which closed them; GPUI keeps an entity's fields | `details::tests::a_test::a_new_selection_closes_the_logs` |
| `details` | The headless tests check the parts drawn, where they sit and the tokens they take, not class names; the names a screen reader reads are the e2e test's | GPUI has no classes, and a headless window builds no accessibility tree | `details::tests`, `grace-e2e/tests/details.rs` |
| `details` | E-details-01's first half, a test's steps and comments from its file before a run, waits for phase P4 with the file's steps | Details shows no steps before a run until P4 ports the file's steps | E-details-01's run half in `grace-e2e/tests/details.rs::failure_details` |
| `settings` | The Settings page lists only Appearance and opens on it; Updates joins with the updater in phase P5, and the page then opens on Updates again | The updater is P5's (migration guide) | `settings::tests::the_page` |
| `settings` | No IPC: the page sets the theme and saves it itself (`settings::choose`), and a theme is one of three enum values, so there is nothing for main to refuse (E-appearance-06). A malformed saved value still reads as Follow Windows | GPUI Grace has one process; the type admits no other theme | `grace_core::settings` tests `appearance` (PU-17), `settings::tests::appearance_r8` |
| `settings`, `window` | The project view's entities stay alive under the page, where Electron Grace kept its DOM mounted and hidden; the page is a new entity at each open | GPUI keeps an entity's state whether it is drawn or not | `settings::tests::appearance_r8::a_change_keeps_the_rest_of_the_views_state`, `grace-e2e/tests/appearance.rs` (Back finds the opened feature) |
| `settings` | Escape goes back through a key binding in the page's context on the window's root; a shown tooltip takes Escape first because `tooltip` intercepts the keystroke before any binding. E-appearance-05's tooltip half waits for P5's update button, the page's only tooltip | GPUI dispatches keys through contexts, not a document listener | `grace-e2e/tests/appearance.rs::escape_goes_back_from_the_settings_page`, `grace-e2e/tests/tooltips.rs::escape_in_a_dialog_closes_the_tooltip_first` |
| `settings` | A theme change with a dialog up (E-appearance-04) is a headless test: the e2e driver cannot flip Windows' setting, and a person cannot reach Settings past the modal dialog | The only path is the system's appearance under Follow Windows | `properties::tests::a_theme_change_keeps_the_draft_and_the_focus` |
| `grace-e2e` | The theme in force is read off the window's pixels on Linux (`import`), where Playwright read the CSS and `nativeTheme.themeSource`; on Windows the e2e tests check the radios and `settings.json` | AT-SPI and UI Automation expose no colours | `grace-e2e/tests/appearance.rs` |
| `grace-e2e`, `window` | The window tests (`window.rs`, E-window-01 to 12) read the bounds Grace saves on close, in GPUI's own coordinates, where Playwright read Electron's `getBounds`; the e2e build writes the main screen's work area to `e2e-screen.json`. The driver moves and sizes the window with `xdotool` on Linux and the Transform pattern on Windows. Maximizing and minimizing (E-window-02, 04, 12) run on Windows only. On Windows GPUI's bounds, the ones `settings.json` keeps, leave the window's frame out, where Electron's held it: Grace saves and places the same rectangle, so a window comes back where it was, and the tests compare what Grace saved before and after a restart | The driver cannot call the app's code, and Xvfb has no window manager to maximize a window | `grace-e2e/tests/window.rs`; two screens and a snapped window by hand, PR 6 |
| `tree` | The rows in view stay still when rows open above them (TO-18, VL-18, E-virtual-04) is a headless test of the tree's scroll offset on the fake runner, where Playwright scrolled a real run's list and read `scrollTop` | The accessibility tree gives no scroll offset; the anchoring is `rows::anchored_scroll_top`, unit-tested in `grace-core` | `tree::tests::a_feature_opening_above_the_view_does_not_move_it` |
| `toolbar`, `window` | Where the toolbar's name, chip and the status bar's path sit at 1200 px and at the 760 px minimum (TB-04, TB-05, E-open-04) is a headless test of the whole window, from the elements' bounds; the e2e test checks the buttons at a real narrow window (E-open-03) | The accessibility tree names a cut text whole, so it cannot tell a cut name from a whole one; GPUI's bounds can | `window::tests::the_name_shows_whole_at_the_default_size`, `a_long_path_at_the_minimum_width` |
| `elements` | A button shows the hand while it can be pressed and the not-allowed sign while it cannot (`Off::off`), a text box the text cursor (`elements::input`), a tree row and the Hide non-matching checkbox the hand. GPUI Kit's `Button` shows the arrow, and its `Input` the text cursor only inside an `InputGroup` | CSS gave Electron Grace these cursors (`buttons.ts`); GPUI Kit has other defaults | `elements::tests::a_button_shows_the_hand_and_a_disabled_one_the_not_allowed_sign`; by hand, PR 6 |
| `tree` | Focus surviving a scroll away (VL-15, E-virtual-03) is a headless test: the wheel is GPUI's simulated scroll event, and the focus row is the tree's, which keeps the keyboard focus as one tab stop while the row is out of view | The e2e driver has no wheel, and a row out of view is not drawn, so it is not in the accessibility tree | `grace::tree::tests::focus_survives_scrolling_away` |
| `theme`, scroll areas | The scrollbars are GPUI Kit's, drawn over the area's right edge, always shown (`ScrollbarMode::Always`), on a clear track with the thumb `control` at 60 %; the thumb widens under the pointer | CSS's thin scrollbar has no GPUI counterpart; GPUI Kit's fade after scrolling would need a timer (ADR 0030) | `grace::theme::tests::scrollbars_are_thin_themed_and_always_shown`; the look by hand, PR 6 (UP-16) |
| `env_picker`, `threads_box` | Whether the boxes hold "production" and 32 whole (E-open-05) is a headless measure of the text in the text box's font against the box's fixed width less GPUI Kit's padding, where Playwright compared the input's `scrollWidth`; the Threads box has no spin buttons to take room | The driver cannot read a text box's text on Linux, and the boxes' width does not follow the window | `grace::env_picker::tests::the_env_and_threads_boxes_show_their_longest_likely_values_whole` |
| `elements`, text boxes | A one-line text box is GPUI Kit's small `Input` given its height through GPUI's `Styled::h` (`elements::input(state, height)`): the toolbar's Env and Threads boxes are 28 px like its buttons, the dialogs' boxes their own height. They were 32 px, GPUI Kit's medium box, since its `Input::h` sets only a multi-line box's height | GPUI Kit sizes a one-line box from its `Size`, not from a height; Electron Grace's shared input class gave every box the controls' height | `grace::window::tests::the_toolbar_keeps_its_items_in_order_and_its_boxes_at_the_controls_height`; the e2e tests of the environment, threads, properties and console boxes |
| `toolbar`, `status_bar`, `window` | The toolbar is the banner landmark, the main area the main landmark and the status bar the content-info one, as Electron Grace's `<header>`, `<main>` and `<footer>` were. AT-SPI reads all three as landmarks, UI Automation as groups | GPUI has no HTML elements: a landmark is an AccessKit role set by hand | `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` (three landmarks, on Linux) |
| `status_bar` | The Settings button is the bar's full 24 px high (R10.3), where the first port drew it 20 px | No target under 24 px, as `h-6` gave it | `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` |
| `window` | A rem is 16 px, as Electron Grace's root: Grace's window sets it on each frame, after GPUI Kit's root has set it to its 13 px text size | GPUI Kit ties the rem to its text size, which shrank every size Grace gives in rems (`gap_2`, `p_4`, `text_xs`, ...) by a fifth; P2's check found the console's text and the home rows small (#321). Kit's own text keeps its 13 px, which it gives in px | P2's screenshots before and after; the e2e suite |
| `settings` | A theme's radio button is GPUI Kit's: a filled circle in the accent colour with a check mark, where Electron Grace's is Windows' own ring with a dot | GPUI has no native radio; Kit's is focusable, keyboard-driven and in the accessibility tree as a radio button with its state | `grace-e2e/tests/appearance.rs` (the radio buttons, their checked state, the click) |
| `spin`, `elements`, `progress`, `details`, `console` | A running mark turns in 20 steps a second, one turn a second as `animate-spin`, and the console's cursor blinks with two frames a second; each asks for its view's frame only when its step changes | GPUI's `with_animation` asks for a frame 60 times a second and each frame builds the view and paints the whole window, where Electron Grace's compositor turns a CSS spinner alone: a 2,000-test run cost a third more CPU than Electron Grace's (#320, VL-29) | `grace::spin::tests`; the marks still turn only while the run lasts and not with reduced motion (`elements` tests, ADR 0030) |
| `window`, `bottom_panel` | The tree, the details and the console are cached views: drawn again only when they are notified, each of them observing `Grace`. A cached view is laid out from its own root, so each pane's root fills its room (`size_full`). Not while an assistive technology is connected: the pinned GPUI builds the accessibility tree anew each frame and leaves a reused view's nodes out | A frame asked by one pane's mark, or the progress bar, no longer builds the other panes (#320). A screen reader still reads every pane, at the old cost | The e2e suite (every pane's updates, with the driver connected, so uncached); `the_cached_panes_fill_their_room`, the headless tests (cached) |
| `elements`, `bottom_panel`, `tree`, `theme` | Kit's focus ring: the ring is GPUI Kit's, 3 px wide just outside the control, in the focus colour at half strength. Kit's controls show it whenever they have the focus, after a click too; the panel's tabs and its splitter, Grace's own, draw the same ring through `elements::ring` while they have the focus from the keyboard. The tree keeps its 2 px ring inside the focus row in the full focus colour, now also when Tab comes back to it after a click. Electron Grace drew a 2 px outline in the full focus colour, 2 px out, on `:focus-visible` (R10.1) | Least customisation of the stack: Kit draws its ring in each of its controls, and Electron's would mean turning Kit's off and drawing Grace's own in every button, box and checkbox. The half-strength ring keeps 3 to 1 on every ground (3.0 to 3.4). A ring outside a tree row would be covered by the rows around it | `grace-e2e/tests/look.rs::every_control_shows_the_focus_ring_in_light_mode`, `…_in_dark_mode`; `grace::theme::tests::kits_half_strength_focus_ring_keeps_3_to_1` |
| `elements`, buttons | A button's label and icon are Grace's own content inside GPUI Kit's `Button` (`elements::button`, `icon_text_button`, `icon_button`), at `buttons.ts`'s sizes: 13 px text and a 16 px icon on a 28 px button, 12 px and 14 px on a 24 px one, 6 px apart; the status bar's Settings 12 px text and a 14 px icon, 6 px each side. Kit's `label` and `icon` are not used on Grace's buttons | GPUI Kit's `Button` sets its content's text size from its own `Size`, 14 px for Small and Medium, and its icon's size too (16, 14 or 12 px), over the sizes Grace gives. The owner's side-by-side look found the chips, Settings and the icons bigger (#321) | `elements::tests::a_buttons_text_and_icon_are_at_its_sizes`, `…no_button_icon_goes_through_gpui_kits_icon`, `filter_bar::tests::the_chips_and_the_checkbox_are_as_wide_as_their_text_at_electron_graces_sizes` |
| `elements`, checkboxes | A checkbox is Grace's own drawing on gpui-base's unstyled `Checkbox` (`elements::checkbox`), as a browser draws `<input type="checkbox" class="accent-accent">`: a 13 px box with 2 px corners on the middle of its label, 6 px from it (4 px in Run properties and the import, `gap-1`), a 1 px edge in the control colour on the surface, and once checked filled with the accent colour with a check mark in the colour on it; the keyboard's ring goes round the box. One turned off draws its box at half strength and keeps its label's colour, as Electron Grace's labels do | GPUI Kit's `Checkbox` fills a checked box in the primary colour, green in both themes where the accent is near black in the light one (#321), and comes in 12, 14 or 16 px. gpui-base's checkbox is the one Kit's own is built on: it brings the role, the checked state, the focus and the click, and leaves the look to the app, as for the radios, so nothing in the stack is patched | `elements::tests::a_checkbox_is_the_browsers_box_with_its_mark_only_when_checked`, `filter_bar::tests::the_chips_and_the_checkbox_are_as_wide_as_their_text_at_electron_graces_sizes`; the screenshots of the PR |
| `elements`, radios | A radio button is Grace's own drawing on gpui-base's unstyled `Radio` and `RadioGroup` (`elements::radio`), as a browser draws `<input type="radio" class="accent-accent">`: a 13 px circle 8 px from its text with a 1 px edge in the control colour, and once checked the edge and a 7 px dot in the accent colour; the keyboard's ring goes round the circle | GPUI Kit's `Radio` fills a checked circle in the primary colour, green in both themes where the accent is near black in the light one, and puts a check mark in it (#321). gpui-base's radio is the one Kit's own is built on: it brings the role, the checked state, the focus and the click, and leaves the look to the app, so nothing in the stack is patched | `settings::tests::appearance_r8::the_checked_radio_is_a_ring_with_a_dot_as_in_a_browser`; `grace-e2e/tests/appearance.rs` |
| `progress` | The fill's rounded ends are its segments' own: the first one rounds its left end, a full bar's last one its right end, and a full bar leaves out its empty segments and their gaps so that the last one ends with the track | GPUI clips a child to its parent's box, not to its rounded corners, so the fill's ends came out square during a run (#321); the browser's `overflow-hidden` clipped Electron Grace's to the track's | `progress::tests::the_bar_r14_1::a_full_bar_fills_the_track_from_end_to_end`; the screenshots of the PR |
| `console` | The console's ground rounds its own bottom corners, just inside the sheet's edge | The same clipping: the console's dark ground was drawn square over the sheet's rounded corners (#321); `overflow-clip` rounded it in Electron Grace | the screenshots of the PR |

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

Split in two. PR 3a, the tree: `grace-core::rows` (rows, names, `virtual.ts`'s rules), `grace::tree` on `uniform_list` with its keys, one tab stop, the rows that start open and the rows a run opens, its rows' statuses in their names; the e2e checks of the tree's rows in `run.rs` (WS-04, WS-08, `E-run-01`, `E-cancel-01`, "Run one outline row") and `tree.rs`. PR 3b, the filter bar: `FilterBar.tsx` with its chips, their fold and panel, Clear, the counter and Hide non-matching (TF-*, CF-*, `tags.spec.ts`), and the tree's filter scroll (VL-20) through it.

## PR 4: environments, properties, threads

`EnvPicker.tsx`, `PropertiesDialog.tsx`, `ImportStep.tsx` and `importPlan.ts`, `Threads.tsx`: EV-*, EP-*, TC-*; `env.spec.ts`, `threads.spec.ts`.

Split in two. PR 4a: the Env picker, the Threads box and `FieldMessage.tsx` (EV-01 to EV-05, EV-12, TC-*, TB-08). PR 4b: the properties dialog and its import (EV-06 to EV-11, EP-*).

Done in PR 4b: the draft and the import plan in `grace-core`, the dialog and its import step, the Properties… button, the `pick_file` seam, and the e2e tests of `env.spec.ts`'s properties (EV-06 to EV-11). The secret ones run on Windows only. `PropertiesDialog.test.tsx`'s look tests (spec R19) wait for PR 6's check by eye, and PD-05 for phase P4's pom environments.

## PR 5: details, console, bottom panel, appearance

`Details.tsx` (WS-06, its P2 part), `Console.tsx` with the banner, colours, find and error stepping (CB-*, CS-*), `BottomPanel.tsx`, `Splitter.tsx`, `panelHeight.ts` (UP-*), and the Appearance section of Settings (`appearance.spec.ts`). `console*.spec.ts`, `details.spec.ts`, `panel.spec.ts`.

Split in three. PR 5a, the console and the bottom panel: `grace-core::{banner, console_find, panel_height, panel_tabs}`, `grace::console` (the banner, the lines, find, errors) and `grace::bottom_panel` (the splitter, the tabs and their order), with `console.spec.ts`, `console-find.spec.ts` and `panel.spec.ts`. PR 5b, the details; PR 5c, the Settings page with its Appearance section and the status bar's Settings button.

## PR 6: the exit check

The P2 e2e tests left, the regressions of P2 (#144, #153, #170, #208, #223, #263), and `npm run bench:gpui` on the GPUI build against Electron Grace's limits, recorded in `docs/checks/`.

### The regressions of P2

| Bug | Test |
|---|---|
| #144 | `grace::progress::tests::the_bar_r14_1::the_passed_part_fills_during_a_big_run`: 20 updates of 200 results 100 ms apart, the passed part wider after each |
| #153 | `grace::tree::tests::the_focus_ring_starts_at_the_rows_indent` |
| #170 | `grace::console::tests::the_cursor_blinks_only_while_grace_works_and_never_with_reduced_motion`, `grace::theme::tests::the_caret_stops_blinking_when_idle_and_never_blinks_with_reduced_motion`; `idle.wakeups.minimised` in `npm run bench:gpui` |
| #208, #223 | `grace-e2e/tests/open.rs::running_one_test_of_a_maven_project_shows_its_result`; Karate's file names relative to its root in `grace_core::karate::command`'s tests |
| #263 | `grace::progress::tests::the_bar_r14_1::the_stripe_hides_the_last_runs_segments_and_they_show_again_after` (LV-04) |

### The component tests

The Vitest component files of P2, test for test. A test of the DOM, CSS or Solid with nothing to port is `n/a`, with why.

#### `Tooltip.test.tsx` (46) in `grace::tooltip::tests`

| Vitest test | Rust test |
|---|---|
| is in the DOM only while open: a manual popover inside the wrapper | `n/a`: no DOM and no `popover`; `opens_nothing_without_a_text` and every test's `shown` read the open tooltip |
| puts `class` on the wrapper | `n/a`: no CSS classes |
| opens nothing for an empty or missing text | `opens_nothing_without_a_text` |
| resets the text styles a row would pass down | `n/a`: the tooltip is a deferred element of the window's root and inherits nothing from a row |
| shows 500 ms after the pointer enters, not at 499 | `shows_after_the_pause_not_before` |
| does not show when the pointer leaves first, and hides when it leaves after | `does_not_show_when_the_pointer_leaves_first_and_hides_when_it_leaves_after` |
| keeps open while the pointer moves from the target onto the tooltip | `keeps_open_while_the_pointer_moves_from_the_target_onto_it` |
| shows at once on keyboard focus and hides on blur | `shows_at_once_on_keyboard_focus_and_once_per_focus` |
| ignores focus that is not `:focus-visible` | `does_not_show_on_a_focus_from_the_mouse` |
| shows on focus when the engine cannot tell `:focus-visible` | `n/a`: GPUI always knows the last input (`last_input_was_keyboard`) |
| shows even over a disabled button | `toolbar::tests::home_and_refresh_the_icon_only_buttons::refresh_shows_its_word_in_a_tooltip_also_while_disabled`, `toolbar::tests::a_narrow_window::gives_each_a_tooltip_with_its_label_while_the_label_is_hidden_also_when_disabled` |
| closes the other tooltip when a second one opens | `shows_one_at_a_time` |
| hides when a scroll moves its target | `hides_when_a_scroll_moves_its_target` |
| stays open when a box that does not hold its target scrolls | `stays_open_when_a_box_that_does_not_hold_its_target_scrolls` |
| hides on a press of its target | `closes_on_escape_and_on_a_press_of_its_target` |
| hides on a click inside it, not on the press that starts one | `a_click_on_it_closes_it_and_reaches_nothing_under_it`; closes on the press (Redesigns) |
| does not come back under a pointer that has not moved since the press | `closes_on_escape_and_on_a_press_of_its_target` |
| calls `showPopover` once the tooltip exists | `n/a`: no `popover` |
| stays closed, not thrown, when the engine refuses to show it | `n/a`: no `popover` |
| pinned: shows at once while true, and hides when it turns false | `shows_at_once_while_the_keyboard_stands_on_it` |
| pinned: shows from the start when it is true on mount | `shows_at_once_while_the_keyboard_stands_on_it` (the first frame that has it pinned) |
| pinned: shows nothing without a text, and shows once the text comes | `a_pinned_tooltip_shows_once_its_text_comes` (fixed here: the line is cut a frame after it is drawn) |
| pinned: opens with a ternary text | `n/a`: Solid's compiler |
| pinned: leaves hover as it was | `does_not_show_when_the_pointer_leaves_first_and_hides_when_it_leaves_after` (hover takes the same path) and `a_pointer_leaving_closes_a_pinned_tooltip` |
| pinned: hides at the cleanup of its owner | `the_keyboard_moving_to_a_row_without_a_tooltip_closes_the_last`, `closes_when_the_pointer_moves_elsewhere_though_its_target_is_gone` (fixed here: a target no longer drawn closes its tooltip) |
| placed: shows once and places it below the target when pinned turns true | `shows_at_once_while_the_keyboard_stands_on_it`, `goes_beside_its_target_on_the_side_asked_and_below_by_default` |
| placed: the same when pinned is true on mount | as the row above |
| placed: stays up and is placed again when a box that holds it scrolls, while pinned | `a_pinned_tooltip_follows_its_target_on_a_scroll_and_closes_once_it_is_out` (fixed here) |
| placed: hides, though pinned, once that scroll takes its target out of the box | as the row above |
| placed: still hides on that scroll when it is not pinned | `hides_when_a_scroll_moves_its_target` |
| placed: hides on Escape and on a pointer leave even while pinned | `shows_at_once_while_the_keyboard_stands_on_it`, `a_pointer_leaving_closes_a_pinned_tooltip` |
| clicks: do not reach a handler on an ancestor | `a_click_on_it_closes_it_and_reaches_nothing_under_it` |
| clicks: a right-click does, and its press closes it | `a_right_click_on_it_reaches_what_lies_under_and_closes_it` (fixed here) |
| clicks: on an `over` tooltip inside its target's box, reach the target | `a_click_on_an_over_tooltip_within_its_target_reaches_the_target` |
| clicks: on an `over` tooltip beyond its target's box, only close it | as the row above |
| clicks: on a tooltip beside its target, only close it | `a_click_on_it_closes_it_and_reaches_nothing_under_it` |
| side: below by default | `goes_beside_its_target_on_the_side_asked_and_below_by_default` |
| side: goes where it is asked and says where it went | as the row above; the side is the caller's, nothing reads it back |
| side: bridges the gap to its target | `keeps_open_while_the_pointer_crosses_the_gap` (fixed here) |
| Escape: closes only the tooltip, then the next reaches the dialog | `closes_on_escape_and_on_a_press_of_its_target` |
| Escape: leaves other keys alone while open | `closes_on_escape_and_on_a_press_of_its_target` |
| Escape: stops listening once the component is gone | `n/a`: the listener is a `Subscription` the window's `Tips` holds, dropped with it |
| describe: adds the tooltip's id to the target | `grace-e2e/tests/tags.rs::chips_list_the_projects_tags` reads a chip's hint as its description: AccessKit has a description, not an id to point at. The other `describe` users are the Updates section and marker, phase P5 |
| describe: leaves the target alone by default | `n/a`: no ids; a target has a description only where its view sets one |
| describe: keeps an id the target already points at | `n/a`: no ids |
| describe: gives two tooltips two ids | `n/a`: no ids |

#### `VirtualList.test.tsx` (28) in `grace::tree::tests`

The tree is GPUI's `uniform_list`, so the list's tests are the tree's. "later" names the phase whose view uses the list that way.

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. VirtualList: draws only the rows in view, plus the margin | ported | `grace-e2e/tests/tree.rs::only_the_rows_in_view_are_drawn` (of 300 rows only 5 to 99 are in the tree). The 35, `data-rows` and the 24000 px height are `drawnRange` and DOM: Redesigns "uniform_list". |
| 2. draws the rows at the bottom after a scroll, and keeps the focus row | ported | `grace-e2e/tests/tree.rs::only_the_rows_in_view_are_drawn` (End draws the last row, drops the first) and `grace::tree::tests::focus_survives_scrolling_away` (focus row kept while off screen). `style.top` is DOM. |
| 3. keeps focus on its row when the list jumps back to the top | ported | `grace::tree::tests::focus_survives_scrolling_away`: the focus stays on the tree and its row while the row is off screen (one handle, Redesigns "one tab stop"). Scrolling back is not driven (weak). |
| 4. keeps focus on its row when the list jumps onto rows below it | ported | Same test (weak, as 3). |
| 5. keeps focus on its row through 100 px steps both ways | ported | Same test (weak, as 3). |
| 6. has one tab stop: the selected row, else the first | ported | `grace::tree::tests::the_focus_row_is_the_selected_one_else_the_first`; one tab stop: `grace-e2e/tests/tree.rs::the_rows_buttons_are_out_of_the_tab_order_but_still_click` |
| 7. moves with Down, Up, Home, End, Page Down and Page Up, selecting as it goes | ported | `grace::tree::tests::home_end_and_pages` (End, Page Up, Page Down, Home: selected and in view); `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` (Down, Up, Home, End); tops: `grace_core::rows::tests::scroll_to_show::scrolls_the_least_to_bring_a_row_into_view`, `grace_core::rows::tests::page_rows::counts_whole_rows_in_view_at_least_1`. |
| 8. does nothing at the ends, and keeps the page still | ported | `grace::tree::tests::the_keys_do_nothing_at_the_ends` (the list keeps still). `preventDefault` is DOM |
| 9. with selectOnMove false, moves the focus without selecting, and selects with Enter or a click | later | P3: only `Examples.tsx` (the data-driven view, DD-*) sets `selectOnMove={false}`; the tree always selects as it moves. |
| 10. selects with Enter and Space | ported | `grace::tree::tests::enter_and_space_select_the_focus_row_and_open_nothing` |
| 11. brings a scrolled-away focus row back before Enter selects it (R4.2) | ported | `grace::tree::tests::a_scrolled_away_focus_row_comes_back_before_enter_or_right` |
| 12. leaves a key on a control inside a row to that control | ported | A row's buttons never keep the focus: a click on one gives it to the tree (`Tree::clicked`), so a key after it is the tree's (Redesigns, `tree`); `grace::tree::tests::closing_a_feature_with_its_chevron_takes_the_focus_to_it` |
| 13. passes other keys to onKey, and stops what it handled | n/a | `onKey` is the Solid component's prop. The tree's own keys are `Tree::row_key` (Right, Left, Ctrl+Enter), proved by `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` and `ctrl_enter_runs_the_focused_row`. GPUI has no `defaultPrevented`; a handled key stops propagation. |
| 14. brings a scrolled-away focus row back before moving | ported | `grace::tree::tests::focus_survives_scrolling_away` (Down after the wheel: the next row, in view); offsets: `grace_core::rows::tests::scroll_to_show::scrolls_the_least_to_bring_a_row_into_view`. |
| 15. takes focus to the row clicked, even through a button in it | ported | `grace::tree::tests::closing_a_feature_with_its_chevron_takes_the_focus_to_it` (a click on the chevron makes its row the focus row and acts once). The Run button goes through the same `Tree::clicked`. |
| 16. selects a row clicked on itself, as a screen reader's activate does, not through a click inside it | ported | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` (a press on the row selects it); `grace::tree::tests::closing_a_feature_with_its_chevron_takes_the_focus_to_it` (a click on a scenario row selects it). A click anywhere on the row selects, so the "not through a click inside it" half goes: Redesigns "one clickable line". |
| 17. keeps a press on a row from taking focus, so its click arrives | ported | `grace::tree::tests::a_click_makes_the_row_focused` (one tab stop, no ring after a click, the ring once a key moves). `mousedown` default and `focusVisible` are DOM. |
| 18. keeps the rows in view still when rows are added above | ported | `grace_core::rows::tests::anchored_scroll_top::keeps_the_first_row_in_view_still_when_rows_are_added_above`; `grace::tree::tests::a_feature_opening_above_the_view_does_not_move_it`. |
| 19. moves focus to the nearest row above when the focus row leaves | ported | `grace_core::rows::tests::survivor::is_the_nearest_row_before_the_key_that_is_still_there`, `survivor::falls_back_to_the_first_row_then_to_none`; `grace::tree::tests::closing_the_feature_of_the_focus_row_hands_the_focus_to_the_nearest_row_above`. |
| 20. falls back to the selected row when an unfocused active row leaves | ported | `grace::tree::tests::an_active_row_that_leaves_falls_back_to_the_selected_one` |
| 21. falls back to the nearest row above when an unfocused active row leaves with the selected one | ported | `grace::tree::tests::closing_the_feature_of_the_focus_row_hands_the_focus_to_the_nearest_row_above` (the other `Tree::build` branch: selected not kept) and the `survivor` tests above. |
| 22. draws rows after the list shrinks below the scroll position | ported | `grace::tree::tests::rows_show_after_the_list_shrinks_below_the_scroll` |
| 23. reveals a row by key | later | P3: `reveal(key)` is the HTTP tab's (`HttpCalls.tsx`, VL-13, VL-19, VL-25); the tree reveals its selected row (VL-20, row 24). The scroll rule itself is `grace_core::rows::tests::scroll_to_show`. |
| 24. reveals the selected row after keeping the rows in view still, when revealSelectedOn changes | ported | `grace::tree::tests::only_a_new_filter_scrolls_to_the_selected_row`; `grace-e2e/tests/tags.rs::a_filter_change_scrolls_to_the_selected_row`; `grace_core::rows::tests::anchored_scroll_top` |
| 25. moves to a row by key from the handle, as an arrow key does | ported | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows` (Right goes to the first child, Left to the parent, each focused and selected, through `Tree::move_to_key`). The handle is the Solid API. |
| 26. tells a row when it has keyboard focus | ported | `grace::tree::tests::a_click_makes_the_row_focused` (the ring only after a key); `grace::tree::tests::the_focus_ring_starts_at_the_rows_indent`. |
| 27. keeps a drawn row's element through a scroll, and updates its attributes | n/a | DOM node identity: `uniform_list` builds the rows afresh each frame (Redesigns "uniform_list"). The selected state updating is read in `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one`. |
| 28. shared: is made once per drawn row for its attributes and content, not again as the focus moves | n/a | Solid memo scope (`shared`). `Tree::row` works out the status and the chip once per drawn row per frame; the cost is the bench's (VL-26 to VL-29, PR 6). |

#### `Tree.test.tsx` (92: 86 tests and two `it.each` of 3)

##### main screen

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. shows a failed scenario's status on its feature | ported | `grace-e2e/tests/run.rs::run_all_with_live_progress` (a scenario failed, its feature "failed"); `grace_core::model::status::tests::derive_status::failed_wins_over_passed_and_running`, `derive_status::not_run_when_any_child_is_not_run_and_none_failed_or_running`; `grace_core::state::tests::reduce::run_started_sets_selection_to_pending_keeps_others_clears_console_sets_total` (pending); the tags in the row's name: `grace_core::rows::tests::row_name_joins_the_label_the_status_word_in_lower_case_the_flaky_words_and_the_tags`. `data-*` is DOM. |
| 2. runs a row from its run button | ported | `grace-e2e/tests/run.rs::run_one_outline_row` (presses "Run add 1 and 2", then "Run add 2 and 3"). |
| 3. shows each row's status by shape, named for a screen reader (R4.4) | ported | `grace::elements::tests::marks_each_status_as_electron_grace_does`, `grace::elements::tests::tells_statuses_apart_by_shape_or_name`; `grace-e2e/tests/elements.rs::names_each_status_mark` (PU-08); the rows' own statuses in `grace-e2e/tests/run.rs::run_all_with_live_progress`. |
| 4. swaps the chevron when a folder opens and closes, and icons folders and features | ported | `grace::tree::tests::a_folder_and_a_feature_have_their_icon_and_a_test_none`; the chevron's names and state `grace-e2e/tests/tree.rs::the_chevron_opens_and_closes_a_feature`; no chevron on a test `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` (fixed here: an invisible one was read as "Expand") |
| 5. marks the selected row with its background and the accent bar, rings focus inside the row | by eye | The ring `grace::tree::tests::the_focus_ring_starts_at_the_rows_indent`, the selected state `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one`; the fill and the accent bar in PR 6d's look check |
| 6. shows the run button muted, green when its row is hovered, focused or selected | ported | `grace::tree::tests::the_run_button_is_green_on_a_hovered_focused_or_selected_row` (fixed here: green on hover and on the focus row too); the colour itself in PR 6d's look check |
| 7. label tooltip: shows a cut label whole in a tooltip, not as a title | ported | `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus`; the pointer path `grace::tooltip::tests::shows_cut_text_whole_only_when_it_is_cut`. `title` and `truncate` are DOM and CSS. |
| 8. label tooltip: shows nothing for a label that fits (R9.4) | ported | The same two tests ("b.feature is whole" gives no tooltip; a fitting line gives none on hover). |
| 9. label tooltip: keeps a row's tooltips on its row: the mark's right, the label's over it, the run's left | ported | `grace::tree::tests::a_rows_tooltips_stay_on_its_row` |
| 10. gives tags the 20 px tag chip, so they clear the rows above and below | ported | `grace::ui::tests::a_tag_chip_is_20_px_high_inside_a_tree_row` |
| 11. gives every run button an accessible name | ported | Pressed by name in `grace-e2e/tests/run.rs::run_one_outline_row`, `grace-e2e/tests/tags.rs::a_feature_run_is_filtered` ("Run basic.feature"), `grace-e2e/tests/tags.rs::run_all_runs_only_matching_tests`. |
| 12. labels a feature with its file name, then its title | ported | `grace_core::rows::tests::a_label_is_a_features_file_name_and_an_unnamed_scenarios_line`; `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus` (tooltip "file · title"); "Run basic.feature" in `grace-e2e/tests/tags.rs::a_feature_run_is_filtered`. Order of the two texts is by construction (weak). Redesigns "title beside the file name". |
| 13. selects a node by clicking its label | ported | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` (row selected); `grace::tree::tests::closing_a_feature_with_its_chevron_takes_the_focus_to_it` (a click selects a scenario); Details' place `grace::details::tests::a_test::heads_it_with_its_name_a_tinted_badge_and_the_meta_line`. A click anywhere: Redesigns "one clickable line". |
| 14. disables run buttons while a run is active and enables Cancel | ported | `grace::tree::tests::the_run_buttons_run_nothing_during_a_run_or_while_busy`; `grace::toolbar::tests::the_actions::keeps_run_all_cancel_open_and_report_as_before` |
| 15. shows discovery as busy: actions off, Cancel on, Finding tests… | ported | `grace::tree::tests::the_run_buttons_run_nothing_during_a_run_or_while_busy`; `grace::progress::tests::after_a_run_r14_2::shows_finding_tests_with_the_turning_icon_while_the_runner_discovers` |
| 16. keeps the stripe until the first result, then reports progress | ported | `grace::progress::tests::the_bar_r14_1::the_stripe_stays_until_the_first_result_and_a_cancel_stops_it`; `grace-e2e/tests/run.rs::run_all_with_live_progress` (no value at "0 / 9", value 9 at the end). |
| 17. drops the stripe when a run is cancelled before any result | ported | The same `the_stripe_stays_until_the_first_result_and_a_cancel_stops_it` (ends on a cancel). |
| 18. fills smoothly and keeps the stripe still under reduced motion | ported | `grace::progress::tests::the_bar_r14_1::the_passed_part_fills_during_a_big_run`, `the_bar_r14_1::reduced_motion_keeps_the_stripe_still`, `the_bar_r14_1::is_6_px_with_2_px_gaps_between_passed_failed_and_cancelled` (three segments). `transition-[width]` and `animate-slide` are CSS: Redesigns `progress` (spring) and `a11y` (reduced motion). |
| 19. draws the stripe over the last run's results | ported | `grace::progress::tests::the_bar_r14_1::the_stripe_hides_the_last_runs_segments_and_they_show_again_after` (LV-04, #263). Paint order of DOM children is NA. |
| 20. labels an unnamed scenario by its line | ported | `grace_core::rows::tests::a_label_is_a_features_file_name_and_an_unnamed_scenarios_line` ("Scenario at line N"); the row, its Run button and Details' heading all take `label_of` and `node_name`. |
| 21. disables Run all without a tree or without leaves, and says why | ported | `grace::toolbar::tests::the_actions::run_all_is_disabled_with_nothing_to_run` (WS-10); `grace-e2e/tests/open.rs::finding_tests_for_the_first_time` ("Finding tests…" in the tree); `grace::progress::tests::after_a_run_r14_2::says_which_build_tool_lists_the_libraries_then_finding_tests` (the tree uses `progress::finding_text`); `grace-e2e/tests/run.rs::nothing_to_run` ("No feature files found"); the layout chip `grace::toolbar::tests::the_project::shows_the_folder_name_and_the_layout_and_leaves_the_path_to_the_status_bar_r1_1`; the error `grace-e2e/tests/open.rs::syntax_error_is_reported_not_hidden` with `grace::ui::tests::an_error_is_an_alert_tinted_failed`. The hint sentence and the spinner class are not asserted (weak). |
| 22. offers the folders below an empty project | later | P4: sub-folders (`openSubfolder`); Redesigns "phases". |
| 23. shows steps once they arrive, with their log on demand | ported | `grace::details::tests::a_test::says_steps_come_when_the_feature_finishes_with_the_spinner_while_a_run_is_on`, `a_test::shows_each_step_with_a_mark_named_by_its_status`, `a_test::toggles_a_steps_log_with_its_button`; `grace-e2e/tests/details.rs::failure_details` (a real run's steps and a log). |
| 24. shows progress, outcome, elapsed and the notice | ported | `grace::progress::tests::after_a_run_r14_2::shows_done_of_total_and_each_count_behind_its_mark_and_word`, `after_a_run_r14_2::names_the_outcome_with_its_icon`, `after_a_run_r14_2::shows_the_elapsed_time_on_two_digits_each`, `grace::progress::tests::the_clock::runs_only_during_a_run_and_shows_the_summarys_time_after`; Report on after a run `grace::toolbar::tests::the_actions::keeps_run_all_cancel_open_and_report_as_before`; the notice `grace_core::state::tests::reduce::notice_sets_the_notice_a_warning_unless_main_says_it_reports_a_success`, `grace-e2e/tests/tags.rs::a_tag_removed_before_a_run_does_not_widen_it` ("Nothing to run"). |
| 25. offers Build in a notice that asks for it, disabled while busy | later | P4: Build (there is no Build button yet; state already carries `NoticeView.build`). |

##### flaky marks

Phase: the feature map puts "Flaky marks and proof" (`FlakyProof.tsx`, FL-*, `flaky.spec.ts`) in P3. `crates/grace/src/tree.rs` already draws the badge, count and tooltip, and `grace_core::rows` already builds the chip words, so rows 26, 27, 30 can be tested now; see the note after the table.

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 26. flaky marks: marks flaky tests and counts them above | ported | `grace::tree::tests::a_flaky_test_has_its_mark_counted_above_and_explained_on_its_left`; `grace_core::rows::tests::the_flaky_chip_counts_the_flaky_tests_under_a_row` (the tree draws the marks ahead of P3's proof) |
| 27. shows the flaky explanation in a tooltip, left of the mark | ported | `grace::tree::tests::a_flaky_test_has_its_mark_counted_above_and_explained_on_its_left` |
| 28. names the mark for a screen reader | later | P3 (E-flaky-*): the name `FLAKY_TITLE` (role Image) is read through the accessibility tree, which needs flaky data from real runs; headless windows build no tree. |
| 29. counts a flaky test the tag filter hides | ported | `grace_core::rows::tests::the_flaky_chip_counts_the_flaky_tests_under_a_row`: `flaky_chip` takes no filter and counts every leaf of the node, hidden ones too (by construction). |
| 30. keeps the mark of a dimmed test | ported | `grace::tree::tests::a_flaky_test_has_its_mark_counted_above_and_explained_on_its_left` |
| 31. clears the marks for a new project | ported | `grace_core::state::tests::reduce::project_event_for_another_folder_clears_the_flaky_marks` |
| 32. drops the mark of a key that now holds another test, keeps the rest | ported | `grace_core::state::tests::reduce::project_event_drops_the_flaky_mark_of_a_key_that_now_names_another_test` |

##### virtual tree

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 33. folders open, features closed: starts with folders open and features closed | ported | `grace::tree::tests::a_project_opens_with_its_features_closed`; `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` (TO-01); `grace_core::rows::tests::a_feature_starts_closed_and_every_other_row_open_and_a_turn_flips_it`. |
| 34. selects and opens a closed feature when its name is clicked | ported | `grace-e2e/tests/tree.rs::features_start_closed_and_a_click_opens_one` (TO-05). Redesigns "one clickable line". |
| 35. keeps an open feature open when its name is clicked again | ported | The same test, its second press (TO-06). |
| 36. still closes a feature with its chevron | ported | `grace-e2e/tests/tree.rs::the_chevron_opens_and_closes_a_feature` (TO-07). |
| 37. opens no feature with the keys that select; Right opens it, its outline open too | ported | `grace::tree::tests::enter_and_space_select_the_focus_row_and_open_nothing`, `an_outline_opens_with_its_feature` |
| 38. keeps what was opened and closed through a refresh; a new feature starts closed | ported | `grace::tree::tests::a_refresh_keeps_what_was_opened_and_closed_and_a_new_feature_starts_closed`; `grace_core::rows::tests::a_feature_starts_closed_and_every_other_row_open_and_a_turn_flips_it`. |
| 39. starts another project with its features closed again | ported | `grace::tree::tests::another_project_starts_closed_again`. |
| 40. a run opens features: opens the feature as a run of the feature starts | ported | `grace::tree::tests::a_run_of_one_feature_opens_it_as_it_starts`. |
| 41. ... as a debug run of the feature starts | later | P4: debug runs. `Tree::follow_run` reads no debug flag, so it is row 40's path once they exist. |
| 42. ... as History's Re-run of a run of one of its tests starts | ported | `grace_core::rows::tests::feature_of::reads_the_feature_of_a_feature_scenario_example_row_or_outline_key` (a scenario selection gives its feature), which `Tree::follow_run` feeds as row 40 does. The History tab is P3; the rule is the tree's. |
| 43. opens nothing as Run all or a rerun of the failed tests starts | ported | `grace::tree::tests::run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it` ("the root opens nothing"). |
| 44. opens a feature when one of its tests fails, and none where they pass | ported | The same test (features with failures open; the one without stays closed). |
| 45. never reopens a feature closed during the run, where it opened on a failure | ported | The same test (TO-12: a failure after the close leaves it closed). |
| 46. ... where it was open before the run | ported | `grace::tree::tests::a_feature_the_person_closed_during_a_run_stays_closed` |
| 47. ... where its name was clicked during the run | ported | `grace::tree::tests::a_feature_the_person_closed_during_a_run_stays_closed` |
| 48. opens nothing for a failure of an earlier run | ported | The second run of `grace::tree::tests::run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it` (a new run id reads its own failures); `grace_core::state::tests::reduce::lists_the_tests_that_fail_in_the_run_once_each_in_order_224_r3_3`. |
| 49. opens the feature only, not a folder or an outline that was closed | ported | `grace::tree::tests::a_run_opens_a_feature_inside_a_closed_folder_and_leaves_the_folder_closed`, `grace::tree::tests::a_failure_leaves_a_closed_outline_closed` (TO-14); the outline's "failed" status `grace_core::model::status::tests::derive_status::failed_wins_over_passed_and_running`. |
| 50. leaves open what a run opened, after it | ported | `grace::tree::tests::run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it` (TO-15). |
| 51. rebuilds no rows for statuses that open nothing | ported | `grace::tree::tests::statuses_do_not_build_the_rows_again`. Redesigns "path rows". |
| 52. drawing: draws the first 25 rows of a project of 30 tests | ported | `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` (every row of a small project, VL-02). No jsdom here. |
| 53. shows the same rows, order and indents as before, and only matches under Hide non-matching | ported | `grace_core::rows::tests::tree_rows::lists_every_node_but_the_root_in_tree_order_with_its_level_and_place_among_its_siblings`, `tree_rows::leaves_out_non_matching_rows_under_hide_non_matching_and_counts_only_the_shown_siblings`; `grace::tree::tests::hide_non_matching_leaves_the_matching_rows`; indents `grace::tree::tests::the_focus_ring_starts_at_the_rows_indent`; `grace-e2e/tests/tags.rs::hide_non_matching_hides_the_other_rows`. |
| 54. keeps the scroll position and the drawn rows when a run updates 500 statuses | ported | `grace::tree::tests::statuses_do_not_build_the_rows_again` (no rebuild, so no anchoring and no scroll move); `grace-e2e/tests/run.rs::run_all_with_live_progress`. "Not one row drawn again" is DOM identity. |
| 55. keys: is one tab stop: the first row, with nothing selected | ported | `grace-e2e/tests/tree.rs::the_rows_buttons_are_out_of_the_tab_order_but_still_click`, `the_keys_move_through_the_rows`; the first row is the focus row from a fresh focus: `grace::tree::tests::focus_survives_scrolling_away` (nine Downs reach row 9). |
| 56. moves and selects with Down, and the details pane follows | ported | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows`; `grace::tree::tests::home_end_and_pages` (selection follows); Details follows the selection `grace::details::tests::a_test::heads_it_with_its_name_a_tinted_badge_and_the_meta_line`. |
| 57. Right and Left: opens a closed feature with Right, focus staying on it | ported | `grace-e2e/tests/tree.rs::the_keys_move_through_the_rows`. |
| 58. ... moves to the first scenario of an open feature with Right, and selects it | ported | The same test (focused and selected "passing hello"). |
| 59. ... closes an open feature with Left, focus staying on it | ported | The same test (Left closes; the focus is not moved by `toggle`). |
| 60. ... moves from a scenario to its feature with Left, and selects it | ported | The same test. |
| 61. ... does nothing with Left on a closed top-level folder | ported | `grace::tree::tests::the_keys_do_nothing_at_the_ends` |
| 62. moves to the last row with End, a screen up with Page Up, and the first with Home | ported | `grace::tree::tests::home_end_and_pages`; `grace_core::rows::tests::page_rows::counts_whole_rows_in_view_at_least_1`; `grace_core::rows::tests::scroll_to_show::scrolls_the_least_to_bring_a_row_into_view` (row 0 gives scroll top 0 with its pad). |
| 63. brings a scrolled-away focused row back before Right opens it (R4.2) | ported | `grace::tree::tests::a_scrolled_away_focus_row_comes_back_before_enter_or_right` |
| 64. runs the focused row with Ctrl+Enter, and not during a run | ported | `grace::tree::tests::ctrl_enter_runs_the_focus_row_only_when_its_run_button_would`; `grace-e2e/tests/tree.rs::ctrl_enter_runs_the_focused_row` |
| 65. debugs the focused row with Ctrl+Shift+Enter; Ctrl+Enter alone still runs it | later | P4: the debugger. The "Ctrl+Enter alone runs" half is `grace-e2e/tests/tree.rs::ctrl_enter_runs_the_focused_row`. |
| 66. runs nothing with Ctrl+Enter on a row the filter mutes: it has no run button | ported | `grace::tree::tests::ctrl_enter_runs_the_focus_row_only_when_its_run_button_would` |
| 67. keeps the row's buttons out of the tab order, and they still click | ported | `grace-e2e/tests/tree.rs::the_rows_buttons_are_out_of_the_tab_order_but_still_click` (VL-11). |
| 68. makes a clicked row the focused one, so the next key moves from it | ported | `grace::tree::tests::a_click_makes_the_row_focused`. |
| 69. focus and scrolling: takes focus to a folder closed by its chevron, keeping the selection | ported | `grace::tree::tests::closing_a_feature_with_its_chevron_takes_the_focus_to_it` (VL-16). |
| 70. moves focus to the nearest shown row above when Hide non-matching hides it | ported | `grace::tree::tests::closing_the_feature_of_the_focus_row_hands_the_focus_to_the_nearest_row_above` (VL-17: the same `Tree::build` branch; here the filter is the cause), `grace_core::rows::tests::survivor::is_the_nearest_row_before_the_key_that_is_still_there`, `grace::tree::tests::hide_non_matching_leaves_the_matching_rows`. |
| 71. keeps focus in the tree when a refresh removes the focused row | ported | The same tests plus `grace::tree::tests::a_refresh_keeps_what_was_opened_and_closed_and_a_new_feature_starts_closed` (a refresh goes through `Tree::build`). The focus stays on the tree by design: Redesigns "one tab stop". |
| 72. scrolls to the selected row after a filter change takes rows away above the view | ported | `grace-e2e/tests/tags.rs::a_filter_change_scrolls_to_the_selected_row` (VL-20; rows come back above the selection instead of leaving); `grace_core::rows::tests::anchored_scroll_top::keeps_the_first_row_in_view_still_when_rows_are_added_above`. |
| 73. scrolls to a test paused in a closed feature, once its rows are open (#224 R3.7) | later | P4: the pause needs the debugger. The same path for a reveal is `grace::tree::tests::a_reveal_opens_the_rows_above_the_selected_test`. |
| 74. does not scroll to the selected row for a new tree and the same filter | ported | `grace::tree::tests::only_a_new_filter_scrolls_to_the_selected_row` |
| 75. shows a cut label whole when Down moves to its row, and closes it on the next Down | ported | `grace::tree::tests::a_cut_label_shows_whole_on_keyboard_focus` (VL-21: Down, Down, Up). `data-side` is DOM. |
| 76. shows a cut label whole when Left or Right moves to its row | ported | The same test: Left and Right go through `Tree::move_to_key` and `move_to`, which set the keyboard ring the tooltip follows (weak: Down and Up only are driven). |
| 77. names the run shortcut on every row and on the run button's tooltip | ported | `grace-e2e/tests/tree.rs::the_rows_buttons_are_out_of_the_tab_order_but_still_click` (tooltip "Run passing hello (Ctrl+Enter)"); `tree::SHORTCUTS` on each row, which AT-SPI does not pass on (parity VL-22). The Shift variant is P4. |
| 78. screen readers: gives rows their level, place and open state, in a tree named Tests | ported | `grace_core::rows::tests::tree_rows::lists_every_node_but_the_root_in_tree_order_with_its_level_and_place_among_its_siblings` (level, position, set size, VL-23); the tree named "Tests" is waited for in every `grace-e2e/tests/tree.rs` test; `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` (expanded state). On Linux the pinned AccessKit adapter drops level and expanded (AGENTS.md). |
| 79. names a row by its label, status, flaky chip and tags | ported | `grace_core::rows::tests::row_name_joins_the_label_the_status_word_in_lower_case_the_flaky_words_and_the_tags`; `grace-e2e/tests/run.rs::run_all_with_live_progress` ("slow loop, running"). |
| 80. the walks behind a row: works out a changed row's status once per update, and does not count its flaky tests again | n/a | Counts calls of a mocked Solid memo. `Tree::row` works out one row's status and chip once per drawn frame; the cost is measured by the bench (VL-27, VL-28, PR 6). |
| 81. ... walks only the new rows when a feature opens (#224 R4.3) | n/a | The same: memo call counts. Redesigns "path rows" (rows built again only when the tree, filter or a turn changes) is `grace::tree::tests::statuses_do_not_build_the_rows_again`. |
| 82. ... works out no status when the selection or the focus moves | n/a | The same: memo call counts. The `leaves` calls it also counts are the HTTP tab's (P3). |

##### tree: step debugger (R5, R7)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 83. a Debug button before Run, disabled during a run | later | P4: the debugger; Redesigns "phases". |
| 84. marks the feature and the scenario holding a breakpoint | later | P4: the debugger (breakpoints); Redesigns "phases". |
| 85. shows the pause mark on the paused test and selects it | later | P4: the debugger. The selection half is core's `grace_core::state::tests::the_step_debugger::the_first_pause_shows_the_variables_tab_and_selects_the_paused_test_later_ones_keep_the_tab`; `tree.rs` already draws the Paused mark. |

##### tree: open in VS Code (open-in-editor-from-tree R1, R2)

All six: LATER, P3. The feature map lists "Open in VS Code, from details and the tree" (#68, #222, `grace-core::editor`, context menu, OE-*, OT-*, `editor.spec.ts`) in P3; Redesigns "phases" says the row's menu comes with it.

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 86. asks for the menu at the pointer, for a test at its line, and keeps the selection | later | P3 (OT-*). |
| 87. asks from a row's run button and from the focused row itself, as the menu key does | later | P3 (OT-*). |
| 88. opens a feature at line 1, and asks nothing for a folder or the list's padding | later | P3 (OT-*). |
| 89. opens a failed test at its failed step during a run, even in a called feature | later | P3 (OT-*). |
| 90. asks from an open tooltip over a row | later | P3 (OT-*). |
| 91. opens a failed test without a place at its own line | later | P3 (OT-*). |

##### tree: reveal a test from Details (#229 R4.3)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 92. reveal opens a closed feature | ported | `grace::tree::tests::a_reveal_opens_the_rows_above_the_selected_test` (the selected test is a row, its feature opens, the other stays closed). The "Next test with this error" button is P3 (failures grouped, #229, GF-*). |

#### `Home.test.tsx` (26)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 draws nothing until main says what is open | ported | `grace_core::state::tests::reduce::home_screen::starts_not_loaded_with_nothing_opening_and_no_list`, `...::an_opening_event_sets_opening_and_loaded_and_null_clears_opening`; `grace::app::tests::a_first_start_says_nothing_is_open_and_sends_the_recent_folders`; `grace-e2e/tests/home.rs::the_last_project_opens_at_start_without_the_home_screen` (polls: home never shows before the project, HS-05). The `#startup` div is gone (Redesigns, `window`) |
| 2 a first start shows home without a list | ported | `grace::home::tests::a_first_start_shows_the_home_screen_without_a_list` (home drawn, no recent-list, the line); `grace-e2e/tests/home.rs::a_first_start_shows_the_home_screen` (heading, sentence, Open folder…, no Recent folders heading or list, no unnamed control). By eye: primary fill, FolderOpen icon |
| 3 the mascot is decorative | ported | `grace::home::tests::the_mascot_is_decorative` (240x180, asset loads, no id/role); `grace-e2e/tests/home.rs::a_first_start_shows_the_home_screen` (no Image node) |
| 4 an open project hides the mascot | ported | `grace::window::tests::the_project_view_sits_on_the_sheet_under_the_toolbar` (`home`, which holds the mascot, is not drawn with a project open); `grace_core::state::tests::added::the_home_screen_shows_the_opens_error_as_a_notice` (`home()` false with a project) |
| 5 Open folder… opens the dialog | ported | `grace::home::tests::open_folder_opens_the_folder_dialog`; `grace::app::tests::open_asks_for_a_folder_and_opens_it`; `grace-e2e/tests/home.rs::opening_a_folder_from_the_home_screen` |
| 6 lists the recent folders | ported | `grace::home::tests::lists_the_recent_folders`, `reopening_a_row_opens_its_folder`, `says_not_found_in_a_cancelled_badge_with_the_warning_icon_and_the_row_is_disabled`; `grace-e2e/tests/home.rs::reopening_a_folder_from_the_list`; `grace-e2e/tests/home.rs::removing_a_row` (the list "Recent folders" and its items, and its heading on Linux, where the driver reads headings) |
| 7 one surface box, a rule between rows (R12.3) | ported | `grace::home::tests::shows_the_list_in_one_surface_box_with_a_rule_between_rows_r12_3` (rows inside the box). By eye: surface fill, radius, rule, folder icon, name 500, mono muted path, hover fill, inset focus ring, missing name muted and not dimmed ( |
| 8 Not found in a cancelled badge with the icon, row disabled | ported | `grace::home::tests::says_not_found_in_a_cancelled_badge_with_the_warning_icon_and_the_row_is_disabled` (badge only on the missing row, a click opens nothing); `grace-e2e/tests/home.rs::a_missing_last_project_shows_the_home_screen` (name holds "Not found"). By eye: tint, 12 px icon |
| 9 a missing folder explained in a tooltip though disabled | ported | `grace::home::tests::explains_a_missing_folder_in_a_tooltip_though_its_row_is_disabled` ("Folder not found" over the row; nothing on a folder that is there) |
| 10 a cut path shows whole in a tooltip, and only then (R9.4) | ported | `grace::home::tests::shows_a_cut_path_whole_in_a_tooltip_and_only_then` |
| 11 remove: 24 px icon button with name and tooltip (R10.3) | ported | `grace::home::tests::removes_a_row_with_its_button_and_takes_the_focus_to_open_folder` (each row's button 24 × 24 px, its tooltip "Remove from list", a click removes that row's path); `grace-e2e/tests/home.rs::removing_a_row` (the name). `title=` is `n/a` |
| 12 removing a row moves the focus to Open folder… | ported | `grace::home::tests::removes_a_row_with_its_button_and_takes_the_focus_to_open_folder` (the focus is on Open folder… and stays there when the new list comes) |
| 13 names a folder by its last segment | ported | `grace::home::tests::names_a_folder_by_its_last_segment_whichever_slash_it_uses` (same three paths) |
| 14 opening with no project shows the project view, not home | ported | `grace_core::state::tests::reduce::home_screen::an_opening_event_sets_opening_and_loaded_and_null_clears_opening`; `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` (path = the opening folder); `grace::toolbar::tests::the_actions::keeps_run_all_cancel_open_and_report_as_before` (busy: Cancel on, Home off); `grace-e2e/tests/open.rs::finding_tests_for_the_first_time` ("Finding tests…" in the tree); `grace::progress::tests::after_a_run_r14_2::shows_finding_tests_with_the_turning_icon_while_the_runner_discovers` |
| 15 a stale folder error does not show while the next opens | ported | `grace_core::state::tests::reduce::home_screen::opening_a_folder_with_no_project_open_drops_the_last_opens_error` |
| 16 opening another folder keeps the open project on screen | ported | `grace_core::state::tests::reduce::home_screen::opening_another_folder_keeps_the_open_projects_error`; `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` ("the open project stays shown while another opens") |
| 17 a cancelled open goes back home with the notice | ported | `grace_core::runner::tests::home::a_failed_open_from_the_home_screen_changes_nothing` (the runner's events: project null, "Cancelled", opening null); `grace_core::state::tests::reduce::notice_sets_the_notice_a_warning_unless_main_says_it_reports_a_success`; `...::added::the_home_screen_shows_the_opens_error_as_a_notice`. The notice drawn on home: `grace::home::tests::a_notice_shows_above_the_column_inside_home` |
| 18 a missing folder shows its reason on home | ported | `grace::home::tests::a_missing_folder_shows_its_reason_on_home`; `grace_core::state::tests::added::the_home_screen_shows_the_opens_error_as_a_notice` |
| 19 the notice is a warning: status, cancelled tint, icon | ported | `grace::ui::tests::a_warning_is_a_status_tinted_cancelled`, `draws_a_different_icon_per_kind` |
| 20 a notice shows above the column, inside home | ported | `grace::home::tests::a_notice_shows_above_the_column_inside_home` |
| 21 home on the ground in both themes, no sheet (R12.1) | ported | `grace::window::tests::home_is_drawn_alone_with_no_toolbar_and_no_sheet`; the ground colour: `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start`, `follow_windows_is_the_default` |
| 22 project view on the sheet under the toolbar; home has none | ported | `grace::window::tests::the_project_view_sits_on_the_sheet_under_the_toolbar`, `the_notice_the_main_area_and_the_panel_are_on_the_sheet` (notice, main and bottom panel inside the sheet, the toolbar outside), `home_is_drawn_alone_with_no_toolbar_and_no_sheet`. `relative` and `overflow-clip` are `n/a` (CSS) |
| 23 the app root is ground and ink, no `dark:` | ported | `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start`, `choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart` (ground pixel in both themes, Linux); `grace::theme::tests::colours_come_only_from_the_theme` (no literal colour; `dark:` has no GPUI twin). By eye: ink text |
| 24 a dry-run error shows in the project view, not as a notice | ported | `grace_core::state::tests::added::the_home_screen_shows_the_opens_error_as_a_notice` (`!home()`, no shown notice with a project); `grace-e2e/tests/home.rs::a_dry_run_error_still_opens_the_project`; `grace-e2e/tests/open.rs::syntax_error_is_reported_not_hidden` |
| 25 Home closes the project | ported | `grace::toolbar::tests::home_and_refresh_the_icon_only_buttons::home_goes_home_and_is_off_while_busy`; `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart`, `home_is_disabled_during_a_run` |
| 26 Home is the first item of the toolbar | ported | `grace::toolbar::tests::order_and_shape_r13_1::puts_the_controls_in_the_order_of_the_spec_run_all_last` (Home leftmost); `grace-e2e/tests/home.rs::home_closes_the_project_and_survives_a_restart` (named Home); the banner landmark: `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests` (banner, main and content info) |

#### `FieldMessage.test.tsx` (5: an `it.each` of 3 and 2)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 hint: 12 px text in its tone with its own icon | ported | `grace::ui::tests::a_field_message_takes_its_kinds_tone_and_icon_or_the_one_asked` (muted, Info). The `id` is `n/a` (Redesigns, `ui`) |
| 2 warning: same, cancelled tone, TriangleAlert | ported | `a_field_message_takes_its_kinds_tone_and_icon_or_the_one_asked` (cancelled, TriangleAlert) |
| 3 error: same, failed tone, CircleAlert | ported | `a_field_message_takes_its_kinds_tone_and_icon_or_the_one_asked` (failed, CircleAlert) |
| 4 takes another icon, and a class for its place | ported | `a_field_message_takes_its_kinds_tone_and_icon_or_the_one_asked` (the lock asked for). `class` and "no id" are `n/a` (DOM) |
| 5 wraps long text beside its icon instead of overflowing | ported | `grace::ui::tests::a_long_field_message_wraps_beside_its_icon` |

#### `store.test.tsx` (8)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 subscribes before ready, applies events, selects, disposes | ported | `grace_core::state::tests::create_app_store::applies_events_and_selects` (notice set, a null project clears it, error, `select`). The IPC subscribe-before-ready order and `dispose` are gone (Redesigns, `state`, `app`: no `ready` handshake) |
| 2 trims console and drops stale keys through a real store draft | ported | `grace_core::state::tests::create_app_store::trims_console_and_drops_stale_keys_through_a_real_store_draft` |
| 3 dismissPrompt drops the startup modal's release | ported | `grace_core::state::tests::create_app_store::dismiss_prompt_drops_the_startup_modals_release` |
| 4 tag filter: starts with no filter | ported | `grace_core::state::tests::create_app_store::tag_filter::starts_with_no_filter` |
| 5 tag filter: filter event replaces the filter | ported | `...::tag_filter::filter_event_replaces_the_filter` |
| 6 tag filter: opening another project resets the filter | ported | `...::tag_filter::opening_another_project_resets_the_filter` ("own copy, never the shared constant" is by value in Rust) |
| 7 http calls: selectCall, select and showPanel | ported | `...::create_app_store::http_calls::select_call_select_and_show_panel` |
| 8 http calls: toggleGroup drops a call it hides | P3 | `AppState` has `http_groups` but no `toggle_group` yet; it ports with the HTTP tab (plan, PR 1 item 1) |

#### `Toolbar.test.tsx` (36)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 is one 44 px row on the ground | ported | `grace::toolbar::tests::order_and_shape_r13_1::is_one_44_px_row_on_the_ground`. By eye: ground fill |
| 2 puts the controls in the order of the spec, Run all last | ported | `grace::toolbar::tests::order_and_shape_r13_1::puts_the_controls_in_the_order_of_the_spec_run_all_last`, `...::properties::sits_after_the_boxes_and_before_refresh`; with Env and Threads: `grace::window::tests::the_toolbar_keeps_its_items_in_order_and_its_boxes_at_the_controls_height`. Watch with P4 |
| 3 separates Properties from Refresh and Open from Report with 1 px dividers | ported | `...::separates_the_project_from_refresh_and_open_from_report_with_1_px_dividers` (1 px, 20 px high, between) |
| 4 every button keeps its label, 28 px tall or a 28 px icon | ported | `...::every_button_is_28_px_tall_and_the_icon_buttons_square`; labels as names: `grace-e2e/tests/open.rs::a_narrow_window_keeps_every_toolbar_button_on_screen_with_its_name` (the `TOOLBAR` list) |
| 5 never wraps a button's label | n/a | CSS class `whitespace-nowrap`; a wrapped label would break the 28 px height asserted in `...::every_button_is_28_px_tall_...` and the widths in `a_narrow_window::*` |
| 6 below 1280 px hides the label of five buttons, not their names | ported | `grace::toolbar::tests::a_narrow_window::below_1280_px_shrinks_open_report_and_cancel_to_a_28_px_icon`, `keeps_the_count_pill_of_re_run_failed`; `grace-e2e/tests/open.rs::a_narrow_window_keeps_every_toolbar_button_on_screen_with_its_name` (Properties…, Open…, Report, Cancel under 40 px at 780, 1200, 1279, each found by name) |
| 7 shrinks four to a 28 px icon, keeps Re-run failed's pill | ported | `a_narrow_window::below_1280_px_shrinks_...`, `keeps_the_count_pill_of_re_run_failed` |
| 8 keeps Run all's label in sight | ported | `a_narrow_window::keeps_run_alls_label_in_sight` |
| 9 asks for the width the classes use | ported | `a_narrow_window::asks_for_the_width_the_electron_classes_use` (`NARROW_BELOW` = 1280); `grace-e2e::open` (1279 narrow, 1281 wide) |
| 10 a tooltip with the label while hidden, also when disabled | ported | `a_narrow_window::gives_each_a_tooltip_with_its_label_while_the_label_is_hidden_also_when_disabled` (open, report, cancel, re-run) |
| 11 no tooltip while the label shows | ported | `a_narrow_window::gives_them_no_tooltip_while_the_label_shows`; Home and Refresh keep theirs: `home_and_refresh_the_icon_only_buttons::*` at 1400 px |
| 12 follows the window as it is resized, stops listening | ported | `a_narrow_window::follows_the_window_as_it_is_resized` (both ways). The `matchMedia` listener count is `n/a`: GPUI reads the viewport at each render |
| 13 below 1280 px hides the words Env and Threads, not the boxes' names | ported | `grace::env_picker::tests::below_1280_px_the_word_env_goes_from_sight_and_says_itself_in_a_tooltip`, `grace::threads_box::tests::below_1280_px_the_word_threads_goes_from_sight_and_says_itself_in_a_tooltip` (the word takes no room); names: `grace-e2e::env`, `grace-e2e::threads` (combo box "Env", spin button "Threads" at the default 1200 px, a narrow window) |
| 14 then says Env and Threads in a tooltip beside the box, nothing while words show | ported | The same two tests (tooltip text "Env", "Threads" when narrow) |
| 15 Home holds the logo, is named Home, tooltip | ported | `home_and_refresh_the_icon_only_buttons::home_holds_the_logo_and_shows_home_in_a_tooltip` (tip "Home", logo asset). By eye: 20 px logo |
| 16 Home goes home, off while busy | ported | `home_and_refresh_the_icon_only_buttons::home_goes_home_and_is_off_while_busy`; `grace-e2e/tests/home.rs::home_is_disabled_during_a_run` |
| 17 Refresh named, icon, tooltip also while disabled | ported | `home_and_refresh_the_icon_only_buttons::refresh_shows_its_word_in_a_tooltip_also_while_disabled` (tip while busy, a click does nothing, then calls refresh) |
| 18 shows the folder name and the layout, not the path (R1.1) | ported | `grace::toolbar::tests::the_project::shows_the_folder_name_and_the_layout_and_leaves_the_path_to_the_status_bar_r1_1`, `draws_the_name_and_the_chip`; the path in the status bar: `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen`. By eye: name 500 |
| 19 names a folder with forward slashes or a trailing separator | ported | `the_project::names_a_folder_opened_with_forward_slashes_too_and_one_with_a_trailing_separator` |
| 20 cuts a long name in the middle, last 12 characters stay (R1.2) | ported | `the_project::cuts_a_long_name_in_the_middle_its_start_gives_way_its_last_12_characters_stay_r1_2`; `grace::window::tests::a_long_path_at_the_minimum_width` (name keeps room before Env). `shrink-100000` is `n/a` (CSS) |
| 21 keeps a name of 12 characters or fewer in its end part | ported | `the_project::keeps_a_name_of_12_characters_or_fewer_in_its_end_part` |
| 22 explains a cut name in a tooltip, nothing while it fits (R1.3) | ported | `the_project::explains_a_cut_name_in_a_tooltip_and_says_nothing_while_it_fits_r1_3` |
| 23 lets the chip go, whole, before the name is cut (R1.2) | ported | `the_project::lets_the_chip_go_whole_before_the_name_is_cut_r1_2`; `grace::window::tests::the_name_shows_whole_at_the_default_size`, `a_long_path_at_the_minimum_width` |
| 24 has no title attribute left | n/a | No `title=` in GPUI (parity: `palette.test.ts` row); tooltips only through `grace::tooltip` |
| 25 Run all is the only primary button, play icon in on-primary ink | by eye | Fill and icon ink are colours; by construction `ButtonKind::Primary` on `run-all-button` only (`toolbar.rs`). Token values: `grace::theme::tests` |
| 26 the failed count in a pill on Re-run failed | ported | `the_actions::shows_the_newest_runs_failed_count_on_re_run_failed`, `re_run_failed_reruns_the_newest_run` (pill drawn, click reruns the newest run) |
| 27 no pill while nothing to re-run | ported | `the_actions::shows_no_pill_while_there_is_nothing_to_re_run` |
| 28 keeps Run all, Cancel and Open as before | ported | `the_actions::run_all_runs_the_trees_root`, `keeps_run_all_cancel_open_and_report_as_before`, `cancel_and_report_ask_the_runner`, `open_asks_for_a_folder` |
| 29 draws an icon on the text buttons, decoratively | by eye | `.icon(..)` on the six buttons in `toolbar.rs`; presence of an icon is not read headless; "decoratively" is by construction |
| 30 Env and Threads use the shared input style | ported | `grace::window::tests::the_toolbar_keeps_its_items_in_order_and_its_boxes_at_the_controls_height` (both boxes 28 px, fixed here). By eye: border, fill, radius |
| 31 a normal run keeps Run all; a debug run shows Pause | P4 | Step debugger (Redesigns, `toolbar`) |
| 32 paused: Continue, Step over, Step into; F5, F10, F11 | P4 | Step debugger |
| 33 Watch: icon button named Watch, off until main says on | P4 | Watch (Redesigns, `toolbar`) |
| 34 Watch asks main for the other state | P4 | Watch |
| 35 Watch says what it does in its tooltip | P4 | Watch |
| 36 Watch stays enabled during a run, off during an open | P4 | Watch |

#### `StatusBar.test.tsx` (18)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 shows the running version and Settings | P5 | Version needs the updater's view. The Settings half: the button named "Settings" is found and pressed in `grace-e2e/tests/appearance.rs::open_appearance` |
| 2 the marker and Update while out of date | P5 | |
| 3 keeps the marker for a skipped version | P5 | |
| 4 no marker when the channel has nothing newer | P5 | |
| 5 names the marker with the running and latest version | P5 | |
| 6 Update installs latest with the current data | P5 | |
| 7 Update disabled while a run or discovery is active (R6.4) | P5 | |
| 8 Update disabled in a build that is not packaged (R13.1) | P5 | |
| 9 Update enabled when packaged and idle | P5 | |
| 10 the marker, the version and Settings open the settings | P5 | Marker and version are P5. The Settings click: `grace-e2e/tests/appearance.rs::open_appearance`, `escape_goes_back_from_the_settings_page` |
| 11 a 24 px bar on the ground, muted 12 px text (R21) | ported | `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` (the 24 px bar and the Settings button's 24 px target, fixed here from 20 px); the content-info landmark: `grace-e2e/tests/open.rs::open_a_project_and_see_its_tests`. By eye: ground, muted 12 px text |
| 12 the update marker as a running tinted badge | P5 | |
| 13 shows nothing before main sends the update view | P5 | The bar itself always draws (`window.rs`); the null-view case is the updater's |
| 14 starts the bar with the full path, mono muted, in two parts | ported | `grace::status_bar::tests::splits_a_path_at_the_project_folder`, `shows_the_path_while_a_project_is_open_and_none_on_the_home_screen`; `grace::window::tests::a_long_path_at_the_minimum_width` (the path ends before the Settings button) |
| 15 splits forward slashes and a trailing separator | ported | `grace::status_bar::tests::splits_a_path_at_the_project_folder` (`/home/me/api/` gives `/home/me/` and `api/`) |
| 16 shows the folder being opened, and the open project while another opens | ported | `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` (`shown_path`, both cases) |
| 17 shows no path on the home screen | ported | `grace::status_bar::tests::shows_the_path_while_a_project_is_open_and_none_on_the_home_screen` (last assertion) |
| 18 explains a cut path in a tooltip, nothing while it fits | ported | `grace::status_bar::tests::explains_a_cut_path_in_a_tooltip_and_says_nothing_while_it_fits` |

#### `Settings.test.tsx` (43)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 the running version, the marker and Update to the latest | P5 | Updates section |
| 2 neither marker nor Update when up to date | P5 | |
| 3 Update to <latest> installs with the current data | P5 | |
| 4 goes back with Back and with Escape | ported | `grace::settings::tests::the_page::the_back_button_goes_back`; `grace-e2e/tests/appearance.rs::escape_goes_back_from_the_settings_page` (from the page and from a radio) |
| 5 leaves Escape to an open dialog | P5 | The page's only dialog is the switch dialog |
| 6 leaves Escape to a tooltip, which closes alone (R9.4) | P5 | Redesigns, `settings`: "E-appearance-05's tooltip half waits for P5's update button". Tooltip then Escape in general: `grace-e2e/tests/tooltips.rs::escape_in_a_dialog_closes_the_tooltip_first` |
| 7 lists the releases on the channel, newest first | P5 | Version table |
| 8 adds the running version the channel does not list | P5 | |
| 9 hides pre-releases on stable, keeps the running row | P5 | |
| 10 tags the running row installed, a saved copy data saved | P5 | |
| 11 Install above the running version, Roll back below | P5 | |
| 12 an empty date for a release without one | P5 | |
| 13 holds the running version only before a check | P5 | |
| 14 does not ask for a check after one | P5 | |
| 15 Install on a newer version without a saved copy installs at once | P5 | Switching |
| 16 Install with a saved copy asks which data first | P5 | |
| 17 Roll back opens the confirmation, "Roll back and restart" switches | P5 | |
| 18 Roll back without a saved copy keeps the data | P5 | |
| 19 Cancel closes the confirmation, installs nothing | P5 | |
| 20 gives focus back to the row's button when the dialog closes | P5 | |
| 21 disables every switch button while a run or discovery is active | P5 | |
| 22 disables them in a build that is not packaged | P5 | |
| 23 enables them when packaged and idle | P5 | |
| 24 selects the channel the settings hold | P5 | The settings (R2, R5, R6.2) |
| 25 changing the channel saves it | P5 | |
| 26 saves the startup check and auto-update, one key at a time | P5 | |
| 27 disables auto-update while the startup check is off | P5 | |
| 28 checks for updates on request, Checking… meanwhile | P5 | |
| 29 says when it last checked | P5 | |
| 30 a failed check's reason with the last good check | P5 | |
| 31 no error after a good check | P5 | |
| 32 only the header and the section's title before main sends the update view | P5 | The null update view. The P2 half, the heading "Settings": `grace-e2e/tests/appearance.rs::follow_windows_is_the_default` |
| 33 the page on a sheet under a header on the ground | ported | `grace::settings::tests::the_page::puts_the_sections_on_a_sheet_under_the_header` (header above the sheet, sheet inset). By eye: sheet fill and radius, header min 44 px |
| 34 the sections in the selectable row style, the open one selected | ported | `grace::settings::tests::the_page::opens_on_appearance_with_its_entry_selected` (the entry exists, left of the section, `section` is Appearance); the navigation "Settings sections" and its selected entry: `grace-e2e/tests/appearance.rs::follow_windows_is_the_default`. By eye: 28 px row, selected fill, accent stripe |
| 35 titles the section in text-lg | P5 | The Updates heading and region; Appearance's own are in 39 |
| 36 tags the versions with tinted badges | P5 | |
| 37 a failed check's reason as an error message | P5 | |
| 38 keeps the dialog's words while it fades out, asks afresh | P5 | The fade-out half goes (Redesigns, `properties` row: GPUI Kit drops a dialog's view with it) |
| 39 opens on Updates, the list switches to Appearance and back | P5 | One section only until P5. The P2 half (opens on Appearance, entry selected, section drawn): `grace::settings::tests::the_page::opens_on_appearance_with_its_entry_selected`. Appearance's heading: `grace-e2e/tests/appearance.rs::follow_windows_is_the_default` |
| 40 three themes in a Theme group, Follow Windows checked | ported | `grace::settings::tests::appearance_r8::lists_the_three_themes_in_a_theme_group` (order, 24 px rows, group); `grace-e2e/tests/appearance.rs::follow_windows_is_the_default` (checked states, nothing saved); `appearance_r8::choosing_the_theme_in_force_saves_nothing` |
| 41 selects the saved theme once main answers | ported | `grace-e2e/tests/appearance.rs::a_saved_dark_is_in_force_at_start` (Dark checked), `choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart` (Light checked after a restart); `grace_core::settings` tests `appearance` (PU-17) |
| 42 choosing a theme saves it and selects it at once | ported | `grace::settings::tests::appearance_r8::choosing_a_theme_applies_it_at_once_and_saves_it`, `a_change_keeps_the_rest_of_the_views_state`; `grace-e2e/tests/appearance.rs::choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart` |
| 43 styles the radios like the channel's, with the accent | by eye | The accent colour of GPUI Kit's `Radio`; the 24 px rows are asserted in `appearance_r8::lists_the_three_themes_in_a_theme_group` |

#### `Details.test.tsx` (34 tests, one an `it.each` of 2)

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1 a test: name, tinted badge with icon and word, meta line | ported | `grace::details::tests::a_test::heads_it_with_its_name_a_tinted_badge_and_the_meta_line` (name, badge, place, time, and the tags as chips after them; none on a test without tags); `grace-e2e/tests/details.rs::failure_details`. By eye: tint, mono |
| 2 a status badge for a failed and a not-run test | ported | `a_test::tints_the_badge_for_a_passed_a_failed_and_a_pending_test` (`badge_tint`, label "Pending", badge drawn for all three) |
| 3 a failed test's error in a failed box titled Error | ported | `a_test::puts_a_failed_tests_error_in_a_box_and_none_for_a_passed_one`; `grace-e2e/tests/details.rs::failure_details` (Group "Error" holds the message alone). By eye: failed tint, mono, icon |
| 4 each step with a mark named by its status | ported | `a_test::shows_each_step_with_a_mark_named_by_its_status` (labels incl. "Interrupted" for an unknown status, rows in order, Background chip only on the background step); `grace-e2e/tests/details.rs::failure_details` (5 steps in a list "Steps"); marks: `grace::elements::tests::marks_each_status_as_electron_grace_does` |
| 5 a step's comments above it, muted, as written | ported | `a_test::shows_a_steps_comments_above_it` (none on a step without; above the text; one line per comment); `grace-e2e/tests/details.rs::failure_details` (the comment in the error text). By eye: muted mono |
| 6 toggles a step's log: ghost button, chevron that turns | ported | `a_test::toggles_a_steps_log_with_its_button` (names "Show/Hide log of ..", none without a log, open and close, log under its step); the chevron changes icon (Right, Down); `grace-e2e/tests/details.rs::failure_details` (press "Show log of method get", the log shows). By eye: sunken log |
| 7 steps come when the feature finishes, spinner while a run is on | ported | `a_test::says_steps_come_when_the_feature_finishes_with_the_spinner_while_a_run_is_on` (spinner only while running; not for a test not reached) |
| 8 a group counts the tests per status, mark, figure and word | ported | `grace::details::tests::a_group::counts_the_tests_per_status_in_tree_order`; the words: `grace-e2e/tests/details.rs::failure_details` ("N scenarios", "1 Failed", "N Passed" on the feature) |
| 9 works out the group's status once per update | n/a | Counts `deriveStatus` calls of a Solid memo; Redesigns, `details` row ("no memo, a group's status is worked out once per frame": cited proof `a_group`). The live badge update is `cx.observe(&grace)` redraw |
| 10 nothing selected is an empty state | ported | `grace::details::tests::nothing_selected::is_an_empty_state`, `a_selection_gone_from_the_tree_shows_nothing`; `grace-e2e/tests/details.rs::failure_details` (text "Select a test to see its details") |
| 11 the location whole in a tooltip only when cut | ported | `grace::details::tests::long_text::shows_the_location_whole_only_when_it_is_cut` |
| 12 a step whole in a tooltip only when cut | ported | `grace::details::tests::long_text::shows_a_step_whole_in_a_tooltip_only_when_it_is_cut` ("Given ...") |
| 13 no raw palette class, no `dark:`, no title | ported | `grace::theme::tests::colours_come_only_from_the_theme` (scans `details.rs` too); `dark:` and `title=` have no GPUI twin |
| 14 lists the steps from the file before a run | P4 | Redesigns, `details` (E-details-01's first half waits for the file's steps) |
| 15 toggles a breakpoint on a step | P4 | Breakpoints |
| 16 marks the step the run is paused before | P4 | Debugger |
| 17 the run's steps replace the file's, same toggles | P4 | Breakpoint toggles |
| 18 opens the test at its line, during a run too | P3 | Open in VS Code |
| 19 no button for a row with no line | P3 | |
| 20 the place on the error's first line is a link | P3 | |
| 21 takes a lowercase drive and `/` separators | P3 | |
| 22a error without a place as text: "Config evaluation failed..." | P3 | `it.each` case 1 |
| 22b error without a place as text: "boom at x.feature:7" | P3 | `it.each` case 2 (the error as plain text is `a_test::puts_a_failed_tests_error_in_a_box_...`) |
| 23 says where the test sits in its group (#229 R4) | P3 | Failure groups and stepping |
| 24 Next selects the next test, keeps the focus | P3 | |
| 25 announces where the selected test sits | P3 | |
| 26 Previous wraps round | P3 | |
| 27 a lone failure has no stepping | P3 | |
| 28 no stepping while a run goes on | P3 | |
| 29 no breakpoint dot on a step below line 1 (#229 R1.4) | P4 | Breakpoints |
| 30 lists the folder's groups, largest first (#229 R5) | P3 | |
| 31 Go to first selects the group's first test | P3 | |
| 32 Go to first moves the focus to the test's heading | P3 | |
| 33 cuts a reason after 4 lines | P3 | |
| 34 no list without a group | P3 | |

#### The small files

| File | Vitest test | Rust test |
|---|---|---|
| `placeTip.test.ts` (16) | the 6 below, the 8 beside and the 2 over | `grace::tooltip::tests::goes_beside_its_target_on_the_side_asked_and_below_by_default` and `stays_in_the_window_flipping_where_it_has_no_room`: the 6 px gap, the flips and the 8 px edge, through GPUI Kit's `Positioner`, which places it; `a_click_on_an_over_tooltip_within_its_target_reaches_the_target` (on the target's top left) |
| `StatusMark.test.tsx` (8) | covers every status; names the words; colours by status | `grace::elements::tests::marks_each_status_as_electron_grace_does` |
| | draws a shape of its own, except interrupted and cancelled | `tells_statuses_apart_by_shape_or_name` |
| | turns only while running | `turns_only_while_running_and_never_with_reduced_motion` |
| | is 14 px unless asked for 16 | `draws_buttons_icons_and_marks_at_their_sizes` |
| `Icon.test.tsx` (3) | a decorative 16 px Lucide svg by default; takes its size | `draws_buttons_icons_and_marks_at_their_sizes`; `grace::assets::tests::has_the_icons_of_electron_grace` (the same Lucide files) |
| | draws one child per node of the icon | `n/a`: GPUI draws the Lucide file, `grace::assets::tests::serves_every_icon` |
| `buttons.test.ts` (2) | dialog buttons in 14 px text; `btnIconSmall` is 24 × 24 | `draws_buttons_icons_and_marks_at_their_sizes` |
| `ui.test.ts` (1) | a dialog takes no pointer events unless it is open | `n/a`: a closed GPUI dialog is not drawn |

## Exit check (from #281)

The parity rows of phase P2 ticked; `npm run bench` on the GPUI build within every limit it shares with Electron Grace, reported in the PR.

## Cost

- **Memory.** The window's state is what Electron Grace's renderer held, without a JavaScript heap or a DOM: the tree once (shared with the runner through an `Arc`), the console's last 10,000 lines, the run's steps.
- **Idle.** No timer, poll or animation while no run is going. The window waits on the runner's channel; the single-instance listener is one thread blocked in `accept`, which wakes only when a second Grace starts.
- **During a run.** One update of the state and one redraw per batch of events, as `batch.ts` gives today; only the views whose data changed redraw.
- **Start.** The window shows after its first frame (#90); the last project's dry run starts after the window, as today.
- **Size.** No new dependency in PR 1: the Shell dialog uses the `windows` crate already in the lock, with two more features.
