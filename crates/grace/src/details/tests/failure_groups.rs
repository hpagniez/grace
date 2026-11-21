//! `Details.test.tsx`'s failure groups (#229 R3 to R5): where a failed test sits among the tests that failed with its
//! error, stepping through them, and a folder's failures by error.
use super::*;

const SAME: &str = "expected status: 200, actual: 503";

fn selected(h: &Harness, cx: &mut VisualTestContext) -> Option<String> {
    h.grace.read_with(cx, |g, _| g.state.selected.clone())
}

/// Enter as GPUI Kit's button reads it: a key down, then its key up presses the focused button.
fn press_enter(cx: &mut VisualTestContext) {
    let keystroke = gpui::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent { keystroke });
    cx.run_until_parked();
}

/// The three tests of `finished()` all failed, with `errors` (a test's error is its reason).
fn all_failed(errors: [&str; 3]) -> AppState {
    let mut s = finished();
    for (key, error) in [OK, BAD, WAIT].into_iter().zip(errors) {
        s.statuses.insert(key.into(), Status::Failed);
        s.results.insert(
            key.into(),
            TestResult {
                duration_millis: Some(5.),
                error: Some(error.into()),
                reason_at: None,
            },
        );
    }
    s
}

// GF-12 the count and the buttons; the focus stays on the pressed button, so Enter steps again
#[gpui::test]
fn steps_through_a_group_and_keeps_the_focus_on_the_button(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., all_failed([SAME; 3]));
    select(&h, &mut cx, OK);
    for id in ["error-group", "previous-failure", "next-failure"] {
        assert!(drawn(&mut cx, id), "{id}");
    }
    click(&mut cx, "next-failure");
    assert_eq!(selected(&h, &mut cx).as_deref(), Some(BAD));
    assert_eq!(h.grace.read_with(&cx, |g, _| g.state.reveals), 1, "the tree shows it");
}

// R4.4: move to a button and press Enter; the pane is drawn again for the new test and the focus is still on the
// button, so the next Enter steps again
#[gpui::test]
fn keeps_the_focus_on_the_pressed_button(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., all_failed([SAME; 3]));
    select(&h, &mut cx, OK);
    let mut first = None;
    for _ in 0..8 {
        cx.update(|window, cx| {
            window.focus_next(cx);
            window.draw(cx).clear(cx);
        });
        press_enter(&mut cx);
        if selected(&h, &mut cx).as_deref() != Some(OK) {
            first = selected(&h, &mut cx);
            break;
        }
    }
    let first = first.expect("the focus reaches Previous or Next, and Enter presses it");
    press_enter(&mut cx);
    cx.run_until_parked();
    let second = selected(&h, &mut cx);
    assert!(
        second.is_some() && second.as_deref() != Some(first.as_str()),
        "{first} then {second:?}"
    );
    assert_eq!(h.grace.read_with(&cx, |g, _| g.state.reveals), 2);
}

// GF-13
#[gpui::test]
fn wraps_round_both_ways(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., all_failed([SAME; 3]));
    select(&h, &mut cx, OK);
    click(&mut cx, "previous-failure");
    assert_eq!(selected(&h, &mut cx).as_deref(), Some(WAIT));
    click(&mut cx, "next-failure");
    assert_eq!(selected(&h, &mut cx).as_deref(), Some(OK));
}

// GF-14
#[gpui::test]
fn a_lone_failure_has_no_stepping(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., all_failed([SAME, "another", "a third"]));
    select(&h, &mut cx, OK);
    assert!(drawn(&mut cx, "details-error"));
    for id in ["error-group", "previous-failure", "next-failure"] {
        assert!(!drawn(&mut cx, id), "{id}");
    }
}

// GF-11
#[gpui::test]
fn shows_no_group_while_a_run_goes_on(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., all_failed([SAME; 3]));
    select(&h, &mut cx, OK);
    assert!(drawn(&mut cx, "error-group"));
    h.update(&mut cx, |s| s.run.active = true);
    assert!(!drawn(&mut cx, "error-group"));
    h.update(&mut cx, |s| s.run.active = false);
    assert!(drawn(&mut cx, "error-group"), "and again once the run ends");
}

