//! `e2e/failures.spec.ts` (E-failures-01, GF-22) through the accessibility tree: a broken `karate-config.js` fails
//! every test of a run, so Grace says it once with a link to the file and line, and steps through the tests that
//! share the error. The app's `e2e` seams record each link and never start VS Code (plan P3, Redesigns, `seams`).
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::Path;
use std::time::Duration;

use grace_e2e::{App, Node, Role, fresh_data_dir};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(120);

/// A project whose `karate-config.js` fails on line 2, with 2 scenarios in `a.feature` and 1 in `b.feature`.
fn broken_config_project() -> std::path::PathBuf {
    let dir = fresh_data_dir().unwrap();
    let write = |file: &str, lines: &[&str]| std::fs::write(dir.join(file), lines.join("\n") + "\n").unwrap();
    write(
        "karate-config.js",
        &["function fn() {", "  karate.fail('no vpn');", "  return {};", "}"],
    );
    let scenario = |name: &str| [format!("Scenario: {name}"), "* print 'x'".to_string(), String::new()];
    let a: Vec<String> = ["Feature: a".to_string(), String::new()]
        .into_iter()
        .chain(scenario("one"))
        .chain(scenario("two"))
        .collect();
    let b: Vec<String> = ["Feature: b".to_string(), String::new()]
        .into_iter()
        .chain(scenario("three"))
        .collect();
    write("a.feature", &a.iter().map(String::as_str).collect::<Vec<_>>());
    write("b.feature", &b.iter().map(String::as_str).collect::<Vec<_>>());
    dir
}

/// The link a file of the project opens at: `vscode://file/<project path>/<file>` without the line.
fn link(dir: &Path, file: &str) -> String {
    let dir = dir.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    format!("vscode://file/{}/{file}", dir.trim_start_matches('/'))
}

/// A tree row, "<label>, <status>...".
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

/// A node that says `text` as its name or its text.
fn says(tree: &Node, text: &str) -> bool {
    tree.all()
        .into_iter()
        .any(|n| n.name.contains(text) || n.text.as_deref().is_some_and(|t| t.contains(text)))
}

/// E-failures-01, GF-18, GF-12, GF-13: the banner, its link at the config's line, and Previous from the first test of
/// the group, which wraps round to the last and has the tree show it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_broken_karate_config_says_so_once_and_the_failures_step() {
    let dir = broken_config_project();
    // the least bottom panel: the banner takes a third of the pane, and a press clicks the centre of a button, which
    // the Details pane must show (plan P3, Redesigns, `grace-e2e` (failures))
    let app =
        App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 96 }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| named(t, "3 tests · Not run yet").is_some())
        .unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the 3 tests failed", RUN, |t| {
        ["one", "two", "three"]
            .iter()
            .all(|n| row(t, n).is_some_and(|r| r.name.contains(", failed")))
    })
    .unwrap_or_else(|e| panic!("{e}: {}", app.tree().unwrap().dump()));

    let tree = app
        .wait_for("the banner", WAIT, |t| {
            named(t, "karate-config.js failed for 3 tests").is_some()
        })
        .unwrap();
    assert!(says(&tree, "no vpn"), "Karate's reason shows: {}", tree.dump());
    app.press_named(Role::Button, "Open karate-config.js in VS Code")
        .unwrap();
    app.wait_until("the link at line 2", WAIT, || {
        app.opened_urls() == [format!("{}:2", link(&dir, "karate-config.js"))]
    })
    .unwrap();

    let tree = app.tree().unwrap();
    app.click(row(&tree, "one").unwrap()).unwrap();
    app.wait_for("1 of 3", WAIT, |t| {
        named(t, "1 of 3 tests failed with this error").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Previous test with this error").unwrap();
    app.wait_for("the last test selected and shown", WAIT, |t| {
        row(t, "three").is_some_and(|r| r.states.selected) && named(t, "3 of 3 tests failed with this error").is_some()
    })
    .unwrap_or_else(|e| panic!("{e}: {}", app.tree().unwrap().dump()));
}
