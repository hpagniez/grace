//! `runner.debug.test.ts`: the step debugger in the runner (docs/specs/2026-11-02-step-debugger.md), with the bundled
//! Java, Karate and the vendored `listener.jar`.
use super::*;
use crate::karate::tree::find_node;
use crate::model::env::parse_properties_edit;
use crate::model::events::{Breakpoint, DebugCommand};
use serde_json::json;
use std::collections::BTreeMap;
use std::io;
use std::net::TcpStream;
use std::thread::JoinHandle;

/// A fixture project, open, with `breakpoints` set.
fn open_with(name: &str, breakpoints: &[(&str, u32)]) -> (TestRunner, Fixture) {
    let root = fixture(name);
    let t = setup();
    assert!(t.runner.open(&root.root).unwrap());
    for (file, line) in breakpoints {
        t.runner.toggle_breakpoint(file, *line);
    }
    (t, root)
}

fn open(name: &str) -> (TestRunner, Fixture) {
    open_with(name, &[])
}

/// A debug run of `key`, not awaited yet.
fn debug(t: &TestRunner, key: &str) -> JoinHandle<io::Result<()>> {
    let key = key.to_string();
    t.bg(move |r| r.debug(&key))
}

/// The n-th pause (from 1), once it has come.
#[track_caller]
fn nth(t: &TestRunner, n: usize) -> Value {
    until(|| t.count("debugPaused") >= n);
    t.all("debugPaused")[n - 1]["paused"].clone()
}

/// The n-th pause's variables, as `kind value` by name.
fn vars(t: &TestRunner, n: usize) -> BTreeMap<String, String> {
    nth(t, n)["vars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let (kind, value) = (v["kind"].as_str().unwrap(), v["value"].as_str().unwrap());
            (v["name"].as_str().unwrap().to_string(), format!("{kind} {value}"))
        })
        .collect()
}

fn var(t: &TestRunner, n: usize, name: &str) -> Value {
    nth(t, n)["vars"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap_or_else(|| panic!("no variable {name}"))
        .clone()
}

#[track_caller]
fn finished(done: JoinHandle<io::Result<()>>) {
    done.join().unwrap().unwrap();
}

fn save_secret(t: &TestRunner, name: &str, value: &str) {
    let edit = json!({ "shared": [{ "name": name, "value": value, "secret": true }], "byEnv": {} });
    assert!(t.runner.save_properties(&parse_properties_edit(&edit).unwrap()));
}

/// The value after `name=` in Java's arguments of the last debug run.
fn java_arg(spawns: &Mutex<Vec<Started>>, name: &str) -> String {
    let spawns = lock(spawns);
    let run = spawns.iter().rev().find(|s| !s.dry).unwrap();
    run.args
        .iter()
        .find_map(|a| a.strip_prefix(&format!("-D{name}=")))
        .unwrap()
        .to_string()
}

#[test]
fn a_breakpoint_pauses_before_its_step_with_the_variables() {
    let (t, _root) = open_with("debug", &[("debug.feature", 8)]);
    let key = t.key("variables");
    let done = debug(&t, &key);
    assert_match(
        &nth(&t, 1),
        json!({
            "file": "debug.feature",
            "line": 8,
            "depth": 0,
            "prefix": "*",
            "keyword": "def",
            "text": "user = { id: 7 }",
            "key": key,
        }),
    );
    assert_eq!(
        vars(&t, 1),
        BTreeMap::from([
            ("base".into(), "string from background".into()),
            ("name".into(), "string grace".into())
        ])
    );
    assert_eq!(t.last("runStarted")["debug"], true);
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
    assert_eq!(t.count("debugResumed"), 1);
    assert_match(
        &t.last("runFinished"),
        json!({ "outcome": "finished", "summary": { "scenariosPassed": 1, "scenariosFailed": 0 } }),
    );
}

#[test]
fn step_over_stays_at_the_same_depth_step_into_enters_the_called_feature() {
    let (t, _root) = open_with("debug", &[("debug.feature", 9)]);
    let key = t.key("variables");
    let done = debug(&t, &key);
    nth(&t, 1);
    t.runner.debug_command(DebugCommand::Over);
    assert_match(&nth(&t, 2), json!({ "file": "debug.feature", "line": 10, "depth": 0 }));
    let called = var(&t, 2, "called");
    assert_eq!(called["kind"], "json");
    let value: Value = serde_json::from_str(called["value"].as_str().unwrap()).unwrap();
    assert_match(&value, json!({ "arg": "in", "inner": "in!" }));
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);

    let again = debug(&t, &key);
    nth(&t, 3);
    t.runner.debug_command(DebugCommand::Into);
    assert_match(
        &nth(&t, 4),
        json!({ "file": "called.feature", "line": 5, "depth": 1, "key": key }),
    );
    t.runner.debug_command(DebugCommand::Continue);
    finished(again);
}

#[test]
fn a_pause_asked_as_the_run_starts_stops_before_the_first_step() {
    let (t, _root) = open("debug");
    let done = debug(&t, "f:debug.feature");
    until(|| t.count("runStarted") == 1);
    t.runner.debug_command(DebugCommand::Pause);
    assert_match(&nth(&t, 1), json!({ "file": "debug.feature", "line": 4 }));
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
    assert_eq!(t.count("debugPaused"), 1);
}

