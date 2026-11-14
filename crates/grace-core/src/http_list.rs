//! `src/renderer/http.ts` and the rules of `HttpCalls.tsx`: the HTTP tab's calls, from each test's calls, the tree
//! selection and the status groups switched on (docs/specs/2026-10-25-http-inspector.md R3).
//!
//! Electron Grace listed the narrowed calls on every change to count them for the tab, and kept a test's listed
//! calls per calls array (`WeakMap`) so unchanged rows were not drawn again (#124). Here the tab's count reads the
//! calls without listing them ([`shown_count`]), and the list is built only while the HTTP tab shows
//! (`grace::http`).
use std::collections::HashMap;

use crate::karate::tree::find_node;
use crate::model::http::{HTTP_GROUPS, HttpCall, HttpGroup, HttpHeader, curl_command, http_group, pretty_body};
use crate::model::node::TestNode;
use crate::rows::{leaves, node_name};
use crate::state::{AppState, CallRef};

/// A call of the list, with where it is and the test that made it.
#[derive(Clone, Debug)]
pub struct ListedCall<'a> {
    pub at: CallRef,
    pub call: &'a HttpCall,
    pub test: &'a TestNode,
}

/// The count of each chip: All, then each group in `HTTP_GROUPS`' order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupCounts {
    all: usize,
    groups: [usize; 6],
}

impl GroupCounts {
    /// `None` is All.
    pub fn get(&self, group: Option<HttpGroup>) -> usize {
        match group {
            None => self.all,
            Some(g) => self.groups[place(g)],
        }
    }
}

fn place(g: HttpGroup) -> usize {
    HTTP_GROUPS.iter().position(|x| x.group == g).unwrap_or_default()
}

/// The node the tree selection names, else the root: the tests under it are the list's.
fn scope<'a>(tree: &'a TestNode, selected: Option<&str>) -> &'a TestNode {
    selected.and_then(|k| find_node(tree, k)).unwrap_or(tree)
}

/// The tests under the tree selection (all of them with none).
pub fn scope_tests<'a>(tree: Option<&'a TestNode>, selected: Option<&str>) -> Vec<&'a TestNode> {
    tree.map(|t| leaves(scope(t, selected))).unwrap_or_default()
}

/// The calls of these tests, in run order: by start time, the tests' and the log's order breaking ties.
pub fn calls_of<'a>(tests: &[&'a TestNode], calls: &'a HashMap<String, Vec<HttpCall>>) -> Vec<ListedCall<'a>> {
    let mut list: Vec<ListedCall<'a>> = tests
        .iter()
        .flat_map(|&test| {
            calls
                .get(&test.key)
                .into_iter()
                .flatten()
                .enumerate()
                .map(move |(index, call)| ListedCall {
                    at: CallRef {
                        key: test.key.clone(),
                        index,
                    },
                    call,
                    test,
                })
        })
        .collect();
    list.sort_by(|a, b| a.call.start_time.total_cmp(&b.call.start_time)); // stable
    list
}

/// The calls of the tests under the tree selection (all of them with none), in run order.
pub fn narrowed_calls(s: &AppState) -> Vec<ListedCall<'_>> {
    calls_of(&scope_tests(s.tree.as_ref(), s.selected.as_deref()), &s.calls)
}

pub fn group_counts(list: &[ListedCall]) -> GroupCounts {
    let mut counts = GroupCounts {
        all: list.len(),
        ..GroupCounts::default()
    };
    for g in list.iter().filter_map(|l| http_group(l.call.status)) {
        counts.groups[place(g)] += 1;
    }
    counts
}

/// Whether a call is in any of the groups; with no group on, every call is.
fn in_groups(call: &HttpCall, groups: &[HttpGroup]) -> bool {
    groups.is_empty() || http_group(call.status).is_some_and(|g| groups.contains(&g))
}

/// The calls in any of the groups; with no group on, every call.
pub fn shown_calls<'a>(mut list: Vec<ListedCall<'a>>, groups: &[HttpGroup]) -> Vec<ListedCall<'a>> {
    list.retain(|l| in_groups(l.call, groups));
    list
}

/// The groups after a click on a chip; `None` is All, which turns every group off.
pub fn toggled_groups(groups: &[HttpGroup], g: Option<HttpGroup>) -> Vec<HttpGroup> {
    match g {
        None => Vec::new(),
        Some(g) if groups.contains(&g) => groups.iter().copied().filter(|x| *x != g).collect(),
        Some(g) => groups.iter().copied().chain([g]).collect(),
    }
}

pub fn total_calls(calls: &HashMap<String, Vec<HttpCall>>) -> usize {
    calls.values().map(Vec::len).sum()
}

