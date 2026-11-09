//! The tree through the accessibility tree: `e2e/virtual.spec.ts`'s rows in view and keys, and the rules of which
//! rows start open (TO-*). The scroll checks of `virtual.spec.ts` that read the DOM's pixels (a row's `top`, the
//! scroller's `scrollTop`) are the headless tests' in `grace::tree`.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::PathBuf;
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);

/// The tree item whose name starts with `label` and a comma: its name goes on with its status and tags.
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn rows(tree: &Node) -> Vec<String> {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::TreeItem)
        .map(|n| n.name.clone())
        .collect()
}

/// The tree item that has the focus.
fn focused(tree: &Node) -> Option<String> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.states.focused)
        .map(|n| n.name.clone())
}

fn open(dir: &std::path::Path) -> App {
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tree", WAIT, |t| {
        t.find(&Role::Tree, "Tests").is_some() && !rows(t).is_empty()
    })
    .unwrap();
    app
}

fn open_clean() -> App {
    open(&project("clean").unwrap())
}

/// TO-01, TO-05, TO-06: a project opens with its features closed; a click on one opens and selects it, and a second
/// click leaves it open.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn features_start_closed_and_a_click_opens_one() {
    let app = open_clean();
    let tree = app.tree().unwrap();
    assert_eq!(
        rows(&tree),
        [
            "tests, not run",
            "basic.feature, not run",
            "outline.feature, not run",
            "slow.feature, not run"
        ],
        "{}",
        tree.dump()
    );
    for _ in 0..2 {
        let basic = row(&app.tree().unwrap(), "basic.feature").unwrap().clone();
        app.press(&basic).unwrap();
        app.wait_for("basic.feature open and selected", WAIT, |t| {
            row(t, "passing hello").is_some() && row(t, "basic.feature").is_some_and(|r| r.states.selected)
        })
        .unwrap();
    }
}

/// TO-07: the chevron still closes a feature.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_chevron_opens_and_closes_a_feature() {
    let app = open_clean();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    app.wait_for("basic.feature open", WAIT, |t| row(t, "passing hello").is_some())
        .unwrap();
    app.press_named(Role::Button, "Collapse basic.feature").unwrap();
    app.wait_for("basic.feature closed", WAIT, |t| {
        row(t, "passing hello").is_none() && t.find(&Role::Button, "Expand basic.feature").is_some()
    })
    .unwrap();
}

/// VL-06 to VL-09, TO-08: the tree is one tab stop; the arrows move and select, Right and Left open, close and move
/// to a child or the parent, Home and End go to the ends; moving never opens a feature.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_keys_move_through_the_rows() {
    let app = open_clean();
    let tests = row(&app.tree().unwrap(), "tests").unwrap().clone();
    app.press(&tests).unwrap();
    let at = |name: &str| {
        let name = name.to_owned();
        app.wait_for(&format!("the focus on {name}"), WAIT, move |t| {
            focused(t).is_some_and(|f| f.starts_with(&format!("{name}, ")))
                && row(t, &name).is_some_and(|r| r.states.selected)
        })
        .unwrap()
    };
    // a press selects; the window has the keyboard focus only from the first key on (no window manager here)
    app.wait_for("tests selected", WAIT, |t| {
        row(t, "tests").is_some_and(|r| r.states.selected)
    })
    .unwrap();
    app.key(Key::Down).unwrap();
    let tree = at("basic.feature");
    assert!(
        row(&tree, "passing hello").is_none(),
        "moving opens nothing: {}",
        tree.dump()
    );
    app.key(Key::Right).unwrap();
    app.wait_for("basic.feature open", WAIT, |t| row(t, "passing hello").is_some())
        .unwrap();
    app.key(Key::Right).unwrap();
    at("passing hello");
    app.key(Key::Left).unwrap();
    at("basic.feature");
    app.key(Key::Left).unwrap();
    app.wait_for("basic.feature closed", WAIT, |t| row(t, "passing hello").is_none())
        .unwrap();
    app.key(Key::End).unwrap();
    at("slow.feature");
    app.key(Key::Up).unwrap();
    at("outline.feature");
    app.key(Key::Home).unwrap();
    at("tests");
}

/// VL-10, TO-10: Ctrl+Enter runs the focused row, and a run of one feature opens it as it starts.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn ctrl_enter_runs_the_focused_row() {
    let app = open_clean();
    let outline = row(&app.tree().unwrap(), "outline.feature").unwrap().clone();
    // a press on the row opens it; close it again, so the run is what opens it
    app.press(&outline).unwrap();
    app.wait_for("outline.feature open", WAIT, |t| row(t, "add <a> and <b>").is_some())
        .unwrap();
    app.key(Key::Left).unwrap();
    app.wait_for("outline.feature closed", WAIT, |t| row(t, "add <a> and <b>").is_none())
        .unwrap();
    app.key(Key::CtrlEnter).unwrap();
    app.wait_for("the feature's run, its rows passed", Duration::from_secs(60), |t| {
        t.all().iter().any(|n| n.name == "Finished")
            && row(t, "outline.feature").is_some_and(|r| r.name.contains("passed"))
    })
    .unwrap();
    let tree = app.tree().unwrap();
    assert!(
        row(&tree, "add <a> and <b>").is_some(),
        "the run opened it: {}",
        tree.dump()
    );
}

/// A project of `features` features of one scenario each, in one folder.
fn many_features(features: usize) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grace-e2e-many-{}-{features}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    for i in 0..features {
        std::fs::write(
            dir.join("tests").join(format!("feature-{i:03}.feature")),
            format!("Feature: feature {i}\n\nScenario: scenario {i}\n  * def x = {i}\n"),
        )
        .unwrap();
    }
    dir
}

/// VL-01, VL-14: of 300 rows, only those in view are drawn, and End brings the last into view.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn only_the_rows_in_view_are_drawn() {
    let dir = many_features(300);
    let app = open(&dir);
    let tree = app
        .wait_for("the features", Duration::from_secs(60), |t| {
            row(t, "feature-000.feature").is_some()
        })
        .unwrap();
    let drawn = rows(&tree).len();
    assert!((5..100).contains(&drawn), "{drawn} rows drawn: {}", tree.dump());
    assert!(row(&tree, "feature-299.feature").is_none());
    app.press(row(&tree, "tests").unwrap()).unwrap();
    app.key(Key::End).unwrap();
    let tree = app
        .wait_for("the last row in view and focused", WAIT, |t| {
            focused(t).is_some_and(|f| f.starts_with("feature-299.feature, "))
        })
        .unwrap();
    assert!(rows(&tree).len() < 100, "{}", tree.dump());
    assert!(row(&tree, "feature-000.feature").is_none(), "scrolled: {}", tree.dump());
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}
