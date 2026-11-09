// `src/renderer/reduce.test.ts`, test for test, on the same fixtures; then `store.test.tsx`'s tests of phase P2
// (`toggleGroup` ports with the HTTP tab in P3).
use super::*;
use crate::karate::events::read_all_events;
use crate::karate::run_state::RunState;
use crate::karate::tree::{build_tree, leaf_keys};
use crate::model::env::PropertyListsView;
use crate::model::events::{Paused, ScenarioUpdate};
use crate::model::node::Layout;
use indexmap::IndexMap;
use serde_json::json;
use std::path::Path;
use std::sync::LazyLock;

fn fixture(name: &str) -> Vec<crate::karate::events::KarateEvent> {
    read_all_events(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures/events")
            .join(name),
    )
    .unwrap()
}

static TREE: LazyLock<TestNode> = LazyLock::new(|| build_tree(&fixture("dryrun-clean.jsonl"), "clean").unwrap());
static KEYS: LazyLock<Vec<String>> = LazyLock::new(|| leaf_keys(&TREE));

fn keys() -> &'static [String] {
    &KEYS
}

fn p() -> Project {
    Project {
        root: "D:/p".into(),
        layout: Layout::Standalone,
    }
}

fn other() -> Project {
    Project {
        root: "D:/q".into(),
        layout: Layout::Standalone,
    }
}

fn project_with(project: Option<Project>, tree: Option<TestNode>, error: Option<&str>) -> AppEvent {
    AppEvent::Project {
        project,
        tree,
        error: error.map(str::to_string),
        subfolders: None,
    }
}

fn project() -> AppEvent {
    project_with(Some(p()), Some(TREE.clone()), None)
}

fn project_tree(t: Option<TestNode>) -> AppEvent {
    project_with(Some(p()), t, None)
}

fn not_found() -> AppEvent {
    project_with(None, None, Some("Folder not found"))
}

fn started_with(ks: &[String], env: Option<&str>) -> AppEvent {
    AppEvent::RunStarted {
        selection: TREE.key.clone(),
        run_id: "20261025-120000-000".into(),
        keys: ks.to_vec(),
        total: ks.len() as u32,
        started_at: 1000.0,
        env: env.map(str::to_string),
        debug: false,
    }
}

fn started(ks: &[String]) -> AppEvent {
    started_with(ks, None)
}

fn apply(s: &mut AppState, es: impl IntoIterator<Item = AppEvent>) {
    for e in es {
        s.reduce(&e);
    }
}

fn env_view(env: Option<&str>) -> EnvView {
    EnvView {
        env: env.map(str::to_string),
        suggestions: vec![],
        overlays: vec![],
        properties: PropertyListsView {
            shared: vec![],
            by_env: IndexMap::new(),
        },
        secrets_available: true,
    }
}

fn environment(env: Option<&str>) -> AppEvent {
    AppEvent::Environment { view: env_view(env) }
}

fn entry(id: &str) -> HistoryEntry {
    serde_json::from_value(json!({
        "id": id, "startedAt": 1000, "durationMillis": 2000, "env": null, "threads": 1,
        "filter": {"include": [], "exclude": [], "hideNonMatching": false}, "selection": null, "failedOf": null,
        "outcome": "finished", "exitCode": 0, "passed": 1, "failed": 0, "total": 1, "reportAvailable": true
    }))
    .unwrap()
}

fn history(runs: Vec<HistoryEntry>) -> AppEvent {
    AppEvent::History { runs }
}

fn a_call() -> HttpCall {
    serde_json::from_value(json!({
        "method": "GET", "url": "http://localhost/hello", "requestHeaders": [], "requestBody": null, "status": 200,
        "responseHeaders": [], "responseBody": null, "millis": 5, "error": null, "via": null, "startTime": 1,
        "attempt": null
    }))
    .unwrap()
}

fn update(key: &str, status: Status) -> AppEvent {
    AppEvent::Scenarios {
        updates: vec![ScenarioUpdate {
            key: key.into(),
            status,
            duration_millis: None,
            error: None,
            reason_at: None,
        }],
    }
}

/// The run of `run-clean.jsonl` as the runner reports it, then its end.
fn whole_run() -> Vec<AppEvent> {
    let mut rs = RunState::new(&TREE, leaf_keys(&TREE));
    let mut out = rs.apply(&fixture("run-clean.jsonl")).unwrap();
    out.extend(rs.finish(Some(1), false, true));
    out
}

/// A copy of the tree where the node `key` has another name: a scenario inserted above shifted the keys.
fn renamed(key: &str) -> TestNode {
    fn rename(n: &mut TestNode, key: &str) {
        if n.key == key {
            n.name = "inserted".into();
        }
        for c in &mut n.children {
            rename(c, key);
        }
    }
    let mut t = TREE.clone();
    rename(&mut t, key);
    t
}

/// The statuses, results, number of steps, selection and run: what a project for another folder resets.
type RunViewParts = (
    HashMap<String, Status>,
    HashMap<String, TestResult>,
    usize,
    Option<String>,
    RunView,
);

/// What a project for another folder resets.
fn run_view(s: &AppState) -> RunViewParts {
    (
        s.statuses.clone(),
        s.results.clone(),
        s.steps.len(),
        s.selected.clone(),
        s.run.clone(),
    )
}

