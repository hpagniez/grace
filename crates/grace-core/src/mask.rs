//! `src/main/mask.ts`: masks the values of secret run properties in what a run reports, so a secret never reaches the
//! window, the history or the console. Every spelling of a secret that can show up in text is masked: as typed,
//! URL-encoded, JSON-escaped and XML-escaped.
//!
//! The TypeScript walks any `AppEvent` as untyped JSON, at any depth, and masks every string except the fields in
//! `EXACT`; a field a future event gains is masked without anyone listing it. Here an event is typed, so the walk is
//! typed too (`Masker::mask_event`): every field is named in a destructuring with no `..`, so a field added to an event
//! stops this file compiling until someone decides whether it is run text. That keeps the TypeScript's guarantee, and
//! costs no copy of the event: strings are changed in place, and only those that hold a secret are touched. Parts of
//! an event that are still plain JSON (`Updates`, `UpdatePrompt`, `UpdateProgress`) go through the TypeScript's
//! generic walk, `Masker::mask_json`. The differences from the TypeScript are listed on `mask_event`.
use crate::model::env::{EnvView, RowView};
use crate::model::events::{AppEvent, DebugVar, Paused, Project, ScenarioUpdate, Step};
use crate::model::flaky::{FlakyEnv, FlakyRun, FlakyTest};
use crate::model::history::HistoryEntry;
use crate::model::http::{HttpCall, HttpHeader};
use crate::model::js;
use crate::model::node::TestNode;
use crate::model::tags::FilterState;
use serde_json::Value;

/// What a secret becomes.
pub const MASK: &str = "****";

/// Identifiers the window matches events and tests on, never run text: masking them would break it. The JSON keys the
/// generic walk (`Masker::mask_json`) leaves alone, at any depth; the typed walk names the same fields one by one.
const EXACT: [&str; 8] = ["type", "key", "keys", "selection", "runId", "status", "outcome", "env"];

/// `encodeURIComponent`: everything but `A-Z a-z 0-9 - _ . ! ~ * ' ( )` as `%XX` of its UTF-8 bytes. A `&str` holds
/// no lone surrogate, the one input the JavaScript function throws on (the TypeScript's `shortcut:` note).
fn encode_uri_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `JSON.stringify(value).slice(1, -1)`: the text as it sits inside a JSON string. `serde_json` escapes as JavaScript
/// does: `"`, `\`, `\b \f \n \r \t`, other control characters as lower-case `\u00xx`, the rest as it is.
fn json_escape(value: &str) -> String {
    let quoted = serde_json::to_string(value).unwrap_or_default();
    quoted
        .get(1..quoted.len().saturating_sub(1))
        .unwrap_or_default()
        .to_string()
}

/// The text as a debugger `xml` variable shows it.
fn xml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// Every spelling of a secret that can show up in text: as typed, URL-encoded and JSON-escaped (and XML-escaped), without
/// duplicates, in that order. An empty value has none, since it would match everywhere.
pub fn secret_forms(value: &str) -> Vec<String> {
    if value.is_empty() {
        return vec![];
    }
    let mut forms: Vec<String> = Vec::with_capacity(4);
    for form in [
        value.to_string(),
        encode_uri_component(value),
        json_escape(value),
        xml_escape(value),
    ] {
        if !forms.contains(&form) {
            forms.push(form);
        }
    }
    forms
}

/// `text` with its end masked when that end is the start of a secret's form: a debugger value cut at 64 KB can end
/// part-way through a secret, which the whole-form masker does not see. The longest such end is masked.
///
/// The TypeScript counts prefixes in UTF-16 units; a prefix of a form that ends inside a surrogate pair is no end of a
/// `&str`, so counting by characters picks the same end.
pub fn mask_cut_end(text: &str, values: &[impl AsRef<str>]) -> String {
    let mut cut = 0;
    for form in values.iter().flat_map(|v| secret_forms(v.as_ref())) {
        // the prefixes of the form, longest first, its whole not among them
        let ends = form.char_indices().map(|(i, _)| i).rev();
        for end in ends.filter(|&end| end > cut && end <= text.len()) {
            if text.ends_with(&form[..end]) {
                cut = end;
                break;
            }
        }
    }
    if cut == 0 {
        text.to_string()
    } else {
        format!("{}{MASK}", &text[..text.len() - cut])
    }
}

/// Replaces every form of every secret with `****`, longest form first, matching literally.
#[derive(Clone, Debug, Default)]
pub struct Masker {
    forms: Vec<String>,
}

impl Masker {
    /// A masker of these secret values (the TypeScript's `makeMasker`). Empty values are ignored.
    pub fn new(values: &[impl AsRef<str>]) -> Self {
        let mut forms: Vec<String> = values.iter().flat_map(|v| secret_forms(v.as_ref())).collect();
        // stable, as JavaScript's `sort`: of two forms of one length, the first stays first
        forms.sort_by_key(|f| std::cmp::Reverse(js::utf16_len(f)));
        Self { forms }
    }

