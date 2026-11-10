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

/// `tests/o.feature`: an outline of two rows.
fn outline_state() -> AppState {
    let file = "tests/o.feature";
    let rows = (1..=2)
        .map(|i| node(&format!("r:{file}#0.{i}"), NodeKind::Row, vec![]))
        .collect();
    let outline = node(&format!("o:{file}#0"), NodeKind::Outline, rows);
    let feature = node(&format!("f:{file}"), NodeKind::Feature, vec![outline]);
    let mut s = testing::project("/p", Layout::Standalone, 0);
    s.tree = Some(node(
        "d:",
        NodeKind::Folder,
        vec![node("d:tests", NodeKind::Folder, vec![feature])],
    ));
    s
}

/// TO-09: opening a feature shows its outline open, with its rows.
#[gpui::test]
fn an_outline_opens_with_its_feature(cx: &mut TestAppContext) {
    let (_h, mut cx, tree) = setup(cx, outline_state());
    toggle(&tree, "f:tests/o.feature", &mut cx);
    assert_eq!(
        keys(&tree, &mut cx),
        [
            "d:tests",
            "f:tests/o.feature",
            "o:tests/o.feature#0",
            "r:tests/o.feature#0.1",
            "r:tests/o.feature#0.2"
        ]
    );
}

/// TO-14
#[gpui::test]
fn a_failure_leaves_a_closed_outline_closed(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, outline_state());
    toggle(&tree, "f:tests/o.feature", &mut cx);
    toggle(&tree, "o:tests/o.feature#0", &mut cx);
    run(&h, &mut cx, "r1", "d:");
    fail(&h, &mut cx, "r:tests/o.feature#0.2");
    assert_eq!(
        keys(&tree, &mut cx),
        ["d:tests", "f:tests/o.feature", "o:tests/o.feature#0"]
    );
}

/// TO-18: a run that opens a feature above the view moves the list by its new rows, so the rows in view stay.
#[gpui::test]
fn a_feature_opening_above_the_view_does_not_move_it(cx: &mut TestAppContext) {
    let names: Vec<String> = (0..80).map(|i| format!("f{i:02}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let (h, mut cx, tree) = setup(cx, state("/p", &names));
    let offset = |tree: &Entity<Tree>, cx: &mut VisualTestContext| cx.update(|_, cx| tree.read(cx).view().scroll_top);
    // "f00.feature" is the second row; 20 rows down it is above the view
    let top = PAD + 20. * ROW_HEIGHT;
    tree.update(&mut cx, |t, _| {
        t.scroll
            .0
            .borrow()
            .base_handle
            .set_offset(gpui::point(gpui::px(0.), gpui::px(-top)));
    });
    cx.run_until_parked();
    assert_eq!(offset(&tree, &mut cx), top);
    run(&h, &mut cx, "r1", "d:");
    fail(&h, &mut cx, "s:tests/f00.feature#1");
    cx.run_until_parked();
    assert!(cx.update(|_, cx| tree.read(cx).is_open("f:tests/f00.feature")));
    assert_eq!(offset(&tree, &mut cx), top + 2. * ROW_HEIGHT);
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

/// 80 features, the tree focused from the keyboard on its first row.
fn many(cx: &mut TestAppContext) -> (Harness, VisualTestContext, Entity<Tree>) {
    let names: Vec<String> = (0..80).map(|i| format!("f{i:02}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let (h, mut cx, tree) = setup(cx, state("/p", &names));
    focus(&tree, &mut cx);
    (h, cx, tree)
}

fn focus(tree: &Entity<Tree>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let f = tree.read(cx).focus.clone();
        window.focus(&f, cx);
    });
    cx.run_until_parked();
}

fn active(tree: &Entity<Tree>, cx: &mut VisualTestContext) -> Option<usize> {
    cx.update(|_, cx| {
        let t = tree.read(cx);
        t.active.as_ref().and_then(|a| t.keys.iter().position(|k| k == a))
    })
}

fn selected(h: &Harness, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| h.grace.read(cx).state.selected.clone())
}

/// Whether row `i` is wholly in the tree's view.
fn in_view(tree: &Entity<Tree>, i: usize, cx: &mut VisualTestContext) -> bool {
    let v = cx.update(|_, cx| tree.read(cx).view());
    let top = PAD + i as f32 * ROW_HEIGHT;
    top >= v.scroll_top - 0.5 && top + ROW_HEIGHT <= v.scroll_top + v.height + 0.5
}

fn at_row(i: usize, level: usize, x: f32) -> gpui::Point<gpui::Pixels> {
    gpui::point(
        gpui::px(8. + (level - 1) as f32 * INDENT + x),
        gpui::px(PAD + (i as f32 + 0.5) * ROW_HEIGHT),
    )
}

/// VL-09: End goes to the last row, Page Up one view above, Page Down one view below, Home to the first; each
/// selects its row and brings it into view.
#[gpui::test]
fn home_end_and_pages(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = many(cx);
    let last = keys(&tree, &mut cx).len() - 1;
    cx.simulate_keystrokes("end");
    assert_eq!(active(&tree, &mut cx), Some(last));
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("f:tests/f79.feature"));
    assert!(in_view(&tree, last, &mut cx));
    let page = page_rows(cx.update(|_, cx| tree.read(cx).view()));
    assert!(page > 5, "a page of {page} rows");
    cx.simulate_keystrokes("pageup");
    assert_eq!(active(&tree, &mut cx), Some(last - page));
    assert!(in_view(&tree, last - page, &mut cx));
    cx.simulate_keystrokes("pageup pagedown");
    assert_eq!(active(&tree, &mut cx), Some(last - page));
    cx.simulate_keystrokes("home");
    assert_eq!(active(&tree, &mut cx), Some(0));
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("d:tests"));
    assert!(in_view(&tree, 0, &mut cx));
}

