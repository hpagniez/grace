//! `src/main/karate/http.ts`: the HTTP calls of a test, read from the text Karate puts in a step's log. The text is
//! Karate's, not a contract: the patterns are the TypeScript's, and the tests run on recorded logs
//! (docs/specs/2026-10-25-http-inspector.md, "What Karate writes"). After any Karate version change, `npm run
//! fixtures` re-records them.
use super::events::DataError;
use super::js;
use crate::model::http::{Attempt, HttpCall, HttpHeader};
use regex::Regex;
use serde_json::Value;
use std::cmp::Ordering;
use std::sync::LazyLock;

/// What one step log says about a call; the step supplies the rest (`error`, `via`, `start_time`).
// Rust-native: its own struct for the TypeScript's `Omit<HttpCall, "error" | "via" | "startTime">`, and `Result`s in
// place of thrown `TypeError`s (`DataError`). Parity: the same calls from the same logs (`parse_step_log`'s 13 tests)
// and the same unreadable data (`data_that_is_not_shaped_as_karate_writes_it_is_an_error`).
#[derive(Clone, Debug, PartialEq)]
pub struct LoggedCall {
    pub method: String,
    pub url: Option<String>,
    pub request_headers: Vec<HttpHeader>,
    pub request_body: Option<String>,
    pub status: Option<i32>,
    pub response_headers: Vec<HttpHeader>,
    pub response_body: Option<String>,
    pub millis: Option<f64>,
    pub attempt: Option<Attempt>,
}

fn regex(pattern: String) -> Regex {
    Regex::new(&pattern).expect("a fixed pattern")
}

// JavaScript's `\d` is [0-9], `\S` is not a JavaScript space and `.` is not a line terminator; Rust's differ.
static REQUEST_LINE: LazyLock<Regex> =
    LazyLock::new(|| regex(format!(r"^([0-9]+) > ([^{}]*) ?({}*)$", js::WS, js::DOT)));
static STARTS_REQUEST: LazyLock<Regex> = LazyLock::new(|| regex(r"^[0-9]+ > ".into()));
static RESPONSE_TIME: LazyLock<Regex> = LazyLock::new(|| regex(r"^response time in milliseconds: ([0-9]+)$".into()));
static RESPONSE_LINE: LazyLock<Regex> = LazyLock::new(|| regex(r"^([0-9]+) < ([0-9]+)".into()));
// The two lines Karate adds after a `method` step: `METHOD URL`, then `STATUS (T ms)`.
static SUMMARY_REQUEST: LazyLock<Regex> = LazyLock::new(|| regex(format!(r"^[^{ws}]+ [^{ws}]+$", ws = js::WS)));
static SUMMARY_STATUS: LazyLock<Regex> = LazyLock::new(|| regex(r"^[0-9]+ \([0-9]+ ms\)$".into()));

/// A `<prefix>Name: value` line as a header; `None` when the line is not one.
fn header(line: &str, prefix: &str) -> Option<HttpHeader> {
    let rest = line.strip_prefix(prefix)?;
    let colon = rest.find(':').filter(|colon| *colon >= 1)?;
    let value = &rest[colon + 1..];
    Some(HttpHeader {
        name: rest[..colon].to_owned(),
        value: value.strip_prefix(' ').unwrap_or(value).to_owned(),
    })
}

