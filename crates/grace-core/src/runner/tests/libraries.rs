//! `runner.libraries.test.ts` outside "the project's Java code", which is `build`'s. Every build tool here is a
//! stand-in the test writes (P4 plan, porting rule 3): batch files on Windows, `sh` scripts on Linux. The runner
//! looks for one only where `Setup::lookup_env` says, which is nowhere unless a test says so.
use super::*;
use crate::karate::libraries::{INIT_SCRIPT, read_kept};
use crate::model::events::BuildTool;
use crate::report_zip::zip_report;

const FEATURE: &str = "Feature: t\nScenario: lib\n  * match read('classpath:data.json').lib == true\n";

/// A stand-in's body in each platform's language.
#[derive(Default)]
pub(super) struct Script {
    cmd: String,
    sh: String,
}

impl Script {
    pub(super) fn new(cmd: impl Into<String>, sh: impl Into<String>) -> Self {
        Self {
            cmd: cmd.into(),
            sh: sh.into(),
        }
    }

    /// This, then `next`.
    pub(super) fn then(self, next: Script) -> Self {
        Self::new(
            format!("{}\r\n{}", self.cmd, next.cmd),
            format!("{}\n{}", self.sh, next.sh),
        )
    }
}

/// Writes `jar` as the whole list, where Grace asks for it.
pub(super) fn lists(jar: &Path) -> Script {
    let jar = jar.display();
    Script::new(
        format!(">\"%GRACE_CLASSPATH_FILE%\" echo {jar}"),
        format!("echo '{jar}' > \"$GRACE_CLASSPATH_FILE\""),
    )
}

/// Prints `line` and fails.
pub(super) fn fails(line: &str) -> Script {
    Script::new(format!("echo {line}\r\nexit /b 1"), format!("echo '{line}'\nexit 1"))
}

pub(super) fn sleeps(seconds: u32) -> Script {
    Script::new(
        format!("ping -n {} 127.0.0.1 >nul", seconds + 1),
        format!("sleep {seconds}"),
    )
}

/// Logs `late`, which a stopped tool never does.
pub(super) fn logs_late() -> Script {
    Script::new(
        "echo late>>\"%~dp0calls.log\"",
        "echo late >> \"$(dirname \"$0\")/calls.log\"",
    )
}

/// The file name of a stand-in `name` (`mvnw`, `gradlew`, `mvn`, `gradle`).
fn tool_name(name: &str) -> String {
    match (cfg!(windows), name.starts_with("gradle")) {
        (false, _) => name.to_string(),
        (true, true) => format!("{name}.bat"),
        (true, false) => format!("{name}.cmd"),
    }
}

/// Writes a stand-in build tool `name` into `dir`: it logs its arguments to `<dir>/calls.log`, then runs `body`.
/// Returns the log's path.
pub(super) fn stub_tool(dir: &Path, name: &str, body: Script) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let file = dir.join(tool_name(name));
    #[cfg(windows)]
    fs::write(
        &file,
        format!("@echo off\r\necho %*>>\"%~dp0calls.log\"\r\n{}\r\n", body.cmd),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // copied by another process: a script this one holds open for writing, or a Java another test started
        // meanwhile inherited, cannot be run ("Text file busy")
        let source = dir.join("stand-in.txt");
        let text = format!(
            "#!/bin/sh\necho \"$@\" >> \"$(dirname \"$0\")/calls.log\"\n{}\n",
            body.sh
        );
        fs::write(&source, text).unwrap();
        let copied = std::process::Command::new("cp")
            .args([&source, &file])
            .status()
            .unwrap();
        assert!(copied.success());
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    dir.join("calls.log")
}

