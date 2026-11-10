//! `e2e/open.spec.ts` (walking skeleton and UI polish specs), through the accessibility tree. The toolbar's look at
//! a narrow window and the boxes' widths (E-open-03 to 05) need the window resized: PR 6's checks by hand.
use std::path::Path;
use std::time::Duration;

use grace_e2e::{App, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

/// A tree row by its label: the row's name is the label, then its status and tags.
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && (n.name == label || n.name.starts_with(&format!("{label}, "))))
}

fn rows(tree: &Node) -> Vec<String> {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::TreeItem)
        .map(|n| n.name.split(", ").next().unwrap_or_default().to_string())
        .collect()
}

/// Grace with `dir` as the last project, its tests found or its error shown.
fn open(dir: &Path) -> App {
    App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap()
}

/// Waits for the folder dialog's first ask and returns where it started.
fn first_dialog(app: &App) -> Option<String> {
    app.wait_until("the folder dialog", Duration::from_secs(10), || {
        !app.folder_dialogs().is_empty()
    })
    .unwrap();
    app.folder_dialogs().remove(0)
}

/// WS-01, E-open-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn open_a_project_and_see_its_tests() {
    let dir = project("clean").unwrap();
    let app = App::launch().unwrap();
    let home = app
        .wait_for("the home screen", WAIT, |t| {
            t.find(&Role::Button, "Open folder…").is_some()
        })
        .unwrap();
    app.answer_folder(Some(&dir)).unwrap();
    app.press(home.find(&Role::Button, "Open folder…").unwrap()).unwrap();
    let features = ["basic.feature", "outline.feature", "slow.feature"];
    let tree = app
        .wait_for("the tests folder and its features", WAIT, |t| {
            row(t, "tests").is_some() && features.iter().all(|f| row(t, f).is_some())
        })
        .unwrap();
    // folders and feature files only, until a feature opens (#224 R1.1)
    assert_eq!(
        rows(&tree),
        ["tests", "basic.feature", "outline.feature", "slow.feature"],
        "{}",
        tree.dump()
    );
    for f in features {
        assert!(
            row(&tree, f).unwrap().states.expanded != Some(true),
            "{f} starts closed: {}",
            tree.dump()
        );
    }
    let path = dir.to_string_lossy().to_string();
    assert!(named(&tree, &path).is_some(), "the project's path: {}", tree.dump());
    assert!(named(&tree, "Standalone").is_some(), "the layout: {}", tree.dump());

    app.press_named(Role::Button, "Expand outline.feature").unwrap();
    let tree = app
        .wait_for("the outline and its rows", WAIT, |t| {
            row(t, "add <a> and <b>").is_some()
        })
        .unwrap();
    let shown = rows(&tree);
    let at = shown.iter().position(|r| r == "add <a> and <b>").unwrap();
    // the outline's three Examples rows follow it, before the next feature
    let after: Vec<_> = shown[at + 1..].iter().take_while(|r| *r != "slow.feature").collect();
    assert_eq!(after.len(), 3, "{shown:?}");
    // a called feature is not a test
    assert!(shown.iter().all(|r| !r.contains("create-user")), "{shown:?}");
}

/// WS-02, E-open-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn maven_layout_with_classpath_resources() {
    let dir = project("maven").unwrap();
    let app = open(&dir);
    app.wait_for("the Maven layout and its test", WAIT, |t| {
        named(t, "Maven").is_some() && named(t, "1 test · Not run yet").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run finished", RUN, |t| {
        named(t, "Finished").is_some()
            && named(t, "1 / 1").is_some()
            && named(t, "1 passed").is_some()
            && named(t, "0 failed").is_some()
    })
    .unwrap();
}

/// WS-03, E-open-07
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn syntax_error_is_reported_not_hidden() {
    let dir = project("broken").unwrap();
    let app = open(&dir);
    let tree = app
        .wait_for("the error naming the file", WAIT, |t| {
            t.all().iter().any(|n| n.name.contains("syntax.feature"))
        })
        .unwrap();
    assert!(
        row(&tree, "basic.feature").is_none(),
        "no tree under the error: {}",
        tree.dump()
    );
    // Run all is disabled: a press starts nothing
    let _ = app.press_named(Role::Button, "Run all");
    std::thread::sleep(Duration::from_millis(1000));
    let tree = app.tree().unwrap();
    assert!(named(&tree, "Running…").is_none(), "{}", tree.dump());
}

/// UP-09, E-open-09
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn open_starts_in_the_last_projects_parent_folder() {
    let dir = project("clean").unwrap();
    let app = open(&dir);
    app.wait_for("the tests found", WAIT, |t| row(t, "tests").is_some())
        .unwrap();
    app.answer_folder(None).unwrap();
    app.press_named(Role::Button, "Open…").unwrap();
    assert_eq!(
        first_dialog(&app),
        Some(dir.parent().unwrap().to_string_lossy().to_string())
    );
}

