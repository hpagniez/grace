//! `e2e/run.spec.ts` and `e2e/cancel.spec.ts`, through the accessibility tree: Run all, Cancel, Report and closing
//! during a run, on the real runner and Java. The parts that look at the tree's rows (a scenario turning running,
//! passed or failed; a feature or one row run from its own button) come with the tree, in P2's third PR (P2 plan).
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use grace_e2e::{App, Node, Role, project};
use serde_json::json;

const RUN: Duration = Duration::from_secs(60);

fn label<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

fn bar(tree: &Node) -> Option<&Node> {
    tree.find(&Role::ProgressBar, "Run progress")
}

/// Grace on the clean sample project, its 9 tests found.
fn open_clean() -> (std::path::PathBuf, App) {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", Duration::from_secs(30), |t| {
        label(t, "9 tests · Not run yet").is_some()
    })
    .unwrap();
    (dir, app)
}

fn run_all(app: &App) {
    app.press_named(Role::Button, "Run all").unwrap();
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn run_all_with_live_progress() {
    let (_, app) = open_clean();
    run_all(&app);
    // the bar waits, with no values, until the first result (LV-05, LV-09)
    let tree = app
        .wait_for("0 / 9", Duration::from_secs(15), |t| label(t, "0 / 9").is_some())
        .unwrap();
    assert_eq!(bar(&tree).unwrap().value, None, "{}", tree.dump());
    app.wait_for("Running…", Duration::from_secs(15), |t| {
        label(t, "Running…").is_some()
    })
    .unwrap();
    // as Playwright's expect, which retries: the driver reads the tree node by node, so one read can hold a
    // node from before an update and one from after it
    app.wait_for("Finished with 8 passed and 1 failed", RUN, |t| {
        label(t, "Finished").is_some()
            && label(t, "9 / 9").is_some()
            && label(t, "8 passed").is_some()
            && label(t, "1 failed").is_some()
            && bar(t).and_then(|b| b.value) == Some(9.)
    })
    .unwrap();
}

/// Every file of the project with its contents' size and bytes, folders as such.
fn snapshot(dir: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
        for e in std::fs::read_dir(at).unwrap().flatten() {
            let path = e.path();
            let rel = path.strip_prefix(root).unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                out.insert(rel, None);
                walk(root, &path, out);
            } else {
                out.insert(rel, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_project_is_not_modified() {
    let (dir, app) = open_clean();
    let before = snapshot(&dir);
    run_all(&app);
    app.wait_for("Finished", RUN, |t| label(t, "Finished").is_some())
        .unwrap();
    assert!(snapshot(&dir) == before, "the run changed the project");
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn report_opens_the_runs_html_report() {
    let (_, app) = open_clean();
    run_all(&app);
    app.wait_for("Finished", RUN, |t| label(t, "Finished").is_some())
        .unwrap();
    app.press_named(Role::Button, "Report").unwrap();
    app.wait_until("the report opened", Duration::from_secs(10), || {
        !app.opened_paths().is_empty()
    })
    .unwrap();
    let opened = app.opened_paths();
    assert!(opened[0].ends_with("karate-summary.html"), "{opened:?}");
    assert!(Path::new(&opened[0]).is_file(), "{opened:?}");
}

/// Run all, until Java runs and the first test has not finished: the slow feature holds the run.
fn start_a_run() -> App {
    let (_, app) = open_clean();
    run_all(&app);
    app.wait_for("Running…", Duration::from_secs(15), |t| {
        label(t, "Running…").is_some()
    })
    .unwrap();
    app.wait_until("Java", Duration::from_secs(30), || !app.java_pids().is_empty())
        .unwrap();
    app
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn cancel_leaves_nothing_behind() {
    let app = start_a_run();
    app.press_named(Role::Button, "Cancel").unwrap();
    app.wait_for("Cancelled", Duration::from_secs(15), |t| {
        label(t, "Cancelled").is_some()
    })
    .unwrap();
    app.wait_until("no Java", Duration::from_secs(15), || app.java_pids().is_empty())
        .unwrap();
    // Report is disabled: a press opens nothing (AT-SPI on Linux does not carry a button's disabled state)
    let _ = app.press_named(Role::Button, "Report");
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(app.opened_paths(), Vec::<String>::new());
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn closing_the_app_during_a_run() {
    let mut app = start_a_run();
    let java = app.java_pids();
    assert!(!java.is_empty());
    app.close().unwrap();
    assert!(app.exited(Duration::from_secs(20)), "Grace quits on close");
    app.wait_until("no Java", Duration::from_secs(15), || app.java_pids().is_empty())
        .unwrap();
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn nothing_to_run() {
    let dir = std::env::temp_dir().join(format!("grace-e2e-empty-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    let tree = app
        .wait_for("0 tests", Duration::from_secs(30), |t| {
            label(t, "0 tests · Not run yet").is_some()
        })
        .unwrap();
    assert!(tree.find(&Role::Button, "Run all").is_some(), "{}", tree.dump());
    // Run all is disabled: a press starts nothing
    let _ = app.press_named(Role::Button, "Run all");
    std::thread::sleep(Duration::from_millis(1000));
    let tree = app.tree().unwrap();
    assert!(label(&tree, "Running…").is_none(), "{}", tree.dump());
    assert!(label(&tree, "0 tests · Not run yet").is_some(), "{}", tree.dump());
    let _ = std::fs::remove_dir_all(dir);
}
