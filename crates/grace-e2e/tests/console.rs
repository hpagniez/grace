//! `e2e/console.spec.ts` and `e2e/console-find.spec.ts`, through the accessibility tree: the banner, a run's lines
//! and the console's find and errors on the real runner and Java. A line is a label named by its text; only the
//! lines in view are in the tree, as GPUI draws only those.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

/// Whether the find bar's count says `count`.
fn found(tree: &Node, count: &str) -> bool {
    tree.all()
        .into_iter()
        .any(|n| n.role == Role::Status && n.name == count)
}

/// Opens the find bar with Ctrl+F and types `text`: only the lines in view are in the tree, so the count says what
/// the whole console holds.
fn find(app: &App, text: &str) {
    app.key(Key::CtrlF).unwrap();
    app.fill(Role::Entry, "Find in console", text).unwrap();
}

fn versions() -> (String, String) {
    let vendor: serde_json::Value = serde_json::from_str(include_str!("../../../scripts/vendor.json")).unwrap();
    (
        vendor["karate"]["version"].as_str().unwrap().to_string(),
        vendor["jre"]["version"].as_str().unwrap().to_string(),
    )
}

/// Grace on `name`'s sample project, its tests found.
fn open(name: &str) -> (std::path::PathBuf, App) {
    let dir = project(name).unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| starting(t, "tests ").is_some())
        .unwrap();
    (dir, app)
}

fn run_all(app: &App) {
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run over", RUN, |t| {
        named(t, "Finished").is_some() || starting(t, "Finished").is_some()
    })
    .unwrap();
}

/// E-console-01, CB-11
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn open_run_and_run_again() {
    let (dir, app) = open("clean");
    let (karate, java) = versions();
    let tree = app.tree().unwrap();
    let banner = starting(&tree, "Grace ").expect("the versions line").name.clone();
    assert!(banner.contains(&format!("Karate {karate}")), "{banner}");
    assert!(banner.contains(&format!("Java {java}")), "{banner}");
    assert_eq!(
        starting(&tree, "project ").unwrap().name,
        format!("project {}", dir.display())
    );
    let tests = starting(&tree, "tests ").unwrap().name.clone();
    assert!(tests.ends_with(" in 3 features"), "{tests}");
    assert!(starting(&tree, "Ready.").is_some());

    run_all(&app);
    let tree = app.tree().unwrap();
    assert!(
        starting(&tree, "Ready.").is_none(),
        "the state line goes once Karate prints"
    );
    // basic.feature's "passing hello" prints this once per run
    find(&app, "env is");
    app.wait_for("one line of the run", WAIT, |t| found(t, "1 of 1"))
        .unwrap();
    app.key(Key::Escape).unwrap();

    run_all(&app);
    find(&app, "env is");
    // the first run's output is gone
    app.wait_for("one line of the second run", WAIT, |t| found(t, "1 of 1"))
        .unwrap();
}

/// A project of one feature holding `text`, in a folder of its own.
fn one_feature(text: &str) -> std::path::PathBuf {
    let dir = grace_e2e::fresh_data_dir().unwrap().join("find");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("find.feature"), text).unwrap();
    dir
}

fn open_dir(dir: &std::path::Path) -> App {
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| starting(t, "tests ").is_some())
        .unwrap();
    app
}

/// The count a Status node says, among `counts`.
fn status_among(tree: &Node, counts: &[&str]) -> Option<String> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Status && counts.contains(&n.name.as_str()))
        .map(|n| n.name.clone())
}

/// E-console-find-01, CS-14
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn find_in_a_real_run() {
    let dir =
        one_feature("Feature: find\n\nScenario: one\n  * print 'zebra-42'\n\nScenario: two\n  * print 'zebra-42'\n");
    let app = open_dir(&dir);
    run_all(&app);
    find(&app, "zebra-42");
    let tree = app
        .wait_for("two matches", WAIT, |t| {
            status_among(t, &["1 of 2", "2 of 2"]).is_some()
        })
        .unwrap();
    let first = status_among(&tree, &["1 of 2", "2 of 2"]).unwrap();
    app.key(Key::Enter).unwrap();
    let next = if first == "1 of 2" { "2 of 2" } else { "1 of 2" };
    app.wait_for("the next match", WAIT, |t| found(t, next)).unwrap();
    app.key(Key::ShiftEnter).unwrap();
    app.wait_for("the match before", WAIT, |t| found(t, &first)).unwrap();
}

/// E-console-find-02, CS-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn step_through_a_failing_runs_errors() {
    let dir = one_feature(
        "Feature: errors\n\nScenario: fails\n  * match 1 == 2\n\nScenario: logs\n  * karate.logger.error('custom boom')\n",
    );
    let app = open_dir(&dir);
    run_all(&app);
    app.wait_for("two errors", WAIT, |t| found(t, "2 errors")).unwrap();
    assert!(app.tree().unwrap().find(&Role::Group, "Errors").is_some());
    // the ERROR line comes during the run, the failed test in Karate's summary at its end
    app.key(Key::F8).unwrap();
    let tree = app
        .wait_for("an error current", WAIT, |t| {
            status_among(t, &["1 of 2 errors", "2 of 2 errors"]).is_some()
        })
        .unwrap();
    if status_among(&tree, &["1 of 2 errors"]).is_some() {
        app.key(Key::F8).unwrap();
    }
    app.wait_for("the second error", WAIT, |t| found(t, "2 of 2 errors"))
        .unwrap();
    app.key(Key::F8).unwrap();
    app.wait_for("round to the first", WAIT, |t| found(t, "1 of 2 errors"))
        .unwrap();
    app.key(Key::ShiftF8).unwrap();
    app.wait_for("back to the second", WAIT, |t| found(t, "2 of 2 errors"))
        .unwrap();
    // the buttons step too
    app.press_named(Role::Button, "Next error").unwrap();
    app.wait_for("the button steps", WAIT, |t| found(t, "1 of 2 errors"))
        .unwrap();
}

/// CS-06, CS-11: Ctrl+F from another tab shows the Console; Escape closes the bar and keeps its text.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn ctrl_f_opens_the_bar_from_another_tab() {
    let (_dir, app) = open("clean");
    run_all(&app);
    let history = starting(&app.tree().unwrap(), "History")
        .expect("the History tab")
        .name
        .clone();
    app.press_named(Role::Tab, &history).unwrap();
    app.wait_for("the History tab", WAIT, |t| {
        t.find(&Role::Tab, &history).is_some_and(|n| n.states.selected)
    })
    .unwrap();
    assert!(
        app.tree().unwrap().find(&Role::Button, "Find").is_none(),
        "Find is the Console tab's"
    );
    find(&app, "env is");
    app.wait_for("the Console tab and a match", WAIT, |t| {
        t.find(&Role::Tab, "Console").is_some_and(|n| n.states.selected) && found(t, "1 of 1")
    })
    .unwrap();
    app.key(Key::Escape).unwrap();
    app.wait_for_no_node(Role::Entry, "Find in console").unwrap();
    app.press_named(Role::Button, "Find").unwrap();
    app.wait_for("the text kept", WAIT, |t| found(t, "1 of 1")).unwrap();
}
