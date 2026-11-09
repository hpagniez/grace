//! `src/main/karate/run-state.ts`: turns Karate's events into what a run reports: status updates for the scenarios and
//! rows being run, their steps and HTTP calls when a feature ends, and the events that end the run.
use super::events::{DataError, EventKind, KarateEvent};
use super::http::extract_calls;
use super::js;
use super::steps::extract_steps;
use super::tree::leaf_index;
use crate::model::events::{AppEvent, RunOutcome, ScenarioUpdate, Summary};
use crate::model::history::TestStatusFinal;
use crate::model::http::HttpCall;
use crate::model::node::{Status, TestNode};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// A tracked test's final status and duration, as `RunState::results` has them once the run is over.
#[derive(Clone, Debug, PartialEq)]
pub struct LeafResult {
    pub status: TestStatusFinal,
    /// `None` when Karate gave none.
    pub duration_millis: Option<f64>,
}

/// Turns Karate events into status updates for the scenarios and rows with the given keys (the caller applies any
/// filter).
pub struct RunState {
    pub keys: Vec<String>,
    /// `keys` as a set, for the lookup `apply` makes for every event.
    // Rust-native: a set where the TypeScript searched `keys` with `includes`. Parity: the same keys are tracked
    // (`run_state_tracks_exactly_the_given_keys`).
    wanted: HashSet<String>,
    /// The key of the test of a `{file}|{refId}`. The tree's nodes are not kept: nothing else of them is read.
    // Rust-native: keys, where the TypeScript kept the tree's nodes, so a `RunState` borrows nothing and can move to
    // the thread that tails the run. Parity: `run_state_tracks_exactly_the_given_keys` and every test that applies a
    // run to a tree.
    index: HashMap<String, String>,
    status: IndexMap<String, Status>,
    duration: HashMap<String, f64>,
    summary: Option<Summary>,
    suite_exit: bool,
}

impl RunState {
    pub fn new(tree: &TestNode, keys: Vec<String>) -> Self {
        Self {
            wanted: keys.iter().cloned().collect(),
            keys,
            index: leaf_index(tree)
                .into_iter()
                .map(|(at, n)| (at, n.key.clone()))
                .collect(),
            status: IndexMap::new(),
            duration: HashMap::new(),
            summary: None,
            suite_exit: false,
        }
    }

    fn key_of(&self, file: &str, ref_id: &str) -> Option<String> {
        self.index
            .get(&format!("{file}|{ref_id}"))
            .filter(|key| self.wanted.contains(*key))
            .cloned()
    }