/// UP-10, E-open-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn open_with_no_last_project() {
    let app = App::launch().unwrap();
    app.wait_for("the home screen", WAIT, |t| {
        t.find(&Role::Button, "Open folder…").is_some()
    })
    .unwrap();
    app.answer_folder(None).unwrap();
    app.press_named(Role::Button, "Open folder…").unwrap();
    assert_eq!(first_dialog(&app), None);
}

/// UP-11, E-open-11
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_last_project_folder_was_deleted() {
    let dir = project("clean").unwrap();
    let app = open(&dir);
    app.wait_for("the tests found", WAIT, |t| row(t, "tests").is_some())
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let app = app.relaunch(&[]).unwrap();
    app.wait_for("the home screen", WAIT, |t| {
        t.find(&Role::Button, "Open folder…").is_some()
    })
    .unwrap();
    app.answer_folder(None).unwrap();
    app.press_named(Role::Button, "Open folder…").unwrap();
    assert_eq!(first_dialog(&app), None);
}

/// The toolbar's buttons, in their order (`TOOLBAR_BUTTONS`; Watch comes with phase P4).
const TOOLBAR: [&str; 8] = [
    "Home",
    "Properties…",
    "Refresh",
    "Open…",
    "Report",
    "Re-run failed",
    "Cancel",
    "Run all",
];

/// Where Grace's view is: its root, named Grace inside the window (the window itself has no place through AT-SPI).
fn view(tree: &Node) -> Option<grace_e2e::Rect> {
    tree.all()
        .into_iter()
        .find(|n| n.name == "Grace" && n.role != Role::Window)
        .and_then(|n| n.rect)
        .filter(|r| r.width > 0)
}

/// Sizes the window and waits until the tree shows it at that width, give or take Windows' frame, and `ok`.
fn window_width(app: &App, width: i32, ok: impl Fn(&Node) -> bool) -> Node {
    app.set_window_rect(0, 0, width, 700).unwrap();
    app.wait_for(&format!("the window {width} px wide"), WAIT, |t| {
        view(t).is_some_and(|r| (r.width - width).abs() <= 40) && ok(t)
    })
    .unwrap()
}

fn button_width(tree: &Node, name: &str) -> Option<i32> {
    tree.find(&Role::Button, name).and_then(|b| b.rect).map(|r| r.width)
}

/// Every toolbar button is named and inside the window (`expectToolbarFits`).
fn toolbar_fits(tree: &Node) {
    let window = view(tree).expect("the view's place");
    for name in TOOLBAR {
        let b = tree
            .find(&Role::Button, name)
            .unwrap_or_else(|| panic!("{name}: {}", tree.dump()));
        let r = b.rect.unwrap_or_else(|| panic!("{name} has no place"));
        assert!(
            r.x >= window.x && r.x + r.width <= window.x + window.width,
            "{name} at {r:?} inside the window {window:?}"
        );
    }
}

/// E-open-03: below 1280 px four buttons are their icon, with the name spoken; at the floor every button
/// is still in the window.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_narrow_window_keeps_every_toolbar_button_on_screen_with_its_name() {
    let dir = project("clean").unwrap();
    let app = open(&dir);
    app.wait_for("the tests found", WAIT, |t| row(t, "tests").is_some())
        .unwrap();
    for width in [780, 1200, 1279] {
        let tree = window_width(&app, width, |t| button_width(t, "Open…").is_some_and(|w| w < 40));
        toolbar_fits(&tree);
        for name in ["Properties…", "Open…", "Report", "Cancel"] {
            let w = button_width(&tree, name).unwrap();
            assert!(w < 40, "{name} is its icon at {width}: {w} px");
        }
        let name = tree.find(&Role::Text, "clean").and_then(|n| n.rect);
        assert!(
            name.is_some_and(|r| r.width > 25),
            "the folder's name at {width}: {}",
            tree.dump()
        );
    }
    for width in [1281, 1500] {
        let tree = window_width(&app, width, |t| button_width(t, "Open…").is_some_and(|w| w > 60));
        toolbar_fits(&tree);
    }
    // the floor: only a window manager holds the minimum width, and Xvfb has none
    if cfg!(windows) {
        app.set_window_rect(0, 0, 1, 700).unwrap();
        let tree = app
            .wait_for("the window at its minimum", WAIT, |t| {
                view(t).is_some_and(|r| r.width >= 720 && r.width < 780)
            })
            .unwrap();
        toolbar_fits(&tree);
    }
}
