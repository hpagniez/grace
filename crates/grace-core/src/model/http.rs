//! `src/shared/http.ts`: the HTTP call model and its pure helpers (docs/specs/2026-10-25-http-inspector.md).
use super::js;
use regex::Regex;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

/// The `n`th of `of` attempts of a retried call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    pub n: u32,
    pub of: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpCall {
    pub method: String,
    /// None when the step log held no request.
    pub url: Option<String>,
    pub request_headers: Vec<HttpHeader>,
    pub request_body: Option<String>,
    /// None for a call with no response.
    pub status: Option<i32>,
    pub response_headers: Vec<HttpHeader>,
    pub response_body: Option<String>,
    #[serde(with = "crate::model::js_num_opt")]
    pub millis: Option<f64>,
    pub error: Option<String>,
    /// Where a called feature made the call, like `tests/common/create-user.feature:8`.
    pub via: Option<String>,
    #[serde(with = "crate::model::js_num")]
    pub start_time: f64,
    pub attempt: Option<Attempt>,
}

/// The status classes the HTTP tab filters by.
///
/// Differs from the TypeScript's string union `"1xx" | … | "none"`: an enum, with the TypeScript's text from `as_str`,
/// which is also what it is written as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpGroup {
    #[serde(rename = "1xx")]
    Informational,
    #[serde(rename = "2xx")]
    Success,
    #[serde(rename = "3xx")]
    Redirect,
    #[serde(rename = "4xx")]
    ClientError,
    #[serde(rename = "5xx")]
    ServerError,
    #[serde(rename = "none")]
    NoResponse,
}

impl HttpGroup {
    /// `"1xx"` … `"5xx"`, `"none"`.
    pub fn as_str(self) -> &'static str {
        match self {
            HttpGroup::Informational => "1xx",
            HttpGroup::Success => "2xx",
            HttpGroup::Redirect => "3xx",
            HttpGroup::ClientError => "4xx",
            HttpGroup::ServerError => "5xx",
            HttpGroup::NoResponse => "none",
        }
    }
}

/// A group and the label that heads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpGroupLabel {
    pub group: HttpGroup,
    pub label: &'static str,
}

pub const HTTP_GROUPS: [HttpGroupLabel; 6] = [
    HttpGroupLabel {
        group: HttpGroup::Informational,
        label: "1xx Informational",
    },
    HttpGroupLabel {
        group: HttpGroup::Success,
        label: "2xx Success",
    },
    HttpGroupLabel {
        group: HttpGroup::Redirect,
        label: "3xx Redirect",
    },
    HttpGroupLabel {
        group: HttpGroup::ClientError,
        label: "4xx Client error",
    },
    HttpGroupLabel {
        group: HttpGroup::ServerError,
        label: "5xx Server error",
    },
    HttpGroupLabel {
        group: HttpGroup::NoResponse,
        label: "No response",
    },
];

/// The group of a status; `None` for a code outside 100–599, which counts in All only.
pub fn http_group(status: Option<i32>) -> Option<HttpGroup> {
    match status {
        None => Some(HttpGroup::NoResponse),
        Some(100..=199) => Some(HttpGroup::Informational),
        Some(200..=299) => Some(HttpGroup::Success),
        Some(300..=399) => Some(HttpGroup::Redirect),
        Some(400..=499) => Some(HttpGroup::ClientError),
        Some(500..=599) => Some(HttpGroup::ServerError),
        Some(_) => None,
    }
}

/// The headers curl sets itself, which `curl_command` leaves out.
const CURL_SETS: [&str; 3] = ["host", "content-length", "connection"];

