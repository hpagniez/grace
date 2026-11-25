//! `FlakyProof.test.tsx`'s view tests, headless on the fake runner, with Details around the boxes as the app has it.
//! What the boxes say is `grace_core::flaky_proof::tests`'; what a screen reader reads and the real runs are
//! `grace-e2e/tests/flaky.rs`'s. The proof holds no text from a run, only times, counts and thread counts, so no secret
//! can reach it (porting rule 2).
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Bounds, Entity, IntoElement as _, Modifiers, ParentElement as _, Pixels, Styled as _,
    TestAppContext, VisualTestContext, div, px,
};
use grace_core::model::env::EnvView;
use grace_core::model::flaky::{FlakyEnv, FlakyRun, FlakyStatus};
use grace_core::model::node::{Layout, NodeKind, Status, TestNode};
use grace_core::state::{AppState, TestResult};

use crate::details::Details;
use crate::testing::{self, Harness};
use crate::theme::Theme;

const COIN: &str = "s:tests/x.feature#1";
const STEADY: &str = "s:tests/x.feature#2";
const NOW: f64 = 1_792_000_000_000.;

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

fn run(id: &str, status: FlakyStatus, threads: Option<u32>, report: bool) -> FlakyRun {
    FlakyRun {
        id: id.into(),
        started_at: NOW - 60_000.,
        status,
        threads,
        report_available: report,
    }
}

fn env(name: Option<&str>, runs: Vec<FlakyRun>) -> FlakyEnv {
    FlakyEnv {
        env: name.map(String::from),
        passed: 1,
        failed: 1,
        runs,
    }
}

fn pair(a: &str, b: &str) -> Vec<FlakyRun> {
    vec![
        run(a, FlakyStatus::Passed, Some(1), true),
        run(b, FlakyStatus::Failed, Some(2), true),
    ]
}

fn in_force(name: &str) -> EnvView {
    EnvView {
        env: Some(name.into()),
        suggestions: vec![],
        overlays: vec![],
        properties: Default::default(),
        secrets_available: true,
    }
}

/// `coin`, selected and flaky in `envs`, tagged `slow`, with `steady` beside it.
fn state(envs: Vec<FlakyEnv>) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(node(
        NodeKind::Folder,
        "d:",
        "p",
        None,
        &[],
        vec![node(
            NodeKind::Feature,
            "f:tests/x.feature",
            "x",
            None,
            &[],
            vec![
                node(NodeKind::Scenario, COIN, "coin", Some(3), &["slow"], vec![]),
                node(NodeKind::Scenario, STEADY, "steady", Some(6), &[], vec![]),
            ],
        )],
    ));
    s.selected = Some(COIN.into());
    s.flaky.insert(COIN.into(), envs);
    s
}

fn setup(cx: &mut TestAppContext, width: f32, s: AppState) -> (Harness, VisualTestContext) {
    let slot: Rc<RefCell<Option<Entity<Details>>>> = Rc::default();
    let keep = Rc::clone(&slot);
    let (h, mut vcx) = testing::open_with(cx, width, s, move |grace, _, cx| {
        let details = cx.new(|cx| Details::new(grace.clone(), cx));
        details.update(cx, |d, _| d.clock = Rc::new(|| NOW));
        *keep.borrow_mut() = Some(details.clone());
        // the pane is half the window wide, as the app's main area has it
        Box::new(move |_, _, _| {
            div()
                .w(px(width / 2.))
                .h(px(600.))
                .flex()
                .child(details.clone())
                .into_any_element()
        })
    });
    vcx.update(|window, _| window.refresh());
    vcx.run_until_parked();
    (h, vcx)
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(id.to_string().leak())
}

fn at(cx: &mut VisualTestContext, id: &str) -> Bounds<Pixels> {
    bounds(cx, id).unwrap_or_else(|| panic!("{id} is drawn"))
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let b = at(cx, id);
    cx.simulate_click(b.center(), Modifiers::none());
    cx.run_until_parked();
}

fn top(cx: &mut VisualTestContext, env: &str) -> f32 {
    f32::from(at(cx, &format!("flaky-proof:{env}")).top())
}

/// 1: one box per environment, the one in force first, then none, then the rest by name.
#[gpui::test]
fn puts_a_box_per_environment_the_one_in_force_first(cx: &mut TestAppContext) {
    let mut s = state(vec![
        env(Some("dev"), pair("a", "b")),
        env(None, pair("c", "d")),
        env(Some("qa"), pair("e", "f")),
    ]);
    s.env = Some(in_force("qa"));
    let (_h, mut cx) = setup(cx, 1008., s);
    let tops: Vec<f32> = ["qa", "-", "dev"].iter().map(|e| top(&mut cx, e)).collect();
    assert!(
        tops[0] < tops[1] && tops[1] < tops[2],
        "qa, none, dev from the top: {tops:?}"
    );
}

/// 2: the box goes after the place, the time and the tags, and before the error.
#[gpui::test]
fn sits_after_the_tags_and_before_the_error(cx: &mut TestAppContext) {
    let mut s = state(vec![env(Some("qa"), pair("a", "b"))]);
    s.statuses.insert(COIN.into(), Status::Failed);
    s.results.insert(
        COIN.into(),
        TestResult {
            duration_millis: Some(1500.),
            error: Some("boom".into()),
            reason_at: None,
        },
    );
    let (_h, mut cx) = setup(cx, 1008., s);
    let proof = at(&mut cx, "flaky-proof:qa");
    for before in ["details-location", "details-duration", "details-tag-0"] {
        assert!(
            at(&mut cx, before).bottom() <= proof.top(),
            "{before} is above the proof"
        );
    }
    assert!(
        proof.bottom() <= at(&mut cx, "details-error-box").top(),
        "the error is below it"
    );
}

