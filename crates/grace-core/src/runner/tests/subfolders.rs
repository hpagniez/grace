//! `runner.subfolders.test.ts`: the offer of the Karate projects below a folder with no test, and opening one. Every
//! build tool is a stand-in the test writes (P4 plan, porting rule 3), as in `libraries`.
use super::libraries::{Script, calls, stub_tool};
use super::*;
use crate::settings::load_recent_projects;

const BASIC: &str = "Feature: basic\nScenario: one\n  * print 1\nScenario: two\n  * print 2\n";
const ONE: &str = "Feature: t\nScenario: s\n  * print 1\n";
const CONFIG: &str = "function fn() { return {}; }";

/// A stand-in that only logs its call.
fn tool(dir: &Path) -> PathBuf {
    stub_tool(dir, "mvnw", Script::new("rem", ":"))
}

/// Another tool's pom.xml at the root, a stand-in Maven wrapper, and the Karate project in `karate/`.
fn repo() -> (TempDir, String, PathBuf) {
    let (dir, root) = project(&[
        ("pom.xml", "<project/>"),
        ("karate/karate-config.js", CONFIG),
        ("karate/tests/basic.feature", BASIC),
    ]);
    let log = tool(dir.path());
    (dir, root, log)
}

/// The folders the last tree offered; none when its event has no `subfolders`.
fn offered(t: &TestRunner) -> Vec<String> {
    match t.last_event("project") {
        AppEvent::Project { subfolders, .. } => subfolders.unwrap_or_default(),
        e => panic!("not a project event: {e:?}"),
    }
}

/// The root of the project shown.
fn shown(t: &TestRunner) -> String {
    t.last("project")["project"]["root"].as_str().unwrap().to_string()
}

fn under(root: &str, folder: &str) -> String {
    Path::new(root).join(folder).to_string_lossy().into_owned()
}

#[test]
fn offers_the_karate_project_below_a_folder_with_no_test_without_listing_libraries() {
    let t = setup();
    let (_dir, root, log) = repo();
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("project")["subfolders"], json!(["karate"]));
    assert_eq!(leaf_keys(&t.tree()), Vec::<String>::new());
    assert_eq!(calls(&log), Vec::<String>::new());
    assert_eq!(t.notices(), Vec::<String>::new());
}

#[test]
fn skips_the_listing_on_refresh_too() {
    let t = setup();
    let (_dir, root, log) = repo();
    t.runner.open(&root).unwrap();
    t.runner.refresh().unwrap();
    assert_eq!(calls(&log), Vec::<String>::new());
    assert_eq!(t.notices(), Vec::<String>::new());
}

