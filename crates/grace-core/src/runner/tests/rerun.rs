//! `runner.rerun.test.ts`.
use super::discovery::cancel_in_dry_run;
use super::*;
use crate::settings::{EnvSettings, StoredLists, StoredRow, load_project_env, load_project_filter, save_project_env};
use indexmap::IndexMap;
use std::collections::BTreeSet;

const GONE: &str = "This run is no longer in the history";

fn smoke() -> crate::model::tags::FilterState {
    filter(&["smoke"], &[], false)
}

fn names_and_statuses(r: &Value) -> Vec<(String, String)> {
    r["tests"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap().to_string(),
                t["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[test]
fn rerun_failed_runs_only_the_failed_tests_that_can_run() {
    let t = setup();
    let fx = fixture("history");
    t.runner.open(&fx.root).unwrap();
    t.runner.run("d:").unwrap();
    let first = t.last("history")["runs"][0].clone();
    fs::write(fx.join("flag.json"), r#"{ "fixed": true }"#).unwrap();
    let tree = t.tree();
    let keys = [by_name(&tree, "fixable").key, by_name(&tree, "row 2").key];
    let from = t.len();
    t.runner.rerun_failed(first["id"].as_str().unwrap()).unwrap();
    assert_match(&t.last("runStarted"), json!({ "keys": keys, "total": 2 }));
    let after = &t.json()[from..];
    let updated: BTreeSet<String> = after
        .iter()
        .filter(|e| e["type"] == "scenarios")
        .flat_map(|e| e["updates"].as_array().unwrap().clone())
        .map(|u| u["key"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(updated, BTreeSet::from(keys.clone()));
    let notices: Vec<&Value> = after
        .iter()
        .filter(|e| e["type"] == "notice")
        .map(|e| &e["message"])
        .collect();
    assert_eq!(
        notices,
        [&json!(
            "1 failed row of a dynamic Examples table can't be run alone. Run its outline."
        )]
    );
    let id = folders(&t.data_dir, &fx.root)[1].clone();
    let r = record(&t.data_dir, &fx.root, &id);
    assert_match(
        &r,
        json!({ "selection": null, "failedOf": first["startedAt"], "outcome": "finished" }),
    );
    assert_eq!(
        names_and_statuses(&r),
        pairs(&[("fixable", "passed"), ("row 2", "failed")])
    );
    assert_match(
        &t.last("history")["runs"][0],
        json!({ "id": id, "failedOf": first["startedAt"], "passed": 1, "failed": 1, "total": 2 }),
    );
}

#[test]
fn rerun_failed_passes_its_targets_in_a_file() {
    let t = setup();
    let fx = fixture("env");
    let gate = hold_slow(&fx.root);
    t.runner.open(&fx.root).unwrap();
    let slow = by_name(&t.tree(), "slow");
    let failed = stored(&slow, TestStatusFinal::Failed);
    let id = seed(&t.data_dir, &fx.root, 1, |r| {
        r.selection = None;
        r.failed_of = Some(1.0);
        r.tests = vec![failed.clone()];
    })
    .remove(0);
    let file = project_dir(&t.data_dir, &fx.root).join("run-targets.txt");
    let run = t.bg(move |r| r.rerun_failed(&id));
    until(|| t.count("runStarted") > 0);
    let target = format!(
        "{}:{}",
        fx.join("tests").join("env.feature").display(),
        slow.line.unwrap()
    );
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        format!("\"{}\"\r\n", target.replace('\\', "\\\\"))
    );
    let lines = java_command_lines(&fx.root);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(&format!("@{}", file.display())));
    assert!(!lines[0].contains("env.feature"));
    gate.release();
    run.join().unwrap().unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    assert_eq!(t.last("runStarted")["keys"], json!([slow.key]));
    assert!(!file.exists());
}

#[test]
fn rerun_switches_environment_and_filter_back() {
    let t = setup();
    let fx = fixture("history");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    t.runner.set_filter(smoke());
    t.runner.run("f:tests/fix.feature").unwrap();
    let green = t.key("stays green");
    assert_eq!(t.last("runStarted")["keys"], json!([green]));
    let id = t.last("history")["runs"][0]["id"].as_str().unwrap().to_string();
    t.runner.set_env(None).unwrap();
    t.runner.set_filter(no_filter());
    t.runner.rerun(&id).unwrap();
    assert_eq!(t.last("environment")["view"]["env"], "qa");
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env.as_deref(), Some("qa"));
    assert_eq!(t.last_event("filter"), AppEvent::Filter { filter: smoke() });
    assert_eq!(load_project_filter(&t.data_dir, &fx.root), smoke());
    assert_match(
        &t.last("runStarted"),
        json!({ "keys": [green], "total": 1, "env": "qa" }),
    );
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 1, "scenariosPassed": 1 }),
    );
    let second = folders(&t.data_dir, &fx.root)[1].clone();
    assert_match(
        &record(&t.data_dir, &fx.root, &second),
        json!({
            "env": "qa",
            "filter": { "include": ["smoke"], "exclude": [], "hideNonMatching": false },
            "selection": { "kind": "feature", "key": "f:tests/fix.feature" },
            "failedOf": null,
        }),
    );
}