/// VL-15, E-virtual-03: the wheel scrolls the focus row away and the focus stays on it; the next key brings the list
/// back to it and moves on.
#[gpui::test]
fn focus_survives_scrolling_away(cx: &mut TestAppContext) {
    let (_h, mut cx, tree) = many(cx);
    for _ in 0..9 {
        cx.simulate_keystrokes("down");
    }
    assert_eq!(active(&tree, &mut cx), Some(9));
    cx.simulate_event(gpui::ScrollEvent {
        position: at_row(3, 2, 100.),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-100. * ROW_HEIGHT))),
        ..Default::default()
    });
    cx.run_until_parked();
    assert!(!in_view(&tree, 9, &mut cx), "scrolled away");
    assert!(cx.update(|window, cx| tree.read(cx).focus.is_focused(window)));
    assert_eq!(active(&tree, &mut cx), Some(9));
    cx.simulate_keystrokes("down");
    assert_eq!(active(&tree, &mut cx), Some(10));
    assert!(in_view(&tree, 10, &mut cx));
}

/// VL-12, E-virtual-07: a click on a row makes it the focus row, without the ring; the next Down moves on from it.
#[gpui::test]
fn a_click_makes_the_row_focused(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = many(cx);
    cx.simulate_click(at_row(4, 2, 150.), gpui::Modifiers::none());
    assert_eq!(active(&tree, &mut cx), Some(4));
    assert!(cx.update(|window, cx| tree.read(cx).focus.is_focused(window)));
    assert!(!cx.update(|_, cx| tree.read(cx).keyboard), "no ring after a click");
    cx.simulate_keystrokes("down");
    let (i, key) = (5, keys(&tree, &mut cx)[5].clone());
    assert_eq!(active(&tree, &mut cx), Some(i));
    assert_eq!(selected(&h, &mut cx), Some(key));
    assert!(
        cx.update(|_, cx| tree.read(cx).keyboard),
        "the ring once a key moves it"
    );
}

