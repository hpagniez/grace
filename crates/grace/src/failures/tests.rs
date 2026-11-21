//! `ConfigBanner.test.tsx`'s tests (#229 R6), headless: the banner after a run that failed on `karate-config.js`.
use gpui::{Bounds, IntoElement as _, Modifiers, ParentElement as _, Pixels, TestAppContext, VisualTestContext, div};
use grace_core::model::node::{Layout, NodeKind, Status, TestNode};
use grace_core::state::{AppState, TestResult};

use crate::testing::{self, Harness};

const FIRST: &str = "s:tests/shared.feature#1";
const SECOND: &str = "s:tests/shared.feature#2";
const REASON: &str = "Config evaluation failed: karate-config.js - js failed:\n==========\n  Line: 4, Col: 2\n  Code: karate.callSingle('classpath:auth.feature', { baseUrl: baseUrl });\n  Error: callSingle failed: classpath:auth.feature - expected status: 200, actual: 201\n==========";

fn leaf(key: &str, name: &str, line: u32) -> TestNode {
    TestNode {
        key: key.into(),
        kind: NodeKind::Scenario,
        name: name.into(),
        file: "tests/shared.feature".into(),
        line: Some(line),
        ref_id: None,
        tags: vec![],
        example: None,
        children: vec![],
    }
}

fn node(kind: NodeKind, key: &str, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: "p".into(),
        file: String::new(),
        line: None,
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}

/// A finished run whose `failed` tests (key, error) failed.
fn after_a_run(failed: &[(&str, &str)]) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(node(
        NodeKind::Folder,
        "d:",
        vec![node(
            NodeKind::Feature,
            "f:tests/shared.feature",
            vec![leaf(FIRST, "first", 7), leaf(SECOND, "second", 10)],
        )],
    ));
    for (key, error) in failed {
        s.statuses.insert(key.to_string(), Status::Failed);
        s.results.insert(
            key.to_string(),
            TestResult {
                duration_millis: None,
                error: Some(error.to_string()),
                reason_at: Some(0),
            },
        );
        s.run.failed_keys.push(key.to_string());
    }
    s
}

fn setup(cx: &mut TestAppContext, state: AppState) -> (Harness, VisualTestContext) {
    testing::open(cx, 800., state, move |grace, _, cx| {
        let state = grace.read(cx).state.clone();
        div().children(super::banner(grace, &state, cx)).into_any_element()
    })
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let b = bounds(cx, id).unwrap_or_else(|| panic!("{id} is drawn"));
    cx.simulate_click(b.center(), Modifiers::default());
    cx.run_until_parked();
}

// GF-18
#[gpui::test]
fn names_the_config_file_and_opens_it_at_its_line(cx: &mut TestAppContext) {
    let (h, mut cx) = setup(cx, after_a_run(&[(FIRST, REASON), (SECOND, REASON)]));
    assert!(bounds(&mut cx, "config-failure").is_some());
    assert!(bounds(&mut cx, "config-failure-reason").is_some());
    click(&mut cx, "config-failure-open");
    assert_eq!(h.backend.calls(), ["editor_file karate-config.js"]);
    assert_eq!(h.seams.urls(), ["vscode://file/karate-config.js:4"]);
}

// GF-19
#[gpui::test]
fn opens_line_1_when_the_message_has_none(cx: &mut TestAppContext) {
    let reason = "Config evaluation failed: classpath:karate-config-dev.js - boom";
    let (h, mut cx) = setup(cx, after_a_run(&[(FIRST, reason)]));
    click(&mut cx, "config-failure-open");
    assert_eq!(h.seams.urls(), ["vscode://file/karate-config-dev.js:1"]);
}

// GF-20
#[gpui::test]
fn goes_while_a_run_goes_on(cx: &mut TestAppContext) {
    let (h, mut cx) = setup(cx, after_a_run(&[(FIRST, REASON), (SECOND, REASON)]));
    assert!(bounds(&mut cx, "config-failure").is_some());
    h.update(&mut cx, |s| s.run.active = true);
    assert!(bounds(&mut cx, "config-failure").is_none());
}

// GF-20: the next run's start empties the last run's failed tests
#[gpui::test]
fn counts_only_the_last_runs_failures(cx: &mut TestAppContext) {
    let (h, mut cx) = setup(cx, after_a_run(&[(FIRST, REASON), (SECOND, REASON)]));
    h.update(&mut cx, |s| s.run.failed_keys = vec![FIRST.into()]);
    assert!(bounds(&mut cx, "config-failure").is_some());
    h.update(&mut cx, |s| s.run.failed_keys.clear());
    assert!(bounds(&mut cx, "config-failure").is_none());
}

// GF-21
#[gpui::test]
fn shows_nothing_for_ordinary_failures(cx: &mut TestAppContext) {
    let (_, mut cx) = setup(cx, after_a_run(&[(FIRST, "C:/p/x.feature:7\nThen status 200\nboom")]));
    assert!(bounds(&mut cx, "config-failure").is_none());
}

#[test]
fn says_the_count_in_the_singular_and_the_plural() {
    assert_eq!(
        super::headline("karate-config.js", 1),
        "karate-config.js failed for 1 test"
    );
    assert_eq!(
        super::headline("karate-config.js", 24),
        "karate-config.js failed for 24 tests"
    );
}
