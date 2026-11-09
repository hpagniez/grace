//! `src/main/flaky.ts`: flaky tests (F15): tests that passed and failed in runs of the same project files and
//! environment (docs/specs/2026-10-26-flaky-tests.md).
//!
//! Where it differs from a line-for-line port (plan P1, Redesigns): the TypeScript keys its maps on
//! `JSON.stringify([...])` strings, built again for every stored test of every run; here a group is the pair
//! (fingerprint, environment) and a test's identity the triple (file, kind, name), borrowed from the records and
//! hashed as they are. The groups, tests and environments come out in the order they were first seen, as a `Map`'s do
//! (`IndexMap`), so the list is in the TypeScript's order.
use crate::history::ListedRun;
use crate::karate::tree::{NodeRef, find_node, resolve_node};
use crate::model::flaky::{FlakyEnv, FlakyRun, FlakyStatus, FlakyTest};
use crate::model::history::{StoredTest, TestStatusFinal};
use crate::model::node::{NodeKind, TestNode};
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::collections::HashSet;

/// What a test is called across runs: its file, kind and name (the TypeScript's `identity`). Its key is left out: it
/// is what tells apart two tests that share an identity.
type Identity<'a> = (&'a str, NodeKind, &'a str);

fn identity(t: &StoredTest) -> Identity<'_> {
    let n = NodeRef::from(t);
    (n.file, n.kind, n.name)
}

/// The proof of one stored test (by key) in one group: the first stored one, that of the newest run, and every run
/// that stored it `passed` or `failed`.
struct Proof<'a> {
    test: &'a StoredTest,
    runs: Vec<FlakyRun>,
}

