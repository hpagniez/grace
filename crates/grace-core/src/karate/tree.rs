//! `src/main/karate/tree.ts`: the test tree, built from the depth-0 events of a dry run (AGENTS.md: key nodes by
//! feature + `refId`; filter `callDepth == 0`), and the lookups the runner makes in it.
use super::events::{DataError, EventKind, KarateEvent};
use super::js;
use crate::model::history::{LeafKind, StoredTest};
use crate::model::node::{NodeKind, TestNode};
use indexmap::IndexMap;
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static REF_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\[([0-9]+)(?:\.([0-9]+))?:([0-9]+)\]$").expect("a fixed pattern"));

/// A Karate refId, `[section:line]` or `[section.example:line]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefId {
    pub section: u64,
    pub example: Option<u64>,
    pub line: u64,
}

pub fn parse_ref_id(ref_id: &str) -> Result<RefId, DataError> {
    let invalid = || DataError::new(format!("invalid refId: {ref_id}"));
    let caps = REF_ID.captures(ref_id).ok_or_else(invalid)?;
    let number = |at: usize| caps.get(at).map(|m| m.as_str().parse::<u64>());
    Ok(RefId {
        section: number(1).and_then(Result::ok).ok_or_else(invalid)?,
        example: match number(2) {
            None => None,
            Some(n) => Some(n.map_err(|_| invalid())?),
        },
        line: number(3).and_then(Result::ok).ok_or_else(invalid)?,
    })
}

fn node(kind: NodeKind, key: String, name: String, file: &str) -> TestNode {
    TestNode {
        key,
        kind,
        name,
        file: file.to_owned(),
        line: None,
        ref_id: None,
        tags: Vec::new(),
        example: None,
        children: Vec::new(),
    }
}

fn by_file(a: &TestNode, b: &TestNode) -> std::cmp::Ordering {
    js::cmp(&a.file, &b.file)
}

fn sort_folders(folder: &mut TestNode) {
    let (mut folders, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut folder.children)
        .into_iter()
        .partition(|c| c.kind == NodeKind::Folder);
    let mut features: Vec<_> = rest.into_iter().filter(|c| c.kind == NodeKind::Feature).collect();
    // stable, by UTF-16 units as JavaScript's `<` compares
    folders.sort_by(by_file);
    features.sort_by(by_file);
    folders.iter_mut().for_each(sort_folders);
    folder.children = folders.into_iter().chain(features).collect();
}

/// The nodes while they are built: children are indices, so a folder found by path can take a child at any time; the
/// tree is put together once, at the end.
// Rust-native: an arena in place of the TypeScript's `Map` of folder nodes that hold their children. Parity: the
// tree `build_tree` returns is the same (`builds_the_clean_project_tree`, `handles_two_examples_tables`).
struct Arena {
    nodes: Vec<Option<TestNode>>,
    kids: Vec<Vec<usize>>,
    folders: HashMap<String, usize>,
}

impl Arena {
    fn push(&mut self, n: TestNode) -> usize {
        self.nodes.push(Some(n));
        self.kids.push(Vec::new());
        self.nodes.len() - 1
    }

    fn node(&mut self, i: usize) -> &mut TestNode {
        self.nodes[i].as_mut().expect("a node not yet taken")
    }

    fn folder_for(&mut self, dir: &str) -> usize {
        if let Some(i) = self.folders.get(dir) {
            return *i;
        }
        let cut = dir.rfind('/');
        let parent = self.folder_for(cut.map_or("", |at| &dir[..at]));
        let name = &dir[cut.map_or(0, |at| at + 1)..];
        let i = self.push(node(NodeKind::Folder, format!("d:{dir}"), name.to_owned(), dir));
        self.kids[parent].push(i);
        self.folders.insert(dir.to_owned(), i);
        i
    }

    fn assemble(&mut self, i: usize) -> TestNode {
        let mut n = self.nodes[i].take().expect("a node taken once");
        let kids = std::mem::take(&mut self.kids[i]);
        n.children = kids.into_iter().map(|k| self.assemble(k)).collect();
        n
    }
}

fn string_of(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_owned()
}