/// 3, 5: a row for each proof run, in the order main gave, each with its mark and its Report.
#[gpui::test]
fn lists_the_proof_runs_in_order_each_with_its_report(cx: &mut TestAppContext) {
    let (_h, mut cx) = setup(cx, 1008., state(vec![env(Some("qa"), pair("c", "b"))]));
    let (first, second) = (at(&mut cx, "flaky-run:qa:c"), at(&mut cx, "flaky-run:qa:b"));
    assert!(first.bottom() <= second.top(), "newest first");
    for id in ["c", "b"] {
        assert!(bounds(&mut cx, &format!("flaky-report:qa:{id}")).is_some());
        assert!(bounds(&mut cx, &format!("flaky-run-mark:qa:{id}")).is_some());
    }
}

/// 4: the box is headed by the shuffle icon, 14 px, inside it.
#[gpui::test]
fn heads_the_box_with_the_shuffle_icon(cx: &mut TestAppContext) {
    let (_h, mut cx) = setup(cx, 1008., state(vec![env(Some("qa"), pair("a", "b"))]));
    let (b, i) = (at(&mut cx, "flaky-proof:qa"), at(&mut cx, "flaky-proof-icon:qa"));
    assert!(b.contains(&i.origin) && i.size.width == px(14.));
}

/// #398: the box colours its heading as it fills, in the cancelled tokens, in both themes (`TINT.cancelled`); the
/// icon takes the same colour. The counts and rows set ink themselves.
#[test]
fn colours_the_heading_and_its_icon_as_the_cancelled_box() {
    for dark in [false, true] {
        let t = Theme::new(dark).tokens;
        let (mut b, mut i) = (super::tinted(t), super::shuffle(t));
        assert_eq!(b.style().text.color, Some(Theme::color(t.cancelled)));
        assert_eq!(i.style().text.color, Some(Theme::color(t.cancelled)));
    }
}

/// 6: Report opens that run's report, only when it has one, and not while the runner is busy or a run goes on.
#[gpui::test]
fn opens_a_runs_report_only_when_it_has_one_and_the_runner_is_idle(cx: &mut TestAppContext) {
    let envs = vec![env(
        Some("qa"),
        vec![
            run("b", FlakyStatus::Passed, Some(1), true),
            run("a", FlakyStatus::Failed, Some(1), false),
        ],
    )];
    let (h, mut cx) = setup(cx, 1008., state(envs));
    click(&mut cx, "flaky-report:qa:b");
    assert_eq!(h.backend.calls(), ["run_report_path b"]);
    assert_eq!(
        h.seams.opened(),
        [std::path::PathBuf::from("/runs/b/karate-summary.html")]
    );
    click(&mut cx, "flaky-report:qa:a"); // no report: nothing
    assert_eq!(h.backend.calls().len(), 1);
    h.update(&mut cx, |s| s.busy = true);
    click(&mut cx, "flaky-report:qa:b");
    assert_eq!(h.backend.calls().len(), 1, "a busy runner");
    h.update(&mut cx, |s| {
        s.busy = false;
        s.run.active = true;
    });
    click(&mut cx, "flaky-report:qa:b");
    assert_eq!(h.backend.calls().len(), 1, "a run in progress");
    h.update(&mut cx, |s| s.run.active = false);
    click(&mut cx, "flaky-report:qa:b");
    assert_eq!(h.backend.calls().len(), 2);
}

/// 7: another environment in force moves its box to the top.
#[gpui::test]
fn follows_the_environment_in_force(cx: &mut TestAppContext) {
    let envs = vec![env(Some("dev"), pair("a", "b")), env(Some("qa"), pair("c", "d"))];
    let (h, mut cx) = setup(cx, 1008., state(envs));
    assert!(top(&mut cx, "dev") < top(&mut cx, "qa"), "none in force: by name");
    h.update(&mut cx, |s| s.env = Some(in_force("qa")));
    assert!(top(&mut cx, "qa") < top(&mut cx, "dev"));
}

/// 8: a test that is not flaky, and a feature, show no box.
#[gpui::test]
fn shows_no_proof_for_a_test_that_is_not_flaky_or_a_group(cx: &mut TestAppContext) {
    let (h, mut cx) = setup(cx, 1008., state(vec![env(Some("qa"), pair("a", "b"))]));
    assert!(bounds(&mut cx, "flaky-proof:qa").is_some());
    h.update(&mut cx, |s| s.selected = Some(STEADY.into()));
    assert!(bounds(&mut cx, "flaky-proof:qa").is_none());
    h.update(&mut cx, |s| s.selected = Some("f:tests/x.feature".into()));
    assert!(bounds(&mut cx, "flaky-proof:qa").is_none());
}

/// A press is a click at the centre of the button: every Report must lie inside the pane, at CI's window and at the
/// window's least width, with the longest words a row has (a dated run, "Failed", "8 threads").
#[gpui::test]
fn every_report_lies_inside_the_pane_at_1008_and_760_px(cx: &mut TestAppContext) {
    for width in [1008., 760.] {
        let old = FlakyRun {
            started_at: 1_700_000_000_000.,
            ..run("old", FlakyStatus::Failed, Some(8), true)
        };
        let envs = vec![env(Some("qa"), vec![old, run("b", FlakyStatus::Passed, Some(1), true)])];
        let (_h, mut vcx) = setup(cx, width, state(envs));
        let pane = at(&mut vcx, "details");
        for id in ["old", "b"] {
            let b = at(&mut vcx, &format!("flaky-report:qa:{id}"));
            assert!(
                pane.contains(&b.center()) && b.right() <= pane.right(),
                "{id} at {width}: {b:?} in {pane:?}"
            );
        }
    }
}
