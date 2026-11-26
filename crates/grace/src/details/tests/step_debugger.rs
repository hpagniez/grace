//! `Details.test.tsx` 16 and the paused test's pane (step debugger R7.1, R7.3; step comments SC-06). What a screen
//! reader reads is `grace-e2e/tests/debug.rs`'s.
use grace_core::model::events::{FileStep, Paused};

use super::*;

/// The steps of `broken` as its file has them: a debug run pauses before its feature finishes.
fn file_steps() -> Vec<FileStep> {
    [(7, "name = 'grace'"), (8, "user = { id: 7 }")]
        .into_iter()
        .map(|(line, text)| FileStep {
            keyword: "*".into(),
            text: format!("def {text}"),
            line,
            background: false,
            comments: (line == 8).then(|| vec!["# the user".into()]),
        })
        .collect()
}

fn pause(file: &str, line: u32, depth: u32) -> Paused {
    Paused {
        file: file.into(),
        line,
        depth,
        ..testing::paused(BAD)
    }
}

/// A debug run of the finished state's `broken`, its pane on the file's steps.
fn show(cx: &mut TestAppContext) -> (Harness, VisualTestContext) {
    let mut state = finished();
    state.steps.remove(BAD);
    state.run.active = true;
    state.run.debug = true;
    state.statuses.insert(BAD.into(), Status::Running);
    let (h, mut cx, _) = setup(cx, 800., state);
    h.backend.steps.lock().unwrap().insert(BAD.into(), file_steps());
    select(&h, &mut cx, BAD);
    (h, cx)
}

fn set(h: &Harness, cx: &mut VisualTestContext, paused: Option<Paused>) {
    h.update(cx, |s| s.paused = paused);
}

fn marked(cx: &mut VisualTestContext) -> Vec<usize> {
    (0..2).filter(|i| drawn(cx, &format!("step-paused-{i}"))).collect()
}

/// 16 ("marks the step the run is paused before, or names the called feature"), SC-06 and SD-21.
#[gpui::test]
fn marks_the_step_the_run_is_paused_before_with_its_comments_or_names_the_called_feature(cx: &mut TestAppContext) {
    let (h, mut cx) = show(cx);
    assert!(marked(&mut cx).is_empty() && !drawn(&mut cx, "paused-in"));
    set(&h, &mut cx, Some(pause("tests/x.feature", 8, 0)));
    assert_eq!(marked(&mut cx), [1]);
    let row = at(&mut cx, "step-paused-1");
    for part in ["step-comments-1", "step-text-1", "step-paused-word-1"] {
        assert!(row.contains(&at(&mut cx, part).center()), "{part} in the marked row");
    }
    assert!(
        at(&mut cx, "step-text-1").right() <= at(&mut cx, "step-paused-word-1").left(),
        "\"Paused\" after the step"
    );
    assert!(!drawn(&mut cx, "paused-in"));
    // inside a called feature no row is marked: the place is named under the steps
    set(&h, &mut cx, Some(pause("called.feature", 8, 1)));
    assert!(marked(&mut cx).is_empty());
    assert!(at(&mut cx, "step-text-1").bottom() <= at(&mut cx, "paused-in").top());
    set(&h, &mut cx, None);
    assert!(marked(&mut cx).is_empty() && !drawn(&mut cx, "paused-in"));
}

/// The row is the paused one only for the paused test, its own file and its own line.
#[gpui::test]
fn marks_nothing_for_another_test_another_file_or_another_line(cx: &mut TestAppContext) {
    let (h, mut cx) = show(cx);
    for other in [
        pause("tests/y.feature", 8, 0),
        pause("tests/x.feature", 9, 0),
        Paused {
            key: Some(OK.into()),
            ..pause("tests/x.feature", 8, 0)
        },
        Paused {
            key: None,
            ..pause("tests/x.feature", 8, 1)
        },
    ] {
        set(&h, &mut cx, Some(other.clone()));
        assert!(marked(&mut cx).is_empty() && !drawn(&mut cx, "paused-in"), "{other:?}");
    }
}

/// SD-19: the paused test's badge has the pause mark (R7.2), still `running` underneath, and the pane's own mark of
/// any other test is its status'.
#[test]
fn the_paused_tests_mark_is_the_pause() {
    assert_eq!(crate::tree::mark_of(Status::Running, true), MarkStatus::Paused);
    assert_eq!(crate::tree::mark_of(Status::Running, false), MarkStatus::Running);
    assert_eq!(crate::tree::mark_of(Status::Passed, false), MarkStatus::Passed);
}
