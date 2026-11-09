//! `runner.history.test.ts`. `holdFolder` and an exclusive lock, which only Windows has, are the `history::held` seam
//! here: the folder fails to go, on every platform.
use super::discovery::cancel_in_dry_run;
use super::*;
use crate::history::held;
use crate::settings::load_project_env;

fn has_status(t: &TestRunner, status: &str) -> bool {
    t.all("scenarios")
        .iter()
        .any(|e| e["updates"].as_array().unwrap().iter().any(|u| u["status"] == status))
}

fn ids_of(runs: &Value) -> Vec<String> {
    runs.as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_run_writes_run_json_in_its_own_folder() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run("d:").unwrap();
    let ids = folders(&t.data_dir, &fx.root);
    assert_eq!(ids.len(), 1);
    let id = &ids[0];
    assert_eq!(t.last("runStarted")["runId"], json!(id));
    let r = record(&t.data_dir, &fx.root, id);
    assert_match(
        &r,
        json!({
            "version": 1,
            "startedAt": t.last("runStarted")["startedAt"],
            "env": null,
            "threads": 1,
            "filter": json(&AppEvent::Filter { filter: no_filter() })["filter"],
            "selection": { "kind": "folder", "file": "", "line": null, "key": "d:" },
            "failedOf": null,
            "outcome": "finished",
            "exitCode": 1,
        }),
    );
    assert!(r["durationMillis"].as_f64().unwrap() > 0.0);
    let tests = r["tests"].as_array().unwrap();
    assert_eq!(tests.len(), 9);
    let failed: Vec<Value> = tests.iter().filter(|t| t["status"] == "failed").cloned().collect();
    assert_match(
        &Value::Array(failed),
        json!([{ "kind": "scenario", "file": "tests/basic.feature", "name": "failing match" }]),
    );
    assert_eq!(tests.iter().filter(|t| t["status"] == "passed").count(), 8);
    assert!(!project_dir(&t.data_dir, &fx.root).join("run").exists());
    // listed once the run's end events were sent, with its report
    let types: Vec<Value> = t.json().iter().map(|e| e["type"].clone()).collect();
    let last_history = types.iter().rposition(|k| k == "history").unwrap();
    assert!(last_history > types.iter().position(|k| k == "runFinished").unwrap());
    assert_match(
        &t.last("history")["runs"],
        json!([{ "id": id, "passed": 8, "failed": 1, "total": 9, "outcome": "finished", "reportAvailable": true }]),
    );
    assert_eq!(
        t.runner.report_path(),
        Some(record_file(&t.data_dir, &fx.root, id).with_file_name("karate-summary.html"))
    );
}

#[test]
fn run_json_holds_no_secret() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    t.runner.save_properties(
        &crate::model::env::parse_properties_edit(&json!({
            "shared": [],
            "byEnv": { "qa": [{ "name": "api.token", "value": SECRET, "secret": true }] },
        }))
        .unwrap(),
    );
    t.runner.run(&t.key("always")).unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    let id = &folders(&t.data_dir, &fx.root)[0];
    let text = fs::read_to_string(record_file(&t.data_dir, &fx.root, id)).unwrap();
    assert!(!text.contains(SECRET));
    assert_match(
        &serde_json::from_str(&text).unwrap(),
        json!({ "env": "qa", "outcome": "finished" }),
    );
}

#[test]
fn two_runs_keep_two_folders_newest_first() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    let key = t.key("always");
    t.runner.run(&key).unwrap();
    t.runner.run(&key).unwrap();
    let mut ids = folders(&t.data_dir, &fx.root);
    assert_eq!(ids.len(), 2);
    let runs = t.last("history")["runs"].clone();
    ids.reverse();
    assert_eq!(ids_of(&runs), ids);
    assert!(runs[0]["startedAt"].as_f64() > runs[1]["startedAt"].as_f64());
}

#[test]
fn retention_keeps_20() {
    let t = setup();
    let fx = fixture("env");
    let seeded = seed(&t.data_dir, &fx.root, 20, |_| {});
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.last("history")["runs"].as_array().unwrap().len(), 20);
    t.runner.run(&t.key("always")).unwrap();
    let ids = folders(&t.data_dir, &fx.root);
    assert_eq!(ids.len(), 20);
    assert!(!ids.contains(&seeded[0]));
    assert_eq!(ids[..19], seeded[1..]);
    let runs = t.last("history")["runs"].clone();
    assert_eq!(runs.as_array().unwrap().len(), 20);
    assert_eq!(runs[0]["id"], json!(ids[19]));
}

/// Saves `api.token` as a secret, then makes its ciphertext unreadable: what the tests of a refused run start from.
fn unreadable_secret(fx: &Fixture) -> Arc<TempDir> {
    let first = setup();
    first.runner.open(&fx.root).unwrap();
    first.runner.save_properties(
        &crate::model::env::parse_properties_edit(&json!({
            "shared": [{ "name": "api.token", "value": SECRET, "secret": true }],
            "byEnv": {},
        }))
        .unwrap(),
    );
    first.runner.dispose();
    let settings = first.data_dir.join("settings.json");
    let cipher = FakeSecretBox::new(true).encrypt(SECRET).unwrap();
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, text.replace(&cipher, "Zm9v")).unwrap();
    Arc::clone(&first.data)
}