fn body_of(lines: &[&str]) -> Option<String> {
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// The calls one step log holds (spec R1). A response attaches to the request with the same number `N`, wherever it
/// sits in the log. Karate's format is not a contract: an error means a line that starts a call could not be read.
pub fn parse_step_log(log: &str) -> Result<Vec<LoggedCall>, DataError> {
    let mut lines = js::lines(log);
    if lines.last() == Some(&"") {
        lines.pop(); // the newline that ends the last line
    }
    let mut end = lines.len();
    if end >= 2 && SUMMARY_STATUS.is_match(lines[end - 1]) && SUMMARY_REQUEST.is_match(lines[end - 2]) {
        end -= 2;
    }
    // A marker line inside a body is body text: `request:` starts a call only before a `N > ` line,
    // and `response time in milliseconds` starts a response only before a `N < ` line.
    let starts_call = |i: usize| lines[i] == "request:" && i + 1 < end && STARTS_REQUEST.is_match(lines[i + 1]);
    let starts_response =
        |i: usize| i + 1 < end && RESPONSE_TIME.is_match(lines[i]) && RESPONSE_LINE.is_match(lines[i + 1]);

    let mut requests: Vec<(String, LoggedCall)> = Vec::new();
    let mut i = 0;
    while i < end {
        if starts_call(i) {
            let Some(caps) = REQUEST_LINE.captures(lines[i + 1]) else {
                return Err(DataError::new("Karate step log: a request line could not be read"));
            };
            let n = caps[1].to_owned();
            let mut call = LoggedCall {
                method: caps[2].to_owned(),
                url: Some(caps[3].to_owned()),
                request_headers: Vec::new(),
                request_body: None,
                status: None,
                response_headers: Vec::new(),
                response_body: None,
                millis: None,
                attempt: None,
            };
            i += 2;
            let prefix = format!("{n} > ");
            while i < end {
                let Some(h) = header(lines[i], &prefix) else {
                    break;
                };
                call.request_headers.push(h);
                i += 1;
            }
            let mut body = Vec::new();
            while i < end && !starts_call(i) && !starts_response(i) {
                body.push(lines[i]);
                i += 1;
            }
            if body.last() == Some(&"") {
                body.pop();
            }
            call.request_body = body_of(&body);
            requests.push((n, call));
        } else if starts_response(i) {
            let millis = RESPONSE_TIME
                .captures(lines[i])
                .map_or(f64::NAN, |c| c[1].parse::<f64>().unwrap_or(f64::NAN));
            let Some(caps) = RESPONSE_LINE.captures(lines[i + 1]) else {
                i += 1;
                continue;
            };
            let n = caps[1].to_owned();
            let status = caps[2].parse::<f64>().map_or(i32::MAX, |s| s as i32);
            i += 2;
            let prefix = format!("{n} < ");
            let mut headers = Vec::new();
            while i < end {
                let Some(h) = header(lines[i], &prefix) else {
                    break;
                };
                headers.push(h);
                i += 1;
            }
            let mut body = Vec::new();
            while i < end && !starts_call(i) && !starts_response(i) {
                body.push(lines[i]);
                i += 1;
            }
            while body.last() == Some(&"") {
                body.pop();
            }
            // A response with no request of its number has no call to join: Karate never writes one.
            if let Some((_, call)) = requests.iter_mut().find(|(rn, call)| *rn == n && call.status.is_none()) {
                call.status = Some(status);
                call.response_headers = headers;
                call.response_body = body_of(&body);
                call.millis = Some(millis);
            }
        } else {
            i += 1;
        }
    }
    // An unanswered request with another request after it is a followed redirect, not a call.
    let count = requests.len();
    let mut calls: Vec<LoggedCall> = requests
        .into_iter()
        .enumerate()
        .filter(|(k, (_, call))| call.status.is_some() || *k == count - 1)
        .map(|(_, (_, call))| call)
        .collect();
    // Retries resend one request: several calls to different requests are separate calls, not attempts.
    if calls.len() > 1
        && calls
            .iter()
            .all(|c| c.method == calls[0].method && c.url == calls[0].url)
    {
        let of = calls.len() as u32;
        for (k, call) in calls.iter_mut().enumerate() {
            call.attempt = Some(Attempt { n: k as u32 + 1, of });
        }
    }
    Ok(calls)
}

/// Appends the calls of these steps, then of the features they called, at any depth. `feature` is `None` at depth 0.
fn collect(step_results: &[Value], feature: Option<&str>, out: &mut Vec<HttpCall>) -> Result<(), DataError> {
    for r in step_results {
        let via = match feature {
            None => None,
            Some(feature) => Some(format!(
                "{feature}:{}",
                js::display(js::prop(js::prop(r, "step")?, "line")?)
            )),
        };
        let result = js::prop(r, "result")?;
        let start_time = js::prop(result, "startTime")?.as_f64().unwrap_or(0.0);
        let error = js::text(js::prop(result, "errorMessage")?).map(str::to_owned);
        let log = match js::prop(r, "stepLog")? {
            Value::Null => "",
            Value::String(log) => log,
            _ => return Err(DataError::new("Karate event data: `stepLog` is not text")),
        };
        let logged = parse_step_log(log)?;
        let none_logged = logged.is_empty();
        for c in logged {
            out.push(HttpCall {
                error: if c.status.is_none() { error.clone() } else { None },
                method: c.method,
                url: c.url,
                request_headers: c.request_headers,
                request_body: c.request_body,
                status: c.status,
                response_headers: c.response_headers,
                response_body: c.response_body,
                millis: c.millis,
                via: via.clone(),
                start_time,
                attempt: c.attempt,
            });
        }
        // Karate logs nothing when the connection fails, so the failed `method` step is the only trace.
        if none_logged
            && js::prop(result, "status")? == "failed"
            && js::prop(js::prop(r, "step")?, "keyword")? == "method"
        {
            let Some(text) = js::text(js::prop(js::prop(r, "step")?, "text")?) else {
                return Err(DataError::new("Karate event data: a step's `text` is not text"));
            };
            out.push(HttpCall {
                method: js::trim(text).to_uppercase(),
                url: None,
                request_headers: Vec::new(),
                request_body: None,
                status: None,
                response_headers: Vec::new(),
                response_body: None,
                millis: None,
                error: error.clone(),
                via: via.clone(),
                start_time,
                attempt: None,
            });
        }
        for called in js::items(js::prop(r, "callResults")?, "callResults")? {
            let relative_path = js::display(js::prop(called, "relativePath")?);
            for sc in js::items(js::prop(called, "scenarioResults")?, "scenarioResults")? {
                collect(
                    js::items(js::prop(sc, "stepResults")?, "stepResults")?,
                    Some(&relative_path),
                    out,
                )?;
            }
        }
    }
    Ok(())
}

/// The calls of one scenario of a feature, by Karate refId.
#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioCalls {
    pub ref_id: String,
    pub calls: Vec<HttpCall>,
}