/// VL-16: the chevron that closes the focus row's feature takes the focus to the feature and keeps the selection.
#[gpui::test]
fn closing_a_feature_with_its_chevron_takes_the_focus_to_it(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    toggle(&tree, "f:tests/a.feature", &mut cx);
    cx.simulate_click(at_row(2, 3, 150.), gpui::Modifiers::none());
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("s:tests/a.feature#0"));
    cx.simulate_click(at_row(1, 2, ROW_HEIGHT / 2.), gpui::Modifiers::none());
    assert!(!cx.update(|_, cx| tree.read(cx).is_open("f:tests/a.feature")));
    assert_eq!(active(&tree, &mut cx), Some(1));
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("s:tests/a.feature#0"));
}

/// VL-21, E-virtual-09: the keyboard moving to a row whose label is cut shows the whole label and the feature's title;
/// moving on closes it.
/// A click on that row shows it neither at once nor after the pause (E-virtual-07).
#[gpui::test]
fn a_cut_label_shows_whole_on_keyboard_focus(cx: &mut TestAppContext) {
    let long = "a".repeat(400);
    let (h, mut cx, tree) = setup(cx, state("/p", &["b", &long]));
    // the file name, then the feature's title (here its key)
    let whole = format!("{long}.feature · f:tests/{long}.feature");
    focus(&tree, &mut cx);
    cx.simulate_keystrokes("down");
    assert_eq!(h.tip(&mut cx), None, "b.feature is whole");
    cx.simulate_keystrokes("down");
    assert_eq!(h.tip(&mut cx).as_deref(), Some(whole.as_str()));
    cx.simulate_keystrokes("up");
    assert_eq!(h.tip(&mut cx), None);

    cx.simulate_click(at_row(2, 2, 150.), gpui::Modifiers::none());
    cx.executor().advance_clock(crate::tooltip::DELAY);
    cx.run_until_parked();
    assert_eq!(active(&tree, &mut cx), Some(2));
    assert_eq!(h.tip(&mut cx), None);
}

/// #153: the focus ring, the row's border, starts at the row's indent, on a feature as on a folder.
#[gpui::test]
fn the_focus_ring_starts_at_the_rows_indent(cx: &mut TestAppContext) {
    let (_h, mut cx, tree) = many(cx);
    cx.simulate_keystrokes("down");
    assert!(cx.update(|_, cx| tree.read(cx).keyboard));
    let folder = cx.debug_bounds("tree-row:d:tests").unwrap();
    let feature = cx.debug_bounds("tree-row:f:tests/f00.feature").unwrap();
    assert_eq!(folder.left(), gpui::px(8.));
    assert_eq!(feature.left(), gpui::px(8. + INDENT));
    assert_eq!(feature.right(), folder.right(), "both end before the list's edge");
}

fn is_open(tree: &Entity<Tree>, key: &str, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| tree.read(cx).is_open(key))
}

fn scroll_top(tree: &Entity<Tree>, cx: &mut VisualTestContext) -> f32 {
    cx.update(|_, cx| tree.read(cx).view().scroll_top)
}

/// Scrolls the list with the wheel, over its fourth row.
fn wheel(cx: &mut VisualTestContext, rows: f32) {
    cx.simulate_event(gpui::ScrollEvent {
        position: at_row(3, 2, 100.),
        delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-rows * ROW_HEIGHT))),
        ..Default::default()
    });
    cx.run_until_parked();
}

/// VL-06 (`VirtualList.test.tsx` "has one tab stop: the selected row, else the first"): the focus row of a tree that
/// takes the focus is the selected row, else the first.
#[gpui::test]
fn the_focus_row_is_the_selected_one_else_the_first(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    let at = |tree: &Entity<Tree>, h: &Harness, cx: &mut VisualTestContext| {
        cx.update(|_, cx| tree.read(cx).focus_index(&h.grace.read(cx).state))
    };
    focus(&tree, &mut cx);
    assert_eq!(at(&tree, &h, &mut cx), Some(0));
    h.update(&mut cx, |s| s.selected = Some("f:tests/b.feature".into()));
    assert_eq!(at(&tree, &h, &mut cx), Some(2));
}

