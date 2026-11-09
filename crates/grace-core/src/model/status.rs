//! `src/shared/status.ts`: a parent's status from its tests'.
use super::node::{Status, TestNode};
use std::collections::HashMap;

/// Highest precedence first; a parent shows the first of these that any child has.
const PRECEDENCE: [Status; 5] = [
    Status::Failed,
    Status::Running,
    Status::Interrupted,
    Status::Pending,
    Status::NotRun,
];

/// The status of a node: a test's own (`NotRun` when it has none), a folder's or feature's from its children's.
pub fn derive_status(node: &TestNode, statuses: &HashMap<String, Status>) -> Status {
    if node.is_leaf() {
        return statuses.get(&node.key).copied().unwrap_or(Status::NotRun);
    }
    if node.children.is_empty() {
        return Status::NotRun;
    }
    // Differs from the TypeScript's set and `find`, with the same result: the position in `PRECEDENCE` of the best
    // status any child has, found in one pass without collecting the children's statuses (a status with no place in
    // the list, `Passed`, never wins, and a parent of only passed tests is passed).
    node.children
        .iter()
        .filter_map(|c| {
            let s = derive_status(c, statuses);
            PRECEDENCE.iter().position(|p| *p == s)
        })
        .min()
        .map_or(Status::Passed, |i| PRECEDENCE[i])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::node::NodeKind;

    fn leaf(key: &str) -> TestNode {
        TestNode {
            key: key.into(),
            kind: NodeKind::Scenario,
            name: key.into(),
            file: "a.feature".into(),
            line: Some(1),
            ref_id: None,
            tags: vec![],
            example: None,
            children: vec![],
        }
    }

    fn feature(children: Vec<TestNode>) -> TestNode {
        TestNode {
            key: "f:a.feature".into(),
            kind: NodeKind::Feature,
            name: "a".into(),
            file: "a.feature".into(),
            line: None,
            ref_id: None,
            tags: vec![],
            example: None,
            children,
        }
    }

    fn derive(statuses: &[Status]) -> Status {
        let kids = (0..statuses.len()).map(|i| leaf(&format!("s{i}"))).collect();
        let by_key = statuses
            .iter()
            .enumerate()
            .map(|(i, s)| (format!("s{i}"), *s))
            .collect();
        derive_status(&feature(kids), &by_key)
    }

    mod derive_status {
        use super::*;
        use Status::*;

        #[test]
        fn failed_wins_over_passed_and_running() {
            assert_eq!(derive(&[Passed, Failed]), Failed);
            assert_eq!(derive(&[Running, Failed]), Failed);
        }

        #[test]
        fn running_wins_over_passed() {
            assert_eq!(derive(&[Passed, Running]), Running);
        }

        #[test]
        fn all_passed_is_passed() {
            assert_eq!(derive(&[Passed, Passed]), Passed);
        }

        #[test]
        fn not_run_when_any_child_is_not_run_and_none_failed_or_running() {
            assert_eq!(derive(&[Passed, NotRun]), NotRun);
        }

        #[test]
        fn interrupted_when_no_child_failed() {
            assert_eq!(derive(&[Passed, Interrupted]), Interrupted);
        }

        #[test]
        fn pending_when_a_child_waits() {
            assert_eq!(derive(&[Passed, Pending]), Pending);
        }

        #[test]
        fn leaf_returns_its_own_status_default_not_run() {
            let own = HashMap::from([("x".to_string(), Passed)]);
            assert_eq!(derive_status(&leaf("x"), &own), Passed);
            assert_eq!(derive_status(&leaf("x"), &HashMap::new()), NotRun);
        }

        #[test]
        fn empty_folder_is_not_run() {
            assert_eq!(derive_status(&feature(vec![]), &HashMap::new()), NotRun);
        }

        #[test]
        fn recurses_through_nested_nodes() {
            let outline = TestNode {
                key: "o:x".into(),
                ..feature(vec![leaf("a"), leaf("b")])
            };
            let tree = feature(vec![outline]);
            let statuses = HashMap::from([("a".to_string(), Passed), ("b".to_string(), Failed)]);
            assert_eq!(derive_status(&tree, &statuses), Failed);
        }
    }

    /// Not in the TypeScript: the precedence list is the whole rule, so check each step of it against the next.
    #[test]
    fn precedence_is_failed_running_interrupted_pending_not_run_passed() {
        use Status::*;
        let order = [Failed, Running, Interrupted, Pending, NotRun, Passed];
        for (i, high) in order.iter().enumerate() {
            for low in &order[i + 1..] {
                assert_eq!(derive(&[*low, *high]), *high);
                assert_eq!(derive(&[*high, *low]), *high);
            }
        }
    }
}
