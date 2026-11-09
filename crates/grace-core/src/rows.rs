//! `treeRows.ts`, `nodes.ts` and `virtual.ts`: the tree's rows and the rules of a list of rows, without a window.
//!
//! `virtual.ts`'s `drawnRange` is not here: GPUI's `uniform_list` draws only the rows in view itself.
use std::collections::{HashMap, HashSet};

use crate::model::flaky::FlakyEnv;
use crate::model::node::{NodeKind, Status, TestNode};
use crate::model::tags::{FilterState, filter_on, matching_leaves};

/// One row of the tree: where its node is and where it sits among the shown rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub key: String,
    /// The node's place under the root: the index of each child on the way down.
    pub path: Vec<usize>,
    /// aria-level: 1 for the root's children.
    pub level: usize,
    /// Shown siblings.
    pub set_size: usize,
    /// 1-based among shown siblings.
    pub pos_in_set: usize,
    /// `None` at level 1.
    pub parent_key: Option<String>,
}

impl TreeRow {
    /// The row's node in `root`, the tree the row was built from.
    pub fn node<'a>(&self, root: &'a TestNode) -> &'a TestNode {
        self.path.iter().fold(root, |n, &i| &n.children[i])
    }
}

/// The rows a person can scroll to, in order: children of closed nodes left out, and non-matching rows left out
/// under "Hide non-matching" (R2.1). The root itself is not a row. Reads no status, so a status update never changes
/// the rows (R2.3).
pub fn tree_rows(root: &TestNode, is_open: impl Fn(&str) -> bool, filter: &FilterState) -> Vec<TreeRow> {
    let hide = filter.hide_non_matching && filter_on(filter);
    let mut rows = Vec::new();
    let mut path = Vec::new();
    walk(root, 1, None, hide, filter, &is_open, &mut path, &mut rows);
    rows
}

#[allow(clippy::too_many_arguments)]
fn walk(
    parent: &TestNode,
    level: usize,
    parent_key: Option<&str>,
    hide: bool,
    filter: &FilterState,
    is_open: &impl Fn(&str) -> bool,
    path: &mut Vec<usize>,
    rows: &mut Vec<TreeRow>,
) {
    // A folder, feature or outline matches when a test under it does.
    let shown: Vec<usize> = (0..parent.children.len())
        .filter(|&i| !hide || !matching_leaves(&parent.children[i], filter).is_empty())
        .collect();
    for (pos, &i) in shown.iter().enumerate() {
        let node = &parent.children[i];
        path.push(i);
        rows.push(TreeRow {
            key: node.key.clone(),
            path: path.clone(),
            level,
            set_size: shown.len(),
            pos_in_set: pos + 1,
            parent_key: parent_key.map(str::to_owned),
        });
        if is_open(&node.key) {
            walk(node, level + 1, Some(&node.key), hide, filter, is_open, path, rows);
        }
        path.pop();
    }
}

/// `STATUS_LABEL`: a status in words.
pub fn status_label(s: Status) -> &'static str {
    match s {
        Status::NotRun => "Not run",
        Status::Pending => "Pending",
        Status::Running => "Running",
        Status::Passed => "Passed",
        Status::Failed => "Failed",
        Status::Interrupted => "Interrupted",
    }
}

/// A row's accessible name (R5.2): label, status word in lower case, the flaky chip's words if any, then tags.
pub fn row_name(label: &str, status: Status, flaky: Option<&str>, tags: &[String]) -> String {
    let status = status_label(status).to_lowercase();
    let mut parts = vec![label, status.as_str()];
    parts.extend(flaky);
    parts.extend(tags.iter().map(String::as_str));
    parts.join(", ")
}

/// The key of the feature `key` sits in, read from the key as the runner builds it: `s:` and `o:` keys end in
/// `#<section>`. `None` for a folder or no key.
pub fn feature_of(key: Option<&str>) -> Option<String> {
    let key = key?;
    if key.starts_with("f:") {
        return Some(key.to_owned());
    }
    let rest = key.strip_prefix("s:").or_else(|| key.strip_prefix("o:"))?;
    // `lastIndexOf` gives -1 with no `#`, and `slice(2, -1)` then drops the last character
    let end = rest.rfind('#').unwrap_or(rest.len().saturating_sub(1));
    Some(format!("f:{}", &rest[..end]))
}

/// The keys of the nodes above `key`, from the root down; empty when `key` is not under `n`.
pub fn ancestors(n: &TestNode, key: &str) -> Vec<String> {
    for c in &n.children {
        if c.key == key {
            return vec![n.key.clone()];
        }
        let above = ancestors(c, key);
        if !above.is_empty() {
            let mut out = vec![n.key.clone()];
            out.extend(above);
            return out;
        }
    }
    Vec::new()
}