#[test]
fn rerun_with_a_vanished_included_tag_runs_nothing_never_more() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_filter(filter(&["wip"], &[], false));
    t.runner.run("d:").unwrap();
    assert_eq!(t.last("runStarted")["total"], 2);
    let id = t.last("history")["runs"][0]["id"].as_str().unwrap().to_string();
    t.runner.set_filter(no_filter());
    let file = fx.join("tests/basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(
        &file,
        regex::Regex::new(" ?@wip").unwrap().replace_all(&text, "").into_owned(),
    )
    .unwrap();
    t.runner.rerun(&id).unwrap();
    assert_eq!(t.last_notice(), "Nothing to run");
    assert_eq!(t.count("runStarted"), 1);
    assert_eq!(folders(&t.data_dir, &fx.root), [id]);
}

#[test]
fn rerun_of_a_deleted_test_says_the_tests_of_this_run_no_longer_exist() {
    let t = setup();
    let fx = fixture("clean");
    t.runner.open(&fx.root).unwrap();
    t.runner.run(&t.key("failing match")).unwrap();
    let id = t.last("history")["runs"][0]["id"].as_str().unwrap().to_string();
    let file = fx.join("tests/basic.feature");
    let text = fs::read_to_string(&file).unwrap();
    // `[\s\S]*?(?=Scenario:)`: up to the next scenario, which the regex crate's lack of look-ahead keeps
    let re = regex::Regex::new(r"@wip\r?\nScenario: failing match[\s\S]*?Scenario:").unwrap();
    let edited = re.replace(&text, "Scenario:").into_owned();
    assert_ne!(edited, text);
    fs::write(&file, edited).unwrap();
    t.runner.rerun(&id).unwrap();
    assert_eq!(t.last_notice(), "The tests of this run no longer exist");
    assert_eq!(t.count("runStarted"), 1);
    assert_eq!(folders(&t.data_dir, &fx.root), [id]);
}

#[test]
fn rerun_of_a_rerun_failed_run_runs_all_its_tests_and_keeps_failed_of() {
    let t = setup();
    let fx = fixture("history");
    t.runner.open(&fx.root).unwrap();
    let tree = t.tree();
    let fixable = by_name(&tree, "fixable");
    let row2 = by_name(&tree, "row 2");
    let gone = StoredTest {
        ref_id: "[9:99]".into(),
        name: "gone".into(),
        line: Some(99),
        key: "s:tests/fix.feature#9".into(),
        ..stored(&fixable, TestStatusFinal::Failed)
    };
    let tests = vec![
        stored(&fixable, TestStatusFinal::Passed),
        stored(&row2, TestStatusFinal::Failed),
        stored(&by_name(&tree, "dynamic 2"), TestStatusFinal::Failed),
        gone.clone(),
    ];
    let id = seed(&t.data_dir, &fx.root, 1, |r| {
        r.selection = None;
        r.failed_of = Some(1234.0);
        r.tests = tests.clone();
    })
    .remove(0);
    t.runner.rerun(&id).unwrap();
    assert_eq!(
        t.last_notice(),
        "1 failed test no longer exists. 1 failed row of a dynamic Examples table can't be run alone. Run its outline."
    );
    assert_match(
        &t.last("runStarted"),
        json!({ "keys": [fixable.key, row2.key], "total": 2 }),
    );
    let second = folders(&t.data_dir, &fx.root)[1].clone();
    let r = record(&t.data_dir, &fx.root, &second);
    assert_match(&r, json!({ "selection": null, "failedOf": 1234 }));
    assert_eq!(
        names_and_statuses(&r),
        pairs(&[("fixable", "failed"), ("row 2", "failed")])
    );

    // with no test left, nothing runs
    let only_gone = seed(&t.data_dir, &fx.root, 1, |r| {
        r.selection = None;
        r.failed_of = Some(1234.0);
        r.tests = vec![gone.clone()];
    })
    .remove(0);
    t.runner.rerun_failed(&only_gone).unwrap();
    assert_eq!(t.last_notice(), "1 failed test no longer exists. Nothing to run.");
    assert_eq!(folders(&t.data_dir, &fx.root).len(), 3);
}

