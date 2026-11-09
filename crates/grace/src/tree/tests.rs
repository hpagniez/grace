//! `Tree.test.tsx`'s rules of which rows are open (TO-*), and `VirtualList.test.tsx`'s rows and focus row, headless
//! on the fake runner. Keys, clicks and what assistive technology reads are `grace-e2e/tests/tree.rs`'s.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, IntoElement as _, TestAppContext, VisualTestContext};
use grace_core::model::node::{Layout, NodeKind, Status, TestNode};
use grace_core::state::AppState;

use super::*;
use crate::testing::{self, Harness};

fn node(key: &str, kind: NodeKind, children: Vec<TestNode>) -> TestNode {
    let file = key
        .get(2..)
        .map(|k| k.split('#').next().unwrap_or_default().to_owned())
        .unwrap_or_default();
    TestNode {
        key: key.into(),
        kind,
        name: key.into(),
        file,
        line: Some(1),
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}

/// `tests/` holding a feature of two scenarios per name.
fn tree(features: &[&str]) -> TestNode {
    let features = features
        .iter()
        .map(|f| {
            let file = format!("tests/{f}.feature");
            node(
                &format!("f:{file}"),
                NodeKind::Feature,
                (0..2)
                    .map(|i| node(&format!("s:{file}#{i}"), NodeKind::Scenario, vec![]))
                    .collect(),
            )
        })
        .collect();
    node(
        "d:",
        NodeKind::Folder,
        vec![node("d:tests", NodeKind::Folder, features)],
    )
}

fn state(root: &str, features: &[&str]) -> AppState {
    let mut s = testing::project(root, Layout::Standalone, 0);
    s.tree = Some(tree(features));
    s
}

fn setup(cx: &mut TestAppContext, state: AppState) -> (Harness, VisualTestContext, Entity<Tree>) {
    let slot: Rc<RefCell<Option<Entity<Tree>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let (h, vcx) = testing::open(cx, 1200., state, move |grace, _, cx| {
        s.borrow_mut()
            .get_or_insert_with(|| cx.new(|cx| Tree::new(grace.clone(), cx)))
            .clone()
            .into_any_element()
    });
    let tree = slot.borrow().clone().unwrap();
    (h, vcx, tree)
}

fn keys(tree: &Entity<Tree>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| tree.read(cx).keys().to_vec())
}