/// The selected call with its test; `None` when it is not there any more.
pub fn find_call(s: &AppState) -> Option<ListedCall<'_>> {
    let at = s.selected_call.as_ref()?;
    let test = find_node(s.tree.as_ref()?, &at.key)?;
    let call = s.calls.get(&at.key)?.get(at.index)?;
    Some(ListedCall {
        at: at.clone(),
        call,
        test,
    })
}

/// How many calls under the tree selection are in `groups`, without listing them.
fn count_in(s: &AppState, groups: &[HttpGroup]) -> usize {
    let Some(tree) = &s.tree else { return 0 };
    leaves(scope(tree, s.selected.as_deref()))
        .iter()
        .filter_map(|t| s.calls.get(&t.key))
        .flatten()
        .filter(|c| in_groups(c, groups))
        .count()
}

/// The rows the HTTP tab shows, its count (R3): the calls under the tree selection in the groups switched on. It
/// lists nothing, so a run's calls cost the closed tab only this count (#124).
pub fn shown_count(s: &AppState) -> usize {
    count_in(s, &s.http_groups)
}

/// A message's headers as `Name: value` lines and its body indented, each present only when the call logged it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub headers: Option<String>,
    pub body: Option<String>,
}

/// What sits under a call's Response heading: what the server sent, or the error of a call that got no response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Response {
    Received(Message),
    Failed(String),
}

/// What the details pane reads of a call (`CallDetails.tsx`): worked out once for the call, not on each draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallText {
    /// The code and its group, `201 · 2xx Success`; `No response`; a code outside 100–599 alone.
    pub status: String,
    /// How many bytes of `status` are the code, which the badge sets in the mono.
    pub code_len: usize,
    pub test: String,
    /// The test's file and line.
    pub location: String,
    pub via: Option<String>,
    /// Under a Request heading, which shows only when a logged request has something to show.
    pub request: Option<Message>,
    pub response: Option<Response>,
    /// Copy as cURL's text; `None` when no request was logged.
    pub curl: Option<String>,
}

fn message(headers: &[HttpHeader], body: Option<&str>) -> Option<Message> {
    let headers = (!headers.is_empty()).then(|| {
        headers
            .iter()
            .map(|h| format!("{}: {}", h.name, h.value))
            .collect::<Vec<_>>()
            .join("\n")
    });
    let body = body.filter(|b| !b.is_empty()).map(|b| pretty_body(b).into_owned());
    (headers.is_some() || body.is_some()).then_some(Message { headers, body })
}

pub fn call_text(call: &HttpCall, test: &TestNode) -> CallText {
    let (status, code_len) = match (call.status, http_group(call.status)) {
        (None, _) => ("No response".to_owned(), 0),
        (Some(s), None) => (s.to_string(), s.to_string().len()),
        (Some(s), Some(_)) => (format!("{s} · {}", group_label(Some(s))), s.to_string().len()),
    };
    let response = match call.status {
        Some(_) => message(&call.response_headers, call.response_body.as_deref()).map(Response::Received),
        None => call.error.clone().filter(|e| !e.is_empty()).map(Response::Failed),
    };
    CallText {
        status,
        code_len,
        test: node_name(test),
        location: match test.line {
            Some(line) => format!("{}:{line}", test.file),
            None => test.file.clone(),
        },
        via: call.via.as_ref().map(|v| format!("via {v}")),
        // a request is the call's own only when it was logged: with no URL the headers are not its own
        request: call
            .url
            .as_ref()
            .and_then(|_| message(&call.request_headers, call.request_body.as_deref())),
        response,
        curl: curl_command(call),
    }
}

/// A call's accessible name: what its row reads, status first (virtual lists R5.4).
pub fn call_name(call: &HttpCall, test: &TestNode) -> String {
    let status = call.status.map_or_else(|| "no status".to_owned(), |s| s.to_string());
    let place = call
        .url
        .as_deref()
        .or_else(|| call.error.as_deref().and_then(|e| e.lines().next()))
        .unwrap_or_default();
    [status.as_str(), call.method.as_str(), place, node_name(test).as_str()].join(", ")
}

/// A status's group in words, for the badge's tooltip; a code outside 100–599 has no group, so it names itself.
pub fn group_label(status: Option<i32>) -> String {
    match http_group(status) {
        Some(g) => HTTP_GROUPS[place(g)].label.to_owned(),
        None => format!("Status {}", status.unwrap_or_default()),
    }
}

/// What the tab says when it shows no call: none at all, none under the selection, or none in the chosen groups.
pub fn empty_text(s: &AppState) -> String {
    if total_calls(&s.calls) == 0 {
        return "No HTTP calls yet. Calls appear when a feature finishes.".to_owned();
    }
    if count_in(s, &[]) > 0 {
        return "No calls in the chosen groups".to_owned();
    }
    let name = s.tree.as_ref().map_or_else(
        || "the project".to_owned(),
        |t| node_name(scope(t, s.selected.as_deref())),
    );
    format!("No HTTP calls in {name}")
}

#[cfg(test)]
mod tests;