fn reset() -> RunViewParts {
    (HashMap::new(), HashMap::new(), 0, None, RunView::default())
}

mod reduce {
    use super::*;

    #[test]
    fn run_started_sets_selection_to_pending_keeps_others_clears_console_sets_total() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        s.statuses.insert("other".into(), Status::Passed);
        s.console.push_back("old".into());
        apply(&mut s, [started(&keys()[..2])]);
        assert_eq!(s.statuses[&keys()[0]], Status::Pending);
        assert_eq!(s.statuses[&keys()[1]], Status::Pending);
        assert_eq!(s.statuses["other"], Status::Passed);
        assert!(s.console.is_empty());
        assert!(s.run.active);
        assert_eq!((s.run.total, s.run.done, s.run.started_at), (2, 0, Some(1000.0)));
    }

    #[test]
    fn keeps_the_runs_starting_key_224_r3_1() {
        let mut s = AppState::new();
        let mut e = started(keys());
        if let AppEvent::RunStarted { selection, .. } = &mut e {
            *selection = keys()[1].clone();
        }
        apply(&mut s, [project(), e]);
        assert_eq!(s.run.selection.as_deref(), Some(keys()[1].as_str()));
    }

    #[test]
    fn lists_the_tests_that_fail_in_the_run_once_each_in_order_224_r3_3() {
        let mut s = AppState::new();
        apply(&mut s, [project(), started(&keys()[..3])]);
        apply(
            &mut s,
            [
                update(&keys()[1], Status::Failed),
                update(&keys()[0], Status::Passed),
                update(&keys()[2], Status::Failed),
                update(&keys()[1], Status::Failed),
            ],
        );
        assert_eq!(s.run.failed_keys, vec![keys()[1].clone(), keys()[2].clone()]);
        apply(&mut s, [started(&keys()[..3])]);
        assert!(s.run.failed_keys.is_empty());
    }

    #[test]
    fn replays_a_whole_run_done_9_passed_8_failed_1_outcome_finished() {
        let mut s = AppState::new();
        apply(&mut s, [project(), started(keys())]);
        apply(&mut s, whole_run());
        assert!(!s.run.active);
        assert_eq!((s.run.done, s.run.passed, s.run.failed), (9, 8, 1));
        assert_eq!(s.run.outcome, Some(RunOutcome::Finished));
        assert!(s.run.report_available);
        assert!(s.run.summary.is_some());
        assert!(!s.steps.is_empty());
        assert!(!s.calls.is_empty());
        assert!(s.results.values().any(|r| r.error.is_some()));
    }

    #[test]
    fn stores_calls_per_test() {
        let mut s = AppState::new();
        apply(
            &mut s,
            [
                project(),
                AppEvent::Calls {
                    key: keys()[0].clone(),
                    calls: vec![a_call()],
                },
                AppEvent::Calls {
                    key: keys()[1].clone(),
                    calls: vec![],
                },
            ],
        );
        let want: HashMap<String, Vec<HttpCall>> =
            [(keys()[0].clone(), vec![a_call()]), (keys()[1].clone(), vec![])].into();
        assert_eq!(s.calls, want);
    }

    #[test]
    fn run_started_clears_the_calls_of_the_tests_it_runs_and_keeps_the_others() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        s.calls.insert(keys()[0].clone(), vec![a_call()]);
        s.calls.insert(keys()[1].clone(), vec![a_call()]);
        apply(&mut s, [started(&keys()[..1])]);
        assert!(!s.calls.contains_key(&keys()[0]));
        assert_eq!(s.calls[&keys()[1]], vec![a_call()]);
    }

    #[test]
    fn run_started_drops_the_selected_call_of_a_test_it_runs() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        s.calls.insert(keys()[0].clone(), vec![a_call()]);
        s.selected_call = Some(CallRef {
            key: keys()[0].clone(),
            index: 0,
        });
        apply(&mut s, [started(&keys()[..1])]);
        assert_eq!(s.selected_call, None);
        // a run of other tests keeps the bottom tab and the selected call
        s.calls.insert(keys()[1].clone(), vec![a_call()]);
        let kept = CallRef {
            key: keys()[1].clone(),
            index: 0,
        };
        s.selected_call = Some(kept.clone());
        s.panel = Panel::Http;
        apply(&mut s, [started(&keys()[..1])]);
        assert_eq!(s.panel, Panel::Http);
        assert_eq!(s.selected_call, Some(kept));
    }

    #[test]
    fn a_new_project_and_an_environment_change_clear_calls_and_the_selected_call() {
        let mut s = AppState::new();
        let call = CallRef {
            key: keys()[0].clone(),
            index: 0,
        };
        let groups = vec![HttpGroup::ClientError, HttpGroup::NoResponse];
        let filled = |s: &mut AppState| {
            s.calls.insert(keys()[0].clone(), vec![a_call()]);
            s.selected_call = Some(call.clone());
            s.http_groups = groups.clone();
        };
        let cleared = |s: &AppState| {
            assert!(s.calls.is_empty());
            assert_eq!(s.selected_call, None);
            assert_eq!(s.http_groups, groups); // the chips last for the session
        };
        apply(&mut s, [project(), environment(Some("qa"))]);
        filled(&mut s);
        apply(&mut s, [environment(Some("qa"))]); // the same environment keeps them
        assert_eq!(s.calls[&keys()[0]], vec![a_call()]);
        assert_eq!(s.selected_call, Some(call.clone()));
        apply(&mut s, [environment(Some("prod"))]);
        cleared(&s);
        filled(&mut s);
        apply(&mut s, [environment(None)]);
        cleared(&s);
        filled(&mut s);
        apply(&mut s, [project_with(Some(other()), Some(TREE.clone()), None)]);
        cleared(&s);
        filled(&mut s);
        apply(&mut s, [not_found()]);
        cleared(&s);
    }

    #[test]
    fn a_key_that_no_longer_names_the_same_test_loses_its_calls() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        let (same, moved) = (keys()[0].clone(), keys()[1].clone());
        for k in [same.as_str(), moved.as_str(), "gone"] {
            s.calls.insert(k.into(), vec![a_call()]);
        }
        let at = |key: &str| CallRef {
            key: key.into(),
            index: 0,
        };
        s.selected_call = Some(at(&same));
        apply(&mut s, [project()]); // a refresh keeps the selected call
        assert_eq!(s.selected_call, Some(at(&same)));
        s.selected_call = Some(at(&moved));
        apply(&mut s, [project_tree(Some(renamed(&moved)))]);
        let want: HashMap<String, Vec<HttpCall>> = [(same, vec![a_call()])].into();
        assert_eq!(s.calls, want);
        assert_eq!(s.selected_call, None);
    }

    #[test]
    fn does_not_double_count_a_repeated_final_status() {
        let mut s = AppState::new();
        apply(&mut s, [project(), started(keys())]);
        let up = update(&keys()[0], Status::Passed);
        apply(&mut s, [up.clone(), up]);
        assert_eq!((s.run.done, s.run.passed), (1, 1));
    }

    #[test]
    fn counts_the_tests_a_cancelled_run_interrupted_once_each_and_apart_from_done() {
        let mut s = AppState::new();
        apply(&mut s, [project(), started(keys())]);
        assert_eq!(s.run.interrupted, 0);
        let up = update(&keys()[0], Status::Interrupted);
        apply(&mut s, [update(&keys()[0], Status::Running), up.clone(), up]);
        assert_eq!(
            (s.run.done, s.run.passed, s.run.failed, s.run.interrupted),
            (0, 0, 0, 1)
        );
        apply(&mut s, [started(&keys()[1..2])]); // the next run starts from none
        assert_eq!(s.run.interrupted, 0);
    }

    #[test]
    fn project_event_keeps_statuses_of_existing_keys_and_drops_the_others() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        s.statuses.insert(keys()[0].clone(), Status::Passed);
        s.statuses.insert("gone".into(), Status::Failed);
        s.results.insert(
            "gone".into(),
            TestResult {
                duration_millis: Some(1.0),
                ..TestResult::default()
            },
        );
        s.steps.insert("gone".into(), vec![]);
        s.notice = Some(NoticeView {
            text: "x".into(),
            success: false,
            build: false,
        });
        apply(&mut s, [project()]);
        assert_eq!(s.statuses[&keys()[0]], Status::Passed);
        assert!(!s.statuses.contains_key("gone"));
        assert!(!s.results.contains_key("gone"));
        assert!(!s.steps.contains_key("gone"));
        assert_eq!(s.notice, None);
    }

    #[test]
    fn project_event_drops_the_status_of_a_key_that_now_names_another_test() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        let (same, moved) = (keys()[0].clone(), keys()[1].clone());
        let boom = TestResult {
            error: Some("boom".into()),
            ..TestResult::default()
        };
        for k in [&same, &moved] {
            s.statuses.insert(k.clone(), Status::Failed);
            s.results.insert(k.clone(), boom.clone());
            s.steps.insert(k.clone(), vec![]);
        }
        s.selected = Some(moved.clone());
        // a scenario inserted above shifts section numbers: `moved` is now another scenario
        apply(&mut s, [project_tree(Some(renamed(&moved)))]);
        assert_eq!(s.statuses[&same], Status::Failed);
        assert_eq!(s.results[&same], boom);
        assert_eq!(s.steps[&same], vec![]);
        assert!(!s.statuses.contains_key(&moved));
        assert!(!s.results.contains_key(&moved));
        assert!(!s.steps.contains_key(&moved));
        assert_eq!(s.selected, None);
    }

    #[test]
    fn project_event_for_another_folder_resets_the_run_view() {
        let mut s = AppState::new();
        apply(&mut s, [project(), started(keys())]);
        apply(&mut s, whole_run());
        s.selected = Some(keys()[0].clone());
        apply(&mut s, [project_with(Some(other()), Some(TREE.clone()), None)]);
        assert_eq!(run_view(&s), reset());
        apply(&mut s, [started(keys()), not_found()]);
        assert_eq!(run_view(&s), reset());
    }

    #[test]
    fn project_event_keeps_a_selection_that_still_exists_and_clears_one_that_is_gone() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        s.selected = Some(keys()[0].clone());
        apply(&mut s, [project()]);
        assert_eq!(s.selected.as_ref(), Some(&keys()[0]));
        s.selected = Some("gone".into());
        apply(&mut s, [project()]);
        assert_eq!(s.selected, None);
        s.selected = Some(keys()[0].clone());
        apply(&mut s, [project_tree(None)]);
        assert_eq!(s.selected, None);
    }

    mod environment {
        use super::*;

        /// A finished run with a selected test: everything an environment change must clear.
        fn with_results() -> AppState {
            let mut s = AppState::new();
            apply(&mut s, [project(), environment(Some("qa")), started(keys())]);
            apply(&mut s, whole_run());
            s.selected = Some(keys()[0].clone());
            s
        }

        #[test]
        fn env_change_clears_results() {
            let mut s = with_results();
            assert!(s.run.report_available);
            apply(&mut s, [environment(Some("prod"))]);
            assert_eq!(run_view(&s), reset());
            assert!(!s.run.report_available);
            assert_eq!(s.env.as_ref().unwrap().env.as_deref(), Some("prod"));
            apply(&mut s, [started(keys()), update(&keys()[0], Status::Passed)]);
            apply(&mut s, [environment(None)]); // to no environment, too
            assert_eq!(run_view(&s), reset());
        }

        #[test]
        fn same_env_keeps_them() {
            let mut s = with_results();
            let before = run_view(&s);
            apply(&mut s, [environment(Some("qa"))]); // re-sent after a refusal or a properties save
            assert_eq!(run_view(&s), before);
            assert_eq!(s.env.as_ref().unwrap().env.as_deref(), Some("qa"));
        }

        #[test]
        fn keeps_the_filter_and_the_console_when_the_environment_changes() {
            let mut s = with_results();
            s.filter = FilterState {
                include: vec!["smoke".into()],
                exclude: vec![],
                hide_non_matching: true,
            };
            s.console = VecDeque::from(["line".to_string()]);
            apply(&mut s, [environment(Some("prod"))]);
            assert_eq!(s.filter.include, vec!["smoke".to_string()]);
            assert_eq!(s.console, VecDeque::from(["line".to_string()]));
        }

        #[test]
        fn is_dropped_by_another_folder_or_none_and_ignored_with_no_project() {
            let mut s = with_results();
            apply(&mut s, [project(), environment(Some("qa"))]); // same folder refreshed
            assert_eq!(s.env.as_ref().unwrap().env.as_deref(), Some("qa"));
            apply(&mut s, [project_with(Some(other()), Some(TREE.clone()), None)]);
            assert_eq!(s.env, None);
            apply(&mut s, [project(), environment(Some("qa")), not_found()]);
            assert_eq!(s.env, None);
            apply(&mut s, [environment(Some("qa"))]); // the runner can still hold the previous project's view
            assert_eq!(s.env, None);
        }

        #[test]
        fn run_started_records_the_environment_of_the_run() {
            let mut s = AppState::new();
            apply(
                &mut s,
                [project(), environment(Some("qa")), started_with(keys(), Some("qa"))],
            );
            assert_eq!(s.run.env.as_deref(), Some("qa"));
            apply(&mut s, [started(keys())]);
            assert_eq!(s.run.env, None);
            assert_eq!(AppState::new().run.env, None);
        }
    }

    mod history {
        use super::*;

        const RUN: &str = "20261025-120000-000";

        /// A finished run whose report exists, as the window holds it after runFinished.
        fn finished() -> AppState {
            let mut s = AppState::new();
            apply(
                &mut s,
                [
                    project(),
                    started(keys()),
                    AppEvent::RunFinished {
                        outcome: RunOutcome::Finished,
                        exit_code: Some(0),
                        summary: None,
                        report_available: true,
                    },
                ],
            );
            s
        }

        fn ids(s: &AppState) -> Vec<&str> {
            s.history.iter().map(|r| r.id.as_str()).collect()
        }

        #[test]
        fn history_event_replaces_the_list() {
            let mut s = AppState::new();
            assert!(s.history.is_empty());
            assert_eq!(s.panel, Panel::Console);
            apply(&mut s, [project(), history(vec![entry("a"), entry("b")])]);
            assert_eq!(ids(&s), ["a", "b"]);
            apply(&mut s, [history(vec![entry("c")])]);
            assert_eq!(ids(&s), ["c"]);
        }

        #[test]
        fn run_started_keeps_the_bottom_tab_and_records_the_run_id() {
            let mut s = AppState::new();
            assert_eq!(s.run.run_id, None);
            s.panel = Panel::History;
            apply(&mut s, [project(), started(keys())]);
            assert_eq!(s.panel, Panel::History);
            assert_eq!(s.run.run_id.as_deref(), Some(RUN));
        }

        #[test]
        fn clearing_the_history_disables_report() {
            let mut s = finished();
            assert!(s.run.report_available);
            apply(&mut s, [history(vec![entry(RUN), entry("20261025-110000-000")])]);
            assert!(s.run.report_available); // its folder is still listed with a report
            apply(&mut s, [history(vec![entry("20261025-110000-000")])]);
            assert!(!s.run.report_available); // retention dropped it
            let mut t = finished();
            let mut no_report = entry(RUN);
            no_report.report_available = false;
            apply(&mut t, [history(vec![no_report])]);
            assert!(!t.run.report_available);
            let mut u = finished();
            apply(&mut u, [history(vec![])]);
            assert!(!u.run.report_available);
        }

        #[test]
        fn another_project_empties_the_history() {
            let mut s = AppState::new();
            apply(&mut s, [project(), history(vec![entry("a")])]);
            apply(&mut s, [project()]);
            assert_eq!(s.history.len(), 1); // a refresh of the same project keeps it
            apply(&mut s, [project_with(Some(other()), Some(TREE.clone()), None)]);
            assert!(s.history.is_empty());
            apply(&mut s, [history(vec![entry("a")]), not_found()]);
            assert!(s.history.is_empty());
        }
    }

    fn chunk(from: usize, to: usize) -> AppEvent {
        AppEvent::Console {
            lines: (from..to).map(|i| format!("l{i}")).collect(),
        }
    }

    #[test]
    fn console_keeps_the_last_10000_lines() {
        let mut s = AppState::new();
        apply(&mut s, [chunk(0, 4000), chunk(4000, 8000), chunk(8000, 10500)]);
        assert_eq!(s.console.len(), CONSOLE_LIMIT);
        assert_eq!(s.console.back().unwrap(), "l10499");
        assert_eq!(s.console[0], "l500");
    }

    #[test]
    fn counts_the_console_lines_dropped_at_the_cap_or_by_a_run_from_0_at_project_close() {
        let mut s = AppState::new();
        apply(&mut s, [project()]);
        assert_eq!(s.console_dropped, 0);
        apply(&mut s, [chunk(0, CONSOLE_LIMIT + 5)]);
        apply(
            &mut s,
            [AppEvent::Console {
                lines: vec!["a".into(), "b".into()],
            }],
        );
        assert_eq!(s.console_dropped, 7);
        assert_eq!(s.console[0], "l7"); // a line's number is console_dropped + its index
        // a new run's lines take new numbers: a place held in the last run never lands on one of them
        apply(&mut s, [started(keys())]);
        assert_eq!(s.console_dropped, 7 + CONSOLE_LIMIT as u64);
        apply(&mut s, [not_found()]);
        assert_eq!(s.console_dropped, 0);
    }

    #[test]
    fn busy_follows_the_runner_and_a_finished_run_ends_it() {
        let mut s = AppState::new();
        assert!(!s.busy);
        apply(&mut s, [AppEvent::Busy { busy: true }]);
        assert!(s.busy);
        apply(&mut s, [AppEvent::Busy { busy: false }]);
        assert!(!s.busy);
        apply(&mut s, [project(), AppEvent::Busy { busy: true }, started(keys())]);
        apply(
            &mut s,
            RunState::new(&TREE, leaf_keys(&TREE)).finish(Some(1), true, false),
        );
        assert!(!s.busy);
    }

    #[test]
    fn notice_sets_the_notice_a_warning_unless_main_says_it_reports_a_success() {
        let mut s = AppState::new();
        let notice = |text: &str, success, build| NoticeView {
            text: text.into(),
            success,
            build,
        };
        apply(&mut s, [AppEvent::notice("hi")]);
        assert_eq!(s.notice, Some(notice("hi", false, false)));
        apply(
            &mut s,
            [AppEvent::Notice {
                message: "Updated to 1.0.0-rc.8.".into(),
                success: Some(true),
                build: false,
            }],
        );
        assert_eq!(s.notice, Some(notice("Updated to 1.0.0-rc.8.", true, false)));
        apply(
            &mut s,
            [AppEvent::Notice {
                message: "Not built.".into(),
                success: None,
                build: true,
            }],
        );
        assert_eq!(s.notice, Some(notice("Not built.", false, true)));
    }

    #[test]
    fn listing_says_whether_the_build_tool_builds_the_java_code() {
        let mut s = AppState::new();
        apply(
            &mut s,
            [AppEvent::Listing {
                tool: Some(BuildTool::Maven),
                build: true,
            }],
        );
        assert_eq!((s.listing, s.building), (Some(BuildTool::Maven), true));
        apply(
            &mut s,
            [AppEvent::Listing {
                tool: None,
                build: false,
            }],
        );
        assert_eq!((s.listing, s.building), (None, false));
    }

    mod home_screen {
        use super::*;

        fn opening(root: Option<&str>) -> AppEvent {
            AppEvent::Opening {
                root: root.map(str::to_string),
            }
        }

        #[test]
        fn starts_not_loaded_with_nothing_opening_and_no_list() {
            let s = AppState::new();
            assert_eq!((s.loaded, s.opening.clone(), s.recent.clone()), (false, None, None));
        }

        #[test]
        fn an_opening_event_sets_opening_and_loaded_and_null_clears_opening() {
            let mut s = AppState::new();
            apply(&mut s, [opening(Some("D:/p"))]);
            assert_eq!((s.opening.as_deref(), s.loaded), (Some("D:/p"), true));
            apply(&mut s, [opening(None)]);
            assert_eq!((s.opening.as_deref(), s.loaded), (None, true));
        }

        #[test]
        fn opening_a_folder_with_no_project_open_drops_the_last_opens_error() {
            let mut s = AppState::new();
            apply(&mut s, [project_with(None, None, Some("Folder not found: D:/x"))]);
            apply(&mut s, [opening(None)]);
            assert_eq!(s.error.as_deref(), Some("Folder not found: D:/x"));
            apply(&mut s, [opening(Some("D:/p"))]);
            assert_eq!(s.error, None);
        }

        #[test]
        fn opening_another_folder_keeps_the_open_projects_error() {
            let mut s = AppState::new();
            apply(&mut s, [project_with(Some(p()), None, Some("syntax"))]);
            apply(&mut s, [opening(Some("D:/q"))]);
            assert_eq!(s.error.as_deref(), Some("syntax"));
        }

        #[test]
        fn a_project_event_sets_loaded_even_a_null_project() {
            let mut s = AppState::new();
            apply(&mut s, [project_with(None, None, None)]);
            assert!(s.loaded);
        }

        #[test]
        fn a_recent_event_stores_the_entries() {
            let mut s = AppState::new();
            let entries = vec![
                RecentEntry {
                    path: "D:\\work\\api".into(),
                    exists: true,
                },
                RecentEntry {
                    path: "E:\\usb\\old".into(),
                    exists: false,
                },
            ];
            apply(
                &mut s,
                [AppEvent::Recent {
                    entries: entries.clone(),
                }],
            );
            assert_eq!(s.recent, Some(entries));
        }

        #[test]
        fn a_null_project_empties_the_console() {
            let mut s = AppState::new();
            apply(
                &mut s,
                [
                    project(),
                    AppEvent::Console {
                        lines: vec!["a".into(), "b".into()],
                    },
                ],
            );
            apply(&mut s, [project_with(None, None, None)]);
            assert!(s.console.is_empty());
        }

        #[test]
        fn a_project_for_another_folder_keeps_the_console() {
            let mut s = AppState::new();
            apply(
                &mut s,
                [
                    project(),
                    AppEvent::Console {
                        lines: vec!["a".into()],
                    },
                ],
            );
            apply(&mut s, [project_with(Some(other()), Some(TREE.clone()), None)]);
            assert_eq!(s.console, VecDeque::from(["a".to_string()]));
        }
    }

    mod updates {
        use super::*;

        fn release() -> Value {
            json!({
                "version": "1.0.0-rc.6", "prerelease": true, "publishedAt": "2026-10-29T10:00:00Z",
                "url": "https://github.com/hpagniez/grace/releases/download/v1.0.0-rc.6/Grace-win32-x64-1.0.0-rc.6.zip",
                "size": 218_557_130, "sha256": "a".repeat(64)
            })
        }

        #[test]
        fn start_empty() {
            let s = AppState::new();
            assert_eq!((s.updates, s.update_prompt, s.update_progress), (None, None, None));
        }

        #[test]
        fn each_event_sets_its_state_and_a_project_event_keeps_them() {
            let mut s = AppState::new();
            let view = json!({"running": "1.0.0-rc.5", "packaged": false, "releases": [release()]});
            let progress = json!({"version": "1.0.0-rc.6", "rollback": false, "error": "boom"});
            apply(
                &mut s,
                [
                    AppEvent::Updates { view: view.clone() },
                    AppEvent::UpdatePrompt { release: release() },
                    AppEvent::UpdateProgress {
                        progress: Some(progress.clone()),
                    },
                    project(),
                ],
            );
            assert_eq!(s.updates, Some(view));
            assert_eq!(s.update_prompt, Some(release()));
            assert_eq!(s.update_progress, Some(progress));
            apply(&mut s, [AppEvent::UpdateProgress { progress: None }]);
            assert_eq!(s.update_progress, None);
        }
    }
}

