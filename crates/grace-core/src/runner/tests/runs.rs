//! `runner.runs.test.ts`.
use super::discovery::cancel_in_dry_run;
use super::*;
use crate::karate::command::OUT_OF_MEMORY;
use std::collections::BTreeSet;

fn has_status(t: &TestRunner, status: &str) -> bool {
    t.all("scenarios")
        .iter()
        .any(|e| e["updates"].as_array().unwrap().iter().any(|u| u["status"] == status))
}

#[test]
fn says_so_when_java_runs_out_of_memory_and_the_next_run_does_not() {
    let t = setup();
    let fx = fixture("clean");
    // One array over the 2 GB ceiling: Java refuses it at once, without using the memory.
    fs::write(
        fx.join("tests/big.feature"),
        [
            "Feature: big",
            "",
            "Scenario: asks for more than the limit",
            "  * def bytes = Java.type('java.lang.reflect.Array').newInstance(Java.type('java.lang.Byte').TYPE, 2147483000)",
            "",
            "Scenario: after it",
            "  * print 'after it ran'",
            "",
        ]
        .join("\n"),
    )
    .unwrap();
    t.runner.open(&fx.root).unwrap();
    t.runner.run("f:tests/big.feature").unwrap();
    assert_match(&t.last("runFinished"), json!({ "outcome": "failed", "exitCode": 3 }));
    assert!(printed_has(&t.events(), "java.lang.OutOfMemoryError"));
    assert!(!printed_has(&t.events(), "after it ran"));
    assert_eq!(t.last_notice(), OUT_OF_MEMORY);
    let notices = t.count("notice");
    t.runner.run(&t.key("after it")).unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    assert_eq!(t.count("notice"), notices);
}

#[test]
fn runs_all_with_live_progress() {
    let t = setup();
    let (clean, maven) = (fixture("clean"), fixture("maven"));
    t.runner.open(&clean.root).unwrap();
    assert!(!t.runner.is_busy());
    let run = t.bg(|r| r.run("d:"));
    until(|| t.runner.is_busy());
    run.join().unwrap().unwrap();
    assert!(!t.runner.is_busy());
    assert_match(&t.last("runStarted"), json!({ "selection": "d:", "total": 9 }));
    let types: Vec<Value> = t.json().iter().map(|e| e["type"].clone()).collect();
    let first_update = types.iter().position(|k| k == "scenarios").expect("a scenarios event");
    assert!(first_update < types.iter().position(|k| k == "runFinished").unwrap());
    let done = t.last("runFinished");
    assert_match(
        &done,
        json!({ "outcome": "finished", "exitCode": 1, "reportAvailable": true }),
    );
    assert_match(&done["summary"], json!({ "scenariosPassed": 8, "scenariosFailed": 1 }));
    let report = t.runner.report_path().expect("a report");
    assert!(report.exists());
    t.runner.open(&maven.root).unwrap();
    assert_eq!(t.runner.report_path(), None);
}

#[test]
fn runs_one_outline_row() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run("s:tests/outline.feature#1.2").unwrap();
    assert_match(
        &t.last("runStarted"),
        json!({ "total": 1, "keys": ["s:tests/outline.feature#1.2"] }),
    );
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 1, "scenariosPassed": 1 }),
    );
}

#[test]
fn runs_the_row_clicked_when_its_table_has_comment_and_blank_lines() {
    let t = setup();
    let fx = fixture("edge");
    t.runner.open(&fx.root).unwrap();
    let key = "s:tests/rows.feature#1.2";
    t.runner.run(key).unwrap();
    assert_match(&t.last("runStarted"), json!({ "keys": [key], "total": 1 }));
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 1, "scenariosPassed": 1 }),
    );
    let finals: Vec<Value> = t
        .all("scenarios")
        .iter()
        .flat_map(|e| e["updates"].as_array().unwrap().clone())
        .filter(|u| u["status"] != "running")
        .collect();
    assert_match(&Value::Array(finals), json!([{ "key": key, "status": "passed" }]));
}

#[test]
fn refuses_a_row_of_a_dynamic_examples_table() {
    let t = setup();
    let fx = fixture("edge");
    t.runner.open(&fx.root).unwrap();
    let rows = by_name(&t.tree(), "dynamic <a>").children;
    assert_eq!(rows.iter().map(|r| r.line).collect::<Vec<_>>(), [None, None]);
    // data-driven view R1.2
    assert_eq!(
        rows.iter().map(|r| json!(r.example)).collect::<Vec<_>>(),
        [json!({ "a": "1" }), json!({ "a": "2" })]
    );
    t.runner.run(&rows[0].key).unwrap();
    assert_eq!(t.last_notice(), "Run the outline to run its rows");
    assert_eq!(t.count("runStarted"), 0);
}

