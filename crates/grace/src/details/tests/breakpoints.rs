//! `Details.test.tsx`'s "details: steps and breakpoints" tests 14, 15 and 17, and test 29 (step debugger R3, R4): the
//! steps the feature file gives before a run, and each step's breakpoint toggle. Test 16, the paused step, comes with
//! the debug run. The toggle's accessible name is `grace-e2e/tests/details.rs`'s.
use super::*;
use grace_core::model::events::{Breakpoint, FileStep};

const FILE: &str = "tests/x.feature";

fn file_step(text: &str, line: u32, background: bool) -> FileStep {
    FileStep {
        keyword: "*".into(),
        text: text.into(),
        line,
        background,
        comments: None,
    }
}

/// The background's step on line 4, then two of the test's own, the second with a comment.
fn from_file() -> Vec<FileStep> {
    let mut user = file_step("def user = { id: 7 }", 8, false);
    user.comments = Some(vec!["# the user".into()]);
    vec![
        file_step("def base = 1", 4, true),
        file_step("def name = 'grace'", 7, false),
        user,
    ]
}

/// The finished run's pane, where the file gives `BAD` and `OK` their steps.
fn with_file(cx: &mut TestAppContext, state: AppState) -> (Harness, VisualTestContext, Entity<Details>) {
    let (h, vcx, d) = setup(cx, 800., state);
    let mut steps = h.backend.steps.lock().unwrap();
    steps.insert(BAD.into(), from_file());
    steps.insert(OK.into(), from_file());
    drop(steps);
    (h, vcx, d)
}

fn asked(h: &Harness) -> Vec<String> {
    h.backend.steps_asked.lock().unwrap().clone()
}

fn dots(cx: &mut VisualTestContext) -> Vec<bool> {
    (0..3).map(|i| drawn(cx, &format!("breakpoint-dot-{i}"))).collect()
}

fn breakpoint(h: &Harness, cx: &mut VisualTestContext, lines: &[u32]) {
    let set = lines
        .iter()
        .map(|&line| Breakpoint {
            file: FILE.into(),
            line,
        })
        .collect();
    h.update(cx, |s| s.breakpoints = set);
}

// `Details.test.tsx` 14, SD-14
#[gpui::test]
fn lists_the_steps_from_the_file_before_a_run_with_no_status_or_time(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, BAD);
    assert_eq!(asked(&h), [BAD]);
    let tops: Vec<_> = (0..3).map(|i| at(&mut cx, &format!("step-text-{i}")).top()).collect();
    assert!(tops.windows(2).all(|w| w[0] < w[1]), "one row per step, in order");
    assert!(drawn(&mut cx, "step-background-0") && !drawn(&mut cx, "step-background-1"));
    assert!(at(&mut cx, "step-comments-2").bottom() <= at(&mut cx, "step-text-2").top());
    for id in ["step-mark-0", "step-time-0", "step-toggle-0", "steps-later"] {
        assert!(!drawn(&mut cx, id), "{id}");
    }
}

#[gpui::test]
fn says_steps_come_when_the_feature_finishes_while_the_file_gives_none(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, WAIT);
    assert_eq!(asked(&h), [WAIT]);
    assert!(drawn(&mut cx, "steps-later"));
    assert!(!drawn(&mut cx, "step-text-0"));
}

// `Details.test.tsx` 15, SD-15
#[gpui::test]
fn toggles_a_breakpoint_on_a_step_and_shows_the_runners_set(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, BAD);
    assert_eq!(breakpoint_name(false, 8), "Add breakpoint at line 8");
    assert_eq!(breakpoint_name(true, 8), "Remove breakpoint at line 8");
    // at the step's left
    assert!(at(&mut cx, "breakpoint-2").right() <= at(&mut cx, "step-text-2").left());
    assert_eq!(dots(&mut cx), [false; 3]);
    click(&mut cx, "breakpoint-2");
    assert_eq!(h.backend.calls(), [format!("toggle_breakpoint {FILE} 8")]);
    // the window shows the set the runner sends, not its own click
    assert_eq!(dots(&mut cx), [false; 3]);
    breakpoint(&h, &mut cx, &[8]);
    assert_eq!(dots(&mut cx), [false, false, true]);
    // another feature's line 8 is not this one's
    h.update(&mut cx, |s| s.breakpoints[0].file = "tests/other.feature".into());
    assert_eq!(dots(&mut cx), [false; 3]);
}

#[gpui::test]
fn shows_the_toggles_tooltip_beside_it(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, BAD);
    assert_eq!(
        hover(&h, &mut cx, "breakpoint-2").as_deref(),
        Some("Add breakpoint at line 8")
    );
}

// the keyboard reaches a toggle, and Enter presses it
#[gpui::test]
fn toggles_a_breakpoint_from_the_keyboard(cx: &mut TestAppContext) {
    let mut state = finished();
    // the pane with no other button before the steps
    state.statuses.insert(BAD.into(), Status::NotRun);
    let (h, mut cx, _) = with_file(cx, state);
    select(&h, &mut cx, BAD);
    for _ in 0..4 {
        cx.update(|window, cx| {
            window.focus_next(cx);
            window.draw(cx).clear(cx);
        });
        press_enter(&mut cx);
        if h.backend.calls().iter().any(|c| c.starts_with("toggle_breakpoint")) {
            break;
        }
    }
    assert!(
        h.backend.calls().contains(&format!("toggle_breakpoint {FILE} 4")),
        "{:?}",
        h.backend.calls()
    );
}