mod the_step_debugger {
    use super::*;
    use crate::model::events::VarKind;

    fn paused(key: Option<&str>, line: u32) -> AppEvent {
        AppEvent::DebugPaused {
            paused: Paused {
                key: key.map(str::to_string),
                file: "debug.feature".into(),
                line,
                depth: 0,
                prefix: "*".into(),
                keyword: "def".into(),
                text: "x = 1".into(),
                vars: vec![DebugVar {
                    name: "x".into(),
                    kind: VarKind::Number,
                    type_name: None,
                    value: line.to_string(),
                    cut: None,
                }],
            },
        }
    }

    fn debug_started() -> AppEvent {
        let mut e = started(keys());
        if let AppEvent::RunStarted { debug, .. } = &mut e {
            *debug = true;
        }
        e
    }

    fn finished() -> AppEvent {
        AppEvent::RunFinished {
            outcome: RunOutcome::Finished,
            exit_code: Some(0),
            summary: None,
            report_available: false,
        }
    }

    #[test]
    fn the_first_pause_shows_the_variables_tab_and_selects_the_paused_test_later_ones_keep_the_tab() {
        let mut s = AppState::new();
        apply(&mut s, [project(), debug_started()]);
        assert!(s.run.debug);
        apply(&mut s, [paused(Some(&keys()[1]), 8)]);
        assert_eq!((s.panel, s.selected.as_ref()), (Panel::Variables, Some(&keys()[1])));
        assert_eq!(s.paused.as_ref().unwrap().line, 8);
        apply(&mut s, [AppEvent::DebugResumed]);
        assert_eq!(s.paused, None);
        assert_eq!(s.vars.as_ref().unwrap()[0].value, "8");
        s.panel = Panel::Console;
        apply(&mut s, [paused(Some(&keys()[1]), 9)]);
        assert_eq!(s.panel, Panel::Console);
        assert_eq!(s.vars.as_ref().unwrap()[0].value, "9");
    }