    /// At most one `scenarios` event (last status per key wins), then a `steps` and a `calls` event per scenario.
    /// An error means an event's data was not shaped as Karate writes it; the run's status can no longer be trusted
    /// (the caller ends the run).
    // Rust-native: a `Result` in place of a thrown `TypeError`, and typed `AppEvent`s in place of loose objects.
    // Parity: the same events in the same order (every `run_state` test); the unreadable calls of a feature still only
    // lose that feature's calls (`keeps_the_run_going_when_a_features_calls_cannot_be_read`).
    pub fn apply(&mut self, events: &[KarateEvent]) -> Result<Vec<AppEvent>, DataError> {
        let mut updates: IndexMap<String, ScenarioUpdate> = IndexMap::new();
        let mut steps: Vec<AppEvent> = Vec::new();
        for ev in events {
            let d = &ev.data;
            if ev.kind == EventKind::SuiteExit {
                self.suite_exit = true;
                let s = js::prop(d, "summary")?;
                self.summary = if js::truthy(s) {
                    Some(Summary {
                        scenario_count: js::as_u32(js::prop(s, "scenarioCount")?).unwrap_or(0),
                        scenarios_passed: js::as_u32(js::prop(s, "scenariosPassed")?).unwrap_or(0),
                        scenarios_failed: js::as_u32(js::prop(s, "scenariosFailed")?).unwrap_or(0),
                        duration_millis: js::prop(s, "durationMillis")?.as_f64().unwrap_or(0.0),
                    })
                } else {
                    None
                };
            } else if !ev.at_depth_zero() {
                continue; // called features
            } else if matches!(ev.kind, EventKind::ScenarioEnter | EventKind::ScenarioExit) {
                let file = js::display(js::prop(d, "feature")?);
                let ref_id = js::display(js::prop(d, "refId")?);
                let Some(key) = self.key_of(&file, &ref_id) else {
                    continue;
                };
                let mut update = ScenarioUpdate {
                    key: key.clone(),
                    status: Status::Running,
                    duration_millis: None,
                    error: None,
                    reason_at: None,
                };
                if ev.kind == EventKind::ScenarioExit {
                    update.status = if js::truthy(js::prop(d, "skipped")?) {
                        Status::NotRun
                    } else if js::truthy(js::prop(d, "passed")?) {
                        Status::Passed
                    } else {
                        Status::Failed
                    };
                    update.duration_millis = js::prop(d, "durationMillis")?.as_f64();
                    if let Some(millis) = update.duration_millis {
                        self.duration.insert(key.clone(), millis);
                    }
                    let error = js::prop(d, "error")?;
                    if update.status == Status::Failed && js::truthy(error) {
                        let Some(error) = error.as_str() else {
                            return Err(DataError::new("Karate event data: `error` is not text"));
                        };
                        update.error = Some(error.to_owned());
                        // Karate's errorReason is its error less the place, the step's comment and its text (#229 R2.1)
                        if let Some(reason) = js::prop(d, "errorReason")?.as_str()
                            && error.ends_with(reason)
                        {
                            update.reason_at = Some(js::len(error) - js::len(reason));
                        }
                    }
                }
                self.status.insert(key.clone(), update.status);
                updates.insert(key, update);
            } else if ev.kind == EventKind::FeatureExit {
                let relative_path = js::display(js::prop(d, "relativePath")?);
                // A failed karate-config.js sends no SCENARIO_EXIT: only this result says the test failed (#229 R1.1).
                for sc in js::items(js::prop(d, "scenarioResults")?, "scenarioResults")? {
                    if js::prop(sc, "failed")? != &Value::Bool(true) {
                        continue;
                    }
                    let Some(key) = self.key_of(&relative_path, &js::display(js::prop(sc, "refId")?)) else {
                        continue;
                    };
                    if matches!(self.status.get(&key), Some(Status::Passed | Status::Failed)) {
                        continue;
                    }
                    let mut update = ScenarioUpdate {
                        key: key.clone(),
                        status: Status::Failed,
                        duration_millis: js::prop(sc, "durationMillis")?.as_f64(),
                        error: None,
                        reason_at: None,
                    };
                    if let Some(error) = js::prop(sc, "error")?.as_str() {
                        update.error = Some(error.to_owned());
                        update.reason_at = Some(0);
                    }
                    if let Some(millis) = update.duration_millis {
                        self.duration.insert(key.clone(), millis);
                    }
                    self.status.insert(key.clone(), Status::Failed);
                    updates.insert(key, update);
                }
                // Karate's log format is not a contract: a feature whose calls cannot be read loses them, not the run
                // its progress. Every test of it still gets an empty `calls` event below.
                let calls: HashMap<String, Vec<HttpCall>> = match extract_calls(d) {
                    Ok(found) => found.into_iter().map(|sc| (sc.ref_id, sc.calls)).collect(),
                    Err(_) => HashMap::new(),
                };
                for sc in extract_steps(d)? {
                    let Some(key) = self.key_of(&relative_path, &sc.ref_id) else {
                        continue;
                    };
                    steps.push(AppEvent::Steps {
                        key: key.clone(),
                        steps: sc.steps,
                    });
                    steps.push(AppEvent::Calls {
                        key,
                        calls: calls.get(&sc.ref_id).cloned().unwrap_or_default(),
                    });
                }
            }
        }
        let mut out = Vec::with_capacity(steps.len() + 1);
        if !updates.is_empty() {
            out.push(AppEvent::Scenarios {
                updates: updates.into_values().collect(),
            });
        }
        out.extend(steps);
        Ok(out)
    }

