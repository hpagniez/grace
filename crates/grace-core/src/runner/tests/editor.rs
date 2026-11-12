//! `runner.editor.test.ts` (OE-05, OE-06).
use super::*;

/// Every node at or under `n`.
fn flat(n: &TestNode) -> Vec<&TestNode> {
    std::iter::once(n).chain(n.children.iter().flat_map(flat)).collect()
}

#[test]
fn resolves_a_tree_path_through_the_classpath_and_keeps_an_absolute_path() {
    let t = setup();
    let fx = fixture("maven");
    t.runner.open(&fx.root).unwrap();
    let AppEvent::Project { tree: Some(tree), .. } = t.last_event("project") else {
        panic!("no tree");
    };
    let feature = flat(&tree)
        .into_iter()
        .find(|n| n.kind == NodeKind::Feature && n.file.ends_with("users.feature"))
        .unwrap();
    let file = ["src", "test", "java", "examples", "users.feature"]
        .iter()
        .fold(fx.path().to_path_buf(), |p, part| p.join(part));
    assert_eq!(t.runner.editor_file(&feature.file), Some(file.clone()));
    assert_eq!(t.runner.editor_file(&file.to_string_lossy()), Some(file));
    assert_eq!(t.runner.editor_file("../x.feature"), None);
}

#[test]
fn opens_nothing_with_no_project_open() {
    assert_eq!(setup().runner.editor_file("examples/users.feature"), None);
}

#[test]
fn refuses_a_file_outside_the_open_project_and_one_that_was_deleted() {
    let t = setup();
    let fx = fixture("clean");
    let other = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let outside = other.join("tests/basic.feature");
    assert!(outside.is_file());
    assert_eq!(t.runner.editor_file(&outside.to_string_lossy()), None);
    assert_eq!(t.runner.editor_file("tests/gone.feature"), None);
    let inside = fx.join("tests").join("basic.feature");
    assert_eq!(t.runner.editor_file("tests/basic.feature"), Some(inside.clone()));
    std::fs::remove_file(&inside).unwrap();
    assert_eq!(t.runner.editor_file("tests/basic.feature"), None);
}
