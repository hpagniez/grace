//! `runner.fingerprint.test.ts`. A fake walk is a `Walk` closure; a walk that rejects is one that panics.
use super::*;
use crate::fingerprint::project_fingerprint;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

fn with_walk(walk: impl Fn(&Path) -> Option<String> + Send + Sync + 'static) -> TestRunner {
    setup_with(Setup {
        fingerprint: Some(Arc::new(walk)),
        ..Setup::default()
    })
}

fn a64() -> String {
    "a".repeat(64)
}

fn only_folder(t: &TestRunner, root: &str) -> String {
    let ids = folders(&t.data_dir, root);
    assert_eq!(ids.len(), 1, "{ids:?}");
    ids[0].clone()
}

#[test]
fn a_run_of_an_unchanged_project_stores_its_fingerprint() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    let id = only_folder(&t, &fx.root);
    let fingerprint = record(&t.data_dir, &fx.root, &id)["fingerprint"].clone();
    assert_ne!(fingerprint, Value::Null);
    assert_eq!(fingerprint, json!(project_fingerprint(fx.path())));
}

#[test]
fn a_file_saved_during_a_run_stores_no_fingerprint() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    fs::write(fx.join("note.txt"), "saved during the run").unwrap();
    run.join().unwrap().unwrap();
    let id = only_folder(&t, &fx.root);
    assert_eq!(record(&t.data_dir, &fx.root, &id)["fingerprint"], Value::Null);
}

#[test]
fn different_walks_store_no_fingerprint() {
    let walks = Arc::new(Mutex::new(vec!["a".repeat(64), "b".repeat(64)]));
    let fake = {
        let walks = Arc::clone(&walks);
        move |_: &Path| Some(lock(&walks).remove(0))
    };
    let t = with_walk(fake);
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    assert!(lock(&walks).is_empty());
    let id = only_folder(&t, &fx.root);
    assert_eq!(record(&t.data_dir, &fx.root, &id)["fingerprint"], Value::Null);
}

#[test]
fn a_cancel_during_the_first_walk_starts_no_java() {
    let slot: Arc<OnceLock<Runner>> = Arc::default();
    let fake = {
        let slot = Arc::clone(&slot);
        move |_: &Path| {
            slot.get().unwrap().cancel();
            Some(a64())
        }
    };
    let t = with_walk(fake);
    let _ = slot.set(t.runner.clone());
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(t.last_notice(), "Run cancelled");
    assert_eq!(folders(&t.data_dir, &fx.root), Vec::<String>::new());
}

#[test]
fn dispose_skips_the_second_walk() {
    let walks = Arc::new(AtomicU32::new(0));
    let fake = {
        let walks = Arc::clone(&walks);
        move |_: &Path| {
            walks.fetch_add(1, Ordering::SeqCst);
            Some(a64())
        }
    };
    let t = with_walk(fake);
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    t.runner.dispose();
    assert_eq!(walks.load(Ordering::SeqCst), 1);
    let id = only_folder(&t, &fx.root);
    assert_match(
        &record(&t.data_dir, &fx.root, &id),
        json!({ "outcome": "cancelled", "fingerprint": null }),
    );
    run.join().unwrap().unwrap();
}

#[test]
fn a_first_walk_that_gave_none_skips_the_second() {
    let walks = Arc::new(AtomicU32::new(0));
    let fake = {
        let walks = Arc::clone(&walks);
        move |_: &Path| {
            walks.fetch_add(1, Ordering::SeqCst);
            None
        }
    };
    let t = with_walk(fake);
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    assert_eq!(walks.load(Ordering::SeqCst), 1);
    let id = only_folder(&t, &fx.root);
    assert_match(
        &record(&t.data_dir, &fx.root, &id),
        json!({ "outcome": "finished", "fingerprint": null }),
    );
}

#[test]
fn the_stored_duration_ends_when_java_exits_not_after_the_second_walk() {
    let walks = Arc::new(AtomicU32::new(0));
    let second_walk_at = Arc::new(Mutex::new(0.0));
    let fake = {
        let (walks, at) = (Arc::clone(&walks), Arc::clone(&second_walk_at));
        move |_: &Path| {
            if walks.fetch_add(1, Ordering::SeqCst) + 1 == 2 {
                *lock(&at) = super::super::now_ms();
                thread::sleep(Duration::from_millis(300));
            }
            Some(a64())
        }
    };
    let t = with_walk(fake);
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    let id = only_folder(&t, &fx.root);
    let r = record(&t.data_dir, &fx.root, &id);
    // the walk starts right after the exit, so a run that ended there is over by then
    let ended = r["startedAt"].as_f64().unwrap() + r["durationMillis"].as_f64().unwrap();
    assert!(ended <= *lock(&second_walk_at));
}

#[test]
fn the_second_walk_comes_before_run_finished() {
    let slot: Arc<OnceLock<Arc<Sink>>> = Arc::default();
    let walks = Arc::new(AtomicU32::new(0));
    let finished_at_second_walk: Arc<OnceLock<bool>> = Arc::default();
    let fake = {
        let (slot, walks, seen) = (
            Arc::clone(&slot),
            Arc::clone(&walks),
            Arc::clone(&finished_at_second_walk),
        );
        move |_: &Path| {
            if walks.fetch_add(1, Ordering::SeqCst) + 1 == 2 {
                let finished = lock(&slot.get().unwrap().events)
                    .iter()
                    .any(|(e, _)| matches!(e, AppEvent::RunFinished { .. }));
                let _ = seen.set(finished);
            }
            Some(a64())
        }
    };
    let t = with_walk(fake);
    let _ = slot.set(Arc::clone(&t.sink));
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    assert_eq!(finished_at_second_walk.get(), Some(&false));
    let types: Vec<Value> = t.json().iter().map(|e| e["type"].clone()).collect();
    let last = |k: &str| types.iter().rposition(|x| x == k).unwrap();
    assert!(last("runFinished") < last("history"));
}

#[test]
fn a_cancel_during_the_second_walk_keeps_the_runs_outcome() {
    let slot: Arc<OnceLock<Runner>> = Arc::default();
    let walks = Arc::new(AtomicU32::new(0));
    let fake = {
        let (slot, walks) = (Arc::clone(&slot), Arc::clone(&walks));
        move |_: &Path| {
            if walks.fetch_add(1, Ordering::SeqCst) + 1 == 2 {
                slot.get().unwrap().cancel();
            }
            Some(a64())
        }
    };
    let t = with_walk(fake);
    let _ = slot.set(t.runner.clone());
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    let id = only_folder(&t, &fx.root);
    assert_match(
        &record(&t.data_dir, &fx.root, &id),
        json!({ "outcome": "finished", "fingerprint": a64() }),
    );
}

#[test]
fn a_failing_second_walk_still_ends_the_run() {
    let walks = Arc::new(AtomicU32::new(0));
    let fake = {
        let walks = Arc::clone(&walks);
        move |_: &Path| {
            if walks.fetch_add(1, Ordering::SeqCst) + 1 == 2 {
                panic!("walk failed");
            }
            Some(a64())
        }
    };
    let t = with_walk(fake);
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("passing hello")).unwrap();
    assert_eq!(t.count("runFinished"), 1);
    let id = only_folder(&t, &fx.root);
    assert_match(
        &record(&t.data_dir, &fx.root, &id),
        json!({ "outcome": "finished", "fingerprint": null }),
    );
}
