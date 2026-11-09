//! `runner.properties.test.ts`. The TypeScript's mocked `spawnKarate` is the spawn hook (`record_spawns`).
use super::*;
use crate::model::env::{PropertiesEdit, parse_properties_edit};
use crate::settings::{EnvSettings, StoredLists, StoredRow, load_project_env, save_project_env};
use indexmap::IndexMap;

fn plain(name: &str, value: &str) -> Value {
    json!({ "name": name, "value": value, "secret": false })
}
fn secret(name: &str, value: &str) -> Value {
    json!({ "name": name, "value": value, "secret": true })
}
fn keep(name: &str) -> Value {
    json!({ "name": name, "secret": true, "keep": true })
}
fn variable(name: &str, value: &str, is_secret: bool) -> Value {
    json!({ "name": name, "value": value, "secret": is_secret, "envVar": true })
}

/// A `PropertiesEdit` as the window sends it.
fn edit(x: Value) -> PropertiesEdit {
    parse_properties_edit(&x).unwrap_or_else(|| panic!("not an edit: {x}"))
}

fn save(t: &TestRunner, x: Value) -> bool {
    t.runner.save_properties(&edit(x))
}

fn arg_file(data_dir: &Path, root: &str) -> PathBuf {
    project_dir(data_dir, root).join("run-args.txt")
}

fn properties_view(t: &TestRunner) -> Value {
    t.last("environment")["view"]["properties"].clone()
}

fn run_of(starts: &Mutex<Vec<Started>>) -> Started {
    lock(starts).iter().find(|s| !s.dry).cloned().expect("a run")
}

#[test]
fn shared_and_env_properties_env_wins() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(
        &t,
        json!({ "shared": [plain("timeout", "5000")], "byEnv": { "qa": [plain("timeout", "9000")] } }),
    );
    t.runner.set_env(Some("qa")).unwrap();
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events(), "timeout = 9000"));
    let from = t.len();
    t.runner.set_env(None).unwrap();
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events()[from..], "timeout = 5000"));
}

#[test]
fn a_secret_reaches_the_run_and_is_masked() {
    let t = setup();
    let fx = fixture("env");
    // a failing match puts the value in the scenario's error too
    fs::write(
        fx.join("tests/leak.feature"),
        [
            "Feature: leak",
            "",
            "Scenario: leak",
            "  * print 'api.token =', karate.properties['api.token']",
            "  * match karate.properties['api.token'] == 'other'",
            "",
        ]
        .join("\n"),
    )
    .unwrap();
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    save(
        &t,
        json!({ "shared": [], "byEnv": { "qa": [secret("api.token", SECRET)] } }),
    );
    assert_eq!(
        properties_view(&t)["byEnv"]["qa"],
        json!([{ "name": "api.token", "secret": true, "state": "set" }])
    );
    t.runner.run("f:tests/leak.feature").unwrap();
    assert_match(&t.last("runFinished")["summary"], json!({ "scenariosFailed": 1 }));
    assert!(printed_has(&t.events(), "api.token = ****"));
    let failed = t
        .all("scenarios")
        .iter()
        .flat_map(|e| e["updates"].as_array().unwrap().clone())
        .find(|u| u["status"] == "failed")
        .unwrap();
    assert!(failed["error"].as_str().unwrap().contains("****"));
    let logs: Vec<String> = t
        .all("steps")
        .iter()
        .flat_map(|e| e["steps"].as_array().unwrap().clone())
        .map(|s| s["log"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(logs.iter().any(|l| l.contains("api.token = ****")));
    assert!(!Value::Array(t.json()).to_string().contains(SECRET));
}

#[test]
fn an_environment_variable_reaches_the_run_masked_and_stays_out_of_run_args_txt() {
    let t = setup();
    let starts = t.record_spawns();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    save(
        &t,
        json!({ "shared": [], "byEnv": { "qa": [variable("APP_API_TOKEN", SECRET, true)] } }),
    );
    assert_eq!(
        properties_view(&t)["byEnv"]["qa"],
        json!([{ "name": "APP_API_TOKEN", "secret": true, "state": "set", "envVar": true }])
    );
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events(), "APP_API_TOKEN = ****"));
    assert!(!Value::Array(t.json()).to_string().contains(SECRET));
    let run = run_of(&starts);
    assert_eq!(run.var("APP_API_TOKEN").as_deref(), Some(SECRET));
    assert!(!run.args.iter().any(|a| a.ends_with("run-args.txt")));
}