    #[test]
    fn the_end_of_the_run_clears_the_pause_and_leaves_the_variables_tab() {
        let mut s = AppState::new();
        apply(
            &mut s,
            [project(), debug_started(), paused(Some(&keys()[0]), 8), finished()],
        );
        assert_eq!(s.paused, None);
        assert_eq!(s.panel, Panel::Console);
        apply(&mut s, [debug_started()]);
        assert_eq!(s.vars, None);
    }

    #[test]
    fn a_pause_outside_the_tree_keeps_the_selection() {
        let mut s = AppState::new();
        apply(&mut s, [project(), debug_started()]);
        s.selected = Some(keys()[2].clone());
        apply(&mut s, [paused(None, 8)]);
        assert_eq!(s.selected.as_ref(), Some(&keys()[2]));
    }

    #[test]
    fn takes_mains_breakpoints() {
        let mut s = AppState::new();
        let bp = Breakpoint {
            file: "a.feature".into(),
            line: 3,
        };
        apply(
            &mut s,
            [
                project(),
                AppEvent::Breakpoints {
                    breakpoints: vec![bp.clone()],
                },
            ],
        );
        assert_eq!(s.breakpoints, vec![bp]);
    }
}

mod watch {
    use super::*;

    #[test]
    fn starts_off_and_follows_mains_watch_event() {
        let mut s = AppState::new();
        assert!(!s.watch);
        s.reduce(&AppEvent::Watch { on: true });
        assert!(s.watch);
        s.reduce(&AppEvent::Watch { on: false });
        assert!(!s.watch);
    }
}

