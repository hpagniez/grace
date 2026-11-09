//! `runner.trends.test.ts`.
use super::*;

fn hello() -> StoredTest {
    StoredTest {
        kind: LeafKind::Scenario,
        file: "tests/basic.feature".into(),
        ref_id: "[1:7]".into(),
        name: "passing hello".into(),
        line: Some(7),
        key: "s:tests/basic.feature#1".into(),
        status: TestStatusFinal::Passed,
        duration_millis: Some(400.0),
    }
}

#[test]
fn gives_the_trend_of_the_open_project() {
    let t = setup();
    let fx = fixture("clean");
    let ids = seed(&t.data_dir, &fx.root, 2, |r| r.tests = vec![hello()]);
    t.runner.open(&fx.root).unwrap();
    let all = t.runner.trend(None);
    // oldest first
    assert_eq!(all.iter().map(|p| p.id.clone()).collect::<Vec<_>>(), ids);
    assert_match(
        &serde_json::to_value(&all[0]).unwrap(),
        json!({ "passed": 1, "failed": 0, "total": leaves(Some(&t.tree())) }),
    );
    assert_match(
        &serde_json::to_value(t.runner.trend(Some("s:tests/basic.feature#1"))).unwrap(),
        json!([{ "total": 1 }, { "total": 1 }]),
    );
}

#[test]
fn gives_nothing_with_no_project() {
    assert!(setup().runner.trend(None).is_empty());
}
