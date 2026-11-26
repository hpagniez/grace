//! `e2e/watch.spec.ts`, through the accessibility tree: watch mode in the app.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::Path;
use std::time::Duration;

use grace_e2e::{App, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

/// Whether the tree row of `label` says `status`.
fn says(tree: &Node, label: &str, status: &str) -> bool {
    let start = format!("{label}, {status}");
    tree.all()
        .into_iter()
        .any(|n| n.role == Role::TreeItem && (n.name == start || n.name.starts_with(&format!("{start}, "))))
}

fn history(tree: &Node, runs: usize) -> bool {
    tree.find(&Role::Tab, &format!("History ({runs})")).is_some()
}

/// Whether the Watch button is there and pressed as `on` says.
fn watch_is(tree: &Node, on: bool) -> bool {
    tree.find(&Role::Button, "Watch")
        .is_some_and(|b| b.states.checked == on)
}

/// Presses Watch and waits for the runner's answer: it sends the choice once the watcher is on, or off.
fn turn_watch(app: &App, on: bool) {
    app.press_named(Role::Button, "Watch").unwrap();
    app.wait_for(&format!("Watch pressed: {on}"), WAIT, |t| watch_is(t, on))
        .unwrap();
}

/// Saves `tests/<name>.feature` with one scenario whose step is `step`.
fn save(dir: &Path, name: &str, step: &str) {
    let text = format!("Feature: {name}\n\nScenario: one\n  * {step}\n");
    std::fs::write(dir.join("tests").join(format!("{name}.feature")), text).unwrap();
}

/// E-watch-01, WM-17
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_save_runs_the_saved_feature_while_watch_is_on() {
    let dir = project("tags").unwrap();
    // the panel at its smallest leaves the tree room for every row
    let settings = json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 96 });
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    app.wait_for("the tests found, watch off", WAIT, |t| {
        t.find(&Role::Button, "Expand tags.feature").is_some() && watch_is(t, false)
    })
    .unwrap();

    turn_watch(&app, true);
    let root = dir.to_string_lossy();
    assert_eq!(app.settings()["projects"][root.as_ref()]["watch"], true);
    save(&dir, "w1", "match 1 == 2");
    app.wait_for("the saved feature's run, failed", RUN, |t| {
        says(t, "w1.feature", "failed") && history(t, 1)
    })
    .unwrap();

    // off, a save runs nothing: on again, the next run is the next save's alone
    turn_watch(&app, false);
    save(&dir, "w1", "match 2 == 2");
    turn_watch(&app, true);
    save(&dir, "w2", "match 3 == 3");
    app.wait_for("the second feature's run alone", RUN, |t| {
        says(t, "w2.feature", "passed") && says(t, "w1.feature", "failed") && history(t, 2)
    })
    .unwrap();
}