#[test]
fn both_kinds_in_one_run() {
    let t = setup();
    let starts = t.record_spawns();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    save(
        &t,
        json!({ "shared": [], "byEnv": { "qa": [plain("timeout", "9000"), variable("APP_API_TOKEN", "v", false)] } }),
    );
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events(), "timeout = 9000"));
    assert!(printed_has(&t.events(), "APP_API_TOKEN = v"));
    assert!(run_of(&starts).args.iter().any(|a| a.ends_with("run-args.txt")));
}

/// The variables of a name in any letter case that Java got: one, which Windows reads whatever its case. Linux reads
/// it only in its own case, so there the run's output is checked on Windows alone.
// Rust-native: the TypeScript reads the run's output, which on Linux (case-sensitive variables) cannot show a
// variable saved in another case. The Java environment is what the runner makes; the output is Windows'.
fn assert_one_variable(t: &TestRunner, starts: &Mutex<Vec<Started>>, name: &str, value: &str) {
    let vars = run_of(starts).vars_any_case(name);
    assert_eq!(vars.len(), 1, "{vars:?}");
    assert_eq!(vars[0].1, value);
    if cfg!(windows) {
        assert!(printed_has(&t.events(), &format!("{name} = {value}")));
    }
}

#[test]
fn the_machines_variable_is_replaced_in_any_case() {
    let t = setup();
    let starts = t.record_spawns();
    let mut parent: crate::karate::command::Env = std::env::vars_os().collect();
    parent.push(("APP_API_TOKEN".into(), "machine".into()));
    t.runner.set_parent_env(parent);
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(
        &t,
        json!({ "shared": [variable("app_api_token", "grace", false)], "byEnv": {} }),
    );
    t.runner.run(&t.key("always")).unwrap();
    assert_one_variable(&t, &starts, "APP_API_TOKEN", "grace");
}

#[test]
fn the_environments_variable_wins_over_a_shared_one_in_another_case() {
    let t = setup();
    let starts = t.record_spawns();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    save(
        &t,
        json!({
            "shared": [variable("APP_API_TOKEN", "shared", false)],
            "byEnv": { "qa": [variable("app_api_token", "qa", false)] },
        }),
    );
    t.runner.run(&t.key("always")).unwrap();
    assert_one_variable(&t, &starts, "APP_API_TOKEN", "qa");
}

#[test]
fn the_environments_variable_wins_even_where_it_replaces_a_shared_property() {
    let t = setup();
    let starts = t.record_spawns();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    // qa's APP_API_TOKEN takes the place of the shared property of that name, before app_api_token
    save(
        &t,
        json!({
            "shared": [plain("APP_API_TOKEN", "p"), variable("app_api_token", "shared", false)],
            "byEnv": { "qa": [variable("APP_API_TOKEN", "qa", false)] },
        }),
    );
    t.runner.run(&t.key("always")).unwrap();
    assert_one_variable(&t, &starts, "APP_API_TOKEN", "qa");
}

#[test]
fn the_kind_is_saved_and_switching_it_keeps_the_secret() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(&t, json!({ "shared": [secret("API_TOKEN", SECRET)], "byEnv": {} }));
    let before = load_project_env(&t.data_dir, &fx.root).properties.shared[0].clone();
    let mut kept = keep("API_TOKEN");
    kept["envVar"] = json!(true);
    save(&t, json!({ "shared": [kept], "byEnv": {} }));
    assert_eq!(
        load_project_env(&t.data_dir, &fx.root).properties.shared,
        vec![before.as_env_var()]
    );
    assert_eq!(
        properties_view(&t)["shared"],
        json!([{ "name": "API_TOKEN", "secret": true, "state": "set", "envVar": true }])
    );
}

#[test]
fn dry_runs_get_no_environment_variables() {
    let t = setup();
    let starts = t.record_spawns();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(
        &t,
        json!({ "shared": [variable("APP_API_TOKEN", "v", false)], "byEnv": {} }),
    );
    t.runner.run(&t.key("always")).unwrap();
    let dry: Vec<Started> = lock(&starts).iter().filter(|s| s.dry).cloned().collect();
    assert!(!dry.is_empty());
    for s in &dry {
        assert_eq!(s.var("APP_API_TOKEN"), None);
    }
    assert_eq!(run_of(&starts).var("APP_API_TOKEN").as_deref(), Some("v"));
}