/// Single-quoted for bash: a `'` inside is closed, escaped and opened again.
fn sq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// A bash-quoted `curl` command for the call; `None` when no request was logged.
pub fn curl_command(c: &HttpCall) -> Option<String> {
    let url = c.url.as_deref()?;
    // `/^[A-Za-z]+$/`
    let method = if !c.method.is_empty() && c.method.bytes().all(|b| b.is_ascii_alphabetic()) {
        c.method.clone()
    } else {
        sq(&c.method)
    };
    // `-X HEAD` makes curl wait for a body that never comes: `--head` sends the same request and returns.
    // (`toUpperCase() === "HEAD"` in the TypeScript; no character outside ASCII upper-cases into H, E, A or D, so the
    // ASCII comparison is the same.)
    let head = c.method.eq_ignore_ascii_case("HEAD");
    let mut parts = vec![if head {
        format!("curl --head {}", sq(url))
    } else {
        format!("curl -X {method} {}", sq(url))
    }];
    for h in &c.request_headers {
        // (`toLowerCase()` in the TypeScript; likewise no character outside ASCII lower-cases into these names)
        if !CURL_SETS.iter().any(|set| h.name.eq_ignore_ascii_case(set)) {
            parts.push(format!("-H {}", sq(&format!("{}: {}", h.name, h.value))));
        }
    }
    if let Some(body) = c.request_body.as_deref().filter(|b| !b.is_empty() && !head) {
        parts.push(format!("--data-raw {}", sq(body)));
    }
    Some(parts.join(" "))
}

/// JavaScript's `\s` as the inside of a character class (`js::is_space`), for the patterns below.
const SPACE: &str = "\t\n\u{b}\u{c}\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

/// Comments first: they may hold a `>`.
static XML_TOKENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<!--[\s\S]*?-->|<[^>]*>").unwrap());
static XML_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!("^<(/?)([^{SPACE}/>]+)[^>]*?(/?)>$")).unwrap());
static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]*"|'[^']*'"#).unwrap());

/// The pieces of the text around and between its tags and comments, trimmed, none empty: JavaScript's
/// `src.split(XML_TOKENS)` (whose pattern captures what it splits on) then `trim()` and `filter`.
fn xml_tokens(src: &str) -> Vec<&str> {
    fn push<'a>(tokens: &mut Vec<&'a str>, piece: &'a str) {
        let piece = js::trim(piece);
        if !piece.is_empty() {
            tokens.push(piece);
        }
    }
    let mut tokens = Vec::new();
    let mut last = 0;
    for m in XML_TOKENS.find_iter(src) {
        push(&mut tokens, &src[last..m.start()]);
        push(&mut tokens, m.as_str());
        last = m.end();
    }
    push(&mut tokens, &src[last..]);
    tokens
}

/// The body indented with 2 spaces, or `None` when it is not well-formed enough to trust.
fn indent_xml(src: &str) -> Option<String> {
    if !src.starts_with('<') {
        return None;
    }
    let tokens = xml_tokens(src);
    let mut out: Vec<String> = Vec::new();
    let mut open: Vec<&str> = Vec::new();
    let line = |out: &mut Vec<String>, depth: usize, s: &str| out.push("  ".repeat(depth) + s);
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i];
        if !t.starts_with('<') {
            if open.is_empty() {
                return None; // text after the root
            }
            line(&mut out, open.len(), t);
        } else if t.starts_with("<![CDATA[") {
            return None;
        } else if t.starts_with("<!--") {
            if !t.ends_with("-->") {
                return None;
            }
            line(&mut out, open.len(), t);
        } else if t.starts_with("<!") || t.starts_with("<?") {
            line(&mut out, open.len(), t);
        } else {
            // A quote still open once the quoted strings are gone: a `>` inside an attribute value split the tag.
            if QUOTED.replace_all(t, "").contains(['"', '\'']) {
                return None;
            }
            let m = XML_TAG.captures(t)?;
            let (close, name, self_close) = (&m[1], m.get(2)?.as_str(), &m[3]);
            if !close.is_empty() && !self_close.is_empty() {
                return None;
            }
            if !close.is_empty() {
                if open.pop() != Some(name) {
                    return None;
                }
                line(&mut out, open.len(), t);
            } else if !self_close.is_empty() {
                line(&mut out, open.len(), t);
            } else {
                // `<a>text</a>` and `<a></a>` stay on one line.
                let has_text = tokens.get(i + 1).is_some_and(|next| !next.starts_with('<'));
                let end = tokens.get(i + if has_text { 2 } else { 1 });
                if end.is_some_and(|e| e.strip_prefix("</").and_then(|e| e.strip_suffix('>')) == Some(name)) {
                    let text = if has_text { tokens[i + 1] } else { "" };
                    line(&mut out, open.len(), &format!("{t}{text}{}", end.unwrap()));
                    i += if has_text { 2 } else { 1 };
                } else {
                    line(&mut out, open.len(), t);
                    open.push(name);
                }
            }
        }
        i += 1;
    }
    open.is_empty().then(|| out.join("\n"))
}

