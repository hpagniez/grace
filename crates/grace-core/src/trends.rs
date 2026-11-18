//! `src/main/trends.ts`: trends (#93): pass rate and time per test of a tree selection over the stored runs
//! (docs/specs/2026-11-04-trends.md).
//!
//! Where it differs from a line-for-line port (plan P1, Redesigns): the resolver's index keeps, per identity, the one
//! node or "more than one" and not the list of nodes the TypeScript builds to look only at its length, and its keys are
//! borrowed triples, not `JSON.stringify` strings.
use crate::history::ListedRun;
use crate::karate::tree::{NodeRef, find_node, leaf_keys};
use crate::model::history::{StoredTest, TestStatusFinal, when_label};
use crate::model::node::{NodeKind, TestNode};
use crate::model::trends::TrendPoint;
use crate::rows::{format_millis, label_of};
use std::collections::{HashMap, HashSet};

/// What a test is called across runs: its file, kind and name.
type Identity<'a> = (&'a str, NodeKind, &'a str);

/// `resolve_node`'s rule through one index of `tree`: the node of the same key and name, else the one node of the same
/// file, kind and name. `resolve_node` walks the tree per call; this is called per stored test.
struct Resolver<'a> {
    by_key: HashMap<&'a str, &'a TestNode>,
    /// `None`: more than one node has the identity, so none is the one.
    by_identity: HashMap<Identity<'a>, Option<&'a TestNode>>,
}

impl<'a> Resolver<'a> {
    fn new(tree: &'a TestNode) -> Self {
        let mut resolver = Self {
            by_key: HashMap::new(),
            by_identity: HashMap::new(),
        };
        resolver.walk(tree);
        resolver
    }

    fn walk(&mut self, n: &'a TestNode) {
        self.by_key.insert(&n.key, n);
        self.by_identity
            .entry((&n.file, n.kind, &n.name))
            .and_modify(|same| *same = None)
            .or_insert(Some(n));
        for child in &n.children {
            self.walk(child);
        }
    }

    fn resolve(&self, t: &StoredTest) -> Option<&'a TestNode> {
        let old = NodeRef::from(t);
        if let Some(same) = self.by_key.get(old.key).filter(|same| same.name == old.name) {
            return Some(same);
        }
        self.by_identity.get(&(old.file, old.kind, old.name)).copied().flatten()
    }
}

/// The trend of the node `key` (the whole tree when None) in `runs` (newest first), oldest first. A run counts when
/// it ran on `env` and is no debug run; a test counts when it was stored `passed` or `failed` and resolves to a test
/// under the node. A run with no counted test has no point.
pub fn find_trend(
    runs: &[ListedRun],
    tree: Option<&TestNode>,
    key: Option<&str>,
    env: Option<&str>,
) -> Vec<TrendPoint> {
    let Some(tree) = tree else {
        return Vec::new();
    };
    let Some(selected) = (match key {
        None => Some(tree),
        Some(key) => find_node(tree, key),
    }) else {
        return Vec::new();
    };
    let keys = leaf_keys(selected);
    let under: HashSet<&str> = keys.iter().map(String::as_str).collect();
    let resolver = Resolver::new(tree);
    let mut points = Vec::new();
    for ListedRun { id, record, .. } in runs {
        if record.debug || record.env.as_deref() != env {
            continue;
        }
        let (mut passed, mut failed, mut millis, mut timed) = (0u32, 0u32, 0.0, 0u32);
        for t in &record.tests {
            if !matches!(t.status, TestStatusFinal::Passed | TestStatusFinal::Failed) {
                continue;
            }
            let Some(n) = resolver.resolve(t) else {
                continue;
            };
            if !under.contains(n.key.as_str()) {
                continue;
            }
            if t.status == TestStatusFinal::Passed {
                passed += 1;
            } else {
                failed += 1;
            }
            if let Some(d) = t.duration_millis {
                millis += d;
                timed += 1;
            }
        }
        if passed + failed == 0 {
            continue;
        }
        points.push(TrendPoint {
            id: id.clone(),
            started_at: record.started_at,
            passed,
            failed,
            total: under.len() as u32,
            mean_millis: (timed > 0).then(|| millis / f64::from(timed)),
        });
    }
    points.reverse();
    points
}