/// What an `exampleData` holds, as `Object.entries` lists it: an object's entries, a list's by index.
fn example_entries(data: &Value) -> Vec<(String, &Value)> {
    match data {
        Value::Object(map) => js::entries(map).into_iter().map(|(k, v)| (k.clone(), v)).collect(),
        Value::Array(items) => items.iter().enumerate().map(|(i, v)| (i.to_string(), v)).collect(),
        _ => Vec::new(),
    }
}

/// Builds folder > feature > scenario | outline > row from the dry run's depth-0 events.
/// An outline node appears with its first row (the section number only exists on the rows);
/// an outline with no rows is dropped. A row's key number is its 1-based position in the
/// outline across all Examples tables, because refId examples restart per table.
/// Row lines are left `None`: the runner fills them from the feature file with `row_lines`.
/// A row's values come from its result's `exampleData` in the feature's `FEATURE_EXIT`; a value that
/// is not a string (a dynamic table's JSON) is kept as its JSON text.
/// Folders come first, then features, each sorted by path; scenarios keep file order.
pub fn build_tree(events: &[KarateEvent], root_name: &str) -> Result<TestNode, DataError> {
    let mut arena = Arena {
        nodes: Vec::new(),
        kids: Vec::new(),
        folders: HashMap::new(),
    };
    let root = arena.push(node(NodeKind::Folder, "d:".into(), root_name.to_owned(), ""));
    arena.folders.insert(String::new(), root);

    let mut feature: Option<usize> = None;
    let mut outline_event: Option<&KarateEvent> = None;
    let mut outline: Option<usize> = None;
    // by `{file}|{refId}`, in the order they were first seen; the values stay in `events` (Rust-native: borrowed, where
    // the TypeScript held the same objects; parity: `keeps_each_rows_examples_values_from_the_dry_run`)
    let mut examples: IndexMap<String, &Value> = IndexMap::new();
    for ev in events {
        let d = &ev.data;
        if !ev.at_depth_zero() {
            continue;
        }
        match ev.kind {
            EventKind::FeatureExit => {
                let relative_path = js::display(js::prop(d, "relativePath")?);
                for r in js::items(js::prop(d, "scenarioResults")?, "scenarioResults")? {
                    let example = js::prop(r, "exampleData")?;
                    if js::truthy(example) {
                        let at = format!("{relative_path}|{}", js::display(js::prop(r, "refId")?));
                        examples.insert(at, example);
                    }
                }
            }
            EventKind::FeatureEnter => {
                let Some(file) = js::prop(d, "path")?.as_str() else {
                    return Err(DataError::new("Karate event data: `path` is not text"));
                };
                let cut = file.rfind('/');
                let name = match js::prop(d, "name")? {
                    n if js::truthy(n) => js::display(n),
                    _ => file[cut.map_or(0, |at| at + 1)..].to_owned(),
                };
                let mut n = node(NodeKind::Feature, format!("f:{file}"), name, file);
                n.tags = js::strings(js::prop(d, "tags")?);
                let at = arena.push(n);
                let folder = arena.folder_for(cut.map_or("", |at| &file[..at]));
                arena.kids[folder].push(at);
                feature = Some(at);
                outline_event = None;
                outline = None;
            }
            EventKind::OutlineEnter => {
                outline_event = Some(ev);
                outline = None;
            }
            EventKind::ScenarioEnter => {
                let slug = js::display(js::prop(d, "slug")?);
                let Some(feature) = feature else {
                    return Err(DataError::new(format!("SCENARIO_ENTER before FEATURE_ENTER: {slug}")));
                };
                let ref_id = js::prop(d, "refId")?;
                let section = parse_ref_id(&js::display(ref_id))?.section;
                let file = arena.node(feature).file.clone();
                let line = js::as_u32(js::prop(d, "line")?);
                let tags = js::strings(js::prop(d, "tags")?);
                let name = string_of(js::prop(d, "name")?);
                if !js::truthy(js::prop(d, "isOutlineExample")?) {
                    let mut n = node(NodeKind::Scenario, format!("s:{file}#{section}"), name, &file);
                    n.line = line;
                    n.ref_id = Some(js::display(ref_id));
                    n.tags = tags;
                    let at = arena.push(n);
                    arena.kids[feature].push(at);
                    outline_event = None;
                    outline = None;
                    continue;
                }
                let Some(entered) = outline_event else {
                    return Err(DataError::new(format!("outline row without OUTLINE_ENTER: {slug}")));
                };
                // the first row makes the outline node; the rows after it, in any Examples table, join it
                let parent = match outline {
                    Some(at) => at,
                    None => {
                        let o = &entered.data;
                        let mut n = node(
                            NodeKind::Outline,
                            format!("o:{file}#{section}"),
                            string_of(js::prop(o, "name")?),
                            &file,
                        );
                        n.line = js::as_u32(js::prop(o, "line")?);
                        n.tags = js::strings(js::prop(o, "tags")?);
                        let at = arena.push(n);
                        arena.kids[feature].push(at);
                        outline = Some(at);
                        at
                    }
                };
                let rows = arena.kids[parent].len() + 1;
                let mut row = node(NodeKind::Row, format!("s:{file}#{section}.{rows}"), name, &file);
                row.ref_id = Some(js::display(ref_id));
                row.tags = tags;
                let at = arena.push(row);
                arena.kids[parent].push(at);
            }
            _ => {}
        }
    }
    let mut tree = arena.assemble(root);
    let index = leaf_paths(&tree);
    for (at, data) in examples {
        let Some(path) = index.get(&at) else { continue };
        let row = node_at(&mut tree, path);
        if row.kind == NodeKind::Row {
            row.example = Some(
                example_entries(data)
                    .into_iter()
                    .map(|(column, v)| {
                        let value = match v {
                            Value::String(s) => s.clone(),
                            other => js::json_stringify(other),
                        };
                        (column, value)
                    })
                    .collect(),
            );
        }
    }
    sort_folders(&mut tree);
    Ok(tree)
}

