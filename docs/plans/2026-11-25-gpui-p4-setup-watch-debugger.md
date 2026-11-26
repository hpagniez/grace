---
status: approved
issue: 283
summary: "GPUI phase 4 in eleven slices: pom defaults, libraries, Build, the sub-folder offer, watch mode and the step debugger on the unchanged listener jar"
---

# Plan: GPUI migration, phase P4 project setup, watch and the step debugger

Date 2026-11-25. Issue [#283](https://github.com/hpagniez/grace/issues/283), under [#278](https://github.com/hpagniez/grace/issues/278). Spec: the [migration guide](../specs/2026-11-08-gpui-migration.md) (R1 to R10, feature map P4, regression #236), [ADR 0031](../decisions/0031-move-grace-to-rust-and-gpui.md). Checklist: [parity](../specs/2026-11-08-gpui-migration-parity.md). Model: the [P3 plan](2026-11-12-gpui-p3-panels.md).

P4 ports what is left of opening a project and the two features that change how a run starts: Maven, Gradle, standalone and `karate-pom.json` projects, with pom.xml's `karate.env`; the libraries a Maven or Gradle project's own build lists, and finding Maven and Gradle off PATH on each platform; the not-built notice and its Build button; the sub-folder offer of an empty project; watch mode; and the step debugger, with breakpoints, pause, step and the Variables tab, over the same socket protocol and the unchanged `listener.jar`. It ends when the rows below are ticked and its exit check is recorded. The Electron app does not change.

## Rows to tick

Counted from the parity file on `develop` at `cf6b888`, by its Ported column.

| Part of the list | Open | Of which |
|---|---|---|
| Spec scenarios | 107 | 103 of P4's own sections (PL 13, PS 9, SD 23, FB 6, WM 17, BN 10, PD 17, SO 8) and 4 handed on: SC-06, SC-08 and GF-04 from P3, TO-16 from P2 |
| End-to-end tests | 12 | 11 of P4's files (`debug` 2, `project-setup` 8, `watch` 1) and E-details-01's first half (a test's steps before a run) from P2 |
| Unit and component files | 14 files, 188 tests | `compiled.test.ts` 12, `debug.test.ts` 8, `libraries.test.ts` 29, `maven.test.ts` 19, `pom.test.ts` 5 (the `pomStart` tests; its other 14 are P1's), `subfolders.test.ts` 10, `watch.test.ts` 13, `runner.debug.test.ts` 11, `runner.libraries.test.ts` 31, `runner.pom.test.ts` 12, `runner.setup.test.ts` 9, `runner.subfolders.test.ts` 7, `runner.watch.test.ts` 18, `Variables.test.tsx` 4 |
| Tests waiting in files P2 ticked | 22 | `Details.test.tsx` 14 to 17 and 29, `Toolbar.test.tsx` 31 to 36, `Tree.test.tsx` 22, 25, 41, 65, 73, 83, 84, 85 (the numbers of the [P2 plan's tables](2026-11-09-gpui-p2-main-screen.md#the-component-tests)), and `BottomPanel.test.tsx`'s Variables tests: "shows only while a debug run is going" and the Variables halves of "makes the three tabs draggable, and not Variables" and "allows the drop only over the three tabs". The checklist shows `BottomPanel.test.tsx` ticked, but GPUI Grace has no Variables tab (the P2 plan's Redesigns, `bottom_panel`, handed it to P4): slice 10 corrects that row |

`runner.subfolders.test.ts` holds 8 tests on `develop`, where the checklist counts 7: slice 5 ports the 8 and corrects the count. SD-01 ("variables") and SD-02 ("second") are not scenarios of Grace: they are the two `Scenario:` lines of the fixture feature in the debugger spec's Background, which the list's generator took as rows. Slice 9 marks them `n/a` with that reason; nobody using Grace could notice them.

## What is already there

P1 ported the run's core and P2 and P3 the screens, so P4 is mostly the setup modules, two new kinds of background work and their controls:

- `grace-core::karate`: `command::detect_layout` with Maven's and Gradle's compiled output after the source folders and `karate-pom.json`'s `paths` and `configDir` (`command::tests::detect_layout::{compiled_output, karate_pom_json}`), `extension_jars` (`command::tests::extension_jars`), `build_args` with `--no-pom` on every run and, for a debug run, the listener's jar, port and token (`build_args::adds_the_debug_listener_and_its_socket_to_a_debug_run_only`; the one thread is the runner's to pass), `pom::read_pom` (`pom::tests::read_pom`, 23 tests), `steps::file_steps` (`steps::tests::file_steps`), and `debug::DEBUG_LISTENER` alone of `debug.ts`.
- `grace-core::karate::process`: `spawn_karate` with `SpawnOptions::{env, verbatim_args}` and `cmd()`, which run a build tool as `cmd.exe /d /s /c "…"` on Windows, in the job object, cancelled with `taskkill /T /F`.
- `grace-core::runner`: `setup.rs`'s `ProjectSetup` trait with `NoSetup` plugged in, called where `runner.ts` lists libraries, checks the Java code, reads pom.xml's environment and offers sub-folders; `pom_start` (`setup::tests::takes_karate_pom_json_then_pom_xml`); the open project's `breakpoints` (always empty) and `watching` (as saved, nothing watched). Every place a P4 part acts is marked `P4` in `runner/mod.rs`. Missing: `steps`, `toggle_breakpoint`, `debug`, `debug_command`, `set_watch`, `build`, `open_subfolder`, the tool's `listing` events.
- `grace-core` elsewhere: `model::events` has `Breakpoint`, `DebugCommand`, `DebugVar`, `Paused`, `FileStep`, `BuildTool` and the events `breakpoints`, `debugPaused`, `debugResumed`, `watch`, `listing` and the notice's `build`; `state` reduces them (`state::tests::the_step_debugger`, `watch`, `listing_says_whether_the_build_tool_builds_the_java_code`), with `Panel::Variables`; `mask` masks a pause, an XML-escaped value and a cut end (`mask::tests::mask_cut_end`); `flaky` leaves debug runs out (`flaky::tests::…::a_debug_run_proves_nothing_a_long_pause_can_fail_a_test`); `history` writes and reads `debug: true` (`writes_debug_after_tests`); `settings` loads and saves `watch` and the saved controls; `vendor::VendorPaths::listener`; `fingerprint::{SKIPPED_DIRS, OUTPUT_DIRS}`.
- `grace`: the tree's Paused mark, the progress line's "Paused" with its clock stopped (`progress::tests::a_paused_debug_run_says_paused_turns_nothing_and_stops_the_clock`) and its "Listing the project's libraries with Maven…" and "Building the project's Java code…" lines.
- Not there: `grace-core::karate::{maven, libraries, compiled, subfolders}`, the debug session of `debug.ts`, `grace-core::watch`; in `grace`, the `Backend` calls above, the notice's Build button, the tree's offer, Debug button and breakpoint dots, the steps before a run and their toggles in Details, the Watch toggle and the debug controls in the toolbar, and the Variables tab.

## Porting rules

The rules of [P1](2026-11-09-gpui-p1-core.md#porting-rules), [P2](2026-11-09-gpui-p2-main-screen.md#porting-rules) and [P3](2026-11-12-gpui-p3-panels.md#porting-rules) hold: logic in `grace-core` and drawing in `grace`, the same words and accessible names, accessibility by hand, the outside world behind `grace::seams`, port and do not transcribe. And for P4:

1. **One process at a time.** A build tool, like Java, is the runner's one cancellable process: Cancel stops it, it dies with Grace, and while it runs the runner is busy.
2. **Nothing wakes while idle.** A watcher's threads sleep in the OS until a file changes; the 300 ms quiet time exists only between a save and its batch; the debug socket and its threads exist only during a debug run; a paused run reads nothing on a deadline (ADR 0030, step debugger R2.4).
3. **Tests never reach a real Maven or Gradle.** A test writes a stand-in wrapper into its project, as Electron Grace's do: `mvnw.cmd` or `gradlew.bat` on Windows, an executable `mvnw` or `gradlew` on Linux. Java and Karate stay real in the runner's tests, as in P1.
4. **One function per platform** (guide R5.3): finding a tool, starting it and watching files each sit behind one function with a test on each platform.
5. **A row is ticked in the slice that proves it.** A row that two slices share is ticked by the later one.

## Slices

One PR each, in this order, each `Part of #283`, each from the merged `develop`. A slice ends when its rows are ticked with their proofs, its Redesigns rows are written, and the checks of [GPUI Grace's rules](../../.claude/rules/gpui.md) pass: format, clippy, `cargo test --workspace`, the whole e2e suite on CI with its summary in the PR (the draft PR's Linux job, and `gh workflow run test.yml --ref <branch>` for Windows, which is also the Windows run by hand; never on the owner's machine, [owner-machine](../process/owner-machine.md#run-e2e-on-ci-first)), and before and after screenshots where the look changes.

| Slice | What it does | Rows that end it | Model |
|---|---|---|---|
| 1 | Tick what P1 to P3 already built, each row with the test that proves it; write the test where none does | PS-01 to PS-09, PL-01, PD-09 to PD-11; E-project-setup-01, 04, 06; `runner.setup.test.ts` | `sonnet` |
| 2 | pom.xml's `karate.env` and the pom's starting values: `grace-core::karate::maven`, `pomStart` | PD-01 to PD-08, PD-12 to PD-17; E-project-setup-02, 03; `maven.test.ts`, `pom.test.ts`'s 5, `runner.pom.test.ts` | `sonnet` |
| 3 | Find Maven and Gradle, and list a project's libraries with its own build | PL-02 to PL-10, PL-13, FB-01 to FB-06; E-project-setup-05; `libraries.test.ts`, `runner.libraries.test.ts`'s 14 | `sonnet` |
| 4 | The not-built notice and its Build button; #236 | PL-11, PL-12, BN-01 to BN-10; E-project-setup-08; `compiled.test.ts`, `runner.libraries.test.ts`'s 17; `Tree.test.tsx` 25 | `sonnet` |
| 5 | The sub-folder offer of an empty project | SO-01 to SO-08; E-project-setup-07; `subfolders.test.ts`, `runner.subfolders.test.ts`; `Tree.test.tsx` 22 | `sonnet` |
| 6 | The watcher: `grace-core::watch` on each platform's file events | WM-06, WM-07, WM-09; `watch.test.ts` | `opus` |
| 7 | Watch mode in the runner, the Watch toggle, the e2e test | WM-01 to WM-05, WM-08, WM-10 to WM-17; E-watch-01; `runner.watch.test.ts`; `Toolbar.test.tsx` 33 to 36 | `sonnet` |
| 8 | A test's steps before a run, and breakpoints in Details and the tree | SD-14 to SD-16, SC-08, GF-04; E-details-01; `runner.debug.test.ts`'s 2; `Details.test.tsx` 14, 15, 17, 29; `Tree.test.tsx` 84 | `sonnet` |
| 9 | The debug session and the debug run in `grace-core` | SD-01 to SD-13; `debug.test.ts`, `runner.debug.test.ts`'s 9 | `opus` |
| 10 | The debugger in the window: Debug, the toolbar's controls and keys, the pause, the Variables tab | SD-17 to SD-23, SC-06, TO-16; E-debug-01, E-debug-02; `Variables.test.tsx`; `Details.test.tsx` 16; `Toolbar.test.tsx` 31, 32; `Tree.test.tsx` 41, 65, 73, 83, 85; `BottomPanel.test.tsx`'s 3 | `sonnet` |
| 11 | The exit check | The regressions table; the check record | `sonnet` |

Two slices are marked hard. The watcher (6) works on two operating systems' file events, each with its own way to lose names (a buffer that overflows), to report a deleted root (Node's `fs.watch` reported the root's own path in a loop; the test decides what `ReadDirectoryChangesW` and inotify do) and to miss a renamed one, and must cost nothing while no file changes. The debug session (9) runs a socket server and two threads against a Java listener that cannot change, stops the run's events tail while paused with no deadline, and must survive Cancel and a broken socket at any point. Spec rows: 13 + 14 + 16 + 12 + 8 + 3 + 14 + 5 + 13 + 9 + 0 = 107 (103 of P4's own and the 4 handed on: SC-08 and GF-04 in slice 8, SC-06 and TO-16 in slice 10). E2e rows: 3 + 2 + 1 + 1 + 1 + 0 + 1 + 1 + 0 + 2 + 0 = 12 (E-details-01 is slice 8's). Unit tests: 9 + 36 + 43 + 29 + 17 + 13 + 18 + 2 + 17 + 4 = 188, of which 187 are ported and 1 is `n/a` (`runner.watch.test.ts` test 2, with WM-02). Tests waiting in ticked files: 1 + 1 + 4 + 5 + 11 = 22 (slices 4, 5, 7, 8 and 10).

### Slice 1: what is already built

No new behaviour is expected. Port `runner.setup.test.ts` test for test into `grace_core::runner::tests::setup` (real Java and Karate), and for each row find the Rust test that proves it and tick the row with it:

- PS-01 to PS-04: `command::tests::detect_layout::karate_pom_json` (`paths`, `configDir`, the unused keys, a file it cannot read), `build_args` (`--no-pom`), and the ported runner tests.
- PS-05: the notice on open and not on Refresh ("says it only on open").
- PS-06, PS-07, PS-09: `command::tests::detect_layout::compiled_output`; PS-08: `grace-e2e/tests/open.rs::running_one_test_of_a_maven_project_shows_its_result` (#208) and "runs one row of a Maven outline".
- PL-01: `command::tests::extension_jars` and the two `.karate/ext` runner tests.
- PD-09 to PD-11 in `pom::tests::read_pom`: PD-09 `takes_env_tags_and_threads`; PD-10 the five tests of `names_what_it_cannot_show!` (`names_the_tag_entries_of_any_of_it_cannot_show` and the four beside it), the notice's sentence in `command::tests::detect_layout::karate_pom_json::says_both_settings_first`; PD-11 `names_an_env_or_a_thread_count_grace_cannot_take`.
- `project_setup.rs` in `grace-e2e/tests`: E-project-setup-01 (karate-pom.json's tests, config and notice), 04 (Maven's compiled output), 06 (a jar in `.karate/ext`). The jar is `grace_core::report_zip::zip_report` of a folder holding `data.json` (a jar is a zip), not Windows' `tar.exe`, so the test runs on Linux too.

A row with no test gets one, written first and seen failing where the behaviour is missing. A row whose behaviour turns out not to be built goes to its feature's slice (PS-01 to PS-05 to slice 2, PS-06 to PS-09 and PL-01 to slice 3), said in the PR.

What slice 1 built: no behaviour and no Redesigns row. All of its rows were built, so none goes to a later slice. `runner::tests::setup`'s 9 tests passed at once (with the extension folders taken off the classpath, its two `.karate/ext` tests fail, so the jar they zip is what Java loads); `grace-e2e/tests/project_setup.rs` holds E-project-setup-01, 04 and 06. PS-01 and PS-05 name `env` among the pom's unused settings, which the later pom defaults spec changed (PD-01, slice 2: `env` starts the Env box); their proofs use `dryRun`. PD-09 to PD-11 and PS-07 end in opening a project, which `read_pom`'s and `detect_layout`'s tests do not do, so `runner::tests::setup` holds one test more for each (with the pom's values ignored, PD-09's and PD-10's fail). The tests, test for test:

#### `runner.setup.test.ts` (9) in `grace_core::runner::tests::setup`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. takes paths and configDir from karate-pom.json and names the rest | ported | `takes_paths_and_config_dir_from_karate_pom_json_and_names_the_rest` |
| 2. writes JUnit XML when karate-pom.json asks | ported | `writes_junit_xml_when_karate_pom_json_asks` |
| 3. says it only on open | ported | `says_it_only_on_open` |
| 4. puts Maven's compiled output on the classpath | ported | `puts_mavens_compiled_output_on_the_classpath` |
| 5. runs one row of a Maven outline | ported | `runs_one_row_of_a_maven_outline` |
| 6. runs one row of an outline in the configDir | ported | `runs_one_row_of_an_outline_in_the_config_dir` |
| 7. uses the default folders when karate-pom.json cannot be read | ported | `uses_the_default_folders_when_karate_pom_json_cannot_be_read` |
| 8. puts the jars of the project's .karate/ext on the classpath | ported | `puts_the_jars_of_the_projects_karate_ext_on_the_classpath` |
| 9. puts the jars of the home .karate/ext on the classpath | ported | `puts_the_jars_of_the_home_karate_ext_on_the_classpath` |

### Slice 2: pom.xml's environment and the pom's starting values

Specs: [pom defaults](../specs/2026-11-04-pom-defaults.md), [project setup](../specs/2026-11-02-project-setup.md).

1. `grace-core::karate::maven` from `maven.ts`: its small tag reader on `regex` (already a dependency), Surefire then Failsafe then `<properties>`, one `${name}`, never a notice. `maven.test.ts` test for test.
2. `pomStart`'s 5 tests of `pom.test.ts` in `grace_core::runner::setup::tests`, with pom.xml read through `maven::maven_env`.
3. The runner calls `maven::maven_env` where `runner.ts` does, and `ProjectSetup::maven_env` goes. Each later slice does the same with its methods (3 `list_libraries` and `kept_jars`, 4 `java_code`, 5 `karate_folders`), and slice 5 deletes the trait and `NoSetup` (Redesigns, `runner` (setup)).
4. `runner.pom.test.ts` test for test in `grace_core::runner::tests::pom`, real Java and Karate: PD-01 to PD-08, PD-12 to PD-17. PD-05 is the Properties dialog's save, which P2 left for this slice.
5. `project_setup.rs`: E-project-setup-02 (karate-pom.json's environment, filter and thread count), 03 (Maven's `karate.env`). E-project-setup-02 reads the chips' states through the accessibility tree, as `tags.rs` does; the Env and Threads boxes are checked by what they did, as `threads.rs::the_box_starts_from_karate_pom_json_and_the_persons_count_wins` does, since AT-SPI cannot read an `Input`'s text.

What slice 2 built: `grace-core::karate::maven`, which `runner::pom_start` calls when karate-pom.json gives no `env`, so a Maven project now starts with pom.xml's `karate.env`; `ProjectSetup::maven_env` and the P1 test with its stand-in `PomXml` are gone. One Redesigns row (`maven`). No row is handed on, and the look does not change. `runner::tests::pom` holds one test more for each of PD-15, PD-16 and PD-17, whose scenarios open a project, which no Vitest test did; PD-14's rows are `maven`'s tests, opened by "Surefire passes a Maven project's karate.env". Only that test failed before the runner called `maven_env`, the rest being P1's behaviour, so each was seen failing with its behaviour broken: the pom's values ignored, the saved values ignored, the environment saved with the properties, dropped tags saved, a filter change not saved, an unreadable `settings.json` read as nothing saved, an environment the pom no longer gives kept, a used `env` named in the notice, pom.xml read over karate-pom.json's `env`, pom.xml read for an unused `env`, an environment given without a pom.xml. E-project-setup-03 adds a scenario that passes only in `dev` (AT-SPI cannot read the Env box). For slice 3: the Maven and Gradle projects of `runner::tests::pom` and of E-project-setup-03 hold no stand-in wrapper, so listing their libraries must not reach a real tool (porting rule 3). The tests, test for test:

#### `maven.test.ts` (19) in `grace_core::karate::maven::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. takes the karate.env property alone | ported | `takes_the_karate_env_property_alone` |
| 2. takes Surefire's entry as written | ported | `takes_surefires_entry_as_written` |
| 3. resolves Surefire's ${karate.env} from the properties | ported | `resolves_surefires_karate_env_from_the_properties` |
| 4. resolves one ${…} inside other text | ported | `resolves_one_reference_inside_other_text` |
| 5. gives none when Surefire's property is missing, without falling back | ported | `gives_none_when_surefires_property_is_missing_without_falling_back` |
| 6. takes Failsafe's entry | ported | `takes_failsafes_entry` |
| 7. takes Surefire's entry in pluginManagement | ported | `takes_surefires_entry_in_plugin_management` |
| 8. puts Surefire before Failsafe | ported | `puts_surefire_before_failsafe` |
| 9. leaves out an execution's configuration | ported | `leaves_out_an_executions_configuration` |
| 10. leaves out a profile's properties | ported | `leaves_out_a_profiles_properties` |
| 11. leaves out a comment | ported | `leaves_out_a_comment` |
| 12. gives none for a name Grace does not take | ported | `gives_none_for_a_name_grace_does_not_take` |
| 13. gives none when elements are not well nested | ported | `gives_none_when_elements_are_not_well_nested` |
| 14. gives none for more than one ${ | ported | `gives_none_for_more_than_one_reference` |
| 15. gives none when the resolved value still holds ${ | ported | `gives_none_when_the_resolved_value_still_holds_a_reference` |
| 16. lets an empty Surefire entry decide | ported | `lets_an_empty_surefire_entry_decide` |
| 17. reads a namespaced project with a BOM and Windows line ends | ported | `reads_a_namespaced_project_with_a_bom_and_windows_line_ends` |
| 18. reads CDATA as text | ported | `reads_cdata_as_text` |
| 19. gives none without a pom.xml | ported | `gives_none_without_a_pom_xml` |

#### `pom.test.ts`'s `pomStart` (5) in `grace_core::runner::setup::tests::pom_start`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. takes pom.xml's karate.env without a karate-pom.json | ported | `takes_pom_xmls_karate_env_without_a_karate_pom_json` |
| 2. puts karate-pom.json's env first | ported | `puts_karate_pom_jsons_env_first` |
| 3. keeps pom.xml out when karate-pom.json's env is unused | ported | `keeps_pom_xml_out_when_karate_pom_jsons_env_is_unused` |
| 4. reads a karate-pom.json that cannot be read as absent | ported | `reads_a_karate_pom_json_that_cannot_be_read_as_absent` |
| 5. takes the filter and the thread count | ported | `takes_the_filter_and_the_thread_count` |

#### `runner.pom.test.ts` (12) in `grace_core::runner::tests::pom`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. karate-pom.json starts the controls, and a run passes them | ported | `karate_pom_json_starts_the_controls_and_a_run_passes_them` (also checks Java's arguments) |
| 2. what the person saved wins | ported | `what_the_person_saved_wins` |
| 3. each control decides on its own | ported | `each_control_decides_on_its_own` |
| 4. picking no environment is saved | ported | `picking_no_environment_is_saved` |
| 5. saving run properties does not save the pom's environment | ported | `saving_run_properties_does_not_save_the_poms_environment` |
| 6. a pom filter whose tags no test has is not saved | ported | `a_pom_filter_whose_tags_no_test_has_is_not_saved` |
| 7. a change in the filter bar saves the whole filter shown | ported | `a_change_in_the_filter_bar_saves_the_whole_filter_shown` |
| 8. Refresh applies an edited pom to the controls with nothing saved | ported | `refresh_applies_an_edited_pom_to_the_controls_with_nothing_saved` |
| 9. Refresh keeps the controls in force when settings.json cannot be read | ported | `refresh_keeps_the_controls_in_force_when_settings_json_cannot_be_read` |
| 10. Refresh drops a value the pom no longer gives | ported | `refresh_drops_a_value_the_pom_no_longer_gives` |
| 11. the notice names only what Grace does not use | ported | `the_notice_names_only_what_grace_does_not_use` |
| 12. Surefire passes a Maven project's karate.env | ported | `surefire_passes_a_maven_projects_karate_env` |

### Slice 3: find Maven and Gradle, list the libraries

Specs: [project libraries](../specs/2026-11-02-project-libraries.md), [find build tools](../specs/2026-11-03-find-build-tools.md).

1. `grace-core::karate::libraries` from `libraries.ts`, `libraries.test.ts` test for test: `find_tool` (wrapper, PATH, `MAVEN_HOME`, `M2_HOME`, `GRADLE_HOME`, IntelliJ IDEA's Maven newest first), `library_key` with `sha2` (already a dependency), the kept list in `libraries.json`, `parse_libraries`, `failure_reason`, the init script, `tool_env` lending the bundled Java to Maven (JM-01's "build tool's Java"), `BUILD_COMMAND`, `has_jdk`, `build_failure`. Linux's tool search and command line as Redesigns, `libraries` (Linux), says, each with its own tests; the IntelliJ search and the `.cmd` tests are Windows-only.
2. The runner: `run_tool` over `spawn_karate` (the process Cancel stops, with `listing` events before and after), `list_libraries` on open and Refresh and never on an environment switch or a run, the kept jars after the extension folders on Java's classpath. `RunnerOptions::lookup_env` is where a test points the search, as Electron Grace's `lookupEnv`.
3. `runner.libraries.test.ts`'s 14 tests outside "the project's Java code" (its first 9 and its last 5) in `grace_core::runner::tests::libraries`, each wrapper a stand-in written by the test (porting rule 3): PL-02 to PL-10, PL-13, FB-01 to FB-06.
4. `project_setup.rs`: E-project-setup-05, its stand-in `mvnw.cmd` and an executable `mvnw` on Linux, which put a `report_zip` jar on the list.

What slice 3 built: `grace-core::karate::libraries`, and the runner's `list_libraries` and `run_tool`, so GPUI Grace now lists a Maven or Gradle project's libraries with its own build on open and Refresh, shows the listing, stops it on Cancel, says why it failed, and puts the kept jars on Java's classpath; `ProjectSetup::list_libraries` and `kept_jars` are gone. The window needed nothing: P2 drew the listing line and the notice, so the look does not change. `RunnerOptions::lookup_env` is where the tool is looked for. The functions of the Build button that `libraries.test.ts` tests (`Goal::Build`, `has_jdk`, `build_failure`) are ported and have no caller until slice 4. Two Redesigns rows (`libraries` (lists), `e2e` (build tools)). No row is handed on. `runner::tests::libraries` holds one test more for each of FB-02 to FB-05, whose scenarios open a project, which `find_tool`'s tests do not do, and one for a tool that succeeds without writing a list (project libraries R6.1). The "spaces and &" test of the listing is ported here; the one of Build (`runner.libraries.test.ts`, "builds in a folder whose path has spaces and &") calls `build()` and goes with slice 4. The tests, test for test:

#### `libraries.test.ts` (29) in `grace_core::karate::libraries::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. takes the project's wrapper before PATH | ported | `find_tool::takes_the_projects_wrapper_before_path` |
| 2. searches PATH in order, mvn.cmd before mvn.bat | ported | `find_tool::searches_path_in_order_mvn_cmd_before_mvn_bat` (Windows; Linux: `linux::searches_path_at_colons_in_order`) |
| 3. finds gradlew.bat, else gradle.bat | ported | `find_tool::finds_the_gradle_wrapper_else_gradle` |
| 4. reads a quoted PATH entry as cmd.exe does | ported | `find_tool::reads_a_quoted_path_entry_as_cmd_exe_does` (Windows) |
| 5. then looks in MAVEN_HOME, then M2_HOME, and in GRADLE_HOME for Gradle | ported | `find_tool::then_looks_in_maven_home_then_m2_home_and_in_gradle_home_for_gradle` |
| 6. then takes IntelliJ IDEA's Maven, the newest build first | ported | `find_tool::then_takes_intellij_ideas_maven_the_newest_build_first` (Windows) |
| 7. finds nothing | ported | `find_tool::finds_nothing` |
| 8. names where it looked | ported | `find_tool::names_where_it_looked` |
| 9. changes with the layout's build files only | ported | `library_key::changes_with_the_layouts_build_files_only` (also pins Electron Grace's key of the same files) |
| 10. reads Gradle's version catalog | ported | `library_key::reads_gradles_version_catalog` |
| 11. round-trips | ported | `the_kept_list::round_trips` |
| 12. reads anything else as none | ported | `the_kept_list::reads_anything_else_as_none` |
| 13. is stale when missing, of another tool or key, or a jar is gone | ported | `the_kept_list::is_stale_when_missing_of_another_tool_or_key_or_a_jar_is_gone` |
| 14. gives the jars only to the same tool | ported | `the_kept_list::gives_the_jars_only_to_the_same_tool` |
| 15. passes every path through a quoted variable | ported | `the_command::passes_every_path_through_a_quoted_variable` |
| 16. keeps the person's environment and adds Grace's variables | ported | `the_command::keeps_the_persons_environment_and_adds_graces_variables` |
| 17. lends the bundled Java only when there is no other | ported | `the_command::lends_the_bundled_java_only_when_there_is_no_other` |
| 18. lends nothing without a Java to lend (Gradle needs a JDK) | ported | `the_command::lends_nothing_without_a_java_to_lend` |
| 19. keeps existing entries in order, once, less the classpath | ported | `parse_libraries::keeps_existing_entries_in_order_once_less_the_classpath` |
| 20. takes Maven's first error | ported | `failure_reason::takes_mavens_first_error` |
| 21. takes the cause under Maven's POM problems line | ported | `failure_reason::takes_the_cause_under_mavens_pom_problems_line` |
| 22. takes what Gradle says went wrong | ported | `failure_reason::takes_what_gradle_says_went_wrong` |
| 23. takes the deepest cause Gradle gives | ported | `failure_reason::takes_the_deepest_cause_gradle_gives` |
| 24. takes a launcher's ERROR line | ported | `failure_reason::takes_a_launchers_error_line` |
| 25. else the last line, cut to 300 characters | ported | `failure_reason::else_the_last_line_cut_to_300_characters` |
| 26. runs the plain compile of the test code | ported | `building::runs_the_plain_compile_of_the_test_code` |
| 27. finds a JDK by javac.exe in JAVA_HOME or on PATH | ported | `building::finds_a_jdk_by_javac_in_java_home_or_on_path` |
| 28. names javac's first error before Gradle's cause | ported | `building::names_javacs_first_error_before_gradles_cause` |
| 29. adds no stop after a question and says when a JDK is missing | ported | `building::adds_no_stop_after_a_question_and_says_when_a_jdk_is_missing` |

#### `runner.libraries.test.ts` (14 of 31) in `grace_core::runner::tests::libraries`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. runs mvnw.cmd and puts the listed jar on the classpath | ported | `runs_mvnw_and_puts_the_listed_jar_on_the_classpath` |
| 2. runs the Maven of MAVEN_HOME when the project has no wrapper | ported | `runs_the_maven_of_maven_home_when_the_project_has_no_wrapper` |
| 3. runs gradlew.bat without a daemon | ported | `runs_gradlew_without_a_daemon` |
| 4. keeps the list until a build file changes | ported | `keeps_the_list_until_a_build_file_changes` |
| 5. never lists on an environment switch or a run | ported | `never_lists_on_an_environment_switch_or_a_run` |
| 6. says when there is no tool | ported | `says_when_there_is_no_tool` |
| 7. keeps the last list when the tool fails | ported | `keeps_the_last_list_when_the_tool_fails` |
| 8. ends the notice with one full stop | ported | `ends_the_notice_with_one_full_stop` |
| 9. puts the classpath in its order | ported | `puts_the_classpath_in_its_order` (also checks Java's `-cp`) |
| 27. joins the open notice | ported | `joins_the_open_notice` |
| 28. runs the tool in a folder whose path has spaces and & | ported | `runs_the_tool_in_a_folder_whose_path_has_spaces_and_ampersands` |
| 29. says it is listing, also in a replay | ported | `says_it_is_listing_also_in_a_replay` |
| 30. stops the tool on Cancel | ported | `stops_the_tool_on_cancel` |
| 31. runs no tool for a standalone project | ported | `runs_no_tool_for_a_standalone_project` |

### Slice 4: the not-built notice and Build

Spec: [build from the notice](../specs/2026-11-04-build-from-notice.md).

1. `grace-core::karate::compiled` from `compiled.ts`, `compiled.test.ts` test for test: the newest `.java` under `src/test/java` against the newest `.class`, a JUnit runner left out (#236, PL-12), `built.json`.
2. The runner's `java_code` and `build`: the plain compile through `run_tool` with `building`, then a Refresh; "Build cancelled.", the build's first error, the JDK sentence, busy refused. `runner.libraries.test.ts`'s 17 tests under "the project's Java code" in `grace_core::runner::tests::build`: PL-11, PL-12, BN-01 to BN-10.
3. `Backend::build` and `Grace::build`; the notice's Build button, shown when `NoticeView.build` says so and off while busy (`Tree.test.tsx` 25 is the notice's test, ported where the notice is drawn).
4. `project_setup.rs`: E-project-setup-08, with the stand-in that writes a class. `screens_build_notice`.

What slice 4 built: `grace-core::karate::compiled`, the runner's `java_code` and `build`, `Backend::build`, `Grace::build` and the notice's Build button, so GPUI Grace now says on open and Refresh that a Maven or Gradle project's Java code is not built, leaving a JUnit runner out (#236), and builds it on a click with the project's own tool, then refreshes. `ProjectSetup::java_code` and `JavaCode` are gone. The look changes by the button, which follows the notice's text as Electron Grace's does: `ui::notice`'s text no longer fills the row. One Redesigns row (`compiled`). No row is handed on. `runner::tests::build` holds one test more for each of PL-12, BN-05, BN-07 and BN-10, which no Vitest test plays; `compiled::tests` one more, for a byte order mark before a runner's import and a source that is not UTF-8, both of which Node read. "never lends Grace's Java" is checked as the build tool's `JAVA_HOME` being the person's own, since on Linux the tests' Java can be that very folder. `Tree.test.tsx` 25 is `grace::window::tests::offers_build_in_a_notice_that_asks_for_it_off_while_busy`. The plan counted `compiled.test.ts` as 11 tests; it holds 12. The tests, test for test:

#### `compiled.test.ts` (12) in `grace_core::karate::compiled::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. says nothing without Java code in src/test/java | ported | `test_code_notice::says_nothing_without_java_code_in_src_test_java` |
| 2. says when the code was never built | ported | `test_code_notice::says_when_the_code_was_never_built` |
| 3. counts an empty compiled folder as not built | ported | `test_code_notice::counts_an_empty_compiled_folder_as_not_built` |
| 4. says nothing when the newest class is newer than every source | ported | `test_code_notice::says_nothing_when_the_newest_class_is_newer_than_every_source` |
| 5. says when a source changed after the last build | ported | `test_code_notice::says_when_a_source_changed_after_the_last_build` |
| 6. reads Gradle's compiled folder and names its task | ported | `test_code_notice::reads_gradles_compiled_folder_and_names_its_task` |
| 7. leaves out a JUnit runner (%s): Grace never runs it | ported | `test_code_notice::{leaves_out_a_karate_1_junit_runner, leaves_out_a_karate_2_junit_runner, leaves_out_the_parallel_junit_runner}` |
| 8. still checks a helper beside a runner | ported | `test_code_notice::still_checks_a_helper_beside_a_runner` |
| 9. says nothing for a standalone project | ported | `test_code_notice::says_nothing_for_a_standalone_project` |
| 10. counts sources older than the last Build as built | ported | `the_last_build::counts_sources_older_than_the_last_build_as_built` |
| 11. never counts a project with no class as built | ported | `the_last_build::never_counts_a_project_with_no_class_as_built` |
| 12. keeps the time in built.json | ported | `the_last_build::keeps_the_time_in_built_json` |

#### `runner.libraries.test.ts` (17 of 31, "the project's Java code") in `grace_core::runner::tests::build`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. says on open when it is not built | ported | `says_on_open_when_it_is_not_built` |
| 2. comes after the libraries' notice | ported | `comes_after_the_libraries_notice` |
| 3. says nothing once built, and says on Refresh when it changed since | ported | `says_nothing_once_built_and_says_on_refresh_when_it_changed_since` |
| 4. offers Build with the not-built notice | ported | `offers_build_with_the_not_built_notice` |
| 5. offers no Build without a tool | ported | `offers_no_build_without_a_tool` |
| 6. offers Build in a joined notice | ported | `offers_build_in_a_joined_notice` |
| 7. offers Build after the libraries' failure on Refresh | ported | `offers_build_after_the_libraries_failure_on_refresh` |
| 8. builds with test-compile, then refreshes | ported | `builds_with_test_compile_then_refreshes` |
| 9. builds in a folder whose path has spaces and & | ported | `builds_in_a_folder_whose_path_has_spaces_and_ampersands` |
| 10. says the build's first error and keeps Build | ported | `says_the_builds_first_error_and_keeps_build` |
| 11. says nothing of a JDK when JAVA_HOME has javac.exe | ported | `says_nothing_of_a_jdk_when_java_home_has_javac` |
| 12. stops the build on Cancel | ported | `stops_the_build_on_cancel` |
| 13. says it is building, also in a replay | ported | `says_it_is_building_also_in_a_replay` |
| 14. refuses a second build while one runs | ported | `refuses_a_second_build_while_one_runs` |
| 15. says when the tool is gone at the click | ported | `says_when_the_tool_is_gone_at_the_click` |
| 16. a Gradle build that rewrites no class still counts | ported | `a_gradle_build_that_rewrites_no_class_still_counts` |
| 17. runs nothing with no project or for a standalone project | ported | `runs_nothing_with_no_project_or_for_a_standalone_project` |

### Slice 5: the sub-folder offer

Spec: [sub-folder offer](../specs/2026-11-04-subfolder-offer.md).

1. `grace-core::karate::subfolders` from `subfolders.ts`, `subfolders.test.ts` test for test: 3 levels, 10 found, 2,000 folders, 2 s; no junction followed. The walk runs on its own thread and the answer is waited for 2 s at most, as the TypeScript's race with its timer did.
2. The runner: the offer in the `project` event when the tree has no test, kept in the replay; no library listing with nowhere to look for tests (SO-08); `open_subfolder` opens only a folder it offered (SO-07). `runner.subfolders.test.ts`'s 8 tests in `grace_core::runner::tests::subfolders`.
3. The tree's empty state: a button "Open <folder>/" per offer (`Tree.test.tsx` 22), through `Backend::open_subfolder`.
4. `project_setup.rs`: E-project-setup-07. `screens_subfolder_offer`.

What slice 5 built: `grace-core::karate::subfolders`, the runner's offer and `open_subfolder`, `Backend::open_subfolder`, `Grace::open_subfolder` and the empty tree's buttons, so GPUI Grace now offers the Karate projects below a folder with no test and opens the one picked. `ProjectSetup`, `NoSetup` and `RunnerOptions::setup` are gone. The look changes by the buttons under the empty tree's hint, and that tree and a dry run's error now scroll when they do not fit, as Electron Grace's do. No library listing with nowhere to look for tests (SO-08) was built in slice 3; its tests, like those of a tree with tests and of a failed dry run, passed at once, and each was seen failing with its behaviour broken. One Redesigns row (`subfolders`). No row is handed on. The checklist counted `runner.subfolders.test.ts` as 7 tests; it holds 8, so P4's unit tests are 189, not 188. `runner::tests::subfolders` holds one test more for each of SO-03 and SO-04, whose scenarios open a project, which `karate_folders`' tests do not do, one for a nested offer opened (`tests/api`) and one for an offer picked while busy; `subfolders::tests` two more, for names in `localeCompare`'s order and a read that never returns. `Tree.test.tsx` 22 is `grace::tree::tests::offers_the_folders_below_an_empty_project`, with `scrolls_to_the_offers_that_do_not_fit` beside it. The tests, test for test:

#### `subfolders.test.ts` (10) in `grace_core::karate::subfolders::tests`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. offers each kind of Karate project | ported | `offers_each_kind_of_karate_project` |
| 2. skips a build file whose src/test has no feature | ported | `skips_a_build_file_whose_src_test_has_no_feature` |
| 3. looks 3 levels down | ported | `looks_3_levels_down` |
| 4. skips tool and output folders in any case | ported | `skips_tool_and_output_folders_in_any_case` |
| 5. never offers the project folder and does not look inside an offered one | ported | `never_offers_the_project_folder_and_does_not_look_inside_an_offered_one` |
| 6. does not offer the sources of a folder with a build file | ported | `does_not_offer_the_sources_of_a_folder_with_a_build_file` |
| 7. ignores a marker that is a folder | ported | `ignores_a_marker_that_is_a_folder` |
| 8. follows no junction | ported | `follows_no_junction` (makes no link: Redesigns, `subfolders`) |
| 9. orders by level, then by path | ported | `orders_by_level_then_by_path` |
| 10. stops at the limits | ported | `stops_at_the_limits` |

#### `runner.subfolders.test.ts` (8) in `grace_core::runner::tests::subfolders`

| Vitest test | Class | Rust test or reason |
|---|---|---|
| 1. offers the Karate project below a folder with no test, without listing libraries | ported | `offers_the_karate_project_below_a_folder_with_no_test_without_listing_libraries` |
| 2. skips the listing on Refresh too | ported | `skips_the_listing_on_refresh_too` |
| 3. lists libraries when karate-pom.json paths give feature roots | ported | `lists_libraries_when_karate_pom_json_paths_give_feature_roots` |
| 4. offers nothing when the tree has tests | ported | `offers_nothing_when_the_tree_has_tests` |
| 5. offers nothing after a failed dry run | ported | `offers_nothing_after_a_failed_dry_run` |
| 6. keeps the offer in the replay | ported | `keeps_the_offer_in_the_replay` |
| 7. opens an offered folder as any project | ported | `opens_an_offered_folder_as_any_project` |
| 8. refuses a folder it did not offer | ported | `refuses_a_folder_it_did_not_offer` |

### Slice 6: the watcher

Spec: [watch mode](../specs/2026-11-03-watch-mode.md), R2.

1. `grace-core::watch` from `watch.ts`, `watch.test.ts` test for test: `counts`, one quiet window of 300 ms for all roots, the modified-time check against the start less 2 s and the time last counted, a root that is a file, every feature of the folder roots after an overflow, one error and a stop when a root is deleted, renamed or moved away (its parent watched without its subfolders), `close` freeing the parent.
2. Each platform's events as Redesigns, `watch`, says. The quiet window is a thread waiting on a channel, with a timeout only while a batch is open.
3. Windows' own tests in `watch::tests::windows`: the walk of a `FILE_NOTIFY_INFORMATION` buffer made by hand (two entries, one cut short, a name that is not ASCII); a completion of 0 bytes and one of `ERROR_NOTIFY_ENUM_DIR` both read as an overflow; `close` before the thread has issued its read, and `close` while it waits, each returning at once.
4. The idle check: a test that, with no file changing for 2 s, the watcher's threads use no CPU: `QueryThreadCycleTime` on Windows (`Win32_System_Threading`, already a feature), `voluntary_ctxt_switches` in `/proc/self/task/<tid>/status` on Linux, the watcher giving the test its threads (`#[cfg(test)]`). It runs in `cargo test` on both platforms.

### Slice 7: watch mode in the runner, and the toggle

1. The runner's `set_watch`, `arm` and `drain`: saved per project, applied during a run, refused while opening; saves waiting while busy, one run for a batch, the filter applied, a called feature skipped quietly, Cancel and turning off drop the saves, a watcher error turns watch mode off in memory with "Watch mode stopped: <reason>". `runner.watch.test.ts` in `grace_core::runner::tests::watch`: 17 tests ported; test 2 ("refuses a value that is not a boolean") is `n/a` with WM-02, which is "nothing to refuse", as OE-07: `set_watch` takes a `bool`. Rows: WM-01 to WM-05, WM-08, WM-10 to WM-16.
2. `Backend::set_watch`; the toolbar's Watch toggle after Refresh: an icon button named "Watch", pressed when on, `EyeOff` and `Eye`, its tooltip, on during a run and off during an open (`Toolbar.test.tsx` 33 to 36, and test 2's place of Watch in the toolbar's order).
3. `watch.rs` in `grace-e2e/tests`: E-watch-01 and WM-17, a save of the test's own fixture copy. `screens_watch`.
4. The idle check of the watch spec's Cost, before the PR is ready: `npm run bench:gpui -- --watch`, a new option of `bench/gpui.mjs` that seeds the data folder's `settings.json` with the bench's project as `lastProject` and `projects[<root>].watch: true`, as `bench/app.mjs`'s `startApp` seeds Electron Grace's, so the idle measures (`idle.memory`, `idle.cpu.shown`, `idle.cpu.minimised`, `idle.wakeups.minimised`) are taken with watch mode on. `test.yml`'s `gpui-idle` job gets a second step that runs it and fails above 5 wake-ups a second, as its first does; the PR's `gh workflow run test.yml --ref <branch>` runs it on a Windows VM.

### Slice 8: steps before a run, and breakpoints

Spec: [step debugger](../specs/2026-11-02-step-debugger.md), R3 and R4.

1. The runner's `steps(key)` (a scenario's or a row's steps from its feature file, through `file_steps`) and `toggle_breakpoint` (a line of a feature of the tree, the whole set sent back, replayed to a window). Their 2 tests of `runner.debug.test.ts` ("lists a test's steps before any run", "gives a reloaded window the breakpoints, and refuses a line of no feature") in `grace_core::runner::tests::debug`.
2. Details: a selected test with no run steps shows its file's steps, read once per selection off the UI thread (SD-14, `Details.test.tsx` 14, E-details-01's first half, SC-08); each step row, run or not, has its breakpoint toggle, "Add breakpoint at line N" and "Remove breakpoint at line N", with the dot (SD-15, `Details.test.tsx` 15, 17); a background step's dot in every scenario (SD-16); no toggle on Karate's synthetic step (GF-04, `Details.test.tsx` 29).
3. The tree: the dot after a feature's and a test's name while one of its lines holds a breakpoint (`Tree.test.tsx` 84).
4. `details.rs` in `grace-e2e/tests`: E-details-01 whole. `screens_breakpoints`.

Until slice 10 starts a debug run from the window, a breakpoint pauses nothing: the toggles set, show and replay breakpoints, and nothing else yet. The PR says so (the P2 plan's rule: a control that does what it says, never one that does nothing).

### Slice 9: the debug session and the debug run

1. `grace-core::karate::debug` from `debug.ts`: the session as Redesigns, `debug` (session) and `debug` (token), say; `parse_pause` into `Paused` and `DebugVar`. `debug.test.ts` test for test, its listener a socket of the test's.
2. The runner's `debug` and `debug_command`: a debug run on one thread with the listener and the session's port and token; on a pause, one last read of the events file, then no deadline until a command; `debugPaused` through the run's masker, `debugResumed` on a command; breakpoints sent on change; Cancel while paused; a socket that closes lets the run go on; the record's `debug: true` and `threads: 1`.
3. `runner.debug.test.ts`'s other 9 tests in `grace_core::runner::tests::debug`, on the `debug` fixture with the vendored `listener.jar`: SD-03 to SD-13. SD-01 and SD-02 are `n/a` (Rows to tick).
4. Added: `a_silent_connection_does_not_keep_the_listener_out` (a local connection that sends nothing, then the real listener, which is served and pauses).
5. `getrandom = "=0.3.4"` added to `grace-core`'s `Cargo.toml`, its size stated in the PR (Redesigns, `debug` (token)).

### Slice 10: the debugger in the window

1. `Backend::{debug, debug_command}`. The tree: a Debug button before Run with the bug icon and "Debug <name> (Ctrl+Shift+Enter)" (`Tree.test.tsx` 83), Ctrl+Shift+Enter on the focused row (65, SD-18; test 77's Shift variant of the shortcut's name), the run following a debug run (41), the paused test selected, its rows opened and scrolled to (73, 85, TO-16).
2. The toolbar: Pause while Java runs, Continue, Step over and Step into while paused, in place of Run all, icons below 1280 px; F5, F10 and F11 anywhere in the window while paused (`Toolbar.test.tsx` 31, 32, SD-20). Nothing turns while paused (SD-19, with the progress line's test of P2).
3. Details: the paused step marked "Paused" with its comments (`Details.test.tsx` 16, SC-06), "Paused in called.feature:5" inside a called feature (SD-21).
4. The Variables tab after HTTP during a debug run, selected by the first pause, gone with the run, not draggable and taking no drop (RP-02's second half, added to its Proof): a list of the variables by name, kind or Java type and a cut value, a `json` or `xml` value opening into GPUI Kit's selectable text, dimmed with "Running…" between pauses (SD-22). `Variables.test.tsx` test for test in `grace::variables::tests`; `BottomPanel.test.tsx`'s 3 Variables tests in `grace::bottom_panel::tests::tabs`, and that file's parity row corrected to name them (it was ticked with them missing).
5. `debug.rs` in `grace-e2e/tests`: E-debug-01 (SD-17) and E-debug-02 (SD-23), on the `debug` fixture. `screens_debugger`, paused, light and dark, and at 760 px.

### Slice 11: the exit check

1. The regression of P4, with its test:

   | Bug | What must hold | Test |
   |---|---|---|
   | #236 | A project whose only Java code is its JUnit runner is not said to be unbuilt | `grace_core::karate::compiled::tests`' runner cases and the runner's PL-12 test, both from slice 4 |

2. The debugger works with the unchanged listener jar: `git diff cf6b888 -- listener/` is empty, and E-debug-01 passes on the e2e build with `vendor/listener.jar`, which `npm run vendor` compiles from those sources. The release build has no e2e hooks: a debug run of the `debug` fixture in the packaged build (`npm run package:gpui`) is by hand, in item 3.
3. By hand on the owner's machine, each with his go ([owner-machine](../process/owner-machine.md)): watch mode's idle CPU and memory on the bench's 2,000-test project, on and off (watch mode spec, Cost), with `idle.wakeups.minimised` while watching; a debug run in the packaged build, paused for a minute with its CPU at zero, then stepped and continued; F5, F10 and F11 in a real window; real build tools as the owner answers open question 1. The idle measures with watch mode on are slice 7's on CI; here they are taken again on the reference machine.
4. `npm run bench:gpui` on the GPUI build, reported not judged (guide R7.1).
5. The record in `docs/checks/<date>-gpui-p4-exit.md`; every row of the tables above ticked or handed on by its Proof text; #278's State edited. The owner closes #283.

## Redesigns

Changes from Electron Grace's design (guide R3.6). The rows below are decided by this plan; each slice adds its own in its PR.

| Module | Change from Electron Grace | Why | Parity kept by |
|---|---|---|---|
| `runner` (setup) | `ProjectSetup` and `NoSetup` (P1's seam) go, a method per slice from 2 to 5: the runner calls `karate::{maven, libraries, compiled, subfolders}` where `runner.ts` calls them. Where a tool is looked for is `RunnerOptions::lookup_env`, as Electron Grace's `lookupEnv` | The trait stood in for modules not yet ported; with them ported it is a layer with one implementation. The runner's tests steer the search as the Vitest tests did | `runner::tests::{pom, libraries, build, subfolders}` |
| `maven` | An element nested more than 100 deep is refused, so such a pom.xml gives no environment, as one that is not well nested | Dropping a tree of elements recurses in Rust: one nested deep enough overflows the stack and ends Grace. No real pom comes near | `karate::maven::tests`, the 19 of `maven.test.ts` and `gives_none_for_elements_nested_deeper_than_any_pom` |
| `libraries` (Linux) | Windows runs the tool as Electron Grace does: `cmd.exe /d /s /c "…"`, every path in a `GRACE_*` variable, as P1 ported it in `process` (`cmd()`, `verbatim_args`): the wrappers and `mvn.cmd` are batch files, which only `cmd.exe` runs. Linux, which Electron Grace never ran on, starts `mvnw`, `mvn`, `gradlew` or `gradle` itself with an argument list, the classpath file an argument (`-Dmdep.outputFile=<file>`) and still in `GRACE_CLASSPATH_FILE` for the init script; it looks for `java` and `javac` without `.exe`, splits PATH at `:`, has no IntelliJ search, and its no-tool notice names `mvnw` and `gradlew` | The Windows strings are pinned by the libraries spec (R3) and `libraries.test.ts`; Linux has no `cmd.exe` and needs no quoting | `libraries.test.ts`'s "the command" on the same strings; `runner.libraries.test.ts`'s "runs the tool in a folder whose path has spaces and &" and "builds in a folder whose path has spaces and &"; Linux tests beside them |
| `libraries` (lists) | The Linux list is split at `:` and its newlines, `File.pathSeparator` there, which Maven and the init script join it with; two entries that differ in letter case are two files. On both platforms a launcher or wrapper is taken only when it is a file, where `existsSync` took a folder of that name too. PATH is split by `std::env::split_paths`, which on Windows keeps a `;` inside quotes in its entry, as `cmd.exe` does, where `split(";")` broke the entry there. The Linux build tool runs as Java does (nice 10, its own process group, `LC_ALL=C.UTF-8`, `process::spawn_karate`'s seam) | Linux has its own list separator and case-sensitive paths; a folder is not a program | `libraries::tests`, `libraries::tests::linux` |
| `e2e` (build tools) | The e2e build gives its runner an empty `lookup_env`, so only a project's own wrapper runs, where Electron's `launch()` took the tools' folders off PATH, dropped their home variables and pointed `ProgramFiles` and `LOCALAPPDATA` at an empty folder. The release build looks where Grace's own variables lead | On Linux `mvn` and `gradle` can share `/usr/bin` with `java` and `xdotool`, which a test cannot take off PATH | E-project-setup-03 and 05; `runner::tests::libraries` for the search |
| `compiled` | The walk for the newest `.java` and `.class` enters no linked folder, and leaves out a folder it cannot read, where a recursive `readdir` that failed anywhere read as no file at all. `built.json`'s time is read as whole milliseconds, which is all Grace writes | A link back to a parent folder never ends a walk; one unreadable folder must not hide the notice | `karate::compiled::tests`, `runner::tests::build` |
| `subfolders` | The walk is a blocking loop on a thread of its own that sends each folder as it finds it; the caller waits for it on a channel for 2 s at most, where the TypeScript raced its asynchronous walk against a timer. `follows_no_junction` makes no link: it reads one the system already has (`Documents and Settings` on Windows, `/proc/self` on Linux), then walks a folder read as that one is. On Windows a reparse point that is neither a junction nor a symbolic link, such as a OneDrive placeholder, is the file or folder it stands for, so its project is offered, where Node read every reparse point as a link | One thread that blocks and no timer (ADR 0030); the deny rules refuse a test that makes a junction or a symbolic link; a project synced by OneDrive is still a project | `karate::subfolders::tests`, `runner::tests::subfolders` |
| `watch` | Windows: `ReadDirectoryChangesW` through `windows` 0.62.2, one thread per handle. The handle is opened with `CreateFileW` (`FILE_FLAG_BACKUP_SEMANTICS`, `FILE_FLAG_OVERLAPPED`, every share mode) and read overlapped with an event; the thread waits on that event and a stop event, with no timeout. `close` sets the stop event, so a close that lands before the read is issued is not lost, then `CancelIoEx`, waits for the read to end (`GetOverlappedResult`) before the buffer is freed, and `CloseHandle`. An overflow is a read of 0 bytes or `ERROR_NOTIFY_ENUM_DIR`. The `unsafe` calls sit in one `#[cfg(windows)] mod sys` of `watch.rs`, each block with its `// SAFETY:` comment as in `process.rs`; the walk of the variable-length `FILE_NOTIFY_INFORMATION` entries is safe code over a byte slice, each offset checked against its length. Linux: `notify` 8.2.2's inotify backend, which blocks with no timeout and flags an overflow `Rescan`. Not `notify` on Windows, which the guide's feature map named: its loop waits 100 ms at a time (`WaitForSingleObjectEx(.., 100, ..)` in its `windows.rs`), 10 wake-ups a second while watch mode is on, and it reports no overflow. No new crate: `notify` is in the lock and in Grace's build through GPUI Kit, and the two `windows` features it needs (`Win32_Storage_FileSystem`, `Win32_System_IO`) are ones GPUI's Windows backend already compiles | ADR 0030 and the spec's "never a poll"; the burst test needs the overflow; an event, not a synchronous read, so a close never races the start of the call | `watch.test.ts` on both platforms; `watch::tests::windows` and the idle test (slice 6); `npm run bench:gpui -- --watch` in `gpui-idle` (slice 7) |
| `debug` (session) | Grace listens on `127.0.0.1`, port 0, with `std::net::TcpListener`: one thread accepts, and each connection reads its first line on a thread of its own, so a connection that never sends one keeps no other out, as Node served them side by side; the first with the token is kept and the others closed, and its thread then reads the listener's pause lines; a write is a line of JSON, as `debug.ts`'s. The protocol and `listener/grace/DebugListener.java` do not change | One process, no event loop: threads blocked on the socket cost nothing while paused (R2.4) | `debug.test.ts`, `runner.debug.test.ts`, `a_silent_connection_does_not_keep_the_listener_out`, E-debug-01 on the unchanged jar |
| `debug` (token) | The token's 16 random bytes come from `getrandom` 0.3.4, added in slice 9 as a direct dependency of `grace-core`, where Node had `randomBytes`. It is the version GPUI's `rand` links on both platforms (0.4.3 is in the lock too, through AccessKit's `uuid`). The pin follows the lock, as `sha2`'s and `base64`'s follow the versions GPUI links: a weekly bump (ADR 0032) that moves GPUI's `rand` moves it too, and if 0.3 leaves the lock it moves to the version left | `std` has no random source; no new crate | `debug.test.ts`'s wrong-token test (SD-08) |

## Open questions for the owner

1. **Real build tools by hand.** CI and the tests use stand-in wrappers (porting rule 3). For slice 11, does the owner's machine have Maven, Gradle, a JDK or IntelliJ IDEA's Maven, for one real listing and Build each, and which of them should slice 11 run?
2. **#283's Karate surface.** It names `-p`, the pom file. Grace passes `--no-pom` to every dry run and run (pom defaults R1.5, `build_args`) and never `-p`; P4 adds neither. Should the issue's line say `--no-pom`?

## Traps

- Headless `#[gpui::test]` windows build no accessibility tree: names, roles and states are e2e tests (`crates/grace-e2e`).
- The e2e driver cannot scroll a list or focus one without clicking a row, which opens it: keys that need a focused list with nothing open (Ctrl+Shift+Enter on a row, the breakpoint toggles by keyboard) are headless view tests.
- On Linux AT-SPI drops a button's disabled state and a row's level and expanded state: an e2e test of Build off while busy, or of a paused feature opened, checks what the control did.
- E2e runs on CI, not on the owner's machine ([owner-machine](../process/owner-machine.md#run-e2e-on-ci-first)): the draft PR's Linux job, and `gh workflow run test.yml --ref <branch>` for Windows. A run there needs his go, and then no Karate beside it but the tests' own fixtures (`debug`, `maven`, `clean`) and no second e2e run (AGENTS.md, #191).
- AT-SPI cannot read the text in a GPUI Kit `Input`, and `App::wait_for_text` reads it on Windows only: on Linux an e2e test checks what the Env or Threads box did.
- `line_clamp` and text with a newline panic GPUI's single-line shaping: a variable's value, a build's error and a step's text are one line each where they are cut.
- The e2e suite runs on Linux too: each `mvnw.cmd` or `gradlew.bat` stand-in needs an executable `mvnw` or `gradlew` twin, written by the test (no shell script in the repository).
- A blocking `accept` does not return when its listener is dropped: the session wakes it at the run's end, or a run that the listener never reached leaves a thread behind.
- On Windows a watched folder's parents cannot be renamed or deleted while it is watched: a test removes its temporary folder only after `close`.
- `runner/mod.rs`, `toolbar`, `details` and `tree` are touched by most slices: one slice at a time, each from the merged `develop`.
- Commit headers at most 72 characters, checked before the first push: agents cannot force-push. A `Closes #283` in a commit would close the issue: write `Part of #283`.

## Exit check (from #283)

The parity rows of phase P4 ticked; the debugger works with the unchanged listener jar.

## Cost

- **Idle, watch mode off.** Nothing: no handle, thread, socket or timer.
- **Idle, watch mode on.** On Windows one handle and one blocked thread per feature root and per folder root's parent (four for a Maven project), and no wake-up until a file changes; no walk of the project at start. On Linux inotify needs a watch per folder, so starting walks the feature roots once. A 300 ms wait only between a save and its batch.
- **Opening a project.** A Maven or Gradle project runs its build tool once when its build files changed since the last listing, then keeps the list in `libraries.json`; the tool is a process like Java, cancelled as Java is. The test code check reads the `.java` files and the `.class` times on open and Refresh. An empty tree walks at most 2,000 folders for 2 s for the sub-folder offer.
- **During a debug run.** One listening socket, then one connection and two threads, and Java on one thread with `listener.jar`. While paused, nothing: the threads and Java are blocked on the socket and the events file is not read. A normal run gets no listener, socket or argument.
- **Size.** No new crate. `grace-core` gains `notify` (Linux) and `getrandom` as direct dependencies, both already in Grace.exe, and two `windows` features GPUI already compiles.