/// What the strip says when no stored run ran the selection.
pub const NONE: &str = "No stored run ran these tests";
/// What the strip covers with nothing selected.
const ALL: &str = "All tests";
/// px, the height of each of a strip's two rows (`h-6`).
pub const ROW: f32 = 24.;
/// px, the gap between the passed and the failed part of a pass bar (`gap-0.5`).
pub const GAP: f32 = 2.;
/// px, the least a non-zero time bar is (`min-h-0.5`).
pub const MIN_TIME: f32 = 2.;

/// "92%", rounded down so one failure never reads 100%; "Passed" or "Failed" for one test.
pub fn pass_rate_label(p: &TrendPoint) -> String {
    if p.total == 1 {
        return if p.passed > 0 { "Passed" } else { "Failed" }.into();
    }
    format!("{}%", p.passed * 100 / (p.passed + p.failed))
}

/// "Today 14:32: 12 passed, 3 failed, 15 of 412 tests, 1.40 s per test"; for one test "Today 14:32: passed, 1.40 s".
pub fn trend_tip(p: &TrendPoint, now: f64) -> String {
    let one = p.total == 1;
    let mut parts = if one {
        vec![if p.passed > 0 { "passed" } else { "failed" }.to_owned()]
    } else {
        vec![
            format!("{} passed", p.passed),
            format!("{} failed", p.failed),
            format!("{} of {} tests", p.passed + p.failed, p.total),
        ]
    };
    if let Some(ms) = p.mean_millis {
        parts.push(if one {
            format_millis(ms)
        } else {
            format!("{} per test", format_millis(ms))
        });
    }
    format!("{}: {}", when_label(p.started_at, now), parts.join(", "))
}

/// What the strip covers: "All tests" for the whole tree, else the selected node's tree label.
pub fn trend_caption(tree: Option<&TestNode>, selected: Option<&str>) -> String {
    match (tree, selected) {
        (Some(tree), Some(key)) => find_node(tree, key)
            .filter(|n| n.key != tree.key)
            .map_or_else(|| ALL.into(), label_of),
        _ => ALL.into(),
    }
}

/// One run's column: the heights in px of its bars, what it is called and which run it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// The run folder's name.
    pub id: String,
    /// Also its accessible name.
    pub tip: String,
    /// The failed part of the pass bar, above; 0 draws nothing.
    pub failed: f32,
    /// The passed part, below.
    pub passed: f32,
    /// The time bar; `None` for a run whose counted tests have no time.
    pub time: Option<f32>,
}

/// The strip, worked out: the columns oldest to newest and the words beside them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Strip {
    pub columns: Vec<Column>,
    /// The newest column's pass rate, "" with none.
    pub pass: String,
    /// The newest column's time per test.
    pub time: String,
    /// The top of the time rows' scale: the tallest column's time, "" when there is none above 0.
    pub max: String,
}

