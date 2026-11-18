//! `Trends.test.tsx`'s tests, headless on the fake runner: the strip as the History tab draws it. The words and the
//! height of each bar are `grace_core::trends::tests`'; what assistive technology reads and a real run are
//! `grace-e2e/tests/trends.rs`'s.
use std::rc::Rc;

use gpui::{
    AppContext as _, Bounds, Entity, IntoElement as _, Modifiers, ParentElement as _, Pixels, Styled as _,
    TestAppContext, VisualTestContext, div, point, px,
};
use grace_core::model::history::HistoryEntry;
use grace_core::model::trends::TrendPoint;
use grace_core::state::Panel;

use crate::bottom_panel::BottomPanel;
use crate::history::History;
use crate::testing::{self, Harness};

/// A clock of the test's own (the History tab's), so no label depends on the date.
fn now() -> f64 {
    1_792_000_000_000.
}

fn run(id: &str) -> HistoryEntry {
    HistoryEntry {
        ..testing::run(id, 1, 1)
    }
}

fn trend(id: &str, passed: u32, failed: u32, mean: Option<f64>) -> TrendPoint {
    TrendPoint {
        id: id.into(),
        started_at: now() - 3_600_000.,
        passed,
        failed,
        total: 2,
        mean_millis: mean,
    }
}

const FIRST: &str = "s:a.feature#0";
const SECOND: &str = "s:a.feature#1";

/// The History tab on a project of two scenarios (`s0`, `s1`), with no run yet.
fn setup(cx: &mut TestAppContext) -> (Harness, VisualTestContext, Entity<History>) {
    let slot: Rc<std::cell::RefCell<Option<Entity<History>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let mut state = testing::project("D:/p", grace_core::model::node::Layout::Standalone, 2);
    state.panel = Panel::History;
    let (h, cx) = testing::open_rooted(cx, 1200., state, move |grace, _, cx| {
        let list = s
            .borrow_mut()
            .get_or_insert_with(|| {
                let list = cx.new(|_| History::new(grace.clone()));
                list.update(cx, |l, _| l.clock = Rc::new(now));
                list
            })
            .clone();
        div().size_full().flex().flex_col().child(list).into_any_element()
    });
    let list = slot.borrow().clone().unwrap();
    cx.run_until_parked();
    (h, cx, list)
}

/// What the runner answers for a selection.
fn answer(h: &Harness, key: Option<&str>, points: Vec<TrendPoint>) {
    h.backend.trends.lock().unwrap().insert(key.map(str::to_owned), points);
}

fn ids(runs: &[&str]) -> Vec<HistoryEntry> {
    runs.iter().map(|id| run(id)).collect()
}

fn asked(h: &Harness) -> Vec<String> {
    h.backend.trends_asked()
}

fn bounds(cx: &mut VisualTestContext, selector: String) -> Bounds<Pixels> {
    cx.debug_bounds(selector.clone().leak())
        .unwrap_or_else(|| panic!("{selector} is drawn"))
}

fn drawn(cx: &mut VisualTestContext, selector: &str) -> bool {
    cx.debug_bounds(selector.to_string().leak()).is_some()
}

fn click(cx: &mut VisualTestContext, selector: String) {
    let b = bounds(cx, selector);
    cx.simulate_click(b.center(), Modifiers::none());
    cx.run_until_parked();
}

/// Changes the state and draws once, leaving the runner's answers (which come on the background executor) for
/// the test to let in.
fn change_now(h: &Harness, cx: &mut VisualTestContext, change: impl FnOnce(&mut grace_core::state::AppState)) {
    h.grace.update(cx, |g, cx| {
        change(g.state_mut());
        cx.notify();
    });
    h.view.update(cx, |_, cx| cx.notify());
    cx.update(|window, cx| {
        let arena = window.draw(cx);
        arena.clear(cx);
    });
}

/// What the strip shows: the selection it is for, its caption and its columns' ids, oldest first.
fn shown(list: &Entity<History>, cx: &mut VisualTestContext) -> Option<(Option<String>, String, Vec<String>)> {
    cx.update(|_, cx| {
        let strip = list.read(cx).trends()?;
        let strip = strip.read(cx);
        let s = strip.shown()?;
        Some((
            s.key.clone(),
            s.caption.clone(),
            s.strip.columns.iter().map(|c| c.id.clone()).collect(),
        ))
    })
}