// SD-16: the background's line is each scenario's
#[gpui::test]
fn a_background_breakpoint_shows_in_every_scenario(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    breakpoint(&h, &mut cx, &[4]);
    for key in [BAD, OK] {
        select(&h, &mut cx, key);
        assert_eq!(dots(&mut cx), [true, false, false], "{key}");
    }
}

// `Details.test.tsx` 17
#[gpui::test]
fn the_runs_steps_replace_the_files_with_the_same_toggles(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, BAD);
    breakpoint(&h, &mut cx, &[4]);
    h.update(&mut cx, |s| {
        let run = ["def base = 1", "def name = 'grace'", "def user = { id: 7 }"]
            .iter()
            .zip([4, 7, 8])
            .map(|(text, line)| {
                let mut run = step(text, "passed");
                run.line = line;
                run
            })
            .collect();
        s.steps.insert(BAD.into(), run);
    });
    for i in 0..3 {
        assert!(drawn(&mut cx, &format!("breakpoint-{i}")), "toggle {i}");
        assert!(drawn(&mut cx, &format!("step-mark-{i}")), "mark {i}");
        assert!(drawn(&mut cx, &format!("step-time-{i}")), "time {i}");
    }
    assert_eq!(dots(&mut cx), [true, false, false]);
    click(&mut cx, "breakpoint-2");
    assert_eq!(h.backend.calls(), [format!("toggle_breakpoint {FILE} 8")]);
}

// `Details.test.tsx` 29, GF-04: Karate's synthetic step of a failed config has line 0
#[gpui::test]
fn has_no_toggle_on_a_step_below_line_1(cx: &mut TestAppContext) {
    let mut state = finished();
    let mut synthetic = step("Scenario execution failed: boom", "failed");
    synthetic.line = 0;
    state.steps.insert(BAD.into(), vec![synthetic, step("ok", "passed")]);
    let (h, mut cx, _) = setup(cx, 800., state);
    select(&h, &mut cx, BAD);
    assert!(!drawn(&mut cx, "breakpoint-0"));
    assert!(drawn(&mut cx, "breakpoint-1"));
    // the two texts start in line all the same
    assert_eq!(at(&mut cx, "step-mark-0").left(), at(&mut cx, "step-mark-1").left());
}

// ADR 0030: the file is read for a test newly shown, never for a redraw
#[gpui::test]
fn reads_the_files_steps_once_per_selection_and_never_on_a_redraw(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, BAD);
    assert_eq!(asked(&h), [BAD]);
    for _ in 0..3 {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }
    breakpoint(&h, &mut cx, &[8]);
    h.update(&mut cx, |s| {
        s.statuses.insert(BAD.into(), Status::Running);
        s.run.active = true;
    });
    select(&h, &mut cx, BAD);
    assert_eq!(asked(&h), [BAD], "redraws and a run's changes read nothing");
    select(&h, &mut cx, OK);
    select(&h, &mut cx, BAD);
    assert_eq!(asked(&h), [BAD, OK, BAD]);
    // the run's steps need no file; once a new run drops them, the file is read again
    h.update(&mut cx, |s| {
        s.steps.insert(BAD.into(), vec![step("ok", "passed")]);
    });
    assert_eq!(asked(&h).len(), 3);
    h.update(&mut cx, |s| {
        s.steps.remove(BAD);
    });
    assert_eq!(asked(&h).len(), 4);
    // a test shown with its run's steps, a group and nothing selected read nothing
    h.update(&mut cx, |s| {
        s.steps.insert(OK.into(), vec![step("ok", "passed")]);
    });
    select(&h, &mut cx, OK);
    select(&h, &mut cx, FEATURE);
    h.update(&mut cx, |s| s.selected = None);
    assert_eq!(asked(&h).len(), 4);
}

// a Refresh may follow an edit of the file: the lines shown are the file's again
#[gpui::test]
fn reads_the_file_again_once_the_tests_are_listed_again(cx: &mut TestAppContext) {
    let (h, mut cx, _) = with_file(cx, finished());
    select(&h, &mut cx, BAD);
    h.backend
        .steps
        .lock()
        .unwrap()
        .insert(BAD.into(), vec![file_step("def added = 1", 9, false)]);
    h.update(&mut cx, |s| s.trees += 1);
    assert_eq!(asked(&h), [BAD, BAD]);
    assert!(drawn(&mut cx, "step-text-0") && !drawn(&mut cx, "step-text-1"));
}

#[gpui::test]
fn reads_the_steps_of_the_test_already_selected_as_the_pane_is_made(cx: &mut TestAppContext) {
    let mut state = finished();
    state.selected = Some(BAD.into());
    let (h, _, _) = setup(cx, 800., state);
    assert_eq!(asked(&h), [BAD]);
}
