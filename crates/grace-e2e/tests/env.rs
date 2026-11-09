//! `e2e/env.spec.ts`'s tests of the environment box (EV-01 to EV-05, EV-11's environment, EV-12), through the
//! accessibility tree. The box is a combo box named "Env"; its suggestions a list named "Environments". The run
//! properties' tests (EV-06 to EV-10) come with the properties dialog.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(60);
const ENV: &str = "Env";

/// The tree item whose name starts with `label` and a comma.
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn says(tree: &Node, label: &str, status: &str) -> bool {
    row(tree, label)
        .is_some_and(|r| r.name == format!("{label}, {status}") || r.name.starts_with(&format!("{label}, {status}, ")))
}

fn label<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

/// Grace on the env sample project, its feature open.
fn open_env(env: &[(&str, &str)]) -> (PathBuf, App) {
    let dir = project("env").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), env).unwrap();
    app.wait_for("the tests found", WAIT, |t| {
        t.find(&Role::Button, "Expand env.feature").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Expand env.feature").unwrap();
    app.wait_for("the feature open", WAIT, |t| row(t, "always").is_some())
        .unwrap();
    (dir, app)
}

/// The environment `settings.json` holds for the project at `dir`.
fn saved_env(app: &App, dir: &Path) -> serde_json::Value {
    app.settings()["projects"][dir.to_string_lossy().as_ref()]["env"].clone()
}

/// Types the environment and presses Enter, then waits until the runner has saved it (`chooseEnv`).
fn choose_env(app: &App, dir: &Path, name: &str) {
    app.fill(Role::ComboBox, ENV, name).unwrap();
    app.key(Key::Enter).unwrap();
    app.wait_until(&format!("{name} saved"), WAIT, || saved_env(app, dir) == json!(name))
        .unwrap();
}

/// Runs the "always" scenario on its own and waits for its end.
fn run_always(app: &App) {
    app.press_named(Role::Button, "Run always").unwrap();
    app.wait_for("always ran", RUN, |t| says(t, "always", "passed"))
        .unwrap();
}

/// EV-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn suggestions_come_from_overlay_files_and_env_tags() {
    let (_, app) = open_env(&[]);
    let env = app.wait_for_text(Role::ComboBox, ENV, "").unwrap();
    assert!(env.states.enabled);
    app.click(&env).unwrap();
    let tree = app
        .wait_for("the suggestions", WAIT, |t| {
            t.find(&Role::List, "Environments").is_some()
        })
        .unwrap();
    let names: Vec<_> = tree
        .find(&Role::List, "Environments")
        .unwrap()
        .all()
        .into_iter()
        .filter(|n| n.role == Role::ListItem)
        .map(|n| n.name.clone())
        .collect();
    assert_eq!(names, ["(none)", "prod", "qa"], "{}", tree.dump());
}

/// EV-02
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn changing_the_environment_updates_the_tree() {
    let (dir, app) = open_env(&[]);
    let tree = app.tree().unwrap();
    assert!(
        row(&tree, "qa only").is_none() && row(&tree, "not prod").is_some(),
        "{}",
        tree.dump()
    );

    choose_env(&app, &dir, "qa");
    app.wait_for("qa only", WAIT, |t| {
        row(t, "qa only").is_some() && row(t, "not prod").is_some()
    })
    .unwrap();

    choose_env(&app, &dir, "prod");
    app.wait_for("prod's tests", WAIT, |t| {
        row(t, "qa only").is_none() && row(t, "not prod").is_none() && row(t, "always").is_some()
    })
    .unwrap();
}

/// EV-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn changing_the_environment_clears_results() {
    let (dir, app) = open_env(&[]);
    choose_env(&app, &dir, "qa");
    app.wait_for("qa's tree", WAIT, |t| row(t, "qa only").is_some())
        .unwrap();
    run_always(&app);
    // a button's disabled state reaches UI Automation only: the pinned AT-SPI adapter drops it
    app.wait_for("Report enabled", WAIT, |t| {
        t.find(&Role::Button, "Report").is_some_and(|b| b.states.enabled)
    })
    .unwrap();

    choose_env(&app, &dir, "prod");
    app.wait_for("the result cleared", WAIT, |t| {
        says(t, "always", "not run")
            && (cfg!(target_os = "linux") || t.find(&Role::Button, "Report").is_some_and(|b| !b.states.enabled))
    })
    .unwrap();
}

/// EV-04
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_run_uses_the_environment() {
    let (dir, app) = open_env(&[]);
    choose_env(&app, &dir, "qa");
    app.wait_for("qa's tree", WAIT, |t| row(t, "qa only").is_some())
        .unwrap();
    run_always(&app);
    app.wait_for("the run's environment", WAIT, |t| {
        t.all().iter().any(|n| n.role == Role::Text && n.name == "qa")
    })
    .unwrap();
    assert!(run_log(&app).contains("karate.env = qa"));
}

/// EV-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn an_environment_without_an_overlay_file() {
    let (dir, app) = open_env(&[]);
    choose_env(&app, &dir, "staging");
    let hint = "No karate-config-staging.js found. Fine if karate-config.js handles it.";
    app.wait_for("the hint", WAIT, |t| label(t, hint).is_some()).unwrap();
    app.wait_for_text(Role::ComboBox, ENV, "staging").unwrap();
    run_always(&app);
    assert!(run_log(&app).contains("karate.env = staging"));
}

/// EV-11, the environment: it survives a restart (the properties' part comes with their dialog)
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_environment_survives_a_restart() {
    let (dir, app) = open_env(&[]);
    choose_env(&app, &dir, "qa");
    let app = app.relaunch(&[]).unwrap();
    app.wait_for_text(Role::ComboBox, ENV, "qa").unwrap();
    app.press_named(Role::Button, "Expand env.feature").unwrap();
    app.wait_for("qa only", WAIT, |t| row(t, "qa only").is_some()).unwrap();
}

/// EV-12
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_machines_karate_env_is_ignored() {
    let (_, app) = open_env(&[("KARATE_ENV", "prod")]);
    // discovery ignored it too: with prod, "not prod" would be absent
    assert!(row(&app.tree().unwrap(), "not prod").is_some());
    app.wait_for_text(Role::ComboBox, ENV, "").unwrap();
    run_always(&app);
    assert!(run_log(&app).contains("karate.env = null"));
}

/// Karate's log of the newest run, from its events file: the console comes with P2's fifth PR.
fn run_log(app: &App) -> String {
    let events = newest_events(app.data_dir()).expect("a run's events");
    std::fs::read_to_string(events).unwrap_or_default()
}

/// The newest run's `karate-events.jsonl` under the data folder.
pub fn newest_events(dir: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.file_name().is_some_and(|n| n == "karate-events.jsonl") {
                out.push(p);
            }
        }
    }
    let mut all = Vec::new();
    walk(dir, &mut all);
    all.into_iter()
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}
