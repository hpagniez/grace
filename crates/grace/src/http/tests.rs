//! `HttpCalls.test.tsx`'s tests, headless on the fake runner, and the list's rules of `VirtualList.test.tsx` that the
//! HTTP tab uses. What assistive technology reads (the list box, its options) is `grace-e2e/tests/http.rs`'s.
use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext, div, point, px};
use grace_core::model::events::AppEvent;
use grace_core::model::http::{Attempt, HttpCall, HttpGroup};
use grace_core::model::node::{Layout, NodeKind, TestNode};
use grace_core::panel_tabs::TabId;
use grace_core::rows::scroll_to_show;
use grace_core::state::{AppState, CallRef, Panel};

use super::*;
use crate::bottom_panel::{BottomPanel, tab_name};
use crate::testing::{self, Harness};

const ONE: &str = "s:tests/users.feature#1";
const TWO: &str = "s:tests/users.feature#2";

fn call(status: Option<i32>, start_time: f64) -> HttpCall {
    HttpCall {
        method: "GET".into(),
        url: Some("http://localhost/users".into()),
        request_headers: vec![],
        request_body: None,
        status,
        response_headers: vec![],
        response_body: None,
        millis: None,
        error: None,
        via: None,
        start_time,
        attempt: None,
    }
}

fn node(key: &str, kind: NodeKind, name: &str, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: name.into(),
        file: "tests/users.feature".into(),
        line: Some(1),
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}

fn tree() -> TestNode {
    node(
        "d:",
        NodeKind::Folder,
        "root",
        vec![node(
            "f:tests/users.feature",
            NodeKind::Feature,
            "users",
            vec![
                node(ONE, NodeKind::Scenario, "create user", vec![]),
                node(TWO, NodeKind::Scenario, "list users", vec![]),
            ],
        )],
    )
}

/// Four calls: 201, two 404 attempts, and one with no response.
fn all() -> Vec<(&'static str, Vec<HttpCall>)> {
    vec![
        (
            ONE,
            vec![
                HttpCall {
                    method: "POST".into(),
                    millis: Some(6.),
                    via: Some("tests/common/create-user.feature:8".into()),
                    ..call(Some(201), 1.)
                },
                HttpCall {
                    url: Some("http://localhost/users/9".into()),
                    millis: Some(12.),
                    attempt: Some(Attempt { n: 1, of: 2 }),
                    ..call(Some(404), 2.)
                },
                HttpCall {
                    url: Some("http://localhost/users/9".into()),
                    attempt: Some(Attempt { n: 2, of: 2 }),
                    ..call(Some(404), 3.)
                },
            ],
        ),
        (
            TWO,
            vec![HttpCall {
                url: None,
                error: Some("Connection refused\n  at java.net.Socket.connect".into()),
                ..call(None, 4.)
            }],
        ),
    ]
}

fn state(calls: Vec<(&str, Vec<HttpCall>)>) -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(tree());
    s.calls = calls.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    s.panel = Panel::Http;
    s
}

