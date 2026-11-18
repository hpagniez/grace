// `src/renderer/History.test.tsx`'s rules, test for test: what a row says and which of its buttons take a press.
use super::*;
use crate::model::events::RunOutcome;
use crate::model::node::NodeKind;
use crate::model::tags::{FilterState, no_filter};
use chrono::{Local, TimeZone};

/// `new Date(2026, 9, day, h, m).getTime()`: a local time, so the tests mean the same in every time zone.
fn at(day: u32, h: u32, m: u32) -> f64 {
    Local
        .with_ymd_and_hms(2026, 10, day, h, m, 0)
        .unwrap()
        .timestamp_millis() as f64
}

/// "Now" of the Vitest file: Sunday 2026-10-25 15:00 local time.
fn now() -> f64 {
    at(25, 15, 0)
}

fn selection(kind: NodeKind, file: &str, name: &str) -> StoredSelection {
    StoredSelection {
        kind,
        file: file.into(),
        name: name.into(),
        line: None,
        key: "k".into(),
    }
}

fn entry(id: &str) -> HistoryEntry {
    HistoryEntry {
        id: id.into(),
        started_at: at(25, 14, 32),
        duration_millis: 72_000.,
        env: None,
        threads: Some(1),
        filter: no_filter(),
        selection: Some(selection(NodeKind::Folder, "", "clean")),
        failed_of: None,
        outcome: RunOutcome::Finished,
        exit_code: Some(1),
        passed: 41,
        failed: 2,
        total: 43,
        report_available: true,
        debug: false,
    }
}

fn filter(include: &[&str], exclude: &[&str]) -> FilterState {
    FilterState {
        include: include.iter().map(|t| t.to_string()).collect(),
        exclude: exclude.iter().map(|t| t.to_string()).collect(),
        hide_non_matching: false,
    }
}

mod duration_label {
    use super::*;

    #[test]
    fn shows_xs_under_a_minute_and_xm_yys_from_a_minute() {
        assert_eq!(duration_label(0.), "0s");
        assert_eq!(duration_label(999.), "0s");
        assert_eq!(duration_label(18_000.), "18s");
        assert_eq!(duration_label(59_999.), "59s");
        assert_eq!(duration_label(60_000.), "1m 00s");
        assert_eq!(duration_label(72_000.), "1m 12s");
        assert_eq!(duration_label(125_400.), "2m 05s");
    }
}

mod row_text {
    use super::*;

    #[test]
    fn lists_runs_with_labels() {
        let a = row_text(
            &HistoryEntry {
                filter: filter(&["smoke"], &["slow"]),
                ..entry("a")
            },
            now(),
        );
        assert_eq!(a.when, "Today 14:32");
        assert_eq!(a.env, "—");
        assert_eq!(
            a.tags,
            [
                Tag {
                    text: "@smoke".into(),
                    excluded: false
                },
                Tag {
                    text: "not @slow".into(),
                    excluded: true
                },
            ]
        );
        assert_eq!(a.what, "All");
        assert_eq!(a.result, "41 passed, 2 failed of 43");
        assert_eq!(a.duration, "1m 12s");

        let b = row_text(
            &HistoryEntry {
                started_at: at(24, 17, 20),
                env: Some("qa".into()),
                duration_millis: 18_000.,
                outcome: RunOutcome::Cancelled,
                exit_code: None,
                passed: 1,
                failed: 0,
                total: 5,
                selection: Some(selection(NodeKind::Scenario, "tests/users.feature", "create user")),
                ..entry("b")
            },
            now(),
        );
        assert_eq!(b.when, "Yesterday 17:20");
        assert_eq!(b.env, "qa");
        assert!(b.tags.is_empty());
        assert_eq!(b.what, "create user");
        assert_eq!(b.result, "Cancelled, 1 passed, 0 failed of 5");
        assert_eq!(b.duration, "18s");

        let c = row_text(
            &HistoryEntry {
                started_at: at(20, 9, 5),
                duration_millis: 400.,
                selection: None,
                failed_of: Some(at(25, 14, 32)),
                outcome: RunOutcome::Failed,
                exit_code: Some(3),
                ..entry("c")
            },
            now(),
        );
        assert_eq!(c.when, "2026-10-20 09:05");
        assert_eq!(c.what, "Failed tests of Today 14:32");
        assert_eq!(c.result, "Error (exit 3)");
        assert_eq!(c.duration, "0s");
    }

    #[test]
    fn shows_each_runs_thread_count_or_a_dash() {
        assert_eq!(
            row_text(
                &HistoryEntry {
                    threads: Some(8),
                    ..entry("a")
                },
                now()
            )
            .threads,
            "8"
        );
        assert_eq!(
            row_text(
                &HistoryEntry {
                    threads: None,
                    ..entry("b")
                },
                now()
            )
            .threads,
            "—"
        );
    }

    #[test]
    fn a_run_with_no_filter_has_no_tag() {
        assert!(row_text(&entry("a"), now()).tags.is_empty());
    }

