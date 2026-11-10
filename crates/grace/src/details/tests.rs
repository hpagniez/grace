//! `Details.test.tsx`'s P2 tests, headless on the fake runner: a test, a group, nothing selected and long text.
//! Classes have no GPUI twin, so the look is checked through the tokens a part takes and where the parts are drawn;
//! what a screen reader reads is `grace-e2e/tests/details.rs`'s. The steps before a run, breakpoints, Open in VS Code
//! and the failure groups come with their phases.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{AppContext as _, Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext, div, point, px};
use grace_core::model::node::Layout;
use grace_core::state::TestResult;

use super::*;
use crate::testing::{self, Harness};

const OK: &str = "s:tests/x.feature#1";
const BAD: &str = "s:tests/x.feature#2";
const WAIT: &str = "s:tests/x.feature#3";
const FEATURE: &str = "f:tests/x.feature";

fn node(kind: NodeKind, key: &str, name: &str, line: Option<u32>, tags: &[&str], children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: name.into(),
        file: if kind == NodeKind::Folder {
            String::new()
        } else {
            "tests/x.feature".into()
        },
        line,
        ref_id: None,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        example: None,
        children,
    }
}

fn tree() -> TestNode {
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
            vec![
                node(NodeKind::Scenario, OK, "fine", Some(3), &["smoke", "slow"], vec![]),
                node(NodeKind::Scenario, BAD, "broken", Some(6), &[], vec![]),
                node(NodeKind::Scenario, WAIT, "waiting", Some(9), &[], vec![]),
            ],
        )],
    )
}

fn step(text: &str, status: &str) -> Step {
    Step {
        keyword: "Given".into(),
        text: text.into(),
        line: 4,
        status: status.into(),
        millis: 3.,
        background: false,
        log: None,
        comments: None,
    }
}

/// A run of the three tests that has finished: `fine` passed, `broken` failed, `waiting` left pending.
fn finished() -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(tree());
    s.statuses.insert(OK.into(), Status::Passed);
    s.statuses.insert(BAD.into(), Status::Failed);
    s.statuses.insert(WAIT.into(), Status::Pending);
    s.results.insert(
        OK.into(),
        TestResult {
            duration_millis: Some(1500.),
            error: None,
            reason_at: None,
        },
    );
    s.results.insert(
        BAD.into(),
        TestResult {
            duration_millis: Some(40.),
            error: Some("boom at x.feature:7".into()),
            reason_at: None,
        },
    );
    s
}

fn setup(cx: &mut TestAppContext, width: f32, state: AppState) -> (Harness, VisualTestContext, Entity<Details>) {
    let slot: Rc<RefCell<Option<Entity<Details>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let (h, vcx) = testing::open_with(cx, width, state, move |grace, _, cx| {
        let d = cx.new(|cx| Details::new(grace.clone(), cx));
        *s.borrow_mut() = Some(d.clone());
        Box::new(move |_, _, _| div().h(px(600.)).flex().child(d.clone()).into_any_element())
    });
    let details = slot.borrow().clone().unwrap();
    (h, vcx, details)
}

fn select(h: &Harness, cx: &mut VisualTestContext, key: &str) {
    h.update(cx, |s| s.selected = Some(key.into()));
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
}

fn drawn(cx: &mut VisualTestContext, id: &str) -> bool {
    bounds(cx, id).is_some()
}

fn at(cx: &mut VisualTestContext, id: &str) -> Bounds<Pixels> {
    bounds(cx, id).unwrap_or_else(|| panic!("{id} is drawn"))
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let b = at(cx, id);
    cx.simulate_click(b.center(), Modifiers::default());
    cx.run_until_parked();
}

/// The tooltip after the pointer rests on `id`.
fn hover(h: &Harness, cx: &mut VisualTestContext, id: &str) -> Option<SharedString> {
    let b = at(cx, id);
    cx.simulate_mouse_move(b.center(), None, Modifiers::none());
    cx.executor()
        .advance_clock(crate::tooltip::DELAY + Duration::from_millis(1));
    cx.run_until_parked();
    let tip = h.tip(cx);
    cx.simulate_mouse_move(point(px(1.), px(690.)), None, Modifiers::none());
    cx.run_until_parked();
    tip
}

mod a_test {
    use super::*;

