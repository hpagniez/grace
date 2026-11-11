//! Screenshots of GPUI Grace for a pull request: each test drives the app to a state and saves the window as a PNG
//! in `GRACE_SCREENS`. Run on Linux with `GRACE_SCREENS="$PWD/target/screens" sh scripts/e2e-gpui.sh screens_`: the
//! `screens` feature, which the e2e suite leaves out, since a screenshot checks nothing.
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

fn out() -> PathBuf {
    let dir = PathBuf::from(std::env::var("GRACE_SCREENS").expect("GRACE_SCREENS, the folder for the screenshots"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Saves Grace's window, once the tree has settled, as `name`.png.
fn shot(app: &App, name: &str) {
    // the frame that follows the last change: read the tree twice, the window is drawn by then
    let _ = app.tree();
    let _ = app.tree();
    let id = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "^Grace$"])
        .output()
        .unwrap();
    let id = String::from_utf8_lossy(&id.stdout).lines().next().unwrap().to_string();
    let path = out().join(format!("{name}.png"));
    let ok = Command::new("import")
        .args(["-window", &id, path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(ok.success());
}

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

fn open(dir: &std::path::Path, dark: bool) -> App {
    let mut settings = json!({ "lastProject": dir.to_string_lossy() });
    if dark {
        settings["appearance"] = json!({ "theme": "dark" });
    }
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| starting(t, "tests ").is_some())
        .unwrap();
    app
}

fn run_all(app: &App) {
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
}

fn one_feature(text: &str) -> PathBuf {
    let dir = grace_e2e::fresh_data_dir().unwrap().join("checkout");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("checkout.feature"), text).unwrap();
    dir
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_console() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    shot(&app, "console-ready");
    run_all(&app);
    shot(&app, "console-after-a-run");
    app.key(Key::CtrlF).unwrap();
    app.fill(Role::Entry, "Find in console", "INFO").unwrap();
    app.wait_for("matches", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.role == Role::Status && n.name.contains(" of "))
    })
    .unwrap();
    app.key(Key::Enter).unwrap();
    shot(&app, "console-find");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_console_errors() {
    let dir = one_feature(
        "Feature: checkout\n\nScenario: totals add up\n  * def total = 2 + 2\n  * match total == 5\n\nScenario: payment logs a problem\n  * karate.logger.error('card declined')\n  * print 'retrying'\n",
    );
    let app = open(&dir, false);
    run_all(&app);
    app.wait_for("the errors", WAIT, |t| starting(t, "2 errors").is_some())
        .unwrap();
    app.key(Key::F8).unwrap();
    app.wait_for("an error current", WAIT, |t| {
        starting(t, "1 of 2").is_some() || starting(t, "2 of 2").is_some()
    })
    .unwrap();
    shot(&app, "console-errors");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_console_dark() {
    let dir = project("clean").unwrap();
    let app = open(&dir, true);
    run_all(&app);
    shot(&app, "console-dark");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_panel_resized() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    run_all(&app);
    for _ in 0..80 {
        let t = app.wait_for("the tree", WAIT, |_| true).unwrap();
        if t.find(&Role::Other("Separator".into()), "Resize bottom panel")
            .is_some_and(|n| n.states.focused)
        {
            break;
        }
        app.key(Key::Tab).unwrap();
    }
    for _ in 0..12 {
        app.key(Key::Up).unwrap();
    }
    app.wait_for("a taller panel", WAIT, |t| {
        t.find(&Role::Other("Separator".into()), "Resize bottom panel")
            .and_then(|n| n.value)
            .is_some_and(|v| v >= 380.)
    })
    .unwrap();
    app.key(Key::Tab).unwrap();
    app.key(Key::AltRight).unwrap();
    app.key(Key::AltRight).unwrap();
    app.wait_for("Console last", WAIT, |t| {
        let tabs: Vec<_> = t
            .all()
            .into_iter()
            .filter(|n| n.role == Role::Tab)
            .map(|n| n.name.clone())
            .collect();
        tabs.last().is_some_and(|n| n == "Console")
    })
    .unwrap();
    shot(&app, "panel-taller-tabs-moved");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_home() {
    let clean = project("clean").unwrap();
    let tags = project("tags").unwrap();
    let gone = std::env::temp_dir().join("grace-screens-gone/payments-api");
    let app = App::launch_with_settings(
        &json!({ "recentProjects": [clean.to_string_lossy(), tags.to_string_lossy(), gone.to_string_lossy()] }),
        &[],
    )
    .unwrap();
    app.wait_for("the home screen", WAIT, |t| starting(t, "Open folder").is_some())
        .unwrap();
    shot(&app, "home");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_toolbar_during_a_run() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run half way", RUN, |t| {
        t.all()
            .into_iter()
            .any(|n| n.name.len() >= 5 && n.name.ends_with(" / 9") && ["3", "4", "5", "6", "7"].contains(&&n.name[..1]))
    })
    .unwrap();
    shot(&app, "toolbar-during-a-run");
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
    shot(&app, "toolbar-after-a-run");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_tree() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    run_all(&app);
    for f in ["Expand outline.feature", "Expand slow.feature"] {
        app.press_named(Role::Button, f).unwrap();
    }
    app.wait_for("the features open", WAIT, |t| {
        starting(t, "Collapse slow.feature").is_some()
    })
    .unwrap();
    shot(&app, "tree");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_filter_bar() {
    for dark in [false, true] {
        let dir = project("tags").unwrap();
        let app = open(&dir, dark);
        let tree = app.tree().unwrap();
        let chip = tree
            .all()
            .into_iter()
            .find(|n| n.role == Role::Button && n.name.ends_with(", not filtered"))
            .unwrap()
            .name
            .clone();
        app.press_named(Role::Button, &chip).unwrap();
        app.wait_for("a chip included", WAIT, |t| {
            t.all().into_iter().any(|n| n.name.ends_with(", included"))
        })
        .unwrap();
        app.press_named(Role::CheckBox, "Hide non-matching").unwrap();
        app.wait_for("Hide non-matching checked", WAIT, |t| {
            t.find(&Role::CheckBox, "Hide non-matching")
                .is_some_and(|n| n.states.checked)
        })
        .unwrap();
        shot(&app, if dark { "filter-bar-dark" } else { "filter-bar" });
    }
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_filter_bar_many_tags() {
    let dir = grace_e2e::fresh_data_dir().unwrap().join("modules");
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    for i in 0..30 {
        std::fs::write(
            dir.join("tests").join(format!("module-{i:02}.feature")),
            format!("@module-{i:02}\nFeature: module {i}\n\nScenario: scenario {i}\n  * def x = {i}\n"),
        )
        .unwrap();
    }
    let app = open(&dir, false);
    let tree = app
        .wait_for("the fold chip", WAIT, |t| {
            t.all()
                .into_iter()
                .any(|n| n.role == Role::Button && n.name.contains(" more tag"))
        })
        .unwrap();
    let more = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.contains(" more tag"))
        .unwrap()
        .name
        .clone();
    app.press_named(Role::Button, &more).unwrap();
    app.wait_for("the panel", WAIT, |t| t.find(&Role::Group, "All tags").is_some())
        .unwrap();
    shot(&app, "filter-bar-all-tags");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_env_suggestions() {
    let dir = project("env").unwrap();
    let app = open(&dir, false);
    let env = app.wait_for_node(Role::ComboBox, "Env").unwrap();
    app.click(&env).unwrap();
    app.wait_for("the suggestions", WAIT, |t| {
        t.find(&Role::List, "Environments").is_some()
    })
    .unwrap();
    shot(&app, "env-suggestions");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_properties() {
    let dir = project("env").unwrap();
    std::fs::write(
        dir.join("grace-shots.properties"),
        "baseUrl=https://qa.example.test\ntimeout=5000\n",
    )
    .unwrap();
    let app = open(&dir, false);
    app.press_named(Role::Button, "Properties…").unwrap();
    app.wait_for("the dialog", WAIT, |t| t.find(&Role::TabList, "Environments").is_some())
        .unwrap();
    app.press_named(Role::Button, "Add property").unwrap();
    let tree = app
        .wait_for("the new row", WAIT, |t| {
            t.find(&Role::ListItem, "New property").is_some()
        })
        .unwrap();
    let name_box = tree
        .find(&Role::ListItem, "New property")
        .unwrap()
        .all()
        .into_iter()
        .find(|n| n.name == "Property name")
        .unwrap()
        .clone();
    app.click(&name_box).unwrap();
    app.type_text("baseUrl").unwrap();
    let tree = app
        .wait_for("the row", WAIT, |t| t.find(&Role::ListItem, "baseUrl").is_some())
        .unwrap();
    let value_box = tree
        .find(&Role::ListItem, "baseUrl")
        .unwrap()
        .all()
        .into_iter()
        .find(|n| n.name == "Property value")
        .unwrap()
        .clone();
    app.click(&value_box).unwrap();
    app.type_text("https://qa.example.test").unwrap();
    shot(&app, "properties");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_details() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    run_all(&app);
    let tree = app.tree().unwrap();
    let row = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with("failing match, "))
        .unwrap()
        .clone();
    app.click(&row).unwrap();
    let tree = app
        .wait_for("the steps", WAIT, |t| t.find(&Role::List, "Steps").is_some())
        .unwrap();
    let toggle = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.starts_with("Show log of method get"))
        .unwrap()
        .name
        .clone();
    app.press_named(Role::Button, &toggle).unwrap();
    app.wait_for("the log", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.name.starts_with("Hide log of method get"))
    })
    .unwrap();
    shot(&app, "details-failed-test");
    let tree = app.tree().unwrap();
    let feature = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with("basic.feature, "))
        .unwrap()
        .clone();
    app.click(&feature).unwrap();
    app.wait_for("the group", WAIT, |t| {
        t.all().into_iter().any(|n| n.name.ends_with(" scenarios"))
    })
    .unwrap();
    shot(&app, "details-group");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_settings() {
    let dir = project("clean").unwrap();
    for (dark, name) in [(false, "settings-appearance"), (true, "settings-appearance-dark")] {
        let app = open(&dir, dark);
        app.press_named(Role::Button, "Settings").unwrap();
        app.wait_for("the page", WAIT, |t| t.find(&Role::Group, "Theme").is_some())
            .unwrap();
        shot(&app, name);
    }
}