mod reduce_karates_reason_229_r2_1 {
    use super::*;

    #[test]
    fn keeps_where_the_reason_starts_until_the_test_runs_again() {
        let mut s = AppState::new();
        apply(&mut s, [project(), started(&keys()[..1])]);
        apply(
            &mut s,
            [AppEvent::Scenarios {
                updates: vec![ScenarioUpdate {
                    key: keys()[0].clone(),
                    status: Status::Failed,
                    duration_millis: None,
                    error: Some("C:/x:1\nboom".into()),
                    reason_at: Some(7),
                }],
            }],
        );
        assert_eq!(
            s.results[&keys()[0]],
            TestResult {
                duration_millis: None,
                error: Some("C:/x:1\nboom".into()),
                reason_at: Some(7),
            }
        );
        apply(&mut s, [started(&keys()[..1])]);
        assert!(!s.results.contains_key(&keys()[0]));
    }
}

// `store.test.tsx`: the store is `AppState` itself; its subscription to IPC is gone (P2 plan, Redesigns).
mod create_app_store {
    use super::*;

    fn node(key: &str, kind: NodeKind, children: Vec<TestNode>) -> TestNode {
        TestNode {
            key: key.into(),
            kind,
            name: key.into(),
            file: "a.feature".into(),
            line: Some(1),
            ref_id: None,
            tags: vec![],
            example: None,
            children,
        }
    }