/// The lines of a stand-in's log; none before its first call.
pub(super) fn calls(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// A project of `build` (`pom.xml` or `build.gradle`) under `parent`, whose one scenario passes only with the listed
/// jar on the classpath.
pub(super) fn project_in(parent: &Path, build: &str) -> (TempDir, String) {
    fs::create_dir_all(parent).unwrap();
    let dir = tempfile::Builder::new().prefix("kk-libs-").tempdir_in(parent).unwrap();
    let text = if build == "pom.xml" { "<project/>" } else { "" };
    write(dir.path(), &[(build, text), ("src/test/java/t.feature", FEATURE)]);
    let root = dir.path().to_string_lossy().into_owned();
    (dir, root)
}

pub(super) fn maven() -> (TempDir, String) {
    project_in(&std::env::temp_dir(), "pom.xml")
}

pub(super) fn gradle() -> (TempDir, String) {
    project_in(&std::env::temp_dir(), "build.gradle")
}

/// `<dir>/lib.jar` holding `data` as data.json: a jar is a zip.
fn make_jar(dir: &Path, data: &str) -> PathBuf {
    let source = temp("kk-jar-");
    write(source.path(), &[("data.json", data)]);
    fs::create_dir_all(dir).unwrap();
    let jar = dir.join("lib.jar");
    zip_report(source.path(), &jar).unwrap();
    jar
}

/// The scenario's jar, in a plain temporary folder: the stand-in's `echo` does not quote it.
pub(super) fn jar() -> (TempDir, PathBuf) {
    let dir = temp("kk-libjar-");
    let jar = make_jar(dir.path(), r#"{ "lib": true }"#);
    (dir, jar)
}

/// A runner that looks for build tools where these variables lead.
pub(super) fn looking(vars: &[(&str, &Path)]) -> TestRunner {
    setup_with(Setup {
        lookup_env: vars.iter().map(|(k, v)| (k.into(), v.into())).collect(),
        ..Setup::default()
    })
}

fn assert_passed(t: &TestRunner) {
    assert_match(
        &t.last("runFinished")["summary"],
        json!({ "scenarioCount": 1, "scenariosPassed": 1 }),
    );
}

/// `touch`: a change of the build file.
pub(super) fn touch(root: &str, build: &str) {
    let file = Path::new(root).join(build);
    let text = fs::read_to_string(&file).unwrap();
    fs::write(file, format!("{text} ")).unwrap();
}

pub(super) const NO_MAVEN: &str = if cfg!(windows) {
    "Grace could not list the project's libraries with Maven: the project has no mvnw.cmd, and mvn is not on PATH, \
     in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA. Tests that use them will fail. Install Maven, or add the Maven \
     wrapper to the project."
} else {
    "Grace could not list the project's libraries with Maven: the project has no mvnw, and mvn is not on PATH, in \
     MAVEN_HOME or in M2_HOME. Tests that use them will fail. Install Maven, or add the Maven wrapper to the project."
};

#[test]
fn runs_mvnw_and_puts_the_listed_jar_on_the_classpath() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "mvnw", lists(&jar));
    t.runner.open(&root).unwrap();
    let file = project_dir(&t.data_dir, &root).join("libraries.txt");
    let called = calls(&log);
    assert_eq!(called.len(), 1, "{called:?}");
    assert!(called[0].contains("dependency:build-classpath"), "{called:?}");
    assert!(
        called[0].contains(&format!("-Dmdep.outputFile={}", file.display())),
        "{called:?}"
    );
    assert!(!file.exists(), "read, then deleted");
    t.runner.run("d:").unwrap();
    assert_passed(&t);
}

#[test]
fn runs_the_maven_of_maven_home_when_the_project_has_no_wrapper() {
    let home = temp("kk-maven-");
    let (_jar, jar) = jar();
    let log = stub_tool(&home.path().join("bin"), "mvn", lists(&jar));
    let t = looking(&[("MAVEN_HOME", home.path())]);
    let (_dir, root) = maven();
    t.runner.open(&root).unwrap();
    assert_eq!(calls(&log).len(), 1);
    t.runner.run("d:").unwrap();
    assert_passed(&t);
}

