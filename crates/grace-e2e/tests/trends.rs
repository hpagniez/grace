//! `e2e/trends.spec.ts` (E-trends-01 to E-trends-03) through the accessibility tree: the strip at the top of the History
//! tab, on runs seeded into the project's history before Grace opens it.
//!
//! The strip is a group named "Trend of <what it covers>"; its caption and its two labels are text nodes inside it and
//! each column is a button named by its run, which the group holds. A row of the history is a list item of "Run history".
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::Path;
use std::time::Duration;

use grace_e2e::history::{Seed, seed_runs};
use grace_e2e::{App, Node, Role, fresh_data_dir, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const LIST: &str = "Run history";

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

/// A tree row, "<label>, <status>".
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

/// The History tab, whose name carries its count.
fn tab(tree: &Node) -> Option<&Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Tab && n.name.starts_with("History"))
}

/// The strip, once it covers `what`.
fn strip<'a>(tree: &'a Node, what: &str) -> Option<&'a Node> {
    tree.find(&Role::Group, &format!("Trend of {what}"))
}

/// The columns of the strip, oldest first.
fn columns(strip: &Node) -> Vec<&Node> {
    strip
        .all()
        .into_iter()
        .skip(1)
        .filter(|n| n.role == Role::Button)
        .collect()
}

/// Whether the strip has a text node saying `text`: its caption or one of its labels.
fn says(strip: &Node, text: &str) -> bool {
    strip
        .all()
        .into_iter()
        .skip(1)
        .any(|n| n.role == Role::Text && n.name == text)
}

/// The rows of the history, newest first.
fn rows(tree: &Node) -> Vec<&Node> {
    tree.find(&Role::List, LIST)
        .map(|l| l.all().into_iter().filter(|n| n.role == Role::ListItem).collect())
        .unwrap_or_default()
}

/// Waits until the project is ready: its tests found and the window no longer busy. Every control is off, and a press
/// of one returns Ok and does nothing, while the opening discovery lasts (see `tests/history.rs`).
fn wait_ready(app: &App) {
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
    app.wait_for("the History tab", WAIT, |t| tab(t).is_some()).unwrap();
    // "Ready." can show a moment before every control is on again, and a press of an off control does nothing; the
    // button is read on Windows only (the pinned AT-SPI adapter drops a disabled state)
    #[cfg(windows)]
    app.wait_for("Run all on", WAIT, |t| {
        t.find(&Role::Button, "Run all").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
}

/// Shows the History tab, and waits until it does.
fn show_history(app: &App) {
    // a press can meet a tree that changed under it (AT-SPI: UnknownObject), and every read is of the final one
    let t = app.wait_for("the History tab", WAIT, |t| tab(t).is_some()).unwrap();
    app.press(tab(&t).unwrap()).unwrap();
    app.wait_for("the History tab shown", WAIT, |t| {
        tab(t).is_some_and(|n| n.states.selected)
    })
    .unwrap();
}

/// Grace on `dir` with the runs of `seeds` in its history already, as `(count, seed)`, the History tab shown.
fn open(dir: &Path, seeds: &[(usize, Seed)]) -> (App, Vec<String>) {
    let data = fresh_data_dir().unwrap();
    let ids = seeds
        .iter()
        .flat_map(|(n, seed)| seed_runs(&data, dir, *n, seed))
        .collect();
    std::fs::write(
        data.join("settings.json"),
        json!({ "lastProject": dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    wait_ready(&app);
    show_history(&app);
    (app, ids)
}

/// E-trends-01, TR-02, TR-03: the strip covers every test, then the one selected in the tree.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_strip_follows_the_tree_selection() {
    let dir = project("clean").unwrap();
    let (app, _) = open(&dir, &[(3, Seed::default())]);
    app.wait_for("the strip of the whole suite", WAIT, |t| {
        strip(t, "All tests").is_some_and(|s| {
            columns(s).len() == 3 && says(s, "All tests") && says(s, "Pass rate 50%") && says(s, "Time per test 350 ms")
        })
    })
    .unwrap();
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    let t = app
        .wait_for("passing hello", WAIT, |t| row(t, "passing hello").is_some())
        .unwrap();
    app.click(row(&t, "passing hello").unwrap()).unwrap();
    let t = app
        .wait_for("the strip of passing hello", WAIT, |t| {
            strip(t, "passing hello").is_some_and(|s| {
                columns(s).len() == 3
                    && says(s, "passing hello")
                    && says(s, "Pass rate Passed")
                    && says(s, "Time per test 400 ms")
            })
        })
        .unwrap();
    assert!(
        strip(&t, "All tests").is_none(),
        "one strip, for the selection: {}",
        t.dump()
    );
}

/// E-trends-02, TR-15: a click on a column shows its run's row in the table.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_column_finds_its_run_in_the_table() {
    let dir = project("clean").unwrap();
    let (app, _) = open(&dir, &[(20, Seed::default())]);
    let t = app
        .wait_for("the strip", WAIT, |t| {
            strip(t, "All tests").is_some_and(|s| columns(s).len() == 20) && rows(t).len() == 20
        })
        .unwrap();
    // the panel holds from its tab bar to the status bar, whose Settings button is as tall as the bar: the oldest run's
    // row is below it at the default height
    let bottom = |t: &Node| {
        t.find(&Role::Button, "Settings")
            .and_then(|n| n.rect)
            .map(|r| r.y)
            .expect("the status bar's Settings button")
    };
    let top = |t: &Node| {
        tab(t)
            .and_then(|n| n.rect)
            .map(|r| r.y + r.height)
            .expect("the tab bar")
    };
    let in_view = |t: &Node| {
        let oldest = rows(t).last().and_then(|r| r.rect).expect("the oldest run's row");
        oldest.y >= top(t) && oldest.y + oldest.height <= bottom(t)
    };
    assert!(
        !in_view(&t),
        "below the fold at the panel's default height: {}",
        t.dump()
    );
    let oldest = columns(strip(&t, "All tests").unwrap())[0];
    app.click(oldest).unwrap();
    app.wait_for("the oldest run's row in view", WAIT, in_view)
        .unwrap_or_else(|e| panic!("{e}"));
}

/// E-trends-03, TR-06: a run on another environment has no column.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_run_on_another_environment_has_no_column() {
    let dir = project("clean").unwrap();
    let on_dev = Seed {
        ago: 10,
        record: json!({ "env": "dev" }),
        ..Seed::default()
    };
    let (app, _) = open(&dir, &[(2, on_dev), (1, Seed::default())]);
    app.wait_for("three runs, one column", WAIT, |t| {
        rows(t).len() == 3 && strip(t, "All tests").is_some_and(|s| columns(s).len() == 1)
    })
    .unwrap();
}
