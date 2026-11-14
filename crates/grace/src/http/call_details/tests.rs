//! `CallDetails.test.tsx`'s 16 tests, headless on the fake runner, through `Details`, which shows the call in place of
//! the node while the HTTP tab is up. Classes have no GPUI twin: a part is drawn or not (its debug selector has
//! bounds), and the text each part reads is `grace_core::http_list::call_text`'s, tested there. What a screen reader
//! reads is `grace-e2e/tests/http.rs`'s.
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{AppContext as _, Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext, div, px};
use grace_core::http_list::{CallText, Response};
use grace_core::mask::Masker;
use grace_core::model::events::AppEvent;
use grace_core::model::http::{HttpCall, HttpHeader};
use grace_core::model::node::{Layout, NodeKind, TestNode};
use grace_core::state::{AppState, CallRef, Panel};

use super::*;
use crate::details::Details;
use crate::testing::{self, Harness};

const ONE: &str = "s:a.feature#1";
const TWO: &str = "s:a.feature#2";
const SECRET: &str = "s3cret-token-123";

fn call(over: impl FnOnce(&mut HttpCall)) -> HttpCall {
    let mut c = HttpCall {
        method: "GET".into(),
        url: Some("http://localhost/users".into()),
        request_headers: vec![],
        request_body: None,
        status: Some(200),
        response_headers: vec![],
        response_body: None,
        millis: None,
        error: None,
        via: None,
        start_time: 0.,
        attempt: None,
    };
    over(&mut c);
    c
}

fn header(name: &str, value: &str) -> HttpHeader {
    HttpHeader {
        name: name.into(),
        value: value.into(),
    }
}

fn node(key: &str, kind: NodeKind, name: &str, line: u32, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: name.into(),
        file: "a.feature".into(),
        line: Some(line),
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}

/// The calls of `CallDetails.test.tsx`, in the order its list drew them: 0 and 1 under the first scenario, 2 to 5
/// under the second.
fn calls() -> Vec<(&'static str, Vec<HttpCall>)> {
    vec![
        (
            ONE,
            vec![
                call(|c| {
                    c.method = "POST".into();
                    c.status = Some(201);
                    c.millis = Some(6.);
                    c.via = Some("tests/common/create-user.feature:8".into());
                    c.request_headers = vec![
                        header("Content-Type", "application/json"),
                        header("Accept", "application/json"),
                    ];
                    c.request_body = Some(r#"{"name":"bob","tags":[1,2]}"#.into());
                    c.response_headers = vec![header("X-Id", "7")];
                    c.response_body = Some(r#"{"id":7}"#.into());
                    c.start_time = 1.;
                }),
                call(|c| {
                    c.method = "POST".into();
                    c.url = Some("http://localhost/echo".into());
                    c.request_body = Some("<user><name>bob</name></user>".into());
                    c.start_time = 2.;
                }),
            ],
        ),
        (
            TWO,
            vec![
                // 2: timed out, the request was logged
                call(|c| {
                    c.url = Some("http://localhost/slow".into());
                    c.status = None;
                    c.request_headers = vec![header("Accept", "*/*")];
                    c.error = Some("Read timed out\n  at java.net.Socket.read".into());
                    c.start_time = 3.;
                }),
                // 3: refused, no request logged
                call(|c| {
                    c.url = None;
                    c.status = None;
                    c.error = Some("Connection refused".into());
                    c.start_time = 4.;
                }),
                // 4: nothing logged to show under Request or Response
                call(|c| {
                    c.method = "DELETE".into();
                    c.url = Some("http://localhost/users/7".into());
                    c.status = Some(204);
                    c.start_time = 5.;
                }),
                // 5: no response and no error
                call(|c| {
                    c.url = Some("http://localhost/hang".into());
                    c.status = None;
                    c.start_time = 6.;
                }),
            ],
        ),
    ]
}

fn state() -> AppState {
    let mut s = testing::project("D:/p", Layout::Standalone, 0);
    s.tree = Some(node(
        "d:",
        NodeKind::Folder,
        "root",
        0,
        vec![node(
            "f:a.feature",
            NodeKind::Feature,
            "users",
            1,
            vec![
                node(ONE, NodeKind::Scenario, "create user", 3, vec![]),
                node(TWO, NodeKind::Scenario, "list users", 9, vec![]),
            ],
        )],
    ));
    s.calls = calls().into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    s.panel = Panel::Http;
    s
}

fn setup(cx: &mut TestAppContext) -> (Harness, VisualTestContext, Entity<Details>) {
    setup_with(cx, state())
}

fn setup_with(cx: &mut TestAppContext, state: AppState) -> (Harness, VisualTestContext, Entity<Details>) {
    let slot: Rc<RefCell<Option<Entity<Details>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let (h, vcx) = testing::open_with(cx, 900., state, move |grace, _, cx| {
        let d = cx.new(|cx| Details::new(grace.clone(), cx));
        *s.borrow_mut() = Some(d.clone());
        Box::new(move |_, _, _| div().h(px(900.)).flex().child(d.clone()).into_any_element())
    });
    let details = slot.borrow().clone().unwrap();
    (h, vcx, details)
}

/// Shows call `i` of the list above, as a click on its row does.
fn show(h: &Harness, cx: &mut VisualTestContext, i: usize) {
    let (key, index) = match i {
        0 | 1 => (ONE, i),
        _ => (TWO, i - 2),
    };
    h.update(cx, |s| {
        s.panel = Panel::Http;
        s.select_call(CallRef { key: key.into(), index })
    });
}

fn bounds(cx: &mut VisualTestContext, id: &str) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(id.to_string().into_boxed_str()))
}

