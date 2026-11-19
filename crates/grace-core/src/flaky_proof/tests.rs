// `src/renderer/FlakyProof.test.tsx`'s rules, test for test: what the proof boxes of Details say.
use super::*;
use crate::model::flaky::{FlakyEnv, FlakyRun, FlakyStatus};
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

fn run(id: &str, started_at: f64, status: FlakyStatus, threads: Option<u32>, report: bool) -> FlakyRun {
    FlakyRun {
        id: id.into(),
        started_at,
        status,
        threads,
        report_available: report,
    }
}

fn env(name: Option<&str>, passed: u32, failed: u32, runs: Vec<FlakyRun>) -> FlakyEnv {
    FlakyEnv {
        env: name.map(String::from),
        passed,
        failed,
        runs,
    }
}

/// 1: the boxes go the current environment first, then none, then the rest by name; each is headed and counted.
#[test]
fn orders_the_boxes_and_words_them() {
    let envs = [
        env(Some("dev"), 1, 1, vec![]),
        env(None, 3, 2, vec![]),
        env(Some("qa"), 1, 2, vec![]),
    ];
    let boxes = proof_boxes(&envs, Some("qa"), now(), false);
    let view: Vec<(&str, &str)> = boxes.iter().map(|b| (b.heading.as_str(), b.counts.as_str())).collect();
    assert_eq!(
        view,
        [
            ("Flaky on qa", "Passed 1 time, failed 2 times with the same project files"),
            (
                "Flaky with no environment",
                "Passed 3 times, failed 2 times with the same project files"
            ),
            ("Flaky on dev", "Passed 1 time, failed 1 time with the same project files"),
        ]
    );
}

/// 3: a run says when, its result word, its thread count and whether its Report takes a press, in the order given.
#[test]
fn lists_the_proof_runs_as_given_newest_first() {
    let envs = [env(
        Some("qa"),
        1,
        2,
        vec![
            run("c", at(25, 14, 32), FlakyStatus::Failed, Some(1), true),
            run("b", at(24, 17, 20), FlakyStatus::Passed, Some(8), true),
            run("a", at(21, 9, 5), FlakyStatus::Failed, None, true),
        ],
    )];
    let boxes = proof_boxes(&envs, Some("qa"), now(), false);
    let rows: Vec<(&str, &str, &str, &str)> = boxes[0]
        .runs
        .iter()
        .map(|r| (r.id.as_str(), r.when.as_str(), r.result, r.threads.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("c", "Today 14:32", "Failed", "1 thread"),
            ("b", "Yesterday 17:20", "Passed", "8 threads"),
            ("a", "2026-10-21 09:05", "Failed", "\u{2014}"),
        ]
    );
    assert_eq!(
        boxes[0].runs.iter().map(|r| r.status).collect::<Vec<_>>(),
        [FlakyStatus::Failed, FlakyStatus::Passed, FlakyStatus::Failed]
    );
}

/// 6: Report takes a press only for a run that has a report, and not while a run goes on or the runner is busy.
#[test]
fn report_follows_the_runs_report_and_waits_for_the_runner() {
    let envs = [env(
        Some("qa"),
        1,
        1,
        vec![
            run("b", at(25, 14, 0), FlakyStatus::Passed, Some(1), true),
            run("a", at(25, 13, 0), FlakyStatus::Failed, Some(1), false),
        ],
    )];
    let on = |locked| {
        proof_boxes(&envs, Some("qa"), now(), locked)[0]
            .runs
            .iter()
            .map(|r| r.report)
            .collect::<Vec<_>>()
    };
    assert_eq!(on(false), [true, false]);
    assert_eq!(on(true), [false, false]);
}

/// 8: a test that is not flaky has no environment, so no box.
#[test]
fn a_test_that_is_not_flaky_has_no_box() {
    assert!(proof_boxes(&[], None, now(), false).is_empty());
}