/// The columns of `points` (oldest first, as `find_trend` gives them), each part of a bar its share of the row.
pub fn strip(points: &[TrendPoint], now: f64) -> Strip {
    let max = points.iter().filter_map(|p| p.mean_millis).fold(0., f64::max);
    let columns = points
        .iter()
        .map(|p| {
            let counted = p.passed + p.failed;
            let room = if p.passed > 0 && p.failed > 0 { ROW - GAP } else { ROW };
            let failed = room * p.failed as f32 / counted as f32;
            Column {
                id: p.id.clone(),
                tip: trend_tip(p, now),
                failed,
                passed: room - failed,
                time: p.mean_millis.map(|ms| match ms {
                    ms if ms > 0. => (ROW * (ms / max) as f32).max(MIN_TIME),
                    _ => 0.,
                }),
            }
        })
        .collect();
    let newest = points.last();
    Strip {
        columns,
        pass: newest.map(pass_rate_label).unwrap_or_default(),
        time: newest
            .and_then(|p| p.mean_millis)
            .map(format_millis)
            .unwrap_or_default(),
        max: if max > 0. { format_millis(max) } else { String::new() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::fixtures::{default_tree, listed, one, stored, tree, two, with};
    use crate::karate::tree::resolve_node;
    use crate::model::history::LeafKind;
    use TestStatusFinal::{Failed, Interrupted, NotRun, Passed};

    /// `stored` with the time the TypeScript's optional last argument gives.
    fn timed(mut t: StoredTest, ms: f64) -> StoredTest {
        t.duration_millis = Some(ms);
        t
    }

    fn point(id: &str, started_at: f64, passed: u32, failed: u32, total: u32) -> TrendPoint {
        TrendPoint {
            id: id.into(),
            started_at,
            passed,
            failed,
            total,
            mean_millis: None,
        }
    }

    fn ids(points: &[TrendPoint]) -> Vec<&str> {
        points.iter().map(|p| p.id.as_str()).collect()
    }

    mod find_trend {
        use super::*;

        #[test]
        fn covers_the_whole_tree_oldest_first() {
            // list_runs gives them newest first
            let runs = [
                listed(2, vec![one(Failed), two(Failed)]),
                listed(1, vec![one(Passed), two(Failed)]),
            ];
            assert_eq!(
                find_trend(&runs, Some(&default_tree()), None, None),
                [point("run-1", 1.0, 1, 1, 4), point("run-2", 2.0, 0, 2, 4)]
            );
        }

        #[test]
        fn counts_only_the_tests_of_the_selection() {
            let runs = [listed(
                1,
                vec![one(Passed), two(Failed), stored(LeafKind::Row, "rows 1", "3.1", Passed)],
            )];
            let tree = default_tree();
            let counts = |key: &str| {
                let points = find_trend(&runs, Some(&tree), Some(key), None);
                assert_eq!(points.len(), 1);
                (points[0].passed, points[0].failed, points[0].total)
            };
            assert_eq!(counts("s:tests/a.feature#1"), (1, 0, 1));
            assert_eq!(counts("f:tests/a.feature"), (2, 1, 4));
            assert_eq!(counts("o:tests/a.feature#3"), (1, 0, 2));
        }

        #[test]
        fn gives_no_point_to_a_run_with_no_counted_test() {
            let runs = [listed(1, vec![two(Passed)])];
            assert!(find_trend(&runs, Some(&default_tree()), Some("s:tests/a.feature#1"), None).is_empty());
        }

        #[test]
        fn does_not_count_not_run_and_interrupted_tests() {
            let runs = [listed(1, vec![one(NotRun), two(Interrupted)])];
            assert!(find_trend(&runs, Some(&default_tree()), None, None).is_empty());
        }

        #[test]
        fn leaves_out_a_debug_run() {
            let runs = [
                with(listed(2, vec![one(Failed)]), |r| r.record.debug = true),
                listed(1, vec![one(Passed)]),
            ];
            assert_eq!(ids(&find_trend(&runs, Some(&default_tree()), None, None)), ["run-1"]);
        }

        #[test]
        fn counts_only_the_runs_of_the_environment() {
            let runs = [
                with(listed(2, vec![one(Passed)]), |r| r.record.env = Some("qa".into())),
                with(listed(1, vec![one(Passed)]), |r| r.record.env = Some("dev".into())),
            ];
            let tree = default_tree();
            assert_eq!(ids(&find_trend(&runs, Some(&tree), None, Some("dev"))), ["run-1"]);
            assert!(find_trend(&runs, Some(&tree), None, None).is_empty());
        }

        #[test]
        fn gives_the_mean_time_of_the_counted_tests_that_have_one() {
            let tree = default_tree();
            let at = |tests: Vec<StoredTest>| find_trend(&[listed(1, tests)], Some(&tree), None, None)[0].mean_millis;
            assert_eq!(
                at(vec![timed(one(Passed), 1000.0), timed(two(Failed), 2000.0)]),
                Some(1500.0)
            );
            assert_eq!(at(vec![one(Passed), timed(two(Failed), 300.0)]), Some(300.0));
            assert_eq!(at(vec![one(Passed), two(Failed)]), None);
            // a test that does not count brings no time
            assert_eq!(
                at(vec![timed(one(Passed), 100.0), timed(two(NotRun), 900.0)]),
                Some(100.0)
            );
        }

        #[test]
        fn finds_a_test_whose_key_moved_by_its_name() {
            let moved = tree(&[("two", 2), ("one", 5)]);
            let runs = [listed(1, vec![one(Passed)])];
            let points = find_trend(&runs, Some(&moved), Some("s:tests/a.feature#5"), None);
            assert_eq!(points.len(), 1);
            assert_eq!((points[0].passed, points[0].total), (1, 1));
        }

        #[test]
        fn does_not_count_a_test_whose_name_is_ambiguous() {
            let twice = tree(&[("one", 4), ("one", 5)]);
            let runs = [listed(1, vec![one(Passed)])];
            assert!(find_trend(&runs, Some(&twice), None, None).is_empty());
        }

        #[test]
        fn gives_nothing_for_an_unknown_key_or_no_tree() {
            let runs = [listed(1, vec![one(Passed)])];
            assert!(find_trend(&runs, Some(&default_tree()), Some("s:nope#1"), None).is_empty());
            assert!(find_trend(&runs, None, None, None).is_empty());
        }

        /// Not in the TypeScript, where a comment says the resolver is `resolveNode`'s rule through an index: here
        /// the two are made to agree on every stored test of these trees, found or not.
        #[test]
        fn resolves_as_resolve_node_does() {
            let trees = [
                default_tree(),
                tree(&[("two", 2), ("one", 5)]),
                tree(&[("one", 4), ("one", 5)]),
                tree(&[("added", 1), ("one", 2), ("one", 4)]),
                tree(&[("x", 1), ("one", 2)]),
                tree(&[]),
            ];
            let tests = [
                one(Passed),
                two(Passed),
                stored(LeafKind::Scenario, "x", "1", Passed),
                stored(LeafKind::Scenario, "x", "9", Passed),
                stored(LeafKind::Row, "rows 1", "3.1", Passed),
                stored(LeafKind::Row, "rows 2", "3.9", Passed),
                stored(LeafKind::Row, "one", "1", Passed),
            ];
            for tree in &trees {
                let resolver = Resolver::new(tree);
                for t in &tests {
                    let by_index = resolver.resolve(t).map(|n| &n.key);
                    let by_walk = resolve_node(tree, t.into()).map(|n| &n.key);
                    assert_eq!(by_index, by_walk, "{} {} in {:?}", t.key, t.name, tree.children.len());
                }
            }
        }
    }

    /// `trendLabels.test.ts`, test for test (7), and the columns `Trends.tsx` made of boxes, worked out.
    mod labels {
        use super::*;
        use crate::history::fixtures::node;
        use chrono::{Local, TimeZone as _};

        fn local(h: u32, m: u32) -> f64 {
            Local.with_ymd_and_hms(2026, 11, 4, h, m, 0).unwrap().timestamp_millis() as f64
        }

        fn at() -> f64 {
            local(14, 32)
        }

        fn now() -> f64 {
            local(18, 0)
        }

        fn p(passed: u32, failed: u32, total: u32, mean_millis: Option<f64>) -> TrendPoint {
            TrendPoint {
                id: "20261104-143200-000".into(),
                started_at: at(),
                passed,
                failed,
                total,
                mean_millis,
            }
        }

        fn base() -> TrendPoint {
            p(12, 3, 412, Some(1400.))
        }

        #[test]
        fn pass_rate_gives_a_percentage_rounded_down() {
            assert_eq!(pass_rate_label(&base()), "80%");
            assert_eq!(pass_rate_label(&p(299, 1, 412, None)), "99%");
            assert_eq!(pass_rate_label(&p(4, 0, 412, None)), "100%");
        }

        #[test]
        fn pass_rate_words_one_test_as_passed_or_failed() {
            assert_eq!(pass_rate_label(&p(1, 0, 1, None)), "Passed");
            assert_eq!(pass_rate_label(&p(0, 1, 1, None)), "Failed");
        }

        #[test]
        fn tip_names_the_run_its_counts_and_its_time_per_test() {
            assert_eq!(
                trend_tip(&base(), now()),
                "Today 14:32: 12 passed, 3 failed, 15 of 412 tests, 1.40 s per test"
            );
            assert!(trend_tip(&p(12, 3, 412, Some(120.)), now()).ends_with(", 120 ms per test"));
        }

        #[test]
        fn tip_words_one_test_by_its_status_and_time() {
            assert_eq!(
                trend_tip(&p(1, 0, 1, Some(1400.)), now()),
                "Today 14:32: passed, 1.40 s"
            );
        }

        #[test]
        fn tip_leaves_out_a_time_it_does_not_have() {
            assert_eq!(
                trend_tip(&p(12, 3, 412, None), now()),
                "Today 14:32: 12 passed, 3 failed, 15 of 412 tests"
            );
        }

        fn users_tree() -> TestNode {
            let with_file = |n: TestNode| TestNode {
                file: "tests/users.feature".into(),
                ..n
            };
            let scenario = with_file(node(
                NodeKind::Scenario,
                "s:tests/users.feature#1",
                "creates a user",
                Vec::new(),
            ));
            let feature = with_file(node(
                NodeKind::Feature,
                "f:tests/users.feature",
                "Users",
                vec![scenario],
            ));
            TestNode {
                file: String::new(),
                ..node(NodeKind::Folder, "d:", "project", vec![feature])
            }
        }

        #[test]
        fn caption_says_all_tests_for_the_whole_tree() {
            let tree = users_tree();
            assert_eq!(trend_caption(Some(&tree), None), "All tests");
            assert_eq!(trend_caption(Some(&tree), Some("d:")), "All tests");
            assert_eq!(trend_caption(Some(&tree), Some("s:gone#1")), "All tests");
            assert_eq!(trend_caption(None, None), "All tests");
        }

        #[test]
        fn caption_names_a_node_as_the_tree_does() {
            let tree = users_tree();
            assert_eq!(
                trend_caption(Some(&tree), Some("f:tests/users.feature")),
                "users.feature"
            );
            assert_eq!(
                trend_caption(Some(&tree), Some("s:tests/users.feature#1")),
                "creates a user"
            );
        }

        /// TR-10: passed below, failed above, a 2 px gap between them in a bar 24 px tall.
        #[test]
        fn the_pass_bar_splits_passed_below_and_failed_above_with_a_gap() {
            let s = strip(&[p(3, 1, 4, None)], now());
            let c = &s.columns[0];
            assert_eq!((c.passed, c.failed), (16.5, 5.5));
            assert_eq!(c.passed + c.failed + GAP, ROW, "the gap makes up the rest of the row");
        }

        /// A share of 0 draws nothing (R3.1), and TR-11: a single test is all passed or all failed.
        #[test]
        fn a_share_of_nothing_draws_nothing_and_one_test_fills_the_bar() {
            let s = strip(&[p(1, 0, 1, None), p(0, 1, 1, None)], now());
            assert_eq!((s.columns[0].passed, s.columns[0].failed), (ROW, 0.));
            assert_eq!((s.columns[1].passed, s.columns[1].failed), (0., ROW));
        }

        /// TR-13: the tallest column fills its row, the others scale to it, a run with no time has no bar, and a
        /// non-zero time is at least 2 px.
        #[test]
        fn the_time_bars_scale_to_the_tallest_and_the_top_of_the_scale_is_said() {
            let s = strip(
                &[
                    p(3, 1, 4, Some(500.)),
                    p(4, 0, 4, Some(2000.)),
                    p(4, 0, 4, None),
                    p(4, 0, 4, Some(1.)),
                    p(4, 0, 4, Some(0.)),
                ],
                now(),
            );
            let times: Vec<_> = s.columns.iter().map(|c| c.time).collect();
            assert_eq!(times, [Some(6.), Some(ROW), None, Some(MIN_TIME), Some(0.)]);
            assert_eq!(s.max, "2.00 s");
        }

        #[test]
        fn a_strip_of_zero_times_has_no_top_of_scale() {
            let s = strip(&[p(1, 0, 1, Some(0.))], now());
            assert_eq!((s.columns[0].time, s.max.as_str()), (Some(0.), ""));
        }

        /// TR-12, TR-13: the labels are the newest column's.
        #[test]
        fn the_labels_are_the_newest_columns() {
            let s = strip(&[p(12, 3, 412, Some(1000.)), p(299, 1, 412, Some(1500.))], now());
            assert_eq!((s.pass.as_str(), s.time.as_str()), ("99%", "1.50 s"));
            let none = strip(&[], now());
            assert_eq!(
                (none.pass.as_str(), none.time.as_str(), none.max.as_str()),
                ("", "", "")
            );
            assert!(none.columns.is_empty());
        }

        /// TR-14: a column's name is its tooltip, and it keeps its run.
        #[test]
        fn a_column_is_named_by_its_run() {
            let s = strip(&[base()], now());
            assert_eq!(s.columns[0].id, "20261104-143200-000");
            assert_eq!(
                s.columns[0].tip,
                "Today 14:32: 12 passed, 3 failed, 15 of 412 tests, 1.40 s per test"
            );
        }
    }
}
