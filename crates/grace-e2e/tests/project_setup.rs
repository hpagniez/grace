//! `e2e/project-setup.spec.ts`, through the accessibility tree: how a project's own setup shapes the tests Grace finds
//! and runs. The sub-folder offer comes with phase P4's slice 5.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::history::runs_dir;
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

/// Passes only with a library holding data.json on the classpath.
const LIB_FEATURE: &str =
    "Feature: lib\nScenario: reads a library\n  * match read('classpath:data.json').lib == true\n";

/// `<dir>/lib.jar` holding that data.json: a jar is a zip. `test` names the source folder, one per test.
fn lib_jar(dir: &Path, test: &str) -> PathBuf {
    let src = scratch(&format!("{test}-source"));
    write(&src, &[("data.json", r#"{ "lib": true }"#)]);
    std::fs::create_dir_all(dir).unwrap();
    let jar = dir.join("lib.jar");
    grace_core::report_zip::zip_report(&src, &jar).unwrap();
    let _ = std::fs::remove_dir_all(src);
    jar
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

/// E-project-setup-02
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn karate_pom_json_starts_the_environment_filter_and_thread_count() {
    let dir = scratch("pom-start");
    // fast passes only in qa; the other two must not run
    write(
        &dir,
        &[
            (
                "a.feature",
                "Feature: a\n@smoke\nScenario: fast\n  * match cfg == 'qa'\n\
                 @smoke @slow\nScenario: slow\n  * print 'slow'\nScenario: plain\n  * print 'plain'\n",
            ),
            ("karate-config.js", "function fn() { return { cfg: 'base' }; }"),
            ("karate-config-qa.js", "function fn() { return { cfg: 'qa' }; }"),
            (
                "karate-pom.json",
                r#"{"env":"qa","tags":["@smoke","~@slow"],"threads":3}"#,
            ),
        ],
    );
    let app = open(&dir);
    app.wait_for("the feature", WAIT, |t| {
        t.find(&Role::Button, "Expand a.feature").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Expand a.feature").unwrap();
    app.wait_for("the tests and the chips", WAIT, |t| {
        row(t, "plain").is_some()
            && t.find(&Role::Button, "@smoke, included").is_some()
            && t.find(&Role::Button, "@slow, excluded").is_some()
    })
    .unwrap();
    // on Linux these find the boxes only: the run below shows what they hold
    app.wait_for_text(Role::ComboBox, "Env", "qa").unwrap();
    app.wait_for_text(Role::SpinButton, "Threads", "3").unwrap();

    app.press_named(Role::Button, "Run all").unwrap();
    let tree = app
        .wait_for("fast passed and the run over", RUN, |t| {
            passed(t, "fast") && t.all().iter().any(|n| n.name == "Finished")
        })
        .unwrap();
    // read after the run, when a notice of the open is certainly shown
    assert!(
        tree.all().iter().all(|n| !n.name.starts_with("Grace does not use")),
        "{}",
        tree.dump()
    );
    // the rows the filter dims: not run, and without a Run button
    for name in ["slow", "plain"] {
        assert!(
            row(&tree, name).is_some_and(|r| !r.name.contains("passed") && !r.name.contains("failed"))
                && tree.find(&Role::Button, &format!("Run {name}")).is_none(),
            "{name}: {}",
            tree.dump()
        );
    }
    let runs: Vec<PathBuf> = std::fs::read_dir(runs_dir(app.data_dir(), &dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(runs.len(), 1, "{runs:?}");
    let events = std::fs::read_to_string(runs[0].join("karate-json").join("karate-events.jsonl")).unwrap();
    let suite = events.lines().find(|l| l.contains("\"SUITE_ENTER\"")).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(suite).unwrap()["data"]["threads"],
        3
    );
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}

/// E-project-setup-03
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn mavens_karate_env_starts_the_environment() {
    let dir = project("maven").unwrap();
    write(
        &dir,
        &[
            (
                "pom.xml",
                "<project><modelVersion>4.0.0</modelVersion><groupId>x</groupId><artifactId>x</artifactId><version>1</version>\
                 <properties><karate.env>dev</karate.env></properties><build><plugins><plugin>\
                 <artifactId>maven-surefire-plugin</artifactId><configuration><systemPropertyVariables>\
                 <karate.env>${karate.env}</karate.env></systemPropertyVariables></configuration>\
                 </plugin></plugins></build></project>",
            ),
            (
                "src/test/java/examples/env.feature",
                "Feature: env\nScenario: runs in dev\n  * match karate.env == 'dev'\n",
            ),
        ],
    );
    let app = open(&dir);
    // on Linux this finds the box only: the run below shows what it holds
    app.wait_for_text(Role::ComboBox, "Env", "dev").unwrap();
    run_scenario(&app, "env.feature", "runs in dev");
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

/// E-project-setup-05
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_maven_projects_libraries_come_from_its_build() {
    let dir = project("maven").unwrap();
    write(&dir, &[("src/test/java/examples/lib.feature", LIB_FEATURE)]);
    let built = scratch("maven-jar");
    let jar = lib_jar(&built, "maven-jar");
    // a stand-in for Maven: it lists the jar where Grace asks
    #[cfg(windows)]
    std::fs::write(
        dir.join("mvnw.cmd"),
        format!("@echo off\r\n>\"%GRACE_CLASSPATH_FILE%\" echo {}\r\n", jar.display()),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = dir.join("mvnw");
        let script = format!("#!/bin/sh\necho '{}' > \"$GRACE_CLASSPATH_FILE\"\n", jar.display());
        std::fs::write(&wrapper, script).unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let app = open(&dir);
    run_scenario(&app, "lib.feature", "reads a library");
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
    let _ = std::fs::remove_dir_all(built);
}

/// E-project-setup-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn jars_in_karate_ext_are_on_the_classpath() {
    let dir = scratch("ext");
    write(&dir, &[("lib.feature", LIB_FEATURE)]);
    let ext = dir.join(".karate").join("ext");
    lib_jar(&ext, "ext");

    let app = open(&dir);
    run_scenario(&app, "lib.feature", "reads a library");
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}

/// E-project-setup-08
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_not_built_notice_builds_the_projects_java_code() {
    let dir = project("maven").unwrap();
    write(&dir, &[("src/test/java/common/Helper.java", "class Helper {}")]);
    // a stand-in for Maven: it lists no library, and test-compile writes the helper's class
    #[cfg(windows)]
    write(
        &dir,
        &[(
            "mvnw.cmd",
            "@echo off\r\nif defined GRACE_CLASSPATH_FILE (type nul>\"%GRACE_CLASSPATH_FILE%\" & exit /b 0)\r\n\
             mkdir \"%~dp0target\\test-classes\" 2>nul\r\ntype nul>\"%~dp0target\\test-classes\\Helper.class\"\r\n",
        )],
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = dir.join("mvnw");
        let script = "#!/bin/sh\n\
             if [ -n \"$GRACE_CLASSPATH_FILE\" ]; then : > \"$GRACE_CLASSPATH_FILE\"; exit 0; fi\n\
             classes=\"$(dirname \"$0\")/target/test-classes\"\nmkdir -p \"$classes\"\n: > \"$classes/Helper.class\"\n";
        std::fs::write(&wrapper, script).unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let class = dir.join("target").join("test-classes").join("Helper.class");
    let app = open(&dir);
    let notice = "The project's Java code in src/test/java is not built: build it (mvn test-compile) before running.";
    // "Ready." shows once the open is over: before that the button is off, and a press of it does nothing
    app.wait_for("the notice, its Build button and the open over", WAIT, |t| {
        let said = |start: &str| t.all().iter().any(|n| n.name.starts_with(start));
        said(notice) && said("Ready.") && t.find(&Role::Button, "Build").is_some()
    })
    .unwrap();
    assert!(!class.exists());
    app.press_named(Role::Button, "Build").unwrap();
    app.wait_for("the class built and the notice gone", WAIT, |t| {
        class.exists() && t.all().iter().all(|n| n.name != notice) && t.find(&Role::Button, "Build").is_none()
    })
    .unwrap();
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}
