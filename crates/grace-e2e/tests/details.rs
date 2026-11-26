//! `e2e/details.spec.ts` through the accessibility tree: a test's steps and comments from its file before a run, then
//! its place, error, steps and a step's log after a real run; and the breakpoint toggles and dots as a screen reader
//! hears them.
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

/// The steps Details lists.
fn step_items(tree: &Node) -> Vec<&Node> {
    tree.find(&Role::List, "Steps").map_or_else(Vec::new, |steps| {
        steps.all().into_iter().filter(|n| n.role == Role::ListItem).collect()
    })
}

/// Whether the row of `label` says it holds a breakpoint.
fn dotted(tree: &Node, label: &str) -> bool {
    row(tree, label).is_some_and(|r| r.find(&Role::Image, "Has breakpoints").is_some())
}

/// SD-15's names: the toggle says what a press does and on which line, and the rows say they hold a breakpoint.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_breakpoint_set_before_a_run_shows_on_its_step_and_in_the_tree() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    // the tree comes with the dry run, which takes longer than a press waits
    app.wait_for("the tests found", WAIT, |t| named(t, "9 tests · Not run yet").is_some())
        .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    let tree = app
        .wait_for("the test's row", WAIT, |t| row(t, "failing match").is_some())
        .unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    app.press_named(Role::Button, "Add breakpoint at line 20").unwrap();
    app.wait_for("the breakpoint set", WAIT, |t| {
        t.find(&Role::Button, "Remove breakpoint at line 20").is_some()
            && t.find(&Role::Button, "Add breakpoint at line 4").is_some()
            && dotted(t, "basic.feature")
            && dotted(t, "failing match")
            && row(t, "passing hello").is_some()
            && !dotted(t, "passing hello")
    })
    .unwrap();
    app.press_named(Role::Button, "Remove breakpoint at line 20").unwrap();
    app.wait_for("no breakpoint", WAIT, |t| {
        t.find(&Role::Button, "Add breakpoint at line 20").is_some()
            && row(t, "failing match").is_some()
            && !dotted(t, "basic.feature")
            && !dotted(t, "failing match")
    })
    .unwrap();
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
    // before a run, the steps and their comments come from the file (step comments R1.2)
    let tree = app
        .wait_for("the test's row", WAIT, |t| row(t, "failing match").is_some())
        .unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    let expected = "# expected greeting is wrong on purpose";
    app.wait_for("the file's steps, one with its comment", WAIT, |t| {
        step_items(t).len() == 5
            && step_items(t)
                .into_iter()
                .any(|s| s.name.contains("response.message == 'goodbye'") && named(s, expected).is_some())
    })
    .unwrap();
    app.press_named(Role::Button, "Run basic.feature").unwrap();
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
    assert_eq!(
        step_items(&tree).len(),
        5,
        "the background's url and 4 steps: {}",
        steps.dump()
    );
    // the run's comment is on the failed step alone, as the file's was
    let commented: Vec<_> = step_items(&tree)
        .into_iter()
        .filter(|s| named(s, expected).is_some())
        .collect();
    assert_eq!(commented.len(), 1, "{}", steps.dump());
    assert!(
        commented[0].name.contains("response.message == 'goodbye'"),
        "{}",
        steps.dump()
    );

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

    // a feature: its scenarios counted, and how many ended each way (`Details.test.tsx`)
    let tree = app.tree().unwrap();
    app.click(row(&tree, "basic.feature").unwrap()).unwrap();
    app.wait_for("the feature's counts", WAIT, |t| {
        t.find(&Role::Group, "Details").is_some_and(|d| {
            let names: Vec<_> = d.all().into_iter().map(|n| n.name.clone()).collect();
            names.iter().any(|n| n.ends_with(" scenarios"))
                && names.iter().any(|n| n == "1 Failed")
                && names.iter().any(|n| n.ends_with(" Passed"))
        })
    })
    .unwrap();
}
