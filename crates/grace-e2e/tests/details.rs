//! `e2e/details.spec.ts`, its run part, through the accessibility tree: a failed test's place, error, steps and a
//! step's log after a real run. The steps from the file before a run come with phase P4.
use std::time::Duration;

use grace_e2e::{App, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn failure_details() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| named(t, "9 tests · Not run yet").is_some())
        .unwrap();
    app.wait_for("nothing selected", WAIT, |t| {
        named(t, "Select a test to see its details").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    let tree = app
        .wait_for("failing match failed", RUN, |t| {
            t.all().into_iter().any(|n| n.name.starts_with("Finished"))
                && row(t, "failing match").is_some_and(|r| r.name.contains(", failed"))
        })
        .unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();

    let tree = app
        .wait_for("the test's details", WAIT, |t| {
            named(t, "tests/basic.feature:15").is_some()
        })
        .unwrap();
    let details = tree
        .find(&Role::Group, "Details")
        .unwrap_or_else(|| panic!("{}", tree.dump()));
    assert!(
        named(details, "failing match").is_some(),
        "the heading: {}",
        details.dump()
    );
    assert!(named(details, "Failed").is_some(), "the badge: {}", details.dump());
    let error = details
        .find(&Role::Group, "Error")
        .unwrap_or_else(|| panic!("{}", details.dump()));
    let text = error
        .all()
        .into_iter()
        .map(|n| n.name.clone())
        .find(|n| n.contains("basic.feature:20"))
        .unwrap_or_else(|| panic!("{}", error.dump()));
    // Karate's comment on the failed step comes with its error (step comments R2.4)
    let at = text.find("basic.feature:20").unwrap();
    let comment = text.find("# expected greeting is wrong on purpose").expect(&text);
    let step = text.find("And match").expect(&text);
    assert!(at < comment && comment < step, "{text}");

    let steps = details
        .find(&Role::List, "Steps")
        .unwrap_or_else(|| panic!("{}", details.dump()));
    let items: Vec<_> = steps.all().into_iter().filter(|n| n.role == Role::ListItem).collect();
    assert_eq!(items.len(), 5, "the background's url and 4 steps: {}", steps.dump());

    let toggle = steps
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.starts_with("Show log of method get"))
        .unwrap_or_else(|| panic!("{}", steps.dump()))
        .name
        .clone();
    app.press_named(Role::Button, &toggle).unwrap();
    let hide = toggle.replacen("Show", "Hide", 1);
    app.wait_for("the log", WAIT, |t| {
        named(t, &hide).is_some()
            && t.all()
                .into_iter()
                .any(|n| n.name.contains("GET") && n.name.contains("200"))
    })
    .unwrap();
}
