//! `e2e/home.spec.ts` (home screen spec), through the accessibility tree. The tests that need the toolbar's Home
//! button or the tree port with them (P2 plan, PR 2 and PR 3). They start the real app, so they are ignored by
//! `cargo test --workspace`; `scripts/e2e-gpui.sh` builds Grace with the `e2e` feature and runs them.
use std::path::Path;
use std::time::Duration;

use grace_e2e::{App, Node, Role, project};
use serde_json::json;

const HEADING: &str = "Open a Karate project";
const OPEN: &str = "Open folder…";

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

fn home(app: &App) -> Node {
    app.wait_for("the home screen", Duration::from_secs(15), |t| {
        named(t, HEADING).is_some()
    })
    .unwrap()
}

/// A recent folder's row: its button, named by the folder's name, then its path.
fn row<'a>(tree: &'a Node, path: &Path) -> Option<&'a Node> {
    let path = path.to_string_lossy();
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.ends_with(path.as_ref()))
}

/// The project view shows the folder's path.
fn project_shown(app: &App, dir: &Path) -> Node {
    let path = dir.to_string_lossy().to_string();
    app.wait_for("the project view", Duration::from_secs(20), |t| {
        named(t, &path).is_some() && named(t, HEADING).is_none()
    })
    .unwrap()
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_first_start_shows_the_home_screen() {
    let app = App::launch().unwrap();
    let tree = home(&app);
    assert!(
        named(
            &tree,
            "Pick the folder that holds your .feature files. Grace brings its own Java and Karate."
        )
        .is_some(),
        "{}",
        tree.dump()
    );
    assert!(tree.find(&Role::Button, OPEN).is_some(), "{}", tree.dump());
    assert!(named(&tree, "Recent folders").is_none());
    assert!(tree.find(&Role::List, "Recent folders").is_none());
    // the mascot is decorative: nothing in the tree but the controls and the words
    assert!(tree.all().iter().all(|n| n.role != Role::Image), "{}", tree.dump());
    assert_eq!(tree.unnamed_controls(), Vec::<String>::new(), "{}", tree.dump());
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn opening_a_folder_from_the_home_screen() {
    let dir = project("clean").unwrap();
    let app = App::launch().unwrap();
    let tree = home(&app);
    app.answer_folder(Some(&dir)).unwrap();
    app.press(tree.find(&Role::Button, OPEN).unwrap()).unwrap();
    project_shown(&app, &dir);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_last_project_opens_at_start_without_the_home_screen() {
    let dir = project("clean").unwrap();
    let app = App::launch().unwrap();
    app.answer_folder(Some(&dir)).unwrap();
    app.press(home(&app).find(&Role::Button, OPEN).unwrap()).unwrap();
    project_shown(&app, &dir);
    app.wait_for("the last project saved", Duration::from_secs(10), |_| {
        app.settings()["lastProject"] == json!(dir.to_string_lossy())
    })
    .unwrap();

    let app = app.relaunch(&[]).unwrap();
    // the home screen never shows on the way: the window waits for the runner's word (HS-05)
    let start = std::time::Instant::now();
    let path = dir.to_string_lossy().to_string();
    while start.elapsed() < Duration::from_secs(20) {
        let Ok(tree) = app.tree() else { continue };
        assert!(named(&tree, HEADING).is_none(), "home showed: {}", tree.dump());
        if named(&tree, &path).is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("the last project did not open");
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_missing_last_project_shows_the_home_screen() {
    let gone = std::env::temp_dir().join(format!("grace-e2e-gone-{}", std::process::id()));
    let gone_text = gone.to_string_lossy().to_string();
    let app =
        App::launch_with_settings(&json!({ "lastProject": gone_text, "recentProjects": [gone_text] }), &[]).unwrap();
    let tree = app
        .wait_for("the missing row", Duration::from_secs(15), |t| row(t, &gone).is_some())
        .unwrap();
    let r = row(&tree, &gone).unwrap();
    assert!(r.name.contains("Not found"), "{}", r.name);
    assert!(
        named(&tree, "Pick a folder, or reopen one you used before.").is_some(),
        "{}",
        tree.dump()
    );
    // a disabled row takes no press
    let _ = app.press(r);
    std::thread::sleep(Duration::from_millis(500));
    let tree = home(&app);
    assert!(named(&tree, &gone_text).is_none() || row(&tree, &gone).is_some());
    assert!(
        tree.all().iter().all(|n| n.role != Role::Status || n.name.is_empty()),
        "{}",
        tree.dump()
    );
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_folder_dialog_starts_next_to_the_newest_existing_folder() {
    let base = std::env::temp_dir().join(format!("grace-e2e-recent-{}", std::process::id()));
    let gone = base.join("one").join("gone");
    let a = base.join("two").join("a");
    let b = base.join("three").join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let list = [&gone, &a, &b].map(|p| p.to_string_lossy().to_string());
    let app = App::launch_with_settings(&json!({ "recentProjects": list }), &[]).unwrap();
    let tree = app
        .wait_for("three rows", Duration::from_secs(15), |t| row(t, &b).is_some())
        .unwrap();
    app.answer_folder(None).unwrap();
    app.press(tree.find(&Role::Button, OPEN).unwrap()).unwrap();
    let start = std::time::Instant::now();
    while app.folder_dialogs().is_empty() && start.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(
        app.folder_dialogs(),
        vec![Some(a.parent().unwrap().to_string_lossy().to_string())]
    );
    let _ = std::fs::remove_dir_all(base);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn removing_a_row() {
    let dir = project("clean").unwrap();
    let clean = dir.to_string_lossy().to_string();
    let app = App::launch_with_settings(
        &json!({ "recentProjects": [clean], "projects": { clean.clone(): { "threads": 2 } } }),
        &[],
    )
    .unwrap();
    let remove = app.wait_for_node(Role::Button, "Remove from list").unwrap();
    if let Some(r) = remove.rect {
        assert!(r.width >= 24 && r.height >= 24, "R10.3: {r:?}");
    }
    app.press(&remove).unwrap();
    app.wait_for("no list", Duration::from_secs(10), |t| row(t, &dir).is_none())
        .unwrap();
    app.wait_for("the list saved", Duration::from_secs(10), |_| {
        app.settings()["recentProjects"] == json!([])
    })
    .unwrap();
    assert_eq!(app.settings()["projects"][&clean]["threads"], json!(2));
    assert!(dir.exists());
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn reopening_a_folder_from_the_list() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "recentProjects": [dir.to_string_lossy()] }), &[]).unwrap();
    let tree = app
        .wait_for("the row", Duration::from_secs(15), |t| row(t, &dir).is_some())
        .unwrap();
    app.press(row(&tree, &dir).unwrap()).unwrap();
    project_shown(&app, &dir);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_second_start_on_the_same_data_folder_hands_over_to_the_first() {
    let app = App::launch().unwrap();
    home(&app);
    let second = std::process::Command::new(grace_e2e::grace_exe())
        .env("GRACE_USER_DATA", app.data_dir())
        .status()
        .unwrap();
    assert!(second.success(), "the second Grace quits on its own");
    home(&app); // the first is still there
}

/// HS-10, E-home-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_dry_run_error_still_opens_the_project() {
    let dir = project("broken").unwrap();
    let app = App::launch().unwrap();
    app.answer_folder(Some(&dir)).unwrap();
    app.press(home(&app).find(&Role::Button, OPEN).unwrap()).unwrap();
    let path = dir.to_string_lossy().to_string();
    app.wait_for("the error and the project's path", Duration::from_secs(30), |t| {
        named(t, &path).is_some() && t.all().iter().any(|n| n.name.contains("syntax.feature"))
    })
    .unwrap();
    app.press_named(Role::Button, "Home").unwrap();
    app.wait_for("home, the project first in the list", Duration::from_secs(15), |t| {
        named(t, HEADING).is_some() && row(t, &dir).is_some()
    })
    .unwrap();
}

/// HS-14, E-home-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_list_works_from_the_keyboard() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "recentProjects": [dir.to_string_lossy()] }), &[]).unwrap();
    app.wait_for("the row", Duration::from_secs(15), |t| row(t, &dir).is_some())
        .unwrap();
    let on_row = |t: &Node| row(t, &dir).is_some_and(|r| r.states.focused);
    let mut tabs = 0;
    while !app.tree().is_ok_and(|t| on_row(&t)) {
        assert!(tabs < 5, "Tab never reached the row: {}", app.tree().unwrap().dump());
        app.key(grace_e2e::Key::Tab).unwrap();
        tabs += 1;
        let _ = app.wait_for("the focus to move", Duration::from_secs(2), |t| on_row(t));
    }
    app.key(grace_e2e::Key::Enter).unwrap();
    project_shown(&app, &dir);
}

/// HS-16, E-home-11
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn home_closes_the_project_and_survives_a_restart() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    project_shown(&app, &dir);
    app.press_named(Role::Button, "Home").unwrap();
    app.wait_for("home, the project in the list", Duration::from_secs(15), |t| {
        named(t, HEADING).is_some() && row(t, &dir).is_some()
    })
    .unwrap();
    app.wait_until("lastProject forgotten", Duration::from_secs(10), || {
        app.settings().get("lastProject").is_none()
    })
    .unwrap();

    let app = app.relaunch(&[]).unwrap();
    let tree = app
        .wait_for("home again, the project in the list", Duration::from_secs(15), |t| {
            named(t, HEADING).is_some() && row(t, &dir).is_some()
        })
        .unwrap();
    assert!(
        tree.find(&Role::Button, "Run all").is_none(),
        "home, not the project: {}",
        tree.dump()
    );
}

/// HS-17, E-home-12
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn home_is_disabled_during_a_run() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    project_shown(&app, &dir);
    app.wait_for("slow.feature", Duration::from_secs(30), |t| {
        t.find(&Role::Button, "Run slow.feature").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Run slow.feature").unwrap();
    app.wait_for("the run going", Duration::from_secs(30), |t| {
        named(t, "Running…").is_some()
    })
    .unwrap();
    // a press on Home does nothing while the run goes
    let _ = app.press_named(Role::Button, "Home");
    std::thread::sleep(Duration::from_millis(1000));
    let tree = app.tree().unwrap();
    assert!(named(&tree, HEADING).is_none(), "home during a run: {}", tree.dump());

    app.wait_for("the run finished", Duration::from_secs(60), |t| {
        named(t, "Finished").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Home").unwrap();
    home(&app);
    app.wait_until("no Java", Duration::from_secs(15), || app.java_pids().is_empty())
        .unwrap();
}
