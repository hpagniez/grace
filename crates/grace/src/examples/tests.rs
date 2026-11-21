//! `Examples.test.tsx`'s view tests, headless on the fake runner: the table drawn alone in a pane, and once inside
//! Details. What a screen reader reads, and the real runs, are `grace-e2e/tests/examples.rs`'s; what the table holds
//! is `grace_core::examples::tests`'.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AppContext as _, Bounds, Entity, IntoElement as _, Modifiers, ParentElement as _, Pixels, Styled as _,
    TestAppContext, VisualTestContext, div, px,
};
use grace_core::model::node::{Layout, NodeKind, Status, TestNode};
use grace_core::state::{AppState, TestResult};

use crate::details::Details;
use crate::examples::Examples;
use crate::testing::{self, Harness};

const F: &str = "tests/o.feature";
const OUTLINE: &str = "o:tests/o.feature#1";

fn row_key(n: usize) -> String {
    format!("s:{F}#1.{n}")
}

fn node(kind: NodeKind, key: &str, name: &str, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: name.into(),
        file: if kind == NodeKind::Folder {
            String::new()
        } else {
            F.into()
        },
        line: Some(3),
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}

fn row(n: usize, values: &[(&str, String)]) -> TestNode {
    let mut r = node(NodeKind::Row, &row_key(n), &format!("row {n}"), vec![]);
    r.example = Some(values.iter().map(|(k, v)| (k.to_string(), v.clone())).collect());
    r
}

fn tree(rows: Vec<TestNode>) -> TestNode {
    node(
        NodeKind::Folder,
        "d:",
        "p",
        vec![node(
            NodeKind::Feature,
            &format!("f:{F}"),
            "o",
            vec![
                node(NodeKind::Outline, OUTLINE, "add <a> and <b>", rows),
                node(NodeKind::Scenario, &format!("s:{F}#2"), "plain", vec![]),
            ],
        )],
    )
}

fn three() -> Vec<TestNode> {
    [("1", "2", "3"), ("2", "3", "5"), ("5", "5", "10")]
        .iter()
        .enumerate()
        .map(|(i, (a, b, e))| {
            row(
                i + 1,
                &[("a", a.to_string()), ("b", b.to_string()), ("expected", e.to_string())],
            )
        })
        .collect()
}

fn state(rows: Vec<TestNode>) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(tree(rows));
    s.selected = Some(OUTLINE.into());
    s
}

/// The table alone, in a pane 600 px wide and 300 px tall.
fn setup(cx: &mut TestAppContext, s: AppState) -> (Harness, VisualTestContext, Entity<Examples>) {
    let slot: Rc<RefCell<Option<Entity<Examples>>>> = Rc::default();
    let keep = Rc::clone(&slot);
    let (h, mut vcx) = testing::open_with(cx, 800., s, move |grace, _, cx| {
        let table = cx.new(|cx| Examples::new(grace.clone(), cx));
        *keep.borrow_mut() = Some(table.clone());
        Box::new(move |_, _, _| {
            div()
                .w(px(600.))
                .h(px(300.))
                .flex()
                .child(table.clone())
                .into_any_element()
        })
    });
    vcx.update(|window, _| window.refresh());
    vcx.run_until_parked();
    let table = slot.borrow().clone().unwrap();
    (h, vcx, table)
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(id.to_string().leak())
}

fn focus(table: &Entity<Examples>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let f = table.read(cx).focus.clone();
        window.focus(&f, cx);
    });
    cx.run_until_parked();
}

fn selected(h: &Harness, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| h.grace.read(cx).state.selected.clone())
}

/// 1 (DD-01, DD-06): a line per row, a cell per column, in the order the columns were first met; a column only some
/// rows have is an empty cell for the others.
#[gpui::test]
fn draws_a_line_per_row_with_a_cell_per_column(cx: &mut TestAppContext) {
    let (_h, mut cx, _t) = setup(cx, state(three()));
    assert!(bounds(&mut cx, "examples-head").is_some());
    for c in 0..3 {
        assert!(bounds(&mut cx, &format!("example-column:{c}")).is_some());
    }
    assert!(bounds(&mut cx, "example-column:3").is_none());
    let (first, last) = (
        bounds(&mut cx, "example:1").unwrap(),
        bounds(&mut cx, "example:3").unwrap(),
    );
    assert_eq!(f32::from(first.size.height), 24.);
    assert!(first.bottom() <= last.top());
    for c in 0..3 {
        assert!(bounds(&mut cx, &format!("example-value:3:{c}")).is_some());
    }
}

/// 1 (DD-06): a column only some rows have is an empty cell for the others.
#[gpui::test]
fn gives_the_other_rows_an_empty_cell_under_a_column_only_some_have(cx: &mut TestAppContext) {
    let (_h, mut cx, _t) = setup(
        cx,
        state(vec![
            row(1, &[("a", "1".into()), ("b", "2".into())]),
            row(2, &[("a", "3".into()), ("c", "4".into())]),
        ]),
    );
    for c in 0..3 {
        assert!(
            bounds(&mut cx, &format!("example-value:2:{c}")).is_some(),
            "a cell under column {c}"
        );
    }
}