/// HTTP calls of every scenario in a depth-0 `FEATURE_EXIT`'s data, by Karate refId, in start-time order. Calls of
/// called features count for the test whose step made them. An error means a part of the data was not shaped as Karate
/// writes it: the caller keeps the run going without this feature's calls.
// A `refId` that is not a string reads as "": no test has one, so its calls match nothing, as `undefined` did.
pub fn extract_calls(feature_exit_data: &Value) -> Result<Vec<ScenarioCalls>, DataError> {
    let mut scenarios = Vec::new();
    for sc in js::items(js::prop(feature_exit_data, "scenarioResults")?, "scenarioResults")? {
        let mut calls = Vec::new();
        collect(
            js::items(js::prop(sc, "stepResults")?, "stepResults")?,
            None,
            &mut calls,
        )?;
        // stable: equal start times keep the log order
        calls.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(Ordering::Equal));
        scenarios.push(ScenarioCalls {
            ref_id: js::text(js::prop(sc, "refId")?).unwrap_or_default().to_owned(),
            calls,
        });
    }
    Ok(scenarios)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::karate::events::{EventKind, read_all_events};
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;

    // The spec's example of a successful call (docs/specs/2026-10-25-http-inspector.md, "What Karate writes").
    fn call() -> String {
        [
            "request:",
            "1 > POST http://localhost:60219/users",
            "1 > Content-Type: application/json; charset=UTF-8",
            "1 > Host: localhost:60219",
            "1 > Content-Length: 16",
            "1 > Connection: keep-alive",
            "1 > User-Agent: Apache-HttpClient/5.6.5 (Java/21.0.12.1)",
            "{",
            "  \"name\": \"alice\"",
            "}",
            "",
            "response time in milliseconds: 6",
            "1 < 201 POST http://localhost:60219/users",
            "1 < content-length: 23",
            "1 < Content-Type: application/json; charset=UTF-8",
            "{",
            "  \"name\": \"alice\",",
            "  \"id\": 1",
            "}",
            "",
            "POST http://localhost:60219/users",
            "201 (6 ms)",
            "",
        ]
        .join("\n")
    }

    const RESPONSE_BODY: &str = "{\n  \"name\": \"alice\",\n  \"id\": 1\n}";

    fn h(name: &str, value: &str) -> HttpHeader {
        HttpHeader {
            name: name.into(),
            value: value.into(),
        }
    }

    mod parse_step_log {
        use super::*;

        fn parse(log: &str) -> Vec<LoggedCall> {
            parse_step_log(log).unwrap()
        }

        #[test]
        fn parses_one_call() {
            assert_eq!(
                parse(&call()),
                [LoggedCall {
                    method: "POST".into(),
                    url: Some("http://localhost:60219/users".into()),
                    request_headers: vec![
                        h("Content-Type", "application/json; charset=UTF-8"),
                        h("Host", "localhost:60219"),
                        h("Content-Length", "16"),
                        h("Connection", "keep-alive"),
                        h("User-Agent", "Apache-HttpClient/5.6.5 (Java/21.0.12.1)"),
                    ],
                    request_body: Some("{\n  \"name\": \"alice\"\n}".into()),
                    status: Some(201),
                    response_headers: vec![
                        h("content-length", "23"),
                        h("Content-Type", "application/json; charset=UTF-8"),
                    ],
                    response_body: Some(RESPONSE_BODY.into()),
                    millis: Some(6.0),
                    attempt: None,
                }]
            );
        }

        #[test]
        fn ignores_text_before_request_and_the_summary_lines() {
            let one = parse(&call());
            assert_eq!(parse(&format!("karate-config.js running, env = null\n{}", call())), one);
        }

        #[test]
        fn keeps_blank_lines_inside_a_body() {
            let log = call().replacen("\"id\": 1\n}", "\"id\": 1\n}\n\nb", 1).replacen(
                "{\n  \"name\": \"alice\",",
                "a\n\n{\n  \"name\": \"alice\",",
                1,
            );
            let calls = parse(&log);
            assert_eq!(
                calls[0].response_body.as_deref(),
                Some("a\n\n{\n  \"name\": \"alice\",\n  \"id\": 1\n}\n\nb")
            );
        }

        #[test]
        fn a_request_line_inside_a_body_does_not_start_a_call() {
            let log = call().replacen("}\n\nPOST", "}\nrequest:\nnot a header\n\nPOST", 1);
            let calls = parse(&log);
            assert_eq!(calls.len(), 1);
            assert_eq!(
                calls[0].response_body,
                Some(format!("{RESPONSE_BODY}\nrequest:\nnot a header"))
            );
        }

        #[test]
        fn reads_crlf_lines() {
            assert_eq!(parse(&call().replace('\n', "\r\n")), parse(&call()));
        }

        #[test]
        fn numbers_retry_attempts() {
            let pair = |n: u32| {
                format!(
                    "request:\n{n} > GET http://h/x\n{n} > Host: h\n\nresponse time in milliseconds: {n}\n{n} < 404 GET http://h/x\n{n} < Content-Length: 0\n\n"
                )
            };
            let calls = parse(&(pair(1) + &pair(2) + &pair(3)));
            assert_eq!(
                calls.iter().map(|c| c.attempt).collect::<Vec<_>>(),
                [
                    Some(Attempt { n: 1, of: 3 }),
                    Some(Attempt { n: 2, of: 3 }),
                    Some(Attempt { n: 3, of: 3 })
                ]
            );
            assert_eq!(
                calls.iter().map(|c| c.status).collect::<Vec<_>>(),
                [Some(404), Some(404), Some(404)]
            );
            assert_eq!(calls[0].request_body, None);
            assert_eq!(calls[0].response_body, None);
        }

        #[test]
        fn does_not_call_two_different_requests_in_one_log_attempts() {
            let pair = |n: u32, method: &str, url: &str| {
                format!(
                    "request:\n{n} > {method} {url}\n{n} > Host: h\n\nresponse time in milliseconds: {n}\n{n} < 200 {method} {url}\n{n} < Content-Length: 0\n\n"
                )
            };
            let calls = parse(&(pair(1, "GET", "http://h/hello") + &pair(2, "GET", "http://h/boom")));
            assert_eq!(
                calls.iter().map(|c| c.url.as_deref()).collect::<Vec<_>>(),
                [Some("http://h/hello"), Some("http://h/boom")]
            );
            assert_eq!(calls.iter().map(|c| c.attempt).collect::<Vec<_>>(), [None, None]);
            // the same URL with another method is not a retry either
            let other = parse(&(pair(1, "GET", "http://h/x") + &pair(2, "POST", "http://h/x")));
            assert_eq!(other.iter().map(|c| c.attempt).collect::<Vec<_>>(), [None, None]);
        }

        #[test]
        fn pairs_a_response_with_the_request_of_the_same_number() {
            let log = "request:\n1 > GET http://h/a\n1 > Host: h\n\n\
                request:\n2 > GET http://h/b\n2 > Host: h\n\n\
                response time in milliseconds: 5\n2 < 200 GET http://h/b\n2 < Content-Length: 3\ntwo\n\n\
                response time in milliseconds: 7\n1 < 404 GET http://h/a\n1 < Content-Length: 3\none\n";
            let calls = parse(log);
            assert_eq!(
                calls
                    .iter()
                    .map(|c| (c.url.as_deref(), c.status, c.millis, c.response_body.as_deref()))
                    .collect::<Vec<_>>(),
                [
                    (Some("http://h/a"), Some(404), Some(7.0), Some("one")),
                    (Some("http://h/b"), Some(200), Some(5.0), Some("two"))
                ]
            );
            // two URLs: not attempts
            assert_eq!(calls.iter().map(|c| c.attempt).collect::<Vec<_>>(), [None, None]);
        }

        #[test]
        fn drops_a_followed_redirects_unanswered_request() {
            let log = "request:\n1 > GET http://h/moved\n1 > Host: h\n\n\
                request:\n2 > GET http://h/moved\n2 > Host: h\n\nresponse time in milliseconds: 1\n2 < 200 GET http://h/moved\n2 < Content-Length: 2\nok\n\nGET http://h/moved\n200 (1 ms)\n";
            let calls = parse(log);
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].status, Some(200));
            assert_eq!(calls[0].millis, Some(1.0));
            assert_eq!(calls[0].response_body.as_deref(), Some("ok"));
            assert_eq!(calls[0].attempt, None);
        }

        #[test]
        fn keeps_a_last_unanswered_request_as_no_response() {
            let calls = parse("request:\n1 > GET http://h/slow\n1 > Host: h\n\n");
            assert_eq!(
                calls,
                [LoggedCall {
                    method: "GET".into(),
                    url: Some("http://h/slow".into()),
                    request_headers: vec![h("Host", "h")],
                    request_body: None,
                    status: None,
                    response_headers: vec![],
                    response_body: None,
                    millis: None,
                    attempt: None,
                }]
            );
        }

        #[test]
        fn keeps_a_response_time_line_with_no_response_line_after_it_as_body_text() {
            let log = "request:\n1 > POST http://h/x\n1 > Host: h\nresponse time in milliseconds: 3\nrest\n\n\
                response time in milliseconds: 4\n1 < 200 POST http://h/x\n1 < Content-Length: 0\n";
            let calls = parse(log);
            assert_eq!(calls.len(), 1);
            assert_eq!(
                calls[0].request_body.as_deref(),
                Some("response time in milliseconds: 3\nrest")
            );
            assert_eq!(calls[0].status, Some(200));
            assert_eq!(calls[0].millis, Some(4.0));
        }

        #[test]
        fn drops_a_response_whose_number_matches_no_request() {
            let log = "request:\n1 > GET http://h/a\n1 > Host: h\n\n\
                response time in milliseconds: 2\n7 < 500 GET http://h/z\n7 < Content-Length: 0\n\n\
                response time in milliseconds: 3\n1 < 200 GET http://h/a\n1 < Content-Length: 0\n";
            let calls = parse(log);
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].url.as_deref(), Some("http://h/a"));
            assert_eq!(calls[0].status, Some(200));
            assert_eq!(calls[0].millis, Some(3.0));
            assert_eq!(calls[0].attempt, None);
        }

        #[test]
        fn finds_nothing_in_a_log_without_request() {
            assert_eq!(parse("env is null devFlag is false"), []);
        }
    }

    fn exit_data(file: &str, feature: &str) -> Value {
        read_all_events(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../test/fixtures/events")
                .join(file),
        )
        .unwrap()
        .into_iter()
        .find(|e| e.kind == EventKind::FeatureExit && e.at_depth_zero() && e.data["relativePath"] == feature)
        .unwrap()
        .data
    }

    /// Calls by scenario name, through the refId as the app's tree keys them.
    fn calls_by_name(file: &str, feature: &str) -> HashMap<String, Vec<HttpCall>> {
        let data = exit_data(file, feature);
        let calls = extract_calls(&data).unwrap();
        data["scenarioResults"]
            .as_array()
            .unwrap()
            .iter()
            .map(|sc| {
                let found = calls.iter().find(|c| sc["refId"] == c.ref_id.as_str());
                (sc["name"].as_str().unwrap().to_owned(), found.unwrap().calls.clone())
            })
            .collect()
    }

    mod extract_calls {
        use super::*;

        #[test]
        fn reads_every_case_of_the_http_sample() {
            let by = calls_by_name("run-http.jsonl", "tests/http.feature");

            // `ok`'s first step also carries start-mock.feature (config callSingle), which logs nothing
            assert_eq!(by["ok"].len(), 1);
            let ok = &by["ok"][0];
            assert_eq!(ok.method, "GET");
            assert_eq!(ok.status, Some(200));
            assert_eq!(ok.via, None);
            assert_eq!(ok.error, None);

            assert_eq!(
                by["retry"].iter().map(|c| (c.status, c.attempt)).collect::<Vec<_>>(),
                [
                    (Some(404), Some(Attempt { n: 1, of: 3 })),
                    (Some(404), Some(Attempt { n: 2, of: 3 })),
                    (Some(404), Some(Attempt { n: 3, of: 3 })),
                ]
            );

            assert_eq!(
                by["server error"].iter().map(|c| c.status).collect::<Vec<_>>(),
                [Some(500)]
            );

            assert_eq!(by["refused"].len(), 1);
            assert_eq!(by["refused"][0].method, "GET");
            assert_eq!(by["refused"][0].url, None);
            assert_eq!(by["refused"][0].status, None);
            assert!(
                by["refused"][0]
                    .error
                    .as_deref()
                    .unwrap()
                    .contains("Connection refused")
            );

            assert_eq!(by["timeout"].len(), 1);
            assert!(by["timeout"][0].url.as_deref().unwrap().ends_with("/slow"));
            assert_eq!(by["timeout"][0].status, None);
            assert!(by["timeout"][0].error.as_deref().unwrap().contains("Read timed out"));

            assert_eq!(
                by["redirect followed"].iter().map(|c| c.status).collect::<Vec<_>>(),
                [Some(200)]
            );

            assert_eq!(by["redirect kept"].len(), 1);
            assert_eq!(by["redirect kept"][0].status, Some(302));
            assert!(
                by["redirect kept"][0]
                    .response_headers
                    .contains(&h("Location", "/hello"))
            );

            assert_eq!(by["via helper"].len(), 1);
            let via = &by["via helper"][0];
            assert_eq!(via.method, "POST");
            assert_eq!(via.status, Some(201));
            assert_eq!(via.via.as_deref(), Some("tests/common/create-user.feature:8"));

            assert_eq!(
                by["xml"][0].request_body.as_deref(),
                Some("<user><name>bob</name></user>")
            );
            assert_eq!(by["xml"][0].status, Some(200));

            assert_eq!(by["secret header"].len(), 1);
            assert_eq!(by["secret header"][0].status, Some(200));
            assert!(by["secret header"][0].request_headers.contains(&h("X-Token", "none")));

            assert_eq!(by.values().map(Vec::len).sum::<usize>(), 12);
        }

        #[test]
        fn puts_a_callonce_call_under_the_first_scenario_only() {
            let by = calls_by_name("run-http.jsonl", "tests/shared.feature");
            assert_eq!(by["first"].len(), 1);
            assert_eq!(
                by["first"][0].via.as_deref(),
                Some("tests/common/create-user.feature:8")
            );
            assert_eq!(by["second"], []);
        }

        #[test]
        fn puts_a_callsingle_call_under_the_first_scenario() {
            let by = calls_by_name("run-http-config.jsonl", "tests/shared.feature");
            assert!(by["first"].iter().any(|c| c.via.as_deref() == Some("auth.feature:8")));
            assert_eq!(by["second"], []);
        }

        #[test]
        fn puts_a_failed_callsingles_call_under_the_first_scenarios_config_step() {
            let by = calls_by_name("run-http-config-fail.jsonl", "tests/shared.feature");
            assert_eq!(by["first"].len(), 1);
            assert_eq!(by["first"][0].via.as_deref(), Some("auth.feature:8"));
            assert_eq!(by["first"][0].status, Some(201));
            assert_eq!(by["second"], []);
        }

        #[test]
        fn walks_call_results_two_deep() {
            let step = |line: u32, extra: Value| {
                let mut s = json!({
                    "step": { "keyword": "method", "text": "get", "line": line },
                    "result": { "status": "passed", "startTime": 1 },
                });
                s.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
                s
            };
            let log = "request:\n1 > GET http://h/x\n1 > Host: h\n\nresponse time in milliseconds: 2\n1 < 200 GET http://h/x\n";
            let feature = |relative_path: &str, step_results: Vec<Value>| json!({ "relativePath": relative_path, "scenarioResults": [{ "stepResults": step_results }] });
            let calls = extract_calls(&json!({
                "scenarioResults": [{
                    "refId": "[1:1]",
                    "stepResults": [step(3, json!({
                        "callResults": [feature("a.feature", vec![step(5, json!({
                            "callResults": [feature("b.feature", vec![step(9, json!({ "stepLog": log }))])]
                        }))])]
                    }))]
                }]
            }))
            .unwrap();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].ref_id, "[1:1]");
            assert_eq!(calls[0].calls.len(), 1);
            assert_eq!(calls[0].calls[0].status, Some(200));
            assert_eq!(calls[0].calls[0].via.as_deref(), Some("b.feature:9"));
        }

        #[test]
        fn orders_a_tests_calls_by_start_time() {
            let log = |n: u32| {
                format!(
                    "request:\n1 > GET http://h/{n}\n1 > Host: h\n\nresponse time in milliseconds: 1\n1 < 200 GET http://h/{n}\n"
                )
            };
            let step = |line: u32, start_time: u32, log: String| {
                json!({
                    "step": { "keyword": "method", "text": "get", "line": line },
                    "result": { "status": "passed", "startTime": start_time },
                    "stepLog": log,
                })
            };
            let calls = extract_calls(&json!({
                "scenarioResults": [{
                    "refId": "[1:1]",
                    "stepResults": [
                        step(1, 300, log(1)),
                        step(2, 100, log(2)),
                        step(3, 200, log(3)),
                        // equal start times keep the log order
                        step(4, 200, log(4)),
                    ]
                }]
            }))
            .unwrap();
            assert_eq!(
                calls[0]
                    .calls
                    .iter()
                    .map(|c| c.url.as_deref().unwrap())
                    .collect::<Vec<_>>(),
                ["http://h/2", "http://h/3", "http://h/4", "http://h/1"]
            );
        }

        // New in the port: what was a thrown `TypeError` (and `run_state` catches) is an error.
        #[test]
        fn data_that_is_not_shaped_as_karate_writes_it_is_an_error() {
            let broken = |call_results: Value| {
                json!({ "scenarioResults": [{ "refId": "[1:1]", "stepResults": [{
                    "step": { "keyword": "method", "text": "get", "line": 1 },
                    "result": { "status": "passed", "startTime": 1 },
                    "callResults": call_results,
                }]}]})
            };
            assert!(extract_calls(&broken(json!(42))).is_err());
            assert!(extract_calls(&broken(json!({}))).is_err());
            assert_eq!(extract_calls(&broken(json!([]))).unwrap()[0].calls, []);
            assert!(extract_calls(&json!({"scenarioResults": [{"stepResults": [{}]}]})).is_err());
        }
    }
}
