//! `runner.watch.test.ts`: watch mode, with real saves in a fixture copy and the real watcher. Its second test
//! ("refuses a value that is not a boolean") has nothing to refuse here: `set_watch` takes a `bool` (WM-02).
use super::*;
use crate::settings::{load_project_watch, save_project_watch};
use crate::watch::QUIET;
use std::sync::mpsc;

const PASSING: &str = "Scenario: one\n  * match 1 == 1\n";
const BROKEN: &str = "Scenario: bad gherkin\n  Given def x = \n  Examples:\n  | oops\nScenario Outline\n";

/// Saves `tests/<name>.feature` in the tags project.
fn save_as(root: &Fixture, name: &str, body: &str) {
    write(
        root.path(),
        &[(&format!("tests/{name}.feature"), &format!("Feature: {name}\n\n{body}"))],
    );
}

fn save(root: &Fixture, name: &str) {
    save_as(root, name, PASSING);
}

/// The tags project, open, with watch mode on. The runner is first, so it is dropped first: Windows keeps a watched
/// folder's parents from being removed until the watcher closes.
struct Watching {
    t: TestRunner,
    root: Fixture,
}

fn watching() -> Watching {
    let root = fixture("tags");
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    t.runner.set_watch(true);
    Watching { t, root }
}

/// Operations started so far: a watch run starts one.
fn starts(t: &TestRunner) -> usize {
    t.all("busy").iter().filter(|b| b["busy"] == true).count()
}

/// Past a batch's quiet time, and idle again.
fn settle(t: &TestRunner) {
    thread::sleep(QUIET + Duration::from_millis(500));
    until(|| !t.runner.is_busy());
}

/// A batch waits for the runner to be idle.
fn until_waiting(t: &TestRunner) {
    until(|| !lock(&t.runner.0.waiting).is_empty());
}

fn watcher_armed(t: &TestRunner) -> bool {
    lock(&t.runner.0.watcher).is_some()
}

fn selections(t: &TestRunner) -> Vec<String> {
    t.all("runStarted")
        .iter()
        .map(|e| e["selection"].as_str().unwrap().to_string())
        .collect()
}

fn finished(t: &TestRunner, runs: usize) {
    until(|| t.count("runFinished") >= runs);
}

fn stopped(t: &TestRunner) {
    until(|| t.notices().iter().any(|n| n.starts_with("Watch mode stopped: ")));
}