#[test]
fn a_refused_run_creates_no_folder() {
    let fx = fixture("env");
    let data = unreadable_secret(&fx);
    let t = setup_with(Setup {
        data: Some(data),
        ..Setup::default()
    });
    t.runner.open(&fx.root).unwrap();
    t.runner.run("d:").unwrap();
    assert!(t.last_notice().contains("can't be read"));
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(folders(&t.data_dir, &fx.root), Vec::<String>::new());
    assert_eq!(t.last("history")["runs"], json!([]));
    assert_eq!(t.count("history"), 1); // the one of open
}

#[test]
fn a_cancelled_run_is_recorded_as_cancelled_with_not_run_for_tests_that_never_started() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| has_status(&t, "running"));
    t.runner.cancel();
    run.join().unwrap().unwrap();
    let id = folders(&t.data_dir, &fx.root)[0].clone();
    let r = record(&t.data_dir, &fx.root, &id);
    assert_match(&r, json!({ "outcome": "cancelled", "exitCode": null }));
    let tests: Vec<(Value, Value)> = r["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| (t["name"].clone(), t["status"].clone()))
        .collect();
    assert_eq!(
        tests,
        [
            (json!("slow loop"), json!("interrupted")),
            (json!("smoke and slow"), json!("notRun"))
        ]
    );
    assert_match(
        &t.last("history")["runs"],
        json!([{ "id": id, "outcome": "cancelled", "passed": 0, "failed": 0, "total": 2, "reportAvailable": false }]),
    );
}

#[test]
fn dispose_records_the_killed_run_as_cancelled() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let run = t.bg(|r| r.run("f:tests/slow.feature"));
    until(|| t.count("runStarted") > 0);
    t.runner.dispose();
    // written before dispose returned, not when the run's call returns
    let ids = folders(&t.data_dir, &fx.root);
    assert_eq!(ids.len(), 1);
    assert_match(
        &record(&t.data_dir, &fx.root, &ids[0]),
        json!({ "outcome": "cancelled" }),
    );
    run.join().unwrap().unwrap();
}

#[test]
fn replay_includes_history() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.last("history")["runs"], json!([]));
    t.runner.run(&t.key("always")).unwrap();
    assert_eq!(t.last("history")["runs"].as_array().unwrap().len(), 1);
    assert_eq!(t.runner.replay(), idle_replay(&t, no_filter(), 1));
}

#[test]
fn every_rediscovery_sends_history_also_a_cancelled_one() {
    let t = setup();
    let fx = fixture("env");
    let id = seed(&t.data_dir, &fx.root, 1, |_| {}).remove(0);
    t.runner.open(&fx.root).unwrap();
    assert_eq!(ids_of(&t.last("history")["runs"]), [id.as_str()]);
    t.runner.refresh().unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    assert_eq!(t.count("history"), 3);
    let from = t.len();
    let switching = t.bg(|r| r.set_env(Some("prod")));
    cancel_in_dry_run(&t);
    switching.join().unwrap().unwrap();
    assert_eq!(t.json()[from..].iter().filter(|e| e["type"] == "history").count(), 1);
    assert_eq!(ids_of(&t.last("history")["runs"]), [id]);
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env.as_deref(), Some("qa"));
}

#[test]
fn a_run_that_fails_to_start_is_recorded_as_failed() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    let pid_file = t.data_dir.join("running.pid");
    fs::create_dir(&pid_file).unwrap(); // writing a file there fails
    let before = super::super::now_ms();
    t.runner.run("f:tests/slow.feature").unwrap();
    assert_eq!(t.count("runStarted"), 0);
    let id = folders(&t.data_dir, &fx.root)[0].clone();
    let r = record(&t.data_dir, &fx.root, &id);
    assert_match(&r, json!({ "outcome": "failed", "exitCode": null }));
    // the folder's time: milliseconds, as `Date#getTime` has them
    let started_at = r["startedAt"].as_f64().unwrap();
    assert!(started_at >= before.floor());
    assert!(started_at <= super::super::now_ms());
    let statuses: Vec<Value> = r["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["status"].clone())
        .collect();
    assert_eq!(statuses, [json!("notRun"), json!("notRun")]);
    assert_match(
        &t.last("history")["runs"],
        json!([{ "id": id, "outcome": "failed", "total": 2 }]),
    );
    fs::remove_dir(&pid_file).unwrap();
}

