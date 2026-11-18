//! `e2e/history.spec.ts` (E-history-01 to E-history-13) through the accessibility tree: a run listed in the History tab,
//! kept over a restart, cancelled, re-run and re-run failed, cleared, and every action waiting for the runner; and
//! E-look-16's tab half (what the History and HTTP tabs say before any run) and E-look-17 (Clear history's button
//! order and Escape). Export (E-history-14) is slice 6's.
//!
//! A row is a list item of the list "Run history" named "<when>, <what ran>, <result>"; each cell a label inside it, whose
//! name is its text, and each button a button inside it. A button's disabled state reaches UI Automation only (the pinned
//! AT-SPI adapter drops it), so the checks of it run on Windows.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::history::{Seed, runs_dir, seed_runs};
use grace_e2e::{App, Key, Node, Role, fresh_data_dir, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(120);
const LIST: &str = "Run history";
const NO_RUNS: &str = "No runs yet. Runs appear here when they finish.";

fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

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

/// The rows of the history, newest first.
fn rows(tree: &Node) -> Vec<&Node> {
    tree.find(&Role::List, LIST)
        .map(|l| l.all().into_iter().filter(|n| n.role == Role::ListItem).collect())
        .unwrap_or_default()
}

/// Whether a label of the row says `text`.
fn has(row: &Node, text: &str) -> bool {
    row.all()
        .into_iter()
        .skip(1)
        .any(|n| n.role == Role::Text && n.name == text)
}

/// How many labels of the row say `text`.
fn count(row: &Node, text: &str) -> usize {
    row.all()
        .into_iter()
        .skip(1)
        .filter(|n| n.role == Role::Text && n.name == text)
        .count()
}

fn button<'a>(row: &'a Node, name: &str) -> &'a Node {
    row.all()
        .into_iter()
        .skip(1)
        .find(|n| n.role == Role::Button && n.name == name)
        .unwrap_or_else(|| panic!("{name} in {}", row.dump()))
}