    #[test]
    fn applies_events_and_selects() {
        // "subscribes before ready, applies events, selects, disposes": the subscription and `dispose` were IPC's
        let mut s = AppState::new();
        s.reduce(&AppEvent::notice("hi"));
        assert_eq!(s.notice.as_ref().unwrap().text, "hi");
        s.reduce(&project_with(None, None, Some("boom")));
        assert_eq!(s.notice, None);
        assert_eq!(s.error.as_deref(), Some("boom"));
        s.select("f:a");
        assert_eq!(s.selected.as_deref(), Some("f:a"));
    }

    #[test]
    fn trims_console_and_drops_stale_keys_through_a_real_store_draft() {
        let mut s = AppState::new();
        let tree = |kids| node("f:a.feature", NodeKind::Feature, kids);
        let scenario = |k| node(k, NodeKind::Scenario, vec![]);
        s.reduce(&project_tree(Some(tree(vec![scenario("s:a#1"), scenario("s:a#2")]))));
        s.reduce(&AppEvent::RunStarted {
            selection: "f:a.feature".into(),
            run_id: "20261025-120000-000".into(),
            keys: vec!["s:a#1".into(), "s:a#2".into()],
            total: 2,
            started_at: 1.0,
            env: None,
            debug: false,
        });
        for (from, to) in [(0, 4000), (4000, 8000), (8000, 10500)] {
            s.reduce(&AppEvent::Console {
                lines: (from..to).map(|i| format!("l{i}")).collect(),
            });
        }
        assert_eq!(s.console.len(), CONSOLE_LIMIT);
        assert_eq!(s.console.back().unwrap(), "l10499");
        let mut statuses: Vec<_> = s.statuses.keys().cloned().collect();
        statuses.sort();
        assert_eq!(statuses, ["s:a#1", "s:a#2"]);
        s.reduce(&project_tree(Some(tree(vec![scenario("s:a#1")]))));
        assert_eq!(s.statuses.keys().collect::<Vec<_>>(), ["s:a#1"]);
    }

