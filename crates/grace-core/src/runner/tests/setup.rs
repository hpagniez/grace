//! `runner.setup.test.ts`.
use super::*;
use crate::report_zip::zip_report;

/// a.feature passes only when it really runs with conf/karate-config.js and no environment.
const POM_PROJECT: &[(&str, &str)] = &[
    (
        "tests/a/a.feature",
        "Feature: a\nScenario: a1\n  * match cfg == 'base'\n",
    ),
    ("tests/b/b.feature", "Feature: b\nScenario: b1\n  * print 'b'\n"),
    ("conf/karate-config.js", "function fn() { return { cfg: 'base' }; }"),
    ("conf/karate-config-qa.js", "function fn() { return { cfg: 'qa' }; }"),
    (
        "karate-pom.json",
        r#"{"paths":["tests/a","missing"],"configDir":"conf","dryRun":true}"#,
    ),
];

const ONE_PASSED: &str = r#"{"scenarioCount":1,"scenariosPassed":1}"#;

fn assert_one_passed(t: &TestRunner) {
    assert_match(
        &t.last("runFinished")["summary"],
        serde_json::from_str(ONE_PASSED).unwrap(),
    );
}

const ROWS_FEATURE: &str =
    "Feature: rows\nScenario Outline: row <n>\n  * assert <n> > 0\nExamples:\n  | n |\n  | 1 |\n  | 2 |\n";

#[test]
fn takes_paths_and_config_dir_from_karate_pom_json_and_names_the_rest() {
    let t = setup();
    let (_dir, root) = project(POM_PROJECT);
    t.runner.open(&root).unwrap();
    assert_eq!(leaf_keys(&t.tree()), ["s:tests/a/a.feature#1"]);
    assert_eq!(
        t.last_notice(),
        "Grace does not use these karate-pom.json settings: dryRun. \
         These karate-pom.json paths are not folders or files in the project: missing."
    );
    assert_match(
        &t.last("environment")["view"],
        json!({ "env": null, "overlays": ["qa"] }),
    );
    t.runner.run("d:").unwrap();
    assert_one_passed(&t);
}

