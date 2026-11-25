//! `runner.libraries.test.ts` under "the project's Java code": the not-built notice and its Build button. Every build
//! tool is a stand-in the test writes (P4 plan, porting rule 3), as in `libraries`.
use super::libraries::{
    NO_MAVEN, Script, calls, fails, gradle, jar, lists, logs_late, looking, maven, project_in, sleeps, stub_tool, touch,
};
use super::*;
use crate::karate::compiled::read_built;
use crate::karate::libraries::JAVAC;
use crate::model::events::BuildTool;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

const NOT_BUILT: &str =
    "The project's Java code in src/test/java is not built: build it (mvn test-compile) before running.";
const CHANGED: &str = "The project's Java code in src/test/java changed since its last build: build it again (mvn \
                       test-compile) before running.";
const GRADLE_NOT_BUILT: &str =
    "The project's Java code in src/test/java is not built: build it (gradle testClasses) before running.";
const GRADLE_CHANGED: &str = "The project's Java code in src/test/java changed since its last build: build it again \
                              (gradle testClasses) before running.";
const FAILED: &str = "Grace could not build the project's Java code with Maven: COMPILATION ERROR.";
const NEEDS_JDK: &str = "Building needs a JDK: set JAVA_HOME to one.";
const NO_TOOL: &str = if cfg!(windows) {
    "Grace could not build the project's Java code with Maven: the project has no mvnw.cmd, and mvn is not on PATH, \
     in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA."
} else {
    "Grace could not build the project's Java code with Maven: the project has no mvnw, and mvn is not on PATH, in \
     MAVEN_HOME or in M2_HOME."
};

/// A notice that offers Build, as the window receives it.
fn offers(message: &str) -> Value {
    json!({ "type": "notice", "message": message, "build": true })
}

/// Writes the helper's class, as `test-compile` would, and logs its `JAVA_HOME`.
fn builds() -> Script {
    Script::new(
        "mkdir \"%~dp0target\\test-classes\" 2>nul\r\ntype nul>\"%~dp0target\\test-classes\\Helper.class\"\r\n\
         >>\"%~dp0calls.log\" echo JAVA_HOME=%JAVA_HOME%",
        "d=\"$(dirname \"$0\")\"\nmkdir -p \"$d/target/test-classes\"\n: > \"$d/target/test-classes/Helper.class\"\n\
         echo \"JAVA_HOME=$JAVA_HOME\" >> \"$d/calls.log\"",
    )
}

/// A compiler error, as `-B -q` prints Maven's.
fn compile_error() -> Script {
    fails("[ERROR] COMPILATION ERROR")
}

/// Ends with exit code 0 and writes nothing.
fn does_nothing() -> Script {
    Script::new("rem", ":")
}

/// A project's folder, its root and its libraries' jar.
type Helper = (TempDir, String, TempDir);

/// `project` with a helper's source, its libraries listed by a stand-in `tool`, and none of its code built.
fn with_helper_of(project: (TempDir, String), tool: &str) -> Helper {
    let (dir, root) = project;
    let (jar_dir, jar) = jar();
    stub_tool(dir.path(), tool, lists(&jar));
    write(dir.path(), &[("src/test/java/Helper.java", "class Helper {}")]);
    (dir, root, jar_dir)
}

/// `withHelper()`: the Maven one.
fn with_helper() -> Helper {
    with_helper_of(maven(), "mvnw")
}

/// The calls of `log` that compile the test code.
fn compiles(log: &Path) -> Vec<String> {
    calls(log)
        .into_iter()
        .filter(|line| line.contains("test-compile"))
        .collect()
}

/// Writes `file` if it is not there, and sets its modified time.
fn at(file: &Path, time: SystemTime) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let file = fs::File::options().create(true).append(true).open(file).unwrap();
    file.set_modified(time).unwrap();
}

