//! `runner.discovery.test.ts`.
use super::*;
use crate::karate::command::OUT_OF_MEMORY;
use crate::model::Layout;
use crate::model::events::Project;
use crate::runner::dry_run_error;
use std::sync::OnceLock;

#[test]
fn opens_the_clean_project() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let p = t.last_event("project");
    let AppEvent::Project {
        project, tree, error, ..
    } = &p
    else {
        unreachable!()
    };
    assert_eq!(
        project,
        &Some(Project {
            root: fx.root.clone(),
            layout: Layout::Standalone
        })
    );
    assert_eq!(leaves(tree.as_ref()), 9);
    assert_eq!(error, &None);
    assert_eq!(t.runner.current(), Some(p));
}

#[test]
fn opens_the_maven_fixture_and_runs_it() {
    let t = setup();
    let fx = fixture("maven");
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.last("project")["project"]["layout"], "maven");
    t.runner.run("d:").unwrap();
    let done = t.last("runFinished");
    assert_eq!(done["outcome"], "finished");
    assert_match(&done["summary"], json!({ "scenariosPassed": 1, "scenariosFailed": 0 }));
}

#[test]
fn reports_a_syntax_error() {
    let t = setup();
    let fx = fixture("broken");
    t.runner.open(&fx.root).unwrap();
    let p = t.last("project");
    assert_eq!(p["tree"], Value::Null);
    assert!(p["error"].as_str().unwrap().contains("syntax.feature"));
}

#[test]
fn a_dry_run_that_ran_out_of_heap_says_so_one_that_only_names_the_error_does_not() {
    let heap = "Terminating due to java.lang.OutOfMemoryError: Java heap space";
    assert_eq!(dry_run_error(&["parsing tests/a.feature", heap]), OUT_OF_MEMORY);
    let named = r#"  "error": "java.lang.OutOfMemoryError: Java heap space" in tests/a.feature"#;
    assert_eq!(dry_run_error(&["start", named]), named);
}

#[test]
fn opens_an_empty_folder() {
    let t = setup();
    let root = temp("kk-empty-");
    t.runner.open(&root.path().to_string_lossy()).unwrap();
    let p = t.last_event("project");
    let AppEvent::Project { tree, error, .. } = &p else {
        unreachable!()
    };
    assert_eq!(leaves(tree.as_ref()), 0);
    assert_eq!(error, &None);
}

#[test]
fn open_on_a_missing_folder_emits_an_error() {
    let t = setup();
    let missing = std::env::temp_dir().join("kk-does-not-exist");
    let missing = missing.to_string_lossy();
    assert!(!t.runner.open(&missing).unwrap());
    // the recent list check runs on a thread of its own: it may or may not have arrived
    let events: Vec<AppEvent> = t
        .events()
        .into_iter()
        .filter(|e| !matches!(e, AppEvent::Recent { .. }))
        .collect();
    assert_eq!(
        events,
        vec![
            AppEvent::Busy { busy: true },
            AppEvent::Watch { on: false },
            AppEvent::Project {
                project: None,
                tree: None,
                error: Some(format!("Folder not found: {missing}")),
                subfolders: None,
            },
            AppEvent::Flaky { tests: Vec::new() },
            AppEvent::Busy { busy: false },
        ]
    );
}

#[test]
fn replay_restores_a_window_that_loads_during_discovery() {
    let t = setup();
    assert_eq!(
        t.runner.replay(),
        vec![AppEvent::Opening { root: None }, AppEvent::Busy { busy: false }]
    );
    let fx = fixture("clean");
    // The window may load before the first project event: it still learns the runner is busy. It loads here as the
    // folder starts to be discovered.
    let during: Arc<OnceLock<Vec<AppEvent>>> = Arc::default();
    {
        let (runner, during) = (t.runner.clone(), Arc::clone(&during));
        t.on_event(move |e| {
            if matches!(e, AppEvent::Opening { root: Some(_) }) {
                let _ = during.set(runner.replay());
            }
        });
    }
    t.runner.open(&fx.root).unwrap();
    assert_eq!(
        during.get().unwrap(),
        &vec![
            AppEvent::Opening {
                root: Some(fx.root.clone())
            },
            AppEvent::Busy { busy: true },
        ]
    );
    assert_eq!(t.runner.replay(), idle_replay(&t, no_filter(), 1));
}

/// Cancels the operation going on when the first event `at` matches is emitted: before its dry run starts, as a cancel
/// right after Electron Grace's `open` and `refresh`, which first await the libraries.
pub(super) fn cancel_at(t: &TestRunner, at: impl Fn(&AppEvent) -> bool + Send + Sync + 'static) {
    let runner = t.runner.clone();
    let done = std::sync::Once::new();
    t.on_event(move |e| {
        if at(e) {
            done.call_once(|| runner.cancel());
        }
    });
}

pub(super) fn opening(e: &AppEvent) -> bool {
    matches!(e, AppEvent::Opening { root: Some(_) })
}

pub(super) fn busy(e: &AppEvent) -> bool {
    matches!(e, AppEvent::Busy { busy: true })
}

/// Waits for the operation's dry run Java to run, then cancels it: Electron Grace's `run`, `setEnv` and `rerun` start
/// their dry run before their first `await`, so the cancel that follows kills it.
pub(super) fn cancel_in_dry_run(t: &TestRunner) {
    until(|| t.java_up());
    t.runner.cancel();
}

#[test]
fn cancel_during_open_keeps_the_project_open_before() {
    let t = setup();
    let (clean, maven) = (fixture("clean"), fixture("maven"));
    assert!(t.runner.open(&clean.root).unwrap());
    let before = t.runner.current();
    cancel_at(&t, opening);
    assert!(!t.runner.open(&maven.root).unwrap());
    assert_eq!(Some(t.last_event("project")), before);
    assert_eq!(t.runner.current(), before);
    assert_eq!(t.notices(), ["Cancelled"]);
    // the recent list is sent after its folder checks, so it may come after busy
    assert_eq!(t.last_event("busy"), AppEvent::Busy { busy: false });
    // the restored project is the one that runs
    t.runner.run("s:tests/basic.feature#1").unwrap();
    assert_eq!(t.count("runStarted"), 1);
}

#[test]
fn cancel_during_the_first_open_leaves_no_project() {
    let t = setup();
    let fx = fixture("clean");
    cancel_at(&t, opening);
    assert!(!t.runner.open(&fx.root).unwrap());
    assert_eq!(t.all("project"), vec![json(&no_project())]);
    assert_eq!(t.runner.current(), None);
    t.runner.refresh().unwrap(); // no project: nothing to refresh
    assert_eq!(t.count("busy"), 2);
}

#[test]
fn cancel_during_refresh_keeps_the_tree() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let before = t.runner.current();
    cancel_at(&t, busy);
    t.runner.refresh().unwrap();
    assert_eq!(Some(t.last_event("project")), before);
    assert_eq!(t.notices(), ["Cancelled"]);
}

#[test]
fn opens_a_project_whose_path_has_spaces_and_accents() {
    let t = setup();
    let base = temp("kk-intl-");
    let root = base.path().join("Mes tests").join("équipe");
    let fx = fixture("clean");
    copy_dir(fx.path(), &root);
    t.runner.open(&root.to_string_lossy()).unwrap();
    assert_eq!(t.last("project")["error"], Value::Null);
    assert_eq!(leaves(Some(&t.tree())), 9);
}