/// VL-07, TO-08 ("selects with Enter and Space", "opens no feature with the keys that select"): Enter and Space
/// select the focus row and open nothing; Right opens it.
#[gpui::test]
fn enter_and_space_select_the_focus_row_and_open_nothing(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    focus(&tree, &mut cx);
    cx.simulate_keystrokes("down");
    for key in ["enter", "space"] {
        // the details pane chose another row meanwhile
        h.update(&mut cx, |s| s.selected = Some("f:tests/b.feature".into()));
        tree.update(&mut cx, |t, _| t.active = Some("f:tests/a.feature".into()));
        cx.simulate_keystrokes(key);
        assert_eq!(selected(&h, &mut cx).as_deref(), Some("f:tests/a.feature"), "{key}");
        assert!(!is_open(&tree, "f:tests/a.feature", &mut cx), "{key} opens nothing");
    }
    cx.simulate_keystrokes("right");
    assert!(is_open(&tree, "f:tests/a.feature", &mut cx));
    assert_eq!(active(&tree, &mut cx), Some(1), "the focus stays on it");
}

/// VL-15 (R4.2): a key that acts on a scrolled-away focus row brings it back into view first.
#[gpui::test]
fn a_scrolled_away_focus_row_comes_back_before_enter_or_right(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = many(cx);
    for _ in 0..9 {
        cx.simulate_keystrokes("down");
    }
    for key in ["enter", "right"] {
        wheel(&mut cx, 100.);
        assert!(!in_view(&tree, 9, &mut cx), "scrolled away");
        cx.simulate_keystrokes(key);
        assert!(in_view(&tree, 9, &mut cx), "{key} brings it back");
        assert_eq!(selected(&h, &mut cx).as_deref(), Some("f:tests/f08.feature"));
    }
    assert!(is_open(&tree, "f:tests/f08.feature", &mut cx));
}

/// VL-09, TO-08: Up on the first row and Down on the last do nothing and keep the list still; Left on a closed
/// top-level folder does nothing.
#[gpui::test]
fn the_keys_do_nothing_at_the_ends(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = many(cx);
    cx.simulate_keystrokes("up");
    assert_eq!(selected(&h, &mut cx), None);
    assert_eq!(scroll_top(&tree, &mut cx), 0.);
    cx.simulate_keystrokes("end");
    let last = keys(&tree, &mut cx).len() - 1;
    let top = scroll_top(&tree, &mut cx);
    cx.simulate_keystrokes("down");
    assert_eq!(active(&tree, &mut cx), Some(last));
    assert_eq!(scroll_top(&tree, &mut cx), top);
    cx.simulate_keystrokes("home left");
    assert_eq!(keys(&tree, &mut cx), ["d:tests"], "Left closes the open folder");
    cx.simulate_keystrokes("left");
    assert_eq!(keys(&tree, &mut cx), ["d:tests"]);
    assert_eq!(active(&tree, &mut cx), Some(0));
    assert_eq!(selected(&h, &mut cx).as_deref(), Some("d:tests"));
}

/// VL-17: an active row that leaves while the selected row stays hands the focus to the selected row.
#[gpui::test]
fn an_active_row_that_leaves_falls_back_to_the_selected_one(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    toggle(&tree, "f:tests/b.feature", &mut cx);
    h.update(&mut cx, |s| s.selected = Some("f:tests/a.feature".into()));
    tree.update(&mut cx, |t, _| t.active = Some("s:tests/b.feature#1".into()));
    toggle(&tree, "f:tests/b.feature", &mut cx);
    let at = cx.update(|_, cx| tree.read(cx).focus_index(&h.grace.read(cx).state));
    assert_eq!(at, Some(1), "f:tests/a.feature");
}