fn setup(
    cx: &mut TestAppContext,
    calls: Vec<(&str, Vec<HttpCall>)>,
) -> (Harness, VisualTestContext, Entity<HttpCalls>) {
    let slot: Rc<RefCell<Option<Entity<HttpCalls>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let (h, vcx) = testing::open(cx, 1200., state(calls), move |grace, _, cx| {
        s.borrow_mut()
            .get_or_insert_with(|| cx.new(|cx| HttpCalls::new(grace.clone(), cx)))
            .clone()
            .into_any_element()
    });
    let list = slot.borrow().clone().unwrap();
    (h, vcx, list)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn rows(list: &Entity<HttpCalls>, cx: &mut VisualTestContext) -> Rc<Vec<Row>> {
    cx.update(|_, cx| list.read(cx).rows())
}

fn statuses(list: &Entity<HttpCalls>, cx: &mut VisualTestContext) -> Vec<String> {
    rows(list, cx).iter().map(|r| r.text.status.clone()).collect()
}

fn chip_texts(list: &Entity<HttpCalls>, h: &Harness, cx: &mut VisualTestContext) -> Vec<Chip> {
    let groups = cx.update(|_, cx| h.grace.read(cx).state.http_groups.clone());
    let counts = cx.update(|_, cx| list.read(cx).counts());
    chips(&counts, &groups)
}

fn selected(h: &Harness, cx: &mut VisualTestContext) -> Option<CallRef> {
    cx.update(|_, cx| h.grace.read(cx).state.selected_call.clone())
}

fn at(key: &str, index: usize) -> CallRef {
    CallRef { key: key.into(), index }
}

fn bounds(cx: &mut VisualTestContext, selector: String) -> Bounds<Pixels> {
    cx.debug_bounds(selector.clone().leak())
        .unwrap_or_else(|| panic!("{selector} is drawn"))
}

fn click(cx: &mut VisualTestContext, selector: String) {
    let b = bounds(cx, selector);
    cx.simulate_click(b.center(), Modifiers::none());
    cx.run_until_parked();
}

fn chip(cx: &mut VisualTestContext, group: Option<HttpGroup>) {
    click(cx, format!("http-group-{}", chip_id(group)));
}

fn focus(list: &Entity<HttpCalls>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        let f = list.read(cx).focus.clone();
        window.focus(&f, cx);
    });
    cx.run_until_parked();
}

fn active(list: &Entity<HttpCalls>, cx: &mut VisualTestContext) -> Option<usize> {
    cx.update(|_, cx| {
        let l = list.read(cx);
        l.active.as_ref().and_then(|a| l.index_of(a))
    })
}

fn view(list: &Entity<HttpCalls>, cx: &mut VisualTestContext) -> grace_core::rows::View {
    cx.update(|_, cx| list.read(cx).view())
}

fn scroll_to(list: &Entity<HttpCalls>, cx: &mut VisualTestContext, top: f32) {
    list.update(cx, |l, _| {
        l.scroll.0.borrow().base_handle.set_offset(point(px(0.), px(-top)));
    });
    draw(cx);
}

/// Rests the pointer on `at` until a tooltip would show.
fn rest(cx: &mut VisualTestContext, at: gpui::Point<Pixels>) {
    cx.simulate_mouse_move(point(px(5.), px(690.)), None, Modifiers::none());
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.executor().advance_clock(crate::tooltip::DELAY);
    cx.run_until_parked();
}

fn many(n: usize, from: usize) -> Vec<HttpCall> {
    (from..from + n)
        .map(|i| HttpCall {
            url: Some(format!("http://localhost/{i}")),
            ..call(Some(200), i as f64)
        })
        .collect()
}

/// 1. The tab comes after History and counts its calls; a run keeps it open and the calls of the tests outside it.
#[gpui::test]
fn adds_the_http_tab_after_history(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx, all());
    let state = cx.update(|_, cx| h.grace.read(cx).state.clone());
    assert_eq!(grace_core::panel_tabs::TAB_IDS[2], TabId::Http);
    assert_eq!(tab_name(TabId::History, &state), "History (0)");
    assert_eq!(tab_name(TabId::Http, &state), "HTTP (4)");
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::RunStarted {
            selection: TWO.into(),
            run_id: "20261025-150000-000".into(),
            keys: vec![TWO.into()], // clears the call of "list users" only
            total: 1,
            started_at: 0.,
            env: None,
            debug: false,
        })
    });
    let state = cx.update(|_, cx| h.grace.read(cx).state.clone());
    assert_eq!(state.panel, Panel::Http);
    assert_eq!(tab_name(TabId::Http, &state), "HTTP (3)");
    assert_eq!(state.calls[ONE].len(), 3, "the calls of the test outside the run stay");
}

