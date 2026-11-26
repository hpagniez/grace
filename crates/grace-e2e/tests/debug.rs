//! `e2e/debug.spec.ts` through the accessibility tree: the step debugger in the app, on the `debug` fixture with the
//! real Java, Karate and `listener.jar`. Every pause is waited for in the tree.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

const CONTROLS: [&str; 3] = ["Continue", "Step over", "Step into"];

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

/// The step Details marks "Paused", by its words.
fn paused_step(tree: &Node) -> Option<&str> {
    tree.find(&Role::List, "Steps")?
        .all()
        .into_iter()
        .find(|n| n.role == Role::ListItem && named(n, "Paused").is_some())
        .map(|n| n.name.as_str())
}

/// The names the Variables tab lists, in order.
fn variables(tree: &Node) -> Vec<&str> {
    tree.find(&Role::List, "Variables").map_or_else(Vec::new, |list| {
        list.all()
            .into_iter()
            .filter(|n| n.role == Role::ListItem)
            .map(|n| n.name.as_str())
            .collect()
    })
}

/// Whether the variable `name` shows the label `text`: its kind or its value's line.
fn variable_says(tree: &Node, name: &str, text: &str) -> bool {
    tree.find(&Role::List, "Variables")
        .and_then(|list| list.find(&Role::ListItem, name))
        .is_some_and(|v| named(v, text).is_some())
}

fn waits_at(tree: &Node, step: &str) -> bool {
    CONTROLS.iter().all(|c| tree.find(&Role::Button, c).is_some()) && paused_step(tree) == Some(step)
}

/// The `debug` project open, a breakpoint set on line 8 of "variables" from Details, its debug run started from its
/// row's Debug button and waiting there.
fn paused_at_line_8() -> (App, Node) {
    let dir = project("debug").unwrap();
    // the panel tall enough for the variables, and short enough to leave the tree its rows
    let settings = json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 220 });
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    // the tree comes with the dry run, which takes longer than a press waits
    app.wait_for("the tests found", WAIT, |t| {
        t.find(&Role::Button, "Expand debug.feature").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Expand debug.feature").unwrap();
    let tree = app
        .wait_for("the test's row", WAIT, |t| row(t, "variables").is_some())
        .unwrap();
    app.click(row(&tree, "variables").unwrap()).unwrap();
    app.press_named(Role::Button, "Add breakpoint at line 8").unwrap();
    app.wait_for_node(Role::Button, "Remove breakpoint at line 8").unwrap();
    app.press_named(Role::Button, "Debug variables").unwrap();
    let tree = app
        .wait_for("the pause before line 8", RUN, |t| {
            waits_at(t, "* def user = { id: 7 }")
        })
        .unwrap();
    (app, tree)
}

/// E-debug-01, SD-17: Debug from the tree, paused with the test selected and the step marked, the variables, F10 to
/// the next step, a value opened, then Continue to the end.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn debugging_from_the_tree_paused_stepping_and_finished() {
    let (app, _) = paused_at_line_8();
    let tree = app
        .wait_for("the paused run's whole window", WAIT, |t| {
            let variable = row(t, "variables");
            variable.is_some_and(|r| r.states.selected && r.find(&Role::Image, "Paused").is_some())
                && t.find(&Role::Button, "Cancel").is_some()
                && t.find(&Role::Button, "Run all").is_none()
                && t.find(&Role::Button, "Pause").is_none()
                && t.find(&Role::Tab, "Variables").is_some_and(|tab| tab.states.selected)
                && variables(t) == ["base", "name"]
                && variable_says(t, "base", "from background")
                && variable_says(t, "base", "string")
                && variable_says(t, "name", "grace")
                // the progress line, beside the step's own "Paused"
                && t.all().into_iter().filter(|n| n.role != Role::Image && n.name == "Paused").count() == 2
        })
        .unwrap();
    assert_eq!(tree.unnamed_controls(), Vec::<String>::new());

    app.key(Key::F10).unwrap();
    app.wait_for("the pause before the call", RUN, |t| {
        waits_at(t, "* def called = call read('called.feature') { arg: 'in' }")
            && variables(t) == ["base", "name", "user"]
            && variable_says(t, "user", "json")
    })
    .unwrap();

    app.press_named(Role::Button, "Show user").unwrap();
    app.wait_for("the value whole", WAIT, |t| {
        t.find(&Role::Button, "Hide user").is_some() && variable_says(t, "user", "{\n  \"id\": 7\n}")
    })
    .unwrap();

    app.press_named(Role::Button, "Continue").unwrap();
    app.wait_for("the run over", RUN, |t| {
        named(t, "Finished").is_some()
            && t.find(&Role::Button, "Run all").is_some()
            && CONTROLS.iter().all(|c| t.find(&Role::Button, c).is_none())
            && t.find(&Role::Tab, "Variables").is_none()
            && t.find(&Role::Tab, "Console").is_some_and(|tab| tab.states.selected)
            && paused_step(t).is_none()
            && row(t, "variables").is_some_and(|r| r.name.starts_with("variables, passed"))
    })
    .unwrap();
}

/// E-debug-02, SD-23: at 760 px the debug controls are icons with their names, whole inside the window; Cancel while
/// paused ends the run and leaves no Java.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_debug_controls_at_760_px() {
    let (app, tree) = paused_at_line_8();
    let at = tree.rect.expect("the window's place");
    app.set_window_rect(at.x, at.y, 760, at.height).unwrap();
    // the view's own place: the window's has none through AT-SPI
    let view = |t: &Node| {
        t.all()
            .into_iter()
            .find(|n| n.name == "Grace" && n.role != Role::Window)
            .and_then(|n| n.rect)
    };
    app.wait_for("the controls as icons inside the window", WAIT, |t| {
        view(t).is_some_and(|window| {
            window.width <= 800
                && ["Cancel", "Continue", "Step over", "Step into"].iter().all(|name| {
                    t.find(&Role::Button, name).and_then(|b| b.rect).is_some_and(|b| {
                        b.width > 0 && b.width < 40 && b.x >= window.x && b.x + b.width <= window.x + window.width
                    })
                })
        })
    })
    .unwrap();
    app.press_named(Role::Button, "Cancel").unwrap();
    app.wait_for("the run cancelled", RUN, |t| {
        named(t, "Cancelled").is_some() && t.find(&Role::Button, "Run all").is_some()
    })
    .unwrap();
    app.wait_until("no Java", Duration::from_secs(15), || app.java_pids().is_empty())
        .unwrap();
}
