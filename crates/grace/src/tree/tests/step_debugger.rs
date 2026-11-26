//! The tree during the step debugger (step debugger R5, R7): the Debug button, Ctrl+Shift+Enter, a debug run's
//! opening of its feature and a pause's way to the paused test. What a screen reader reads is
//! `grace-e2e/tests/debug.rs`'s.
use gpui::CursorStyle;
use grace_core::model::events::{AppEvent, Paused};

use super::*;

fn sent(h: &Harness) -> Vec<String> {
    h.backend
        .calls()
        .into_iter()
        .filter(|c| c.starts_with("run ") || c.starts_with("debug "))
        .collect()
}

/// `Tree.test.tsx` 65 (SD-18, "debugs the focused row with Ctrl+Shift+Enter; Ctrl+Enter alone still runs it"), with
/// the guards of the run's key: nothing during a run, while busy, or on a dimmed row.
#[gpui::test]
fn ctrl_shift_enter_debugs_the_focus_row_and_ctrl_enter_alone_still_runs_it(cx: &mut TestAppContext) {
    let mut s = state("/p", &["a", "b"]);
    if let Some(t) = &mut s.tree {
        t.children[0].children[1].children[0].tags = vec!["smoke".into()];
    }
    let (h, mut cx, tree) = setup(cx, s);
    focus(&tree, &mut cx);
    cx.simulate_keystrokes("down");
    h.update(&mut cx, |s| s.selected = None);
    cx.simulate_keystrokes("ctrl-shift-enter");
    cx.run_until_parked();
    assert_eq!(sent(&h), ["debug f:tests/a.feature"]);
    assert_eq!(selected(&h, &mut cx), None, "a debug run selects nothing itself");
    cx.simulate_keystrokes("ctrl-enter");
    cx.run_until_parked();
    assert_eq!(sent(&h), ["debug f:tests/a.feature", "run f:tests/a.feature"]);
    h.update(&mut cx, |s| s.run.active = true);
    cx.simulate_keystrokes("ctrl-shift-enter");
    h.update(&mut cx, |s| {
        s.run.active = false;
        s.busy = true;
    });
    cx.simulate_keystrokes("ctrl-shift-enter");
    h.update(&mut cx, |s| {
        s.busy = false;
        s.filter.include = vec!["smoke".into()];
    });
    cx.simulate_keystrokes("ctrl-shift-enter");
    cx.run_until_parked();
    assert_eq!(sent(&h).len(), 2, "a run, busy, then a dimmed row: {:?}", sent(&h));
}

/// VL-22 and `Tree.test.tsx` 77's Shift variant: each row says both shortcuts.
#[test]
fn a_row_names_the_run_and_the_debug_shortcut() {
    assert_eq!(SHORTCUTS, "Control+Enter Control+Shift+Enter");
}