/// 2. Seven chips with their counts, zeros included, All pressed.
#[gpui::test]
fn shows_every_group_chip_with_its_count(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    let c = chip_texts(&list, &h, &mut cx);
    let texts: Vec<_> = c.iter().map(|c| c.text.as_str()).collect();
    assert_eq!(
        texts,
        [
            "All 4",
            "1xx Informational 0",
            "2xx Success 1",
            "3xx Redirect 0",
            "4xx Client error 2",
            "5xx Server error 0",
            "No response 1"
        ]
    );
    let ids: Vec<_> = c.iter().map(|c| chip_id(c.group)).collect();
    assert_eq!(ids, ["all", "1xx", "2xx", "3xx", "4xx", "5xx", "none"]);
    let pressed: Vec<_> = c.iter().map(|c| c.pressed).collect();
    assert_eq!(pressed, [true, false, false, false, false, false, false]);
    for id in ids {
        bounds(&mut cx, format!("http-group-{id}"));
    }
}

/// 3. A row per call: status, method, URL or error, test, attempt, via and time.
#[gpui::test]
fn shows_a_row_per_call(cx: &mut TestAppContext) {
    let (_h, mut cx, list) = setup(cx, all());
    let r = rows(&list, &mut cx);
    assert_eq!(statuses(&list, &mut cx), ["201", "404", "404", "—"]);
    assert_eq!(
        r[0].text,
        RowText {
            status: "201".into(),
            method: "POST".into(),
            place: "http://localhost/users".into(),
            place_whole: "http://localhost/users".into(),
            test: "create user".into(),
            attempt: None,
            via: Some("via tests/common/create-user.feature:8".into()),
            time: "6 ms".into(),
        }
    );
    assert_eq!(r[1].text.attempt.as_deref(), Some("attempt 1 of 2"));
    assert_eq!(r[1].text.via, None);
    assert_eq!(r[2].text.attempt.as_deref(), Some("attempt 2 of 2"));
    assert_eq!(r[2].text.time, "—");
    assert_eq!(r[3].text.place, "Connection refused");
    assert_eq!(
        r[3].text.place_whole,
        "Connection refused\n  at java.net.Socket.connect"
    );
    assert_eq!(r[3].text.test, "list users");
    for i in 1..=4 {
        bounds(&mut cx, format!("http-call:{i}"));
    }
}

/// 4. Each group's badge takes the colour that group has everywhere else.
#[test]
fn maps_each_group_to_the_tint_of_its_colour() {
    let tints: Vec<_> = grace_core::model::http::HTTP_GROUPS
        .iter()
        .map(|g| group_tint(g.group))
        .collect();
    assert_eq!(
        tints,
        [
            Tint::Skipped,
            Tint::Passed,
            Tint::Running,
            Tint::Cancelled,
            Tint::Failed,
            Tint::Failed
        ]
    );
}

/// 5. PU-11: the badge shows the code tinted by its group, with the group in a tooltip right of it, on its row.
#[gpui::test]
fn the_badge_shows_the_code_tinted_by_its_group_with_the_group_in_a_tooltip(cx: &mut TestAppContext) {
    let calls = vec![
        (ONE, vec![call(Some(200), 1.), call(Some(404), 2.), call(Some(503), 3.)]),
        (
            TWO,
            vec![HttpCall {
                url: None,
                ..call(None, 4.)
            }],
        ),
    ];
    let (h, mut cx, list) = setup(cx, calls);
    assert_eq!(statuses(&list, &mut cx), ["200", "404", "503", "—"]);
    let tints: Vec<_> = rows(&list, &mut cx).iter().map(|r| status_tint(r.status)).collect();
    assert_eq!(tints, [Tint::Passed, Tint::Cancelled, Tint::Failed, Tint::Failed]);
    let mut tips = vec![];
    for i in 1..=4 {
        let badge = bounds(&mut cx, format!("http-call-status:{i}"));
        rest(&mut cx, badge.center());
        tips.push(h.tip(&mut cx).map(|t| t.to_string()));
        let tip = h.tip_bounds(&mut cx).unwrap();
        assert!(tip.left() >= badge.right(), "right of the badge: {tip:?}");
        assert!(
            tip.top() < badge.bottom() && tip.bottom() > badge.top(),
            "on its row: {tip:?}"
        );
    }
    let tips: Vec<_> = tips.into_iter().flatten().collect();
    assert_eq!(
        tips,
        ["2xx Success", "4xx Client error", "5xx Server error", "No response"]
    );
    // never white on a fill: the ink is the status colour, the fill its tint
    let t = crate::theme::LIGHT;
    assert_eq!(Tint::Passed.colours(&t), (t.passed_tint, t.passed));
}

