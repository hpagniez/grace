//! `runner.pom.test.ts`: a project starts from its pom files (docs/specs/2026-11-04-pom-defaults.md).
use super::*;
use crate::model::env::parse_properties_edit;
use crate::settings::{
    EnvSettings, load_project_filter, save_project_env, save_project_filter, save_project_threads, saved_controls,
};

const FEATURE: &str = "Feature: a\n@smoke\nScenario: fast\n  * print 'fast'\n@smoke @slow\nScenario: slow\n  * print 'slow'\nScenario: plain\n  * print 'plain'\n";

/// Writes a standalone project whose karate-pom.json holds `pom` into `dir`.
fn write_pom(dir: &Path, pom: Value) {
    write(dir, &[("a.feature", FEATURE), ("karate-pom.json", &pom.to_string())]);
}

/// A standalone project whose karate-pom.json holds `pom`.
fn with_pom(pom: Value) -> (TempDir, String) {
    let (dir, root) = project(&[]);
    write_pom(dir.path(), pom);
    (dir, root)
}

/// A second runner on `first`'s data folder, once `first` is disposed.
fn next(first: TestRunner) -> TestRunner {
    let data = Arc::clone(&first.data);
    drop(first);
    setup_with(Setup {
        data: Some(data),
        ..Setup::default()
    })
}

fn env(t: &TestRunner) -> Value {
    t.last("environment")["view"]["env"].clone()
}

#[test]
fn karate_pom_json_starts_the_controls_and_a_run_passes_them() {
    let t = setup();
    let starts = t.record_spawns();
    let (_dir, root) = with_pom(json!({ "env": "qa", "tags": ["@smoke", "~@slow"], "threads": 3 }));
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), "qa");
    assert_eq!(
        t.last("filter")["filter"],
        json!({ "include": ["smoke"], "exclude": ["slow"], "hideNonMatching": false })
    );
    assert_eq!(t.last("threads")["count"], 3);
    assert_eq!(t.notices(), Vec::<String>::new());
    t.runner.run("d:").unwrap();
    let id = t.last("runStarted")["runId"].as_str().unwrap().to_string();
    assert_match(
        &record(&t.data_dir, &root, &id),
        json!({ "env": "qa", "threads": 3, "filter": { "include": ["smoke"], "exclude": ["slow"] } }),
    );
    // the scenario's last step: what Java was given
    let args = lock(&starts).iter().find(|s| !s.dry).expect("a run").args.clone();
    for expected in [
        &["--no-pom"][..],
        &["-e", "qa"],
        &["-t=@smoke"],
        &["-t=~@slow"],
        &["-T", "3"],
    ] {
        assert!(
            args.windows(expected.len()).any(|w| w == expected),
            "{expected:?} not in {args:?}"
        );
    }
    assert_eq!(
        saved_controls(&t.data_dir, &root).map(|s| (s.env, s.filter, s.threads)),
        Some((false, false, false))
    );
}

#[test]
fn what_the_person_saved_wins() {
    let t = setup();
    let (_dir, root) = with_pom(json!({ "env": "qa", "tags": ["@smoke"], "threads": 3 }));
    let dev = EnvSettings {
        env: Some("dev".into()),
        ..EnvSettings::default()
    };
    save_project_env(&t.data_dir, &root, &dev).unwrap();
    save_project_filter(&t.data_dir, &root, &no_filter()).unwrap();
    save_project_threads(&t.data_dir, &root, 1).unwrap();
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), "dev");
    assert_eq!(t.last("filter")["filter"], serde_json::to_value(no_filter()).unwrap());
    assert_eq!(t.last("threads")["count"], 1);
}

#[test]
fn each_control_decides_on_its_own() {
    let t = setup();
    let (_dir, root) = with_pom(json!({ "env": "qa", "threads": 3 }));
    save_project_threads(&t.data_dir, &root, 2).unwrap();
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), "qa");
    assert_eq!(t.last("threads")["count"], 2);
}

#[test]
fn picking_no_environment_is_saved() {
    let first = setup();
    let (_dir, root) = with_pom(json!({ "env": "qa" }));
    first.runner.open(&root).unwrap();
    first.runner.set_env(None).unwrap();
    let second = next(first);
    second.runner.open(&root).unwrap();
    assert_eq!(env(&second), Value::Null);
}

#[test]
fn saving_run_properties_does_not_save_the_poms_environment() {
    let first = setup();
    let (dir, root) = with_pom(json!({ "env": "qa" }));
    first.runner.open(&root).unwrap();
    let edit = json!({ "shared": [{ "name": "a", "value": "1", "secret": false }], "byEnv": {} });
    assert!(first.runner.save_properties(&parse_properties_edit(&edit).unwrap()));
    assert_eq!(saved_controls(&first.data_dir, &root).map(|s| s.env), Some(false));
    write_pom(dir.path(), json!({ "env": "dev" }));
    let second = next(first);
    second.runner.open(&root).unwrap();
    assert_eq!(env(&second), "dev");
}

