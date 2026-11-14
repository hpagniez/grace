// `src/renderer/http.test.ts`, test for test.
use std::collections::HashMap;

use super::*;
use crate::model::node::NodeKind;

fn node(key: &str, kind: NodeKind, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: key.into(),
        file: String::new(),
        line: None,
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
        vec![
            node(
                "f:a.feature",
                NodeKind::Feature,
                vec![
                    node("s:a.feature#1", NodeKind::Scenario, vec![]),
                    node("s:a.feature#2", NodeKind::Scenario, vec![]),
                ],
            ),
            node(
                "f:b.feature",
                NodeKind::Feature,
                vec![node("s:b.feature#1", NodeKind::Scenario, vec![])],
            ),
        ],
    )
}

fn call(status: Option<i32>, start_time: f64) -> HttpCall {
    HttpCall {
        method: "GET".into(),
        url: Some("http://localhost/x".into()),
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

fn calls() -> HashMap<String, Vec<HttpCall>> {
    [
        ("s:a.feature#1", vec![call(Some(200), 100.), call(Some(404), 300.)]),
        ("s:a.feature#2", vec![call(Some(500), 200.)]),
        // 999 is in no group
        ("s:b.feature#1", vec![call(None, 50.), call(Some(999), 400.)]),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect()
}

fn state(selected: Option<&str>) -> AppState {
    AppState {
        tree: Some(tree()),
        selected: selected.map(str::to_owned),
        calls: calls(),
        ..AppState::default()
    }
}

fn refs(selected: Option<&str>) -> Vec<String> {
    narrowed_calls(&state(selected))
        .iter()
        .map(|l| format!("{}/{}", l.at.key, l.at.index))
        .collect()
}

fn at(key: &str, index: usize) -> CallRef {
    CallRef { key: key.into(), index }
}

mod narrowed_calls {
    use super::*;

    #[test]
    fn lists_every_call_in_start_time_order() {
        let all = [
            "s:b.feature#1/0",
            "s:a.feature#1/0",
            "s:a.feature#2/0",
            "s:a.feature#1/1",
            "s:b.feature#1/1",
        ];
        assert_eq!(refs(None), all);
        assert_eq!(refs(Some("d:")), all);
        let no_tree = AppState {
            calls: calls(),
            ..AppState::default()
        };
        assert!(narrowed_calls(&no_tree).is_empty());
        // the same start time keeps the log order
        let tied = AppState {
            tree: Some(tree()),
            calls: [(
                "s:a.feature#1".to_owned(),
                vec![call(Some(200), 7.), call(Some(201), 7.), call(Some(202), 7.)],
            )]
            .into(),
            ..AppState::default()
        };
        let statuses: Vec<_> = narrowed_calls(&tied).iter().map(|l| l.call.status).collect();
        assert_eq!(statuses, [Some(200), Some(201), Some(202)]);
    }

    #[test]
    fn narrows_to_the_selected_node() {
        assert_eq!(refs(Some("s:a.feature#1")), ["s:a.feature#1/0", "s:a.feature#1/1"]);
        assert_eq!(refs(Some("s:a.feature#2")), ["s:a.feature#2/0"]);
        assert_eq!(
            refs(Some("f:a.feature")),
            ["s:a.feature#1/0", "s:a.feature#2/0", "s:a.feature#1/1"]
        );
        let s = state(Some("f:b.feature"));
        let first = &narrowed_calls(&s)[0];
        assert_eq!(first.test.key, "s:b.feature#1");
        assert_eq!(first.call.status, None);
    }

    // "keeps the rows of a test whose calls did not change" is object identity (a `WeakMap` per calls array), so
    // unchanged DOM rows were not drawn again. `uniform_list` draws the rows in view afresh each frame; the HTTP list
    // builds its rows again only when the calls, the tree, the selection or the chips change:
    // `grace::http::tests::rows_are_built_again_only_when_what_they_show_changes`.

    #[test]
    fn lists_kept_calls_under_the_nodes_of_a_refreshed_tree() {
        let mut refreshed = tree();
        refreshed.children[1].children[0].name = "renamed".into();
        let s = AppState {
            tree: Some(refreshed),
            selected: Some("s:b.feature#1".into()),
            calls: calls(),
            ..AppState::default()
        };
        assert_eq!(narrowed_calls(&s)[0].test.name, "renamed");
    }
}

#[test]
fn group_counts_counts_every_group_zeros_included() {
    let s = state(None);
    let counts = group_counts(&narrowed_calls(&s));
    assert_eq!(counts.get(None), 5);
    let by: Vec<_> = HTTP_GROUPS.iter().map(|g| counts.get(Some(g.group))).collect();
    // 1xx, 2xx, 3xx, 4xx, 5xx, none
    assert_eq!(by, [0, 1, 0, 1, 1, 1]);
}

#[test]
fn shown_calls_with_no_group_on_shows_all_with_4xx_and_none_on_shows_only_those() {
    let s = state(None);
    let list = narrowed_calls(&s);
    let all: Vec<_> = list.iter().map(|l| l.at.clone()).collect();
    let shown: Vec<_> = shown_calls(list.clone(), &[]).iter().map(|l| l.at.clone()).collect();
    assert_eq!(shown, all);
    let statuses: Vec<_> = shown_calls(list, &[HttpGroup::ClientError, HttpGroup::NoResponse])
        .iter()
        .map(|l| l.call.status)
        .collect();
    assert_eq!(statuses, [None, Some(404)]);
}

#[test]
fn toggled_groups_turns_a_group_on_and_off_all_turns_every_group_off() {
    use HttpGroup::{ClientError, NoResponse};
    assert_eq!(toggled_groups(&[], Some(ClientError)), [ClientError]);
    assert_eq!(
        toggled_groups(&[ClientError], Some(NoResponse)),
        [ClientError, NoResponse]
    );
    assert_eq!(
        toggled_groups(&[ClientError, NoResponse], Some(ClientError)),
        [NoResponse]
    );
    assert_eq!(toggled_groups(&[ClientError, NoResponse], None), []);
}

#[test]
fn total_calls_adds_up_the_calls_of_every_test() {
    assert_eq!(total_calls(&calls()), 5);
    assert_eq!(total_calls(&HashMap::new()), 0);
}

#[test]
fn find_call_resolves_the_selected_call_and_is_none_once_it_is_gone() {
    let find = |selected_call: Option<CallRef>, calls: HashMap<String, Vec<HttpCall>>| {
        let s = AppState {
            tree: Some(tree()),
            calls,
            selected_call,
            ..AppState::default()
        };
        find_call(&s).map(|l| (l.at, l.call.status, l.test.key.clone()))
    };
    assert_eq!(
        find(Some(at("s:a.feature#2", 0)), calls()),
        Some((at("s:a.feature#2", 0), Some(500), "s:a.feature#2".into()))
    );
    assert_eq!(find(None, calls()), None);
    assert_eq!(find(Some(at("s:a.feature#2", 1)), calls()), None); // no such call
    assert_eq!(find(Some(at("s:gone", 0)), calls()), None); // no such test
    assert_eq!(find(Some(at("s:a.feature#2", 0)), HashMap::new()), None); // calls cleared
    let no_tree = AppState {
        calls: calls(),
        selected_call: Some(at("s:a.feature#2", 0)),
        ..AppState::default()
    };
    assert!(find_call(&no_tree).is_none());
}

// New in the port.
mod added {
    use super::*;

    /// #124: the tab's count reads the calls without listing them, and agrees with the list in every case.
    #[test]
    fn shown_count_is_the_length_of_the_shown_list() {
        for selected in [
            None,
            Some("d:"),
            Some("f:a.feature"),
            Some("s:b.feature#1"),
            Some("s:gone"),
        ] {
            for groups in [
                vec![],
                vec![HttpGroup::ClientError, HttpGroup::NoResponse],
                vec![HttpGroup::Redirect],
            ] {
                let mut s = state(selected);
                s.http_groups = groups.clone();
                // a call under a key the tree does not hold is in no list, so in no count
                s.calls
                    .insert("s:stray".into(), vec![call(Some(200), 1.), call(None, 2.)]);
                let listed = shown_calls(narrowed_calls(&s), &groups).len();
                assert_eq!(shown_count(&s), listed, "{selected:?} {groups:?}");
            }
        }
        assert_eq!(shown_count(&AppState::default()), 0);
    }

    /// A call's accessible name: what its row reads, status first (VL-25, `callName` of `HttpCalls.tsx`).
    #[test]
    fn a_call_is_named_by_status_method_url_and_test() {
        let test = node("s:t#1", NodeKind::Scenario, vec![]);
        let test = TestNode {
            name: "list users".into(),
            ..test
        };
        let name = |c: HttpCall| call_name(&c, &test);
        assert_eq!(name(call(Some(404), 0.)), "404, GET, http://localhost/x, list users");
        assert_eq!(name(call(None, 0.)), "no status, GET, http://localhost/x, list users");
        let refused = HttpCall {
            url: None,
            error: Some("Refused\n  at java.net".into()),
            ..call(None, 0.)
        };
        assert_eq!(name(refused), "no status, GET, Refused, list users");
    }

    /// PU-11: the badge's tooltip names the group; a code outside 100–599 has none, so it names itself.
    #[test]
    fn a_status_is_named_by_its_group() {
        assert_eq!(group_label(Some(200)), "2xx Success");
        assert_eq!(group_label(Some(404)), "4xx Client error");
        assert_eq!(group_label(Some(503)), "5xx Server error");
        assert_eq!(group_label(None), "No response");
        assert_eq!(group_label(Some(999)), "Status 999");
        assert_eq!(group_label(Some(0)), "Status 0");
    }

    /// The three empty states of the HTTP tab (PU-31's HTTP half, `HttpCalls.tsx`'s `empty`).
    #[test]
    fn the_empty_text_says_why_the_list_is_empty() {
        let none = AppState {
            tree: Some(tree()),
            ..AppState::default()
        };
        assert_eq!(
            empty_text(&none),
            "No HTTP calls yet. Calls appear when a feature finishes."
        );
        let mut s = state(Some("s:a.feature#2"));
        s.calls.remove("s:a.feature#2");
        assert_eq!(empty_text(&s), "No HTTP calls in s:a.feature#2");
        let mut s = state(Some("s:a.feature#1"));
        s.http_groups = vec![HttpGroup::ServerError];
        assert_eq!(empty_text(&s), "No calls in the chosen groups");
    }
}

// `CallDetails.tsx`'s rules, which the view only draws (HI-04, HI-07, HI-08).
mod call_text {
    use super::*;
    use crate::model::http::HttpHeader;

    fn header(name: &str, value: &str) -> HttpHeader {
        HttpHeader {
            name: name.into(),
            value: value.into(),
        }
    }

    fn test(line: Option<u32>) -> TestNode {
        TestNode {
            name: "create user".into(),
            file: "a.feature".into(),
            line,
            ..node("s:a.feature#1", NodeKind::Scenario, vec![])
        }
    }

    fn text(c: HttpCall) -> CallText {
        call_text(&c, &test(Some(3)))
    }

    #[test]
    fn names_the_status_with_its_group_and_a_code_outside_100_599_alone() {
        assert_eq!(text(call(Some(201), 0.)).status, "201 · 2xx Success");
        assert_eq!(text(call(Some(404), 0.)).status, "404 · 4xx Client error");
        assert_eq!(text(call(Some(503), 0.)).status, "503 · 5xx Server error");
        assert_eq!(text(call(Some(999), 0.)).status, "999");
        assert_eq!(text(call(None, 0.)).status, "No response");
        // the code is the part drawn in the mono
        assert_eq!(text(call(Some(201), 0.)).code_len, 3);
        assert_eq!(text(call(None, 0.)).code_len, 0);
    }

    #[test]
    fn places_the_call_under_its_test_and_its_line() {
        let c = HttpCall {
            via: Some("tests/common/create-user.feature:8".into()),
            ..call(Some(200), 0.)
        };
        let t = call_text(&c, &test(Some(3)));
        assert_eq!(t.test, "create user");
        assert_eq!(t.location, "a.feature:3");
        assert_eq!(t.via.as_deref(), Some("via tests/common/create-user.feature:8"));
        assert_eq!(call_text(&c, &test(None)).location, "a.feature");
        assert_eq!(text(call(Some(200), 0.)).via, None);
    }

    #[test]
    fn shows_headers_and_an_indented_body_under_request_and_response() {
        let t = text(HttpCall {
            request_headers: vec![header("Content-Type", "application/json"), header("Accept", "*/*")],
            request_body: Some(r#"{"name":"bob","tags":[1,2]}"#.into()),
            response_headers: vec![header("X-Id", "7")],
            response_body: Some(r#"{"id":7}"#.into()),
            ..call(Some(201), 0.)
        });
        let request = t.request.unwrap();
        assert_eq!(
            request.headers.as_deref(),
            Some("Content-Type: application/json\nAccept: */*")
        );
        assert_eq!(
            request.body.as_deref(),
            Some("{\n  \"name\": \"bob\",\n  \"tags\": [\n    1,\n    2\n  ]\n}")
        );
        let Some(Response::Received(response)) = t.response else {
            panic!("a response")
        };
        assert_eq!(response.headers.as_deref(), Some("X-Id: 7"));
        assert_eq!(response.body.as_deref(), Some("{\n  \"id\": 7\n}"));
    }

    #[test]
    fn indents_an_xml_body_and_leaves_out_what_the_call_lacks() {
        let t = text(HttpCall {
            request_body: Some("<user><name>bob</name></user>".into()),
            ..call(Some(200), 0.)
        });
        let request = t.request.unwrap();
        assert_eq!(request.body.as_deref(), Some("<user>\n  <name>bob</name>\n</user>"));
        assert_eq!(request.headers, None);
        assert!(t.response.is_none());
    }

    /// HI-07: no response; the request that was logged still shows, and the error takes the response's place.
    #[test]
    fn a_call_with_no_response_shows_its_request_and_its_error() {
        let t = text(HttpCall {
            request_headers: vec![header("Accept", "*/*")],
            error: Some("Read timed out\n  at java.net.Socket.read".into()),
            ..call(None, 0.)
        });
        assert_eq!(t.request.unwrap().headers.as_deref(), Some("Accept: */*"));
        assert_eq!(
            t.response,
            Some(Response::Failed("Read timed out\n  at java.net.Socket.read".into()))
        );
    }

    #[test]
    fn has_no_section_with_nothing_under_it() {
        // a 204 with no headers and no body: the status says it all
        let t = text(call(Some(204), 0.));
        assert!(t.request.is_none() && t.response.is_none());
        // no response and no error either
        let t = text(call(None, 0.));
        assert!(t.request.is_none() && t.response.is_none());
        // no logged request: the response section still holds the error
        let t = text(HttpCall {
            url: None,
            request_headers: vec![header("Accept", "*/*")],
            error: Some("Connection refused".into()),
            ..call(None, 0.)
        });
        assert!(t.request.is_none());
        assert_eq!(t.response, Some(Response::Failed("Connection refused".into())));
    }

    /// HI-08: the same cURL text as `curlCommand`, none without a URL.
    #[test]
    fn copies_the_call_as_curl_none_without_a_url() {
        let t = text(HttpCall {
            method: "POST".into(),
            request_headers: vec![header("Content-Type", "application/json"), header("Host", "localhost")],
            request_body: Some(r#"{"name":"bob"}"#.into()),
            ..call(Some(201), 0.)
        });
        assert_eq!(
            t.curl.as_deref(),
            Some(
                "curl -X POST 'http://localhost/x' -H 'Content-Type: application/json' --data-raw '{\"name\":\"bob\"}'"
            )
        );
        let refused = HttpCall {
            url: None,
            error: Some("Connection refused".into()),
            ..call(None, 0.)
        };
        assert_eq!(text(refused).curl, None);
    }
}
