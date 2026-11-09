//! `treeRows.test.ts`, `nodes.test.ts` and `virtual.test.ts`, test for test (`drawnRange`'s five are GPUI's
//! `uniform_list`'s now), and the turned rows of `Tree.tsx`.
use super::*;
use crate::model::tags::no_filter;

fn node(kind: NodeKind, key: &str, name: &str, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: name.into(),
        file: "tests/x.feature".into(),
        line: None,
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}

fn row(n: u32, name: &str) -> TestNode {
    let mut r = node(NodeKind::Row, &format!("s:tests/outline.feature#1.{n}"), name, vec![]);
    r.line = Some(5 + n);
    r
}

fn tree() -> TestNode {
    let mut passing = node(NodeKind::Scenario, "s:tests/basic.feature#1", "passing hello", vec![]);
    passing.line = Some(6);
    passing.tags = vec!["smoke".into()];
    let mut failing = node(NodeKind::Scenario, "s:tests/basic.feature#2", "failing match", vec![]);
    failing.line = Some(14);
    let mut basic = node(
        NodeKind::Feature,
        "f:tests/basic.feature",
        "basic API checks",
        vec![passing, failing],
    );
    basic.file = "tests/basic.feature".into();
    let mut outline = node(
        NodeKind::Feature,
        "f:tests/outline.feature",
        "outline with examples",
        vec![node(
            NodeKind::Outline,
            "o:tests/outline.feature#1",
            "add <a> and <b>",
            vec![row(1, "add 1 and 2"), row(2, "add 2 and 3")],
        )],
    );
    outline.file = "tests/outline.feature".into();
    node(
        NodeKind::Folder,
        "d:",
        "clean",
        vec![node(NodeKind::Folder, "d:tests", "tests", vec![basic, outline])],
    )
}

fn keys(rows: &[TreeRow]) -> Vec<&str> {
    rows.iter().map(|r| r.key.as_str()).collect()
}

fn smoke(hide: bool) -> FilterState {
    FilterState {
        include: vec!["smoke".into()],
        exclude: vec![],
        hide_non_matching: hide,
    }
}

mod tree_rows {
    use super::*;

    #[test]
    fn lists_every_node_but_the_root_in_tree_order_with_its_level_and_place_among_its_siblings() {
        let t = tree();
        let rows = tree_rows(&t, |_| true, &no_filter());
        assert_eq!(
            keys(&rows),
            [
                "d:tests",
                "f:tests/basic.feature",
                "s:tests/basic.feature#1",
                "s:tests/basic.feature#2",
                "f:tests/outline.feature",
                "o:tests/outline.feature#1",
                "s:tests/outline.feature#1.1",
                "s:tests/outline.feature#1.2",
            ]
        );
        assert_eq!(
            rows.iter().map(|r| r.level).collect::<Vec<_>>(),
            [1, 2, 3, 3, 2, 3, 4, 4]
        );
        assert_eq!(
            (rows[4].pos_in_set, rows[4].set_size, rows[4].parent_key.as_deref()),
            (2, 2, Some("d:tests"))
        );
        assert_eq!(
            (rows[0].pos_in_set, rows[0].set_size, rows[0].parent_key.as_deref()),
            (1, 1, None)
        );
        // and each row finds its node
        assert!(rows.iter().all(|r| r.node(&t).key == r.key));
    }

    #[test]
    fn leaves_out_the_children_of_a_closed_node_but_keeps_the_node() {
        let rows = tree_rows(&tree(), |k| k != "f:tests/basic.feature", &no_filter());
        assert_eq!(
            keys(&rows),
            [
                "d:tests",
                "f:tests/basic.feature",
                "f:tests/outline.feature",
                "o:tests/outline.feature#1",
                "s:tests/outline.feature#1.1",
                "s:tests/outline.feature#1.2",
            ]
        );
    }