#[test]
fn a_pom_filter_whose_tags_no_test_has_is_not_saved() {
    let t = setup();
    let (_dir, root) = with_pom(json!({ "tags": ["~@gone"] }));
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("filter")["filter"], serde_json::to_value(no_filter()).unwrap());
    assert_eq!(saved_controls(&t.data_dir, &root).map(|s| s.filter), Some(false));
}

#[test]
fn a_change_in_the_filter_bar_saves_the_whole_filter_shown() {
    let t = setup();
    let (_dir, root) = with_pom(json!({ "tags": ["@smoke"] }));
    t.runner.open(&root).unwrap();
    let shown = filter(&["smoke"], &[], true);
    t.runner.set_filter(shown.clone());
    assert_eq!(load_project_filter(&t.data_dir, &root), shown);
}

#[test]
fn refresh_applies_an_edited_pom_to_the_controls_with_nothing_saved() {
    let t = setup();
    let (dir, root) = with_pom(json!({ "env": "qa", "threads": 3 }));
    t.runner.open(&root).unwrap();
    t.runner.set_threads(&json!(2));
    write_pom(dir.path(), json!({ "env": "dev", "threads": 4 }));
    t.runner.refresh().unwrap();
    assert_eq!(env(&t), "dev");
    assert_eq!(t.last("threads")["count"], 2);
}

#[test]
fn refresh_keeps_the_controls_in_force_when_settings_json_cannot_be_read() {
    let t = setup();
    let (dir, root) = with_pom(json!({ "env": "qa" }));
    t.runner.open(&root).unwrap();
    t.runner.set_threads(&json!(2));
    let settings = t.data_dir.join("settings.json");
    fs::remove_file(&settings).unwrap();
    fs::create_dir(&settings).unwrap(); // reading it now fails
    write_pom(dir.path(), json!({ "env": "dev", "threads": 4 }));
    t.runner.refresh().unwrap();
    assert_eq!(env(&t), "qa");
    assert_eq!(t.last("threads")["count"], 2);
}

#[test]
fn refresh_drops_a_value_the_pom_no_longer_gives() {
    let t = setup();
    let (dir, root) = with_pom(json!({ "env": "qa" }));
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), "qa");
    write_pom(dir.path(), json!({}));
    t.runner.refresh().unwrap();
    assert_eq!(env(&t), Value::Null);
}

#[test]
fn the_notice_names_only_what_grace_does_not_use() {
    let t = setup();
    let (_dir, root) = with_pom(json!({ "env": "qa", "tags": ["~@slow"], "threads": 2, "dryRun": true }));
    t.runner.open(&root).unwrap();
    assert_eq!(
        t.last_notice(),
        "Grace does not use these karate-pom.json settings: dryRun."
    );
}

/// A Maven pom.xml passing `<karate.env>dev</karate.env>` of its properties through Surefire.
const SUREFIRE_DEV: &str = "<project><properties><karate.env>dev</karate.env></properties><build><plugins><plugin>\
     <artifactId>maven-surefire-plugin</artifactId><configuration><systemPropertyVariables>\
     <karate.env>${karate.env}</karate.env></systemPropertyVariables></configuration>\
     </plugin></plugins></build></project>";

#[test]
fn surefire_passes_a_maven_projects_karate_env() {
    let t = setup();
    let (_dir, root) = project(&[("src/test/java/a.feature", FEATURE), ("pom.xml", SUREFIRE_DEV)]);
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), "dev");
}

// Beyond the Vitest file: parity rows whose scenario opens a project, which no ported test did.

/// PD-15
#[test]
fn opening_takes_karate_pom_jsons_env_over_pom_xmls() {
    let t = setup();
    let (_dir, root) = project(&[
        ("src/test/java/a.feature", FEATURE),
        ("pom.xml", SUREFIRE_DEV),
        ("karate-pom.json", r#"{"env":"qa"}"#),
    ]);
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), "qa");
}

/// PD-16
#[test]
fn opening_keeps_pom_xml_out_when_karate_pom_jsons_env_is_unused_and_names_it() {
    let t = setup();
    let (_dir, root) = project(&[
        ("src/test/java/a.feature", FEATURE),
        ("pom.xml", SUREFIRE_DEV),
        ("karate-pom.json", r#"{"env":"x y"}"#),
    ]);
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), Value::Null);
    // the notice that the libraries could not be listed follows: this Maven project has no tool
    let said = t.last_notice();
    assert!(
        said.starts_with(
            r#"Grace does not use these karate-pom.json settings: env "x y". Grace could not list the project's libraries"#
        ),
        "{said}"
    );
}

/// PD-17
#[test]
fn a_gradle_project_gets_nothing_from_its_build_file() {
    let t = setup();
    let (_dir, root) = project(&[
        ("src/test/java/a.feature", FEATURE),
        ("build.gradle", "test { systemProperty 'karate.env', 'dev' }\n"),
    ]);
    t.runner.open(&root).unwrap();
    assert_eq!(env(&t), Value::Null);
}