    /// True when there is nothing to mask: every call changes nothing, and `mask_event` does not look at the event.
    pub fn is_empty(&self) -> bool {
        self.forms.is_empty()
    }

    /// Masks `text` where it is, allocating only if a form is in it.
    pub fn mask_in_place(&self, text: &mut String) {
        for form in &self.forms {
            if text.contains(form.as_str()) {
                *text = text.replace(form.as_str(), MASK);
            }
        }
    }

    /// `text` masked.
    pub fn mask(&self, text: &str) -> String {
        let mut masked = text.to_string();
        self.mask_in_place(&mut masked);
        masked
    }

    fn opt(&self, text: &mut Option<String>) {
        if let Some(text) = text {
            self.mask_in_place(text);
        }
    }

    fn all(&self, texts: &mut [String]) {
        texts.iter_mut().for_each(|t| self.mask_in_place(t));
    }

    /// The TypeScript's `maskEvent` on a JSON value: every string in it masked, at any depth, except the values of the
    /// keys in `EXACT`. Run text that a field of unknown name carries is masked without anyone listing it.
    pub fn mask_json(&self, value: &mut Value) {
        match value {
            Value::String(s) => self.mask_in_place(s),
            Value::Array(items) => items.iter_mut().for_each(|v| self.mask_json(v)),
            Value::Object(fields) => {
                for (key, v) in fields.iter_mut() {
                    if !EXACT.contains(&key.as_str()) {
                        self.mask_json(v);
                    }
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    /// The event with every string of it masked except the identifiers the window matches on (`EXACT`: the fields
    /// named `type`, `key`, `keys`, `selection`, `runId`, `status`, `outcome` and `env`, and what is in them).
    ///
    /// Differs from the TypeScript's walk, which works on untyped JSON, in four ways, none of which lets a secret
    /// through that it stopped:
    /// - A field that is an enum (a layout, a node kind, a step status, a variable's kind, a build tool, ...) is not
    ///   masked: its values are Grace's own words, and a secret equal to one would turn it into `****`, which the
    ///   window cannot read.
    /// - A row's values under an Examples column, and the rows of an environment, are masked whatever the column or
    ///   the environment is called. The TypeScript leaves a value alone when its *key* is in `EXACT`, so a column named
    ///   `status` or an environment named `env` kept its secrets.
    /// - A scenario's `reasonAt` follows its masked `error`: the TypeScript keeps the old offset, which after a masked
    ///   secret points into the middle of the text. See `scenario_update`.
    /// - A masker with no secret does not walk the event at all.
    pub fn mask_event(&self, event: &mut AppEvent) {
        if self.is_empty() {
            return;
        }
        // Every field is named, with no `..`: a field added to an event is a compile error here until it is placed.
        match event {
            AppEvent::Project {
                project,
                tree,
                error,
                subfolders,
            } => {
                if let Some(Project { root, layout: _ }) = project {
                    self.mask_in_place(root);
                }
                if let Some(tree) = tree {
                    self.node(tree);
                }
                self.opt(error);
                if let Some(subfolders) = subfolders {
                    self.all(subfolders);
                }
            }
            AppEvent::RunStarted {
                selection: _,
                run_id: _,
                keys: _,
                total: _,
                started_at: _,
                env: _,
                debug: _,
            } => {}
            AppEvent::Scenarios { updates } => updates.iter_mut().for_each(|u| self.scenario_update(u)),
            AppEvent::Breakpoints { breakpoints } => {
                for b in breakpoints {
                    self.mask_in_place(&mut b.file);
                }
            }
            AppEvent::DebugPaused { paused } => self.paused(paused),
            AppEvent::DebugResumed => {}
            AppEvent::Steps { key: _, steps } => steps.iter_mut().for_each(|s| self.step(s)),
            AppEvent::Calls { key: _, calls } => calls.iter_mut().for_each(|c| self.call(c)),
            AppEvent::Console { lines } => self.all(lines),
            AppEvent::RunFinished {
                outcome: _,
                exit_code: _,
                summary: _,
                report_available: _,
            } => {}
            AppEvent::Notice {
                message,
                success: _,
                build: _,
            } => self.mask_in_place(message),
            AppEvent::Filter { filter } => self.filter(filter),
            AppEvent::Threads { count: _ } => {}
            AppEvent::Watch { on: _ } => {}
            AppEvent::Environment { view } => self.env_view(view),
            AppEvent::History { runs } => runs.iter_mut().for_each(|r| self.history_entry(r)),
            AppEvent::Flaky { tests } => tests.iter_mut().for_each(|t| self.flaky_test(t)),
            AppEvent::Busy { busy: _ } => {}
            AppEvent::Listing { tool: _, build: _ } => {}
            AppEvent::Opening { root } => self.opt(root),
            AppEvent::Recent { entries } => {
                for e in entries {
                    self.mask_in_place(&mut e.path);
                }
            }
            AppEvent::Updates { view } => self.mask_json(view),
            AppEvent::UpdatePrompt { release } => self.mask_json(release),
            AppEvent::UpdateProgress { progress } => {
                if let Some(progress) = progress {
                    self.mask_json(progress);
                }
            }
        }
    }

    fn node(&self, n: &mut TestNode) {
        let TestNode {
            key: _,
            kind: _,
            name,
            file,
            line: _,
            ref_id,
            tags,
            example,
            children,
        } = n;
        self.mask_in_place(name);
        self.mask_in_place(file);
        self.opt(ref_id);
        self.all(tags);
        if let Some(example) = example {
            // the columns are Karate's names for the values, not run text
            example.values_mut().for_each(|v| self.mask_in_place(v));
        }
        children.iter_mut().for_each(|c| self.node(c));
    }

    /// The error is masked, and `reason_at`, a UTF-16 offset into it, moves with it. Where a secret is cut by the
    /// offset there is no offset to give, and the whole masked error is the reason, as when Karate gives none.
    fn scenario_update(&self, u: &mut ScenarioUpdate) {
        let ScenarioUpdate {
            key: _,
            status: _,
            duration_millis: _,
            error,
            reason_at,
        } = u;
        let Some(error) = error else { return };
        let masked = self.mask(error);
        if masked == *error {
            return;
        }
        if let Some(at) = reason_at {
            *reason_at = self.moved_offset(error, *at, &masked);
        }
        *error = masked;
    }

    /// Where the text from UTF-16 offset `at` of `error` starts in `masked`, `error` masked; `None` if a secret
    /// spans that offset.
    fn moved_offset(&self, error: &str, at: usize, masked: &str) -> Option<usize> {
        let mut units = 0;
        let split = error
            .char_indices()
            .find(|&(_, c)| {
                let here = units;
                units += c.len_utf16();
                here >= at
            })
            .map_or(error.len(), |(i, _)| i);
        let (head, tail) = error.split_at(split);
        let head = self.mask(head);
        (masked.len() == head.len() + self.mask(tail).len() && masked.starts_with(&head)).then(|| js::utf16_len(&head))
    }

    fn paused(&self, p: &mut Paused) {
        let Paused {
            key: _,
            file,
            line: _,
            depth: _,
            prefix,
            keyword,
            text,
            vars,
        } = p;
        self.mask_in_place(file);
        self.mask_in_place(prefix);
        self.mask_in_place(keyword);
        self.mask_in_place(text);
        for v in vars {
            let DebugVar {
                name,
                kind: _,
                // `type`: the Java class's name, an identifier of the window
                type_name: _,
                value,
                cut: _,
            } = v;
            self.mask_in_place(name);
            self.mask_in_place(value);
        }
    }

    fn step(&self, s: &mut Step) {
        let Step {
            keyword,
            text,
            line: _,
            status: _,
            millis: _,
            background: _,
            log,
            comments,
        } = s;
        self.mask_in_place(keyword);
        self.mask_in_place(text);
        self.opt(log);
        if let Some(comments) = comments {
            self.all(comments);
        }
    }

    fn headers(&self, headers: &mut [HttpHeader]) {
        for HttpHeader { name, value } in headers {
            self.mask_in_place(name);
            self.mask_in_place(value);
        }
    }

    fn call(&self, c: &mut HttpCall) {
        let HttpCall {
            method,
            url,
            request_headers,
            request_body,
            status: _,
            response_headers,
            response_body,
            millis: _,
            error,
            via,
            start_time: _,
            attempt: _,
        } = c;
        self.mask_in_place(method);
        self.opt(url);
        self.headers(request_headers);
        self.opt(request_body);
        self.headers(response_headers);
        self.opt(response_body);
        self.opt(error);
        self.opt(via);
    }

    fn filter(&self, f: &mut FilterState) {
        let FilterState {
            include,
            exclude,
            hide_non_matching: _,
        } = f;
        self.all(include);
        self.all(exclude);
    }

    fn env_view(&self, v: &mut EnvView) {
        let EnvView {
            env: _,
            suggestions,
            overlays,
            properties,
            secrets_available: _,
        } = v;
        self.all(suggestions);
        self.all(overlays);
        let rows = properties
            .shared
            .iter_mut()
            .chain(properties.by_env.values_mut().flatten());
        for row in rows {
            match row {
                RowView::Secret(r) => self.mask_in_place(&mut r.name),
                RowView::Plain(r) => {
                    self.mask_in_place(&mut r.name);
                    self.mask_in_place(&mut r.value);
                }
            }
        }
    }

    fn history_entry(&self, e: &mut HistoryEntry) {
        let HistoryEntry {
            id,
            started_at: _,
            duration_millis: _,
            env: _,
            threads: _,
            filter,
            // `selection`: an identifier of the window, and all of it
            selection: _,
            failed_of: _,
            outcome: _,
            exit_code: _,
            passed: _,
            failed: _,
            total: _,
            report_available: _,
            debug: _,
        } = e;
        self.mask_in_place(id);
        self.filter(filter);
    }

    fn flaky_test(&self, t: &mut FlakyTest) {
        let FlakyTest { key: _, envs } = t;
        for FlakyEnv {
            env: _,
            passed: _,
            failed: _,
            runs,
        } in envs
        {
            for FlakyRun {
                id,
                started_at: _,
                status: _,
                threads: _,
                report_available: _,
            } in runs
            {
                self.mask_in_place(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::env::{PlainRow, PropertyListsView, SecretRowView, SecretState};
    use crate::model::events::{Breakpoint, BuildTool, RecentEntry, RunOutcome, Summary, VarKind};
    use crate::model::flaky::FlakyStatus;
    use crate::model::history::StoredSelection;
    use crate::model::http::Attempt;
    use crate::model::node::{Layout, NodeKind, Status};
    use indexmap::IndexMap;
    use serde_json::json;

    mod secret_forms {
        use super::*;

        #[test]
        fn lists_the_value_its_url_encoded_json_escaped_and_xml_escaped_forms() {
            assert_eq!(
                secret_forms("a b\"c"),
                ["a b\"c", "a%20b%22c", "a b\\\"c", "a b&quot;c"]
            );
            assert!(secret_forms("a&<b>'").contains(&"a&amp;&lt;b&gt;&apos;".to_string()));
        }

        #[test]
        fn drops_forms_equal_to_one_already_listed() {
            assert_eq!(secret_forms("token"), ["token"]);
        }

        #[test]
        fn has_no_forms_for_an_empty_value() {
            assert!(secret_forms("").is_empty());
        }

        // Not in the TypeScript: the two escapers are written by hand here, so they are checked against what
        // `encodeURIComponent` and `JSON.stringify` give.
        #[test]
        fn escapes_as_javascript_does() {
            assert_eq!(
                encode_uri_component("é €😀 -_.!~*'()"),
                "%C3%A9%20%E2%82%AC%F0%9F%98%80%20-_.!~*'()"
            );
            assert_eq!(
                encode_uri_component("a/b?c=d&e#f%g+h,i;j:k@l$m"),
                "a%2Fb%3Fc%3Dd%26e%23f%25g%2Bh%2Ci%3Bj%3Ak%40l%24m"
            );
            assert_eq!(
                json_escape("a\"b\\c\n\r\t\u{8}\u{c}\u{1}\u{1f}é\u{7f}\u{2028}"),
                "a\\\"b\\\\c\\n\\r\\t\\b\\f\\u0001\\u001fé\u{7f}\u{2028}"
            );
            assert_eq!(secret_forms("é\n").len(), 3);
        }
    }

    mod mask_cut_end {
        use super::*;

        #[test]
        fn masks_an_end_that_starts_a_secret_longest_first() {
            assert_eq!(mask_cut_end("value tok", &["token"]), "value ****");
            assert_eq!(mask_cut_end("ab", &["abc", "b"]), "****");
        }

        #[test]
        fn leaves_an_end_that_starts_no_secret_and_a_whole_secret_for_the_masker() {
            assert_eq!(mask_cut_end("value x", &["token"]), "value x");
            assert_eq!(mask_cut_end("token", &["token"]), "token");
            assert_eq!(mask_cut_end("abc", &[] as &[&str]), "abc");
        }

        // Not in the TypeScript: ends of another spelling, and of text that is not ASCII.
        #[test]
        fn masks_the_end_of_another_spelling_and_of_a_text_that_is_not_ascii() {
            assert_eq!(mask_cut_end("x a%2", &["a b"]), "x ****");
            assert_eq!(mask_cut_end("ü caf", &["café"]), "ü ****");
            assert_eq!(mask_cut_end("é😀", &["😀x"]), "é****");
            assert_eq!(mask_cut_end("é😀x", &["😀xy"]), "é****");
        }
    }

    mod make_masker {
        use super::*;

        #[test]
        fn masks_every_occurrence() {
            let mask = Masker::new(&["s3cret"]);
            assert_eq!(mask.mask("a s3cret b s3cret\nx s3cret"), "a **** b ****\nx ****");
        }

        #[test]
        fn masks_url_encoded_and_json_escaped_forms() {
            let mask = Masker::new(&["a b\"c"]);
            assert_eq!(
                mask.mask("plain a b\"c, url a%20b%22c, json a b\\\"c"),
                "plain ****, url ****, json ****"
            );
        }

        #[test]
        fn masks_values_with_regex_metacharacters_literally() {
            let mask = Masker::new(&["$&", ".*", "a\\b"]);
            assert_eq!(
                mask.mask("pre $& mid .* post a\\b json a\\\\b end abc"),
                "pre **** mid **** post **** json **** end abc"
            );
        }

        #[test]
        fn masks_the_longest_form_first() {
            assert_eq!(Masker::new(&["abc", "abcdef"]).mask("xabcdefx"), "x****x");
        }

        #[test]
        fn changes_nothing_without_values() {
            assert_eq!(Masker::new(&[] as &[&str]).mask("some s3cret text"), "some s3cret text");
        }

        #[test]
        fn ignores_empty_values() {
            let mask = Masker::new(&["", "s3cret"]);
            assert_eq!(mask.mask("a s3cret b"), "a **** b");
            assert_eq!(Masker::new(&[""]).mask("a b"), "a b");
        }
    }

    const S: &str = "s3cret";

    fn masker() -> Masker {
        Masker::new(&[S])
    }

    fn masked(mut event: AppEvent) -> AppEvent {
        masker().mask_event(&mut event);
        event
    }

    fn node(kind: NodeKind, key: &str, name: &str) -> TestNode {
        TestNode {
            key: key.into(),
            kind,
            name: name.into(),
            file: "tests/x.feature".into(),
            line: None,
            ref_id: None,
            tags: vec![],
            example: None,
            children: vec![],
        }
    }

    fn update(error: &str) -> ScenarioUpdate {
        ScenarioUpdate {
            key: "s:a#1".into(),
            status: Status::Failed,
            duration_millis: None,
            error: Some(error.into()),
            reason_at: None,
        }
    }

    mod mask_event {
        use super::*;

        fn step(text: &str, log: &str, comment: &str) -> Step {
            Step {
                keyword: "*".into(),
                text: text.into(),
                line: 3,
                status: "passed".into(),
                millis: 1.0,
                background: false,
                log: Some(log.into()),
                comments: Some(vec![comment.into()]),
            }
        }

        #[test]
        fn masks_the_run_text_of_every_event() {
            let events = [
                (
                    AppEvent::Console {
                        lines: vec![format!("token {S}"), "plain".into()],
                    },
                    AppEvent::Console {
                        lines: vec!["token ****".into(), "plain".into()],
                    },
                ),
                (
                    AppEvent::Scenarios {
                        updates: vec![update(&format!("got {S}"))],
                    },
                    AppEvent::Scenarios {
                        updates: vec![update("got ****")],
                    },
                ),
                (
                    AppEvent::Steps {
                        key: "s:a#1".into(),
                        steps: vec![step(
                            &format!("def t = '{S}'"),
                            &format!("t = {S}"),
                            &format!("# token {S}"),
                        )],
                    },
                    AppEvent::Steps {
                        key: "s:a#1".into(),
                        steps: vec![step("def t = '****'", "t = ****", "# token ****")],
                    },
                ),
                (
                    AppEvent::notice(format!("bad line {S}")),
                    AppEvent::notice("bad line ****"),
                ),
            ];
            for (event, expected) in events {
                assert_eq!(masked(event), expected);
            }
        }

        #[test]
        fn masks_a_calls_event() {
            let call = HttpCall {
                method: "GET".into(),
                url: Some(format!("http://h/x?t={S}")),
                request_headers: vec![HttpHeader {
                    name: "Authorization".into(),
                    value: format!("Bearer {S}"),
                }],
                request_body: None,
                status: Some(200),
                response_headers: vec![],
                response_body: None,
                millis: Some(3.0),
                error: None,
                via: None,
                start_time: 1.0,
                attempt: None,
            };
            let event = AppEvent::Calls {
                key: "s:a#1".into(),
                calls: vec![call.clone()],
            };
            assert_eq!(
                masked(event),
                AppEvent::Calls {
                    key: "s:a#1".into(),
                    calls: vec![HttpCall {
                        url: Some("http://h/x?t=****".into()),
                        request_headers: vec![HttpHeader {
                            name: "Authorization".into(),
                            value: "Bearer ****".into(),
                        }],
                        ..call
                    }],
                }
            );
        }

        /// The TypeScript hands the walk an event with fields the union does not name. A typed event has none: the
        /// walk that finds run text by shape is `mask_json`, which the plain JSON parts of an event go through.
        #[test]
        fn masks_a_field_it_does_not_know_nested_too() {
            let mut event = json!({
                "type": "runFinished",
                "outcome": "failed",
                "exitCode": 1,
                "summary": null,
                "reportAvailable": false,
                "extra": format!("x {S} y"),
                "nested": { "list": [{ "text": S }, 2, true, null] },
            });
            masker().mask_json(&mut event);
            assert_eq!(
                event,
                json!({
                    "type": "runFinished",
                    "outcome": "failed",
                    "exitCode": 1,
                    "summary": null,
                    "reportAvailable": false,
                    "extra": "x **** y",
                    "nested": { "list": [{ "text": "****" }, 2, true, null] },
                })
            );
            let view = json!({ "text": format!("a {S}"), "key": S, "list": [{ "status": S, "note": S }] });
            assert_eq!(
                masked(AppEvent::Updates { view }),
                AppEvent::Updates {
                    view: json!({ "text": "a ****", "key": S, "list": [{ "status": S, "note": "****" }] })
                }
            );
        }

        #[test]
        fn leaves_the_fields_the_window_matches_on_exact() {
            let m = Masker::new(&["a", "2026"]);
            let started = AppEvent::RunStarted {
                selection: "s:a#1".into(),
                run_id: "20261025-143212-007".into(),
                keys: vec!["s:a#1".into(), "s:a#2".into()],
                total: 2,
                started_at: 5.0,
                env: Some("qa".into()),
                debug: false,
            };
            let updates = AppEvent::Scenarios {
                updates: vec![ScenarioUpdate {
                    key: "s:a#1".into(),
                    status: Status::Passed,
                    duration_millis: None,
                    error: None,
                    reason_at: None,
                }],
            };
            let finished = AppEvent::RunFinished {
                outcome: RunOutcome::Cancelled,
                exit_code: Some(1),
                summary: Some(Summary {
                    scenario_count: 2,
                    scenarios_passed: 1,
                    scenarios_failed: 1,
                    duration_millis: 9.0,
                }),
                report_available: false,
            };
            for original in [started, updates, finished] {
                let mut event = original.clone();
                m.mask_event(&mut event);
                assert_eq!(event, original);
            }
        }

        // Not in the TypeScript. The typed walk must mask what the TypeScript's generic walk masks: for an event of
        // every kind, with the secret in every string of it, the typed walk and `mask_json` of its JSON agree.
        #[test]
        fn the_typed_walk_masks_what_the_generic_walk_masks() {
            for event in samples() {
                let mut want = serde_json::to_value(&event).unwrap();
                masker().mask_json(&mut want);
                let got = serde_json::to_value(masked(event.clone())).unwrap();
                assert_eq!(got, want, "{event:?}");
            }
        }

        // Not in the TypeScript: the secret is left only in an identifier, in any sample.
        #[test]
        fn no_secret_is_left_in_a_sample_but_in_an_identifier() {
            let exact = exact();
            for event in samples() {
                let text = serde_json::to_string(&masked(event)).unwrap();
                assert_eq!(text.matches(S).count(), text.matches(exact.as_str()).count(), "{text}");
            }
        }

        // Not in the TypeScript: an event with no secret to mask is not walked.
        #[test]
        fn a_masker_with_no_secret_changes_no_event() {
            let m = Masker::new(&[""]);
            assert!(m.is_empty());
            for event in samples() {
                let mut same = event.clone();
                m.mask_event(&mut same);
                assert_eq!(same, event);
            }
        }

        // Not in the TypeScript: the enums are Grace's words, not run text.
        #[test]
        fn a_secret_equal_to_a_word_of_grace_leaves_the_words() {
            let m = Masker::new(&["folder", "maven", "string", "Maven", "failed"]);
            let mut event = AppEvent::Project {
                project: Some(Project {
                    root: "/p".into(),
                    layout: Layout::Maven,
                }),
                tree: Some(node(NodeKind::Folder, "d:", "folder")),
                error: None,
                subfolders: None,
            };
            m.mask_event(&mut event);
            let AppEvent::Project { project, tree, .. } = event else {
                unreachable!()
            };
            assert_eq!(project.unwrap().layout, Layout::Maven);
            let tree = tree.unwrap();
            assert_eq!((tree.kind, tree.name.as_str()), (NodeKind::Folder, "****"));
            let listing = AppEvent::Listing {
                tool: Some(BuildTool::Maven),
                build: false,
            };
            let mut event = listing.clone();
            m.mask_event(&mut event);
            assert_eq!(event, listing);
            let failed = AppEvent::Scenarios {
                updates: vec![update("x")],
            };
            let mut event = failed.clone();
            m.mask_event(&mut event);
            assert_eq!(event, failed);
        }

        // Not in the TypeScript: a column or an environment named like an identifier keeps no secret.
        #[test]
        fn masks_example_values_and_environment_rows_whatever_their_names() {
            let mut row = node(NodeKind::Row, "s:a#1.1", "row");
            row.example = Some(IndexMap::from([
                ("status".to_string(), format!("a {S}")),
                ("env".to_string(), format!("b {S}")),
                ("pw".to_string(), format!("c {S}")),
            ]));
            let AppEvent::Project { tree: Some(row), .. } = masked(AppEvent::Project {
                project: None,
                tree: Some(row),
                error: None,
                subfolders: None,
            }) else {
                unreachable!()
            };
            assert_eq!(
                row.example.unwrap().into_iter().collect::<Vec<_>>(),
                [
                    ("status".to_string(), "a ****".to_string()),
                    ("env".to_string(), "b ****".to_string()),
                    ("pw".to_string(), "c ****".to_string()),
                ]
            );
            let view = EnvView {
                env: Some("env".into()),
                suggestions: vec![],
                overlays: vec![],
                properties: PropertyListsView {
                    shared: vec![],
                    by_env: IndexMap::from([(
                        "env".to_string(),
                        vec![RowView::Plain(PlainRow {
                            name: "n".into(),
                            value: S.into(),
                            env_var: None,
                        })],
                    )]),
                },
                secrets_available: true,
            };
            let AppEvent::Environment { view } = masked(AppEvent::Environment { view }) else {
                unreachable!()
            };
            let RowView::Plain(row) = &view.properties.by_env["env"][0] else {
                unreachable!()
            };
            assert_eq!(row.value, "****");
            assert_eq!(view.env.as_deref(), Some("env"));
        }

        // Not in the TypeScript, which keeps the old `reasonAt` after a secret was masked.
        mod reason_at {
            use super::*;

            fn reason(error: &str, at: usize) -> (String, Option<usize>) {
                let AppEvent::Scenarios { mut updates } = masked(AppEvent::Scenarios {
                    updates: vec![ScenarioUpdate {
                        reason_at: Some(at),
                        ..update(error)
                    }],
                }) else {
                    unreachable!()
                };
                let u = updates.remove(0);
                (u.error.unwrap(), u.reason_at)
            }

            #[test]
            fn follows_the_masked_error() {
                // `s3cret` (6 units) before the reason becomes `****` (4): the reason starts 2 earlier
                let error = format!("step {S} failed: boom");
                let at = js::utf16_len(&format!("step {S} failed: "));
                assert_eq!(reason(&error, at), ("step **** failed: boom".to_string(), Some(at - 2)));
            }

            #[test]
            fn stays_when_the_secret_is_after_the_reason_or_not_there() {
                let error = format!("step failed: got {S}");
                assert_eq!(reason(&error, 13), ("step failed: got ****".to_string(), Some(13)));
                assert_eq!(
                    reason("step failed: boom", 13),
                    ("step failed: boom".to_string(), Some(13))
                );
                assert_eq!(reason(&format!("a {S}"), 0), ("a ****".to_string(), Some(0)));
            }

            #[test]
            fn counts_utf16_units() {
                let error = format!("😀 {S}: é boom");
                let at = js::utf16_len(&format!("😀 {S}: "));
                assert_eq!(
                    reason(&error, at),
                    ("😀 ****: é boom".to_string(), Some(js::utf16_len("😀 ****: ")))
                );
            }

            #[test]
            fn is_dropped_when_a_secret_spans_it() {
                // the reason starts inside the secret: the whole masked error is the reason
                let error = format!("x {S}");
                assert_eq!(reason(&error, 4), ("x ****".to_string(), None));
            }
        }
    }

    /// Text with the secret in it.
    fn t(label: &str) -> String {
        format!("{label} {S}")
    }

    /// An identifier of the window, with the secret in it: it must stay as it is.
    fn exact() -> String {
        format!("exact {S}")
    }

    /// An event of every kind, with the secret in every string of it, so a field the typed walk misses shows.
    fn samples() -> Vec<AppEvent> {
        let tree = TestNode {
            key: exact(),
            kind: NodeKind::Folder,
            name: t("root"),
            file: t("dir"),
            line: Some(1),
            ref_id: Some(t("ref")),
            tags: vec![t("tag")],
            example: None,
            children: vec![TestNode {
                example: Some(IndexMap::from([("col".to_string(), t("cell"))])),
                tags: vec![t("tag2"), "plain".into()],
                ..node(NodeKind::Row, &exact(), &t("row"))
            }],
        };
        let filter = FilterState {
            include: vec![t("in")],
            exclude: vec![t("out")],
            hide_non_matching: true,
        };
        let selection = StoredSelection {
            kind: NodeKind::Feature,
            file: exact(),
            name: exact(),
            line: Some(2),
            key: exact(),
        };
        let call = HttpCall {
            method: t("m"),
            url: Some(t("url")),
            request_headers: vec![HttpHeader {
                name: t("hn"),
                value: t("hv"),
            }],
            request_body: Some(t("rb")),
            status: Some(200),
            response_headers: vec![HttpHeader {
                name: t("hn2"),
                value: t("hv2"),
            }],
            response_body: Some(t("pb")),
            millis: Some(3.0),
            error: Some(t("err")),
            via: Some(t("via")),
            start_time: 1.0,
            attempt: Some(Attempt { n: 1, of: 2 }),
        };
        vec![
            AppEvent::Project {
                project: Some(Project {
                    root: t("root"),
                    layout: Layout::Maven,
                }),
                tree: Some(tree),
                error: Some(t("error")),
                subfolders: Some(vec![t("sub")]),
            },
            AppEvent::Project {
                project: None,
                tree: None,
                error: None,
                subfolders: None,
            },
            AppEvent::RunStarted {
                selection: exact(),
                run_id: exact(),
                keys: vec![exact()],
                total: 1,
                started_at: 1.0,
                env: Some(exact()),
                debug: false,
            },
            AppEvent::Scenarios {
                updates: vec![ScenarioUpdate {
                    key: exact(),
                    status: Status::Failed,
                    duration_millis: Some(5.0),
                    error: Some(t("error")),
                    reason_at: None,
                }],
            },
            AppEvent::Breakpoints {
                breakpoints: vec![Breakpoint { file: t("bp"), line: 3 }],
            },
            AppEvent::DebugPaused {
                paused: Paused {
                    key: Some(exact()),
                    file: t("file"),
                    line: 4,
                    depth: 0,
                    prefix: t("prefix"),
                    keyword: t("keyword"),
                    text: t("text"),
                    vars: vec![
                        DebugVar {
                            name: t("var"),
                            kind: VarKind::Other,
                            type_name: Some(exact()),
                            value: t("value"),
                            cut: Some(7),
                        },
                        DebugVar {
                            name: t("v2"),
                            kind: VarKind::String,
                            type_name: None,
                            value: t("value2"),
                            cut: None,
                        },
                    ],
                },
            },
            AppEvent::DebugResumed,
            AppEvent::Steps {
                key: exact(),
                steps: vec![Step {
                    keyword: t("kw"),
                    text: t("text"),
                    line: 5,
                    status: exact(),
                    millis: 1.0,
                    background: false,
                    log: Some(t("log")),
                    comments: Some(vec![t("c1"), t("c2")]),
                }],
            },
            AppEvent::Calls {
                key: exact(),
                calls: vec![call],
            },
            AppEvent::Console {
                lines: vec![t("l1"), "plain".into(), t("l2")],
            },
            AppEvent::RunFinished {
                outcome: RunOutcome::Failed,
                exit_code: Some(1),
                summary: None,
                report_available: true,
            },
            AppEvent::Notice {
                message: t("message"),
                success: Some(true),
                build: true,
            },
            AppEvent::Filter { filter: filter.clone() },
            AppEvent::Threads { count: 2 },
            AppEvent::Watch { on: true },
            AppEvent::Environment {
                view: EnvView {
                    env: Some(exact()),
                    suggestions: vec![t("sug")],
                    overlays: vec![t("overlay")],
                    properties: PropertyListsView {
                        shared: vec![
                            RowView::Plain(PlainRow {
                                name: t("pn"),
                                value: t("pv"),
                                env_var: Some(true),
                            }),
                            RowView::Secret(SecretRowView {
                                name: t("sn"),
                                secret: true,
                                state: SecretState::Set,
                                env_var: None,
                            }),
                        ],
                        by_env: IndexMap::from([(
                            "qa".to_string(),
                            vec![RowView::Plain(PlainRow {
                                name: t("qn"),
                                value: t("qv"),
                                env_var: None,
                            })],
                        )]),
                    },
                    secrets_available: true,
                },
            },
            AppEvent::History {
                runs: vec![HistoryEntry {
                    id: t("id"),
                    started_at: 1.0,
                    duration_millis: 2.0,
                    env: Some(exact()),
                    threads: Some(2),
                    filter,
                    selection: Some(selection),
                    failed_of: None,
                    outcome: RunOutcome::Finished,
                    exit_code: Some(0),
                    passed: 1,
                    failed: 0,
                    total: 1,
                    report_available: true,
                    debug: false,
                }],
            },
            AppEvent::Flaky {
                tests: vec![FlakyTest {
                    key: exact(),
                    envs: vec![FlakyEnv {
                        env: Some(exact()),
                        passed: 1,
                        failed: 1,
                        runs: vec![FlakyRun {
                            id: t("run"),
                            started_at: 1.0,
                            status: FlakyStatus::Failed,
                            threads: None,
                            report_available: false,
                        }],
                    }],
                }],
            },
            AppEvent::Busy { busy: true },
            AppEvent::Listing {
                tool: Some(BuildTool::Gradle),
                build: true,
            },
            AppEvent::Opening { root: Some(t("root")) },
            AppEvent::Opening { root: None },
            AppEvent::Recent {
                entries: vec![RecentEntry {
                    path: t("path"),
                    exists: true,
                }],
            },
            AppEvent::Updates {
                view: json!({ "text": t("a"), "key": exact(), "list": [t("b"), 1] }),
            },
            AppEvent::UpdatePrompt {
                release: json!({ "url": t("u"), "version": exact() }),
            },
            AppEvent::UpdateProgress {
                progress: Some(json!({ "type": exact(), "message": t("m") })),
            },
            AppEvent::UpdateProgress { progress: None },
        ]
    }
}