/// VL-19: rows still show after the list shrinks below where it was scrolled.
#[gpui::test]
fn rows_show_after_the_list_shrinks_below_the_scroll(cx: &mut TestAppContext) {
    let (_h, mut cx, tree) = many(cx);
    cx.simulate_keystrokes("end");
    assert!(scroll_top(&tree, &mut cx) > 0.);
    toggle(&tree, "d:tests", &mut cx);
    assert_eq!(scroll_top(&tree, &mut cx), 0.);
    assert!(cx.debug_bounds("tree-row:d:tests").is_some(), "the row is drawn");
}

/// VL-20, TO-17: the runner sending the same filter after a refresh does not scroll; a new filter brings the selected
/// row back into view.
#[gpui::test]
fn only_a_new_filter_scrolls_to_the_selected_row(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = many(cx);
    cx.simulate_keystrokes("end");
    let last = keys(&tree, &mut cx).len() - 1;
    wheel(&mut cx, -200.);
    assert!(!in_view(&tree, last, &mut cx));
    let again = h.grace.read_with(&cx, |g, _| (*g.state).clone());
    h.set(&mut cx, again);
    assert!(!in_view(&tree, last, &mut cx), "the same filter: no scroll");
    h.update(&mut cx, |s| s.filter.include = vec!["nothing".into()]);
    assert!(in_view(&tree, last, &mut cx), "a new filter: the selected row in view");
}

/// VL-10 ("runs the focused row with Ctrl+Enter, and not during a run", "runs nothing on a row the filter mutes"):
/// Ctrl+Enter runs the focus row, selecting nothing; it runs nothing during a run, while busy, or on a dimmed row.
#[gpui::test]
fn ctrl_enter_runs_the_focus_row_only_when_its_run_button_would(cx: &mut TestAppContext) {
    let mut s = state("/p", &["a", "b"]);
    if let Some(t) = &mut s.tree {
        t.children[0].children[1].children[0].tags = vec!["smoke".into()];
    }
    let (h, mut cx, tree) = setup(cx, s);
    focus(&tree, &mut cx);
    let runs = |h: &Harness| {
        h.backend
            .calls()
            .into_iter()
            .filter(|c| c.starts_with("run "))
            .collect::<Vec<_>>()
    };
    cx.simulate_keystrokes("down");
    h.update(&mut cx, |s| s.selected = None);
    cx.simulate_keystrokes("ctrl-enter");
    cx.run_until_parked();
    assert_eq!(runs(&h), ["run f:tests/a.feature"]);
    assert_eq!(selected(&h, &mut cx), None, "a run selects nothing");
    h.update(&mut cx, |s| s.run.active = true);
    cx.simulate_keystrokes("ctrl-enter");
    h.update(&mut cx, |s| {
        s.run.active = false;
        s.busy = true;
    });
    cx.simulate_keystrokes("ctrl-enter");
    h.update(&mut cx, |s| {
        s.busy = false;
        s.filter.include = vec!["smoke".into()];
    });
    cx.simulate_keystrokes("ctrl-enter");
    cx.run_until_parked();
    assert_eq!(runs(&h).len(), 1, "a run, busy, then a dimmed row: {:?}", runs(&h));
}