    pub fn finish(&mut self, exit_code: Option<i32>, cancelled: bool, report_available: bool) -> Vec<AppEvent> {
        let outcome = if cancelled {
            RunOutcome::Cancelled
        } else if self.suite_exit && matches!(exit_code, Some(0 | 1)) {
            RunOutcome::Finished
        } else {
            RunOutcome::Failed
        };
        let mut out = Vec::new();
        // A finished suite ran everything it was going to: a selected test with no SCENARIO_EXIT did not run.
        let unended: Vec<String> = if outcome == RunOutcome::Finished {
            self.keys
                .iter()
                .filter(|k| self.status.get(*k).copied().unwrap_or(Status::Running) == Status::Running)
                .cloned()
                .collect()
        } else {
            self.status
                .iter()
                .filter(|(_, s)| **s == Status::Running)
                .map(|(k, _)| k.clone())
                .collect()
        };
        let status = if outcome == RunOutcome::Finished {
            Status::NotRun
        } else {
            Status::Interrupted
        };
        for key in &unended {
            self.status.insert(key.clone(), status);
        }
        if !unended.is_empty() {
            out.push(AppEvent::Scenarios {
                updates: unended
                    .iter()
                    .map(|key| ScenarioUpdate {
                        key: key.clone(),
                        status,
                        duration_millis: None,
                        error: None,
                        reason_at: None,
                    })
                    .collect(),
            });
        }
        out.push(AppEvent::RunFinished {
            outcome,
            exit_code,
            summary: self.summary.clone(),
            report_available,
        });
        if outcome == RunOutcome::Finished && !unended.is_empty() {
            let ran = self.keys.len() - unended.len();
            out.push(AppEvent::notice(format!(
                "Karate ran {ran} of {} selected tests",
                self.keys.len()
            )));
        }
        if outcome == RunOutcome::Finished
            && let Some(summary) = &self.summary
            && summary.scenario_count as usize > self.keys.len()
        {
            out.push(AppEvent::notice(format!(
                "Karate ran {} tests, more than the {} selected",
                summary.scenario_count,
                self.keys.len()
            )));
        }
        out
    }

