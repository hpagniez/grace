//! `runner.debug.test.ts`: the step debugger in the runner (docs/specs/2026-11-02-step-debugger.md). Its tests of
//! what comes before a run are here; the debug run's come with it.
use super::*;
use crate::karate::tree::find_node;
use crate::model::events::Breakpoint;

/// A fixture project, open.
fn open(name: &str) -> (TestRunner, Fixture) {
    let root = fixture(name);
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    (t, root)
}

fn lines(t: &TestRunner, key: &str) -> Option<Vec<u32>> {
    Some(t.runner.steps(key)?.iter().map(|s| s.line).collect())
}

#[test]
fn lists_a_tests_steps_before_any_run() {
    let (t, _root) = open("debug");
    assert_eq!(lines(&t, &t.key("variables")).unwrap(), [4, 7, 8, 9, 10]);
    assert_eq!(lines(&t, "f:debug.feature"), None);
}

#[test]
fn gives_a_row_the_steps_of_its_outline_and_none_once_the_file_is_gone() {
    let (t, root) = open("clean");
    let tree = t.tree();
    let row = leaf_keys(&tree)
        .into_iter()
        .find(|k| find_node(&tree, k).is_some_and(|n| n.kind == NodeKind::Row && n.file == "tests/outline.feature"))
        .unwrap();
    assert_eq!(lines(&t, &row).unwrap(), [4, 5]);
    fs::remove_file(root.join("tests/outline.feature")).unwrap();
    assert_eq!(lines(&t, &row), None);
    assert_eq!(lines(&t, "s:gone.feature#0"), None);
}

#[test]
fn gives_a_reloaded_window_the_breakpoints_and_refuses_a_line_of_no_feature() {
    let (t, _root) = open("debug");
    t.runner.toggle_breakpoint("debug.feature", 8);
    let set = AppEvent::Breakpoints {
        breakpoints: vec![Breakpoint {
            file: "debug.feature".into(),
            line: 8,
        }],
    };
    assert!(t.runner.replay().contains(&set));
    for (file, line) in [("nope.feature", 3), ("debug.feature", 0)] {
        let sent = t.count("breakpoints");
        t.runner.toggle_breakpoint(file, line);
        assert_eq!(t.last_notice(), "Invalid request", "{file}:{line}");
        assert_eq!(t.count("breakpoints"), sent, "{file}:{line}");
    }
    t.runner.toggle_breakpoint("debug.feature", 8);
    assert_eq!(t.last("breakpoints")["breakpoints"], serde_json::json!([]));
}