fn columns(list: &Entity<History>, cx: &mut VisualTestContext) -> Vec<String> {
    shown(list, cx).map(|s| s.2).unwrap_or_default()
}

/// 1; TR-02, TR-09: the whole tree with nothing selected, a column per point, oldest left, evenly spaced.
#[gpui::test]
fn asks_for_the_whole_tree_and_draws_a_column_per_point(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(
        &h,
        None,
        vec![trend("a", 1, 1, None), trend("b", 1, 1, None), trend("c", 1, 1, None)],
    );
    h.update(&mut cx, |s| s.history = ids(&["c", "b", "a"]));
    assert_eq!(asked(&h), ["trend null"]);
    let (key, caption, columns) = shown(&list, &mut cx).unwrap();
    assert_eq!((key, caption.as_str()), (None, "All tests"));
    assert_eq!(columns, ["a", "b", "c"]);
    let x = |cx: &mut VisualTestContext, id: &str| bounds(cx, format!("trend-run:{id}")).origin.x;
    let (a, b, c) = (x(&mut cx, "a"), x(&mut cx, "b"), x(&mut cx, "c"));
    assert_eq!(
        (b - a, c - b),
        (px(16.), px(16.)),
        "12 px bars with 4 px of air, evenly spaced"
    );
    assert_eq!(bounds(&mut cx, "trend-run:a".into()).size.width, px(12.));
}

/// 2; TR-03: the strip follows the tree selection.
#[gpui::test]
fn follows_the_tree_selection(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(
        &h,
        Some(FIRST),
        vec![TrendPoint {
            total: 1,
            ..trend("a", 1, 0, Some(1400.))
        }],
    );
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    h.update(&mut cx, |s| s.selected = Some(FIRST.into()));
    assert_eq!(asked(&h).last().map(String::as_str), Some("trend s:a.feature#0"));
    let (key, caption, _) = shown(&list, &mut cx).unwrap();
    assert_eq!((key.as_deref(), caption.as_str()), (Some(FIRST), "s0"));
    let pass = cx.update(|_, cx| {
        list.read(cx)
            .trends()
            .unwrap()
            .read(cx)
            .shown()
            .unwrap()
            .strip
            .pass
            .clone()
    });
    assert_eq!(pass, "Passed");
}