/// R4.6 ("disables run buttons while a run is active"): a row's run button runs nothing during a run or while the
/// runner is busy. Linux's AccessKit drops the disabled state, so this is headless.
#[gpui::test]
fn the_run_buttons_run_nothing_during_a_run_or_while_busy(cx: &mut TestAppContext) {
    let (h, mut cx, _tree) = setup(cx, state("/p", &["a"]));
    let runs = |h: &Harness| h.backend.calls().into_iter().filter(|c| c.starts_with("run ")).count();
    let run_button = |cx: &mut VisualTestContext| {
        let row = cx.debug_bounds("tree-row:f:tests/a.feature").expect("the row");
        gpui::point(row.right() - gpui::px(ROW_HEIGHT / 2.), row.center().y)
    };
    for during in [true, false] {
        h.update(&mut cx, |s| {
            s.run.active = during;
            s.busy = !during;
        });
        let at = run_button(&mut cx);
        cx.simulate_click(at, gpui::Modifiers::none());
        cx.run_until_parked();
        assert_eq!(runs(&h), 0, "during a run: {during}");
    }
    h.update(&mut cx, |s| s.busy = false);
    let at = run_button(&mut cx);
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(runs(&h), 1, "and runs once free");
}

/// TO-12 ("never reopens a feature closed during the run"): a feature open before the run, or opened during it,
/// then closed, stays closed on a later failure in it.
#[gpui::test]
fn a_feature_the_person_closed_during_a_run_stays_closed(cx: &mut TestAppContext) {
    for before in [true, false] {
        let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
        if before {
            toggle(&tree, "f:tests/a.feature", &mut cx);
        }
        run(&h, &mut cx, "r1", "d:");
        if !before {
            toggle(&tree, "f:tests/a.feature", &mut cx);
        }
        assert!(is_open(&tree, "f:tests/a.feature", &mut cx));
        toggle(&tree, "f:tests/a.feature", &mut cx);
        fail(&h, &mut cx, "s:tests/a.feature#1");
        assert!(
            !is_open(&tree, "f:tests/a.feature", &mut cx),
            "open before the run: {before}"
        );
    }
}

/// `Tree.test.tsx` "icons folders and features": a folder's icon opens with it, a feature has its file, and a
/// test has none.
#[test]
fn a_folder_and_a_feature_have_their_icon_and_a_test_none() {
    assert_eq!(kind_icon(NodeKind::Folder, true), Some(Icon::FolderOpen));
    assert_eq!(kind_icon(NodeKind::Folder, false), Some(Icon::Folder));
    assert_eq!(kind_icon(NodeKind::Feature, false), Some(Icon::FileText));
    for kind in [NodeKind::Scenario, NodeKind::Outline, NodeKind::Row] {
        assert_eq!(kind_icon(kind, false), None, "{kind:?}");
    }
}