/// 7. A code outside 100–599 is neutral, named in its tooltip, and counts in All only.
#[gpui::test]
fn keeps_a_code_outside_100_599_neutral_named_in_the_tooltip(cx: &mut TestAppContext) {
    let calls = vec![
        (ONE, vec![call(Some(999), 1.), call(Some(0), 2.)]),
        (
            TWO,
            vec![HttpCall {
                url: None,
                ..call(None, 3.)
            }],
        ),
    ];
    let (h, mut cx, list) = setup(cx, calls);
    assert_eq!(statuses(&list, &mut cx), ["999", "0", "—"]);
    assert_eq!(status_tint(Some(999)), Tint::Skipped);
    assert_eq!(status_tint(Some(0)), Tint::Skipped);
    let badge = bounds(&mut cx, "http-call-status:1".into());
    rest(&mut cx, badge.center());
    assert_eq!(h.tip(&mut cx).as_deref(), Some("Status 999"));
    assert_eq!(chip_texts(&list, &h, &mut cx)[0].text, "All 3");
}

/// 8. HI-02: the chips show only the calls in the chosen groups; All turns the others off.
#[gpui::test]
fn filters_by_the_chosen_groups(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    chip(&mut cx, Some(HttpGroup::ClientError));
    assert_eq!(statuses(&list, &mut cx), ["404", "404"]);
    let pressed: Vec<_> = chip_texts(&list, &h, &mut cx).iter().map(|c| c.pressed).collect();
    assert_eq!(pressed, [false, false, false, false, true, false, false]);
    chip(&mut cx, Some(HttpGroup::NoResponse));
    assert_eq!(statuses(&list, &mut cx), ["404", "404", "—"]);
    // counts follow the narrowing, not the chips
    assert_eq!(chip_texts(&list, &h, &mut cx)[0].text, "All 4");
    chip(&mut cx, None);
    assert_eq!(rows(&list, &mut cx).len(), 4);
    let pressed: Vec<_> = chip_texts(&list, &h, &mut cx).iter().map(|c| c.pressed).collect();
    assert_eq!(pressed, [true, false, false, false, false, false, false]);
}

/// 9. HI-03: the tree selection narrows the list, the chips' counts and the tab's count, which the chips narrow too.
#[gpui::test]
fn narrows_to_the_tree_selection(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    let tab = |h: &Harness, cx: &mut VisualTestContext| {
        let s = cx.update(|_, cx| h.grace.read(cx).state.clone());
        tab_name(TabId::Http, &s).to_string()
    };
    h.update(&mut cx, |s| s.select(TWO));
    assert_eq!(statuses(&list, &mut cx), ["—"]);
    let texts: Vec<_> = chip_texts(&list, &h, &mut cx).into_iter().map(|c| c.text).collect();
    assert_eq!(
        texts,
        [
            "All 1",
            "1xx Informational 0",
            "2xx Success 0",
            "3xx Redirect 0",
            "4xx Client error 0",
            "5xx Server error 0",
            "No response 1"
        ]
    );
    assert_eq!(tab(&h, &mut cx), "HTTP (1)");
    h.update(&mut cx, |s| s.select(ONE));
    assert_eq!(rows(&list, &mut cx).len(), 3);
    assert_eq!(chip_texts(&list, &h, &mut cx)[0].text, "All 3");
    assert_eq!(tab(&h, &mut cx), "HTTP (3)");
    chip(&mut cx, Some(HttpGroup::Success));
    assert_eq!(rows(&list, &mut cx).len(), 1);
    assert_eq!(tab(&h, &mut cx), "HTTP (1)");
}

