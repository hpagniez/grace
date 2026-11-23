//! `failures.test.ts`'s 10 tests, and the stepping and ordering the view reads.
use super::*;
use crate::model::node::NodeKind;
use crate::model::tags::no_filter;

fn test(key: &str, tags: &[&str]) -> TestNode {
    TestNode {
        key: key.into(),
        kind: NodeKind::Scenario,
        name: key.into(),
        file: "x.feature".into(),
        line: Some(1),
        ref_id: Some(format!("[1:{key}]")),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        example: None,
        children: vec![],
    }
}

fn tests() -> Vec<TestNode> {
    vec![
        test("a", &["fast"]),
        test("b", &["fast"]),
        test("c", &[]),
        test("d", &[]),
    ]
}

fn root() -> TestNode {
    TestNode {
        key: "d:".into(),
        kind: NodeKind::Folder,
        name: "p".into(),
        file: String::new(),
        line: None,
        ref_id: None,
        tags: vec![],
        example: None,
        children: tests(),
    }
}

/// A failed result whose error is `head` then the reason, as Karate's SCENARIO_EXIT gives it.
fn failed(reason: &str, head: &str) -> TestResult {
    TestResult {
        duration_millis: None,
        error: Some(format!("{head}{reason}")),
        reason_at: Some(head.encode_utf16().count()),
    }
}

fn groups_of(results: &[(&str, TestResult)]) -> Vec<Vec<String>> {
    let all = tests();
    let refs: Vec<&TestNode> = all.iter().collect();
    let statuses = results.iter().map(|(k, _)| (k.to_string(), Status::Failed)).collect();
    let results = results.iter().map(|(k, r)| (k.to_string(), r.clone())).collect();
    failure_groups(&refs, &statuses, &results)
        .into_iter()
        .map(|g| g.keys)
        .collect()
}

mod reason_of {
    use super::*;

    #[test]
    fn is_the_error_from_where_karates_reason_starts_or_the_whole_error() {
        let with = TestResult {
            error: Some("C:/p/x.feature:5\nThen status 200\nboom".into()),
            reason_at: Some(33),
            ..Default::default()
        };
        assert_eq!(reason_of(Some(&with)), Some("boom"));
        let whole = TestResult {
            error: Some("boom".into()),
            ..Default::default()
        };
        assert_eq!(reason_of(Some(&whole)), Some("boom"));
        assert_eq!(reason_of(Some(&TestResult::default())), None);
        assert_eq!(reason_of(None), None);
    }

    #[test]
    fn counts_utf16_units_as_javascript_does() {
        // "é" is 1 unit and 2 bytes; "😀" is 2 units and 4 bytes
        let r = TestResult {
            error: Some("é😀boom".into()),
            reason_at: Some(3),
            ..Default::default()
        };
        assert_eq!(reason_of(Some(&r)), Some("boom"));
    }
}

mod failure_groups {
    use super::*;

    #[test]
    fn groups_failures_at_different_places_with_one_message() {
        let got = groups_of(&[
            (
                "a",
                failed(
                    "expected status: 200, actual: 503",
                    "C:/p/x.feature:5\nThen status 200\n",
                ),
            ),
            (
                "b",
                failed(
                    "expected status: 200, actual: 503",
                    "C:/p/y.feature:9\n# why\nThen status 200\n",
                ),
            ),
            (
                "c",
                failed(
                    "expected status: 200, actual: 404",
                    "C:/p/y.feature:12\nThen status 200\n",
                ),
            ),
        ]);
        assert_eq!(got, [["a", "b"]]);
    }

    #[test]
    fn leaves_out_a_failed_test_with_no_error_and_a_test_that_did_not_fail() {
        let all = tests();
        let refs: Vec<&TestNode> = all.iter().collect();
        let statuses = HashMap::from([
            ("a".to_string(), Status::Failed),
            ("b".to_string(), Status::Failed),
            ("c".to_string(), Status::Passed),
        ]);
        let results = HashMap::from([
            ("a".to_string(), failed("boom", "")),
            ("b".to_string(), TestResult::default()),
            ("c".to_string(), failed("boom", "")),
        ]);
        assert!(failure_groups(&refs, &statuses, &results).is_empty());
    }

    #[test]
    fn groups_karate_fail_at_different_lines() {
        let at = |line: u32| {
            format!(
                "js failed:\n==========\n  Line: {line}, Col: 0\n  Code: karate.fail('no vpn')\n  Error: no vpn\n=========="
            )
        };
        assert_eq!(
            groups_of(&[("a", failed(&at(10), "")), ("b", failed(&at(7), ""))]),
            [["a", "b"]]
        );
    }

    #[test]
    fn groups_a_thrown_string_at_different_positions() {
        assert_eq!(
            groups_of(&[
                ("a", failed("bad\n    at 15:1", "")),
                ("b", failed("bad\n    at 3:5", ""))
            ]),
            [["a", "b"]]
        );
    }