/// A time long past, `seconds` after the Unix epoch.
fn past(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

/// A JDK's folder: `bin/javac`.
fn jdk() -> TempDir {
    let dir = temp("kk-jdk-");
    write(dir.path(), &[(&format!("bin/{JAVAC}"), "")]);
    dir
}

/// Calls `f` when Grace says it is building.
fn on_building(t: &TestRunner, f: impl Fn() + Send + Sync + 'static) {
    t.on_event(move |e| {
        if matches!(e, AppEvent::Listing { build: true, .. }) {
            f();
        }
    });
}

#[test]
fn says_on_open_when_it_is_not_built() {
    let t = setup();
    let (_dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    assert_eq!(t.last_notice(), NOT_BUILT);
}

#[test]
fn comes_after_the_libraries_notice() {
    let t = setup();
    let (dir, root) = maven();
    write(dir.path(), &[("src/test/java/Helper.java", "class Helper {}")]);
    t.runner.open(&root).unwrap();
    assert_eq!(t.last_notice(), format!("{NO_MAVEN} {NOT_BUILT}"));
}

#[test]
fn says_nothing_once_built_and_says_on_refresh_when_it_changed_since() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    let source = dir.path().join("src/test/java/Helper.java");
    at(&source, past(1_000_000_000));
    at(
        &dir.path().join("target/test-classes/Helper.class"),
        past(1_000_000_100),
    );
    t.runner.open(&root).unwrap();
    t.runner.refresh().unwrap();
    assert_eq!(t.notices(), Vec::<String>::new());
    at(&source, past(1_000_000_200));
    t.runner.refresh().unwrap();
    assert_eq!(t.last_notice(), CHANGED);
}

#[test]
fn offers_build_with_the_not_built_notice() {
    let t = setup();
    let (_dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("notice"), offers(NOT_BUILT));
}

#[test]
fn offers_no_build_without_a_tool() {
    let t = setup();
    let (dir, root) = maven();
    write(dir.path(), &[("src/test/java/Helper.java", "class Helper {}")]);
    t.runner.open(&root).unwrap();
    assert!(t.last_notice().contains(NOT_BUILT));
    assert!(t.last("notice").get("build").is_none());
}

#[test]
fn offers_build_in_a_joined_notice() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    write(dir.path(), &[("karate-pom.json", r#"{"dryRun":true}"#)]);
    t.runner.open(&root).unwrap();
    assert_eq!(
        t.last("notice"),
        offers(&format!(
            "Grace does not use these karate-pom.json settings: dryRun. {NOT_BUILT}"
        ))
    );
}

#[test]
fn offers_build_after_the_libraries_failure_on_refresh() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    stub_tool(dir.path(), "mvnw", fails("[ERROR] Could not resolve dependencies"));
    touch(&root, "pom.xml");
    t.runner.refresh().unwrap();
    assert_eq!(
        t.last("notice"),
        offers(&format!(
            "Grace could not list the project's libraries with Maven: Could not resolve dependencies. \
             Grace uses the libraries it listed before. {NOT_BUILT}"
        ))
    );
}

#[test]
fn builds_with_test_compile_then_refreshes() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    let log = stub_tool(dir.path(), "mvnw", builds());
    let before = t.len();
    t.runner.build().unwrap();
    assert_eq!(compiles(&log), ["-B -q test-compile"]);
    // the person's environment as is: never Grace's Java, which cannot compile (R2.2)
    let own = std::env::var("JAVA_HOME").unwrap_or_default();
    assert!(calls(&log).contains(&format!("JAVA_HOME={own}")), "{:?}", calls(&log));
    let after = &t.json()[before..];
    assert!(after.iter().any(|e| e["type"] == "project"), "{after:?}");
    assert!(after.iter().all(|e| e["type"] != "notice"), "{after:?}");
    assert!(read_built(&project_dir(&t.data_dir, &root)).is_some());
}

#[test]
fn builds_in_a_folder_whose_path_has_spaces_and_ampersands() {
    let t = setup();
    let parent = temp("kk-odd-");
    let (dir, root, _jar) = with_helper_of(project_in(&parent.path().join("a b&c"), "pom.xml"), "mvnw");
    t.runner.open(&root).unwrap();
    let log = stub_tool(dir.path(), "mvnw", builds());
    t.runner.build().unwrap();
    assert_eq!(compiles(&log), ["-B -q test-compile"]);
}

#[test]
fn says_the_builds_first_error_and_keeps_build() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    stub_tool(dir.path(), "mvnw", compile_error());
    let before = t.len();
    t.runner.build().unwrap();
    assert_eq!(t.last("notice"), offers(&format!("{FAILED} {NEEDS_JDK}")));
    // the tree is as it was
    assert!(t.json()[before..].iter().all(|e| e["type"] != "project"));
}