#[test]
fn an_error_before_the_run_ends_leaves_no_folder() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.save_properties(
        &crate::model::env::parse_properties_edit(&json!({
            "shared": [{ "name": "timeout", "value": "5000", "secret": false }],
            "byEnv": {},
        }))
        .unwrap(),
    );
    let blocker = project_dir(&t.data_dir, &fx.root).join("run-args.txt");
    fs::create_dir_all(&blocker).unwrap(); // writing the argument file there fails
    assert!(t.runner.run(&t.key("always")).is_err());
    assert_eq!(folders(&t.data_dir, &fx.root), Vec::<String>::new());
    fs::remove_dir(&blocker).unwrap();
}

#[test]
fn a_record_that_cannot_be_written_is_a_notice_not_an_error() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") > 0);
    // a folder where the file goes: writing it fails
    let id = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    fs::create_dir(record_file(&t.data_dir, &fx.root, &id)).unwrap();
    run.join().unwrap().unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    assert!(t.last_notice().contains(&folder_write_error()), "{}", t.last_notice());
    assert!(t.runner.report_path().is_some());
}

#[test]
fn the_history_sample_project_fails_where_the_spec_says() {
    let t = setup();
    let fx = fixture("history");
    t.runner.open(&fx.root).unwrap();
    t.runner.run("d:").unwrap();
    let id = folders(&t.data_dir, &fx.root)[0].clone();
    let failed: Vec<Value> = record(&t.data_dir, &fx.root, &id)["tests"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["status"] == "failed")
        .map(|t| t["name"].clone())
        .collect();
    assert_eq!(failed, [json!("fixable"), json!("row 2"), json!("dynamic 2")]);
    assert_match(
        &t.last("history")["runs"],
        json!([{ "passed": 3, "failed": 3, "total": 6 }]),
    );
}

#[test]
fn clear_history_deletes_the_runs_and_emits_an_empty_history() {
    let t = setup();
    let fx = fixture("env");
    seed(&t.data_dir, &fx.root, 2, |_| {});
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("always")).unwrap();
    assert!(t.runner.report_path().is_some());
    assert_eq!(t.last("history")["runs"].as_array().unwrap().len(), 3);

    // refused while busy
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") == 2);
    t.runner.clear_history();
    assert_eq!(t.last_notice(), "A run is already in progress");
    assert_eq!(folders(&t.data_dir, &fx.root).len(), 4);
    t.runner.cancel();
    run.join().unwrap().unwrap();

    // a process running inside a run folder holds it
    let held_id = folders(&t.data_dir, &fx.root)[0].clone();
    let held_dir = project_dir(&t.data_dir, &fx.root).join("runs").join(&held_id);
    held::hold(&held_dir);
    t.runner.clear_history();
    held::release(&held_dir);
    assert_eq!(t.last_notice(), "Some runs could not be deleted");
    assert_eq!(folders(&t.data_dir, &fx.root), [held_id]);

    let notices = t.count("notice");
    t.runner.clear_history();
    assert_eq!(t.count("notice"), notices);
    assert_eq!(folders(&t.data_dir, &fx.root), Vec::<String>::new());
    assert_eq!(t.last("history")["runs"], json!([]));
    assert_eq!(t.runner.report_path(), None);
}

#[test]
fn clear_history_keeps_the_toolbar_report_of_a_run_it_could_not_delete() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("always")).unwrap();
    let report = t.runner.report_path().expect("a report");
    // Windows' exclusive lock on the report keeps its folder: the folder is held
    let dir = report.parent().unwrap().to_path_buf();
    held::hold(&dir);
    t.runner.clear_history();
    held::release(&dir);
    assert_eq!(t.last_notice(), "Some runs could not be deleted");
    assert_match(&t.last("history")["runs"], json!([{ "reportAvailable": true }]));
    assert_eq!(t.runner.report_path(), Some(report));
    t.runner.clear_history();
    assert_eq!(folders(&t.data_dir, &fx.root), Vec::<String>::new());
    assert_eq!(t.runner.report_path(), None);
}

#[test]
fn no_report_path_or_export_source_for_a_cancelled_run_or_an_unknown_id() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("always")).unwrap();
    let done = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") == 2);
    t.runner.cancel();
    run.join().unwrap().unwrap();
    let cancelled = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    let runs = project_dir(&t.data_dir, &fx.root).join("runs");
    assert_eq!(
        t.runner.run_report_path(&done),
        Some(runs.join(&done).join("karate-summary.html"))
    );
    assert_eq!(t.runner.run_report_path(&cancelled), None);
    assert_eq!(t.runner.run_report_path("20200101-000000-000"), None);
    let escape = format!("{done}\\..\\{done}");
    let escape_slash = format!("{done}/../{done}");
    for id in [&escape, &escape_slash] {
        assert_eq!(t.runner.run_report_path(id), None);
    }
    assert_eq!(
        t.runner.export_source(&done),
        Some(super::super::ExportSource {
            dir: runs.join(&done),
            name: format!(
                "{}-report-{done}.zip",
                Path::new(&fx.root).file_name().unwrap().to_string_lossy()
            ),
        })
    );
    for id in [&cancelled, "20200101-000000-000", &escape, &escape_slash] {
        assert_eq!(t.runner.export_source(id), None, "{id}");
    }
}
