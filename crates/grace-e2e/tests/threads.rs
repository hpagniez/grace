//! `e2e/threads.spec.ts` (TC-01 to TC-06, TC-09, TC-10), through the accessibility tree. The box is a spin button
//! named "Threads". Karate's own count for a run is read from the run's events (`ranThreads`); the History tab, which
//! showed it too, comes with P2's fifth PR.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::{Value, json};

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);
const THREADS: &str = "Threads";

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

/// Grace on the clean sample project, `pom` written to its karate-pom.json first, its features open.
fn open_clean(pom: Option<Value>) -> (PathBuf, App) {
    let dir = project("clean").unwrap();
    if let Some(pom) = pom {
        std::fs::write(dir.join("karate-pom.json"), pom.to_string()).unwrap();
    }
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    open_features(&app);
    (dir, app)
}

fn open_features(app: &App) {
    app.wait_for("the tests found", WAIT, |t| {
        t.find(&Role::Button, "Expand basic.feature").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    app.wait_for("basic.feature open", WAIT, |t| row(t, "passing hello").is_some())
        .unwrap();
}

/// The runs Grace stored for the project, newest first: each run's folder holds its events.
fn runs(app: &App) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.file_name().is_some_and(|n| n == "run.json") {
                out.push(p.parent().unwrap().to_path_buf());
            }
        }
    }
    let mut all = Vec::new();
    walk(app.data_dir(), &mut all);
    all.sort();
    all.reverse(); // a run's folder is named after its start time
    all
}

/// The thread count Karate reports for a run (`ranThreads`): its suite's start event.
fn ran_threads(run: &Path) -> u64 {
    let events = std::fs::read_to_string(run.join("karate-json").join("karate-events.jsonl")).unwrap();
    let suite = events.lines().find(|l| l.contains("\"SUITE_ENTER\"")).unwrap();
    serde_json::from_str::<Value>(suite).unwrap()["data"]["threads"]
        .as_u64()
        .unwrap()
}

/// Waits until `n` runs are stored: a stored run has ended.
fn finished(app: &App, n: usize) {
    app.wait_until(&format!("{n} runs stored"), RUN, || runs(app).len() >= n)
        .unwrap();
    app.wait_for("the run over", WAIT, |t| {
        t.find(&Role::Button, "Run all").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
}

fn run_all(app: &App, n: usize) {
    app.press_named(Role::Button, "Run all").unwrap();
    finished(app, n);
}

/// Types the count and presses Enter.
fn set_threads(app: &App, value: &str) {
    app.fill(Role::SpinButton, THREADS, value).unwrap();
    app.key(Key::Enter).unwrap();
}

fn saved_threads(app: &App, dir: &Path) -> Value {
    app.settings()["projects"][dir.to_string_lossy().as_ref()]["threads"].clone()
}

/// TC-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_project_with_no_saved_count_runs_with_1_thread() {
    let (_, app) = open_clean(None);
    app.wait_for_text(Role::SpinButton, THREADS, "1").unwrap();
    run_all(&app, 1);
    assert_eq!(ran_threads(&runs(&app)[0]), 1);
}

/// TC-02: no Enter, pressing Run all leaves the box, which commits the count
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn run_all_uses_the_count_in_the_box() {
    let (_, app) = open_clean(None);
    app.fill(Role::SpinButton, THREADS, "2").unwrap();
    let run_all = app.wait_for_node(Role::Button, "Run all").unwrap();
    app.click(&run_all).unwrap();
    finished(&app, 1);
    assert_eq!(ran_threads(&runs(&app)[0]), 2);
}

/// TC-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_single_tests_run_uses_the_count() {
    let (_, app) = open_clean(None);
    set_threads(&app, "2");
    app.press_named(Role::Button, "Run passing hello").unwrap();
    finished(&app, 1);
    assert_eq!(ran_threads(&runs(&app)[0]), 2);
}

/// TC-04
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn rerun_failed_uses_the_current_count() {
    let (_, app) = open_clean(None);
    set_threads(&app, "2");
    run_all(&app, 1);
    app.wait_for("failing match failed", WAIT, |t| says(t, "failing match", "failed"))
        .unwrap();
    set_threads(&app, "1");
    app.press_named(Role::Button, "Re-run failed").unwrap();
    finished(&app, 2);
    let runs = runs(&app);
    assert_eq!((ran_threads(&runs[0]), ran_threads(&runs[1])), (1, 2));
}

/// TC-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_box_starts_from_karate_pom_json_and_the_persons_count_wins() {
    let (_, app) = open_clean(Some(json!({ "threads": 3 })));
    app.wait_for_text(Role::SpinButton, THREADS, "3").unwrap();
    set_threads(&app, "1");
    run_all(&app, 1);
    assert_eq!(ran_threads(&runs(&app)[0]), 1);
}

/// TC-06. GPUI Kit's `Input` keeps a disabled box out of the accessibility tree's disabled state on both platforms (its
/// text element writes the node, and GPUI has no disabled setter), so this checks what the state does instead: the
/// focused box takes no step during the run, and takes one again after it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_box_is_disabled_while_a_run_is_going() {
    let (dir, app) = open_clean(None);
    app.wait_for_text(Role::SpinButton, THREADS, "1").unwrap();
    app.fill(Role::SpinButton, THREADS, "1").unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run going", WAIT, |t| {
        t.find(&Role::Button, "Cancel").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
    app.key(Key::Up).unwrap();
    finished(&app, 1);
    assert_eq!(
        saved_threads(&app, &dir),
        Value::Null,
        "the box took no change during the run"
    );
    app.fill(Role::SpinButton, THREADS, "1").unwrap();
    app.key(Key::Up).unwrap();
    app.wait_until("the box stepped after the run", WAIT, || {
        saved_threads(&app, &dir) == json!(2)
    })
    .unwrap();
}

/// TC-09
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_count_is_saved_per_project() {
    let (dir, app) = open_clean(None);
    set_threads(&app, "3");
    app.wait_until("3 saved", WAIT, || saved_threads(&app, &dir) == json!(3))
        .unwrap();

    let app = app.relaunch(&[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| {
        t.find(&Role::Button, "Expand basic.feature").is_some()
    })
    .unwrap();
    app.wait_for_text(Role::SpinButton, THREADS, "3").unwrap();

    // one step with the arrow keys is a change too
    let box_ = app.wait_for_node(Role::SpinButton, THREADS).unwrap();
    app.click(&box_).unwrap();
    app.key(Key::Up).unwrap();
    app.wait_for_text(Role::SpinButton, THREADS, "4").unwrap();
    app.wait_until("4 saved", WAIT, || saved_threads(&app, &dir) == json!(4))
        .unwrap();
}

/// TC-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_value_out_of_range_is_refused() {
    let (dir, app) = open_clean(None);
    set_threads(&app, "2");
    app.wait_until("2 saved", WAIT, || saved_threads(&app, &dir) == json!(2))
        .unwrap();
    for (i, value) in ["0", "33", "2.5", ""].into_iter().enumerate() {
        set_threads(&app, value);
        // the box takes the typed value and goes back to 2 only when the runner re-sends the count it holds
        app.wait_for_text(Role::SpinButton, THREADS, "2").unwrap();
        if i == 0 {
            app.wait_for("the notice", WAIT, |t| {
                t.all()
                    .iter()
                    .any(|n| n.name == "Threads must be a whole number from 1 to 32")
            })
            .unwrap();
        }
        assert_eq!(saved_threads(&app, &dir), json!(2), "{value:?}");
    }
}