#[test]
fn runs_gradlew_without_a_daemon() {
    let t = setup();
    let (dir, root) = gradle();
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "gradlew", lists(&jar));
    t.runner.open(&root).unwrap();
    let script = project_dir(&t.data_dir, &root).join("libraries.gradle");
    let called = calls(&log);
    assert_eq!(called.len(), 1, "{called:?}");
    let script_name = script.to_string_lossy();
    for arg in ["--no-daemon", "--init-script", &script_name, "graceClasspath"] {
        assert!(called[0].contains(arg), "{arg}: {called:?}");
    }
    assert_eq!(fs::read_to_string(&script).unwrap(), INIT_SCRIPT);
    t.runner.run("d:").unwrap();
    assert_passed(&t);
}

#[test]
fn keeps_the_list_until_a_build_file_changes() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "mvnw", lists(&jar));
    t.runner.open(&root).unwrap();
    t.runner.refresh().unwrap();
    assert_eq!(calls(&log).len(), 1);
    touch(&root, "pom.xml");
    t.runner.refresh().unwrap();
    assert_eq!(calls(&log).len(), 2);
}

#[test]
fn never_lists_on_an_environment_switch_or_a_run() {
    let t = setup();
    let (dir, root) = maven();
    write(
        dir.path(),
        &[("src/test/java/karate-config-qa.js", "function fn() { return {}; }")],
    );
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "mvnw", lists(&jar));
    t.runner.open(&root).unwrap();
    touch(&root, "pom.xml");
    t.runner.set_env(Some("qa")).unwrap();
    t.runner.run("d:").unwrap();
    assert_eq!(calls(&log).len(), 1);
    assert_passed(&t);
}

#[test]
fn says_when_there_is_no_tool() {
    let t = setup();
    let (_dir, root) = maven();
    t.runner.open(&root).unwrap();
    assert_eq!(t.last_notice(), NO_MAVEN);
    assert_eq!(leaves(Some(&t.tree())), 1);
}

#[test]
fn keeps_the_last_list_when_the_tool_fails() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    stub_tool(dir.path(), "mvnw", lists(&jar));
    t.runner.open(&root).unwrap();
    stub_tool(dir.path(), "mvnw", fails("[ERROR] Could not resolve dependencies"));
    touch(&root, "pom.xml");
    t.runner.refresh().unwrap();
    assert_eq!(
        t.last_notice(),
        "Grace could not list the project's libraries with Maven: Could not resolve dependencies. \
         Grace uses the libraries it listed before."
    );
    t.runner.run("d:").unwrap();
    assert_passed(&t);
}

#[test]
fn ends_the_notice_with_one_full_stop() {
    let t = setup();
    let (dir, root) = maven();
    stub_tool(dir.path(), "mvnw", fails("[ERROR] Could not resolve dependencies."));
    t.runner.open(&root).unwrap();
    assert_eq!(
        t.last_notice(),
        "Grace could not list the project's libraries with Maven: Could not resolve dependencies. \
         Tests that use them will fail."
    );
}

/// R6.1: exiting with 0 without writing the list is a failure too (beyond the Vitest file).
#[test]
fn says_when_the_tool_succeeds_without_writing_a_list() {
    let t = setup();
    let (dir, root) = maven();
    stub_tool(
        dir.path(),
        "mvnw",
        Script::new("echo BUILD SUCCESS", "echo 'BUILD SUCCESS'"),
    );
    t.runner.open(&root).unwrap();
    assert_eq!(
        t.last_notice(),
        "Grace could not list the project's libraries with Maven: BUILD SUCCESS. Tests that use them will fail."
    );
    assert_eq!(read_kept(&project_dir(&t.data_dir, &root)), None);
}