/// A node's name; Karate gives an unnamed `Scenario:` an empty name.
pub fn node_name(n: &TestNode) -> String {
    if n.name.is_empty() {
        // `${n.line}` of a null line is "null"
        let line = n.line.map_or_else(|| "null".to_owned(), |l| l.to_string());
        format!("Scenario at line {line}")
    } else {
        n.name.clone()
    }
}

/// A node's label as its tree row shows it: a feature by its file name.
pub fn label_of(n: &TestNode) -> String {
    if n.kind == NodeKind::Feature {
        n.file.rsplit('/').next().unwrap_or_default().to_owned()
    } else {
        node_name(n)
    }
}

/// Scenarios and rows at or under `n`.
pub fn leaves(n: &TestNode) -> Vec<&TestNode> {
    fn walk<'a>(n: &'a TestNode, out: &mut Vec<&'a TestNode>) {
        if n.is_leaf() {
            out.push(n);
        } else {
            n.children.iter().for_each(|c| walk(c, out));
        }
    }
    let mut out = Vec::new();
    walk(n, &mut out);
    out
}

/// A folder, feature or outline matches when a test under it does; with the filter off all match.
pub fn row_matches(n: &TestNode, f: &FilterState) -> bool {
    !filter_on(f) || !matching_leaves(n, f).is_empty()
}

/// What a row's flaky chip says, or `None`: it counts the tests the tag filter hides too, as flakiness is not about
/// tags.
pub fn flaky_chip(n: &TestNode, flaky: &HashMap<String, Vec<FlakyEnv>>) -> Option<String> {
    let count = leaves(n).iter().filter(|l| flaky.contains_key(&l.key)).count();
    match count {
        0 => None,
        _ if n.is_leaf() => Some("flaky".to_owned()),
        _ => Some(format!("{count} flaky")),
    }
}

/// Where a row starts, open or closed: a feature closed, every other row open (#224 R1).
pub fn starts_open(key: &str) -> bool {
    !key.starts_with("f:")
}

/// The rows turned from where they start, by key. A refresh keeps them; another project starts afresh (#224 R1).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Turned(HashSet<String>);

impl Turned {
    pub fn is_open(&self, key: &str) -> bool {
        starts_open(key) != self.0.contains(key)
    }

    pub fn set_open(&mut self, key: &str, open: bool) {
        if open == starts_open(key) {
            self.0.remove(key);
        } else {
            self.0.insert(key.to_owned());
        }
    }
}

/// px, a list whose height is not known yet.
pub const FALLBACK_VIEW: f32 = 600.;

/// Where a list of fixed-height rows is scrolled, and how tall it and its rows are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub scroll_top: f32,
    /// The list's height; 0 means `FALLBACK_VIEW`.
    pub height: f32,
    pub row_height: f32,
    /// px before the first row.
    pub pad: f32,
}

impl View {
    fn height(&self) -> f32 {
        if self.height > 0. { self.height } else { FALLBACK_VIEW }
    }
}

/// The scroll top that shows row `index` with the least scroll (R4.1); `v.scroll_top` when it is in view. Row 0 is
/// 0, its pad shown too.
pub fn scroll_to_show(index: usize, v: View) -> f32 {
    if index == 0 {
        return 0.;
    }
    let top = v.pad + index as f32 * v.row_height;
    let bottom = top + v.row_height;
    if top < v.scroll_top {
        top
    } else if bottom > v.scroll_top + v.height() {
        bottom - v.height()
    } else {
        v.scroll_top
    }
}

/// Whole rows in view, at least 1 (Page Up, Page Down).
pub fn page_rows(v: View) -> usize {
    ((v.height() / v.row_height).floor() as usize).max(1)
}

/// The scroll top that keeps the first row in view still on screen when the rows change (R4.6). 0 stays 0; an
/// anchor that is gone keeps `v.scroll_top`.
pub fn anchored_scroll_top(v: View, before: &[String], after: &[String]) -> f32 {
    if v.scroll_top == 0. {
        return 0.;
    }
    let index = ((v.scroll_top - v.pad) / v.row_height).floor();
    let Some(anchor) = (index >= 0.).then(|| before.get(index as usize)).flatten() else {
        return v.scroll_top;
    };
    match after.iter().position(|k| k == anchor) {
        // The offset into the anchor row survives, so the rows do not jump by a part of a row.
        Some(now) => v.scroll_top + (now as f32 - index) * v.row_height,
        None => v.scroll_top,
    }
}

/// Where focus goes when `key` leaves the list: the nearest row before it that is still there, else the first row,
/// else `None` (R4.4, R4.5).
pub fn survivor(key: &str, before: &[String], after: &[String]) -> Option<String> {
    let kept: HashSet<&str> = after.iter().map(String::as_str).collect();
    let at = before.iter().position(|k| k == key).unwrap_or(0);
    before[..at]
        .iter()
        .rev()
        .find(|k| kept.contains(k.as_str()))
        .or_else(|| after.first())
        .cloned()
}

#[cfg(test)]
mod tests;