fn drawn(cx: &mut VisualTestContext, id: &str) -> bool {
    bounds(cx, id).is_some()
}

fn click(cx: &mut VisualTestContext, id: &str) {
    let b = bounds(cx, id).unwrap_or_else(|| panic!("{id} is drawn"));
    cx.simulate_click(b.center(), Modifiers::default());
    cx.run_until_parked();
}

fn text(d: &Entity<Details>, cx: &mut VisualTestContext) -> Rc<CallText> {
    cx.update(|_, cx| d.read(cx).call_text()).expect("a call shown")
}

fn copied(d: &Entity<Details>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| d.read(cx).copied())
}

/// The parts that read what the call lacks: each is drawn only when the call has it.
const PARTS: [&str; 10] = [
    "call-url",
    "call-time",
    "call-via",
    "call-request-title",
    "call-request-headers",
    "call-request-body",
    "call-response-title",
    "call-response-headers",
    "call-response-body",
    "call-error",
];

fn drawn_parts(cx: &mut VisualTestContext) -> Vec<&'static str> {
    PARTS.into_iter().filter(|p| drawn(cx, p)).collect()
}

// 1 shows the selected call (HI-04)
#[gpui::test]
fn shows_the_selected_call(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    h.update(&mut cx, |s| s.select(ONE));
    assert!(drawn(&mut cx, "details-location"), "the node's details, before a call");
    show(&h, &mut cx, 0);
    let t = text(&d, &mut cx);
    assert_eq!(t.status, "201 · 2xx Success");
    assert_eq!(t.location, "a.feature:3");
    assert_eq!(t.via.as_deref(), Some("via tests/common/create-user.feature:8"));
    assert_eq!(
        t.request.as_ref().and_then(|m| m.headers.as_deref()),
        Some("Content-Type: application/json\nAccept: application/json")
    );
    assert!(matches!(t.response, Some(Response::Received(_))));
    for part in [
        "call-details",
        "call-method",
        "call-status",
        "call-location",
        "copy-curl",
    ] {
        assert!(drawn(&mut cx, part), "{part}");
    }
    assert_eq!(
        drawn_parts(&mut cx),
        [
            "call-url",
            "call-time",
            "call-via",
            "call-request-title",
            "call-request-headers",
            "call-request-body",
            "call-response-title",
            "call-response-headers",
            "call-response-body"
        ]
    );
    // the node's own details give way to the call
    assert!(!drawn(&mut cx, "details-location"));
}

// 2 indents an XML body and leaves out what the call lacks
#[gpui::test]
fn indents_an_xml_body_and_leaves_out_what_the_call_lacks(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    show(&h, &mut cx, 1);
    let body = text(&d, &mut cx).request.clone().unwrap().body;
    assert_eq!(body.as_deref(), Some("<user>\n  <name>bob</name>\n</user>"));
    assert_eq!(text(&d, &mut cx).status, "200 · 2xx Success");
    assert_eq!(
        drawn_parts(&mut cx),
        ["call-url", "call-request-title", "call-request-body"]
    );
}