/// Grace on `dir` with `n` runs in its history already.
fn open_with_runs(dir: &Path, n: usize, seed: &Seed) -> (App, Vec<String>) {
    let data = fresh_data_dir().unwrap();
    let ids = seed_runs(&data, dir, n, seed);
    std::fs::write(
        data.join("settings.json"),
        json!({ "lastProject": dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    wait_for_tests(&app);
    (app, ids)
}

fn open(dir: &Path) -> App {
    open_with_runs(dir, 0, &Seed::default()).0
}

/// Waits until the project is ready: its tests found and the window no longer busy. The toolbar and the History tab are
/// in the tree from the first read, and every control is off (a press of one returns Ok and does nothing) while the
/// opening discovery or an environment switch lasts; "Ready." shows only when it is over and Run all can be pressed
/// (`banner_state`), on both systems and before the first run writes to the console.
fn wait_for_tests(app: &App) {
    wait_ready(app);
    app.wait_for("the History tab", WAIT, |t| tab(t).is_some()).unwrap();
}

fn wait_ready(app: &App) {
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
}

/// Waits until the window is no longer busy, after something that made it busy (an environment switch): Run all can be
/// pressed again. The console holds output after a run and "Ready." is gone, so this reads the button, on Windows only
/// (the pinned AT-SPI adapter drops a disabled state).
// shortcut: no wait on Linux, so a press right after the switch can be lost there; no Linux run has lost one, and the
// loop that presses Clear again stays; replace it if one does
fn wait_idle(app: &App) {
    #[cfg(windows)]
    app.wait_for("Run all on", WAIT, |t| {
        t.find(&Role::Button, "Run all").is_some_and(|b| b.states.enabled)
    })
    .unwrap();
    #[cfg(not(windows))]
    let _ = app;
}

/// The tree, read again until a read goes through.
fn read_tree(app: &App) -> Node {
    app.wait_for("a tree read", WAIT, |_| true).unwrap()
}

/// Presses a button of the newest run's row, reading the tree again when a press meets one that changed.
fn press_in_top_row(app: &App, name: &str) {
    let mut last = String::from("no row");
    for _ in 0..5 {
        let t = read_tree(app);
        match rows(&t).first().map(|r| app.press(button(r, name))) {
            Some(Ok(())) => return,
            Some(Err(e)) => last = e,
            None => {}
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("{name} could not be pressed in the top row: {last}");
}

fn show_history(app: &App) {
    // a tree read or a press can meet a tree that changed under it (AT-SPI: UnknownObject): read again and press again
    let mut pressed = false;
    for _ in 0..5 {
        let t = read_tree(app);
        if tab(&t).is_some_and(|n| app.press(n).is_ok()) {
            pressed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(pressed, "the History tab could not be pressed");
    app.wait_for("the History tab", WAIT, |t| tab(t).is_some_and(|n| n.states.selected))
        .unwrap();
}

fn expand(app: &App, feature: &str) {
    app.press_named(Role::Button, &format!("Expand {feature}")).unwrap();
}

/// Run all, then waits until the run is in the history (`n` runs): a stored run has ended.
fn run_all(app: &App, n: usize) {
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for(&format!("History ({n})"), RUN, |t| tab_is(t, &format!("History ({n})")))
        .unwrap();
    app.wait_for("the run over", WAIT, |t| starting(t, "Finished").is_some())
        .unwrap();
}

fn notice(tree: &Node, text: &str) -> bool {
    named(tree, text).is_some()
}

/// A test's name of the history sample project, to be pressed to run it alone.
fn run_test(app: &App, name: &str) {
    app.press_named(Role::Button, &format!("Run {name}")).unwrap();
}

fn runs_in(data: &Path, dir: &Path) -> Vec<String> {
    let mut ids: Vec<_> = std::fs::read_dir(runs_dir(data, dir))
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids
}

/// E-history-01, RH-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_run_appears_in_the_history() {
    let dir = project("clean").unwrap();
    let app = open(&dir);
    app.wait_for("History (0)", WAIT, |t| tab_is(t, "History (0)")).unwrap();
    expand(&app, "basic.feature");
    run_all(&app, 1);
    show_history(&app);
    let t = app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    let r = rows(&t)[0];
    assert!(r.name.starts_with("Today "), "{}", r.name);
    assert!(has(r, "All") && has(r, "8 passed, 1 failed of 9"), "{}", r.dump());
    // the environment and the tags say "—"
    assert_eq!(count(r, "—"), 2, "{}", r.dump());
    assert!(
        r.all().into_iter().any(|n| n.role == Role::Text
            && n.name.len() >= 2
            && n.name.ends_with('s')
            && n.name
                .trim_end_matches('s')
                .chars()
                .all(|c| c.is_ascii_digit() || c == 'm' || c == ' ')),
        "a duration: {}",
        r.dump()
    );
    // the mark before the result's words is decorative: the words name the status
    assert!(r.all().into_iter().all(|n| n.role != Role::Image), "{}", r.dump());
    for name in ["Report", "Re-run", "Re-run failed"] {
        assert!(button(r, name).states.enabled, "{name}");
    }
}

/// E-history-02, RH-02
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn history_survives_a_restart() {
    let dir = project("clean").unwrap();
    let (app, ids) = open_with_runs(
        &dir,
        2,
        &Seed {
            report: true,
            ..Seed::default()
        },
    );
    app.wait_for("History (2)", WAIT, |t| tab_is(t, "History (2)")).unwrap();
    let app = app.relaunch(&[]).unwrap();
    app.wait_for("History (2) again", WAIT, |t| tab_is(t, "History (2)"))
        .unwrap();
    show_history(&app);
    app.wait_for("the rows", WAIT, |t| rows(t).len() == 2).unwrap();
    // newest first: the Report of the top row opens the newest run's report
    press_in_top_row(&app, "Report");
    app.wait_until("the report opened", WAIT, || !app.opened_paths().is_empty())
        .unwrap();
    let opened = PathBuf::from(&app.opened_paths()[0]);
    assert_eq!(
        opened,
        runs_dir(app.data_dir(), &dir).join(&ids[1]).join("karate-summary.html")
    );
}

/// E-history-03, RH-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_cancelled_run_is_listed_without_a_report() {
    let dir = project("clean").unwrap();
    let app = open(&dir);
    expand(&app, "slow.feature");
    app.wait_for("slow loop", WAIT, |t| t.find(&Role::Button, "Run slow loop").is_some())
        .unwrap();
    run_test(&app, "slow loop");
    app.wait_for("slow loop running", WAIT, |t| says(t, "slow loop", "running"))
        .unwrap();
    app.press_named(Role::Button, "Cancel").unwrap();
    app.wait_for("Cancelled", Duration::from_secs(60), |t| {
        starting(t, "Cancelled").is_some()
    })
    .unwrap();
    app.wait_for("History (1)", WAIT, |t| tab_is(t, "History (1)")).unwrap();
    show_history(&app);
    let t = app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    let r = rows(&t)[0];
    assert!(has(r, "Cancelled, 0 passed, 0 failed of 1"), "{}", r.dump());
    #[cfg(windows)]
    {
        assert!(!button(r, "Report").states.enabled);
        assert!(button(r, "Re-run").states.enabled);
    }
}

/// E-history-04, RH-04: only the failed tests go; the new run's total says how many were selected.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn re_run_failed_runs_only_the_failed_tests() {
    let dir = project("history").unwrap();
    let app = open(&dir);
    expand(&app, "fix.feature");
    app.wait_for("the tests", WAIT, |t| row(t, "fixable").is_some())
        .unwrap();
    run_all(&app, 1);
    app.wait_for("the statuses", WAIT, |t| {
        says(t, "fixable", "failed") && says(t, "row 2", "failed") && says(t, "stays green", "passed")
    })
    .unwrap();
    // "dynamic 2" fails too: it is the third failed test
    app.wait_for("Re-run failed 3", WAIT, |t| {
        t.find(&Role::Button, "Re-run failed").is_some_and(|b| b.states.enabled)
    })
    .unwrap();

    std::fs::write(dir.join("flag.json"), json!({ "fixed": true }).to_string()).unwrap();
    app.press_named(Role::Button, "Re-run failed").unwrap();
    app.wait_for("History (2)", RUN, |t| tab_is(t, "History (2)")).unwrap();
    app.wait_for("fixable fixed", WAIT, |t| {
        says(t, "fixable", "passed") && says(t, "row 2", "failed")
    })
    .unwrap();
    show_history(&app);
    let t = app.wait_for("the rows", WAIT, |t| rows(t).len() == 2).unwrap();
    let newest = rows(&t)[0];
    assert!(
        newest
            .all()
            .into_iter()
            .any(|n| n.role == Role::Text && n.name.starts_with("Failed tests of Today ")),
        "{}",
        newest.dump()
    );
    // two tests were selected, and one of them still fails
    assert!(has(newest, "1 passed, 1 failed of 2"), "{}", newest.dump());
}

/// E-history-05, RH-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_failed_dynamic_row_is_skipped() {
    let dir = project("history").unwrap();
    let app = open(&dir);
    run_all(&app, 1);
    show_history(&app);
    app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    press_in_top_row(&app, "Re-run failed");
    app.wait_for("the notice", RUN, |t| {
        notice(
            t,
            "1 failed row of a dynamic Examples table can't be run alone. Run its outline.",
        )
    })
    .unwrap();
    app.wait_for("History (2)", RUN, |t| tab_is(t, "History (2)")).unwrap();
    // the two other failed tests ran: only the dynamic row stayed out
    let t = app.wait_for("the rows", WAIT, |t| rows(t).len() == 2).unwrap();
    assert!(has(rows(&t)[0], "0 passed, 2 failed of 2"), "{}", rows(&t)[0].dump());
}

/// E-history-06, RH-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn re_run_switches_back_to_the_runs_environment_and_filter() {
    let dir = project("history").unwrap();
    let app = open(&dir);
    let saved_env = |app: &App| app.settings()["projects"][dir.to_string_lossy().as_ref()]["env"].clone();
    app.fill(Role::ComboBox, "Env", "qa").unwrap();
    app.key(Key::Enter).unwrap();
    app.wait_until("qa saved", WAIT, || saved_env(&app) == json!("qa"))
        .unwrap();
    // the filter is refused while the switch lasts
    wait_idle(&app);
    app.press_named(Role::Button, "@smoke, not filtered").unwrap();
    app.wait_for("@smoke included", WAIT, |t| {
        t.find(&Role::Button, "@smoke, included").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Run fix.feature").unwrap();
    app.wait_for("History (1)", RUN, |t| tab_is(t, "History (1)")).unwrap();
    show_history(&app);
    let t = app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    let r = rows(&t)[0];
    for text in ["qa", "@smoke", "tests/fix.feature", "1 passed, 0 failed of 1"] {
        assert!(has(r, text), "{text} in {}", r.dump());
    }

    // back to no environment and no filter
    app.fill(Role::ComboBox, "Env", "(none)").unwrap();
    app.key(Key::Enter).unwrap();
    app.wait_until("none saved", WAIT, || saved_env(&app).is_null())
        .unwrap();
    // Clear is off while the switch lasts: wait for its end. The loop that presses again while the filter still shows
    // stays from before that wait; it is not shown to be needless.
    wait_idle(&app);
    app.wait_for("the filter clear", WAIT, |t| {
        if t.find(&Role::Button, "@smoke, included").is_some() {
            let _ = app.press_named(Role::Button, "Clear");
            return false;
        }
        t.find(&Role::Button, "@smoke, not filtered").is_some()
    })
    .unwrap();

    press_in_top_row(&app, "Re-run");
    app.wait_for("History (2)", RUN, |t| tab_is(t, "History (2)")).unwrap();
    assert_eq!(saved_env(&app), json!("qa"));
    // the Env box says it too; a text box's text reaches the tree on Windows only (guide R8.4)
    #[cfg(windows)]
    app.wait_for_text(Role::ComboBox, "Env", "qa").unwrap();
    let t = app
        .wait_for("@smoke back", WAIT, |t| {
            t.find(&Role::Button, "@smoke, included").is_some()
        })
        .unwrap();
    // only "stays green" matched the filter: one test ran
    assert!(has(rows(&t)[0], "1 passed, 0 failed of 1"), "{}", rows(&t)[0].dump());
    app.wait_for("stays green passed", WAIT, |t| says(t, "stays green", "passed"))
        .unwrap();
}

/// E-history-07, RH-07
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn cancelling_the_environment_switch_of_a_re_run_runs_nothing() {
    let dir = project("env").unwrap();
    let seed = Seed {
        record: json!({
            "env": "qa",
            "tests": [{ "kind": "scenario", "file": "tests/env.feature", "refId": "[1:3]", "name": "always",
                        "line": 3, "key": "s:tests/env.feature#1", "status": "passed" }],
        }),
        ..Seed::default()
    };
    let (app, _) = open_with_runs(&dir, 1, &seed);
    app.wait_for("History (1)", WAIT, |t| tab_is(t, "History (1)")).unwrap();
    show_history(&app);
    let t = app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    let cancel = t.find(&Role::Button, "Cancel").unwrap().clone();
    press_in_top_row(&app, "Re-run");
    // Cancel kills only a process that exists, and the discovery of "qa" lasts a second or two: press it quickly, without
    // reading the tree between presses (a read is slower than the discovery), until the discovery is cancelled
    // shortcut: a timed loop, not a wait: the discovery lasts a second or two and no state of the tree says "now", so the
    // presses cannot wait on one; replace it if the runner ever shows a discovery in the tree
    for _ in 0..40 {
        let _ = app.press(&cancel); // an error while it is off
        std::thread::sleep(Duration::from_millis(40));
    }
    let tree = app
        .wait_for("the switch cancelled", WAIT, |t| {
            notice(t, "Cancelled") || named(t, "Run cancelled").is_some()
        })
        .unwrap();
    assert!(
        notice(&tree, "Cancelled"),
        "the switch, not a run, was cancelled: {}",
        tree.dump()
    );

    assert!(app.settings()["projects"][dir.to_string_lossy().as_ref()]["env"].is_null());
    #[cfg(windows)]
    app.wait_for_text(Role::ComboBox, "Env", "").unwrap();
    app.wait_until("no Java", WAIT, || app.java_pids().is_empty()).unwrap();
    let t = read_tree(&app);
    assert!(tab_is(&t, "History (1)"), "{}", t.dump());
    assert!(starting(&t, "Finished").is_none() && starting(&t, "Running").is_none());
    assert_eq!(runs_in(app.data_dir(), &dir).len(), 1);
}

/// E-history-08, RH-08
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_re_run_of_a_test_that_no_longer_exists() {
    let dir = project("clean").unwrap();
    let seed = Seed {
        record: json!({
            "selection": { "kind": "scenario", "file": "tests/basic.feature", "name": "failing match",
                           "line": 15, "key": "s:tests/basic.feature#2" },
        }),
        ..Seed::default()
    };
    let (app, _) = open_with_runs(&dir, 1, &seed);
    let file = dir.join("tests").join("basic.feature");
    let before = std::fs::read_to_string(&file).unwrap();
    let from = before
        .find(
            "@wip
Scenario: failing match",
        )
        .unwrap();
    let to = before.find("Scenario: calls helper feature").unwrap();
    std::fs::write(&file, format!("{}{}", &before[..from], &before[to..])).unwrap();

    show_history(&app);
    app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    press_in_top_row(&app, "Re-run");
    app.wait_for("the notice", WAIT, |t| {
        notice(t, "The tests of this run no longer exist")
    })
    .unwrap();
    app.wait_until("no Java", WAIT, || app.java_pids().is_empty()).unwrap();
    let t = read_tree(&app);
    assert!(tab_is(&t, "History (1)"), "{}", t.dump());
    assert!(starting(&t, "Finished").is_none());
}

/// E-history-09, RH-09
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn only_the_20_newest_runs_are_kept() {
    let dir = project("clean").unwrap();
    let (app, ids) = open_with_runs(&dir, 20, &Seed::default());
    app.wait_for("History (20)", WAIT, |t| tab_is(t, "History (20)"))
        .unwrap();
    expand(&app, "basic.feature");
    app.wait_for("passing hello", WAIT, |t| {
        t.find(&Role::Button, "Run passing hello").is_some()
    })
    .unwrap();
    run_test(&app, "passing hello");
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
    show_history(&app);
    let t = app
        .wait_for("the new run on top", WAIT, |t| {
            rows(t).len() == 20
                && rows(t)[0]
                    .all()
                    .into_iter()
                    .any(|n| n.role == Role::Text && n.name == "passing hello")
        })
        .unwrap();
    assert!(tab_is(&t, "History (20)"));
    let kept = runs_in(app.data_dir(), &dir);
    assert_eq!(kept.len(), 20);
    assert!(!kept.contains(&ids[0]) && kept.contains(&ids[1]), "{kept:?}");
}

/// E-history-10, RH-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn clear_history() {
    let dir = project("clean").unwrap();
    let (app, _) = open_with_runs(&dir, 2, &Seed::default());
    expand(&app, "basic.feature");
    app.wait_for("failing match", WAIT, |t| {
        t.find(&Role::Button, "Run failing match").is_some()
    })
    .unwrap();
    // a run that finishes in this session enables the toolbar Report; a failed one, Re-run failed
    run_test(&app, "failing match");
    app.wait_for("History (3)", RUN, |t| tab_is(t, "History (3)")).unwrap();
    app.wait_for("the run over", WAIT, |t| starting(t, "Finished").is_some())
        .unwrap();
    #[cfg(windows)]
    app.wait_for("Report and Re-run failed on", WAIT, |t| {
        t.find(&Role::Button, "Report").is_some_and(|b| b.states.enabled)
            && t.find(&Role::Button, "Re-run failed").is_some_and(|b| b.states.enabled)
    })
    .unwrap();

    show_history(&app);
    app.press_named(Role::Button, "Clear history").unwrap();
    app.wait_for("the dialog", WAIT, |t| {
        named(t, "Delete the 3 runs of this project and their reports?").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Delete").unwrap();
    app.wait_for("No runs yet", WAIT, |t| named(t, NO_RUNS).is_some())
        .unwrap();
    app.wait_for("History (0)", WAIT, |t| tab_is(t, "History (0)")).unwrap();
    assert!(runs_in(app.data_dir(), &dir).is_empty());
    #[cfg(windows)]
    app.wait_for("Report and Re-run failed off", WAIT, |t| {
        t.find(&Role::Button, "Report").is_some_and(|b| !b.states.enabled)
            && t.find(&Role::Button, "Re-run failed")
                .is_some_and(|b| !b.states.enabled)
    })
    .unwrap();
}

/// A button of the open dialog: the toolbar has a Cancel of its own, which comes first in the tree.
fn in_dialog<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Dialog)?
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name == name)
}

/// E-look-17, PU-32: Delete first, then Cancel with the focus; Escape closes the dialog and deletes nothing.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn clear_history_keeps_its_button_order_and_closes_on_escape() {
    let dir = project("clean").unwrap();
    let (app, _) = open_with_runs(&dir, 2, &Seed::default());
    app.wait_for("History (2)", WAIT, |t| tab_is(t, "History (2)")).unwrap();
    show_history(&app);
    // a real click, as a person opens it: it makes the window the active one, and a dialog in a window that is not
    // active never takes the focus (#301)
    let t = read_tree(&app);
    app.click(t.find(&Role::Button, "Clear history").unwrap()).unwrap();
    let t = app
        .wait_for("the dialog", WAIT, |t| {
            named(t, "Delete the 2 runs of this project and their reports?").is_some()
                && in_dialog(t, "Cancel").is_some_and(|b| b.states.focused)
        })
        .unwrap_or_else(|e| {
            panic!(
                "{e}
{}",
                app.focus_report()
            )
        });
    let (delete, cancel) = (in_dialog(&t, "Delete").unwrap(), in_dialog(&t, "Cancel").unwrap());
    assert!(
        delete.rect.unwrap().x < cancel.rect.unwrap().x,
        "Delete before Cancel: {}",
        t.dump()
    );
    app.key(Key::Escape).unwrap();
    app.wait_for("the dialog closed", WAIT, |t| {
        named(t, "Delete the 2 runs of this project and their reports?").is_none()
    })
    .unwrap();
    assert_eq!(runs_in(app.data_dir(), &dir).len(), 2);
    assert!(tab_is(&read_tree(&app), "History (2)"));
}

/// E-history-11, RH-11
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_damaged_run_file_does_not_hide_the_others() {
    let dir = project("clean").unwrap();
    let (app, ids) = open_with_runs(&dir, 3, &Seed::default());
    app.wait_for("History (3)", WAIT, |t| tab_is(t, "History (3)")).unwrap();
    let data = app.stop();
    std::fs::write(
        runs_dir(&data, &dir).join(&ids[1]).join("run.json"),
        r#"{ "version": 1, "startedAt": "#,
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    app.wait_for("History (2)", WAIT, |t| tab_is(t, "History (2)")).unwrap();
    show_history(&app);
    app.wait_for("two rows", WAIT, |t| rows(t).len() == 2).unwrap();
}

/// E-history-12, RH-12: a secret never reaches a history file. A secret needs the system's secret storage, which Grace
/// has on Windows only (migration guide R5.3).
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_secret_never_reaches_the_history_files() {
    use grace_e2e::properties::{add_property, choose_scope, open_properties, save_properties};
    const SECRET: &str = "s3cret-token-123";
    let dir = project("env").unwrap();
    let app = open(&dir);
    app.wait_for("the tests", WAIT, |t| {
        t.find(&Role::Button, "Expand env.feature").is_some()
    })
    .unwrap();
    app.fill(Role::ComboBox, "Env", "qa").unwrap();
    app.key(Key::Enter).unwrap();
    app.wait_until("qa saved", WAIT, || {
        app.settings()["projects"][dir.to_string_lossy().as_ref()]["env"] == json!("qa")
    })
    .unwrap();
    open_properties(&app);
    choose_scope(&app, "qa");
    add_property(&app, "qa", "api.token", SECRET, true);
    save_properties(&app);
    app.wait_until("the secret saved", WAIT, || {
        let qa = &app.settings()["projects"][dir.to_string_lossy().as_ref()]["properties"]["byEnv"]["qa"];
        qa[0]["name"] == "api.token" && qa[0]["secret"].is_string()
    })
    .unwrap();

    expand(&app, "env.feature");
    app.wait_for("always", WAIT, |t| t.find(&Role::Button, "Run always").is_some())
        .unwrap();
    run_test(&app, "always");
    app.wait_for("History (1)", RUN, |t| tab_is(t, "History (1)")).unwrap();
    let ids = runs_in(app.data_dir(), &dir);
    assert_eq!(ids.len(), 1);
    let text = std::fs::read_to_string(runs_dir(app.data_dir(), &dir).join(&ids[0]).join("run.json")).unwrap();
    assert!(text.contains(r#""env":"qa""#), "the record of this run: {text}");
    assert!(!text.contains(SECRET));
}

/// E-history-13, RH-13
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn history_actions_wait_for_the_runner() {
    let dir = project("clean").unwrap();
    let (app, _) = open_with_runs(
        &dir,
        1,
        &Seed {
            report: true,
            ..Seed::default()
        },
    );
    app.wait_for("History (1)", WAIT, |t| tab_is(t, "History (1)")).unwrap();
    show_history(&app);
    let t = app.wait_for("the row", WAIT, |t| rows(t).len() == 1).unwrap();
    let on = |t: &Node, on: bool| {
        let r = rows(t)[0];
        ["Report", "Re-run", "Re-run failed"]
            .iter()
            .all(|b| button(r, b).states.enabled == on)
            && t.find(&Role::Button, "Clear history")
                .is_some_and(|b| b.states.enabled == on)
            && t.find(&Role::Button, "Re-run failed")
                .is_some_and(|b| b.states.enabled == on)
    };
    #[cfg(windows)]
    assert!(on(&t, true), "{}", t.dump());
    drop(t);

    expand(&app, "slow.feature");
    app.wait_for("slow loop", WAIT, |t| t.find(&Role::Button, "Run slow loop").is_some())
        .unwrap();
    run_test(&app, "slow loop");
    app.wait_for("slow loop running", WAIT, |t| says(t, "slow loop", "running"))
        .unwrap();
    // the run keeps the tab that is open: History
    #[cfg(windows)]
    app.wait_for("every action off", WAIT, |t| on(t, false)).unwrap();
    #[cfg(not(windows))]
    let _ = on;

    app.press_named(Role::Button, "Cancel").unwrap();
    app.wait_for("History (2)", Duration::from_secs(60), |t| tab_is(t, "History (2)"))
        .unwrap();
    // the cancelled run on top has no report and no failed test: the seeded run below is the one to check
    let t = app.wait_for("two rows", WAIT, |t| rows(t).len() == 2).unwrap();
    #[cfg(windows)]
    {
        let seeded = rows(&t)[1];
        for name in ["Report", "Re-run", "Re-run failed"] {
            assert!(button(seeded, name).states.enabled, "{name}");
        }
        assert!(t.find(&Role::Button, "Clear history").unwrap().states.enabled);
    }
    #[cfg(not(windows))]
    let _ = t;
}

/// E-look-16's tab half, PU-31: before any run the History and HTTP tabs say so.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn before_any_run_the_history_and_http_tabs_say_so() {
    let dir = project("clean").unwrap();
    let app = open(&dir);
    show_history(&app);
    let t = app
        .wait_for("No runs yet", WAIT, |t| named(t, NO_RUNS).is_some())
        .unwrap();
    assert!(tab_is(&t, "History (0)"), "{}", t.dump());
    #[cfg(windows)]
    assert!(
        t.find(&Role::Button, "Clear history")
            .is_some_and(|b| !b.states.enabled)
    );

    let http = t
        .all()
        .into_iter()
        .find(|n| n.role == Role::Tab && n.name.starts_with("HTTP"))
        .unwrap();
    app.press(http).unwrap();
    let t = app
        .wait_for("the HTTP tab", WAIT, |t| {
            named(t, "No HTTP calls yet. Calls appear when a feature finishes.").is_some()
        })
        .unwrap();
    assert!(t.find(&Role::Tab, "HTTP (0)").is_some(), "{}", t.dump());
}