#[test]
fn says_nothing_of_a_jdk_when_java_home_has_javac() {
    let jdk = jdk();
    let t = looking(&[("JAVA_HOME", jdk.path())]);
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    stub_tool(dir.path(), "mvnw", compile_error());
    t.runner.build().unwrap();
    assert_eq!(t.last_notice(), FAILED);
}

#[test]
fn stops_the_build_on_cancel() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    let log = stub_tool(dir.path(), "mvnw", sleeps(3).then(logs_late()));
    let runner = t.runner.clone();
    on_building(&t, move || runner.cancel());
    t.runner.build().unwrap();
    assert_eq!(t.last("notice"), offers("Build cancelled."));
    thread::sleep(Duration::from_secs(5));
    assert!(!calls(&log).contains(&"late".to_string()), "{:?}", calls(&log));
    assert_eq!(read_built(&project_dir(&t.data_dir, &root)), None);
}

#[test]
fn says_it_is_building_also_in_a_replay() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    stub_tool(dir.path(), "mvnw", sleeps(1));
    let replayed: Arc<Mutex<Vec<AppEvent>>> = Arc::default();
    {
        let (replayed, runner) = (Arc::clone(&replayed), t.runner.clone());
        on_building(&t, move || *lock(&replayed) = runner.replay());
    }
    let before = t.len();
    t.runner.build().unwrap();
    let building = AppEvent::Listing {
        tool: Some(BuildTool::Maven),
        build: true,
    };
    assert!(lock(&replayed).contains(&building), "{:?}", lock(&replayed));
    let listing: Vec<Value> = t.json()[before..]
        .iter()
        .filter(|e| e["type"] == "listing")
        .cloned()
        .collect();
    assert_eq!(
        listing,
        [
            json!({ "type": "listing", "tool": "Maven", "build": true }),
            json!({ "type": "listing", "tool": null }),
        ]
    );
}

#[test]
fn refuses_a_second_build_while_one_runs() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    let log = stub_tool(dir.path(), "mvnw", sleeps(1).then(compile_error()));
    let (started, building) = mpsc::channel();
    on_building(&t, move || {
        let _ = started.send(());
    });
    let first = t.bg(|runner| runner.build().unwrap());
    building.recv_timeout(Duration::from_secs(60)).unwrap();
    t.runner.build().unwrap();
    assert_eq!(t.last_notice(), "A run is already in progress");
    first.join().unwrap();
    assert_eq!(compiles(&log).len(), 1);
}

#[test]
fn says_when_the_tool_is_gone_at_the_click() {
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    fs::remove_file(dir.path().join(if cfg!(windows) { "mvnw.cmd" } else { "mvnw" })).unwrap();
    t.runner.build().unwrap();
    assert_eq!(t.last("notice"), json!({ "type": "notice", "message": NO_TOOL }));
}

#[test]
fn a_gradle_build_that_rewrites_no_class_still_counts() {
    let t = setup();
    let (dir, root, _jar) = with_helper_of(gradle(), "gradlew");
    let source = dir.path().join("src/test/java/Helper.java");
    at(&source, past(1_000_000_200));
    at(
        &dir.path().join("build/classes/java/test/Helper.class"),
        past(1_000_000_100),
    );
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("notice"), offers(GRADLE_CHANGED));
    let log = stub_tool(dir.path(), "gradlew", does_nothing());
    let before = t.len();
    t.runner.build().unwrap();
    assert_eq!(
        calls(&log).last().unwrap(),
        "-q --no-daemon --console=plain testClasses"
    );
    assert!(t.json()[before..].iter().all(|e| e["type"] != "notice"));
    at(&source, SystemTime::now() + Duration::from_secs(10));
    t.runner.refresh().unwrap();
    assert_eq!(t.last_notice(), GRADLE_CHANGED);
}

