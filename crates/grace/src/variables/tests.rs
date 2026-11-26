//! `Variables.test.tsx` (step debugger R7.4, R7.5), headless on the fake runner, test for test, then what GPUI Grace
//! adds. What a screen reader reads is `grace-e2e/tests/debug.rs`'s.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity, Modifiers, TestAppContext, VisualTestContext, div, px};
use grace_core::model::events::{AppEvent, DebugVar, VarKind};
use grace_core::model::node::Layout;

use super::*;
use crate::testing::{self, Harness};

fn var(name: &str, kind: VarKind, value: &str) -> DebugVar {
    DebugVar {
        name: name.into(),
        kind,
        type_name: None,
        value: value.into(),
        cut: None,
    }
}

fn vars() -> Vec<DebugVar> {
    vec![
        var("user", VarKind::Json, r#"{"id":7,"tags":["a"]}"#),
        var("doc", VarKind::Xml, "<root><item>x</item></root>"),
        DebugVar {
            cut: Some(36_864),
            ..var("big", VarKind::String, &"x".repeat(10))
        },
        DebugVar {
            type_name: Some("LocalDate".into()),
            ..var("day", VarKind::Other, "2026-11-02")
        },
    ]
}

fn setup(cx: &mut TestAppContext) -> (Harness, VisualTestContext, Entity<Variables>) {
    let slot: Rc<RefCell<Option<Entity<Variables>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let mut state = testing::project("/p", Layout::Standalone, 1);
    state.run.active = true;
    state.run.debug = true;
    let (h, vcx) = testing::open_with(cx, 800., state, move |grace, _, cx| {
        let v = cx.new(|cx| Variables::new(grace.clone(), cx));
        *s.borrow_mut() = Some(v.clone());
        Box::new(move |_, _, _| div().h(px(600.)).flex().flex_col().child(v.clone()).into_any_element())
    });
    let view = slot.borrow().clone().unwrap();
    (h, vcx, view)
}

fn send(h: &Harness, cx: &mut VisualTestContext, e: AppEvent) {
    h.update(cx, |s| s.reduce(&e));
}

fn pause(h: &Harness, cx: &mut VisualTestContext, vars: Vec<DebugVar>) {
    let mut paused = testing::paused("s:a.feature#0");
    paused.vars = vars;
    send(h, cx, AppEvent::DebugPaused { paused });
}

fn at(cx: &mut VisualTestContext, id: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
}

fn toggle(cx: &mut VisualTestContext, name: &str) {
    let b = at(cx, &format!("variable-toggle:{name}")).unwrap_or_else(|| panic!("{name}'s toggle"));
    cx.simulate_click(b.center(), Modifiers::none());
    cx.run_until_parked();
}

fn rows(view: &Entity<Variables>, cx: &mut VisualTestContext) -> Rc<Vec<Row>> {
    cx.update(|_, cx| view.read(cx).rows())
}

fn opened(view: &Entity<Variables>, cx: &mut VisualTestContext, name: &str) -> Option<String> {
    cx.update(|_, cx| view.read(cx).opened(name).map(|s| s.to_string()))
}

/// 1: waits for the first pause.
#[gpui::test]
fn waits_for_the_first_pause(cx: &mut TestAppContext) {
    let (_h, mut cx, view) = setup(cx);
    assert!(at(&mut cx, "empty-state").is_some());
    assert!(rows(&view, &mut cx).is_empty());
    assert_eq!(WAITING, "Variables show when the run pauses");
    assert!(at(&mut cx, "variables-running").is_none());
}

/// 2: lists the variables by name, with their kind or Java type.
#[gpui::test]
fn lists_the_variables_by_name_with_their_kind_or_java_type(cx: &mut TestAppContext) {
    let (h, mut cx, view) = setup(cx);
    pause(&h, &mut cx, vars());
    assert!(at(&mut cx, "empty-state").is_none());
    let rows = rows(&view, &mut cx);
    let seen: Vec<(&str, &str)> = rows.iter().map(|r| (r.name.as_ref(), r.kind.as_ref())).collect();
    assert_eq!(
        seen,
        [
            ("big", "string"),
            ("day", "LocalDate"),
            ("doc", "xml"),
            ("user", "json")
        ]
    );
    assert_eq!(rows[0].cut.as_deref(), Some("36,864 more characters"));
    let tops: Vec<_> = ["big", "day", "doc", "user"]
        .map(|n| at(&mut cx, &format!("variable:{n}")).expect(n).top())
        .into();
    assert!(tops.windows(2).all(|w| w[0] < w[1]), "one row each, in order: {tops:?}");
    assert!(at(&mut cx, "variable-cut:big").is_some());
    assert!(at(&mut cx, "variable-cut:day").is_none());
    // only a json or xml value opens
    assert!(at(&mut cx, "variable-toggle:big").is_none());
    assert!(at(&mut cx, "variable-toggle:day").is_none());
}

/// 3: opens a JSON value indented and an XML value as it is.
#[gpui::test]
fn opens_a_json_value_indented_and_an_xml_value_as_it_is(cx: &mut TestAppContext) {
    let (h, mut cx, view) = setup(cx);
    pause(&h, &mut cx, vars());
    assert!(at(&mut cx, "variable-value:user").is_none());
    toggle(&mut cx, "user");
    assert!(at(&mut cx, "variable-value:user").is_some());
    assert_eq!(
        opened(&view, &mut cx, "user").as_deref(),
        Some("{\n  \"id\": 7,\n  \"tags\": [\n    \"a\"\n  ]\n}")
    );
    toggle(&mut cx, "doc");
    assert_eq!(
        opened(&view, &mut cx, "doc").as_deref(),
        Some("<root><item>x</item></root>")
    );
    toggle(&mut cx, "user");
    assert!(at(&mut cx, "variable-value:user").is_none(), "a second press closes it");
    assert!(at(&mut cx, "variable-value:doc").is_some());
}

/// 4 (SD-22): dims the last variables while the run goes on, with "Running…"; their values still open.
#[gpui::test]
fn dims_the_last_variables_while_the_run_goes_on(cx: &mut TestAppContext) {
    let (h, mut cx, view) = setup(cx);
    pause(&h, &mut cx, vars());
    assert!(!cx.update(|_, cx| view.read(cx).dimmed(cx)));
    assert!(at(&mut cx, "variables-running").is_none());
    toggle(&mut cx, "user");
    send(&h, &mut cx, AppEvent::DebugResumed);
    assert!(cx.update(|_, cx| view.read(cx).dimmed(cx)));
    assert!(at(&mut cx, "variables-running").is_some());
    assert_eq!(rows(&view, &mut cx).len(), 4);
    assert!(at(&mut cx, "variable-value:user").is_some(), "what was open stays");
}

/// The rows are worked out once a pause, never on a redraw; the next pause makes them anew, closed, as Electron
/// Grace's rows were made again.
#[gpui::test]
fn works_the_rows_out_once_a_pause_and_closes_them_on_the_next(cx: &mut TestAppContext) {
    let (h, mut cx, view) = setup(cx);
    pause(&h, &mut cx, vars());
    let first = rows(&view, &mut cx);
    toggle(&mut cx, "user");
    h.update(&mut cx, |s| s.console.push_back("a line".into()));
    assert!(Rc::ptr_eq(&first, &rows(&view, &mut cx)));
    assert!(opened(&view, &mut cx, "user").is_some());
    send(&h, &mut cx, AppEvent::DebugResumed);
    let mut next = vars();
    next[0].value = r#"{"id":8}"#.into();
    pause(&h, &mut cx, next);
    assert!(!Rc::ptr_eq(&first, &rows(&view, &mut cx)));
    assert_eq!(opened(&view, &mut cx, "user"), None);
    toggle(&mut cx, "user");
    assert_eq!(opened(&view, &mut cx, "user").as_deref(), Some("{\n  \"id\": 8\n}"));
}

/// A tab that is closed forgets what was open (Electron Grace unmounted it).
#[gpui::test]
fn closing_the_tab_closes_the_values(cx: &mut TestAppContext) {
    let (h, mut cx, view) = setup(cx);
    pause(&h, &mut cx, vars());
    toggle(&mut cx, "user");
    view.update(&mut cx, |v, _| v.close());
    assert_eq!(opened(&view, &mut cx, "user"), None);
}

/// A value of many lines and of 64 KB is one short line in its row, as tall as any other, and its value cut whole;
/// the toggle stays in sight beside it.
#[gpui::test]
fn a_long_value_with_newlines_is_one_line_that_leaves_the_toggle_in_sight(cx: &mut TestAppContext) {
    let (h, mut cx, view) = setup(cx);
    let long = format!("<a>\n{}\n</a>", "y".repeat(65_000));
    pause(
        &h,
        &mut cx,
        vec![var("short", VarKind::String, "a"), var("wide", VarKind::Xml, &long)],
    );
    let rows = rows(&view, &mut cx);
    assert!(rows[1].line.chars().count() <= grace_core::variables::LINE_LIMIT + 1);
    assert!(!rows[1].line.contains('\n'));
    let (short, wide) = (
        at(&mut cx, "variable:short").unwrap(),
        at(&mut cx, "variable:wide").unwrap(),
    );
    assert_eq!(wide.size.height, short.size.height + px(1.), "the divider apart");
    let button = at(&mut cx, "variable-toggle:wide").expect("its toggle");
    assert!(button.right() <= px(800.), "{button:?}");
    toggle(&mut cx, "wide");
    assert_eq!(opened(&view, &mut cx, "wide"), Some(long));
    let opened_box = at(&mut cx, "variable-value:wide").unwrap();
    assert!(opened_box.size.height <= px(240.), "it scrolls: {opened_box:?}");
    assert!(opened_box.right() <= px(800.), "{opened_box:?}");
}