#[test]
fn cancel_during_a_re_runs_environment_switch_runs_nothing() {
    let t = setup();
    let fx = fixture("env");
    let qa = filter(&["env=qa"], &[], false);
    let id = seed(&t.data_dir, &fx.root, 1, |r| {
        r.env = Some("qa".into());
        r.filter = qa.clone();
    })
    .remove(0);
    t.runner.open(&fx.root).unwrap();
    let rerun = {
        let id = id.clone();
        t.bg(move |r| r.rerun(&id))
    };
    cancel_in_dry_run(&t);
    rerun.join().unwrap().unwrap();
    assert_eq!(t.last_notice(), "Cancelled");
    assert_eq!(t.last("environment")["view"]["env"], Value::Null);
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env, None);
    assert_eq!(t.last_event("filter"), AppEvent::Filter { filter: no_filter() });
    assert_eq!(load_project_filter(&t.data_dir, &fx.root), no_filter());
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(folders(&t.data_dir, &fx.root), [id]);
}

#[test]
fn cancel_after_a_re_runs_environment_switch_with_no_process_runs_nothing() {
    let t = setup();
    let fx = fixture("env");
    let id = seed(&t.data_dir, &fx.root, 1, |r| r.env = Some("qa".into())).remove(0);
    t.runner.open(&fx.root).unwrap();
    {
        let runner = t.runner.clone();
        let projects = std::sync::atomic::AtomicUsize::new(1);
        let done = std::sync::Once::new();
        t.on_event(move |e| {
            if matches!(e, AppEvent::Project { .. }) {
                projects.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
            // the switch's dry run has exited and its filter is sent: the env tags are read next, no JVM is up
            if matches!(e, AppEvent::Filter { .. }) && projects.load(std::sync::atomic::Ordering::SeqCst) == 2 {
                done.call_once(|| runner.cancel());
            }
        });
    }
    t.runner.rerun(&id).unwrap();
    assert_eq!(t.last_notice(), "Run cancelled");
    assert_eq!(t.count("project"), 2); // open, then the switch: no pre-run dry run
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(folders(&t.data_dir, &fx.root), [id.as_str()]);
    let runs: Vec<Value> = t.last("history")["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].clone())
        .collect();
    assert_eq!(runs, [json!(id)]);
}

#[test]
fn a_re_run_whose_environment_has_a_parse_error_switches_and_runs_nothing() {
    let t = setup();
    let fx = fixture("broken");
    let id = seed(&t.data_dir, &fx.root, 1, |r| r.env = Some("qa".into())).remove(0);
    t.runner.open(&fx.root).unwrap();
    t.runner.rerun(&id).unwrap();
    assert!(t.last("project")["error"].as_str().unwrap().contains("syntax.feature"));
    assert_eq!(t.count("project"), 2); // open, then the switch: no pre-run dry run
    assert_eq!(t.last("environment")["view"]["env"], "qa");
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env.as_deref(), Some("qa"));
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(folders(&t.data_dir, &fx.root), [id]);
}

#[test]
fn rerun_checks_the_target_environments_secrets_first() {
    let t = setup();
    let fx = fixture("env");
    save_project_env(
        &t.data_dir,
        &fx.root,
        &EnvSettings {
            env: None,
            properties: StoredLists {
                shared: Vec::new(),
                by_env: IndexMap::from([("qa".to_string(), vec![StoredRow::secret("api.token", "Zm9v")])]),
            },
        },
    )
    .unwrap();
    let id = seed(&t.data_dir, &fx.root, 1, |r| r.env = Some("qa".into())).remove(0);
    t.runner.open(&fx.root).unwrap();
    for failed_only in [false, true] {
        if failed_only {
            t.runner.rerun_failed(&id).unwrap();
        } else {
            t.runner.rerun(&id).unwrap();
        }
        assert_eq!(
            t.last_notice(),
            "Secret api.token (qa) can't be read. Replace it in Properties."
        );
    }
    assert_eq!(t.count("project"), 1); // no discovery
    assert_eq!(t.last("environment")["view"]["env"], Value::Null);
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env, None);
    assert_eq!(t.count("runStarted"), 0);
    assert_eq!(folders(&t.data_dir, &fx.root), [id]);
}

#[test]
fn rerun_refuses_an_unknown_id() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    // a damaged record is not listed either
    let damaged = project_dir(&t.data_dir, &fx.root)
        .join("runs")
        .join("20200101-000000-001");
    fs::create_dir_all(&damaged).unwrap();
    fs::write(damaged.join("run.json"), "{").unwrap();
    let ids = [
        "20200101-000000-000",
        "20200101-000000-001",
        "..\\..\\x",
        "20261025-143205-123\\..",
        "../../x",
        "20261025-143205-123/..",
    ];
    for id in ids {
        t.runner.rerun(id).unwrap();
        assert_eq!(t.last_notice(), GONE);
        t.runner.rerun_failed(id).unwrap();
        assert_eq!(t.last_notice(), GONE);
    }
    assert_eq!(t.count("notice"), ids.len() * 2);
    assert_eq!(t.count("project"), 1);
    assert_eq!(t.count("runStarted"), 0);
}