// 3 shows a call with no response (HI-07)
#[gpui::test]
fn shows_a_call_with_no_response(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    show(&h, &mut cx, 2);
    let t = text(&d, &mut cx);
    assert_eq!(t.status, "No response");
    assert_eq!(t.location, "a.feature:9");
    assert_eq!(
        t.response,
        Some(Response::Failed("Read timed out\n  at java.net.Socket.read".into()))
    );
    // the request was logged
    assert_eq!(
        t.request.as_ref().and_then(|m| m.headers.as_deref()),
        Some("Accept: */*")
    );
    assert_eq!(
        drawn_parts(&mut cx),
        [
            "call-url",
            "call-request-title",
            "call-request-headers",
            "call-response-title",
            "call-error"
        ]
    );
    assert!(drawn(&mut cx, "call-error-box"));
}

// 4 has no heading with nothing under it
#[gpui::test]
fn has_no_heading_with_nothing_under_it(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    // a 204 with no headers and no body: the status says it all
    show(&h, &mut cx, 4);
    assert_eq!(text(&d, &mut cx).status, "204 · 2xx Success");
    assert_eq!(drawn_parts(&mut cx), ["call-url"]);
    // no response and no error either
    show(&h, &mut cx, 5);
    assert_eq!(text(&d, &mut cx).status, "No response");
    assert_eq!(drawn_parts(&mut cx), ["call-url"]);
    // no logged request: the response heading still holds the error
    show(&h, &mut cx, 3);
    assert_eq!(drawn_parts(&mut cx), ["call-response-title", "call-error"]);
}

// 5 copies the call as cURL (HI-08), and says so for two seconds
#[gpui::test]
fn copies_the_call_as_curl(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    show(&h, &mut cx, 0);
    assert!(!copied(&d, &mut cx));
    let width = |cx: &mut VisualTestContext| bounds(cx, "copy-curl").unwrap().size.width;
    let before = width(&mut cx);
    click(&mut cx, "copy-curl");
    assert_ne!(width(&mut cx), before, "the button draws \"Copied\", a different text");
    assert_eq!(
        h.seams.copies(),
        [
            "curl -X POST 'http://localhost/users' -H 'Content-Type: application/json' -H 'Accept: application/json' --data-raw '{\"name\":\"bob\",\"tags\":[1,2]}'"
        ]
    );
    assert!(copied(&d, &mut cx), "Copied, with the check");
    cx.executor().advance_clock(Duration::from_millis(1999));
    cx.run_until_parked();
    assert!(copied(&d, &mut cx));
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert!(!copied(&d, &mut cx), "Copy as cURL again");
    assert_eq!(width(&mut cx), before);
}

// 6, 12: Electron's `navigator.clipboard` could refuse a write, which showed "Copy failed". GPUI's `write_to_clipboard`
// returns nothing, so no test has a failure to show: both are `n/a` (the plan's Redesigns, `seams`).

// 7 styles Copy as cURL as a small button with the copy icon, announced as it changes
#[gpui::test]
fn styles_copy_as_curl_as_a_small_button_with_the_copy_icon_announced_as_it_changes(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx);
    show(&h, &mut cx, 0);
    let small = crate::elements::ButtonSize::Small.look().height;
    assert_eq!(f32::from(bounds(&mut cx, "copy-curl").unwrap().size.height), small);
    assert_eq!(copy_label(false), (Icon::Copy, "Copy as cURL"));
    assert_eq!(copy_label(true), (Icon::Check, "Copied"));
}

// 13 does not carry Copied over to another call
#[gpui::test]
fn does_not_carry_copied_over_to_another_call(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    show(&h, &mut cx, 0);
    click(&mut cx, "copy-curl");
    assert!(copied(&d, &mut cx));
    show(&h, &mut cx, 1);
    assert!(
        text(&d, &mut cx)
            .request
            .clone()
            .unwrap()
            .body
            .unwrap()
            .contains("<user>")
    );
    assert!(!copied(&d, &mut cx));
}