/// 10, 26. PU-31's HTTP half: the three empty states, as an empty state with the globe; the chips stay.
#[gpui::test]
fn shows_the_three_empty_states(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, vec![]);
    let empty = |h: &Harness, cx: &mut VisualTestContext| {
        cx.debug_bounds("empty-state")
            .map(|_| cx.update(|_, cx| grace_core::http_list::empty_text(&h.grace.read(cx).state)))
    };
    assert_eq!(
        empty(&h, &mut cx).as_deref(),
        Some("No HTTP calls yet. Calls appear when a feature finishes.")
    );
    assert_eq!(chip_texts(&list, &h, &mut cx)[0].text, "All 0");
    bounds(&mut cx, "http-group-all".into());
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::Calls {
            key: ONE.into(),
            calls: vec![call(Some(200), 1.)],
        })
    });
    assert_eq!(empty(&h, &mut cx), None);
    assert_eq!(rows(&list, &mut cx).len(), 1);
    h.update(&mut cx, |s| s.select(TWO));
    assert_eq!(empty(&h, &mut cx).as_deref(), Some("No HTTP calls in list users"));
    h.update(&mut cx, |s| s.select(ONE));
    chip(&mut cx, Some(HttpGroup::ServerError));
    assert_eq!(empty(&h, &mut cx).as_deref(), Some("No calls in the chosen groups"));
    chip(&mut cx, None);
    assert_eq!(rows(&list, &mut cx).len(), 1);
}

/// 11. More calls arriving keep the focus row and the focus on the list.
#[gpui::test]
fn keeps_a_row_and_its_focus_when_more_calls_arrive(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    click(&mut cx, "http-call:2".into());
    assert_eq!(active(&list, &mut cx), Some(1));
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::Calls {
            key: TWO.into(),
            calls: vec![call(Some(200), 5.), call(Some(200), 6.)],
        })
    });
    assert_eq!(rows(&list, &mut cx).len(), 5);
    assert_eq!(active(&list, &mut cx), Some(1));
    assert!(cx.update(|window, cx| list.read(cx).focus.is_focused(window)));
}

/// 12. HI-11: a test's new calls take the place of its old ones; the other test's rows stay.
#[gpui::test]
fn shows_the_new_calls_in_place_when_a_tests_calls_are_replaced(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::Calls {
            key: ONE.into(),
            calls: vec![
                HttpCall {
                    method: "PUT".into(),
                    url: Some("http://localhost/users/42".into()),
                    millis: Some(31.),
                    ..call(Some(500), 1.)
                },
                HttpCall {
                    url: Some("http://localhost/users/43".into()),
                    millis: Some(4.),
                    ..call(Some(302), 2.)
                },
            ],
        })
    });
    let r = rows(&list, &mut cx);
    assert_eq!(statuses(&list, &mut cx), ["500", "302", "—"]);
    assert_eq!(
        (r[0].text.method.as_str(), r[0].text.place.as_str()),
        ("PUT", "http://localhost/users/42")
    );
    assert_eq!((r[0].text.time.as_str(), r[0].text.via.clone()), ("31 ms", None));
    assert_eq!(
        (r[1].text.place.as_str(), r[1].text.time.as_str()),
        ("http://localhost/users/43", "4 ms")
    );
    assert_eq!(r[1].text.attempt, None);
    assert_eq!(r[2].text.test, "list users");
    let s = cx.update(|_, cx| h.grace.read(cx).state.clone());
    assert_eq!(tab_name(TabId::Http, &s), "HTTP (3)");
    assert_eq!(chip_texts(&list, &h, &mut cx)[0].text, "All 3");
}

/// 13. A click selects a call; one call is selected at a time.
#[gpui::test]
fn selects_a_call_and_marks_it_selected(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx, all());
    assert_eq!(selected(&h, &mut cx), None);
    click(&mut cx, "http-call:2".into());
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 1)));
    click(&mut cx, "http-call:4".into());
    assert_eq!(selected(&h, &mut cx), Some(at(TWO, 0)));
}