/// R5.1: the project's folders, the home `.karate/ext`, the project's `.karate/ext`, then the build's libraries.
#[test]
fn puts_the_classpath_in_its_order() {
    let t = setup();
    let (dir, root) = maven();
    let from = |place: &str| format!(r#"{{"from":"{place}"}}"#);
    let own = dir.path().join("src/test/java/data.json");
    fs::write(&own, from("project")).unwrap();
    fs::create_dir_all(dir.path().join("target").join("test-classes")).unwrap();
    let home_ext = t.home.path().join(".karate").join("ext");
    let home_jar = make_jar(&home_ext, &from("home"));
    let project_ext = dir.path().join(".karate").join("ext");
    let ext_jar = make_jar(&project_ext, &from("ext"));
    let built = temp("kk-libjar-");
    let built_jar = make_jar(built.path(), &from("build"));
    stub_tool(dir.path(), "mvnw", lists(&built_jar));
    let starts = t.record_spawns();
    t.runner.open(&root).unwrap();

    // the scenario's own order: also Java's arguments
    let args = lock(&starts)[0].args.clone();
    let classpath = &args[args.iter().position(|a| a == "-cp").unwrap() + 1];
    let expected = [
        vendor().jar,
        dir.path().join("src").join("test").join("java"),
        dir.path().join("target").join("test-classes"),
        home_ext.join("*"),
        project_ext.join("*"),
        built_jar.clone(),
    ];
    let entries: Vec<PathBuf> = std::env::split_paths(classpath).collect();
    assert_eq!(entries, expected);

    for (place, next) in [
        ("project", Some(own)),
        ("home", Some(home_jar)),
        ("ext", Some(ext_jar)),
        ("build", None),
    ] {
        write(
            dir.path(),
            &[(
                "src/test/java/t.feature",
                &format!("Feature: t\nScenario: lib\n  * match read('classpath:data.json').from == '{place}'\n"),
            )],
        );
        t.runner.run("d:").unwrap();
        assert_passed(&t);
        if let Some(next) = next {
            fs::remove_file(next).unwrap();
        }
    }
}

#[test]
fn joins_the_open_notice() {
    let t = setup();
    let (dir, root) = maven();
    write(dir.path(), &[("karate-pom.json", r#"{"dryRun":true}"#)]);
    t.runner.open(&root).unwrap();
    assert_eq!(
        t.last_notice(),
        format!("Grace does not use these karate-pom.json settings: dryRun. {NO_MAVEN}")
    );
}

#[test]
fn runs_the_tool_in_a_folder_whose_path_has_spaces_and_ampersands() {
    let t = setup();
    let parent = temp("kk-odd-");
    let (dir, root) = project_in(&parent.path().join("a b&c"), "pom.xml");
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "mvnw", lists(&jar));
    t.runner.open(&root).unwrap();
    assert_eq!(calls(&log).len(), 1);
    t.runner.run("d:").unwrap();
    assert_passed(&t);
}

#[test]
fn says_it_is_listing_also_in_a_replay() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    stub_tool(dir.path(), "mvnw", sleeps(1).then(lists(&jar)));
    let replayed: Arc<Mutex<Vec<AppEvent>>> = Arc::default();
    {
        let (replayed, runner) = (Arc::clone(&replayed), t.runner.clone());
        t.on_event(move |e| {
            if matches!(e, AppEvent::Listing { tool: Some(_), .. }) {
                *lock(&replayed) = runner.replay();
            }
        });
    }
    t.runner.open(&root).unwrap();
    let listing = AppEvent::Listing {
        tool: Some(BuildTool::Maven),
        build: false,
    };
    assert!(lock(&replayed).contains(&listing), "{:?}", lock(&replayed));
    assert_eq!(
        t.all("listing"),
        [
            json!({ "type": "listing", "tool": "Maven" }),
            json!({ "type": "listing", "tool": null }),
        ]
    );
    assert!(t.runner.replay().iter().all(|e| !matches!(e, AppEvent::Listing { .. })));
}

#[test]
fn stops_the_tool_on_cancel() {
    let t = setup();
    let (dir, root) = maven();
    let (_jar, jar) = jar();
    let log = stub_tool(dir.path(), "mvnw", sleeps(3).then(logs_late()).then(lists(&jar)));
    {
        let runner = t.runner.clone();
        t.on_event(move |e| {
            if matches!(e, AppEvent::Listing { tool: Some(_), .. }) {
                runner.cancel();
            }
        });
    }
    assert!(!t.runner.open(&root).unwrap());
    assert_eq!(t.last_notice(), "Cancelled");
    thread::sleep(Duration::from_secs(5));
    assert!(!calls(&log).contains(&"late".to_string()), "{:?}", calls(&log));
    assert_eq!(read_kept(&project_dir(&t.data_dir, &root)), None);
}

#[test]
fn runs_no_tool_for_a_standalone_project() {
    let t = setup();
    let (dir, root) = project(&[("t.feature", "Feature: t\nScenario: s\n  * print 1\n")]);
    let log = stub_tool(dir.path(), "mvnw", Script::default());
    t.runner.open(&root).unwrap();
    assert!(calls(&log).is_empty());
}

// Beyond the Vitest file: the find-build-tools scenarios that open a project, which `find_tool`'s tests do not do.

/// Opens a Maven or Gradle project without a wrapper where `vars` lead to `chosen`'s stand-in, which must be the one
/// that runs; `other`, when given, must not.
fn lists_with(project: (TempDir, String), vars: &[(&str, &Path)], chosen: &Path, name: &str, other: Option<&Path>) {
    let (_jar, jar) = jar();
    let log = stub_tool(chosen, name, lists(&jar));
    let other = other.map(|dir| stub_tool(dir, name, lists(&jar)));
    let t = looking(vars);
    t.runner.open(&project.1).unwrap();
    assert_eq!(calls(&log).len(), 1);
    assert!(other.is_none_or(|log| calls(&log).is_empty()));
    assert_eq!(t.notices(), Vec::<String>::new());
}

/// FB-02
#[test]
fn opening_lists_with_m2_homes_maven_when_maven_home_has_none() {
    let (empty, m2) = (temp("kk-maven-"), temp("kk-m2-"));
    lists_with(
        maven(),
        &[("MAVEN_HOME", empty.path()), ("M2_HOME", m2.path())],
        &m2.path().join("bin"),
        "mvn",
        None,
    );
}

/// FB-03
#[test]
fn opening_lists_with_gradle_homes_gradle() {
    let home = temp("kk-gradle-");
    lists_with(
        gradle(),
        &[("GRADLE_HOME", home.path())],
        &home.path().join("bin"),
        "gradle",
        None,
    );
}

/// FB-04
#[test]
fn opening_lists_with_the_maven_on_path_before_maven_homes() {
    let (on_path, home) = (temp("kk-path-"), temp("kk-maven-"));
    lists_with(
        maven(),
        &[("PATH", on_path.path()), ("MAVEN_HOME", home.path())],
        on_path.path(),
        "mvn",
        Some(&home.path().join("bin")),
    );
}

/// FB-05
#[cfg(windows)]
#[test]
fn opening_lists_with_the_newest_intellij_ideas_maven() {
    let (program_files, local) = (temp("kk-pf-"), temp("kk-lad-"));
    let ide = |parent: &Path, folder: &str, build: &str| {
        let ide = parent.join(folder).join("IntelliJ IDEA");
        write(&ide, &[("build.txt", build)]);
        ide.join(r"plugins\maven\lib\maven3\bin")
    };
    let older = ide(program_files.path(), "JetBrains", "IU-243.21565.193");
    let newer = ide(local.path(), "Programs", "IU-243.21565.1000");
    lists_with(
        maven(),
        &[("ProgramFiles", program_files.path()), ("LOCALAPPDATA", local.path())],
        &newer,
        "mvn",
        Some(&older),
    );
}