/// Child indices from `tree` down to each node that has a refId, by `{file}|{refId}`; a later node of the same id wins.
fn leaf_paths(tree: &TestNode) -> HashMap<String, Vec<usize>> {
    fn walk(n: &TestNode, path: &mut Vec<usize>, out: &mut HashMap<String, Vec<usize>>) {
        if let Some(ref_id) = &n.ref_id {
            out.insert(format!("{}|{ref_id}", n.file), path.clone());
        }
        for (i, child) in n.children.iter().enumerate() {
            path.push(i);
            walk(child, path, out);
            path.pop();
        }
    }
    let mut out = HashMap::new();
    walk(tree, &mut Vec::new(), &mut out);
    out
}

fn node_at<'a>(tree: &'a mut TestNode, path: &[usize]) -> &'a mut TestNode {
    path.iter().fold(tree, |n, i| &mut n.children[*i])
}

/// Line numbers (1-based) of the data rows of the Examples table whose `Examples:` keyword is on
/// `examples_line`. Blank and `#` lines are skipped; the first `|` line is the header; the table ends
/// at the first other line. A dynamic table (`| read('rows.json') |`) has no data rows.
pub fn row_lines(feature_text: &str, examples_line: u32) -> Vec<u32> {
    let lines = js::lines(feature_text);
    let mut rows = Vec::new();
    let mut header = false;
    for (i, line) in lines.iter().enumerate().skip(examples_line as usize) {
        let t = js::trim(line); // `line` is line i + 1
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if !t.starts_with('|') {
            break;
        }
        if header {
            rows.push(i as u32 + 1);
        }
        header = true;
    }
    rows
}

// Rust-native: the lookups below (`parent_of`, `find_node`, `leaf_index`, `resolve_node`, `resolve_tests`) return
// references into the tree, where the TypeScript returned the tree's own objects: nothing is copied. Parity: the same
// nodes come back (the tests compare their keys, names and lines).

/// The node whose child has this key.
pub fn parent_of<'a>(tree: &'a TestNode, key: &str) -> Option<&'a TestNode> {
    for child in &tree.children {
        if child.key == key {
            return Some(tree);
        }
        if let Some(hit) = parent_of(child, key) {
            return Some(hit);
        }
    }
    None
}