    #[test]
    fn leaves_out_non_matching_rows_under_hide_non_matching_and_counts_only_the_shown_siblings() {
        let rows = tree_rows(&tree(), |_| true, &smoke(true));
        assert_eq!(
            keys(&rows),
            ["d:tests", "f:tests/basic.feature", "s:tests/basic.feature#1"]
        );
        assert_eq!((rows[1].pos_in_set, rows[1].set_size), (1, 1));
        assert_eq!((rows[2].level, rows[2].pos_in_set, rows[2].set_size), (3, 1, 1));
    }

    #[test]
    fn keeps_every_row_when_the_filter_only_dims() {
        assert_eq!(tree_rows(&tree(), |_| true, &smoke(false)).len(), 8);
    }
}

#[test]
fn row_name_joins_the_label_the_status_word_in_lower_case_the_flaky_words_and_the_tags() {
    assert_eq!(
        row_name("login.feature", Status::Failed, Some("2 flaky"), &["smoke".into()]),
        "login.feature, failed, 2 flaky, smoke"
    );
    assert_eq!(
        row_name("login works", Status::Failed, Some("flaky"), &[]),
        "login works, failed, flaky"
    );
    assert_eq!(row_name("x", Status::NotRun, None, &[]), "x, not run");
}

mod feature_of {
    use super::*;

    #[test]
    fn reads_the_feature_of_a_feature_scenario_example_row_or_outline_key() {
        assert_eq!(
            feature_of(Some("f:tests/a.feature")).as_deref(),
            Some("f:tests/a.feature")
        );
        assert_eq!(
            feature_of(Some("s:tests/a.feature#3")).as_deref(),
            Some("f:tests/a.feature")
        );
        assert_eq!(
            feature_of(Some("s:tests/a.feature#3.2")).as_deref(),
            Some("f:tests/a.feature")
        );
        assert_eq!(
            feature_of(Some("o:tests/a.feature#3")).as_deref(),
            Some("f:tests/a.feature")
        );
    }

    #[test]
    fn keeps_a_hash_in_the_path() {
        assert_eq!(
            feature_of(Some("s:tests/a#b/x.feature#2")).as_deref(),
            Some("f:tests/a#b/x.feature")
        );
    }

    #[test]
    fn gives_none_for_a_folder_the_root_or_no_key() {
        assert_eq!(feature_of(Some("d:tests")), None);
        assert_eq!(feature_of(Some("d:")), None);
        assert_eq!(feature_of(None), None);
    }
}

fn v(scroll_top: f32) -> View {
    View {
        scroll_top,
        height: 600.,
        row_height: 24.,
        pad: 0.,
    }
}

mod scroll_to_show {
    use super::*;

    #[test]
    fn scrolls_the_least_to_bring_a_row_into_view() {
        assert_eq!(scroll_to_show(50, v(0.)), 624.);
        assert_eq!(scroll_to_show(10, v(624.)), 240.);
        assert_eq!(scroll_to_show(0, View { pad: 8., ..v(100.) }), 0.);
    }

    #[test]
    fn does_not_scroll_when_the_row_is_in_view() {
        assert_eq!(scroll_to_show(30, v(624.)), 624.);
    }
}

mod page_rows {
    use super::*;

    #[test]
    fn counts_whole_rows_in_view_at_least_1() {
        assert_eq!(page_rows(v(0.)), 25);
        let small = |height| View {
            height,
            row_height: 28.,
            ..v(0.)
        };
        assert_eq!(page_rows(small(84.)), 3);
        assert_eq!(page_rows(small(20.)), 1);
    }

    #[test]
    fn measures_a_list_of_height_0_as_600_px() {
        assert_eq!(page_rows(View { height: 0., ..v(0.) }), 25);
    }
}

mod anchored_scroll_top {
    use super::*;

    fn k(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }
    const KEYS: [&str; 5] = ["a", "b", "c", "d", "e"];