#[test]
fn settings_never_hold_the_plain_secret() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(&t, json!({ "shared": [secret("api.token", SECRET)], "byEnv": {} }));
    let text = fs::read_to_string(t.data_dir.join("settings.json")).unwrap();
    assert!(!text.contains(SECRET));
    assert_eq!(
        load_project_env(&t.data_dir, &fx.root).properties.shared,
        vec![StoredRow::secret(
            "api.token",
            FakeSecretBox::new(true).encrypt(SECRET).unwrap()
        )]
    );
}

#[test]
fn properties_it_cannot_save_are_applied_with_a_notice() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    let settings = t.data_dir.join("settings.json");
    fs::remove_file(&settings).unwrap(); // open saved it: replace it with a folder
    fs::create_dir(&settings).unwrap(); // writing a file there fails
    // applied, though not saved
    assert!(save(&t, json!({ "shared": [plain("timeout", "5000")], "byEnv": {} })));
    assert!(t.last_notice().contains("The properties could not be saved"));
    assert_eq!(
        properties_view(&t)["shared"],
        json!([{ "name": "timeout", "value": "5000" }])
    );
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events(), "timeout = 5000"));
}

#[test]
fn the_secret_is_not_on_the_command_line() {
    let t = setup();
    let fx = fixture("env");
    let gate = hold_slow(&fx.root);
    t.runner.open(&fx.root).unwrap();
    save(&t, json!({ "shared": [secret("api.token", SECRET)], "byEnv": {} }));
    let saved = load_project_env(&t.data_dir, &fx.root);
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") > 0);
    let lines = java_command_lines(&fx.root);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains(&format!("@{}", arg_file(&t.data_dir, &fx.root).display())));
    assert!(!lines[0].contains(SECRET));
    // a save while busy is refused
    assert!(!save(&t, json!({ "shared": [], "byEnv": {} })));
    assert_eq!(t.last_notice(), "A run is already in progress");
    assert_eq!(load_project_env(&t.data_dir, &fx.root), saved);
    gate.release();
    run.join().unwrap().unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
}

#[test]
fn no_argument_file_is_left_after_a_run_a_cancel_or_a_failed_spawn() {
    let t = setup();
    let fx = fixture("env");
    let file = arg_file(&t.data_dir, &fx.root);
    t.runner.open(&fx.root).unwrap();
    save(&t, json!({ "shared": [plain("timeout", "5000")], "byEnv": {} }));

    t.runner.run(&t.key("always")).unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    assert!(!file.exists());

    let key = t.key("slow");
    let slow = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") == 2);
    assert_eq!(fs::read(&file).unwrap(), b"\"-Dtimeout=5000\"\r\n");
    t.runner.cancel();
    slow.join().unwrap().unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "cancelled");
    assert!(!file.exists());

    let pid_file = t.data_dir.join("running.pid");
    fs::create_dir(&pid_file).unwrap(); // writing a file there fails
    t.runner.run(&t.key("always")).unwrap();
    assert!(t.last_notice().contains(&folder_write_error()));
    assert!(!file.exists());
    fs::remove_dir(&pid_file).unwrap();

    // only the run gets the missing Java: the pre-run dry run has the right one
    let missing = t.data_dir.join("missing").join("java.exe");
    t.runner.set_spawn_hook(Arc::new(move |s: &mut Spawn| {
        if !s.dry {
            s.java = missing.clone();
        }
    }));
    let from = t.len();
    t.runner.run(&t.key("always")).unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "failed");
    assert!(printed_has(&t.events()[from..], "ENOENT"));
    assert!(!file.exists());
}

/// Windows only: a file opened without delete sharing cannot be deleted there, which Linux allows.
#[cfg(windows)]
#[test]
fn an_argument_file_another_process_holds_is_a_notice_not_an_error() {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(&t, json!({ "shared": [plain("timeout", "5000")], "byEnv": {} }));
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") > 0);
    // without delete sharing, as an antivirus or an indexer can, until its input closes
    let mut holder = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "$f = [IO.File]::Open('{}', 'Open', 'Read', 'ReadWrite'); 'held'; [Console]::In.ReadLine(); $f.Close()",
                arg_file(&t.data_dir, &fx.root).display()
            ),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(holder.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(line.contains("held"), "the holder said {line}");
    run.join().unwrap().unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    assert!(t.last_notice().contains("could not be deleted"));
    let _ = holder.stdin.take().unwrap().write_all(b"\n");
    holder.wait().unwrap();
}