#[test]
fn runs_nothing_with_no_project_or_for_a_standalone_project() {
    let t = setup();
    t.runner.build().unwrap();
    assert_eq!(t.count("busy"), 0);
    let (dir, root) = project(&[("t.feature", "Feature: t\nScenario: s\n  * print 1\n")]);
    let log = stub_tool(dir.path(), "mvnw", Script::default());
    t.runner.open(&root).unwrap();
    let busy = t.count("busy");
    t.runner.build().unwrap();
    assert!(calls(&log).is_empty());
    assert_eq!(t.count("busy"), busy);
}

// Beyond the Vitest file: the scenarios of project libraries and build from the notice that none of its tests plays.

/// PL-12 (#236): the only Java code is the JUnit runner, which Grace never runs.
#[test]
fn a_junit_runner_needs_no_build() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    stub_tool(dir.path(), "mvnw", lists(&jar));
    write(
        dir.path(),
        &[(
            "src/test/java/examples/ExamplesTest.java",
            "package examples;\n\nimport io.karatelabs.junit6.Karate;\n\nclass ExamplesTest {}\n",
        )],
    );
    t.runner.open(&root).unwrap();
    assert_eq!(t.notices(), Vec::<String>::new());
}

/// BN-05
#[test]
fn builds_a_gradle_project_with_test_classes_without_a_daemon() {
    let t = setup();
    let (dir, root, _jar) = with_helper_of(gradle(), "gradlew");
    t.runner.open(&root).unwrap();
    assert_eq!(t.last("notice"), offers(GRADLE_NOT_BUILT));
    let log = stub_tool(
        dir.path(),
        "gradlew",
        Script::new(
            "mkdir \"%~dp0build\\classes\\java\\test\" 2>nul\r\n\
             type nul>\"%~dp0build\\classes\\java\\test\\Helper.class\"",
            "d=\"$(dirname \"$0\")/build/classes/java/test\"\nmkdir -p \"$d\"\n: > \"$d/Helper.class\"",
        ),
    );
    let before = t.len();
    t.runner.build().unwrap();
    assert_eq!(
        calls(&log).last().unwrap(),
        "-q --no-daemon --console=plain testClasses"
    );
    let after = &t.json()[before..];
    assert!(
        after.contains(&json!({ "type": "listing", "tool": "Gradle", "build": true })),
        "{after:?}"
    );
    assert!(after.iter().all(|e| e["type"] != "notice"), "{after:?}");
}

/// BN-07
#[test]
fn says_building_needs_a_jdk_after_the_compilers_question() {
    const QUESTION: &str = "No compiler is provided in this environment. Perhaps you are running on a JRE rather than \
                            a JDK?";
    let t = setup();
    let (dir, root, _jar) = with_helper();
    t.runner.open(&root).unwrap();
    stub_tool(dir.path(), "mvnw", fails(&format!("[ERROR] {QUESTION}")));
    t.runner.build().unwrap();
    assert_eq!(
        t.last_notice(),
        format!("Grace could not build the project's Java code with Maven: {QUESTION} {NEEDS_JDK}")
    );
}

/// BN-10
#[test]
fn builds_only_on_a_click() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "mvnw", lists(&jar));
    write(dir.path(), &[("src/test/java/Helper.java", "class Helper {}")]);
    t.runner.open(&root).unwrap();
    t.runner.refresh().unwrap();
    t.runner.run("d:").unwrap();
    assert!(!calls(&log).is_empty(), "the libraries are listed");
    assert!(compiles(&log).is_empty(), "{:?}", calls(&log));
}
