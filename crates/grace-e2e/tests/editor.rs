//! `e2e/editor.spec.ts`: Open in VS Code from Details and from the tree's menu, through the accessibility tree. The
//! app's `e2e` seams record each link in `e2e-seams.jsonl` and never start VS Code (plan P3, Redesigns, `seams`).
//!
//! E-editor-03's malformed requests have no twin: GPUI Grace has no IPC to send one through, and a path and a line
//! are typed (plan P3, Redesigns, `app`). The file check that refuses a path outside the project is the runner's own
//! test, `runner::tests::editor`.
use std::path::Path;
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);
const OPEN: &str = "Open in VS Code";

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

/// The link a file of the project opens at: `vscode://file/<project path>/<file>` without the line.
fn link(dir: &Path, file: &str) -> String {
    let dir = dir.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    // a Linux path starts with its own `/`: the link has one slash after `file`, a share's alone has two
    format!("vscode://file/{}/{file}", dir.trim_start_matches('/'))
}

fn menu_item(t: &Node) -> bool {
    t.all().into_iter().any(|n| n.role == Role::MenuItem && n.name == OPEN)
}

/// The project open, the feature expanded and the tests found.
fn open_clean() -> (std::path::PathBuf, App) {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| named(t, "9 tests · Not run yet").is_some())
        .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    app.wait_for("the scenarios", WAIT, |t| row(t, "failing match").is_some())
        .unwrap();
    (dir, app)
}

fn run_all(app: &App) {
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("failing match failed", RUN, |t| {
        t.all().into_iter().any(|n| n.name.starts_with("Finished"))
            && row(t, "failing match").is_some_and(|r| r.name.contains(", failed"))
    })
    .unwrap();
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn open_a_scenario_in_vs_code() {
    let (dir, app) = open_clean();
    let at = link(&dir, "tests/basic.feature");

    let tree = app.tree().unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    app.wait_for("the button", WAIT, |t| {
        t.find(&Role::Group, "Details")
            .is_some_and(|d| d.find(&Role::Button, OPEN).is_some())
    })
    .unwrap();
    app.press_named(Role::Button, OPEN).unwrap();
    app.wait_until("the link", WAIT, || app.opened_urls() == [format!("{at}:15")])
        .unwrap();

    run_all(&app);
    let tree = app.tree().unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    // the error's first line is the place Karate wrote: a link to the failing step
    let tree = app
        .wait_for("the error's place", WAIT, |t| {
            t.all()
                .into_iter()
                .any(|n| n.role == Role::Button && n.name.starts_with("Open ") && n.name.contains("basic.feature:20"))
        })
        .unwrap();
    let place = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.contains("basic.feature:20"))
        .unwrap()
        .name
        .clone();
    assert!(place.ends_with(" in VS Code"), "{place}");
    app.press_named(Role::Button, &place).unwrap();
    app.wait_until("the second link", WAIT, || {
        app.opened_urls() == [format!("{at}:15"), format!("{at}:20")]
    })
    .unwrap();
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn open_a_scenario_from_the_tree() {
    let (dir, app) = open_clean();
    let at = link(&dir, "tests/basic.feature");

    let tree = app.tree().unwrap();
    app.right_click(row(&tree, "failing match").unwrap()).unwrap();
    let tree = app.wait_for("the menu", WAIT, menu_item).unwrap();
    // a right-click does not select the row
    assert!(
        row(&tree, "failing match").is_some_and(|r| !r.states.selected),
        "{}",
        tree.dump()
    );
    app.press_named(Role::MenuItem, OPEN).unwrap();
    app.wait_until("the link", WAIT, || app.opened_urls() == [format!("{at}:15")])
        .unwrap();

    run_all(&app);
    let tree = app.tree().unwrap();
    app.right_click(row(&tree, "failing match").unwrap()).unwrap();
    app.wait_for("the menu", WAIT, menu_item).unwrap();
    app.press_named(Role::MenuItem, OPEN).unwrap();
    app.wait_until("the failed step's link", WAIT, || {
        app.opened_urls() == [format!("{at}:15"), format!("{at}:20")]
    })
    .unwrap();

    // the menu key's other form, on the focused row; Escape closes the menu and opens nothing
    let tree = app.tree().unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    app.key(Key::ShiftF10).unwrap();
    app.wait_for("the menu from the keyboard", WAIT, menu_item).unwrap();
    app.key(Key::Escape).unwrap();
    app.wait_for("the menu closed", WAIT, |t| !menu_item(t)).unwrap();
    assert_eq!(app.opened_urls().len(), 2);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn it_says_so_when_vs_code_is_not_installed() {
    let (_dir, app) = open_clean();
    app.answer_vscode_installed(false).unwrap();
    let tree = app.tree().unwrap();
    app.click(row(&tree, "failing match").unwrap()).unwrap();
    app.wait_for("the button", WAIT, |t| t.find(&Role::Button, OPEN).is_some())
        .unwrap();
    app.press_named(Role::Button, OPEN).unwrap();
    app.wait_for("the notice", WAIT, |t| {
        t.all()
            .iter()
            .any(|n| n.name == "Open in VS Code needs VS Code installed")
    })
    .unwrap();
    assert!(app.opened_urls().is_empty());
}