/// `Tree.test.tsx` "shows the run button muted, green when its row is hovered, focused or selected".
#[gpui::test]
fn the_run_button_is_green_on_a_hovered_focused_or_selected_row(cx: &mut TestAppContext) {
    assert!(!run_is_green(false, false, false));
    assert!(run_is_green(true, false, false) && run_is_green(false, true, false) && run_is_green(false, false, true));
    let (_h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    let hovered = |tree: &Entity<Tree>, cx: &mut VisualTestContext| cx.update(|_, cx| tree.read(cx).hovered.clone());
    cx.simulate_mouse_move(at_row(1, 2, 150.), None, gpui::Modifiers::none());
    assert_eq!(hovered(&tree, &mut cx).as_deref(), Some("f:tests/a.feature"));
    cx.simulate_mouse_move(at_row(2, 2, 150.), None, gpui::Modifiers::none());
    assert_eq!(hovered(&tree, &mut cx).as_deref(), Some("f:tests/b.feature"));
    cx.simulate_mouse_move(
        gpui::point(gpui::px(150.), gpui::px(700.)),
        None,
        gpui::Modifiers::none(),
    );
    assert_eq!(hovered(&tree, &mut cx), None);
}

/// `Tree.test.tsx` "keeps a row's tooltips on its row": the mark's on its right, the cut label's over it, the run
/// button's on its left.
#[gpui::test]
fn a_rows_tooltips_stay_on_its_row(cx: &mut TestAppContext) {
    let long = "a".repeat(400);
    let (h, mut cx, _tree) = setup(cx, state("/p", &[&long, "b"]));
    let cut_row = cx
        .debug_bounds(format!("tree-row:f:tests/{long}.feature").leak())
        .expect("the long row");
    let row = cx.debug_bounds("tree-row:f:tests/b.feature").expect("the row");
    let rest = |cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>| {
        cx.simulate_mouse_move(gpui::point(gpui::px(5.), gpui::px(700.)), None, gpui::Modifiers::none());
        cx.simulate_mouse_move(at, None, gpui::Modifiers::none());
        cx.executor().advance_clock(crate::tooltip::DELAY);
        cx.run_until_parked();
    };
    let on_row = |b: gpui::Bounds<gpui::Pixels>| (b.center().y - row.center().y).abs() < gpui::px(1.);
    // the chevron, then the 16 px mark
    let mark_x = row.left() + gpui::px(ROW_HEIGHT + 4. + 8.);
    rest(&mut cx, gpui::point(mark_x, row.center().y));
    assert_eq!(h.tip(&mut cx).as_deref(), Some("Not run"));
    let tip = h.tip_bounds(&mut cx).unwrap();
    assert!(tip.left() > mark_x && on_row(tip), "right of the mark: {tip:?}");

    rest(
        &mut cx,
        gpui::point(cut_row.left() + gpui::px(200.), cut_row.center().y),
    );
    assert!(h.tip(&mut cx).is_some_and(|t| t.starts_with("aaaa")));
    let tip = h.tip_bounds(&mut cx).unwrap();
    assert!(
        tip.top() >= cut_row.top() - gpui::px(1.) && tip.top() < cut_row.center().y,
        "over the label: {tip:?}"
    );

    let run_x = row.right() - gpui::px(ROW_HEIGHT / 2.);
    rest(&mut cx, gpui::point(run_x, row.center().y));
    assert_eq!(h.tip(&mut cx).as_deref(), Some("Run b.feature (Ctrl+Enter)"));
    let tip = h.tip_bounds(&mut cx).unwrap();
    assert!(tip.right() < run_x && on_row(tip), "left of the run button: {tip:?}");
}

/// `Tree.test.tsx`'s flaky marks, drawn ahead of P3's proof: a flaky test has its mark, its feature counts it, the
/// mark's tooltip goes on its left, and a test the filter dims keeps it.
#[gpui::test]
fn a_flaky_test_has_its_mark_counted_above_and_explained_on_its_left(cx: &mut TestAppContext) {
    let mut s = state("/p", &["a"]);
    s.flaky.insert(
        "s:tests/a.feature#0".into(),
        vec![grace_core::model::flaky::FlakyEnv {
            env: None,
            passed: 2,
            failed: 1,
            runs: vec![],
        }],
    );
    let (h, mut cx, tree) = setup(cx, s);
    toggle(&tree, "f:tests/a.feature", &mut cx);
    assert!(
        cx.debug_bounds("flaky-mark:f:tests/a.feature").is_some(),
        "counted on its feature"
    );
    assert!(cx.debug_bounds("flaky-mark:s:tests/a.feature#1").is_none());
    let mark = cx
        .debug_bounds("flaky-mark:s:tests/a.feature#0")
        .expect("the test's mark");
    cx.simulate_mouse_move(mark.center(), None, gpui::Modifiers::none());
    cx.executor().advance_clock(crate::tooltip::DELAY);
    cx.run_until_parked();
    assert_eq!(h.tip(&mut cx).as_deref(), Some(grace_core::model::flaky::FLAKY_TITLE));
    let tip = h.tip_bounds(&mut cx).unwrap();
    assert!(
        tip.right() < mark.left(),
        "left of the mark, not over the next row: {tip:?}"
    );
    h.update(&mut cx, |s| s.filter.include = vec!["nothing".into()]);
    assert!(
        cx.debug_bounds("flaky-mark:s:tests/a.feature#0").is_some(),
        "a dimmed test keeps it"
    );
}