#[test]
fn re_discovers_before_running() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let old_key = t.key("failing match");
    let file = fx.join("tests/basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    let at = text.find("@smoke").unwrap();
    let edited = format!(
        "{}Scenario: inserted first\n  * def y = 2\n\n{}",
        &text[..at],
        &text[at..]
    );
    fs::write(&file, edited).unwrap();
    t.runner.run(&old_key).unwrap();
    assert_eq!(t.count("project"), 2);
    let tree = t.tree();
    assert_eq!(leaves(Some(&tree)), 10);
    let new_key = by_name(&tree, "failing match").key;
    assert_ne!(new_key, old_key);
    assert_match(
        &t.last("runStarted"),
        json!({ "selection": new_key, "keys": [new_key], "total": 1 }),
    );
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 1, "scenariosFailed": 1 }),
    );
    let updated: BTreeSet<String> = t
        .all("scenarios")
        .iter()
        .flat_map(|e| e["updates"].as_array().unwrap().clone())
        .map(|u| u["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(updated, BTreeSet::from([new_key]));
}

#[test]
fn refuses_a_test_that_changed() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let old_key = t.key("failing match");
    let file = fx.join("tests/basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, text.replacen("failing match", "failing compare", 1)).unwrap();
    t.runner.run(&old_key).unwrap();
    assert_eq!(t.last_notice(), "This test changed; the tree was refreshed");
    assert_eq!(t.count("runStarted"), 0);
}

#[test]
fn rejects_a_run_while_another_is_active() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let first = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    t.runner.run("d:").unwrap();
    t.runner.refresh().unwrap();
    assert_eq!(
        t.notices(),
        ["A run is already in progress", "A run is already in progress"]
    );
    t.runner.cancel();
    first.join().unwrap().unwrap();
    assert_eq!(t.count("runStarted"), 1);
    assert_eq!(t.count("project"), 2); // open + the first run's re-discovery
    assert_eq!(t.last("runFinished")["outcome"], "cancelled");
}

#[test]
fn cancel_during_the_pre_run_discovery_runs_nothing() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("d:"));
    cancel_in_dry_run(&t);
    run.join().unwrap().unwrap();
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(t.count("runFinished"), 0);
    assert_eq!(t.notices(), ["Run cancelled"]);
    assert_eq!(java_command_lines(&fx.root), Vec::<String>::new());
}

#[test]
fn cancel_interrupts_and_leaves_no_java() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let pid_file = t.data_dir.join("running.pid");
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| has_status(&t, "running"));
    assert!(pid_file.exists());
    t.runner.cancel();
    run.join().unwrap().unwrap();
    assert_match(
        &t.last("runFinished"),
        json!({ "outcome": "cancelled", "reportAvailable": false }),
    );
    assert!(has_status(&t, "interrupted"));
    assert_eq!(t.runner.report_path(), None);
    assert_eq!(java_command_lines(&fx.root), Vec::<String>::new());
    assert!(!pid_file.exists());
}

#[test]
fn does_not_modify_the_project() {
    let t = setup();
    let fx = fixture("clean");
    let before = hash_tree(fx.path());
    t.runner.open(&fx.root).unwrap();
    t.runner.run("d:").unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    let after = hash_tree(fx.path());
    let outside = |m: BTreeMap<String, String>| -> BTreeMap<String, String> {
        m.into_iter()
            .filter(|(p, _)| !p.split('/').any(|part| part == "karate-temp"))
            .collect()
    };
    assert_eq!(outside(after), outside(before));
}

#[test]
fn a_pid_file_that_cannot_be_written_fails_the_run_and_leaves_no_java() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let pid_file = t.data_dir.join("running.pid");
    fs::create_dir(&pid_file).unwrap(); // writing a file there fails
    t.runner.run("f:tests/slow.feature").unwrap();
    assert_match(
        &t.last("runFinished"),
        json!({ "outcome": "failed", "reportAvailable": false }),
    );
    assert!(t.last_notice().contains(&folder_write_error()), "{}", t.last_notice());
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(java_command_lines(&fx.root), Vec::<String>::new());
    fs::remove_dir(&pid_file).unwrap();
}

#[test]
fn dispose_kills_a_run_and_removes_the_pid_file() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    t.runner.dispose();
    run.join().unwrap().unwrap();
    assert_eq!(java_command_lines(&fx.root), Vec::<String>::new());
    assert!(!t.data_dir.join("running.pid").exists());
}

/// Rust-native: a cancel that comes after the first walk's check, just before Java starts, starts none: the check and
/// the start are one step for `cancel` (`spawn`). The run's folder goes, as the folder of a run that ended early does.
#[test]
fn a_cancel_just_before_java_starts_starts_none() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.save_properties(&crate::model::env::PropertiesEdit {
        shared: vec![crate::model::env::RowEdit::Value {
            name: "timeout".into(),
            value: "5000".into(),
            secret: false,
            env_var: None,
        }],
        ..Default::default()
    });
    let inner = Arc::downgrade(&t.runner.0);
    t.runner.set_spawn_hook(Arc::new(move |s: &mut Spawn| {
        if !s.dry
            && let Some(inner) = inner.upgrade()
        {
            Runner(inner).cancel();
        }
    }));
    t.runner.run(&t.key("always")).unwrap();
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(t.count("runFinished"), 0);
    assert_eq!(t.last_notice(), "Run cancelled");
    assert_eq!(folders(&t.data_dir, &fx.root), Vec::<String>::new());
    assert!(!project_dir(&t.data_dir, &fx.root).join("run-args.txt").exists());
    assert_eq!(java_command_lines(&fx.root), Vec::<String>::new());
}