    /// Final status and duration per tracked key, as `finish` left them. Valid after `finish`: a test that never
    /// started is `NotRun`.
    // Rust-native: an `IndexMap` (key order, as the TypeScript's `Map`) of a `LeafResult` whose `duration_millis` is an
    // `Option`, where the TypeScript left the property out. Parity: `results`' five tests.
    pub fn results(&self) -> IndexMap<String, LeafResult> {
        self.keys
            .iter()
            .map(|key| {
                let status = match self.status.get(key) {
                    Some(Status::Passed) => TestStatusFinal::Passed,
                    Some(Status::Failed) => TestStatusFinal::Failed,
                    Some(Status::Interrupted) => TestStatusFinal::Interrupted,
                    Some(Status::NotRun | Status::Pending | Status::Running) | None => TestStatusFinal::NotRun,
                };
                (
                    key.clone(),
                    LeafResult {
                        status,
                        duration_millis: self.duration.get(key).copied(),
                    },
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::karate::events::read_all_events;
    use crate::karate::tree::{build_tree, leaf_keys};
    use crate::model::node::NodeKind;
    use crate::model::tags::FilterState;
    use serde_json::json;
    use std::path::{Path, PathBuf};
    use std::sync::LazyLock;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures/events")
            .join(name)
    }

    fn events(name: &str) -> Vec<KarateEvent> {
        read_all_events(fixture(name)).unwrap()
    }

    static TREE: LazyLock<TestNode> = LazyLock::new(|| build_tree(&events("dryrun-clean.jsonl"), "tests").unwrap());

    fn new_state() -> RunState {
        RunState::new(&TREE, leaf_keys(&TREE))
    }

    fn smoke_not_wip() -> FilterState {
        FilterState {
            include: vec!["smoke".into()],
            exclude: vec!["wip".into()],
            hide_non_matching: false,
        }
    }

    /// `matchingLeaves` of `src/shared/tags.ts` (ported with `model::tags`): the scenarios and rows under `n` that have
    /// an included tag (or none are included) and no excluded tag.
    fn matching_leaves<'a>(n: &'a TestNode, f: &FilterState) -> Vec<&'a TestNode> {
        if n.is_leaf() {
            let included = f.include.is_empty() || n.tags.iter().any(|t| f.include.contains(t));
            let excluded = n.tags.iter().any(|t| f.exclude.contains(t));
            return if included && !excluded { vec![n] } else { vec![] };
        }
        n.children.iter().flat_map(|c| matching_leaves(c, f)).collect()
    }

    fn updates_of(out: &[AppEvent]) -> Vec<ScenarioUpdate> {
        out.iter()
            .flat_map(|e| match e {
                AppEvent::Scenarios { updates } => updates.clone(),
                _ => vec![],
            })
            .collect()
    }

    fn update(key: &str, status: Status) -> ScenarioUpdate {
        ScenarioUpdate {
            key: key.into(),
            status,
            duration_millis: None,
            error: None,
            reason_at: None,
        }
    }

    fn first_enter(run: &[KarateEvent]) -> KarateEvent {
        run.iter()
            .find(|e| e.kind == EventKind::ScenarioEnter && e.at_depth_zero())
            .unwrap()
            .clone()
    }

    fn suite_exit(run: &[KarateEvent]) -> KarateEvent {
        run.iter().find(|e| e.kind == EventKind::SuiteExit).unwrap().clone()
    }

    fn steps_of(out: &[AppEvent]) -> Vec<(&str, &Vec<crate::model::events::Step>)> {
        out.iter()
            .filter_map(|e| match e {
                AppEvent::Steps { key, steps } => Some((key.as_str(), steps)),
                _ => None,
            })
            .collect()
    }

    fn calls_of(out: &[AppEvent]) -> Vec<(&str, &Vec<HttpCall>)> {
        out.iter()
            .filter_map(|e| match e {
                AppEvent::Calls { key, calls } => Some((key.as_str(), calls)),
                _ => None,
            })
            .collect()
    }

    mod run_state {
        use super::*;

        #[test]
        fn marks_running_then_passed_or_failed() {
            let mut state = new_state();
            let mut last: IndexMap<String, ScenarioUpdate> = IndexMap::new();
            for u in updates_of(&state.apply(&events("run-clean.jsonl")).unwrap()) {
                last.insert(u.key.clone(), u);
            }
            let statuses: Vec<Status> = last.values().map(|u| u.status).collect();
            assert_eq!(statuses.iter().filter(|s| **s == Status::Passed).count(), 8);
            assert_eq!(statuses.iter().filter(|s| **s == Status::Failed).count(), 1);
            let failed = last.values().find(|u| u.status == Status::Failed).unwrap();
            assert_eq!(failed.key, "s:tests/basic.feature#2");
            assert!(failed.error.as_deref().unwrap().contains("basic.feature:20"));
        }

        #[test]
        fn reports_running_for_an_entered_scenario() {
            let enter = first_enter(&events("run-clean.jsonl"));
            assert_eq!(
                updates_of(&new_state().apply(&[enter]).unwrap()),
                [update("s:tests/basic.feature#1", Status::Running)]
            );
        }

        #[test]
        fn sends_at_most_one_scenarios_event_per_call() {
            let out = new_state().apply(&events("run-clean.jsonl")).unwrap();
            assert_eq!(
                out.iter().filter(|e| matches!(e, AppEvent::Scenarios { .. })).count(),
                1
            );
        }

        #[test]
        fn ignores_called_features() {
            let out = new_state().apply(&events("run-clean.jsonl")).unwrap();
            let keys: Vec<String> = updates_of(&out).into_iter().map(|u| u.key).collect();
            assert!(
                !keys
                    .iter()
                    .any(|k| k.contains("common/create-user") || k.contains("start-mock"))
            );
        }

        #[test]
        fn emits_steps_when_a_feature_ends() {
            let out = new_state().apply(&events("run-clean.jsonl")).unwrap();
            let steps = steps_of(&out);
            let failing = steps.iter().find(|(key, _)| *key == "s:tests/basic.feature#2");
            assert!(failing.unwrap().1.iter().any(|s| s.status == "failed"));
            assert_eq!(
                steps
                    .iter()
                    .filter(|(key, _)| key.starts_with("s:tests/basic.feature#"))
                    .count(),
                4
            );
        }

        #[test]
        fn emits_calls_when_a_feature_ends() {
            let out = new_state().apply(&events("run-clean.jsonl")).unwrap();
            let calls = calls_of(&out);
            let calls_for = |key: &str| calls.iter().find(|(k, _)| *k == key).unwrap().1;
            let hello = calls_for("s:tests/basic.feature#1");
            assert_eq!(hello.len(), 1);
            assert_eq!(hello[0].method, "GET");
            assert!(hello[0].url.as_deref().unwrap().contains("/hello"));
            assert_eq!(hello[0].status, Some(200));
            let created = calls_for("s:tests/basic.feature#3");
            assert_eq!(created.len(), 1);
            assert_eq!(created[0].method, "POST");
            assert_eq!(created[0].status, Some(201));
            assert_eq!(created[0].via.as_deref(), Some("tests/common/create-user.feature:8"));
            assert!(out.contains(&AppEvent::Calls {
                key: "s:tests/basic.feature#4".into(),
                calls: vec![]
            }));
            assert_eq!(
                calls
                    .iter()
                    .filter(|(key, _)| key.starts_with("s:tests/basic.feature#"))
                    .count(),
                4
            );
            // each calls event comes right after the steps event of the same key
            for (i, e) in out.iter().enumerate() {
                if let AppEvent::Calls { key, .. } = e {
                    assert!(
                        matches!(&out[i - 1], AppEvent::Steps { key: k, .. } if k == key),
                        "{key}"
                    );
                }
            }
        }

        #[test]
        fn keeps_the_run_going_when_a_features_calls_cannot_be_read() {
            let mut broken = events("run-clean.jsonl");
            let exit = broken
                .iter_mut()
                .find(|e| {
                    e.kind == EventKind::FeatureExit
                        && e.at_depth_zero()
                        && e.data["relativePath"] == "tests/basic.feature"
                })
                .unwrap();
            // `callResults` is read only for calls, so this breaks `extract_calls` and nothing else
            exit.data["scenarioResults"][0]["stepResults"][0]["callResults"] = json!(42);
            let out = new_state().apply(&broken).unwrap();
            let steps = steps_of(&out);
            let calls = calls_of(&out);
            let count = |keys: Vec<&str>, prefix: &str| keys.into_iter().filter(|k| k.starts_with(prefix)).count();
            let basic = "s:tests/basic.feature#";
            assert_eq!(count(steps.iter().map(|(k, _)| *k).collect(), basic), 4);
            assert_eq!(count(calls.iter().map(|(k, _)| *k).collect(), basic), 4);
            for (key, calls) in &calls {
                if key.starts_with(basic) {
                    assert!(calls.is_empty(), "{key}");
                }
            }
            // the other features after it still send theirs, and the statuses still arrive
            assert_eq!(count(steps.iter().map(|(k, _)| *k).collect(), "s:tests/"), 9);
            assert_eq!(count(calls.iter().map(|(k, _)| *k).collect(), "s:tests/"), 9);
            assert_eq!(
                updates_of(&out).iter().filter(|u| u.status == Status::Passed).count(),
                8
            );
        }

        #[test]
        fn emits_no_calls_for_called_features() {
            let out = new_state().apply(&events("run-clean.jsonl")).unwrap();
            assert!(
                !calls_of(&out)
                    .iter()
                    .any(|(k, _)| k.contains("common/create-user") || k.contains("start-mock"))
            );
        }

        // The spike sample (taskkill during tests/slow.feature) has the same slow.feature refIds as the clean tree.
        #[test]
        fn finish_after_cancel_interrupts_running_scenarios() {
            let mut state = new_state();
            state.apply(&events("4-after-taskkill-complete-file.jsonl")).unwrap();
            let out = state.finish(Some(1), true, false);
            assert_eq!(
                updates_of(&out),
                [update("s:tests/slow.feature#1", Status::Interrupted)]
            );
            assert!(matches!(
                out.last(),
                Some(AppEvent::RunFinished {
                    outcome: RunOutcome::Cancelled,
                    exit_code: Some(1),
                    summary: None,
                    ..
                })
            ));
        }

        #[test]
        fn finish_without_suite_exit_is_a_failed_run() {
            let out = new_state().finish(Some(3), false, false);
            assert_eq!(
                out,
                [AppEvent::RunFinished {
                    outcome: RunOutcome::Failed,
                    exit_code: Some(3),
                    summary: None,
                    report_available: false,
                }]
            );
        }

        #[test]
        fn finish_with_suite_exit_carries_the_summary() {
            let mut state = new_state();
            state.apply(&events("run-clean.jsonl")).unwrap();
            let out = state.finish(Some(1), false, true);
            assert_eq!(out.len(), 1);
            let AppEvent::RunFinished {
                outcome,
                report_available,
                summary: Some(summary),
                ..
            } = &out[0]
            else {
                panic!("{:?}", out[0])
            };
            assert_eq!(*outcome, RunOutcome::Finished);
            assert!(report_available);
            assert_eq!(
                (
                    summary.scenario_count,
                    summary.scenarios_passed,
                    summary.scenarios_failed
                ),
                (9, 8, 1)
            );
        }

        #[test]
        fn finish_marks_selected_tests_karate_never_ended_as_not_run() {
            let run = events("run-clean.jsonl");
            let mut state = new_state();
            state.apply(&[first_enter(&run), suite_exit(&run)]).unwrap();
            let out = state.finish(Some(0), false, true);
            assert_eq!(
                updates_of(&out),
                state
                    .keys
                    .iter()
                    .map(|key| update(key, Status::NotRun))
                    .collect::<Vec<_>>()
            );
            assert_eq!(out.last(), Some(&AppEvent::notice("Karate ran 0 of 9 selected tests")));
            assert!(out.iter().any(|e| matches!(
                e,
                AppEvent::RunFinished {
                    outcome: RunOutcome::Finished,
                    ..
                }
            )));
        }

        #[test]
        fn run_state_tracks_exactly_the_given_keys() {
            let keys = vec![
                "s:tests/basic.feature#2".to_owned(),
                "s:tests/slow.feature#1".to_owned(),
            ];
            let mut state = RunState::new(&TREE, keys.clone());
            assert_eq!(state.keys, keys);
            let mut seen: Vec<String> = updates_of(&state.apply(&events("run-clean.jsonl")).unwrap())
                .into_iter()
                .map(|u| u.key)
                .collect();
            seen.sort();
            assert_eq!(seen, keys);
        }

        #[test]
        fn warns_when_karate_ran_more_than_selected() {
            let selected: Vec<String> = matching_leaves(&TREE, &smoke_not_wip())
                .iter()
                .map(|n| n.key.clone())
                .collect();
            let mut state = RunState::new(&TREE, selected);
            state.apply(&events("run-clean.jsonl")).unwrap();
            assert_eq!(
                state.finish(Some(1), false, true).last(),
                Some(&AppEvent::notice("Karate ran 9 tests, more than the 2 selected"))
            );
        }

        #[test]
        fn no_over_run_notice_for_a_matching_count() {
            let mut state = new_state();
            state.apply(&events("run-clean.jsonl")).unwrap();
            assert!(
                !state
                    .finish(Some(1), false, true)
                    .iter()
                    .any(|e| matches!(e, AppEvent::Notice { .. }))
            );
        }

        mod results {
            use super::*;

            #[test]
            fn after_a_finished_run_not_run_for_unexited_keys() {
                let run = events("run-clean.jsonl");
                let mut state = new_state();
                state.apply(&[first_enter(&run), suite_exit(&run)]).unwrap();
                state.finish(Some(0), false, true);
                let r = state.results();
                assert_eq!(r.keys().cloned().collect::<Vec<_>>(), state.keys);
                assert!(r.values().all(|v| v.status == TestStatusFinal::NotRun));
            }

            #[test]
            fn after_a_cancel_interrupted_for_running_not_run_for_never_started() {
                let mut state = new_state();
                state.apply(&events("4-after-taskkill-complete-file.jsonl")).unwrap();
                state.finish(Some(1), true, false);
                let results = state.results();
                let of = |status: TestStatusFinal| -> Vec<&str> {
                    results
                        .iter()
                        .filter(|(_, v)| v.status == status)
                        .map(|(k, _)| k.as_str())
                        .collect()
                };
                assert_eq!(of(TestStatusFinal::Interrupted), ["s:tests/slow.feature#1"]);
                assert_eq!(of(TestStatusFinal::NotRun).len(), 8);
            }

            #[test]
            fn after_a_failed_run_without_suite_exit_same_split() {
                let mut state = new_state();
                state.apply(&events("4-after-taskkill-complete-file.jsonl")).unwrap();
                state.finish(Some(3), false, false);
                assert_eq!(
                    state.results()["s:tests/slow.feature#1"].status,
                    TestStatusFinal::Interrupted
                );
                assert_eq!(
                    state.results()["s:tests/basic.feature#1"].status,
                    TestStatusFinal::NotRun
                );
            }

            #[test]
            fn keep_durations() {
                let run = events("run-clean.jsonl");
                // from the recording: every `npm run fixtures` records other durations
                let took = |feature: &str, ref_id: &str| -> f64 {
                    let exit = run.iter().find(|e| {
                        e.kind == EventKind::ScenarioExit
                            && e.at_depth_zero()
                            && e.data["feature"] == feature
                            && e.data["refId"] == ref_id
                    });
                    exit.unwrap().data["durationMillis"].as_f64().unwrap()
                };
                let mut state = new_state();
                state.apply(&run).unwrap();
                state.finish(Some(1), false, true);
                let r = state.results();
                let result = |status, millis| LeafResult {
                    status,
                    duration_millis: Some(millis),
                };
                assert_eq!(
                    r["s:tests/basic.feature#1"],
                    result(TestStatusFinal::Passed, took("tests/basic.feature", "[1:7]"))
                );
                assert_eq!(
                    r["s:tests/basic.feature#2"],
                    result(TestStatusFinal::Failed, took("tests/basic.feature", "[2:15]"))
                );
                assert_eq!(
                    r["s:tests/slow.feature#1"],
                    result(TestStatusFinal::Passed, took("tests/slow.feature", "[1:7]"))
                );
            }

            #[test]
            fn leave_duration_millis_out_when_karate_gave_none() {
                let mut state = new_state();
                state.finish(Some(3), false, false);
                assert_eq!(
                    state.results().values().next().unwrap(),
                    &LeafResult {
                        status: TestStatusFinal::NotRun,
                        duration_millis: None
                    }
                );
            }
        }
    }

    mod run_state_config_failures_and_karates_reason_229 {
        use super::*;

        const FIRST: &str = "s:tests/shared.feature#1";
        const SECOND: &str = "s:tests/shared.feature#2";
        const ROW: &str = "s:tests/shared.feature#3.1";

        fn leaf(key: &str, kind: NodeKind, ref_id: &str, name: &str) -> TestNode {
            TestNode {
                key: key.into(),
                kind,
                name: name.into(),
                file: "tests/shared.feature".into(),
                line: Some(7),
                ref_id: Some(ref_id.into()),
                tags: vec![],
                example: None,
                children: vec![],
            }
        }

        fn shared() -> TestNode {
            TestNode {
                key: "d:".into(),
                kind: NodeKind::Folder,
                name: "p".into(),
                file: String::new(),
                line: None,
                ref_id: None,
                tags: vec![],
                example: None,
                children: vec![TestNode {
                    key: "f:tests/shared.feature".into(),
                    kind: NodeKind::Feature,
                    name: "shared".into(),
                    file: "tests/shared.feature".into(),
                    line: None,
                    ref_id: None,
                    tags: vec![],
                    example: None,
                    children: vec![
                        leaf(FIRST, NodeKind::Scenario, "[1:7]", "first"),
                        leaf(SECOND, NodeKind::Scenario, "[2:10]", "second"),
                        leaf(ROW, NodeKind::Row, "[3.1:15]", "row 1"),
                    ],
                }],
            }
        }

        /// `text.slice(from)`, counting UTF-16 units as JavaScript does.
        fn slice_utf16(text: &str, from: usize) -> String {
            String::from_utf16(&text.encode_utf16().skip(from).collect::<Vec<_>>()).unwrap()
        }

        #[test]
        fn fails_the_tests_whose_config_failed() {
            let tree = shared();
            let mut state = RunState::new(&tree, vec![FIRST.into(), SECOND.into()]);
            let updates = updates_of(&state.apply(&events("run-http-config-fail.jsonl")).unwrap());
            assert_eq!(
                updates
                    .iter()
                    .map(|u| (u.key.as_str(), u.status, u.duration_millis, u.reason_at))
                    .collect::<Vec<_>>(),
                [
                    (FIRST, Status::Failed, Some(0.0), Some(0)),
                    (SECOND, Status::Failed, Some(0.0), Some(0))
                ]
            );
            for u in &updates {
                let error = u.error.as_deref().unwrap();
                assert!(
                    error.starts_with("Config evaluation failed: karate-config.js - js failed:"),
                    "{error}"
                );
            }
            let out = state.finish(Some(1), false, true);
            assert!(!out.iter().any(|e| matches!(e, AppEvent::Notice { .. })));
            assert_eq!(state.results()[FIRST].status, TestStatusFinal::Failed);
            assert_eq!(state.results()[SECOND].status, TestStatusFinal::Failed);
        }

        #[test]
        fn fails_an_outline_row_whose_config_failed() {
            let exit = KarateEvent {
                kind: EventKind::FeatureExit,
                time_stamp: 1.0,
                thread_id: Some("thread-1".into()),
                data: json!({
                    "relativePath": "tests/shared.feature",
                    "callDepth": 0,
                    "scenarioResults": [{
                        "refId": "[3.1:15]",
                        "failed": true,
                        "error": "Config evaluation failed: karate-config.js - boom",
                        "durationMillis": 0,
                        "stepResults": [],
                    }],
                }),
            };
            let tree = shared();
            let mut state = RunState::new(&tree, leaf_keys(&tree));
            assert_eq!(
                updates_of(&state.apply(&[exit]).unwrap()),
                [ScenarioUpdate {
                    key: ROW.into(),
                    status: Status::Failed,
                    duration_millis: Some(0.0),
                    error: Some("Config evaluation failed: karate-config.js - boom".into()),
                    reason_at: Some(0),
                }]
            );
        }

        #[test]
        fn does_not_fail_a_test_twice_across_batches() {
            let mut state = new_state();
            let failed: Vec<ScenarioUpdate> = events("run-clean.jsonl")
                .iter()
                .flat_map(|e| updates_of(&state.apply(std::slice::from_ref(e)).unwrap()))
                .filter(|u| u.key == "s:tests/basic.feature#2" && u.status == Status::Failed)
                .collect();
            assert_eq!(failed.len(), 1);
        }

        #[test]
        fn sends_where_karates_reason_starts() {
            let failed = updates_of(&new_state().apply(&events("run-clean.jsonl")).unwrap())
                .into_iter()
                .find(|u| u.status == Status::Failed)
                .unwrap();
            assert_eq!(
                slice_utf16(failed.error.as_deref().unwrap(), failed.reason_at.unwrap()),
                "match failed: EQUALS\n  $ | not equal (STRING:STRING)\n  'hello'\n  'goodbye'\n"
            );
            let run = events("run-clean.jsonl");
            let mut odd = run
                .iter()
                .find(|e| e.kind == EventKind::ScenarioExit && e.at_depth_zero() && !js::truthy(&e.data["passed"]))
                .unwrap()
                .clone();
            odd.data["errorReason"] = json!("not its tail");
            assert_eq!(updates_of(&new_state().apply(&[odd]).unwrap())[0].reason_at, None);
        }

        // New in the port: `reasonAt` counts UTF-16 units, as `String.length` did, not bytes or characters.
        #[test]
        fn reason_at_counts_utf16_units() {
            let tree = shared();
            let mut state = RunState::new(&tree, vec![FIRST.into()]);
            let exit = KarateEvent {
                kind: EventKind::ScenarioExit,
                time_stamp: 1.0,
                thread_id: None,
                data: json!({
                    "feature": "tests/shared.feature", "refId": "[1:7]", "callDepth": 0,
                    "passed": false, "skipped": false, "durationMillis": 3,
                    "error": "caf\u{e9} \u{1F600} failed: boom",
                    "errorReason": "failed: boom",
                }),
            };
            let update = &updates_of(&state.apply(&[exit]).unwrap())[0];
            // "café 😀 " is 4 + 1 + 2 + 1 units (the emoji is two); 11 bytes, 7 characters
            assert_eq!(update.reason_at, Some(8));
        }

        // New in the port: data that is not an object of the shape Karate writes ends the run, as a thrown error did.
        #[test]
        fn malformed_event_data_is_an_error() {
            let tree = shared();
            let mut state = RunState::new(&tree, leaf_keys(&tree));
            let exit = KarateEvent {
                kind: EventKind::FeatureExit,
                time_stamp: 1.0,
                thread_id: None,
                data: json!({ "callDepth": 0, "relativePath": "tests/shared.feature", "scenarioResults": [{ "refId": "[1:7]", "stepResults": [{}] }] }),
            };
            assert!(state.apply(&[exit]).is_err());
        }
    }
}
