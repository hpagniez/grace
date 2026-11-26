//! `e2e/flaky.spec.ts` (E-flaky-01 to E-flaky-07) through the accessibility tree: a test that passed and failed with the
//! same project files and environment is marked flaky in the tree, and Details shows the runs that prove it.
//!
//! The mark of a test is an image named "Flaky: passed and failed with the same project files" (`Tree.test.tsx` 28) and
//! its row's name says "flaky" after its status; a feature's and a folder's row says "<n> flaky". Details shows one group
//! per environment, named "Flaky on qa" or "Flaky with no environment", holding a label with the counts and a list
//! "Proof runs" whose items are named "<when>, <result>, <threads>", each with a Report button.
//! E-flaky-01 to 03 run Karate for real on a copy of the flaky sample project, whose `coin.json` lies beside the project
//! folder, outside it, so changing it does not change the project's fingerprint; the others seed two runs of that
//! project into the history before Grace opens it. They start the real app, so they are ignored by
//! `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::history::{Seed, runs_dir, seed_runs};
use grace_e2e::{App, Key, Node, Role, fresh_data_dir, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(120);
const MARK: &str = "Flaky: passed and failed with the same project files";
const NONE: &str = "Flaky with no environment";

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

/// A tree row, "<label>, <status>[, flaky][, <tags>]".
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn says(tree: &Node, label: &str, status: &str) -> bool {
    row(tree, label).is_some_and(|r| r.name.starts_with(&format!("{label}, {status}")))
}

/// The History tab, whose name carries its count.
fn tab(tree: &Node) -> Option<&Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Tab && n.name.starts_with("History"))
}

fn tab_is(tree: &Node, name: &str) -> bool {
    tab(tree).is_some_and(|t| t.name == name)
}

/// How many tests show the flaky mark.
fn marks(tree: &Node) -> usize {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::Image && n.name == MARK)
        .count()
}

/// Whether the row of `label` says it holds `n` flaky tests (a feature or a folder) or is flaky itself (`n` None).
fn flaky_row(tree: &Node, label: &str, n: Option<u32>) -> bool {
    let words = n.map_or_else(|| "flaky".to_string(), |n| format!("{n} flaky"));
    row(tree, label).is_some_and(|r| r.name.split(", ").any(|part| part == words))
}

fn button<'a>(item: &'a Node, name: &str) -> &'a Node {
    item.all()
        .into_iter()
        .skip(1)
        .find(|n| n.role == Role::Button && n.name == name)
        .unwrap_or_else(|| panic!("{name} in {}", item.dump()))
}

/// The proof box of an environment.
fn proof<'a>(tree: &'a Node, heading: &str) -> Option<&'a Node> {
    tree.find(&Role::Group, heading)
}

/// The proof runs of a box, in order.
fn proof_runs(group: &Node) -> Vec<&Node> {
    group.all().into_iter().filter(|n| n.role == Role::ListItem).collect()
}

/// Whether the box has a label that says `text`.
fn says_in(group: &Node, text: &str) -> bool {
    group
        .all()
        .into_iter()
        .skip(1)
        .any(|n| n.role == Role::Text && n.name == text)
}

/// Waits until the project is ready: its tests found and the window no longer busy. Every control is off, and a press of
/// one returns Ok and does nothing, while the opening discovery lasts: "Ready." shows only when it is over and Run all
/// can be pressed (`banner_state`), before the first run writes to the console.
fn wait_ready(app: &App) {
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
    app.wait_for("the History tab", WAIT, |t| tab(t).is_some()).unwrap();
    wait_idle(app);
}