/// Valid JSON text indented with 2 spaces, walked rather than re-serialised so that number and string text stay as
/// logged (`JSON.stringify` would turn 12345678901234567890 into 12345678901234567000).
fn indent_json(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len() * 2);
    let mut depth = 0usize;
    let mut in_string = false;
    let newline = |out: &mut String, depth: usize| {
        out.push('\n');
        (0..depth).for_each(|_| out.push_str("  "));
    };
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if in_string {
            out.push(ch);
            if ch == '\\' {
                i += 1;
                out.extend(chars.get(i));
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
            out.push(ch);
        } else if ch == '{' || ch == '[' {
            let mut j = i + 1;
            while chars.get(j).is_some_and(|c| js::is_space(*c)) {
                j += 1;
            }
            let close = if ch == '{' { '}' } else { ']' };
            if chars.get(j) == Some(&close) {
                out.push(ch);
                out.push(close); // an empty container stays on one line
                i = j;
            } else {
                depth += 1;
                out.push(ch);
                newline(&mut out, depth);
            }
        } else if ch == '}' || ch == ']' {
            // valid JSON closes only what it opened; the saturation keeps a bug from panicking the window
            depth = depth.saturating_sub(1);
            newline(&mut out, depth);
            out.push(ch);
        } else if ch == ',' {
            out.push(',');
            newline(&mut out, depth);
        } else if ch == ':' {
            out.push_str(": ");
        } else if !js::is_space(ch) {
            out.push(ch);
        }
        i += 1;
    }
    out
}

/// Whether `body` is JSON text (what `JSON.parse` takes) whose value is an object or an array.
///
/// `IgnoredAny` makes `serde_json` only check the text, walking it without recursion, so it has no limit of nesting
/// (`from_str` into a `Value` stops at 128 levels), as `JSON.parse` has none.
fn is_json_container(body: &str) -> bool {
    // JSON's whitespace is these four; the first other character of valid JSON tells what the value is
    let first = body.trim_start_matches([' ', '\t', '\n', '\r']).chars().next();
    matches!(first, Some('{' | '[')) && serde_json::from_str::<IgnoredAny>(body).is_ok() // only to check the text is valid
}