/// `Tree.test.tsx` 83 ("a Debug button before Run, disabled during a run"): it debugs its row, sits just before Run,
/// says its key in its tooltip, and does nothing during a run or while the runner is busy.
#[gpui::test]
fn a_debug_button_before_run_debugs_its_row_and_nothing_during_a_run(cx: &mut TestAppContext) {
    let (h, mut cx, _tree) = setup(cx, state("/p", &["a"]));
    let at = |cx: &mut VisualTestContext| cx.debug_bounds("debug:f:tests/a.feature").expect("the Debug button");
    let button = at(&mut cx);
    let row = cx.debug_bounds("tree-row:f:tests/a.feature").expect("the row");
    // the run button takes the row's last 24 px inside its 4 px of padding; Debug ends before it
    let run_left = row.right() - gpui::px(ROW_HEIGHT + 4.);
    assert!(button.right() <= run_left, "{button:?} in {row:?}");
    assert!(button.right() > run_left - gpui::px(8.), "just before Run: {button:?}");
    cx.simulate_mouse_move(button.center(), None, gpui::Modifiers::none());
    cx.executor().advance_clock(crate::tooltip::DELAY);
    cx.run_until_parked();
    assert_eq!(h.tip(&mut cx).as_deref(), Some("Debug a.feature (Ctrl+Shift+Enter)"));
    for during in [true, false] {
        h.update(&mut cx, |s| {
            s.run.active = during;
            s.busy = !during;
        });
        let b = at(&mut cx);
        cx.simulate_click(b.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        assert!(sent(&h).is_empty(), "during a run: {during}");
    }
    h.update(&mut cx, |s| s.busy = false);
    let b = at(&mut cx);
    cx.simulate_click(b.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(sent(&h), ["debug f:tests/a.feature"]);
    // a dimmed row has neither button
    h.update(&mut cx, |s| s.filter.include = vec!["nothing".into()]);
    assert!(cx.debug_bounds("debug:f:tests/a.feature").is_none());
}

/// UP-13: the Debug button shows the hand, and the not-allowed sign during a run, as the run button does.
#[test]
fn a_rows_debug_button_shows_the_hand() {
    let cursor = |mut b: Button| b.style().mouse_cursor;
    let debug = || row_button("debug", "Debug basic.feature".into());
    assert_eq!(cursor(debug()), Some(CursorStyle::PointingHand));
    assert_eq!(cursor(debug().off(true)), Some(CursorStyle::OperationNotAllowed));
}

/// Pauses the debug run in `key`, as the runner's `debugPaused` leaves the state.
fn pause(h: &Harness, cx: &mut VisualTestContext, key: &str) {
    h.update(cx, |s| {
        s.reduce(&AppEvent::DebugPaused {
            paused: Paused {
                key: Some(key.into()),
                file: "tests/x.feature".into(),
                line: 3,
                depth: 0,
                prefix: "*".into(),
                keyword: "def".into(),
                text: "x".into(),
                vars: vec![],
            },
        })
    });
    cx.run_until_parked();
}

/// `Tree.test.tsx` 73 (#224 R3.7, "scrolls to a test paused in a closed feature, once its rows are open"): the pause
/// opens the way, selects the test and brings its row into view, on each pause.
#[gpui::test]
fn a_pause_in_a_closed_feature_opens_it_and_scrolls_to_the_test_on_each_pause(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = many(cx);
    let key = "s:tests/f70.feature#1";
    pause(&h, &mut cx, key);
    assert!(is_open(&tree, "f:tests/f70.feature", &mut cx));
    assert_eq!(selected(&h, &mut cx).as_deref(), Some(key));
    let at = keys(&tree, &mut cx).iter().position(|k| k == key).expect("its row");
    assert!(in_view(&tree, at, &mut cx), "scrolled to the paused test");
    tree.update(&mut cx, |t, cx| {
        let top = gpui::point(gpui::px(0.), gpui::px(0.));
        t.scroll.0.borrow().base_handle.set_offset(top);
        cx.notify();
    });
    cx.run_until_parked();
    assert!(!in_view(&tree, at, &mut cx));
    h.update(&mut cx, |s| s.reduce(&AppEvent::DebugResumed));
    pause(&h, &mut cx, key);
    assert!(in_view(&tree, at, &mut cx), "and again on the next pause");
}

/// TO-16's scenario ("A pause still opens the way to the paused test"): a pause in an outline's second row opens its
/// closed feature, and the row is selected.
#[gpui::test]
fn a_pause_in_an_outlines_row_opens_its_feature_and_selects_it(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, outline_state());
    let row = "r:tests/o.feature#0.2";
    assert!(
        !keys(&tree, &mut cx).iter().any(|k| k == row),
        "the feature starts closed"
    );
    pause(&h, &mut cx, row);
    assert!(is_open(&tree, "f:tests/o.feature", &mut cx));
    assert!(is_open(&tree, "o:tests/o.feature#0", &mut cx));
    assert!(keys(&tree, &mut cx).iter().any(|k| k == row));
    assert_eq!(selected(&h, &mut cx).as_deref(), Some(row));
}

/// `Tree.test.tsx` 41: a debug run of one feature opens it as it starts, as a run does.
#[gpui::test]
fn a_debug_run_of_one_feature_opens_it_as_it_starts(cx: &mut TestAppContext) {
    let (h, mut cx, tree) = setup(cx, state("/p", &["a", "b"]));
    h.update(&mut cx, |s| s.run.debug = true);
    run(&h, &mut cx, "r1", "f:tests/b.feature");
    assert!(is_open(&tree, "f:tests/b.feature", &mut cx));
    assert!(!is_open(&tree, "f:tests/a.feature", &mut cx));
}
