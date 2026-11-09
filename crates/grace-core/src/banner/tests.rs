//! `bannerText.test.ts`, test for test.
use super::*;
use crate::model::events::{Project, RunOutcome};
use crate::model::node::Layout;
use crate::model::tags::FilterState;

fn node(kind: NodeKind, key: &str, children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: key.into(),
        file: key.into(),
        line: (!matches!(kind, NodeKind::Folder | NodeKind::Feature)).then_some(1),
        ref_id: None,
        tags: vec![],
        example: None,
        children,
    }
}
fn scenario(key: &str) -> TestNode {
    node(NodeKind::Scenario, key, vec![])
}
fn feature(key: &str, children: Vec<TestNode>) -> TestNode {
    node(NodeKind::Feature, key, children)
}
fn root(features: Vec<TestNode>) -> TestNode {
    node(NodeKind::Folder, "d:", features)
}

fn opened(tree: Option<TestNode>) -> AppState {
    AppState {
        project: Some(Project {
            root: "D:\\work\\orders-api".into(),
            layout: Layout::Standalone,
        }),
        tree,
        ..AppState::default()
    }
}

fn three_in_two() -> TestNode {
    root(vec![
        feature("f:a", vec![scenario("s:a1"), scenario("s:a2")]),
        feature("f:b", vec![scenario("s:b1")]),
    ])
}

mod tests_text {
    use super::*;

    #[test]
    fn has_no_tests_line_with_no_project() {
        assert_eq!(tests_text(&AppState::default()), None);
    }

    #[test]
    fn counts_scenarios_and_features() {
        assert_eq!(
            tests_text(&opened(Some(three_in_two()))).unwrap(),
            "3 scenarios in 2 features"
        );
    }

    #[test]
    fn uses_the_singular_for_one_of_each() {
        let s = opened(Some(root(vec![feature("f:a", vec![scenario("s:a1")])])));
        assert_eq!(tests_text(&s).unwrap(), "1 scenario in 1 feature");
    }

    #[test]
    fn counts_outline_rows_not_outlines() {
        let outline = node(
            NodeKind::Outline,
            "o:a",
            vec![
                node(NodeKind::Row, "s:a.1", vec![]),
                node(NodeKind::Row, "s:a.2", vec![]),
            ],
        );
        let s = opened(Some(root(vec![feature("f:a", vec![scenario("s:a0"), outline])])));
        assert_eq!(tests_text(&s).unwrap(), "3 scenarios in 1 feature");
    }

    #[test]
    fn counts_ignore_the_tag_filter() {
        let mut s = opened(Some(three_in_two()));
        s.filter = FilterState {
            include: vec!["smoke".into()],
            exclude: vec![],
            hide_non_matching: true,
        };
        assert_eq!(tests_text(&s).unwrap(), "3 scenarios in 2 features");
    }

    #[test]
    fn says_when_there_are_no_tests() {
        let s = opened(Some(root(vec![feature("f:a", vec![])])));
        assert_eq!(tests_text(&s).unwrap(), "No feature files found");
    }

    #[test]
    fn says_it_is_finding_tests_while_busy_with_no_tree() {
        let mut s = opened(None);
        assert_eq!(tests_text(&s), None);
        s.busy = true;
        assert_eq!(tests_text(&s).unwrap(), "finding tests…");
    }

    #[test]
    fn points_at_the_tree_after_a_discovery_error() {
        let mut s = opened(None);
        s.error = Some("boom".into());
        assert_eq!(
            tests_text(&s).unwrap(),
            "could not be listed: see the message in the tree"
        );
    }
}

mod banner_state {
    use super::*;

    #[test]
    fn asks_for_a_project_with_none_open() {
        assert_eq!(banner_state(&AppState::default()), Some(BannerState::None));
    }

    #[test]
    fn is_ready_with_a_project_and_nothing_running() {
        assert_eq!(banner_state(&opened(Some(three_in_two()))), Some(BannerState::Ready));
    }

    #[test]
    fn is_finding_while_busy_and_no_run_is_active() {
        let idle = AppState {
            busy: true,
            ..AppState::default()
        };
        assert_eq!(banner_state(&idle), Some(BannerState::Finding));
        let mut s = opened(Some(three_in_two()));
        s.busy = true;
        assert_eq!(banner_state(&s), Some(BannerState::Finding));
    }

    #[test]
    fn is_starting_once_a_run_is_active_and_karate_has_not_printed() {
        let mut s = opened(Some(three_in_two()));
        s.busy = true;
        s.run.active = true;
        assert_eq!(banner_state(&s), Some(BannerState::Starting));
    }

    #[test]
    fn goes_once_the_console_has_output_in_every_state() {
        let mut busy = opened(Some(three_in_two()));
        busy.busy = true;
        let mut running = busy.clone();
        running.run.active = true;
        for mut s in [AppState::default(), opened(Some(three_in_two())), busy, running] {
            s.console.push_back("x".into());
            assert_eq!(banner_state(&s), None);
        }
    }

    #[test]
    fn has_no_state_line_when_run_all_has_nothing_to_run() {
        let mut broken = opened(None);
        broken.error = Some("boom".into());
        assert_eq!(banner_state(&broken), None);
        let mut broken_tree = opened(Some(three_in_two()));
        broken_tree.error = Some("boom".into());
        assert_eq!(banner_state(&broken_tree), None);
        assert_eq!(banner_state(&opened(Some(root(vec![feature("f:a", vec![])])))), None);
        let mut filtered = opened(Some(three_in_two()));
        filtered.filter = FilterState {
            include: vec!["nothing-has-this".into()],
            exclude: vec![],
            hide_non_matching: false,
        };
        assert_eq!(banner_state(&filtered), None);
    }

    #[test]
    fn is_ready_again_after_a_run_that_printed_nothing() {
        let mut s = opened(Some(three_in_two()));
        s.run.outcome = Some(RunOutcome::Cancelled);
        assert_eq!(banner_state(&s), Some(BannerState::Ready));
    }
}