/// Waits until Run all can be pressed again, after a run or an environment switch (read on Windows only: the pinned
/// AT-SPI adapter drops a disabled state).
// shortcut: no wait on Linux, so a press right after the switch can be lost there; replace it if one is
fn wait_idle(app: &App) {
    #[cfg(windows)]
    app.wait_for("Run all on", WAIT, |t| {
        t.find(&Role::Button, "Run all").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
    #[cfg(not(windows))]
    let _ = app;
}

fn expand(app: &App, feature: &str) {
    app.press_named(Role::Button, &format!("Expand {feature}")).unwrap();
}

/// Selects a test in the tree, which shows it in Details.
fn select(app: &App, name: &str) {
    let t = app.wait_for(name, WAIT, |t| row(t, name).is_some()).unwrap();
    app.click(row(&t, name).unwrap()).unwrap();
}

fn show_history(app: &App) {
    let t = app.wait_for("the History tab", WAIT, |t| tab(t).is_some()).unwrap();
    app.press(tab(&t).unwrap()).unwrap();
    app.wait_for("the History tab shown", WAIT, |t| {
        tab(t).is_some_and(|n| n.states.selected)
    })
    .unwrap();
}

/// Waits until the run is in the history (`n` runs): a stored run has ended, and the marks of its history follow the
/// count in the same batch of events.
fn finished(app: &App, n: usize) {
    let name = format!("History ({n})");
    app.wait_for(&name, RUN, |t| tab_is(t, &name)).unwrap();
    wait_idle(app);
}

/// Types the count, presses Enter and waits until Grace has saved it.
fn set_threads(app: &App, dir: &Path, n: u32) {
    app.fill(Role::SpinButton, "Threads", &n.to_string()).unwrap();
    app.key(Key::Enter).unwrap();
    app.wait_until("the thread count saved", WAIT, || {
        app.settings()["projects"][dir.to_string_lossy().as_ref()]["threads"] == json!(n)
    })
    .unwrap();
}

/// `coin.json`, beside the project folder: what `tests/coin.feature` reads.
fn coin(dir: &Path, heads: bool) {
    std::fs::write(
        dir.parent().unwrap().join("coin.json"),
        json!({ "heads": heads }).to_string(),
    )
    .unwrap();
}

/// Grace on the flaky sample project, `coin.json` holding `heads`, `tests/coin.feature` expanded.
fn open_coin(heads: bool) -> (App, PathBuf) {
    let dir = project("flaky").unwrap();
    coin(&dir, heads);
    let data = fresh_data_dir().unwrap();
    std::fs::write(
        data.join("settings.json"),
        json!({ "lastProject": dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    wait_ready(&app);
    expand(&app, "coin.feature");
    app.wait_for("the tests", WAIT, |t| {
        row(t, "coin").is_some() && row(t, "steady").is_some()
    })
    .unwrap();
    (app, dir)
}

/// Two runs of the flaky sample project, as a run stores them: `coin` failed on 2 threads, then passed on 1, `steady`
/// passed in both, the same fingerprint and no environment. Returns the ids, the failed one first.
fn seed_coin(data: &Path, dir: &Path, report: bool) -> Vec<String> {
    let tests = |coin: &str| {
        json!([
            { "kind": "scenario", "file": "tests/coin.feature", "refId": "[1:3]", "name": "coin", "line": 3,
              "key": "s:tests/coin.feature#1", "status": coin },
            { "kind": "scenario", "file": "tests/coin.feature", "refId": "[2:7]", "name": "steady", "line": 7,
              "key": "s:tests/coin.feature#2", "status": "passed" },
        ])
    };
    let one = |ago: i64, coin: &str, threads: u32| {
        let record = json!({
            "threads": threads,
            "fingerprint": "f".repeat(64),
            "exitCode": if coin == "failed" { 1 } else { 0 },
            "tests": tests(coin),
        });
        seed_runs(data, dir, 1, &Seed { report, ago, record })
    };
    let mut ids = one(1, "failed", 2);
    ids.extend(one(0, "passed", 1));
    ids
}

/// Grace on a fresh copy of the flaky sample project with the two runs of `seed_coin` in its history, `coin.feature`
/// expanded: the tree shows `coin` flaky.
fn open_seeded(report: bool) -> (App, PathBuf, Vec<String>) {
    let dir = project("flaky").unwrap();
    let data = fresh_data_dir().unwrap();
    let ids = seed_coin(&data, &dir, report);
    std::fs::write(
        data.join("settings.json"),
        json!({ "lastProject": dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    wait_ready(&app);
    app.wait_for("History (2)", WAIT, |t| tab_is(t, "History (2)")).unwrap();
    (app, dir, ids)
}

/// E-flaky-01, FL-01, `Tree.test.tsx` 28
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_test_that_flips_with_no_change_is_marked_flaky() {
    let (app, dir) = open_coin(false);
    set_threads(&app, &dir, 2);
    app.press_named(Role::Button, "Run all").unwrap();
    finished(&app, 1);
    app.wait_for("coin failed", WAIT, |t| {
        says(t, "coin", "failed") && says(t, "steady", "passed")
    })
    .unwrap();

    coin(&dir, true);
    set_threads(&app, &dir, 1);
    app.wait_for("Re-run failed on", WAIT, |t| {
        t.find(&Role::Button, "Re-run failed").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
    app.press_named(Role::Button, "Re-run failed").unwrap();
    finished(&app, 2);
    let t = app
        .wait_for("coin marked", WAIT, |t| {
            says(t, "coin", "passed") && flaky_row(t, "coin", None)
        })
        .unwrap();
    assert_eq!(marks(&t), 1, "only coin has the mark, named for a screen reader");
    assert!(!flaky_row(&t, "steady", None));
    assert!(flaky_row(&t, "coin.feature", Some(1)) && flaky_row(&t, "tests", Some(1)));

    select(&app, "coin");
    let t = app
        .wait_for("the proof", WAIT, |t| {
            proof(t, NONE).is_some_and(|g| proof_runs(g).len() == 2)
        })
        .unwrap();
    let group = proof(&t, NONE).unwrap();
    assert!(
        says_in(group, "Passed 1 time, failed 1 time with the same project files"),
        "{}",
        group.dump()
    );
    let runs = proof_runs(group);
    assert!(runs[0].name.ends_with(", Passed, 1 thread"), "{}", runs[0].name);
    assert!(runs[1].name.ends_with(", Failed, 2 threads"), "{}", runs[1].name);
    assert_eq!(proof_boxes(&t), 1);
}

/// How many proof boxes Details shows.
fn proof_boxes(tree: &Node) -> usize {
    tree.all()
        .into_iter()
        .filter(|n| n.role == Role::Group && n.name.starts_with("Flaky "))
        .count()
}

/// E-flaky-02, FL-02
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_fix_in_the_project_is_not_flaky() {
    let dir = project("history").unwrap();
    let data = fresh_data_dir().unwrap();
    std::fs::write(
        data.join("settings.json"),
        json!({ "lastProject": dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    wait_ready(&app);
    expand(&app, "fix.feature");
    app.wait_for("fixable", WAIT, |t| row(t, "fixable").is_some()).unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    finished(&app, 1);
    app.wait_for("fixable failed", WAIT, |t| says(t, "fixable", "failed"))
        .unwrap();

    std::fs::write(dir.join("flag.json"), json!({ "fixed": true }).to_string()).unwrap();
    app.wait_for("Re-run failed on", WAIT, |t| {
        t.find(&Role::Button, "Re-run failed").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
    app.press_named(Role::Button, "Re-run failed").unwrap();
    finished(&app, 2);
    // the files differ between the runs (flag.json changed), so the flip proves nothing
    let t = app
        .wait_for("fixable passed", WAIT, |t| says(t, "fixable", "passed"))
        .unwrap();
    assert!(!flaky_row(&t, "fixable", None));
    assert_eq!(marks(&t), 0);
}

/// E-flaky-03, FL-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn different_environments_are_not_compared() {
    let (app, dir) = open_coin(false);
    app.press_named(Role::Button, "Run coin").unwrap();
    finished(&app, 1);
    app.wait_for("coin failed", WAIT, |t| says(t, "coin", "failed"))
        .unwrap();

    coin(&dir, true);
    app.fill(Role::ComboBox, "Env", "qa").unwrap();
    app.key(Key::Enter).unwrap();
    app.wait_until("qa saved", WAIT, || {
        app.settings()["projects"][dir.to_string_lossy().as_ref()]["env"] == json!("qa")
    })
    .unwrap();
    wait_idle(&app);
    app.wait_for("the tests", WAIT, |t| {
        t.find(&Role::Button, "Run coin").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
    app.press_named(Role::Button, "Run coin").unwrap();
    finished(&app, 2);
    let t = app
        .wait_for("coin passed", WAIT, |t| says(t, "coin", "passed"))
        .unwrap();
    assert!(!flaky_row(&t, "coin", None));
    assert_eq!(marks(&t), 0);
}

/// E-flaky-04, FL-04: the Report of a proof run opens that run's report.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_proof_opens_a_runs_report() {
    let (app, dir, ids) = open_seeded(true);
    expand(&app, "coin.feature");
    select(&app, "coin");
    let t = app
        .wait_for("the proof", WAIT, |t| {
            proof(t, NONE).is_some_and(|g| proof_runs(g).len() == 2)
        })
        .unwrap();
    let failed = proof_runs(proof(&t, NONE).unwrap())
        .into_iter()
        .find(|r| r.name.contains(", Failed, "))
        .unwrap();
    app.press(button(failed, "Report")).unwrap();
    app.wait_until("the report opened", WAIT, || !app.opened_paths().is_empty())
        .unwrap();
    assert_eq!(
        PathBuf::from(&app.opened_paths()[0]),
        runs_dir(app.data_dir(), &dir).join(&ids[0]).join("karate-summary.html")
    );
}

/// E-flaky-05, FL-05: a scenario added above `coin` moves its key; the mark follows the test.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_mark_stays_on_its_test_when_the_tree_changes() {
    let (app, dir, _) = open_seeded(false);
    expand(&app, "coin.feature");
    app.wait_for("coin marked", WAIT, |t| flaky_row(t, "coin", None))
        .unwrap();

    let file = dir.join("tests").join("coin.feature");
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::write(
        &file,
        text.replace("Scenario: coin", "Scenario: added\n  * match 1 == 1\n\nScenario: coin"),
    )
    .unwrap();
    app.press_named(Role::Button, "Refresh").unwrap();
    let t = app
        .wait_for("added and coin marked", WAIT, |t| {
            // one read can hold a row's new name and its old children: wait for the mark with the name
            row(t, "added").is_some() && flaky_row(t, "coin", None) && marks(t) == 1
        })
        .unwrap();
    assert!(!flaky_row(&t, "added", None));
}

/// E-flaky-06, FL-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn marks_survive_a_restart() {
    let (app, _, _) = open_seeded(false);
    app.wait_for("the feature counts it", WAIT, |t| flaky_row(t, "coin.feature", Some(1)))
        .unwrap();
    // one app per data folder: `relaunch` closes the first and starts the second on the same folder
    let app = app.relaunch(&[]).unwrap();
    wait_ready(&app);
    app.wait_for("History (2) again", WAIT, |t| tab_is(t, "History (2)"))
        .unwrap();
    // a restart starts with the features closed (#224 R1.3)
    app.wait_for("the feature counts it", WAIT, |t| flaky_row(t, "coin.feature", Some(1)))
        .unwrap();
    expand(&app, "coin.feature");
    let t = app
        .wait_for("coin marked", WAIT, |t| flaky_row(t, "coin", None))
        .unwrap();
    assert_eq!(marks(&t), 1);
}

/// E-flaky-07, FL-07
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn clear_history_removes_the_marks() {
    let (app, _, _) = open_seeded(false);
    expand(&app, "coin.feature");
    let t = app
        .wait_for("coin marked", WAIT, |t| flaky_row(t, "coin", None))
        .unwrap();
    assert!(flaky_row(&t, "coin.feature", Some(1)));

    show_history(&app);
    app.press_named(Role::Button, "Clear history").unwrap();
    app.wait_for("the dialog", WAIT, |t| {
        t.all()
            .iter()
            .any(|n| n.name == "Delete the 2 runs of this project and their reports?")
    })
    .unwrap();
    app.press_named(Role::Button, "Delete").unwrap();
    app.wait_for("History (0)", WAIT, |t| tab_is(t, "History (0)")).unwrap();
    // the tests stay in the tree: only their marks and the counts above them go
    app.wait_for("no marks", WAIT, |t| {
        row(t, "coin").is_some()
            && marks(t) == 0
            && !flaky_row(t, "coin.feature", Some(1))
            && !flaky_row(t, "tests", Some(1))
    })
    .unwrap();
}