    /// A filter that only hides non-matching tests has no tag to show.
    #[test]
    fn a_filter_with_no_tag_shows_no_chip() {
        let hide = FilterState {
            hide_non_matching: true,
            ..no_filter()
        };
        assert!(
            row_text(
                &HistoryEntry {
                    filter: hide,
                    ..entry("a")
                },
                now()
            )
            .tags
            .is_empty()
        );
    }
}

mod selection_path {
    use super::*;

    #[test]
    fn says_where_the_node_sits() {
        let path = |s| selection_path(Some(&s));
        assert_eq!(
            path(selection(NodeKind::Scenario, "tests/users.feature", "create user")).as_deref(),
            Some("tests/users.feature › create user")
        );
        assert_eq!(
            path(selection(NodeKind::Feature, "tests/users.feature", "users")).as_deref(),
            Some("tests/users.feature")
        );
        assert_eq!(
            path(selection(NodeKind::Folder, "tests/api", "api")).as_deref(),
            Some("tests/api")
        );
    }

    #[test]
    fn says_nothing_for_a_run_that_has_no_path() {
        // All (the root folder) and Failed tests
        assert_eq!(selection_path(Some(&selection(NodeKind::Folder, "", "clean"))), None);
        assert_eq!(selection_path(None), None);
    }
}

mod result_mark {
    use super::*;

    #[test]
    fn a_failure_beats_a_cancel() {
        let mark = |e: HistoryEntry| result_mark(&e);
        assert_eq!(mark(entry("a")), ResultMark::Failed); // finished with 2 failed
        assert_eq!(
            mark(HistoryEntry {
                failed: 0,
                passed: 3,
                total: 3,
                ..entry("b")
            }),
            ResultMark::Passed
        );
        let cancelled = HistoryEntry {
            outcome: RunOutcome::Cancelled,
            exit_code: None,
            failed: 0,
            passed: 1,
            total: 5,
            ..entry("c")
        };
        assert_eq!(mark(cancelled.clone()), ResultMark::Cancelled);
        assert_eq!(mark(HistoryEntry { failed: 1, ..cancelled }), ResultMark::Failed);
        assert_eq!(
            mark(HistoryEntry {
                outcome: RunOutcome::Failed,
                exit_code: Some(3),
                ..entry("d")
            }),
            ResultMark::Failed
        );
    }
}

mod actions {
    use super::*;

    #[test]
    fn report_is_off_without_a_report() {
        let a = actions(
            &HistoryEntry {
                report_available: false,
                ..entry("a")
            },
            false,
        );
        assert!(!a.report);
        assert!(a.rerun);
        assert!(actions(&entry("b"), false).report);
    }

    /// ER-05: Export is on exactly when Report is: a finished run with a report, and the runner quiet.
    #[test]
    fn export_follows_report() {
        let finished = entry("a");
        let cancelled = HistoryEntry {
            outcome: RunOutcome::Cancelled,
            report_available: false,
            ..entry("b")
        };
        for (e, locked) in [
            (&finished, false),
            (&finished, true),
            (&cancelled, false),
            (&cancelled, true),
        ] {
            let a = actions(e, locked);
            assert_eq!(a.export, a.report, "{} locked {locked}", e.id);
        }
        assert!(actions(&finished, false).export);
        assert!(!actions(&cancelled, false).export);
    }

    #[test]
    fn re_run_failed_is_off_with_no_failure() {
        let a = actions(
            &HistoryEntry {
                failed: 0,
                passed: 3,
                total: 3,
                ..entry("a")
            },
            false,
        );
        assert!(!a.rerun_failed);
        assert!(a.rerun);
        assert!(actions(&entry("b"), false).rerun_failed);
    }

    #[test]
    fn every_action_is_off_while_the_runner_is_busy() {
        let a = actions(&entry("a"), true);
        assert_eq!(
            (a.report, a.export, a.rerun, a.rerun_failed),
            (false, false, false, false)
        );
    }

    #[test]
    fn locked_while_a_run_is_active_or_the_runner_is_busy() {
        let mut s = AppState::new();
        assert!(!locked(&s));
        s.busy = true;
        assert!(locked(&s));
        s.busy = false;
        s.run.active = true;
        assert!(locked(&s));
    }

    #[test]
    fn clear_history_needs_a_run_and_a_quiet_runner() {
        let mut s = AppState::new();
        assert!(!can_clear(&s), "no runs");
        s.history = vec![entry("a")];
        assert!(can_clear(&s));
        s.busy = true;
        assert!(!can_clear(&s));
    }
}

mod clear_prompt {
    use super::*;

    #[test]
    fn counts_the_runs_and_words_one_in_the_singular() {
        assert_eq!(clear_prompt(3), "Delete the 3 runs of this project and their reports?");
        assert_eq!(clear_prompt(1), "Delete the 1 run of this project and its report?");
    }
}

#[test]
fn the_empty_text_is_electron_graces() {
    assert_eq!(EMPTY, "No runs yet. Runs appear here when they finish.");
}