/// The tests of `tree` that are flaky in `runs` (newest first), by their key in `tree`. Runs are compared when their
/// fingerprint and environment are equal; thread count, filter and outcome play no part. A flaky test that
/// `resolve_node` cannot find in `tree` is left out. So is one whose key holds no node of its name in `tree` while a
/// run of its group stored two tests of that name: the name could be the other one's. A node gets the proof of one
/// stored test per group.
pub fn find_flaky(runs: &[ListedRun], tree: Option<&TestNode>) -> Vec<FlakyTest> {
    let Some(tree) = tree else {
        return Vec::new();
    };
    let mut groups: IndexMap<(&str, Option<&str>), Vec<&ListedRun>> = IndexMap::new();
    for r in runs {
        // no fingerprint, no proof; a debug run is left out (step debugger R2.7)
        let Some(fingerprint) = r.record.fingerprint.as_deref() else {
            continue;
        };
        if r.record.debug {
            continue;
        }
        groups
            .entry((fingerprint, r.record.env.as_deref()))
            .or_default()
            .push(r);
    }
    // By key in `tree`, then environment.
    let mut found: IndexMap<&str, IndexMap<Option<&str>, FlakyEnv>> = IndexMap::new();
    for group in groups.values() {
        let env = group[0].record.env.as_deref();
        // Identities that one run of the group stored twice.
        let mut twice: HashSet<Identity<'_>> = HashSet::new();
        for r in group {
            let mut seen: HashSet<Identity<'_>> = HashSet::new();
            for t in &r.record.tests {
                if !seen.insert(identity(t)) {
                    twice.insert(identity(t));
                }
            }
        }
        let mut placed: HashSet<&str> = HashSet::new();
        let mut proofs: IndexMap<&str, Proof<'_>> = IndexMap::new();
        for r in group {
            for test in &r.record.tests {
                let status = match test.status {
                    TestStatusFinal::Passed => FlakyStatus::Passed,
                    TestStatusFinal::Failed => FlakyStatus::Failed,
                    TestStatusFinal::NotRun | TestStatusFinal::Interrupted => continue,
                };
                proofs
                    .entry(test.key.as_str())
                    .or_insert_with(|| Proof { test, runs: Vec::new() })
                    .runs
                    .push(FlakyRun {
                        id: r.id.clone(),
                        started_at: r.record.started_at,
                        status,
                        threads: r.record.threads,
                        report_available: r.report_available,
                    });
            }
        }
        for Proof { test, runs: proof } in proofs.into_values() {
            let passed = proof.iter().filter(|r| r.status == FlakyStatus::Passed).count();
            if passed == 0 || passed == proof.len() {
                continue;
            }
            if find_node(tree, &test.key).map(|n| n.name.as_str()) != Some(test.name.as_str())
                && twice.contains(&identity(test))
            {
                continue;
            }
            let Some(node) = resolve_node(tree, test.into()) else {
                continue;
            };
            if !placed.insert(node.key.as_str()) {
                continue;
            }
            let merged = found
                .entry(node.key.as_str())
                .or_default()
                .entry(env)
                .or_insert_with(|| FlakyEnv {
                    env: env.map(String::from),
                    passed: 0,
                    failed: 0,
                    runs: Vec::new(),
                });
            merged.passed += passed as u32;
            merged.failed += (proof.len() - passed) as u32;
            merged.runs.extend(proof);
        }
    }
    found
        .into_iter()
        .map(|(key, envs)| FlakyTest {
            key: key.to_string(),
            envs: envs
                .into_values()
                .map(|mut e| {
                    // newest first; finite times, so `partial_cmp` never fails (and `sort_by` is stable, as `sort` is)
                    e.runs
                        .sort_by(|a, b| b.started_at.partial_cmp(&a.started_at).unwrap_or(Ordering::Equal));
                    e
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::fixtures::{default_tree, listed, one, stored, tree, two, with};
    use crate::model::events::RunOutcome;
    use crate::model::history::LeafKind;
    use crate::model::tags::FilterState;
    use TestStatusFinal::{Failed, Interrupted, NotRun, Passed};

    fn a() -> String {
        "a".repeat(64)
    }

    fn b() -> String {
        "b".repeat(64)
    }

    /// A run that started at `i`, with `one` as given, in the project files `a`.
    fn run(i: u32, status: TestStatusFinal) -> ListedRun {
        with(listed(i, vec![one(status)]), |r| r.record.fingerprint = Some(a()))
    }

    fn keys(flaky: &[FlakyTest]) -> Vec<&str> {
        flaky.iter().map(|f| f.key.as_str()).collect()
    }

    fn run_ids(runs: &[FlakyRun]) -> Vec<&str> {
        runs.iter().map(|r| r.id.as_str()).collect()
    }

    fn flaky_run(id: &str, started_at: f64, status: FlakyStatus, threads: u32, report_available: bool) -> FlakyRun {
        FlakyRun {
            id: id.into(),
            started_at,
            status,
            threads: Some(threads),
            report_available,
        }
    }

    mod find_flaky {
        use super::*;

        #[test]
        fn a_pass_and_a_fail_with_the_same_files_and_environment_make_a_test_flaky() {
            let runs = [
                with(run(2, Failed), |r| {
                    r.record.threads = Some(4);
                    r.report_available = true;
                }),
                run(1, Passed),
            ];
            assert_eq!(
                find_flaky(&runs, Some(&default_tree())),
                [FlakyTest {
                    key: "s:tests/a.feature#1".into(),
                    envs: vec![FlakyEnv {
                        env: None,
                        passed: 1,
                        failed: 1,
                        runs: vec![
                            flaky_run("run-2", 2.0, FlakyStatus::Failed, 4, true),
                            flaky_run("run-1", 1.0, FlakyStatus::Passed, 1, false),
                        ],
                    }],
                }]
            );
        }

        #[test]
        fn two_passes_are_not_flaky() {
            assert!(find_flaky(&[run(2, Passed), run(1, Passed)], Some(&default_tree())).is_empty());
        }

        #[test]
        fn a_null_fingerprint_proves_nothing() {
            let none = |r: ListedRun| with(r, |r| r.record.fingerprint = None);
            let runs = [none(run(2, Failed)), none(run(1, Passed))];
            assert!(find_flaky(&runs, Some(&default_tree())).is_empty());
            let runs = [none(run(2, Failed)), run(1, Passed)];
            assert!(find_flaky(&runs, Some(&default_tree())).is_empty());
        }

        #[test]
        fn a_debug_run_proves_nothing_a_long_pause_can_fail_a_test() {
            let runs = [with(run(2, Failed), |r| r.record.debug = true), run(1, Passed)];
            assert!(find_flaky(&runs, Some(&default_tree())).is_empty());
        }

        #[test]
        fn different_fingerprints_are_not_compared() {
            let runs = [
                with(run(2, Failed), |r| r.record.fingerprint = Some(b())),
                run(1, Passed),
            ];
            assert!(find_flaky(&runs, Some(&default_tree())).is_empty());
        }

        #[test]
        fn different_environments_are_not_compared() {
            let runs = [
                with(run(2, Failed), |r| r.record.env = Some("qa".into())),
                run(1, Passed),
            ];
            assert!(find_flaky(&runs, Some(&default_tree())).is_empty());
        }

        #[test]
        fn the_same_pair_in_two_environments_gives_two_envs() {
            let qa = |r: ListedRun| with(r, |r| r.record.env = Some("qa".into()));
            let runs = [run(4, Failed), run(3, Passed), qa(run(2, Failed)), qa(run(1, Passed))];
            let flaky = find_flaky(&runs, Some(&default_tree()));
            assert_eq!(flaky.len(), 1);
            let envs: Vec<(Option<&str>, u32, u32, usize)> = flaky[0]
                .envs
                .iter()
                .map(|e| (e.env.as_deref(), e.passed, e.failed, e.runs.len()))
                .collect();
            assert_eq!(envs, [(None, 1, 1, 2), (Some("qa"), 1, 1, 2)]);
        }

        #[test]
        fn threads_filter_and_outcome_play_no_part() {
            let runs = [
                with(run(2, Failed), |r| {
                    r.record.threads = Some(8);
                    r.record.outcome = RunOutcome::Cancelled;
                    r.record.exit_code = None;
                }),
                with(run(1, Passed), |r| {
                    r.record.filter = FilterState {
                        include: vec!["smoke".into()],
                        exclude: Vec::new(),
                        hide_non_matching: false,
                    };
                }),
            ];
            assert_eq!(keys(&find_flaky(&runs, Some(&default_tree()))), ["s:tests/a.feature#1"]);
        }

        #[test]
        fn not_run_and_interrupted_do_not_count() {
            let runs = [
                with(run(4, Interrupted), |r| {
                    r.record.tests = vec![one(Interrupted), two(Passed)]
                }),
                with(run(3, NotRun), |r| r.record.tests = vec![one(NotRun), two(NotRun)]),
                with(run(2, Failed), |r| r.record.tests = vec![one(Failed), two(Interrupted)]),
                with(run(1, Passed), |r| r.record.tests = vec![one(Passed), two(NotRun)]),
            ];
            let flaky = find_flaky(&runs, Some(&default_tree()));
            assert_eq!(flaky.len(), 1);
            assert_eq!(flaky[0].key, "s:tests/a.feature#1");
            assert_eq!(run_ids(&flaky[0].envs[0].runs), ["run-2", "run-1"]);
        }

        #[test]
        fn proofs_of_two_fingerprints_of_one_environment_merge() {
            let runs = [
                with(run(4, Failed), |r| r.record.fingerprint = Some(b())),
                run(3, Failed),
                with(run(2, Passed), |r| r.record.fingerprint = Some(b())),
                run(1, Passed),
            ];
            let flaky = find_flaky(&runs, Some(&default_tree()));
            assert_eq!(flaky.len(), 1);
            assert_eq!(flaky[0].envs.len(), 1);
            let env = &flaky[0].envs[0];
            assert_eq!((env.env.as_deref(), env.passed, env.failed), (None, 2, 2));
            assert_eq!(run_ids(&env.runs), ["run-4", "run-3", "run-2", "run-1"]);
        }

        /// Not in the TypeScript, whose `Map`s keep the order things were first seen: the list is in the order the
        /// proofs were first seen, in the newest run's tests first.
        #[test]
        fn lists_tests_in_the_order_their_proofs_were_first_seen() {
            let both = |status| vec![two(status), one(status)];
            let runs = [
                with(run(2, Failed), |r| r.record.tests = both(Failed)),
                with(run(1, Passed), |r| r.record.tests = both(Passed)),
            ];
            let flaky = find_flaky(&runs, Some(&default_tree()));
            assert_eq!(keys(&flaky), ["s:tests/a.feature#2", "s:tests/a.feature#1"]);
        }

        mod placed_by_resolve_node {
            use super::*;

            fn flaky_one() -> [ListedRun; 2] {
                [run(2, Failed), run(1, Passed)]
            }

            #[test]
            fn a_test_that_moved_gets_its_key_in_the_tree() {
                let moved = tree(&[("added", 1), ("one", 2)]);
                assert_eq!(keys(&find_flaky(&flaky_one(), Some(&moved))), ["s:tests/a.feature#2"]);
            }

            #[test]
            fn an_ambiguous_name_gives_nothing() {
                let twice = tree(&[("added", 1), ("one", 2), ("one", 4)]);
                assert!(find_flaky(&flaky_one(), Some(&twice)).is_empty());
            }

            #[test]
            fn a_vanished_test_gives_nothing() {
                assert!(find_flaky(&flaky_one(), Some(&tree(&[("two", 2)]))).is_empty());
            }

            // two scenarios named "x": #1 stable, #2 flaky
            fn x(n: &str, status: TestStatusFinal) -> StoredTest {
                stored(LeafKind::Scenario, "x", n, status)
            }

            fn twins() -> [ListedRun; 2] {
                [
                    with(run(2, Failed), |r| {
                        r.record.tests = vec![x("1", Passed), x("2", Failed)]
                    }),
                    with(run(1, Passed), |r| {
                        r.record.tests = vec![x("1", Passed), x("2", Passed)]
                    }),
                ]
            }

            #[test]
            fn same_named_scenarios_are_told_apart_by_their_keys() {
                let both = tree(&[("x", 1), ("x", 2)]);
                assert_eq!(keys(&find_flaky(&twins(), Some(&both))), ["s:tests/a.feature#2"]);
            }

            #[test]
            fn a_name_the_groups_runs_stored_twice_is_not_placed_by_name() {
                // #2 was renamed: its key now holds "y", and "x" is unique in the tree, but it is #1
                let renamed = tree(&[("x", 1), ("y", 2)]);
                assert!(find_flaky(&twins(), Some(&renamed)).is_empty());
            }

            #[test]
            fn both_twins_flipped_list_each_run_once_when_one_is_gone() {
                let runs = [
                    with(run(4, Failed), |r| {
                        r.record.tests = vec![x("1", Failed), x("2", Failed)]
                    }),
                    with(run(3, Passed), |r| {
                        r.record.tests = vec![x("1", Passed), x("2", Passed)]
                    }),
                ];
                let flaky = find_flaky(&runs, Some(&tree(&[("x", 1)])));
                assert_eq!(flaky.len(), 1);
                assert_eq!(flaky[0].key, "s:tests/a.feature#1");
                assert_eq!(flaky[0].envs.len(), 1);
                assert_eq!((flaky[0].envs[0].passed, flaky[0].envs[0].failed), (1, 1));
                assert_eq!(run_ids(&flaky[0].envs[0].runs), ["run-4", "run-3"]);
            }

            #[test]
            fn a_node_is_placed_from_one_stored_key_of_a_group_only() {
                // no run stored both twins, so only the second rule keeps #2's proof off the one "x" left
                let runs = [
                    with(run(4, Failed), |r| r.record.tests = vec![x("1", Failed)]),
                    with(run(3, Passed), |r| r.record.tests = vec![x("1", Passed)]),
                    with(run(2, Failed), |r| r.record.tests = vec![x("2", Failed)]),
                    with(run(1, Passed), |r| r.record.tests = vec![x("2", Passed)]),
                ];
                let flaky = find_flaky(&runs, Some(&tree(&[("x", 1)])));
                assert_eq!(flaky.len(), 1);
                assert_eq!((flaky[0].envs[0].passed, flaky[0].envs[0].failed), (1, 1));
                assert_eq!(run_ids(&flaky[0].envs[0].runs), ["run-4", "run-3"]);
            }
        }

        #[test]
        fn an_outline_row_is_marked_by_its_row_key() {
            let row = |status| stored(LeafKind::Row, "rows 2", "3.2", status);
            let runs = [
                with(run(2, Failed), |r| r.record.tests = vec![row(Failed)]),
                with(run(1, Passed), |r| r.record.tests = vec![row(Passed)]),
            ];
            assert_eq!(
                keys(&find_flaky(&runs, Some(&default_tree()))),
                ["s:tests/a.feature#3.2"]
            );
        }

        #[test]
        fn no_tree_no_flaky() {
            assert!(find_flaky(&[run(2, Failed), run(1, Passed)], None).is_empty());
        }
    }
}
