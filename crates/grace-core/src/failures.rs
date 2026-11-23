//! `failures.ts`: failed tests that share an error, and a failed karate-config.js (docs/specs/
//! 2026-11-04-group-failures.md R2, R3, R6). `grace::failures` and `grace::details` only draw them.
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use crate::model::node::{Status, TestNode};
use crate::model::tags::{FilterState, filter_on, matching_leaves};
use crate::rows::leaves;
use crate::state::TestResult;

/// Karate's reason: the error from where it starts (R2.1); `None` without an error. `reason_at` counts UTF-16 units,
/// as JavaScript's `slice` does.
pub fn reason_of(r: Option<&TestResult>) -> Option<&str> {
    let r = r?;
    let error = r.error.as_deref()?;
    let want = r.reason_at.unwrap_or(0);
    let mut units = 0;
    let from = error
        .char_indices()
        .find(|(_, c)| {
            let reached = units >= want;
            units += c.len_utf16();
            reached
        })
        .map_or(error.len(), |(i, _)| i);
    Some(&error[from..])
}

/// What two reasons are compared by: the reason less a JS failure's position, which is its place (R2.2): a line
/// `Line: <n>, Col: <m>`, and a last line `at <n>:<m>`.
pub fn same_error(reason: &str) -> String {
    static POSITIONS: OnceLock<(Regex, Regex)> = OnceLock::new();
    let (line, last) = POSITIONS.get_or_init(|| {
        (
            Regex::new(r"(?m)^[ \t]*Line: \d+, Col: \d+\r?(\n|$)").unwrap(),
            Regex::new(r"\r?\n[ \t]*at \d+:\d+\s*$").unwrap(),
        )
    });
    last.replace(&line.replace_all(reason, ""), "").into_owned()
}

/// The tests of `n` the tree shows, in tree order: only the matching ones under "Hide non-matching" (R3.1).
pub fn shown_leaves<'a>(n: &'a TestNode, f: &FilterState) -> Vec<&'a TestNode> {
    if f.hide_non_matching && filter_on(f) {
        matching_leaves(n, f)
    } else {
        leaves(n)
    }
}

/// Failed tests that share an error: 2 or more, in tree order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureGroup {
    pub keys: Vec<String>,
}

/// The failed tests among `tests` (in tree order) that share an error, 2 or more each, in the order of their first
/// tests (R3.1).
pub fn failure_groups(
    tests: &[&TestNode],
    statuses: &HashMap<String, Status>,
    results: &HashMap<String, TestResult>,
) -> Vec<FailureGroup> {
    let mut by: HashMap<String, usize> = HashMap::new();
    let mut groups: Vec<Vec<String>> = Vec::new();
    for t in tests {
        if statuses.get(&t.key) != Some(&Status::Failed) {
            continue;
        }
        let Some(reason) = reason_of(results.get(&t.key)) else {
            continue;
        };
        let at = *by.entry(same_error(reason)).or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[at].push(t.key.clone());
    }
    groups
        .into_iter()
        .filter(|keys| keys.len() > 1)
        .map(|keys| FailureGroup { keys })
        .collect()
}

/// `groups`, largest first; groups of a size keep the order of their first tests (R5.1).
pub fn largest_first(mut groups: Vec<FailureGroup>) -> Vec<FailureGroup> {
    groups.sort_by_key(|g| std::cmp::Reverse(g.keys.len())); // a stable sort
    groups
}

/// The group `key` is in, and its place in it (R4.1).
pub fn group_of<'a>(groups: &'a [FailureGroup], key: &str) -> Option<(&'a FailureGroup, usize)> {
    groups
        .iter()
        .find_map(|g| g.keys.iter().position(|k| k == key).map(|i| (g, i)))
}

/// The key a step of `by` from `index` selects, wrapping round (R4.2).
pub fn step(group: &FailureGroup, index: usize, by: isize) -> &str {
    let n = group.keys.len() as isize;
    &group.keys[(index as isize + by).rem_euclid(n) as usize]
}

/// A failed config file, said once after a run (R6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigFailure {
    /// As Karate names it, without `classpath:`.
    pub file: String,
    pub line: u32,
    pub reason: String,
    pub count: usize,
}

const CONFIG: &str = "Config evaluation failed: ";

/// The config failure among the last run's failed tests (R6): the first in tree order, counting them all.
pub fn config_failure(
    tests: &[&TestNode],
    failed_keys: &[String],
    results: &HashMap<String, TestResult>,
) -> Option<ConfigFailure> {
    // a set: a run of 10,000 failed tests is looked up for each of them, at every draw of the window
    let failed: HashSet<&str> = failed_keys.iter().map(String::as_str).collect();
    let mut first: Option<&str> = None;
    let mut count = 0;
    for t in tests {
        if !failed.contains(t.key.as_str()) {
            continue;
        }
        let Some(reason) = reason_of(results.get(&t.key)).filter(|r| r.starts_with(CONFIG)) else {
            continue;
        };
        first.get_or_insert(reason);
        count += 1;
    }
    let first = first?;
    let head = first[CONFIG.len()..].lines().next().unwrap_or_default();
    let file = head.split_once(" - ").map_or(head, |(file, _)| file);
    static LINE: OnceLock<Regex> = OnceLock::new();
    let line = LINE
        .get_or_init(|| Regex::new("Line: ([0-9]+)").unwrap())
        .captures(first)
        .and_then(|c| c[1].parse().ok())
        .unwrap_or(1);
    Some(ConfigFailure {
        file: file.strip_prefix("classpath:").unwrap_or(file).to_string(),
        line,
        reason: first.to_string(),
        count,
    })
}

#[cfg(test)]
mod tests;