    #[test]
    fn dismiss_prompt_drops_the_startup_modals_release() {
        let mut s = AppState::new();
        let release = json!({"version": "1.0.0-rc.6", "prerelease": true, "publishedAt": "", "url": "", "size": 1,
            "sha256": "a".repeat(64)});
        s.reduce(&AppEvent::UpdatePrompt {
            release: release.clone(),
        });
        assert_eq!(s.update_prompt, Some(release));
        s.dismiss_prompt();
        assert_eq!(s.update_prompt, None);
    }

    mod tag_filter {
        use super::*;

        fn filter() -> FilterState {
            FilterState {
                include: vec!["smoke".into()],
                exclude: vec!["wip".into()],
                hide_non_matching: true,
            }
        }

        fn open(root: &str) -> AppEvent {
            project_with(
                Some(Project {
                    root: root.into(),
                    layout: Layout::Standalone,
                }),
                None,
                None,
            )
        }

        #[test]
        fn starts_with_no_filter() {
            assert_eq!(AppState::new().filter, no_filter());
        }

        #[test]
        fn filter_event_replaces_the_filter() {
            let mut s = AppState::new();
            s.reduce(&open("D:/p"));
            s.reduce(&AppEvent::Filter { filter: filter() });
            assert_eq!(s.filter, filter());
            let slow = FilterState {
                include: vec![],
                exclude: vec!["slow".into()],
                hide_non_matching: false,
            };
            s.reduce(&AppEvent::Filter { filter: slow.clone() });
            assert_eq!(s.filter, slow);
        }