/// The tags project with its tests as the one feature root, so a test can remove it.
fn with_paths() -> Fixture {
    let root = fixture("tags");
    write(root.path(), &[("karate-pom.json", r#"{"paths":["tests"]}"#)]);
    root
}

#[test]
fn turning_on_saves_it_sends_it_and_is_replayed() {
    let Watching { t, root } = watching();
    assert_eq!(t.last("watch"), json!({ "type": "watch", "on": true }));
    assert!(load_project_watch(&t.data_dir, &root.root));
    assert!(t.runner.replay().contains(&AppEvent::Watch { on: true }));
    t.runner.dispose();
    let second = setup_with(Setup {
        data: Some(Arc::clone(&t.data)),
        ..Setup::default()
    });
    assert!(second.runner.open(&root.root).unwrap());
    assert_eq!(second.last("watch")["on"], true);
    let clean = fixture("clean");
    assert!(second.runner.open(&clean.root).unwrap());
    assert_eq!(second.last("watch")["on"], false);
    drop(second); // before the folders it opened
}

#[test]
fn applies_a_choice_it_cannot_save_and_says_so() {
    let Watching { t, root: _root } = watching();
    let settings = t.data_dir.join("settings.json");
    fs::remove_file(&settings).unwrap(); // open saved it: replace it with a folder
    fs::create_dir(&settings).unwrap(); // writing a file there fails
    t.runner.set_watch(false);
    assert!(t.last_notice().starts_with("Watch mode could not be saved: "));
    assert_eq!(t.last("watch")["on"], false);
    assert!(!watcher_armed(&t));
}

#[test]
fn runs_a_saved_feature_as_its_selection_and_its_run_starts_no_other() {
    let Watching { t, root } = watching();
    save(&root, "w1");
    finished(&t, 1);
    let started = t.last("runStarted");
    assert_eq!(started["selection"], "f:tests/w1.feature");
    assert_eq!(started["keys"], json!([t.key("one")]));
    let record = record(&t.data_dir, &root.root, started["runId"].as_str().unwrap());
    assert_eq!(record["selection"]["key"], "f:tests/w1.feature");
    thread::sleep(Duration::from_millis(1500)); // Karate read every feature
    settle(&t);
    assert_eq!(t.count("runStarted"), 1);
}

#[test]
fn runs_nothing_while_off() {
    let Watching { t, root } = watching();
    t.runner.set_watch(false);
    assert!(!watcher_armed(&t));
    let before = starts(&t);
    save(&root, "w1");
    settle(&t);
    assert_eq!(starts(&t), before);
}

#[test]
fn runs_two_features_saved_together_as_one_run_with_no_selection() {
    let Watching { t, root } = watching();
    save(&root, "w1");
    save(&root, "w2");
    finished(&t, 1);
    settle(&t);
    assert_eq!(t.count("runStarted"), 1);
    let started = t.last("runStarted");
    assert_eq!(started["selection"], t.tree().key.as_str());
    assert_eq!(started["keys"].as_array().unwrap().len(), 2);
}

#[test]
fn runs_only_the_tests_that_match_the_filter() {
    let Watching { t, root } = watching();
    t.runner.set_filter(filter(&["smoke"], &[], false));
    save_as(
        &root,
        "w1",
        &format!("@smoke\n{}\n{PASSING}", PASSING.replace("one", "tagged")),
    );
    finished(&t, 1);
    assert_eq!(t.last("runStarted")["keys"], json!([t.key("tagged")]));
}

#[test]
fn skips_a_feature_with_nothing_to_run_without_a_notice() {
    let Watching { t, root } = watching();
    let notices = t.notices().len();
    let before = starts(&t);
    save_as(&root, "helper", &format!("@ignore\n{PASSING}"));
    until(|| starts(&t) == before + 1); // the dry run before a run
    settle(&t);
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(t.notices().len(), notices);
}

#[test]
fn runs_a_save_made_during_a_run_once_the_run_ends() {
    let Watching { t, root } = watching();
    let running = t.bg(|r| r.run("d:"));
    until(|| t.runner.is_busy());
    save(&root, "w1");
    until_waiting(&t);
    assert!(t.runner.is_busy(), "the batch waits for the run");
    running.join().unwrap().unwrap();
    finished(&t, 2);
    until(|| !t.runner.is_busy());
    assert_eq!(selections(&t), ["d:", "f:tests/w1.feature"]);
    assert!(!t.notices().iter().any(|n| n == "A run is already in progress"));
    // the watch run says it is busy only after the run before said it was idle
    let busy: Vec<bool> = t.all("busy").iter().map(|b| b["busy"] == true).collect();
    assert!(busy.iter().enumerate().all(|(i, b)| *b == (i % 2 == 0)), "{busy:?}");
}

#[test]
fn cancel_drops_the_saves_waiting() {
    let Watching { t, root } = watching();
    let running = t.bg(|r| r.run("d:"));
    until(|| t.runner.is_busy());
    save(&root, "w1");
    until_waiting(&t);
    assert!(t.runner.is_busy());
    t.runner.cancel();
    running.join().unwrap().unwrap();
    settle(&t);
    assert!(!selections(&t).iter().any(|s| s == "f:tests/w1.feature"));
}

#[test]
fn turning_watch_mode_off_drops_the_saves_waiting() {
    let Watching { t, root } = watching();
    let running = t.bg(|r| r.run("d:"));
    until(|| t.java_up());
    save(&root, "w1");
    until_waiting(&t);
    assert!(t.runner.is_busy());
    t.runner.set_watch(false); // applied during the run (WM-03)
    assert_eq!(t.last("watch")["on"], false);
    running.join().unwrap().unwrap();
    settle(&t);
    assert_eq!(selections(&t), ["d:"]);
    assert!(!t.notices().iter().any(|n| n == "A run is already in progress"));
}

#[test]
fn closing_the_project_stops_watching() {
    let Watching { t, root } = watching();
    t.runner.close();
    assert_eq!(t.last("watch")["on"], false);
    assert!(!watcher_armed(&t));
    let before = starts(&t);
    save(&root, "w1");
    settle(&t);
    assert_eq!(starts(&t), before);
}

#[test]
fn opening_another_project_stops_watching_the_first() {
    let Watching { t, root } = watching();
    let clean = fixture("clean");
    assert!(t.runner.open(&clean.root).unwrap());
    let before = starts(&t);
    save(&root, "w1");
    settle(&t);
    assert_eq!(starts(&t), before);
}

#[test]
fn a_cancelled_open_keeps_the_projects_watcher() {
    let Watching { t, root } = watching();
    let clean = fixture("clean");
    let other = clean.root.clone();
    let opening = t.bg(move |r| r.open(&other));
    until(|| t.java_up());
    t.runner.cancel();
    assert!(!opening.join().unwrap().unwrap());
    save(&root, "w1");
    finished(&t, 1);
    assert_eq!(t.last("runStarted")["selection"], "f:tests/w1.feature");
}

#[test]
fn watches_a_project_whose_dry_run_failed_and_runs_the_fix() {
    let root = fixture("tags");
    let t = setup();
    save_as(&root, "w1", BROKEN);
    save_project_watch(&t.data_dir, &root.root, true).unwrap();
    assert!(t.runner.open(&root.root).unwrap());
    assert!(!t.last("project")["error"].is_null());
    assert_eq!(t.last("watch")["on"], true);
    save(&root, "w1");
    finished(&t, 1);
    assert_eq!(t.last("runStarted")["selection"], "f:tests/w1.feature");
}

#[test]
fn refuses_the_toggle_while_a_project_is_being_opened() {
    let root = fixture("tags");
    let t = setup();
    let runner = t.runner.clone();
    t.on_event(move |e| {
        if matches!(e, AppEvent::Opening { root: Some(_) }) {
            runner.set_watch(true);
        }
    });
    assert!(t.runner.open(&root.root).unwrap());
    assert_eq!(t.notices(), ["A run is already in progress"]);
    assert_eq!(t.last("watch")["on"], false);
    assert!(!load_project_watch(&t.data_dir, &root.root));
}

#[test]
fn a_watcher_error_stops_watch_mode_in_memory_only() {
    let root = with_paths();
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    t.runner.set_watch(true);
    fs::remove_dir_all(root.join("tests")).unwrap();
    stopped(&t);
    assert_eq!(t.last("watch")["on"], false);
    assert!(load_project_watch(&t.data_dir, &root.root));
}

#[test]
fn dispose_stops_it_before_a_waiting_batch_runs() {
    let Watching { t, root } = watching();
    let before = starts(&t);
    save(&root, "w1");
    t.runner.dispose();
    thread::sleep(QUIET + Duration::from_millis(500));
    assert_eq!(starts(&t), before);
    assert!(!watcher_armed(&t));
}

// a drain that gets in between the toggle, or a quit, and the closing of the watcher
#[test]
fn a_save_waiting_starts_nothing_once_watch_mode_is_off_or_grace_is_quitting() {
    let Watching { t, root } = watching();
    let before = starts(&t);
    let drain_with_a_save = || {
        lock(&t.runner.0.waiting).insert(root.join("tests/tags.feature"));
        t.runner.0.drain();
    };
    lock(&t.runner.0.state).watching = false;
    drain_with_a_save();
    assert_eq!(starts(&t), before, "off");
    lock(&t.runner.0.state).watching = true;
    lock(&t.runner.0.ctl).disposing = true;
    drain_with_a_save();
    assert_eq!(starts(&t), before, "quitting");
}

// R1.3 on: a watcher that failed is gone, so turning watch mode on again watches
#[test]
fn turned_on_again_after_an_error_it_watches() {
    let root = with_paths();
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    t.runner.set_watch(true);
    fs::remove_dir_all(root.join("tests")).unwrap();
    stopped(&t);
    fs::create_dir(root.join("tests")).unwrap();
    t.runner.set_watch(true);
    save(&root, "w1");
    finished(&t, 1);
    assert_eq!(t.last("runStarted")["selection"], "f:tests/w1.feature");
}

// R2.4
#[test]
fn refresh_watches_the_feature_roots_of_the_new_discovery() {
    let root = with_paths();
    write(root.path(), &[("more/keep.txt", "")]);
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    t.runner.set_watch(true);
    write(root.path(), &[("karate-pom.json", r#"{"paths":["tests","more"]}"#)]);
    t.runner.refresh().unwrap();
    write(
        root.path(),
        &[("more/m1.feature", &format!("Feature: m1\n\n{PASSING}"))],
    );
    finished(&t, 1);
    assert_eq!(t.last("runStarted")["selection"], "f:more/m1.feature");
}

// the watcher's callbacks only post: `close` waits for one that is running, whatever its caller holds
#[test]
fn the_watchers_callbacks_never_wait_on_a_lock_its_closer_holds() {
    let root = with_paths();
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    t.runner.set_watch(true);
    let mut slot = lock(&t.runner.0.watcher);
    let state = lock(&t.runner.0.state);
    save(&root, "w1");
    until_waiting(&t); // on_saved ran with both held
    let armed = slot.take(); // and held on, as `arm` holds it while it closes a watcher
    fs::remove_dir_all(root.join("tests")).unwrap(); // on_error
    thread::sleep(Duration::from_secs(1));
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        drop(armed);
        let _ = tx.send(());
    });
    rx.recv_timeout(Duration::from_secs(10))
        .expect("close returned while its caller held both locks");
    lock(&t.runner.0.waiting).clear(); // no run of it as the test ends
    drop((slot, state));
}