/// 15. A row is 28 px.
#[gpui::test]
fn makes_a_row_28_px(cx: &mut TestAppContext) {
    let (_h, mut cx, _list) = setup(cx, all());
    for i in 1..=4 {
        assert_eq!(bounds(&mut cx, format!("http-call:{i}")).size.height, px(ROW_HEIGHT));
    }
    // the selected row's accent bar: 2 px down its left side, over the row, which keeps its height
    assert!(cx.debug_bounds("http-call-bar:2").is_none());
    click(&mut cx, "http-call:2".into());
    let (row, bar) = (
        bounds(&mut cx, "http-call:2".into()),
        bounds(&mut cx, "http-call-bar:2".into()),
    );
    assert_eq!(row.size.height, px(ROW_HEIGHT));
    assert_eq!((bar.left(), bar.size.width), (row.left(), px(2.)));
    assert!(cx.debug_bounds("http-call-bar:1").is_none());
}

/// 19. VL-13: the list is one tab stop on the first call when none is selected; the arrows select the call they
/// move to, which opens it.
#[gpui::test]
fn selects_a_call_with_the_arrow_keys_and_opens_it(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    let state = cx.update(|_, cx| h.grace.read(cx).state.clone());
    assert_eq!(cx.update(|_, cx| list.read(cx).focus_index(&state)), Some(0));
    focus(&list, &mut cx);
    cx.simulate_keystrokes("down");
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 1)));
    assert_eq!(active(&list, &mut cx), Some(1));
    cx.simulate_keystrokes("end");
    assert_eq!(selected(&h, &mut cx), Some(at(TWO, 0)));
    cx.simulate_keystrokes("home");
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 0)));
    // Enter and Space select the focus row
    h.update(&mut cx, |s| s.selected_call = None);
    cx.simulate_keystrokes("enter");
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 0)));
}

/// 20. A run that clears the focused call's test moves the focus row up to a call still there.
#[gpui::test]
fn moves_focus_up_when_the_focused_call_is_cleared_by_a_new_run(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    click(&mut cx, "http-call:3".into());
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::RunStarted {
            selection: ONE.into(),
            run_id: "20261025-150000-000".into(),
            keys: vec![ONE.into()],
            total: 1,
            started_at: 0.,
            env: None,
            debug: false,
        })
    });
    assert_eq!(rows(&list, &mut cx).len(), 1);
    assert_eq!(active(&list, &mut cx), Some(0));
    assert!(cx.update(|window, cx| list.read(cx).focus.is_focused(window)));
}

/// 21, `VirtualList.test.tsx` 23: a chip the person presses brings the selected call into view with the least
/// scroll.
#[gpui::test]
fn scrolls_to_the_selected_call_when_the_chips_change(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, vec![(ONE, many(200, 0))]);
    scroll_to(&list, &mut cx, 149. * ROW_HEIGHT - 100.);
    click(&mut cx, "http-call:150".into());
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 149)));
    scroll_to(&list, &mut cx, 0.);
    // every call is a 2xx: the same calls stay, the selected one out of view
    chip(&mut cx, Some(HttpGroup::Success));
    let v = view(&list, &mut cx);
    assert_eq!(
        v.scroll_top,
        scroll_to_show(149, grace_core::rows::View { scroll_top: 0., ..v })
    );
    assert!(v.scroll_top > 0.);
}

/// 22. Chips that take rows away above the view keep the top row still, then the selected call comes into view.
#[gpui::test]
fn shows_the_selected_call_after_the_chips_take_rows_away_above_the_view(cx: &mut TestAppContext) {
    // every second call is a 404: the 4xx chip keeps the top row in view but moves it up the list
    let calls = (0..100)
        .map(|i| call(Some(if i % 2 == 1 { 404 } else { 200 }), i as f64))
        .collect();
    let (h, mut cx, list) = setup(cx, vec![(ONE, calls)]);
    scroll_to(&list, &mut cx, 99. * ROW_HEIGHT);
    click(&mut cx, "http-call:100".into()); // the 100th call, a 404
    scroll_to(&list, &mut cx, 51. * ROW_HEIGHT); // the 52nd call, a 404, is the top row
    chip(&mut cx, Some(HttpGroup::ClientError));
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 99)));
    // the 52nd call is now the 26th: the list moved to keep it, then the selected call (the 50th) came into view
    let v = view(&list, &mut cx);
    let kept = grace_core::rows::View {
        scroll_top: 25. * ROW_HEIGHT,
        ..v
    };
    assert_eq!(v.scroll_top, scroll_to_show(49, kept));
}

