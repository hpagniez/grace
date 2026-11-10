//! `e2e/panel.spec.ts`, through the accessibility tree: the bottom panel's height from the splitter's keys and across
//! a restart, and its tabs' order. A drag of the splitter or of a tab, and a window resized under the panel, are the
//! headless tests' (`grace::bottom_panel::tests`): the driver has no drag and does not resize the window.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RESIZE: &str = "Resize bottom panel";

fn splitter_role() -> Role {
    Role::Other("Separator".into())
}

fn splitter(tree: &Node) -> Option<&Node> {
    tree.find(&splitter_role(), RESIZE)
}

/// The panel's height, as the splitter says it.
fn height(tree: &Node) -> Option<f64> {
    splitter(tree)?.value
}

/// The tabs' names, first to last.
fn tabs(tree: &Node) -> Vec<String> {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::Tab)
        .map(|n| n.name.split(' ').next().unwrap_or_default().to_string())
        .collect()
}

/// Grace on the clean sample project, with `settings` besides the last project.
fn open(settings: serde_json::Value) -> App {
    let dir = project("clean").unwrap();
    let mut s = settings;
    s["lastProject"] = json!(dir.to_string_lossy());
    let app = App::launch_with_settings(&s, &[]).unwrap();
    app.wait_for("the splitter", WAIT, |t| splitter(t).is_some()).unwrap();
    app
}

fn wait_height(app: &App, what: &str, ok: impl Fn(f64) -> bool) -> f64 {
    let tree = app.wait_for(what, WAIT, |t| height(t).is_some_and(&ok)).unwrap();
    height(&tree).unwrap()
}

/// Tabs until the splitter has the focus, as a keyboard user reaches it.
fn tab_to_splitter(app: &App) {
    for _ in 0..80 {
        // the tree answers once its nodes settle after a key
        let focused = app.wait_for("the tree", WAIT, |_| true).unwrap();
        if splitter(&focused).is_some_and(|n| n.states.focused) {
            return;
        }
        app.key(Key::Tab).unwrap();
    }
    panic!("the splitter never had the focus:\n{}", app.focus_report());
}

/// E-panel-01, UP-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_bottom_panel_starts_at_its_default_height() {
    let app = open(json!({}));
    wait_height(&app, "the default height", |h| h == 192.);
}

/// E-panel-03, E-panel-12, UP-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_splitter_works_from_the_keyboard() {
    let app = open(json!({}));
    tab_to_splitter(&app);
    app.key(Key::Up).unwrap();
    app.key(Key::Up).unwrap();
    wait_height(&app, "two steps up", |h| h == 224.);
    app.wait_until("the height saved", WAIT, || app.settings()["panelHeight"] == json!(224))
        .unwrap();
    app.key(Key::Home).unwrap();
    wait_height(&app, "the least", |h| h == 96.);
    app.key(Key::End).unwrap();
    let most = wait_height(&app, "the most", |h| h > 224.);
    // Tab moves on, and changes nothing
    app.key(Key::Tab).unwrap();
    app.wait_for("the focus past the splitter", WAIT, |t| {
        splitter(t).is_some_and(|n| !n.states.focused)
    })
    .unwrap();
    assert_eq!(height(&app.tree().unwrap()), Some(most));
}

/// E-panel-06, UP-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_height_survives_a_restart() {
    let app = open(json!({}));
    tab_to_splitter(&app);
    app.key(Key::Up).unwrap();
    wait_height(&app, "a step up", |h| h == 208.);
    app.wait_until("the height saved", WAIT, || app.settings()["panelHeight"] == json!(208))
        .unwrap();
    let app = app.relaunch(&[]).unwrap();
    wait_height(&app, "the saved height", |h| h == 208.);
}

/// E-panel-08, UP-08
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_bad_saved_height_falls_back_to_the_default() {
    let app = open(json!({ "panelHeight": "tall" }));
    wait_height(&app, "the default height", |h| h == 192.);
}

/// E-panel-09, UP-04
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_saved_height_taller_than_the_window_is_clamped_at_start() {
    let app = open(json!({ "panelHeight": 5000 }));
    wait_height(&app, "a clamped height", |h| h > 96. && h < 1000.);
}

/// E-panel-13 (the keys and the restart; the drag is the headless tests')
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_tabs_move_with_alt_left_and_alt_right_and_their_order_survives_a_restart() {
    let app = open(json!({}));
    assert_eq!(tabs(&app.tree().unwrap()), ["Console", "History", "HTTP"]);
    // the Console tab has the focus after the splitter
    tab_to_splitter(&app);
    app.key(Key::Tab).unwrap();
    app.wait_for("the Console tab focused", WAIT, |t| {
        t.find(&Role::Tab, "Console").is_some_and(|n| n.states.focused)
    })
    .unwrap();
    app.key(Key::AltRight).unwrap();
    app.wait_for("Console second", WAIT, |t| tabs(t) == ["History", "Console", "HTTP"])
        .unwrap();
    app.wait_for("the moved tab keeps the focus", WAIT, |t| {
        t.find(&Role::Tab, "Console").is_some_and(|n| n.states.focused)
    })
    .unwrap();
    app.key(Key::AltRight).unwrap();
    app.wait_for("Console last", WAIT, |t| tabs(t) == ["History", "HTTP", "Console"])
        .unwrap();
    app.key(Key::AltLeft).unwrap();
    app.wait_for("Console second again", WAIT, |t| {
        tabs(t) == ["History", "Console", "HTTP"]
    })
    .unwrap();
    app.wait_until("the order saved", WAIT, || {
        app.settings()["panelTabs"] == json!(["history", "console", "http"])
    })
    .unwrap();
    let app = app.relaunch(&[]).unwrap();
    app.wait_for("the saved order", WAIT, |t| tabs(t) == ["History", "Console", "HTTP"])
        .unwrap();
}