#[test]
fn runs_on_one_thread_and_records_a_debug_run() {
    let (t, root) = open("debug");
    t.runner.set_threads(&json!(4));
    t.runner.debug(&t.key("second")).unwrap();
    let id = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    assert_match(
        &record(&t.data_dir, &root.root, &id),
        json!({ "threads": 1, "debug": true, "outcome": "finished" }),
    );
    t.runner.run(&t.key("second")).unwrap();
    let id = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    let normal = record(&t.data_dir, &root.root, &id);
    assert_eq!(normal["threads"], 4);
    assert_eq!(normal.get("debug"), None);
    assert_eq!(t.last("runStarted")["debug"], false);
}

#[test]
fn takes_a_breakpoint_added_while_the_run_goes_on() {
    let (t, _root) = open_with("debug", &[("debug.feature", 8)]);
    let done = debug(&t, &t.key("variables"));
    nth(&t, 1);
    t.runner.toggle_breakpoint("debug.feature", 10);
    assert_eq!(
        t.last("breakpoints")["breakpoints"],
        json!([{ "file": "debug.feature", "line": 8 }, { "file": "debug.feature", "line": 10 }])
    );
    t.runner.debug_command(DebugCommand::Continue);
    assert_match(&nth(&t, 2), json!({ "line": 10 }));
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
}

#[test]
fn cancel_while_paused_ends_the_run_cancelled_and_leaves_no_java() {
    let (t, root) = open_with("debug", &[("debug.feature", 8)]);
    let done = debug(&t, &t.key("variables"));
    nth(&t, 1);
    t.runner.cancel();
    finished(done);
    assert_eq!(t.last("runFinished")["outcome"], "cancelled");
    assert_eq!(java_command_lines(&root.root), Vec::<String>::new());
}

#[test]
fn a_socket_that_closes_while_paused_lets_the_run_go_on() {
    let (t, _root) = open_with("debug", &[("debug.feature", 8)]);
    let done = debug(&t, &t.key("variables"));
    nth(&t, 1);
    t.runner.close_debug_session();
    finished(done);
    assert_eq!(t.count("debugResumed"), 1);
    assert_eq!(t.count("debugPaused"), 1);
    assert_eq!(t.last("runFinished")["outcome"], "finished");
}

#[test]
fn masks_secrets_in_variables_and_cuts_a_long_value() {
    let (t, _root) = open_with("debug", &[("big.feature", 6)]);
    save_secret(&t, "token", "s3cr3t");
    let done = debug(&t, &t.key("big value"));
    assert_match(&var(&t, 1, "token"), json!({ "kind": "string", "value": "****" }));
    let big = var(&t, 1, "big");
    assert_eq!(big["value"].as_str().unwrap().len(), 65_536);
    assert_eq!(big["cut"], 102_400 - 65_536);
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
}

// Karate names the file relative to src/test/java or to the project, as the path allows (#208): the dry run and the
// debug run name it alike, so the tree's file is the breakpoint's
#[test]
fn matches_a_breakpoint_in_a_maven_project_by_the_trees_file() {
    let (t, _root) = open("maven");
    let test = by_name(&t.tree(), "config, classpath data and classpath call");
    t.runner.toggle_breakpoint(&test.file, 6);
    let done = debug(&t, &test.key);
    assert_match(&nth(&t, 1), json!({ "file": test.file, "line": 6, "key": test.key }));
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
}

// Added (the plan's slice 9): a local connection that sends nothing, made before Java starts, then the real listener
#[test]
fn a_silent_connection_does_not_keep_the_listener_out() {
    let (t, _root) = open_with("debug", &[("debug.feature", 8)]);
    let silent: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
    t.runner.set_spawn_hook({
        let silent = Arc::clone(&silent);
        Arc::new(move |s: &mut Spawn| {
            if let Some(port) = s.args.iter().find_map(|a| a.strip_prefix("-Dgrace.debug.port=")) {
                lock(&silent).push(TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())).unwrap());
            }
        })
    });
    let done = debug(&t, &t.key("variables"));
    assert_match(&nth(&t, 1), json!({ "file": "debug.feature", "line": 8 }));
    assert_eq!(lock(&silent).len(), 1);
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
    assert_eq!(t.last("runFinished")["outcome"], "finished");
}