/// 23. VL-19: a call that started earlier than the calls in view arrives above them: they stay still on screen.
#[gpui::test]
fn keeps_the_calls_in_view_still_when_an_earlier_call_arrives(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, vec![(ONE, many(60, 2))]);
    scroll_to(&list, &mut cx, 3. * ROW_HEIGHT); // the call at 5 s is the top row
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::Calls {
            key: TWO.into(),
            calls: vec![call(Some(200), 1.)],
        })
    });
    assert_eq!(view(&list, &mut cx).scroll_top, 4. * ROW_HEIGHT);
    // the call at 5 s is still the top row, one place further down the list
    assert_eq!(rows(&list, &mut cx)[4].text.place, "http://localhost/5");
}

/// 24. A cut URL, test name and via show whole in a tooltip over their own text.
#[gpui::test]
fn shows_a_cut_url_test_name_and_via_whole_in_a_tooltip(cx: &mut TestAppContext) {
    let url = format!("http://localhost/{}", "u".repeat(300));
    let name = "t".repeat(100);
    let via = format!("tests/{}.feature:8", "v".repeat(100));
    let calls = vec![(
        TWO,
        vec![HttpCall {
            url: Some(url.clone()),
            via: Some(via.clone()),
            ..call(Some(200), 1.)
        }],
    )];
    let (h, mut cx, _list) = setup(cx, calls);
    h.update(&mut cx, |s| {
        let t = s.tree.as_mut().unwrap();
        t.children[0].children[1].name = name.clone();
        s.trees += 1;
    });
    let row = bounds(&mut cx, "http-call:1".into());
    for (part, whole) in [("url", url), ("test", name), ("via", format!("via {via}"))] {
        let b = bounds(&mut cx, format!("http-call-{part}:1"));
        rest(&mut cx, b.center());
        assert_eq!(h.tip(&mut cx).as_deref(), Some(whole.as_str()), "{part}");
        // over its own text: below, it would take the click meant for the next row
        let tip = h.tip_bounds(&mut cx).unwrap();
        assert!(
            tip.top() >= row.top() - px(1.) && tip.top() < row.center().y,
            "{part}: {tip:?}"
        );
    }
}

/// 25. Text that fits shows no tooltip.
#[gpui::test]
fn shows_nothing_for_text_that_fits(cx: &mut TestAppContext) {
    let (h, mut cx, _list) = setup(cx, all());
    let b = bounds(&mut cx, "http-call-url:1".into());
    rest(&mut cx, point(b.left() + px(10.), b.center().y));
    assert_eq!(h.tip(&mut cx), None);
}

// New in the port.

/// E-virtual-08: a click on a call focuses the list on it without the ring; the ring comes once a key moves it.
#[gpui::test]
fn a_click_on_a_call_shows_no_focus_ring(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    click(&mut cx, "http-call:3".into());
    assert_eq!(selected(&h, &mut cx), Some(at(ONE, 2)));
    assert!(cx.update(|window, cx| list.read(cx).focus.is_focused(window)));
    assert!(!cx.update(|_, cx| list.read(cx).keyboard), "no ring after a click");
    assert!(!cx.update(|window, _| window.last_input_was_keyboard()));
    assert!(cx.debug_bounds("http-call-ring:3").is_none(), "no ring drawn");
    cx.simulate_keystrokes("down");
    assert_eq!(active(&list, &mut cx), Some(3));
    assert!(
        cx.update(|_, cx| list.read(cx).keyboard),
        "the ring once a key moves it"
    );
    assert!(
        cx.debug_bounds("http-call-ring:4").is_some(),
        "the ring drawn on the row moved to"
    );
}

