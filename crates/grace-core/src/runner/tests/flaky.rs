//! `runner.flaky.test.ts`.
use super::*;

fn hello(status: TestStatusFinal) -> StoredTest {
    StoredTest {
        kind: LeafKind::Scenario,
        file: "tests/basic.feature".into(),
        ref_id: "[1:7]".into(),
        name: "passing hello".into(),
        line: Some(7),
        key: "s:tests/basic.feature#1".into(),
        status,
        duration_millis: None,
    }
}

/// Two runs of the same files: `passing hello` passed in the older and failed in the newer.
fn seed_flaky(data_dir: &Path, root: &str) {
    let ids = seed(data_dir, root, 2, |r| {
        r.fingerprint = Some("a".repeat(64));
        r.tests = vec![hello(TestStatusFinal::Passed)];
    });
    let file = record_file(data_dir, root, &ids[1]);
    let mut newer = record(data_dir, root, &ids[1]);
    newer["tests"] = json!([hello(TestStatusFinal::Failed)]);
    fs::write(file, newer.to_string()).unwrap();
}

#[test]
fn sends_the_flaky_tests_with_the_history() {
    let t = setup();
    let fx = fixture("clean");
    seed_flaky(&t.data_dir, &fx.root);
    t.runner.open(&fx.root).unwrap();
    assert_match(
        &t.last("flaky")["tests"],
        json!([{ "key": "s:tests/basic.feature#1", "envs": [{ "env": null, "passed": 1, "failed": 1 }] }]),
    );
    assert!(t.runner.replay().contains(&t.last_event("flaky")));
}

#[test]
fn follows_a_new_tree() {
    let t = setup();
    let fx = fixture("clean");
    seed_flaky(&t.data_dir, &fx.root);
    t.runner.open(&fx.root).unwrap();
    let file = fx.join("tests/basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    let edited = regex::Regex::new(r"(@smoke\r?\nScenario: passing hello)")
        .unwrap()
        .replace(&text, "Scenario: added\n  * match 1 == 1\n\n$1")
        .into_owned();
    assert_ne!(edited, text);
    fs::write(&file, edited).unwrap();
    t.runner.refresh().unwrap();
    let key = t.key("passing hello");
    assert_ne!(key, "s:tests/basic.feature#1");
    // sent with the new tree, before the history that follows it
    let events = t.json();
    let project = events.iter().rposition(|e| e["type"] == "project").unwrap();
    assert_match(
        &events[project + 1],
        json!({ "type": "flaky", "tests": [{ "key": key }] }),
    );
    let keys: Vec<Value> = t.last("flaky")["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["key"].clone())
        .collect();
    assert_eq!(keys, [json!(key)]);
}

#[test]
fn clear_history_leaves_no_flaky_tests() {
    let t = setup();
    let fx = fixture("clean");
    seed_flaky(&t.data_dir, &fx.root);
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.last("flaky")["tests"].as_array().unwrap().len(), 1);
    t.runner.clear_history();
    assert_eq!(t.last("history")["runs"], json!([]));
    assert_eq!(t.last("flaky")["tests"], json!([]));
}