// GF-10
#[gpui::test]
fn leaves_out_a_test_the_tag_filter_hides(cx: &mut TestAppContext) {
    let mut state = all_failed([SAME; 3]);
    state.filter.include = vec!["smoke".into()];
    let (h, mut cx, _) = setup(cx, 800., state);
    select(&h, &mut cx, OK);
    assert!(drawn(&mut cx, "error-group"), "dimmed tests still count");
    h.update(&mut cx, |s| s.filter.hide_non_matching = true);
    assert!(!drawn(&mut cx, "error-group"), "only `fine` is shown, so no group");
}

fn api_tree() -> TestNode {
    let t = |n: u32| {
        node(
            NodeKind::Scenario,
            &format!("s:api/x.feature#{n}"),
            &format!("t{n}"),
            Some(n),
            &[],
            vec![],
        )
    };
    node(
        NodeKind::Folder,
        "d:",
        "p",
        None,
        &[],
        vec![node(
            NodeKind::Feature,
            FEATURE,
            "x",
            None,
            &[],
            (1..=6).map(t).collect(),
        )],
    )
}

/// Six tests: the first failed with "404", the second and fourth with "503", the rest with "timeout".
fn api_state() -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(api_tree());
    for (n, reason) in [
        (1, "404"),
        (2, "503"),
        (3, "timeout"),
        (4, "503"),
        (5, "timeout"),
        (6, "timeout"),
    ] {
        let key = format!("s:api/x.feature#{n}");
        s.statuses.insert(key.clone(), Status::Failed);
        s.results.insert(
            key,
            TestResult {
                duration_millis: None,
                error: Some(reason.into()),
                reason_at: None,
            },
        );
    }
    s
}

// GF-15
#[gpui::test]
fn a_folder_lists_its_failures_by_error_largest_first(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., api_state());
    select(&h, &mut cx, FEATURE);
    assert!(drawn(&mut cx, "failure-groups"));
    let (first, second) = (at(&mut cx, "failure-group-0"), at(&mut cx, "failure-group-1"));
    assert!(first.bottom() <= second.top(), "3 tests, then 2");
    assert!(!drawn(&mut cx, "failure-group-2"), "the lone 404 is not listed");
    h.update(&mut cx, |s| s.run.active = true);
    assert!(!drawn(&mut cx, "failure-groups"), "none while a run goes on");
}

// GF-16
#[gpui::test]
fn go_to_first_selects_the_first_test_of_a_group_in_tree_order(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx, 800., api_state());
    select(&h, &mut cx, FEATURE);
    click(&mut cx, "go-to-first-0");
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("s:api/x.feature#3"));
    assert_eq!(h.grace.read_with(&cx, |g, _| g.state.reveals), 1);
    select(&h, &mut cx, FEATURE);
    click(&mut cx, "go-to-first-1");
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("s:api/x.feature#2"));
}

// `Details.test.tsx` 33: a reason is cut after 4 lines, on an unpadded box inside the padded one
#[gpui::test]
fn cuts_a_reason_after_4_lines(cx: &mut TestAppContext) {
    let long = (1..=10).map(|n| format!("line {n}")).collect::<Vec<_>>().join("\n");
    let mut state = api_state();
    for n in [3, 5, 6] {
        state.results.get_mut(&format!("s:api/x.feature#{n}")).unwrap().error = Some(long.clone());
    }
    let (h, mut cx, _) = setup(cx, 800., state);
    select(&h, &mut cx, FEATURE);
    let height = at(&mut cx, "failure-reason-0").size.height;
    // a line of the mono face is 19.5 px; the box adds 8 px of padding twice and a 1 px border twice. 10 lines would be
    // 213 px.
    assert!(height < px(5. * 19.5 + 18.), "{height:?}: 4 lines at most");
    assert!(height > px(3. * 19.5 + 18.), "{height:?}: 4 lines");
}

// GF-17
#[gpui::test]
fn a_folder_without_a_group_shows_no_list(cx: &mut TestAppContext) {
    let mut state = api_state();
    for (n, reason) in [(2, "a"), (3, "b"), (4, "c"), (5, "d"), (6, "e")] {
        state.results.get_mut(&format!("s:api/x.feature#{n}")).unwrap().error = Some(reason.into());
    }
    let (h, mut cx, _) = setup(cx, 800., state);
    select(&h, &mut cx, FEATURE);
    assert!(drawn(&mut cx, "details-counts"));
    assert!(!drawn(&mut cx, "failure-groups"));
}