/// `http.test.ts`'s "keeps the rows of a test whose calls did not change", as GPUI needs it: the rows are built again
/// only when the calls, the tree, the selection or the chips change, not for a console line or a status.
#[gpui::test]
fn rows_are_built_again_only_when_what_they_show_changes(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    let builds = |cx: &mut VisualTestContext| cx.update(|_, cx| list.read(cx).builds);
    let first = builds(&mut cx);
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::Console {
            lines: vec!["x".into()],
        })
    });
    h.update(&mut cx, |s| {
        s.statuses.insert(ONE.into(), grace_core::model::node::Status::Passed);
    });
    draw(&mut cx);
    assert_eq!(builds(&mut cx), first);
    h.update(&mut cx, |s| s.select(ONE));
    assert_eq!(builds(&mut cx), first + 1);
    chip(&mut cx, Some(HttpGroup::ClientError));
    assert_eq!(builds(&mut cx), first + 2);
}

/// #124: a run's calls build no rows while the HTTP tab is closed; the tab's count follows them all the same.
#[gpui::test]
fn a_runs_calls_build_no_rows_while_the_tab_is_closed(cx: &mut TestAppContext) {
    let slot: Rc<RefCell<Option<Entity<BottomPanel>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let mut start = state(vec![]);
    start.panel = Panel::Console;
    let (h, mut cx) = testing::open_with(cx, 1200., start, move |grace, window, cx| {
        let p = cx.new(|cx| BottomPanel::new(grace.clone(), window, cx));
        *s.borrow_mut() = Some(p.clone());
        Box::new(move |_, _, _| div().flex().flex_col().size_full().child(p.clone()).into_any_element())
    });
    let panel = slot.borrow().clone().unwrap();
    let http = cx.update(|_, cx| panel.read(cx).http.clone());
    let builds = |cx: &mut VisualTestContext| cx.update(|_, cx| http.read(cx).builds);
    for (i, (key, calls)) in all().into_iter().enumerate() {
        h.update(&mut cx, |s| s.reduce(&AppEvent::Calls { key: key.into(), calls }));
        draw(&mut cx);
        assert_eq!(builds(&mut cx), 0, "after the calls of test {i}");
    }
    let s = cx.update(|_, cx| h.grace.read(cx).state.clone());
    assert_eq!(tab_name(TabId::Http, &s), "HTTP (4)");
    h.update(&mut cx, |s| s.show_panel(Panel::Http));
    draw(&mut cx);
    assert_eq!(builds(&mut cx), 1);
    assert_eq!(cx.update(|_, cx| http.read(cx).rows().len()), 4);
}

/// 20, the other way: with a call above the cleared one still listed, the focus row goes up to it (R4.5).
#[gpui::test]
fn the_focus_goes_up_to_the_nearest_call_still_listed(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, all());
    click(&mut cx, "http-call:4".into()); // the call of "list users"
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::RunStarted {
            selection: TWO.into(),
            run_id: "20261025-150000-000".into(),
            keys: vec![TWO.into()],
            total: 1,
            started_at: 0.,
            env: None,
            debug: false,
        })
    });
    assert_eq!(rows(&list, &mut cx).len(), 3);
    assert_eq!(active(&list, &mut cx), Some(2));
}

/// The tab reopened, or another project, starts the list afresh, as Electron Grace mounted it again: at the top,
/// its focus row the selected call or the first.
#[gpui::test]
fn reopening_the_tab_starts_the_list_afresh(cx: &mut TestAppContext) {
    let (h, mut cx, list) = setup(cx, vec![(ONE, many(200, 0))]);
    scroll_to(&list, &mut cx, 50. * ROW_HEIGHT);
    click(&mut cx, "http-call:55".into());
    cx.simulate_keystrokes("down");
    assert_eq!(active(&list, &mut cx), Some(55));
    h.update(&mut cx, |s| s.show_panel(Panel::Console));
    h.update(&mut cx, |s| s.show_panel(Panel::Http));
    draw(&mut cx);
    assert_eq!(active(&list, &mut cx), None);
    assert_eq!(view(&list, &mut cx).scroll_top, 0.);
    assert!(!cx.update(|_, cx| list.read(cx).keyboard));
}