#[test]
fn an_unreadable_secret_blocks_the_run() {
    let first = setup();
    let fx = fixture("env");
    first.runner.open(&fx.root).unwrap();
    save(&first, json!({ "shared": [secret("api.token", SECRET)], "byEnv": {} }));
    first.runner.dispose();
    let settings = first.data_dir.join("settings.json");
    let cipher = FakeSecretBox::new(true).encrypt(SECRET).unwrap();
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, text.replace(&cipher, "Zm9v")).unwrap();

    let t = setup_with(Setup {
        data: Some(Arc::clone(&first.data)),
        ..Setup::default()
    });
    t.runner.open(&fx.root).unwrap();
    assert_eq!(
        properties_view(&t)["shared"],
        json!([{ "name": "api.token", "secret": true, "state": "unreadable" }])
    );
    t.runner.run("d:").unwrap();
    assert_eq!(
        t.last_notice(),
        "Secret api.token (All environments) can't be read. Replace it in Properties."
    );
    assert_eq!(t.count("runStarted"), 0);
    // refused before the pre-run dry run: no JVM started; discovery still works
    assert_eq!(t.count("project"), 1);
    assert_ne!(t.last("project")["tree"], Value::Null);

    // with secret storage unavailable, a secret of the environment cannot be used either
    save_project_env(
        &first.data_dir,
        &fx.root,
        &EnvSettings {
            env: Some("qa".into()),
            properties: StoredLists {
                shared: Vec::new(),
                by_env: IndexMap::from([("qa".to_string(), vec![StoredRow::secret("api.token", cipher)])]),
            },
        },
    )
    .unwrap();
    let off = setup_with(Setup {
        data: Some(Arc::clone(&first.data)),
        secrets: Some(Arc::new(FakeSecretBox::new(false))),
        ..Setup::default()
    });
    off.runner.open(&fx.root).unwrap();
    off.runner.run("d:").unwrap();
    assert_eq!(
        off.last_notice(),
        "Secret api.token (qa) can't be read. Replace it in Properties."
    );
    assert_eq!(off.count("runStarted"), 0);
    assert_eq!(off.count("project"), 1);
}

#[test]
fn keep_copies_the_stored_ciphertext_a_rename_needs_a_new_value() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    save(
        &t,
        json!({ "shared": [secret("api.token", SECRET), plain("timeout", "5000")], "byEnv": {} }),
    );
    let cipher = FakeSecretBox::new(true).encrypt(SECRET).unwrap();
    let applied = save(
        &t,
        json!({ "shared": [keep("api.token"), plain("timeout", "6000")], "byEnv": { "qa": [] } }),
    );
    assert!(applied);
    assert_eq!(t.count("notice"), 0);
    assert_eq!(
        load_project_env(&t.data_dir, &fx.root).properties,
        StoredLists {
            shared: vec![
                StoredRow::secret("api.token", cipher),
                StoredRow::plain("timeout", "6000")
            ],
            by_env: IndexMap::from([("qa".to_string(), Vec::new())]),
        }
    );
    assert_eq!(
        properties_view(&t),
        json!({
            "shared": [
                { "name": "api.token", "secret": true, "state": "set" },
                { "name": "timeout", "value": "6000" },
            ],
            "byEnv": { "qa": [] },
        })
    );

    let saved = load_project_env(&t.data_dir, &fx.root);
    let views = t.count("environment");
    let cases = [
        (
            json!({ "shared": [keep("api.key")], "byEnv": {} }),
            "All environments: api.key",
        ),
        (
            json!({ "shared": [], "byEnv": { "qa": [keep("api.token")] } }),
            "qa: api.token",
        ),
        (
            json!({ "shared": [keep("timeout")], "byEnv": {} }),
            "All environments: timeout",
        ),
    ];
    for (x, row) in cases {
        assert!(!save(&t, x));
        assert_eq!(
            t.last_notice(),
            format!("Properties not saved: {row}: No saved secret to keep")
        );
    }
    assert_eq!(load_project_env(&t.data_dir, &fx.root), saved);
    assert_eq!(t.count("environment"), views);
}