#[test]
fn writes_junit_xml_when_karate_pom_json_asks() {
    let t = setup();
    let (_dir, root) = project(&[
        ("tests/a.feature", "Feature: a\nScenario: a1\n  * print 'a'\n"),
        ("karate-pom.json", r#"{"output":{"junitXml":true}}"#),
    ]);
    t.runner.open(&root).unwrap();
    t.runner.run("d:").unwrap();
    assert_eq!(t.last("runFinished")["outcome"], "finished");
    let project = project_dir(&t.data_dir, &root);
    let runs = folders(&t.data_dir, &root);
    assert_eq!(runs.len(), 1);
    let junit = project.join("runs").join(&runs[0]).join("junit-xml");
    assert_eq!(fs::read_dir(junit).unwrap().count(), 1);
    // the dry run's folder is kept, so a junit-xml in it would show
    assert!(project.join("dryrun").join("karate-json").exists());
    assert!(!project.join("dryrun").join("junit-xml").exists());
    assert!(t.notices().iter().all(|n| !n.contains("output")), "{:?}", t.notices());
}

#[test]
fn says_it_only_on_open() {
    let t = setup();
    let (_dir, root) = project(POM_PROJECT);
    t.runner.open(&root).unwrap();
    let seen = t.count("notice");
    assert!(seen > 0);
    t.runner.refresh().unwrap();
    assert_eq!(t.count("notice"), seen);
}

#[test]
fn puts_mavens_compiled_output_on_the_classpath() {
    let t = setup();
    let fx = fixture("maven");
    write(
        fx.path(),
        &[
            ("target/test-classes/data/built.json", r#"{ "built": true }"#),
            (
                "src/test/java/examples/built.feature",
                "Feature: built\nScenario: reads compiled output\n  * match read('classpath:data/built.json').built == true\n",
            ),
        ],
    );
    t.runner.open(&fx.root).unwrap();
    // the key depends on how Karate names the file: find it by name
    t.runner.run(&t.key("reads compiled output")).unwrap();
    assert_one_passed(&t);
}

/// Karate may name the files relative to src/test/java, which holds karate-config.js (#208).
#[test]
fn runs_one_row_of_a_maven_outline() {
    let t = setup();
    let fx = fixture("maven");
    write(fx.path(), &[("src/test/java/examples/rows.feature", ROWS_FEATURE)]);
    t.runner.open(&fx.root).unwrap();
    assert_eq!(t.last("project")["error"], Value::Null);
    let row = t.key("row 2");
    t.runner.run(&row).unwrap();
    assert_match(&t.last("runStarted"), json!({ "keys": [row], "total": 1 }));
    assert_one_passed(&t);
}

/// Karate names the files relative to a configDir that holds them (#223).
#[test]
fn runs_one_row_of_an_outline_in_the_config_dir() {
    let t = setup();
    let (_dir, root) = project(&[
        ("tests/karate-config.js", "function fn() { return { cfg: 'base' }; }"),
        (
            "tests/a/rows.feature",
            "Feature: rows\nScenario Outline: row <n>\n  * match cfg == 'base'\nExamples:\n  | n |\n  | 1 |\n  | 2 |\n",
        ),
        ("karate-pom.json", r#"{"configDir":"tests"}"#),
    ]);
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("project")["error"], Value::Null);
    let row = t.key("row 2");
    t.runner.run(&row).unwrap();
    let passed = json!({ "key": row, "status": "passed" });
    assert!(
        t.all("scenarios")
            .iter()
            .flat_map(|e| e["updates"].as_array().unwrap().clone())
            .any(|u| matches_object(&u, &passed)),
        "{:?}",
        t.all("scenarios")
    );
    assert_one_passed(&t);
}

#[test]
fn uses_the_default_folders_when_karate_pom_json_cannot_be_read() {
    let t = setup();
    let fx = fixture("clean");
    write(fx.path(), &[("karate-pom.json", "{")]);
    t.runner.open(&fx.root).unwrap();
    let keys = leaf_keys(&t.tree());
    assert!(keys.iter().any(|k| k.starts_with("s:tests/basic.feature#")), "{keys:?}");
    let said = t.last_notice();
    let reason = said
        .strip_prefix("karate-pom.json could not be read: ")
        .and_then(|s| s.strip_suffix(". Grace uses the default folders."));
    assert!(reason.is_some_and(|r| !r.is_empty()), "{said}");
}

/// Passes only when a jar holding data.json is on the classpath.
const EXT_PROJECT: &[(&str, &str)] = &[(
    "t.feature",
    "Feature: t\nScenario: ext\n  * match read('classpath:data.json').ext == true\n",
)];

/// `<dir>/.karate/ext/lib.jar` holding that data.json: a jar is a zip.
fn ext_jar(dir: &Path) {
    let src = temp("kk-jar-");
    write(src.path(), &[("data.json", r#"{ "ext": true }"#)]);
    let ext = dir.join(".karate").join("ext");
    fs::create_dir_all(&ext).unwrap();
    zip_report(src.path(), &ext.join("lib.jar")).unwrap();
}

#[test]
fn puts_the_jars_of_the_projects_karate_ext_on_the_classpath() {
    let t = setup();
    let (dir, root) = project(EXT_PROJECT);
    ext_jar(dir.path());
    t.runner.open(&root).unwrap();
    t.runner.run("d:").unwrap();
    assert_one_passed(&t);
}

#[test]
fn puts_the_jars_of_the_home_karate_ext_on_the_classpath() {
    let t = setup();
    ext_jar(t.home.path());
    let (_dir, root) = project(EXT_PROJECT);
    t.runner.open(&root).unwrap();
    t.runner.run("d:").unwrap();
    assert_one_passed(&t);
}

// Beyond the Vitest file: parity rows whose scenario opens a project, which no ported test did.

/// Scenarios carrying the tags the PD tests' poms name: the filter drops a tag no test has.
const TAGGED: (&str, &str) = (
    "t.feature",
    "Feature: t\n@a @b\nScenario: one\n  * print 1\n@c @d\nScenario: two\n  * print 2\n",
);

/// PD-09
#[test]
fn opening_shows_the_poms_tag_entries_in_the_filter_and_names_none() {
    let t = setup();
    let (_dir, root) = project(&[TAGGED, ("karate-pom.json", r#"{"tags":[" @a , @b ","~@c","~@d"]}"#)]);
    t.runner.open(&root).unwrap();
    assert_match(
        &t.last("filter")["filter"],
        json!({ "include": ["a", "b"], "exclude": ["c", "d"] }),
    );
    assert_eq!(t.notices(), Vec::<String>::new());
}

/// PD-10
#[test]
fn opening_keeps_the_tag_entry_it_can_show_and_names_the_other() {
    let t = setup();
    let (_dir, root) = project(&[TAGGED, ("karate-pom.json", r#"{"tags":["@a","@b"]}"#)]);
    t.runner.open(&root).unwrap();
    assert_match(&t.last("filter")["filter"], json!({ "include": ["a"], "exclude": [] }));
    assert_eq!(
        t.last_notice(),
        r#"Grace does not use these karate-pom.json settings: tags "@b"."#
    );
}

/// PD-11
#[test]
fn opening_picks_no_env_and_1_thread_for_values_it_cannot_take_and_names_them() {
    let t = setup();
    let (_dir, root) = project(&[TAGGED, ("karate-pom.json", r#"{"env":"x y","threads":64}"#)]);
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("environment")["view"]["env"], Value::Null);
    assert_eq!(t.last("threads")["count"], 1);
    assert_eq!(
        t.last_notice(),
        r#"Grace does not use these karate-pom.json settings: env "x y", threads 64."#
    );
}

/// PS-07
#[test]
fn compiled_copies_stay_out_of_the_tree_and_the_environments() {
    let t = setup();
    let fx = fixture("maven");
    write(
        fx.path(),
        &[
            (
                "target/test-classes/old.feature",
                "Feature: old\nScenario: stale\n  * print 1\n",
            ),
            (
                "target/test-classes/karate-config-stale.js",
                "function fn() { return {}; }",
            ),
        ],
    );
    t.runner.open(&fx.root).unwrap();
    let keys = leaf_keys(&t.tree());
    assert!(
        !keys.is_empty() && keys.iter().all(|k| !k.contains("old.feature")),
        "{keys:?}"
    );
    let overlays = t.last("environment")["view"]["overlays"].clone();
    assert!(!overlays.as_array().unwrap().contains(&json!("stale")), "{overlays}");
}