pub fn find_node<'a>(tree: &'a TestNode, key: &str) -> Option<&'a TestNode> {
    if tree.key == key {
        return Some(tree);
    }
    tree.children.iter().find_map(|child| find_node(child, key))
}

/// Scenario and row keys at or under `n`, in tree order.
pub fn leaf_keys(n: &TestNode) -> Vec<String> {
    if n.is_leaf() {
        vec![n.key.clone()]
    } else {
        n.children.iter().flat_map(leaf_keys).collect()
    }
}

/// Maps `{file}|{refId}` to the scenario or row node. refIds are the same in a dry run and the run after it.
pub fn leaf_index(tree: &TestNode) -> HashMap<String, &TestNode> {
    fn walk<'a>(n: &'a TestNode, index: &mut HashMap<String, &'a TestNode>) {
        if let Some(ref_id) = &n.ref_id {
            index.insert(format!("{}|{ref_id}", n.file), n);
        }
        for child in &n.children {
            walk(child, index);
        }
    }
    let mut index = HashMap::new();
    walk(tree, &mut index);
    index
}

/// What `resolve_node` needs of the node it looks for: a tree node, or a stored test.
// Rust-native: a struct for the TypeScript's `Pick<TestNode, "key" | "kind" | "file" | "name">`, with `From` for the
// two things callers hold. Parity: `resolve_node`'s three tests (a tree node) and `resolve_tests`' (stored tests).
#[derive(Clone, Copy, Debug)]
pub struct NodeRef<'a> {
    pub key: &'a str,
    pub kind: NodeKind,
    pub file: &'a str,
    pub name: &'a str,
}

impl<'a> From<&'a TestNode> for NodeRef<'a> {
    fn from(n: &'a TestNode) -> Self {
        Self {
            key: &n.key,
            kind: n.kind,
            file: &n.file,
            name: &n.name,
        }
    }
}

impl<'a> From<&'a StoredTest> for NodeRef<'a> {
    fn from(t: &'a StoredTest) -> Self {
        Self {
            key: &t.key,
            kind: match t.kind {
                LeafKind::Scenario => NodeKind::Scenario,
                LeafKind::Row => NodeKind::Row,
            },
            file: &t.file,
            name: &t.name,
        }
    }
}

/// Finds `old` again after a re-discovery: same key and name, else the one same-kind node of that name in the same
/// feature. Refuses to guess.
pub fn resolve_node<'a>(tree: &'a TestNode, old: NodeRef<'_>) -> Option<&'a TestNode> {
    if let Some(same) = find_node(tree, old.key).filter(|same| same.name == old.name) {
        return Some(same);
    }
    fn walk<'a>(n: &'a TestNode, old: NodeRef<'_>, matches: &mut Vec<&'a TestNode>) {
        if n.kind == old.kind && n.file == old.file && n.name == old.name {
            matches.push(n);
        }
        for child in &n.children {
            walk(child, old, matches);
        }
    }
    let mut matches = Vec::new();
    walk(tree, old, &mut matches);
    match matches.as_slice() {
        [only] => Some(only),
        _ => None,
    }
}

/// What `resolve_tests` found.
#[derive(Debug)]
pub struct ResolvedTests<'a> {
    /// Each test once, in tree order.
    pub found: Vec<&'a TestNode>,
    pub missing: usize,
    pub dynamic: usize,
}