/// A tree longer than the window, its scrollbar, and a cut label shown whole on keyboard focus.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_long_tree() {
    for dark in [false, true] {
        let dir = grace_e2e::fresh_data_dir().unwrap().join("big");
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        let long =
            "feature-000-the-checkout-keeps-the-basket-when-the-session-ends-and-the-customer-signs-in-again-later";
        for name in (1..120).map(|i| format!("feature-{i:03}")).chain([long.to_owned()]) {
            std::fs::write(
                dir.join("tests").join(format!("{name}.feature")),
                format!("Feature: {name}\n\nScenario: works\n  * def x = 1\n"),
            )
            .unwrap();
        }
        let app = open(&dir, dark);
        let tests = app
            .wait_for("the features", WAIT, |t| starting(t, "feature-001.feature").is_some())
            .unwrap();
        let folder = tests
            .all()
            .into_iter()
            .find(|n| n.role == Role::TreeItem && n.name.starts_with("tests, "))
            .unwrap()
            .clone();
        app.press(&folder).unwrap();
        app.key(Key::Down).unwrap();
        app.wait_for_node(Role::Tooltip, &format!("{long}.feature · {long}"))
            .unwrap();
        shot(&app, if dark { "long-tree-dark" } else { "long-tree" });
    }
}

/// The toolbar in a window narrower than 1280 px: four buttons as icons, their names in tooltips.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_narrow_toolbar() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    app.set_window_rect(0, 0, 1000, 640).unwrap();
    let open = app
        .wait_for("the narrow toolbar", WAIT, |t| {
            t.find(&Role::Button, "Open…")
                .and_then(|b| b.rect)
                .is_some_and(|r| r.width < 40)
        })
        .unwrap();
    app.hover(open.find(&Role::Button, "Open…").unwrap()).unwrap();
    app.wait_for_node(Role::Tooltip, "Open…").unwrap();
    shot(&app, "toolbar-narrow");
}