fn toggle(tree: &Entity<Tree>, key: &str, cx: &mut VisualTestContext) {
    tree.update(cx, |t, cx| t.toggle(key, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn a_project_opens_with_its_features_closed(cx: &mut TestAppContext) {
    let (_h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    assert_eq!(
        keys(&tree, &mut cx),
        ["d:tests", "f:tests/a.feature", "f:tests/b.feature"]
    );
}

#[gpui::test]
fn a_refresh_keeps_what_was_opened_and_closed_and_a_new_feature_starts_closed(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    toggle(&tree, "f:tests/a.feature", &mut cx);
    h.set(&mut cx, state("/p", &["a", "b", "c"]));
    assert_eq!(
        keys(&tree, &mut cx),
        [
            "d:tests",
            "f:tests/a.feature",
            "s:tests/a.feature#0",
            "s:tests/a.feature#1",
            "f:tests/b.feature",
            "f:tests/c.feature"
        ]
    );
}

#[gpui::test]
fn another_project_starts_closed_again(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a"]));
    toggle(&tree, "f:tests/a.feature", &mut cx);
    toggle(&tree, "d:tests", &mut cx);
    assert_eq!(keys(&tree, &mut cx), ["d:tests"]);
    h.set(&mut cx, state("/q", &["a"]));
    assert_eq!(keys(&tree, &mut cx), ["d:tests", "f:tests/a.feature"]);
}

fn run(h: &Harness, cx: &mut VisualTestContext, id: &str, selection: &str) {
    h.update(cx, |s| {
        s.run.active = true;
        s.run.run_id = Some(id.into());
        s.run.selection = Some(selection.into());
        s.run.failed_keys.clear();
    });
}

fn fail(h: &Harness, cx: &mut VisualTestContext, key: &str) {
    h.update(cx, |s| s.run.failed_keys.push(key.into()));
}

#[gpui::test]
fn a_run_of_one_feature_opens_it_as_it_starts(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    run(&h, &mut cx, "r1", "f:tests/b.feature");
    assert_eq!(
        keys(&tree, &mut cx),
        [
            "d:tests",
            "f:tests/a.feature",
            "f:tests/b.feature",
            "s:tests/b.feature#0",
            "s:tests/b.feature#1"
        ]
    );
}

#[gpui::test]
fn run_all_opens_only_the_features_with_a_failure_and_never_one_closed_during_it(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b", "c"]));
    run(&h, &mut cx, "r1", "d:");
    assert_eq!(keys(&tree, &mut cx).len(), 4, "the root opens nothing");
    fail(&h, &mut cx, "s:tests/a.feature#1");
    assert!(cx.update(|_, cx| tree.read(cx).is_open("f:tests/a.feature")));
    // closed during the run, a second failure in it leaves it closed (TO-12)
    toggle(&tree, "f:tests/a.feature", &mut cx);
    fail(&h, &mut cx, "s:tests/a.feature#0");
    fail(&h, &mut cx, "s:tests/c.feature#0");
    assert_eq!(
        keys(&tree, &mut cx),
        [
            "d:tests",
            "f:tests/a.feature",
            "f:tests/b.feature",
            "f:tests/c.feature",
            "s:tests/c.feature#0",
            "s:tests/c.feature#1"
        ]
    );
    // what a run opened stays open after it (TO-15); the next run may open a feature again
    h.update(&mut cx, |s| s.run.active = false);
    assert!(cx.update(|_, cx| tree.read(cx).is_open("f:tests/c.feature")));
    run(&h, &mut cx, "r2", "d:");
    fail(&h, &mut cx, "s:tests/a.feature#0");
    assert!(cx.update(|_, cx| tree.read(cx).is_open("f:tests/a.feature")));
}

#[gpui::test]
fn a_run_opens_a_feature_inside_a_closed_folder_and_leaves_the_folder_closed(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a"]));
    toggle(&tree, "d:tests", &mut cx);
    run(&h, &mut cx, "r1", "d:");
    fail(&h, &mut cx, "s:tests/a.feature#0");
    assert_eq!(keys(&tree, &mut cx), ["d:tests"]);
    toggle(&tree, "d:tests", &mut cx);
    assert_eq!(keys(&tree, &mut cx).len(), 4);
}

#[gpui::test]
fn a_reveal_opens_the_rows_above_the_selected_test(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    h.update(&mut cx, |s| s.reveal("s:tests/b.feature#1"));
    assert!(keys(&tree, &mut cx).contains(&"s:tests/b.feature#1".to_owned()));
    assert!(!cx.update(|_, cx| tree.read(cx).is_open("f:tests/a.feature")));
}

#[gpui::test]
fn statuses_do_not_build_the_rows_again(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a"]));
    toggle(&tree, "f:tests/a.feature", &mut cx);
    let before = cx.update(|_, cx| tree.read(cx).rows());
    h.update(&mut cx, |s| {
        s.statuses.insert("s:tests/a.feature#0".into(), Status::Failed);
        s.statuses.insert("s:tests/a.feature#1".into(), Status::Running);
    });
    let after = cx.update(|_, cx| tree.read(cx).rows());
    assert!(Rc::ptr_eq(&before, &after));
}

#[gpui::test]
fn closing_the_feature_of_the_focus_row_hands_the_focus_to_the_nearest_row_above(cx: &mut TestAppContext) {
    let (_h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    toggle(&tree, "f:tests/b.feature", &mut cx);
    tree.update(&mut cx, |t, _| t.active = Some("s:tests/b.feature#1".into()));
    toggle(&tree, "f:tests/b.feature", &mut cx);
    assert_eq!(
        cx.update(|_, cx| tree.read(cx).active.clone()).as_deref(),
        Some("f:tests/b.feature")
    );
}

#[gpui::test]
fn hide_non_matching_leaves_the_matching_rows(cx: &mut TestAppContext) {
    let mut s = state("/p", &["a", "b"]);
    if let Some(t) = &mut s.tree {
        t.children[0].children[1].children[0].tags = vec!["smoke".into()];
    }
    let (h, mut cx, tree) = setup(cx, s);
    h.update(&mut cx, |s| {
        s.filter.include = vec!["smoke".into()];
        s.filter.hide_non_matching = true;
    });
    assert_eq!(keys(&tree, &mut cx), ["d:tests", "f:tests/b.feature"]);
}