    #[test]
    fn does_not_merge_errors_that_differ_in_a_value() {
        assert!(groups_of(&[("a", failed("actual: 503", "")), ("b", failed("actual: 504", ""))]).is_empty());
    }

    #[test]
    fn orders_groups_by_their_first_test_each_in_tree_order() {
        let got = groups_of(&[
            ("d", failed("two", "")),
            ("c", failed("one", "")),
            ("b", failed("two", "")),
            ("a", failed("one", "")),
        ]);
        assert_eq!(got, [["a", "c"], ["b", "d"]]);
    }
}

mod shown_leaves {
    use super::*;

    #[test]
    fn keeps_only_the_matching_tests_under_hide_non_matching() {
        let fast = FilterState {
            include: vec!["fast".into()],
            ..no_filter()
        };
        let tree = root();
        let keys = |f: &FilterState| shown_leaves(&tree, f).iter().map(|t| t.key.clone()).collect::<Vec<_>>();
        assert_eq!(
            keys(&FilterState {
                hide_non_matching: true,
                ..fast.clone()
            }),
            ["a", "b"]
        );
        assert_eq!(keys(&fast), ["a", "b", "c", "d"]);
        // hiding with no tag in force hides nothing
        assert_eq!(
            keys(&FilterState {
                hide_non_matching: true,
                ..no_filter()
            })
            .len(),
            4
        );
    }
}

mod stepping {
    use super::*;

    fn group(keys: &[&str]) -> FailureGroup {
        FailureGroup {
            keys: keys.iter().map(|k| k.to_string()).collect(),
        }
    }

    #[test]
    fn finds_a_tests_group_and_its_place() {
        let groups = [group(&["a", "c"]), group(&["b", "d"])];
        let (g, i) = group_of(&groups, "d").unwrap();
        assert_eq!((g.keys.len(), i), (2, 1));
        assert!(group_of(&groups, "x").is_none());
    }

    #[test]
    fn wraps_round_both_ways() {
        let g = group(&["a", "b", "c"]);
        assert_eq!(step(&g, 0, 1), "b");
        assert_eq!(step(&g, 2, 1), "a");
        assert_eq!(step(&g, 0, -1), "c");
        assert_eq!(step(&g, 1, -1), "a");
    }

    #[test]
    fn lists_the_largest_group_first_and_keeps_tree_order_among_equals() {
        let got = largest_first(vec![group(&["a", "b"]), group(&["c", "d", "e"]), group(&["f", "g"])]);
        let firsts: Vec<_> = got.iter().map(|g| g.keys[0].as_str()).collect();
        assert_eq!(firsts, ["c", "a", "f"]);
    }
}

mod config_failure {
    use super::*;

    const REASON: &str = "Config evaluation failed: karate-config.js - js failed:\n==========\n  Line: 4, Col: 2\n  Code: karate.callSingle('classpath:auth.feature', { baseUrl: baseUrl });\n  Error: callSingle failed\n==========";

    fn run(failed_keys: &[&str], results: &[(&str, &str, Option<usize>)]) -> Option<ConfigFailure> {
        let all = tests();
        let refs: Vec<&TestNode> = all.iter().collect();
        let keys: Vec<String> = failed_keys.iter().map(|k| k.to_string()).collect();
        let results = results
            .iter()
            .map(|(k, e, at)| {
                (
                    k.to_string(),
                    TestResult {
                        error: Some(e.to_string()),
                        reason_at: *at,
                        ..Default::default()
                    },
                )
            })
            .collect();
        config_failure(&refs, &keys, &results)
    }

    #[test]
    fn names_the_file_its_line_the_first_reason_and_the_count() {
        assert_eq!(
            run(&["b", "a"], &[("a", REASON, Some(0)), ("b", REASON, None)]),
            Some(ConfigFailure {
                file: "karate-config.js".into(),
                line: 4,
                reason: REASON.into(),
                count: 2
            })
        );
    }

    #[test]
    fn drops_classpath_and_opens_line_1_when_the_message_has_none() {
        let dev = "Config evaluation failed: classpath:karate-config-dev.js - boom";
        assert_eq!(
            run(&["c"], &[("c", dev, None)]),
            Some(ConfigFailure {
                file: "karate-config-dev.js".into(),
                line: 1,
                reason: dev.into(),
                count: 1
            })
        );
    }

    // as `/Line: (\d+)/` reads it in failures.ts
    #[test]
    fn reads_the_first_line_that_is_a_number() {
        let reason = "Config evaluation failed: karate-config.js - Line: n/a\n  Line: 4, Col: 2";
        assert_eq!(run(&["a"], &[("a", reason, None)]).unwrap().line, 4);
    }

    #[test]
    fn counts_only_the_given_failed_keys_and_is_none_without_a_config_reason() {
        let results = [("a", REASON, None), ("b", REASON, None), ("c", "boom", None)];
        assert_eq!(run(&["a", "c"], &results).unwrap().count, 1);
        assert_eq!(run(&["c"], &results), None);
    }
}
