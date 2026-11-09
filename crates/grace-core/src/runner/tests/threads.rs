//! `runner.threads.test.ts`.
use super::discovery::{cancel_at, opening};
use super::*;
use crate::model::threads::THREADS_ERROR;
use crate::settings::load_project_threads;

#[test]
fn a_run_passes_the_count_and_discovery_passes_1() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_threads(&json!(2));
    assert_eq!(t.last_event("threads"), AppEvent::Threads { count: 2 });
    assert_eq!(load_project_threads(&t.data_dir, &fx.root), 2);
    t.runner.run("d:").unwrap();
    let run_id = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    let run_dir = format!("runs/{run_id}");
    assert_eq!(suite_threads(&events_file(&t.data_dir, &fx.root, &run_dir)), 2);
    assert_eq!(suite_threads(&events_file(&t.data_dir, &fx.root, "dryrun")), 1);
    assert_eq!(record(&t.data_dir, &fx.root, &run_id)["threads"], 2);
    assert_match(&t.last("history")["runs"][0], json!({ "id": run_id, "threads": 2 }));
}

#[test]
fn restores_the_count_on_open_per_project() {
    let first = setup();
    let (clean, maven) = (fixture("clean"), fixture("maven"));
    first.runner.open(&clean.root).unwrap();
    first.runner.set_threads(&json!(3));
    first.runner.dispose();
    let second = setup_with(Setup {
        data: Some(Arc::clone(&first.data)),
        ..Setup::default()
    });
    second.runner.open(&clean.root).unwrap();
    assert_eq!(second.last("threads")["count"], 3);
    assert!(second.runner.replay().contains(&AppEvent::Threads { count: 3 }));
    second.runner.open(&maven.root).unwrap();
    assert_eq!(second.last("threads")["count"], 1);
}

#[test]
fn a_cancelled_open_restores_the_count_before() {
    let t = setup();
    let (clean, maven) = (fixture("clean"), fixture("maven"));
    t.runner.open(&clean.root).unwrap();
    t.runner.set_threads(&json!(3));
    cancel_at(&t, opening);
    assert!(!t.runner.open(&maven.root).unwrap());
    assert_eq!(t.last("threads")["count"], 3);
}

#[test]
fn refuses_a_count_that_is_not_a_whole_number_from_1_to_32() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_threads(&json!(2));
    // NaN has no JSON form: the window's NaN arrives as null, which is in the list too
    for bad in [
        json!(0),
        json!(33),
        json!(2.5),
        Value::from(f64::NAN),
        json!("4"),
        Value::Null,
        json!(-1),
    ] {
        t.runner.set_threads(&bad);
        assert_eq!(t.last_notice(), THREADS_ERROR);
        assert_eq!(t.last("threads")["count"], 2);
        assert_eq!(load_project_threads(&t.data_dir, &fx.root), 2);
    }
}

#[test]
fn rejects_a_change_while_busy() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    let sent = t.count("threads");
    t.runner.set_threads(&json!(4));
    assert_eq!(t.last_notice(), "A run is already in progress");
    assert_eq!(t.count("threads"), sent + 1); // main re-sends the count it holds
    assert_eq!(t.last("threads")["count"], 1);
    assert_eq!(load_project_threads(&t.data_dir, &fx.root), 1);
    t.runner.cancel();
    run.join().unwrap().unwrap();
}

#[test]
fn re_run_uses_the_current_count_not_the_runs() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_threads(&json!(2));
    t.runner.run(&t.key("passing hello")).unwrap();
    let run_id = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    t.runner.set_threads(&json!(1));
    t.runner.rerun(&run_id).unwrap();
    let second = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    assert_ne!(second, run_id);
    assert_eq!(record(&t.data_dir, &fx.root, &run_id)["threads"], 2);
    assert_eq!(record(&t.data_dir, &fx.root, &second)["threads"], 1);
    let dir = format!("runs/{second}");
    assert_eq!(suite_threads(&events_file(&t.data_dir, &fx.root, &dir)), 1);
}

#[test]
fn applies_a_count_it_cannot_save() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let settings = t.data_dir.join("settings.json");
    fs::remove_file(&settings).unwrap(); // open saved it: replace it with a folder
    fs::create_dir(&settings).unwrap(); // writing a file there fails
    t.runner.set_threads(&json!(4));
    assert!(t.last_notice().contains("The thread count could not be saved"));
    assert_eq!(t.last("threads")["count"], 4);
}

#[test]
fn ignores_a_change_with_no_project() {
    let t = setup();
    t.runner.set_threads(&json!(4));
    assert_eq!(t.events(), Vec::<AppEvent>::new());
}