#[test]
fn lists_libraries_when_karate_pom_json_paths_give_feature_roots() {
    let t = setup();
    let (dir, root) = project(&[
        ("pom.xml", "<project/>"),
        ("karate-pom.json", r#"{"paths":["tests"]}"#),
        ("tests/t.feature", ONE),
    ]);
    let log = tool(dir.path());
    t.runner.open(&root).unwrap();
    assert_eq!(calls(&log).len(), 1);
}

#[test]
fn offers_nothing_when_the_tree_has_tests() {
    let t = setup();
    let (_dir, root) = project(&[("t.feature", ONE), ("sub/karate-config.js", CONFIG)]);
    t.runner.open(&root).unwrap();
    assert_eq!(leaf_keys(&t.tree()).len(), 1);
    assert_eq!(t.last("project").get("subfolders"), None);
}

#[test]
fn offers_nothing_after_a_failed_dry_run() {
    let t = setup();
    let broken = fixture("broken");
    write(broken.path(), &[("sub/karate-pom.json", "{}")]);
    t.runner.open(&broken.root).unwrap();
    let p = t.last("project");
    assert!(p["error"].as_str().unwrap().contains("syntax.feature"), "{p}");
    assert_eq!(p.get("subfolders"), None);
}

#[test]
fn keeps_the_offer_in_the_replay() {
    let t = setup();
    let (_dir, root, _log) = repo();
    t.runner.open(&root).unwrap();
    assert_match(
        &json(&t.runner.replay()[0]),
        json!({ "type": "project", "subfolders": ["karate"] }),
    );
}

#[test]
fn opens_an_offered_folder_as_any_project() {
    let t = setup();
    let (_dir, root, _log) = repo();
    t.runner.open(&root).unwrap();
    t.runner.open_subfolder("karate").unwrap();
    assert_eq!(shown(&t), under(&root, "karate"));
    assert_eq!(leaf_keys(&t.tree()).len(), 2);
    assert_eq!(load_recent_projects(&t.data_dir)[0], under(&root, "karate"));
}

#[test]
fn refuses_a_folder_it_did_not_offer() {
    let t = setup();
    let (_dir, root, _log) = repo();
    t.runner.open(&root).unwrap();
    for folder in ["../elsewhere", "karate/"] {
        t.runner.open_subfolder(folder).unwrap();
        assert_eq!(t.last_notice(), "Invalid request");
        assert_eq!(shown(&t), root);
    }
    t.runner.open_subfolder("karate").unwrap();
    assert_eq!(t.notices().len(), 2);
    t.runner.open_subfolder("karate").unwrap(); // karate's own tree offers nothing
    assert_eq!(t.notices().len(), 3);
    assert_eq!(t.last_notice(), "Invalid request");
    assert_eq!(shown(&t), under(&root, "karate"));
}

/// Not in the TypeScript: the offer's `/` becomes the platform's separator, as `path.join` made it, since the root
/// names the project's data folder and its place in the recent list.
#[test]
fn opens_a_nested_offer_with_the_platforms_separators() {
    let t = setup();
    let (_dir, root) = project(&[("tests/api/karate-config.js", CONFIG)]);
    t.runner.open(&root).unwrap();
    assert_eq!(offered(&t), ["tests/api"]);
    t.runner.open_subfolder("tests/api").unwrap();
    let nested = Path::new(&root).join("tests").join("api");
    assert_eq!(shown(&t), nested.to_string_lossy());
}

/// Not in the TypeScript: sub-folder offer R4.3, "picking a folder is refused as any open is" while Grace is busy.
#[test]
fn refuses_an_offered_folder_while_busy() {
    let t = setup();
    let (_dir, root) = project(&[("sub/karate-config.js", CONFIG)]);
    t.runner.open(&root).unwrap();
    let runner = t.runner.clone();
    t.on_event(move |e| {
        if matches!(e, AppEvent::Busy { busy: true }) {
            runner.open_subfolder("sub").unwrap();
        }
    });
    t.runner.refresh().unwrap();
    assert_eq!(t.notices(), ["A run is already in progress"]);
    assert_eq!(shown(&t), root);
}

/// The scenario outline "What looks like a Karate project", each row in a repository of its own.
#[test]
fn offers_what_looks_like_a_karate_project() {
    let rows: [(&str, &[&str], bool); 8] = [
        ("api", &["karate-pom.json"], true),
        ("gradle-tests", &["build.gradle", "src/test/java/a/b.feature"], true),
        ("maven-tests", &["pom.xml", "src/test/resources/c.feature"], true),
        ("tool", &["pom.xml", "src/test/java/Helper.java"], false),
        ("a/b/c", &["karate-config.js"], true),
        ("a/b/c/d", &["karate-config.js"], false),
        ("node_modules/x", &["karate-config.js"], false),
        ("target/test-classes", &["karate-config.js"], false),
    ];
    let t = setup();
    for (folder, files, offers) in rows {
        let (dir, root) = project(&[("pom.xml", "<project/>")]);
        for file in files {
            write(dir.path(), &[(&format!("{folder}/{file}"), "")]);
        }
        t.runner.open(&root).unwrap();
        let expected: &[&str] = if offers { &[folder] } else { &[] };
        assert_eq!(offered(&t), expected, "{folder} with {files:?}");
    }
}

/// The scenario "Grace does not look inside an offered folder".
#[test]
fn offers_no_folder_inside_an_offered_one() {
    let t = setup();
    let (_dir, root) = project(&[
        ("pom.xml", "<project/>"),
        ("karate/karate-config.js", CONFIG),
        ("karate/nested/karate-pom.json", "{}"),
    ]);
    t.runner.open(&root).unwrap();
    assert_eq!(offered(&t), ["karate"]);
}