/// 3; TR-18: one question at a time; the selections that come while it is out are asked once, for the one then
/// current, and the answer to an older question is not drawn.
#[gpui::test]
fn asks_one_at_a_time_and_draws_only_the_newest_answer(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(&h, None, vec![trend("all", 1, 1, None)]);
    answer(&h, Some(FIRST), vec![trend("stale", 1, 0, None)]);
    answer(&h, Some("f:a.feature"), vec![trend("skipped", 1, 0, None)]);
    answer(&h, Some(SECOND), vec![trend("fresh", 1, 0, None)]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    // the tree selection moves down fast, three rows before any answer comes
    change_now(&h, &mut cx, |s| s.selected = Some(FIRST.into()));
    change_now(&h, &mut cx, |s| s.selected = Some("f:a.feature".into()));
    change_now(&h, &mut cx, |s| s.selected = Some(SECOND.into()));
    cx.run_until_parked();
    assert_eq!(
        asked(&h),
        ["trend null", "trend s:a.feature#0", "trend s:a.feature#1"],
        "the row in between is never asked: the selection then current is"
    );
    assert_eq!(
        columns(&list, &mut cx),
        ["fresh"],
        "the answer for the first row is dropped"
    );
}

/// 4: the caption is the one of the answer drawn, not of the selection already made.
#[gpui::test]
fn names_the_selection_of_the_answer_it_draws(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(&h, None, vec![trend("all", 1, 1, None)]);
    answer(&h, Some(FIRST), vec![trend("one", 1, 0, None)]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    change_now(&h, &mut cx, |s| s.selected = Some(FIRST.into()));
    let (_, caption, columns) = shown(&list, &mut cx).unwrap();
    assert_eq!(
        (caption.as_str(), columns),
        ("All tests", vec!["all".to_string()]),
        "the suite's columns are still drawn, and so is its caption"
    );
    cx.run_until_parked();
    assert_eq!(shown(&list, &mut cx).unwrap().1, "s0");
}

/// 5: no answer, no columns, rather than another selection's.
#[gpui::test]
fn drops_the_old_columns_when_main_gives_no_answer(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(&h, None, vec![trend("all", 1, 1, None)]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    assert_eq!(columns(&list, &mut cx), ["all"]);
    h.update(&mut cx, |s| s.selected = Some(FIRST.into())); // main knows no stored run for it
    assert!(columns(&list, &mut cx).is_empty());
    assert_eq!(shown(&list, &mut cx).unwrap().1, "s0");
    assert!(drawn(&mut cx, "trends-none"));
}

/// 6; TR-01: with no history the strip leaves and nothing more is asked.
#[gpui::test]
fn leaves_with_the_history_and_asks_nothing_more(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(&h, None, vec![trend("a", 1, 1, None)]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    assert!(drawn(&mut cx, "trends"));
    h.update(&mut cx, |s| s.history = Vec::new());
    assert!(!drawn(&mut cx, "trends"));
    assert!(shown(&list, &mut cx).is_none());
    assert_eq!(asked(&h), ["trend null"]);
}

/// 7; TR-06: the environment changes, the strip asks again.
#[gpui::test]
fn asks_again_when_the_environment_changes(cx: &mut TestAppContext) {
    use grace_core::model::env::{EnvView, PropertyListsView};
    let (h, mut cx, _list) = setup(cx);
    let view = |env: Option<&str>| EnvView {
        env: env.map(str::to_owned),
        suggestions: Vec::new(),
        overlays: Vec::new(),
        properties: PropertyListsView::default(),
        secrets_available: true,
    };
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    h.update(&mut cx, |s| s.env = Some(view(None)));
    h.update(&mut cx, |s| s.env = Some(view(Some("qa"))));
    assert_eq!(
        asked(&h).len(),
        2,
        "no environment, then qa: the first view changes nothing"
    );
}

/// 8; TR-16: a finished run lists a new run, and the strip asks again.
#[gpui::test]
fn asks_again_when_the_history_changes(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(&h, None, vec![trend("a", 1, 1, None)]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    assert_eq!(columns(&list, &mut cx), ["a"]);
    answer(&h, None, vec![trend("a", 1, 1, None), trend("b", 1, 1, None)]);
    h.update(&mut cx, |s| s.history = ids(&["b", "a"]));
    assert_eq!(asked(&h).len(), 2);
    assert_eq!(columns(&list, &mut cx), ["a", "b"]);
}

/// 9; TR-01: no run, no strip, nothing asked.
#[gpui::test]
fn shows_no_strip_and_asks_nothing_with_no_run(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    assert!(!drawn(&mut cx, "trends"));
    assert!(drawn(&mut cx, "empty-state"));
    assert!(shown(&list, &mut cx).is_none());
    assert!(asked(&h).is_empty());
}

/// Idle: a redraw with what it was drawn on asks nothing and works nothing out.
#[gpui::test]
fn a_redraw_asks_nothing(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx);
    answer(&h, None, vec![trend("a", 1, 1, None)]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    for _ in 0..3 {
        h.update(&mut cx, |s| s.notice = None);
    }
    assert_eq!(asked(&h), ["trend null"]);
}

/// 10; TR-17: with the tab closed nothing is asked, and it asks again when it shows, holding nothing meanwhile.
#[gpui::test]
fn asks_nothing_while_history_is_hidden(cx: &mut TestAppContext) {
    let slot: Rc<std::cell::RefCell<Option<Entity<BottomPanel>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let mut state = testing::project("D:/p", grace_core::model::node::Layout::Standalone, 2);
    state.history = ids(&["a"]);
    let dir = std::env::temp_dir().join(format!("grace-trends-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (h, mut cx) = testing::open_in(cx, dir, 1000., state, move |grace, window, cx| {
        let p = cx.new(|cx| BottomPanel::new(grace.clone(), window, cx));
        *s.borrow_mut() = Some(p.clone());
        Box::new(move |_, _, _| div().size_full().flex().flex_col().child(p.clone()).into_any_element())
    });
    cx.run_until_parked();
    h.update(&mut cx, |s| s.selected = Some(FIRST.into()));
    h.update(&mut cx, |s| s.history = ids(&["b", "a"]));
    assert!(asked(&h).is_empty(), "the Console tab shows");
    h.update(&mut cx, |s| s.panel = Panel::History);
    assert_eq!(asked(&h), ["trend s:a.feature#0"]);
    h.update(&mut cx, |s| s.panel = Panel::Console);
    h.update(&mut cx, |s| s.panel = Panel::History);
    assert_eq!(asked(&h).len(), 2, "shown again, it asks again");
    let panel = slot.borrow().clone().unwrap();
    h.update(&mut cx, |s| s.panel = Panel::Console);
    assert!(
        cx.update(|_, cx| panel.read(cx).history.read(cx).trends().is_none()),
        "the strip left with the tab"
    );
}

/// 11; TR-10: passed below, failed above, the time bar scaled to the tallest column.
#[gpui::test]
fn splits_the_pass_bar_and_scales_the_time_bar(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx);
    answer(
        &h,
        None,
        vec![
            trend("a", 3, 1, Some(500.)),
            trend("b", 4, 0, Some(2000.)),
            trend("c", 1, 1, None),
        ],
    );
    h.update(&mut cx, |s| s.history = ids(&["c", "b", "a"]));
    let (failed, passed) = (
        bounds(&mut cx, "trend-failed:a".into()),
        bounds(&mut cx, "trend-passed:a".into()),
    );
    assert!(
        failed.origin.y < passed.origin.y,
        "failed above, passed on the baseline"
    );
    assert_eq!(
        passed.origin.y - failed.origin.y - failed.size.height,
        px(2.),
        "a gap between them"
    );
    assert_eq!((passed.size.height, failed.size.height), (px(16.5), px(5.5)));
    assert!(!drawn(&mut cx, "trend-failed:b"), "a share of nothing draws nothing");
    assert_eq!(bounds(&mut cx, "trend-time:b".into()).size.height, px(24.));
    assert_eq!(bounds(&mut cx, "trend-time:a".into()).size.height, px(6.));
    assert!(!drawn(&mut cx, "trend-time:c"), "no time, no bar; its pass bar stays");
    assert!(drawn(&mut cx, "trend-passed:c"));
    let time = bounds(&mut cx, "trend-time:b".into());
    let pass = bounds(&mut cx, "trend-passed:b".into());
    assert_eq!(
        time.origin.y - (pass.origin.y + pass.size.height),
        px(4.),
        "the rows are 4 px apart"
    );
}

/// 12, 13; TR-12, TR-13: the labels are the newest column's.
#[gpui::test]
fn labels_the_newest_point(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(
        &h,
        None,
        vec![trend("a", 12, 3, Some(1000.)), trend("b", 299, 1, Some(1500.))],
    );
    h.update(&mut cx, |s| s.history = ids(&["b", "a"]));
    let strip = cx.update(|_, cx| list.read(cx).trends().unwrap().read(cx).shown().unwrap().strip.clone());
    assert_eq!(
        (strip.pass.as_str(), strip.time.as_str(), strip.max.as_str()),
        ("99%", "1.50 s", "1.50 s")
    );
    for id in ["trends-pass-label", "trends-time-label", "trends-time-max"] {
        assert!(drawn(&mut cx, id), "{id}");
    }
}

/// 13; TR-14: each column is named by its run.
#[gpui::test]
fn names_each_column_by_its_run(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(
        &h,
        None,
        vec![TrendPoint {
            total: 412,
            passed: 12,
            failed: 3,
            mean_millis: Some(1000.),
            ..trend("a", 0, 0, None)
        }],
    );
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    let tip = cx.update(|_, cx| {
        list.read(cx).trends().unwrap().read(cx).shown().unwrap().strip.columns[0]
            .tip
            .clone()
    });
    assert!(
        tip.ends_with(": 12 passed, 3 failed, 15 of 412 tests, 1.00 s per test"),
        "{tip}"
    );
}

/// 14; TR-08: the height is kept and it says so.
#[gpui::test]
fn says_so_when_no_stored_run_ran_the_selection(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    assert!(drawn(&mut cx, "trends-none"));
    assert!(columns(&list, &mut cx).is_empty());
    assert_eq!(
        bounds(&mut cx, "trends".into()).size.height,
        px(crate::trends::STRIP_HEIGHT)
    );
}

/// The strip is 88 px tall, a 4 px caption gap and two 24 px rows included (R1.4).
#[gpui::test]
fn is_88_px_tall(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx);
    answer(&h, None, vec![trend("a", 1, 1, Some(10.))]);
    h.update(&mut cx, |s| s.history = ids(&["a"]));
    assert_eq!(bounds(&mut cx, "trends".into()).size.height, px(88.));
    assert_eq!(bounds(&mut cx, "trend-run:a".into()).size.height, px(52.));
}

/// The caption shows the tree's own label of the selection: a masked name stays masked.
#[gpui::test]
fn the_caption_is_the_tree_s_own_label(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    h.update(&mut cx, |s| {
        s.history = ids(&["a"]);
        if let Some(tree) = s.tree.as_mut() {
            tree.children[0].children[0].name = "login with token ****".into();
        }
        s.selected = Some(FIRST.into());
    });
    assert_eq!(shown(&list, &mut cx).unwrap().1, "login with token ****");
}

/// 15; TR-15: a click on a column finds its run's row, and Enter does the same, the focus staying on the column.
#[gpui::test]
fn a_click_shows_the_runs_row(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    let names: Vec<String> = (0..20).map(|i| format!("run-{i:02}")).collect();
    let newest_first: Vec<&str> = names.iter().rev().map(String::as_str).collect();
    answer(&h, None, names.iter().map(|id| trend(id, 1, 1, Some(1.))).collect());
    cx.simulate_resize(gpui::size(px(1200.), px(300.)));
    h.update(&mut cx, |s| s.history = ids(&newest_first));
    let scroll = cx.update(|_, cx| list.read(cx).scroll().clone());
    let view = bounds(&mut cx, "history-scroll".into());
    let inside = |cx: &mut VisualTestContext, row: usize| {
        let (r, v) = (
            bounds(cx, format!("history-row:{row}")),
            bounds(cx, "history-scroll".into()),
        );
        r.top() >= v.top() && r.bottom() <= v.bottom()
    };
    assert!(!inside(&mut cx, 20), "the oldest run is out of view");
    click(&mut cx, "trend-run:run-00".into());
    assert!(
        inside(&mut cx, 20),
        "the oldest run's row {:?} in {view:?}",
        scroll.offset()
    );
    // back to the top: Enter on the column that kept the focus does the same
    scroll.set_offset(point(px(0.), px(0.)));
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    assert!(!inside(&mut cx, 20));
    cx.simulate_keystrokes("enter");
    assert!(inside(&mut cx, 20), "Enter on the focused column");
}

/// R1.4: the titles of the columns stay at the top once the strip has scrolled away, where `sticky` kept them.
#[gpui::test]
fn the_titles_stay_at_the_top_once_the_strip_has_scrolled_away(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx);
    answer(&h, None, vec![trend("a", 1, 1, None)]);
    cx.simulate_resize(gpui::size(px(1200.), px(300.)));
    let many: Vec<String> = (0..20).map(|i| format!("run-{i:02}")).collect();
    h.update(&mut cx, |s| {
        s.history = ids(&many.iter().map(String::as_str).collect::<Vec<_>>())
    });
    assert!(!drawn(&mut cx, "history-header-stuck"));
    let scroll = cx.update(|_, cx| list.read(cx).scroll().clone());
    scroll.set_offset(point(px(0.), px(-200.)));
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let view = bounds(&mut cx, "history-scroll".into());
    let stuck = bounds(&mut cx, "history-header-stuck".into());
    assert_eq!(stuck.origin.y, view.origin.y, "at the top of the list");
}
