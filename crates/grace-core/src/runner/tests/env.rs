//! `runner.env.test.ts`.
use super::discovery::cancel_in_dry_run;
use super::*;
use crate::model::tags::matching_leaves;
use crate::settings::{EnvSettings, StoredLists, StoredRow, load_project_env, load_project_filter, save_project_env};
use indexmap::IndexMap;

/// The names of the tree's tests.
pub(super) fn names(tree: &TestNode) -> Vec<String> {
    matching_leaves(tree, &no_filter())
        .iter()
        .map(|n| n.name.clone())
        .collect()
}

pub(super) fn has(t: &TestRunner, name: &str) -> bool {
    names(&t.tree()).iter().any(|n| n == name)
}

/// Event types after `from`, without the busy, opening and recent events.
pub(super) fn types_after(t: &TestRunner, from: usize) -> Vec<String> {
    t.json()[from..]
        .iter()
        .map(|e| e["type"].as_str().unwrap().to_string())
        .filter(|k| k != "busy" && k != "opening" && k != "recent")
        .collect()
}

fn env_of(t: &TestRunner) -> Value {
    t.last("environment")["view"]["env"].clone()
}

#[test]
fn suggestions_from_overlays_and_tags() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    assert_match(
        &t.last("environment")["view"],
        json!({ "env": null, "suggestions": ["prod", "qa"], "overlays": ["qa"] }),
    );
    assert_eq!(
        types_after(&t, 0),
        [
            "project",
            "flaky",
            "filter",
            "threads",
            "environment",
            "history",
            "flaky",
            "breakpoints",
            "watch"
        ]
    );
}

#[test]
fn set_env_rediscovers() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    assert!(!has(&t, "qa only"));
    assert!(has(&t, "not prod"));
    let from = t.len();
    t.runner.set_env(Some("qa")).unwrap();
    assert_eq!(
        types_after(&t, from),
        [
            "project",
            "flaky",
            "filter",
            "threads",
            "environment",
            "history",
            "flaky"
        ]
    );
    assert!(has(&t, "qa only"));
    assert_eq!(env_of(&t), "qa");
    // the tag of a scenario prod lacks is pruned, as after a Refresh
    t.runner.set_filter(filter(&["env=qa"], &[], false));
    t.runner.set_env(Some("prod")).unwrap();
    assert!(!has(&t, "qa only"));
    assert!(!has(&t, "not prod"));
    assert_eq!(t.last_event("filter"), AppEvent::Filter { filter: no_filter() });
    assert_eq!(load_project_filter(&t.data_dir, &fx.root), no_filter());
}

#[test]
fn a_run_uses_dash_e() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events(), "karate.env = qa"));
    assert_eq!(t.last("runStarted")["env"], "qa");
    assert!(t.runner.report_path().is_some());
    // a result from qa is not the state of prod
    t.runner.set_env(Some("prod")).unwrap();
    assert_eq!(t.runner.report_path(), None);
}

/// `{ shared: [timeout = 5000], byEnv: { qa: [api.token, secret] } }`.
fn saved_properties() -> StoredLists {
    let cipher = FakeSecretBox::new(true).encrypt("tok-123").unwrap();
    StoredLists {
        shared: vec![StoredRow::plain("timeout", "5000")],
        by_env: IndexMap::from([("qa".to_string(), vec![StoredRow::secret("api.token", cipher)])]),
    }
}

#[test]
fn set_env_saves_and_open_restores() {
    let first = setup();
    let fx = fixture("env");
    let properties = saved_properties();
    save_project_env(
        &first.data_dir,
        &fx.root,
        &EnvSettings {
            env: None,
            properties: properties.clone(),
        },
    )
    .unwrap();
    first.runner.open(&fx.root).unwrap();
    first.runner.set_env(Some("qa")).unwrap();
    assert_eq!(
        load_project_env(&first.data_dir, &fx.root),
        EnvSettings {
            env: Some("qa".into()),
            properties
        }
    );
    first.runner.dispose();
    let second = setup_with(Setup {
        data: Some(Arc::clone(&first.data)),
        ..Setup::default()
    });
    second.runner.open(&fx.root).unwrap();
    assert_eq!(env_of(&second), "qa");
    assert!(has(&second, "qa only"));
    assert_eq!(second.runner.replay(), idle_replay(&second, no_filter(), 1));
}

#[test]
fn the_view_shows_secret_rows_without_their_values() {
    let fx = fixture("env");
    let data = Arc::new(temp("kk-data-"));
    let token = FakeSecretBox::new(true).encrypt("tok-123").unwrap();
    save_project_env(
        data.path(),
        &fx.root,
        &EnvSettings {
            env: None,
            properties: StoredLists {
                shared: vec![StoredRow::plain("timeout", "5000")],
                by_env: IndexMap::from([
                    (
                        "qa".to_string(),
                        vec![
                            StoredRow::secret("api.token", token.clone()),
                            StoredRow::secret("broken", "Zm9v"),
                        ],
                    ),
                    ("toString".to_string(), Vec::new()),
                ]),
            },
        },
    )
    .unwrap();
    let t = setup_with(Setup {
        data: Some(Arc::clone(&data)),
        ..Setup::default()
    });
    t.runner.open(&fx.root).unwrap();
    let view = t.last("environment")["view"].clone();
    assert_eq!(view["suggestions"], json!(["prod", "qa", "toString"]));
    assert_eq!(view["secretsAvailable"], true);
    assert_eq!(
        view["properties"],
        json!({
            "shared": [{ "name": "timeout", "value": "5000" }],
            "byEnv": {
                "qa": [
                    { "name": "api.token", "secret": true, "state": "set" },
                    { "name": "broken", "secret": true, "state": "unreadable" },
                ],
                "toString": [],
            },
        })
    );
    let text = view.to_string();
    assert!(!text.contains("tok-123") && !text.contains("Zm9v"));
    assert!(!text.contains(&token));

    let off = setup_with(Setup {
        data: Some(data),
        secrets: Some(Arc::new(FakeSecretBox::new(false))),
        ..Setup::default()
    });
    off.runner.open(&fx.root).unwrap();
    let unavailable = off.last("environment")["view"].clone();
    assert_eq!(unavailable["secretsAvailable"], false);
    assert_eq!(
        unavailable["properties"]["byEnv"]["qa"][0],
        json!({ "name": "api.token", "secret": true, "state": "unreadable" })
    );
}