// Added: the token is for Java's command line only
#[test]
fn the_token_reaches_no_event() {
    let (t, _root) = open_with("debug", &[("debug.feature", 8)]);
    let spawns = t.record_spawns();
    let done = debug(&t, &t.key("variables"));
    nth(&t, 1);
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
    let token = java_arg(&spawns, "grace.debug.token");
    assert_eq!(token.len(), 32);
    let sent = serde_json::to_string(&t.json()).unwrap();
    assert!(!sent.contains(&token));
    assert!(sent.contains(r#""type":"debugResumed""#));
}

// Added: a command with no debug run does nothing, and a normal run gets no listener (R2.1)
#[test]
fn a_command_without_a_debug_run_or_a_pause_does_nothing() {
    let (t, _root) = open("debug");
    for c in [
        DebugCommand::Continue,
        DebugCommand::Over,
        DebugCommand::Into,
        DebugCommand::Pause,
    ] {
        t.runner.debug_command(c);
    }
    let spawns = t.record_spawns();
    t.runner.run(&t.key("second")).unwrap();
    let normal = lock(&spawns).iter().find(|s| !s.dry).unwrap().args.clone();
    assert!(
        !normal
            .iter()
            .any(|a| a.contains("grace.debug") || a.contains("DebugListener"))
    );
    assert_eq!(t.count("debugPaused") + t.count("debugResumed"), 0);
}

// Added: a value cut at 64 KB inside a secret has its cut end masked (step debugger R2.5)
#[test]
fn masks_a_secret_cut_at_the_end_of_a_long_value() {
    let (t, root) = open_with("debug", &[("cut.feature", 5)]);
    fs::write(
        root.join("cut.feature"),
        "Feature: cut\n\n  Scenario: cut secret\n    * def tail = karate.repeat(65533, function(){ return 'x' }).join('') + karate.properties['token']\n    * print 'after'\n",
    )
    .unwrap();
    t.runner.refresh().unwrap();
    t.runner.toggle_breakpoint("cut.feature", 5);
    save_secret(&t, "token", "s3cr3t");
    let done = debug(&t, &t.key("cut secret"));
    let tail = var(&t, 1, "tail");
    let value = tail["value"].as_str().unwrap();
    assert_eq!(tail["cut"], 3);
    assert!(value.ends_with("x****"), "{}", &value[value.len() - 10..]);
    assert!(!value.contains("s3c"));
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
}

// Added (the plan's slice 10, left open by slice 9): Java cuts a value at 65,536 UTF-16 units, here between the two
// halves of an emoji, and writes the half left as `?`, so the line is valid UTF-8 and the run pauses
#[test]
fn a_value_cut_through_an_emoji_pauses_with_its_half_as_a_question_mark() {
    let (t, root) = open_with("debug", &[("cut.feature", 5)]);
    fs::write(
        root.join("cut.feature"),
        "Feature: cut\n\n  Scenario: cut emoji\n    * def tail = karate.repeat(65535, function(){ return 'x' }).join('') + '\\uD83D\\uDE00'\n    * print 'after'\n",
    )
    .unwrap();
    t.runner.refresh().unwrap();
    t.runner.toggle_breakpoint("cut.feature", 5);
    let done = debug(&t, &t.key("cut emoji"));
    let tail = var(&t, 1, "tail");
    let value = tail["value"].as_str().unwrap();
    assert_eq!(tail["cut"], 1);
    assert_eq!(value.len(), 65_536);
    assert!(value.ends_with("x?"), "{}", &value[value.len() - 10..]);
    t.runner.debug_command(DebugCommand::Continue);
    finished(done);
    assert_eq!(t.last("runFinished")["outcome"], "finished");
}

#[test]
fn lists_a_tests_steps_before_any_run() {
    let (t, _root) = open("debug");
    assert_eq!(lines(&t, &t.key("variables")).unwrap(), [4, 7, 8, 9, 10]);
    assert_eq!(lines(&t, "f:debug.feature"), None);
}

fn lines(t: &TestRunner, key: &str) -> Option<Vec<u32>> {
    Some(t.runner.steps(key)?.iter().map(|s| s.line).collect())
}

#[test]
fn gives_a_row_the_steps_of_its_outline_and_none_once_the_file_is_gone() {
    let (t, root) = open("clean");
    let tree = t.tree();
    let row = leaf_keys(&tree)
        .into_iter()
        .find(|k| find_node(&tree, k).is_some_and(|n| n.kind == NodeKind::Row && n.file == "tests/outline.feature"))
        .unwrap();
    assert_eq!(lines(&t, &row).unwrap(), [4, 5]);
    fs::remove_file(root.join("tests/outline.feature")).unwrap();
    assert_eq!(lines(&t, &row), None);
    assert_eq!(lines(&t, "s:gone.feature#0"), None);
}

#[test]
fn gives_a_reloaded_window_the_breakpoints_and_refuses_a_line_of_no_feature() {
    let (t, _root) = open("debug");
    t.runner.toggle_breakpoint("debug.feature", 8);
    let set = AppEvent::Breakpoints {
        breakpoints: vec![Breakpoint {
            file: "debug.feature".into(),
            line: 8,
        }],
    };
    assert!(t.runner.replay().contains(&set));
    for (file, line) in [("nope.feature", 3), ("debug.feature", 0)] {
        let sent = t.count("breakpoints");
        t.runner.toggle_breakpoint(file, line);
        assert_eq!(t.last_notice(), "Invalid request", "{file}:{line}");
        assert_eq!(t.count("breakpoints"), sent, "{file}:{line}");
    }
    t.runner.toggle_breakpoint("debug.feature", 8);
    assert_eq!(t.last("breakpoints")["breakpoints"], serde_json::json!([]));
}
