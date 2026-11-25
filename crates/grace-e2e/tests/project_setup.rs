//! `e2e/project-setup.spec.ts`, through the accessibility tree: how a project's own setup shapes the tests Grace finds
//! and runs. The pom's starting values, the libraries, the sub-folder offer and Build come with phase P4's later slices.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::{App, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

/// A tree row by its label: the row's name is the label, then its status and tags.
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && (n.name == label || n.name.starts_with(&format!("{label}, "))))
}

fn passed(tree: &Node, label: &str) -> bool {
    row(tree, label).is_some_and(|r| r.name.starts_with(&format!("{label}, passed")))
}

/// A new, empty folder for this test's `name`.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grace-e2e-setup-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `files` (relative path, content) under `root`.
fn write(root: &Path, files: &[(&str, &str)]) {
    for (rel, text) in files {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

/// Grace with `dir` as the last project.
fn open(dir: &Path) -> App {
    App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap()
}

/// Opens the feature and runs its scenario `name`, which must pass.
fn run_scenario(app: &App, feature: &str, name: &str) {
    let expand = format!("Expand {feature}");
    app.wait_for("the feature", WAIT, |t| t.find(&Role::Button, &expand).is_some())
        .unwrap();
    app.press_named(Role::Button, &expand).unwrap();
    app.wait_for("the feature open", WAIT, |t| row(t, name).is_some())
        .unwrap();
    app.press_named(Role::Button, &format!("Run {name}")).unwrap();
    app.wait_for("the scenario passed", RUN, |t| passed(t, name)).unwrap();
}

/// E-project-setup-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn karate_pom_json_gives_the_tests_and_the_config_and_a_notice_names_the_rest() {
    let dir = scratch("pom");
    // a1 passes only when it really runs with conf/karate-config.js and no environment
    write(
        &dir,
        &[
            (
                "tests/a/a.feature",
                "Feature: a\nScenario: a1\n  * match cfg == 'base'\n",
            ),
            ("tests/b/b.feature", "Feature: b\nScenario: b1\n  * print 'b'\n"),
            ("conf/karate-config.js", "function fn() { return { cfg: 'base' }; }"),
            ("conf/karate-config-qa.js", "function fn() { return { cfg: 'qa' }; }"),
            (
                "karate-pom.json",
                r#"{"paths":["tests/a"],"configDir":"conf","dryRun":true}"#,
            ),
        ],
    );
    let app = open(&dir);
    let notice = "Grace does not use these karate-pom.json settings: dryRun.";
    let tree = app
        .wait_for("the pom's tests and the notice", WAIT, |t| {
            row(t, "a.feature").is_some() && t.all().iter().any(|n| n.name == notice)
        })
        .unwrap();
    assert!(row(&tree, "b.feature").is_none(), "{}", tree.dump());
    app.wait_for_text(Role::ComboBox, "Env", "").unwrap();

    app.press_named(Role::Button, "Expand a.feature").unwrap();
    app.wait_for("a1", WAIT, |t| row(t, "a1").is_some()).unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("a1 passed and the run over", RUN, |t| {
        passed(t, "a1") && t.all().iter().any(|n| n.name == "Finished")
    })
    .unwrap();

    // the environments come from the configDir
    let env = app.wait_for_node(Role::ComboBox, "Env").unwrap();
    app.click(&env).unwrap();
    app.wait_for("the suggestions (none) and qa", WAIT, |t| {
        t.find(&Role::List, "Environments").is_some_and(|list| {
            let names: Vec<&str> = list
                .all()
                .into_iter()
                .filter(|n| n.role == Role::ListItem)
                .map(|n| n.name.as_str())
                .collect();
            names == ["(none)", "qa"]
        })
    })
    .unwrap();
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}

/// E-project-setup-04
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn mavens_compiled_output_is_on_the_classpath() {
    let dir = project("maven").unwrap();
    write(
        &dir,
        &[
            ("target/test-classes/data/built.json", r#"{ "built": true }"#),
            (
                "src/test/java/examples/built.feature",
                "Feature: built\nScenario: reads compiled output\n  * match read('classpath:data/built.json').built == true\n",
            ),
        ],
    );
    let app = open(&dir);
    run_scenario(&app, "built.feature", "reads compiled output");
}

/// E-project-setup-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn jars_in_karate_ext_are_on_the_classpath() {
    let dir = scratch("ext");
    write(
        &dir,
        &[(
            "lib.feature",
            "Feature: lib\nScenario: reads a library\n  * match read('classpath:data.json').lib == true\n",
        )],
    );
    // a jar is a zip
    let src = scratch("ext-jar");
    write(&src, &[("data.json", r#"{ "lib": true }"#)]);
    let ext = dir.join(".karate").join("ext");
    std::fs::create_dir_all(&ext).unwrap();
    grace_core::report_zip::zip_report(&src, &ext.join("lib.jar")).unwrap();

    let app = open(&dir);
    run_scenario(&app, "lib.feature", "reads a library");
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(src);
}
