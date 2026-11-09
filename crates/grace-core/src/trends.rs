//! `src/main/trends.ts`: trends (#93): pass rate and time per test of a tree selection over the stored runs
//! (docs/specs/2026-11-04-trends.md).
//!
//! Where it differs from a line-for-line port (plan P1, Redesigns): the resolver's index keeps, per identity, the one
//! node or "more than one" and not the list of nodes the TypeScript builds to look only at its length, and its keys are
//! borrowed triples, not `JSON.stringify` strings.
use crate::history::ListedRun;
use crate::karate::tree::{NodeRef, find_node, leaf_keys};
use crate::model::history::{StoredTest, TestStatusFinal};
use crate::model::node::{NodeKind, TestNode};
use crate::model::trends::TrendPoint;
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
}