fn saved_settings(data_dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(data_dir.join("settings.json")).unwrap()).unwrap()
}

#[test]
fn reserved_names_and_bad_values_are_refused() {
    let t = setup();
    let fx = fixture("env");
    assert!(!save(&t, json!({ "shared": [], "byEnv": {} }))); // no project
    t.runner.open(&fx.root).unwrap();
    let views = t.count("environment");
    let cases = [
        (
            json!({ "shared": [plain("karate.options", "-t @x")], "byEnv": {} }),
            "karate.options: Reserved name",
        ),
        (
            json!({ "shared": [plain("JAVA.home", "x")], "byEnv": {} }),
            "JAVA.home: Reserved name",
        ),
        (
            json!({ "shared": [plain("a", "x\ny")], "byEnv": {} }),
            "a: No line breaks",
        ),
        (
            json!({ "shared": [secret("a", "é")], "byEnv": {} }),
            "a: This value cannot be passed to Java exactly.",
        ),
        (
            json!({ "shared": [plain("a", "1"), plain("a", "2")], "byEnv": {} }),
            "a: Duplicate name",
        ),
    ];
    for (x, error) in cases {
        assert!(!save(&t, x));
        assert_eq!(
            t.last_notice(),
            format!("Properties not saved: All environments: {error}")
        );
    }
    assert_eq!(t.count("environment"), views);
    // open saved the recent list only
    assert!(saved_settings(&t.data_dir).get("projects").is_none());

    let off = setup_with(Setup {
        secrets: Some(Arc::new(FakeSecretBox::new(false))),
        ..Setup::default()
    });
    off.runner.open(&fx.root).unwrap();
    assert!(!save(
        &off,
        json!({ "shared": [secret("api.token", SECRET)], "byEnv": {} })
    ));
    assert_eq!(off.last_notice(), "Secret storage is not available on this system");
    assert!(saved_settings(&off.data_dir).get("projects").is_none());
    // plain rows still save
    assert!(save(&off, json!({ "shared": [plain("timeout", "1")], "byEnv": {} })));
    assert_eq!(
        load_project_env(&off.data_dir, &fx.root).properties.shared,
        vec![StoredRow::plain("timeout", "1")]
    );
}

#[test]
fn saved_rows_todays_rules_reject_are_named_in_one_notice_on_open() {
    let fx = fixture("env");
    let data = Arc::new(temp("kk-data-"));
    let properties = json!({
        "shared": [
            { "name": "timeout", "value": "5000" },
            { "name": "java.home", "value": "C:/jdk" },
        ],
        "byEnv": {
            "qa": [{ "name": "greeting", "secret": "Zm9v", "value": "h\u{e9}llo" }, { "secret": "Zm9v" }],
        },
    });
    let mut projects = serde_json::Map::new();
    projects.insert(fx.root.clone(), json!({ "properties": properties }));
    fs::write(
        data.path().join("settings.json"),
        json!({ "projects": projects }).to_string(),
    )
    .unwrap();
    let t = setup_with(Setup {
        data: Some(data),
        ..Setup::default()
    });
    t.runner.open(&fx.root).unwrap();
    t.runner.refresh().unwrap();
    assert_eq!(
        t.notices(),
        [
            "Some saved properties were not loaded and are kept in settings.json: All environments: java.home, qa: greeting, qa: (no name)"
        ]
    );
    assert_eq!(
        properties_view(&t),
        json!({ "shared": [{ "name": "timeout", "value": "5000" }], "byEnv": { "qa": [] } })
    );
}

#[test]
fn no_properties_no_at_argument() {
    let t = setup();
    let fx = fixture("env");
    let _gate = hold_slow(&fx.root); // the cancel ends the run
    t.runner.open(&fx.root).unwrap();
    // rows of another environment are not passed
    save(
        &t,
        json!({ "shared": [], "byEnv": { "qa": [plain("timeout", "9000")] } }),
    );
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") > 0);
    let lines = java_command_lines(&fx.root);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(!lines[0].contains("run-args.txt"));
    assert!(!regex::Regex::new(r#"\s"?@"#).unwrap().is_match(&lines[0]));
    assert!(!arg_file(&t.data_dir, &fx.root).exists());
    t.runner.cancel();
    run.join().unwrap().unwrap();
}