/// A JSON object or array re-indented with 2 spaces, well-formed XML indented; anything else as logged.
///
/// Gives the body itself, not a copy, when it is left as logged.
pub fn pretty_body(body: &str) -> Cow<'_, str> {
    // A bare scalar is left alone: there is nothing to indent.
    if is_json_container(body) {
        return Cow::Owned(indent_json(body));
    }
    match indent_xml(js::trim(body)) {
        Some(xml) => Cow::Owned(xml),
        None => Cow::Borrowed(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn call() -> HttpCall {
        HttpCall {
            method: "GET".into(),
            url: Some("http://h/x".into()),
            request_headers: vec![],
            request_body: None,
            status: Some(200),
            response_headers: vec![],
            response_body: None,
            millis: Some(12.0),
            error: None,
            via: None,
            start_time: 1792937525123.0,
            attempt: None,
        }
    }

    fn header(name: &str, value: &str) -> HttpHeader {
        HttpHeader {
            name: name.into(),
            value: value.into(),
        }
    }

    mod http_group {
        use super::*;

        #[test]
        fn classifies_by_hundred() {
            assert_eq!(http_group(Some(100)), Some(HttpGroup::Informational));
            assert_eq!(http_group(Some(199)), Some(HttpGroup::Informational));
            assert_eq!(http_group(Some(200)), Some(HttpGroup::Success));
            assert_eq!(http_group(Some(302)), Some(HttpGroup::Redirect));
            assert_eq!(http_group(Some(404)), Some(HttpGroup::ClientError));
            assert_eq!(http_group(Some(500)), Some(HttpGroup::ServerError));
            assert_eq!(http_group(Some(599)), Some(HttpGroup::ServerError));
            assert_eq!(http_group(None), Some(HttpGroup::NoResponse));
            assert_eq!(http_group(Some(99)), None);
            assert_eq!(http_group(Some(600)), None);
        }
    }

    mod http_groups {
        use super::*;

        #[test]
        fn lists_the_six_groups_in_order() {
            let groups: Vec<&str> = HTTP_GROUPS.iter().map(|g| g.group.as_str()).collect();
            assert_eq!(groups, ["1xx", "2xx", "3xx", "4xx", "5xx", "none"]);
            let labels: Vec<&str> = HTTP_GROUPS.iter().map(|g| g.label).collect();
            assert_eq!(
                labels,
                [
                    "1xx Informational",
                    "2xx Success",
                    "3xx Redirect",
                    "4xx Client error",
                    "5xx Server error",
                    "No response"
                ]
            );
            // and the enum is written as that text
            for g in HTTP_GROUPS {
                assert_eq!(serde_json::to_value(g.group).unwrap(), json!(g.group.as_str()));
            }
        }
    }

    mod curl_command {
        use super::*;

        #[test]
        fn drops_the_headers_curl_sets() {
            let c = HttpCall {
                url: Some("http://h/x?q=a%20b".into()),
                request_headers: vec![
                    header("Host", "h"),
                    header("content-length", "0"),
                    header("Connection", "keep-alive"),
                    header("Accept", "application/json"),
                    header("Authorization", "***"),
                ],
                ..call()
            };
            assert_eq!(
                curl_command(&c).as_deref(),
                Some("curl -X GET 'http://h/x?q=a%20b' -H 'Accept: application/json' -H 'Authorization: ***'")
            );
        }

        #[test]
        fn adds_the_body_as_logged() {
            let c = HttpCall {
                method: "POST".into(),
                request_body: Some("{\n  \"a\": \"it's\"\n}".into()),
                ..call()
            };
            assert_eq!(
                curl_command(&c).as_deref(),
                Some("curl -X POST 'http://h/x' --data-raw '{\n  \"a\": \"it'\\''s\"\n}'")
            );
        }

        #[test]
        fn adds_no_body_for_an_empty_one() {
            let c = HttpCall {
                method: "POST".into(),
                request_body: Some("".into()),
                ..call()
            };
            assert_eq!(curl_command(&c).as_deref(), Some("curl -X POST 'http://h/x'"));
        }

        #[test]
        fn sends_head_with_head_since_x_head_waits_for_a_body() {
            let headers = vec![header("Host", "h"), header("Accept", "*/*")];
            let c = HttpCall {
                method: "HEAD".into(),
                request_headers: headers,
                ..call()
            };
            assert_eq!(
                curl_command(&c).as_deref(),
                Some("curl --head 'http://h/x' -H 'Accept: */*'")
            );
            let lower = HttpCall {
                method: "head".into(),
                request_body: Some("x".into()),
                ..call()
            };
            assert_eq!(curl_command(&lower).as_deref(), Some("curl --head 'http://h/x'"));
            assert_eq!(
                curl_command(&HttpCall {
                    method: "HEAD".into(),
                    url: None,
                    ..call()
                }),
                None
            );
        }

        #[test]
        fn quotes_every_value() {
            let c = HttpCall {
                url: Some("http://h/it's".into()),
                request_headers: vec![header("X-A", "$HOME `id`")],
                ..call()
            };
            assert_eq!(
                curl_command(&c).as_deref(),
                Some("curl -X GET 'http://h/it'\\''s' -H 'X-A: $HOME `id`'")
            );
        }

        #[test]
        fn quotes_a_method_that_is_not_plain_letters() {
            let c = HttpCall {
                method: "$(id)".into(),
                ..call()
            };
            assert_eq!(curl_command(&c).as_deref(), Some("curl -X '$(id)' 'http://h/x'"));
        }

        #[test]
        fn is_null_without_a_url() {
            assert_eq!(curl_command(&HttpCall { url: None, ..call() }), None);
        }
    }

    mod pretty_body {
        use super::*;

        /// `JSON.stringify(JSON.parse(x), null, 2)`.
        fn restringified(x: &str) -> String {
            serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(x).unwrap()).unwrap()
        }

        #[test]
        fn re_indents_json() {
            let x = r#"{"a":1,"b":[1,2]}"#;
            assert_eq!(pretty_body(x), restringified(x));
            let indented = serde_json::to_string_pretty(&json!({ "a": 1, "b": [1, 2] })).unwrap();
            assert_eq!(pretty_body(&indented), indented);
        }

        #[test]
        fn keeps_number_text() {
            assert_eq!(
                pretty_body(r#"{"id":12345678901234567890,"price":1.50}"#),
                "{\n  \"id\": 12345678901234567890,\n  \"price\": 1.50\n}"
            );
        }

        #[test]
        fn keeps_punctuation_inside_strings() {
            assert_eq!(
                pretty_body(r#"{"a":"x,{y}:[z]\"q"}"#),
                "{\n  \"a\": \"x,{y}:[z]\\\"q\"\n}"
            );
            assert_eq!(
                pretty_body(r#"{"a":"p\\","b":"q"}"#),
                "{\n  \"a\": \"p\\\\\",\n  \"b\": \"q\"\n}"
            );
        }

        #[test]
        fn keeps_empty_containers_on_one_line() {
            assert_eq!(pretty_body(r#"{"a":{},"b":[]}"#), "{\n  \"a\": {},\n  \"b\": []\n}");
            assert_eq!(pretty_body("[ { } , [ ] ]"), "[\n  {},\n  []\n]");
        }

        #[test]
        fn matches_json_stringify_for_nested_ordinary_input() {
            let x = " {\"a\":[{\"b\":null,\"c\":[1,[2,{\"d\":true}]]},\"s\"],\n\"e\":{\"f\":-1.5}} ";
            assert_eq!(pretty_body(x), restringified(x));
        }

        #[test]
        fn leaves_a_json_scalar_as_logged() {
            assert_eq!(pretty_body("1.0"), "1.0");
            assert_eq!(pretty_body("null"), "null");
        }

        #[test]
        fn indents_xml() {
            assert_eq!(
                pretty_body("<user><name>bob</name></user>"),
                "<user>\n  <name>bob</name>\n</user>"
            );
            assert_eq!(
                pretty_body(r#"<?xml version="1.0"?><a x="1"><b/><c>t</c></a>"#),
                "<?xml version=\"1.0\"?>\n<a x=\"1\">\n  <b/>\n  <c>t</c>\n</a>"
            );
        }

        #[test]
        fn indents_xml_that_is_already_indented_and_keeps_comments_and_empty_elements() {
            let pretty = "<a>\n  <!-- 1 > 0 -->\n  <b></b>\n  <c>t</c>\n</a>";
            assert_eq!(pretty_body(pretty), pretty);
        }

        #[test]
        fn leaves_anything_else_as_logged() {
            for x in [
                "{not json",
                "<a><b></a>",
                "<a x=\"1>2\">t</a>",
                "<a><![CDATA[<b>]]></a>",
                "<a>1 < 2</a>",
                "<a></a> trailing",
                "<a>",
                "plain text",
                "",
            ] {
                assert_eq!(pretty_body(x), x);
                assert!(matches!(pretty_body(x), Cow::Borrowed(_)), "{x:?} is not copied");
            }
        }

        #[test]
        fn handles_a_200_kb_json_array() {
            let rows: Vec<_> = (0..3000)
                .map(|i| json!({ "id": i, "name": format!("user-{i}"), "email": format!("user-{i}@example.com"), "tags": ["a", "b"] }))
                .collect();
            let body = serde_json::to_string(&rows).unwrap();
            assert!(body.encode_utf16().count() > 200_000);
            let out = pretty_body(&body);
            assert_eq!(out, serde_json::to_string_pretty(&rows).unwrap());
            assert_eq!(serde_json::from_str::<Vec<serde_json::Value>>(&out).unwrap(), rows);
        }

        /// Not in the TypeScript: JSON nested deeper than `serde_json`'s own limit of 128 levels is indented too, and
        /// JavaScript's `\s` and `trim()` are not Rust's.
        #[test]
        fn indents_json_nested_past_128_levels_and_trims_as_javascript() {
            for levels in [100, 129, 1000] {
                let nested = format!("{}{}", "[".repeat(levels), "]".repeat(levels));
                let out = pretty_body(&nested);
                assert_eq!(out.lines().count(), levels * 2 - 1, "{levels} levels");
                assert!(out.starts_with("[\n  [\n"));
            }
            // U+FEFF is trimmed by JavaScript and not by Rust; U+0085 the other way round
            assert_eq!(pretty_body("\u{feff}<a><b/></a>\u{feff}"), "<a>\n  <b/>\n</a>");
            assert_eq!(pretty_body("\u{85}<a></a>"), "\u{85}<a></a>");
        }
    }
}