    #[gpui::test]
    fn heads_it_with_its_name_a_tinted_badge_and_the_meta_line(cx: &mut TestAppContext) {
        let mut picked = finished();
        assert_eq!(shown(&picked), Shown::Nothing);
        picked.selected = Some(OK.into());
        assert_eq!(shown(&picked), Shown::Test(OK.into()));
        let fine = find_node(picked.tree.as_ref().unwrap(), OK).unwrap();
        assert_eq!(location(fine), "tests/x.feature:3");
        assert_eq!(format_millis(1500.), "1.50 s");
        assert_eq!(format_millis(40.), "40 ms");
        let (h, mut cx, _) = setup(cx, 800., finished());
        select(&h, &mut cx, OK);
        for id in ["details-status", "details-location", "details-duration"] {
            assert!(drawn(&mut cx, id), "{id}");
        }
        // the badge on its own line, then the place and the time on one
        let (badge, place) = (at(&mut cx, "details-status"), at(&mut cx, "details-location"));
        assert!(badge.bottom() <= place.top());
        assert!(place.right() <= at(&mut cx, "details-duration").left());
        // `Details.test.tsx`: the test's tags as chips, after the place and the time
        let time = at(&mut cx, "details-duration");
        let (smoke, slow) = (at(&mut cx, "details-tag-0"), at(&mut cx, "details-tag-1"));
        assert!(time.right() <= smoke.left() && smoke.right() <= slow.left());
        select(&h, &mut cx, BAD);
        assert!(!drawn(&mut cx, "details-tag-0"), "a test without tags has none");
    }

    #[gpui::test]
    fn tints_the_badge_for_a_passed_a_failed_and_a_pending_test(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 800., finished());
        let t = cx.update(|_, cx| Theme::global(cx).tokens);
        assert_eq!(badge_tint(Status::Passed, t), (t.passed_tint, t.passed));
        assert_eq!(badge_tint(Status::Failed, t), (t.failed_tint, t.failed));
        assert_eq!(badge_tint(Status::Pending, t), (t.skipped_tint, t.skipped));
        assert_eq!(badge_tint(Status::NotRun, t), (t.skipped_tint, t.skipped));
        for key in [OK, BAD, WAIT] {
            select(&h, &mut cx, key);
            assert!(drawn(&mut cx, "details-status"));
        }
        // the word names the badge, so the mark in it is decorative
        assert_eq!(status_label(Status::Pending), "Pending");
    }

    #[gpui::test]
    fn puts_a_failed_tests_error_in_a_box_and_none_for_a_passed_one(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 800., finished());
        select(&h, &mut cx, BAD);
        assert!(drawn(&mut cx, "details-error"));
        assert!(at(&mut cx, "details-location").bottom() <= at(&mut cx, "details-error").top());
        select(&h, &mut cx, OK);
        assert!(!drawn(&mut cx, "details-error"));
    }

    #[gpui::test]
    fn shows_each_step_with_a_mark_named_by_its_status(cx: &mut TestAppContext) {
        let mut state = finished();
        let mut bg = step("print 'x'", "something-new");
        bg.keyword = "*".into();
        bg.background = true;
        state.steps.insert(
            BAD.into(),
            vec![
                step("path 'a'", "passed"),
                step("method get", "failed"),
                step("match 1 == 1", "skipped"),
                bg,
            ],
        );
        let (h, mut cx, _) = setup(cx, 800., state);
        select(&h, &mut cx, BAD);
        assert_eq!(
            ["passed", "failed", "skipped", "something-new"].map(|s| step_mark(s).label()),
            // a status Karate has not shown us takes the amber mark
            ["Passed", "Failed", "Skipped", "Interrupted"]
        );
        let tops: Vec<_> = (0..4).map(|i| at(&mut cx, &format!("step-text-{i}")).top()).collect();
        assert!(tops.windows(2).all(|w| w[0] < w[1]), "one row per step, in order");
        // only the background step has the chip
        assert_eq!(
            (0..4)
                .map(|i| drawn(&mut cx, &format!("step-background-{i}")))
                .collect::<Vec<_>>(),
            [false, false, false, true]
        );
        assert!(!drawn(&mut cx, "steps-later"));
    }

    #[gpui::test]
    fn shows_a_steps_comments_above_it(cx: &mut TestAppContext) {
        let mut state = finished();
        let mut commented = step("method get", "failed");
        commented.comments = Some(vec!["# why".into(), "# where from".into()]);
        state
            .steps
            .insert(BAD.into(), vec![step("path 'a'", "passed"), commented]);
        let (h, mut cx, _) = setup(cx, 800., state);
        select(&h, &mut cx, BAD);
        assert!(!drawn(&mut cx, "step-comments-0"));
        let comments = at(&mut cx, "step-comments-1");
        assert!(comments.bottom() <= at(&mut cx, "step-text-1").top());
        assert!(
            comments.size.height > at(&mut cx, "step-text-1").size.height,
            "one line per comment"
        );
    }

    #[gpui::test]
    fn toggles_a_steps_log_with_its_button(cx: &mut TestAppContext) {
        let mut state = finished();
        let mut logged = step("print 'x'", "passed");
        logged.log = Some("x".into());
        state
            .steps
            .insert(BAD.into(), vec![logged, step("match 1 == 1", "passed")]);
        let (h, mut cx, d) = setup(cx, 800., state);
        select(&h, &mut cx, BAD);
        assert_eq!(log_toggle_name("print 'x'", false), "Show log of print 'x'");
        assert_eq!(log_toggle_name("print 'x'", true), "Hide log of print 'x'");
        assert!(!drawn(&mut cx, "step-toggle-1"), "no button for a step without a log");
        assert!(!drawn(&mut cx, "step-log-0"));
        click(&mut cx, "step-toggle-0");
        assert!(cx.update(|_, cx| d.read(cx).log_open(0)));
        // the log sits under its step, above the next
        assert!(at(&mut cx, "step-log-0").bottom() <= at(&mut cx, "step-text-1").top());
        click(&mut cx, "step-toggle-0");
        assert!(!drawn(&mut cx, "step-log-0"));
    }

    #[gpui::test]
    fn a_new_selection_closes_the_logs(cx: &mut TestAppContext) {
        let mut state = finished();
        let mut logged = step("print 'x'", "passed");
        logged.log = Some("x".into());
        state.steps.insert(BAD.into(), vec![logged.clone()]);
        state.steps.insert(OK.into(), vec![logged]);
        let (h, mut cx, d) = setup(cx, 800., state);
        select(&h, &mut cx, BAD);
        click(&mut cx, "step-toggle-0");
        assert!(drawn(&mut cx, "step-log-0"));
        select(&h, &mut cx, OK);
        assert!(!cx.update(|_, cx| d.read(cx).log_open(0)));
        assert!(!drawn(&mut cx, "step-log-0"));
    }

    #[gpui::test]
    fn says_steps_come_when_the_feature_finishes_with_the_spinner_while_a_run_is_on(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 800., finished());
        select(&h, &mut cx, BAD);
        assert!(drawn(&mut cx, "steps-later"));
        assert!(!drawn(&mut cx, "steps-spinner"), "nothing runs");
        h.update(&mut cx, |s| {
            s.run.active = true;
            s.statuses.insert(BAD.into(), Status::Running);
        });
        assert!(drawn(&mut cx, "steps-spinner"));
        // a test the run has not reached has none
        h.update(&mut cx, |s| {
            s.statuses.insert(BAD.into(), Status::NotRun);
        });
        assert!(!drawn(&mut cx, "steps-spinner"));
    }
}