    #[test]
    fn keeps_the_first_row_in_view_still_when_rows_are_added_above() {
        let after = k(&["x", "y", "a", "b", "c", "d", "e"]);
        assert_eq!(anchored_scroll_top(v(48.), &k(&KEYS), &after), 96.);
        assert_eq!(anchored_scroll_top(v(60.), &k(&KEYS), &after), 108.);
    }

    #[test]
    fn counts_the_padding() {
        let after = k(&["x", "y", "a", "b", "c", "d", "e"]);
        assert_eq!(anchored_scroll_top(View { pad: 8., ..v(56.) }, &k(&KEYS), &after), 104.);
    }

    #[test]
    fn leaves_a_list_at_the_top_at_the_top() {
        assert_eq!(
            anchored_scroll_top(v(0.), &k(&KEYS), &k(&["x", "a", "b", "c", "d", "e"])),
            0.
        );
    }

    #[test]
    fn keeps_the_scroll_top_when_the_anchor_is_gone() {
        assert_eq!(anchored_scroll_top(v(48.), &k(&KEYS), &k(&["a", "b"])), 48.);
        assert_eq!(anchored_scroll_top(v(4800.), &k(&KEYS), &k(&KEYS)), 4800.);
    }
}

mod survivor {
    use super::*;

    fn k(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn is_the_nearest_row_before_the_key_that_is_still_there() {
        assert_eq!(
            survivor("c", &k(&["a", "b", "c", "d"]), &k(&["a", "d"])).as_deref(),
            Some("a")
        );
        assert_eq!(
            survivor("c", &k(&["a", "b", "c", "d"]), &k(&["a", "b", "d"])).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn falls_back_to_the_first_row_then_to_none() {
        assert_eq!(survivor("a", &k(&["a", "b"]), &k(&["b"])).as_deref(), Some("b"));
        assert_eq!(survivor("a", &k(&["a"]), &[]), None);
    }
}

#[test]
fn a_feature_starts_closed_and_every_other_row_open_and_a_turn_flips_it() {
    let mut t = Turned::default();
    assert!(!t.is_open("f:a.feature"));
    assert!(t.is_open("d:tests") && t.is_open("o:a.feature#1"));
    t.set_open("f:a.feature", true);
    t.set_open("d:tests", false);
    assert!(t.is_open("f:a.feature") && !t.is_open("d:tests"));
    // back where it started, it is no longer held as turned
    t.set_open("f:a.feature", false);
    t.set_open("d:tests", true);
    assert_eq!(t, Turned::default());
}

#[test]
fn a_label_is_a_features_file_name_and_an_unnamed_scenarios_line() {
    let t = tree();
    let basic = &t.children[0].children[0];
    assert_eq!(label_of(basic), "basic.feature");
    assert_eq!(label_of(&basic.children[0]), "passing hello");
    let mut unnamed = basic.children[1].clone();
    unnamed.name = String::new();
    assert_eq!(label_of(&unnamed), "Scenario at line 14");
}

#[test]
fn ancestors_are_the_keys_above_from_the_root_down() {
    assert_eq!(
        ancestors(&tree(), "s:tests/outline.feature#1.2"),
        ["d:", "d:tests", "f:tests/outline.feature", "o:tests/outline.feature#1"]
    );
    assert!(ancestors(&tree(), "s:nowhere#1").is_empty());
}

#[test]
fn the_flaky_chip_counts_the_flaky_tests_under_a_row() {
    let t = tree();
    let mut flaky = HashMap::new();
    assert_eq!(flaky_chip(&t, &flaky), None);
    flaky.insert("s:tests/basic.feature#1".to_owned(), vec![]);
    flaky.insert("s:tests/outline.feature#1.1".to_owned(), vec![]);
    assert_eq!(flaky_chip(&t, &flaky).as_deref(), Some("2 flaky"));
    assert_eq!(
        flaky_chip(&t.children[0].children[0].children[0], &flaky).as_deref(),
        Some("flaky")
    );
}