/// 2 (DD-02): a line takes its status and time from the state as a run changes it, and is not drawn again from a
/// new table.
#[gpui::test]
fn takes_a_lines_result_from_the_state_without_making_the_table_again(cx: &mut TestAppContext) {
    let (h, mut cx, t) = setup(cx, state(three()));
    let before = cx.update(|_, cx| Rc::as_ptr(&t.read(cx).table()));
    h.update(&mut cx, |s| {
        s.statuses.insert(row_key(2), Status::Passed);
        s.results.insert(
            row_key(2),
            TestResult {
                duration_millis: Some(6.),
                error: None,
                reason_at: None,
            },
        );
    });
    assert!(bounds(&mut cx, "example-time:2").is_some());
    let after = cx.update(|_, cx| Rc::as_ptr(&t.read(cx).table()));
    assert_eq!(before, after, "a result redraws a line, not the table");
}

/// 3 (DD-03): a click on a line opens its row: the tree's selection is the row.
#[gpui::test]
fn a_click_opens_the_row(cx: &mut TestAppContext) {
    let (h, mut cx, _t) = setup(cx, state(three()));
    let b = bounds(&mut cx, "example:3").unwrap();
    cx.simulate_click(b.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(selected(&h, &mut cx), Some(row_key(3)));
}

/// 4 (DD-04, `VirtualList.test.tsx` 9): the arrow keys move the focus line and open nothing; Enter opens it.
#[gpui::test]
fn the_arrow_keys_move_without_opening_and_enter_opens(cx: &mut TestAppContext) {
    let (h, mut cx, t) = setup(cx, state(three()));
    focus(&t, &mut cx);
    cx.simulate_keystrokes("down");
    assert_eq!(selected(&h, &mut cx), Some(OUTLINE.into()), "a move opens nothing");
    assert!(
        bounds(&mut cx, "example-ring:2").is_some(),
        "the second line has the focus"
    );
    cx.simulate_keystrokes("end");
    assert!(bounds(&mut cx, "example-ring:3").is_some());
    cx.simulate_keystrokes("home");
    assert!(bounds(&mut cx, "example-ring:1").is_some());
    cx.simulate_keystrokes("down enter");
    assert_eq!(selected(&h, &mut cx), Some(row_key(2)));
}

/// 5 (DD-07): a big outline draws only the lines in view.
#[gpui::test]
fn draws_only_the_lines_in_view_of_a_big_outline(cx: &mut TestAppContext) {
    let rows = (1..=10_000).map(|i| row(i, &[("n", i.to_string())])).collect();
    let (_h, mut cx, t) = setup(cx, state(rows));
    assert!(bounds(&mut cx, "example:1").is_some());
    assert!(bounds(&mut cx, "example:60").is_none());
    focus(&t, &mut cx);
    cx.simulate_keystrokes("end");
    assert!(
        bounds(&mut cx, "example:10000").is_some(),
        "the end of the table is scrolled to"
    );
    assert!(bounds(&mut cx, "example:1").is_none());
}

/// 6: another outline's table opens at its top; a refresh of the same one keeps the place.
#[gpui::test]
fn keeps_its_place_on_a_refresh_and_starts_at_the_top_for_another_outline(cx: &mut TestAppContext) {
    let rows = || (1..=100).map(|i| row(i, &[("n", i.to_string())])).collect::<Vec<_>>();
    let (h, mut cx, t) = setup(cx, state(rows()));
    focus(&t, &mut cx);
    cx.simulate_keystrokes("end");
    assert!(bounds(&mut cx, "example:100").is_some());
    h.update(&mut cx, |s| {
        s.tree = Some(tree(rows()));
        s.trees += 1;
    });
    assert!(
        bounds(&mut cx, "example:100").is_some(),
        "the same outline keeps its place"
    );
    // a refresh draws the new values (Examples.test.tsx 6)
    h.update(&mut cx, |s| {
        let mut t = tree(rows());
        t.children[0].children[0].children[0] = row(1, &[("n", "changed".into())]);
        s.tree = Some(t);
        s.trees += 1;
    });
    assert_eq!(t.read_with(&cx, |e, _| e.table().lines[0].values.clone()), ["changed"]);
    h.update(&mut cx, |s| {
        let mut other = tree(rows());
        other.children[0].children[0].key = "o:tests/o.feature#5".into();
        s.tree = Some(other);
        s.selected = Some("o:tests/o.feature#5".into());
        s.trees += 1;
    });
    assert!(
        bounds(&mut cx, "example:1").is_some(),
        "another outline starts at the top"
    );
}

/// 7 (R2.1): Details draws the table under an outline's counts, and none for a scenario.
#[gpui::test]
fn details_shows_it_for_an_outline_only(cx: &mut TestAppContext) {
    let (h, mut cx) = testing::open_with(cx, 1008., state(three()), move |grace, _, cx| {
        let details = cx.new(|cx| Details::new(grace.clone(), cx));
        Box::new(move |_, _, _| {
            div()
                .w(px(504.))
                .h(px(600.))
                .flex()
                .child(details.clone())
                .into_any_element()
        })
    });
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let (counts, table) = (
        bounds(&mut cx, "details-counts").unwrap(),
        bounds(&mut cx, "examples").unwrap(),
    );
    assert!(counts.bottom() <= table.top());
    h.update(&mut cx, |s| s.selected = Some(format!("s:{F}#2")));
    assert!(bounds(&mut cx, "examples").is_none());
}