mod a_group {
    use super::*;

    #[gpui::test]
    fn counts_the_tests_per_status_in_tree_order(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 800., finished());
        select(&h, &mut cx, FEATURE);
        let state = h.grace.read_with(&cx, |g, _| g.state.clone());
        assert_eq!(shown(&state), Shown::Group(FEATURE.into()));
        let feature = find_node(state.tree.as_ref().unwrap(), FEATURE).unwrap();
        assert_eq!(
            counts(feature, &state),
            [(Status::Passed, 1), (Status::Failed, 1), (Status::Pending, 1)]
        );
        assert_eq!(derive_status(feature, &state.statuses), Status::Failed);
        assert_eq!(location(feature), "tests/x.feature");
        assert_eq!(location(state.tree.as_ref().unwrap()), "/");
        for id in ["details-status", "details-location", "details-counts"] {
            assert!(drawn(&mut cx, id), "{id}");
        }
        assert!(!drawn(&mut cx, "steps-later"));
    }
}

mod nothing_selected {
    use super::*;

    #[gpui::test]
    fn is_an_empty_state(cx: &mut TestAppContext) {
        let (_, mut cx, _) = setup(cx, 800., finished());
        assert!(drawn(&mut cx, "details"));
        assert!(!drawn(&mut cx, "details-status"));
        assert_eq!(NOTHING_SELECTED, "Select a test to see its details");
    }

    #[gpui::test]
    fn a_selection_gone_from_the_tree_shows_nothing(cx: &mut TestAppContext) {
        let (h, mut cx, _) = setup(cx, 800., finished());
        select(&h, &mut cx, "s:tests/gone.feature#1");
        assert!(!drawn(&mut cx, "details-status"));
    }
}

mod long_text {
    use super::*;

    const LONG: &str = "match response == { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7, h: 8, i: 9, j: 10, k: 11 }";

    #[gpui::test]
    fn shows_a_step_whole_in_a_tooltip_only_when_it_is_cut(cx: &mut TestAppContext) {
        let mut state = finished();
        state
            .steps
            .insert(BAD.into(), vec![step("path 'a'", "passed"), step(LONG, "passed")]);
        let (h, mut cx, _) = setup(cx, 400., state);
        select(&h, &mut cx, BAD);
        assert_eq!(hover(&h, &mut cx, "step-text-0"), None);
        assert_eq!(
            hover(&h, &mut cx, "step-text-1").as_deref(),
            Some(format!("Given {LONG}").as_str())
        );
    }

    #[gpui::test]
    fn shows_the_location_whole_only_when_it_is_cut(cx: &mut TestAppContext) {
        let mut state = finished();
        let mut t = tree();
        t.children[0].children[0].file = format!("tests/{}x.feature", "deep/".repeat(20));
        state.tree = Some(t);
        let (h, mut cx, _) = setup(cx, 400., state);
        select(&h, &mut cx, BAD);
        assert_eq!(hover(&h, &mut cx, "details-location"), None);
        select(&h, &mut cx, OK);
        let tip = hover(&h, &mut cx, "details-location").expect("a tooltip on the cut place");
        assert!(tip.ends_with("/x.feature:3"), "{tip}");
    }
}