        #[test]
        fn opening_another_project_resets_the_filter() {
            let mut s = AppState::new();
            s.reduce(&open("D:/p"));
            s.reduce(&AppEvent::Filter { filter: filter() });
            s.reduce(&open("D:/p")); // a refresh keeps it
            assert_eq!(s.filter, filter());
            s.reduce(&open("D:/q"));
            assert_eq!(s.filter, no_filter());
            s.reduce(&AppEvent::Filter { filter: filter() });
            s.reduce(&not_found());
            assert_eq!(s.filter, no_filter());
            // "the store holds its own copy, never the shared constant": a Rust value is always its own copy
            assert_eq!(
                no_filter(),
                FilterState {
                    include: vec![],
                    exclude: vec![],
                    hide_non_matching: false
                }
            );
        }
    }

    mod http_calls {
        use super::*;

        #[test]
        fn select_call_select_and_show_panel() {
            let mut s = AppState::new();
            s.reduce(&project_tree(Some(node(
                "f:a.feature",
                NodeKind::Feature,
                vec![
                    node("s:a#1", NodeKind::Scenario, vec![]),
                    node("s:a#2", NodeKind::Scenario, vec![]),
                ],
            ))));
            s.reduce(&AppEvent::Calls {
                key: "s:a#1".into(),
                calls: vec![a_call(), a_call()],
            });
            let ok = CallRef {
                key: "s:a#1".into(),
                index: 0,
            };
            s.select_call(ok.clone());
            s.show_panel(Panel::Http);
            assert_eq!(s.selected_call, Some(ok.clone())); // the HTTP tab keeps it
            s.show_panel(Panel::Console);
            assert_eq!(s.panel, Panel::Console);
            assert_eq!(s.selected_call, None);
            s.select_call(ok.clone());
            s.show_panel(Panel::History);
            assert_eq!(s.selected_call, None);
            s.select_call(ok);
            s.select("s:a#2"); // a tree click gives the details pane back
            assert_eq!(s.selected.as_deref(), Some("s:a#2"));
            assert_eq!(s.selected_call, None);
        }
    }
}

mod added {
    use super::*;

    // New in the port: `reduce.ts` gave the box a fresh object per `threads` event, so it reset even on the count it
    // held; the Rust state counts the events instead (P2 plan, Redesigns).
    #[test]
    fn each_threads_event_is_a_new_generation_even_with_the_same_count() {
        let mut s = AppState::new();
        s.reduce(&AppEvent::Threads { count: 2 });
        assert_eq!(s.threads, None); // no project yet
        s.reduce(&project());
        s.reduce(&AppEvent::Threads { count: 2 });
        let first = s.threads.unwrap();
        s.reduce(&AppEvent::Threads { count: 2 });
        let second = s.threads.unwrap();
        assert_eq!((first.count, second.count), (2, 2));
        assert_ne!(first.generation, second.generation);
    }

    #[test]
    fn the_home_screen_shows_the_opens_error_as_a_notice() {
        let mut s = AppState::new();
        s.reduce(&project_with(None, None, Some("Folder not found: D:/x")));
        assert!(s.home());
        assert_eq!(s.shown_notice().unwrap().text, "Folder not found: D:/x");
        s.reduce(&project_with(Some(p()), None, Some("syntax")));
        assert!(!s.home());
        assert_eq!(s.shown_notice(), None);
    }
}