// 14 disables Copy as cURL without a URL
#[gpui::test]
fn disables_copy_as_curl_without_a_url(cx: &mut TestAppContext) {
    let (h, mut cx, d) = setup(cx);
    show(&h, &mut cx, 3);
    assert_eq!(text(&d, &mut cx).status, "No response");
    assert!(!drawn(&mut cx, "call-url"));
    assert!(text(&d, &mut cx).curl.is_none());
    // that the button is off is the e2e test's (UI Automation reads it); a press does nothing here either way
    click(&mut cx, "copy-curl");
    assert!(h.seams.copies().is_empty());
    assert!(!copied(&d, &mut cx));
}

// 15 gives the pane back to the node (HI-10)
#[gpui::test]
fn gives_the_pane_back_to_the_node(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx);
    h.update(&mut cx, |s| s.select(ONE));
    show(&h, &mut cx, 0);
    assert!(drawn(&mut cx, "call-details"));
    // a click on a test in the tree
    h.update(&mut cx, |s| s.select(ONE));
    assert!(!drawn(&mut cx, "call-details"));
    assert!(drawn(&mut cx, "details-location"));
    // the HTTP tab is still up, but nothing is selected in it
    assert_eq!(cx.update(|_, cx| h.grace.read(cx).state.panel), Panel::Http);

    show(&h, &mut cx, 0);
    assert!(drawn(&mut cx, "call-details"));
    h.update(&mut cx, |s| s.show_panel(Panel::Console));
    assert!(!drawn(&mut cx, "call-details"));
    assert!(drawn(&mut cx, "details-location"));
}

// 16 gives the pane back to the node when a new run empties its calls
#[gpui::test]
fn gives_the_pane_back_to_the_node_when_a_new_run_empties_its_calls(cx: &mut TestAppContext) {
    let (h, mut cx, _) = setup(cx);
    h.update(&mut cx, |s| s.select(ONE));
    show(&h, &mut cx, 0);
    assert!(drawn(&mut cx, "call-details"));
    h.update(&mut cx, |s| {
        s.reduce(&AppEvent::Calls {
            key: ONE.into(),
            calls: vec![],
        })
    });
    assert!(!drawn(&mut cx, "call-details"));
    assert!(drawn(&mut cx, "details-location"));
}

// New in the port.
mod added {
    use super::*;

    /// P3 plan, Porting rules 2 (HI-09): an event the runner's `Masker` masked, through the state and the pane, holds
    /// the secret in no part of the pane and not in the cURL text. The runner's own masking is `runner::tests`'.
    #[gpui::test]
    fn shows_and_copies_a_masked_secret_never_the_secret(cx: &mut TestAppContext) {
        let mut event = AppEvent::Calls {
            key: ONE.into(),
            calls: vec![call(|c| {
                c.request_headers = vec![header("X-Token", SECRET)];
                c.request_body = Some(format!(r#"{{"token":"{SECRET}"}}"#));
                c.response_body = Some(format!("echo {SECRET}"));
                c.url = Some(format!("http://localhost/x?t={SECRET}"));
            })],
        };
        Masker::new(&[SECRET]).mask_event(&mut event);
        let mut s = state();
        s.reduce(&event);
        let (h, mut cx, d) = setup_with(cx, s);
        show(&h, &mut cx, 0);
        click(&mut cx, "copy-curl");
        let t = text(&d, &mut cx);
        assert!(format!("{t:?}").contains("X-Token: ****"), "{t:?}");
        assert!(!format!("{t:?}").contains(SECRET), "{t:?}");
        let copies = h.seams.copies();
        assert!(copies[0].contains("-H 'X-Token: ****'"), "{copies:?}");
        assert!(!copies[0].contains(SECRET), "{copies:?}");
    }

    /// Work is done once for a call, not on each draw of the pane (`createMemo` in `CallDetails.tsx`): a run's events
    /// that change nothing about the call do not make its text again.
    #[gpui::test]
    fn works_the_text_out_once_while_the_call_is_the_same(cx: &mut TestAppContext) {
        let (h, mut cx, d) = setup(cx);
        show(&h, &mut cx, 0);
        let first = text(&d, &mut cx);
        h.update(&mut cx, |s| s.run.active = true);
        assert!(Rc::ptr_eq(&first, &text(&d, &mut cx)));
        show(&h, &mut cx, 1);
        assert!(!Rc::ptr_eq(&first, &text(&d, &mut cx)));
    }
}