#[test]
fn cancel_during_set_env_restores_the_previous_environment() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    let before = t.runner.current();
    let view = t.last_event("environment");
    let from = t.len();
    let switching = t.bg(|r| r.set_env(Some("prod")));
    cancel_in_dry_run(&t);
    switching.join().unwrap().unwrap();
    assert_eq!(
        types_after(&t, from),
        [
            "project",
            "filter",
            "threads",
            "environment",
            "history",
            "flaky",
            "notice"
        ]
    );
    assert_eq!(Some(t.last_event("project")), before);
    assert_eq!(t.last_event("environment"), view);
    assert_eq!(t.last_notice(), "Cancelled");
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env.as_deref(), Some("qa"));
    // the restored environment is the one discovery uses
    t.runner.refresh().unwrap();
    assert!(has(&t, "qa only"));
}

#[test]
fn a_parse_error_still_applies_and_saves_the_environment() {
    let t = setup();
    let fx = fixture("broken");
    t.runner.open(&fx.root).unwrap();
    t.runner.set_env(Some("qa")).unwrap();
    assert!(t.last("project")["error"].as_str().unwrap().contains("syntax.feature"));
    assert_eq!(env_of(&t), "qa");
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env.as_deref(), Some("qa"));
}

#[test]
fn invalid_env_name_is_refused() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    for name in ["bad name", "-Dx=1", "", &"a".repeat(65)] {
        let from = t.len();
        t.runner.set_env(Some(name)).unwrap();
        assert_eq!(t.last_notice(), "Invalid environment name");
        // the view in force is re-sent, so a window that shows the typed name reverts
        assert_eq!(types_after(&t, from), ["notice", "environment"]);
    }
    t.runner.set_env(None).unwrap(); // the same environment: nothing to do
    assert_eq!(t.count("project"), 1);
    assert_eq!(env_of(&t), Value::Null);
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env, None);
}

#[test]
fn set_env_while_busy_gets_the_busy_notice() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    let key = t.key("slow");
    let run = t.bg(move |r| r.run(&key));
    until(|| t.count("runStarted") > 0);
    let from = t.len();
    t.runner.set_env(Some("qa")).unwrap();
    assert_eq!(t.last_notice(), "A run is already in progress");
    let seen: Vec<String> = types_after(&t, from)
        .into_iter()
        .filter(|k| k == "notice" || k == "environment")
        .collect();
    assert_eq!(seen, ["notice", "environment"]);
    t.runner.cancel();
    run.join().unwrap().unwrap();
    assert_eq!(t.count("environment"), 2);
    assert_eq!(env_of(&t), Value::Null);
}

#[test]
fn a_change_refused_during_a_discovery_re_sends_the_environment_in_force() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    let switching = t.bg(|r| r.set_env(Some("qa")));
    until(|| t.last_event("busy") == AppEvent::Busy { busy: true });
    let from = t.len();
    t.runner.set_env(Some("prod")).unwrap();
    // the window must not show the environment still being discovered
    assert_eq!(types_after(&t, from), ["notice", "environment"]);
    assert_eq!(t.last_notice(), "A run is already in progress");
    assert_eq!(env_of(&t), Value::Null);
    let replayed = t
        .runner
        .replay()
        .into_iter()
        .find_map(|e| match e {
            AppEvent::Environment { view } => Some(view),
            _ => None,
        })
        .unwrap();
    assert_eq!(replayed.env, None);
    switching.join().unwrap().unwrap();
    assert_eq!(env_of(&t), "qa");
}

#[test]
fn a_discovery_that_throws_keeps_the_environment_before() {
    let t = setup();
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    let before = t.runner.replay();
    // a file where the run folders go: emptying the dry run folder throws
    let runs = project_dir(&t.data_dir, &fx.root);
    fs::remove_dir_all(&runs).unwrap();
    fs::write(&runs, "").unwrap();
    let from = t.len();
    assert!(t.runner.set_env(Some("qa")).is_err());
    // the window still shows the name it sent until main re-sends the environment in force
    assert_eq!(types_after(&t, from), ["environment"]);
    assert_eq!(env_of(&t), Value::Null);
    assert_eq!(t.runner.replay(), before);
    assert_eq!(load_project_env(&t.data_dir, &fx.root).env, None);
    fs::remove_file(&runs).unwrap();
    t.runner.refresh().unwrap(); // discovers with the environment before
    assert!(!has(&t, "qa only"));
}

#[test]
fn karate_env_in_the_parent_environment_is_ignored() {
    let t = setup();
    let mut parent: crate::karate::command::Env = std::env::vars_os().collect();
    parent.push(("KARATE_ENV".into(), "prod".into()));
    t.runner.set_parent_env(parent);
    let fx = fixture("env");
    t.runner.open(&fx.root).unwrap();
    assert!(has(&t, "not prod"));
    t.runner.run(&t.key("always")).unwrap();
    assert!(printed_has(&t.events(), "karate.env = null"));
    assert_eq!(t.last("runStarted")["env"], Value::Null);
}