/// Finds stored tests again in a re-discovered tree with `resolve_node`. `found` holds each test once,
/// in tree order. A test that is gone or ambiguous counts as `missing`; a found one with no line (a row
/// of a dynamic Examples table) cannot be targeted alone and counts as `dynamic`.
pub fn resolve_tests<'a>(tree: &'a TestNode, tests: &[StoredTest]) -> ResolvedTests<'a> {
    // Rust-native: a node found is remembered by its address in `tree` (the TypeScript's `Set` of objects held by
    // identity), so a test found twice is listed once. Parity: `finds_moved_and_same_key_tests_...` ("found once").
    let mut hits: HashSet<*const TestNode> = HashSet::new();
    let mut missing = 0;
    let mut dynamic = 0;
    for t in tests {
        match resolve_node(tree, t.into()) {
            None => missing += 1,
            Some(n) if n.line.is_none() => dynamic += 1,
            Some(n) => {
                hits.insert(n);
            }
        }
    }
    fn walk<'a>(n: &'a TestNode, hits: &HashSet<*const TestNode>, found: &mut Vec<&'a TestNode>) {
        if hits.contains(&std::ptr::from_ref(n)) {
            found.push(n);
        }
        for child in &n.children {
            walk(child, hits, found);
        }
    }
    let mut found = Vec::new();
    walk(tree, &hits, &mut found);
    ResolvedTests {
        found,
        missing,
        dynamic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::karate::events::read_all_events;
    use serde_json::json;
    use std::path::{Path, PathBuf};

    fn fixture(dir: &str, name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test/fixtures")
            .join(dir)
            .join(name)
    }

    fn events(name: &str) -> Vec<KarateEvent> {
        read_all_events(fixture("events", name)).unwrap()
    }

    fn clean() -> Vec<KarateEvent> {
        events("dryrun-clean.jsonl")
    }

    fn tree_of(evs: &[KarateEvent], root: &str) -> TestNode {
        build_tree(evs, root).unwrap()
    }

    /// A copy of the events with `f` applied to the data of every event of `file`.
    fn mutate(evs: &[KarateEvent], file: &str, f: impl Fn(&mut Value)) -> Vec<KarateEvent> {
        let mut copy = evs.to_vec();
        for ev in &mut copy {
            if ev.data["feature"] == file {
                f(&mut ev.data);
            }
        }
        copy
    }

    /// Shift section numbers and lines of basic.feature, as if scenarios were added above.
    fn shift_basic(evs: &[KarateEvent], sections: u64, lines: u64) -> Vec<KarateEvent> {
        mutate(evs, "tests/basic.feature", |d| {
            let r = parse_ref_id(d["refId"].as_str().unwrap()).unwrap();
            d["refId"] = json!(format!("[{}:{}]", r.section + sections, r.line + lines));
            d["line"] = json!(d["line"].as_u64().unwrap() + lines);
        })
    }

    fn example(pairs: &[(&str, &str)]) -> Option<IndexMap<String, String>> {
        Some(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
    }

    mod parse_ref_id {
        use super::*;

        #[test]
        fn parses_rows_and_scenarios() {
            assert_eq!(
                parse_ref_id("[1.3:7]").unwrap(),
                RefId {
                    section: 1,
                    example: Some(3),
                    line: 7
                }
            );
            assert_eq!(
                parse_ref_id("[3:22]").unwrap(),
                RefId {
                    section: 3,
                    example: None,
                    line: 22
                }
            );
        }

        // New in the port: the error message is the TypeScript's.
        #[test]
        fn refuses_anything_else_as_the_typescript_does() {
            for bad in ["", "1:7", "[1:7", "[a:7]", "[1.:7]", "[1:7]\n", "[\u{661}:7]"] {
                assert_eq!(
                    parse_ref_id(bad).unwrap_err().to_string(),
                    format!("invalid refId: {bad}")
                );
            }
        }
    }

    mod build_tree {
        use super::*;

        #[test]
        fn builds_the_clean_project_tree() {
            let root = tree_of(&clean(), "clean");
            assert_eq!(root.kind, NodeKind::Folder);
            assert_eq!(root.key, "d:");
            assert_eq!(root.file, "");
            assert_eq!(root.name, "clean");
            assert_eq!(
                root.children.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
                ["d:tests"]
            );
            assert_eq!(
                root.children[0]
                    .children
                    .iter()
                    .map(|c| c.key.as_str())
                    .collect::<Vec<_>>(),
                [
                    "f:tests/basic.feature",
                    "f:tests/outline.feature",
                    "f:tests/slow.feature"
                ]
            );
            assert_eq!(leaf_keys(&root).len(), 9);
            fn walk<'a>(n: &'a TestNode, files: &mut Vec<&'a str>) {
                files.push(&n.file);
                n.children.iter().for_each(|c| walk(c, files));
            }
            let mut files = Vec::new();
            walk(&root, &mut files);
            assert!(!files.iter().any(|f| f.starts_with("tests/common/")));
        }

        #[test]
        fn groups_outline_rows() {
            let tree = tree_of(&clean(), "clean");
            let outline = find_node(&tree, "o:tests/outline.feature#1").unwrap();
            assert_eq!(outline.kind, NodeKind::Outline);
            assert_eq!(outline.name, "add <a> and <b>");
            assert_eq!(outline.line, Some(3));
            assert_eq!(outline.ref_id, None);
            assert_eq!(outline.children.len(), 3);
            let row = outline.children.iter().find(|r| r.name == "add 2 and 3").unwrap();
            assert_eq!(row.key, "s:tests/outline.feature#1.2");
            assert_eq!(row.kind, NodeKind::Row);
            assert_eq!(row.line, None); // the runner reads row lines from the feature file
            assert_eq!(row.ref_id.as_deref(), Some("[1.2:7]"));
        }

        #[test]
        fn handles_two_examples_tables() {
            let tree = tree_of(&events("dryrun-outline-sizes.jsonl"), "extra");
            let feature = find_node(&tree, "f:extra/outline-sizes.feature").unwrap();
            assert_eq!(
                feature.children.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
                ["o:extra/outline-sizes.feature#1", "o:extra/outline-sizes.feature#2"]
            );
            let rows = &feature.children[1].children;
            assert_eq!(rows.len(), 7);
            assert_eq!(
                rows.iter().map(|r| r.ref_id.as_deref().unwrap()).collect::<Vec<_>>(),
                [
                    "[2.1:13]", "[2.2:13]", "[2.3:13]", "[2.4:13]", "[2.5:13]", "[2.1:22]", "[2.2:22]"
                ]
            );
            assert_eq!(rows[5].key, "s:extra/outline-sizes.feature#2.6");
            assert_eq!(rows[6].key, "s:extra/outline-sizes.feature#2.7");
            let keys = leaf_keys(&tree);
            assert_eq!(keys.len(), 8);
            assert_eq!(keys.iter().collect::<HashSet<_>>().len(), 8);
        }

        #[test]
        fn keeps_each_rows_examples_values_from_the_dry_run() {
            let tree = tree_of(&clean(), "clean");
            assert_eq!(
                find_node(&tree, "s:tests/outline.feature#1.2").unwrap().example,
                example(&[("a", "2"), ("b", "3"), ("expected", "5")])
            );
            assert_eq!(find_node(&tree, "o:tests/outline.feature#1").unwrap().example, None);
            let index = leaf_index(&tree);
            let scenario = index["tests/basic.feature|[1:7]"];
            assert_eq!(scenario.kind, NodeKind::Scenario);
            assert_eq!(scenario.example, None);
        }

        #[test]
        fn keeps_the_values_of_rows_in_two_examples_tables() {
            let tree = tree_of(&events("dryrun-outline-sizes.jsonl"), "extra");
            let rows = &find_node(&tree, "o:extra/outline-sizes.feature#2").unwrap().children;
            let expected: Vec<_> = ["1", "2", "3", "4", "5", "6", "7"]
                .iter()
                .map(|a| example(&[("a", a)]))
                .collect();
            assert_eq!(rows.iter().map(|r| r.example.clone()).collect::<Vec<_>>(), expected);
        }

        #[test]
        fn keeps_a_non_string_value_as_json_and_leaves_a_row_without_values_bare() {
            let mut evs = clean();
            let exit = evs
                .iter_mut()
                .find(|e| e.kind == EventKind::FeatureExit && e.data["relativePath"] == "tests/outline.feature")
                .unwrap();
            let results = exit.data["scenarioResults"].as_array_mut().unwrap();
            results[0]["exampleData"] = json!({ "a": 1, "b": { "x": 1 } });
            results[2].as_object_mut().unwrap().remove("exampleData");
            let tree = tree_of(&evs, "clean");
            let rows = &find_node(&tree, "o:tests/outline.feature#1").unwrap().children;
            assert_eq!(rows[0].example, example(&[("a", "1"), ("b", "{\"x\":1}")]));
            assert_eq!(rows[2].example, None);
        }

        #[test]
        fn keeps_scenario_tags() {
            let tree = tree_of(&clean(), "clean");
            let node = find_node(&tree, "s:tests/basic.feature#2").unwrap();
            assert_eq!(node.name, "failing match");
            assert_eq!(node.tags, ["wip"]);
            assert_eq!(node.line, Some(15));
            assert_eq!(node.ref_id.as_deref(), Some("[2:15]"));
        }

        // New in the port: a table's columns keep their order, and a JSON number is written as JavaScript writes it.
        #[test]
        fn keeps_the_columns_in_the_order_of_the_table_and_numbers_as_javascript_writes_them() {
            let mut evs = clean();
            let exit = evs
                .iter_mut()
                .find(|e| e.kind == EventKind::FeatureExit && e.data["relativePath"] == "tests/outline.feature")
                .unwrap();
            let results = exit.data["scenarioResults"].as_array_mut().unwrap();
            results[0]["exampleData"] =
                serde_json::from_str(r#"{"zeta":100.0,"alpha":"x","10":1.5,"2":null}"#).unwrap();
            let tree = tree_of(&evs, "clean");
            let row = find_node(&tree, "s:tests/outline.feature#1.1").unwrap();
            // `Object.entries`: the integer-like keys first, ascending
            assert_eq!(
                row.example
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>(),
                ["2=null", "10=1.5", "zeta=100", "alpha=x"]
            );
        }

        // New in the port: what the TypeScript threw as its own errors, with its messages.
        #[test]
        fn refuses_events_out_of_order_with_the_typescript_messages() {
            let enter = |kind: &str, extra: Value| {
                let mut data = json!({"callDepth": 0, "slug": "a:b", "refId": "[1:7]", "name": "n"});
                data.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
                KarateEvent {
                    kind: EventKind::from_name(kind),
                    time_stamp: 0.0,
                    thread_id: None,
                    data,
                }
            };
            let scenario = enter("SCENARIO_ENTER", json!({}));
            assert_eq!(
                build_tree(&[scenario], "x").unwrap_err().to_string(),
                "SCENARIO_ENTER before FEATURE_ENTER: a:b"
            );
            let feature = enter("FEATURE_ENTER", json!({"path": "a.feature"}));
            let row = enter("SCENARIO_ENTER", json!({"isOutlineExample": true}));
            assert_eq!(
                build_tree(&[feature.clone(), row], "x").unwrap_err().to_string(),
                "outline row without OUTLINE_ENTER: a:b"
            );
            let bad = enter("SCENARIO_ENTER", json!({"refId": "nope"}));
            assert_eq!(
                build_tree(&[feature, bad], "x").unwrap_err().to_string(),
                "invalid refId: nope"
            );
        }
    }

    mod row_lines {
        use super::*;

        fn text(file: &str) -> String {
            std::fs::read_to_string(fixture("projects", file)).unwrap()
        }

        #[test]
        fn takes_the_rows_after_the_header() {
            assert_eq!(row_lines(&text("clean/tests/outline.feature"), 7), [9, 10, 11]);
        }

        #[test]
        fn skips_comment_and_blank_lines_inside_the_table() {
            assert_eq!(row_lines(&text("edge/tests/rows.feature"), 7), [10, 12, 13]);
        }

        #[test]
        fn finds_no_rows_in_a_dynamic_examples_table() {
            assert!(row_lines(&text("edge/tests/rows.feature"), 18).is_empty());
        }

        #[test]
        fn reads_crlf_files_and_stops_at_the_next_keyword() {
            let crlf = ["Examples:", "| a |", "| 1 |", "Scenario: x", "| 2 |", ""].join("\r\n");
            assert_eq!(row_lines(&crlf, 1), [3]);
        }
    }

    mod leaf_index {
        use super::*;

        #[test]
        fn maps_file_ref_id_to_the_node() {
            let tree = tree_of(&clean(), "clean");
            let index = leaf_index(&tree);
            assert_eq!(index.len(), 9);
            assert_eq!(index["tests/outline.feature|[1.2:7]"].name, "add 2 and 3");
        }
    }

    mod resolve_node {
        use super::*;

        fn old_node() -> TestNode {
            let tree = tree_of(&clean(), "clean");
            find_node(&tree, "s:tests/basic.feature#2").unwrap().clone()
        }

        #[test]
        fn finds_a_scenario_that_moved() {
            let old = old_node();
            let moved = tree_of(&shift_basic(&clean(), 1, 5), "clean");
            assert_ne!(find_node(&moved, &old.key).unwrap().name, "failing match");
            let found = resolve_node(&moved, (&old).into()).unwrap();
            assert_eq!(found.name, "failing match");
            assert_eq!(found.key, "s:tests/basic.feature#3");
            assert_eq!(found.line, Some(20));
        }

        #[test]
        fn returns_null_when_the_name_is_ambiguous() {
            let dup = mutate(&shift_basic(&clean(), 2, 5), "tests/basic.feature", |d| {
                if d["name"] == "passing hello" {
                    d["name"] = json!("failing match");
                }
            });
            assert!(resolve_node(&tree_of(&dup, "clean"), (&old_node()).into()).is_none());
        }

        #[test]
        fn returns_null_when_the_scenario_is_gone() {
            let gone: Vec<KarateEvent> = clean()
                .into_iter()
                .filter(|e| e.data["name"] != "failing match")
                .collect();
            assert!(resolve_node(&tree_of(&gone, "clean"), (&old_node()).into()).is_none());
        }
    }

    mod resolve_tests {
        use super::*;
        use crate::model::history::TestStatusFinal;

        const F: &str = "tests/a.feature";

        fn leaf(kind: NodeKind, key: String, name: &str, line: Option<u32>, children: Vec<TestNode>) -> TestNode {
            TestNode {
                key,
                kind,
                name: name.into(),
                file: F.into(),
                line,
                ref_id: None,
                tags: vec![],
                example: None,
                children,
            }
        }

        fn tree() -> TestNode {
            let scenario = |n: u32, name: &str, line: u32| {
                leaf(NodeKind::Scenario, format!("s:{F}#{n}"), name, Some(line), vec![])
            };
            leaf(
                NodeKind::Folder,
                "d:".into(),
                "x",
                None,
                vec![leaf(
                    NodeKind::Feature,
                    format!("f:{F}"),
                    "a",
                    None,
                    vec![
                        scenario(1, "other", 3),
                        // was #1 before "other" was added above it
                        scenario(2, "moved", 8),
                        scenario(3, "stays", 14),
                        leaf(
                            NodeKind::Outline,
                            format!("o:{F}#4"),
                            "dyn <n>",
                            Some(20),
                            vec![leaf(NodeKind::Row, format!("s:{F}#4.1"), "dyn 1", None, vec![])],
                        ),
                        scenario(5, "twin", 30),
                        scenario(6, "twin", 36),
                    ],
                )],
            )
        }

        fn stored(kind: LeafKind, key: String, name: &str, line: Option<u32>) -> StoredTest {
            StoredTest {
                kind,
                file: F.into(),
                ref_id: "[1:1]".into(),
                name: name.into(),
                line,
                key,
                status: TestStatusFinal::Failed,
                duration_millis: None,
            }
        }

        #[test]
        fn finds_moved_and_same_key_tests_counts_missing_and_dynamic() {
            let scenario =
                |n: u32, name: &str, line: u32| stored(LeafKind::Scenario, format!("s:{F}#{n}"), name, Some(line));
            let tests = [
                scenario(3, "stays", 14), // same key
                scenario(9, "gone", 50),  // deleted
                // row of a dynamic Examples table
                stored(LeafKind::Row, format!("s:{F}#4.1"), "dyn 1", None),
                // Its key matches neither twin: a same-key hit would pass before the ambiguity check.
                scenario(7, "twin", 40),
                scenario(1, "moved", 3),  // key shifted, name kept
                scenario(3, "stays", 14), // again: found once
            ];
            let tree = tree();
            let resolved = resolve_tests(&tree, &tests);
            // tree order
            assert_eq!(
                resolved.found.iter().map(|n| n.key.clone()).collect::<Vec<_>>(),
                [format!("s:{F}#2"), format!("s:{F}#3")]
            );
            assert_eq!(resolved.missing, 2);
            assert_eq!(resolved.dynamic, 1);
        }
    }
}
