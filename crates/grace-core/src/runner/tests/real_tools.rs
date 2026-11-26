//! Temporary, for P4's exit check (plan, open question 1): one real library listing and one real Build each with Maven
//! and with Gradle, found as a person's are, on GitHub's Windows runner. Removed once it has run.
use super::*;
use crate::karate::compiled::read_built;
use crate::karate::libraries::{find_tool, read_kept};
use crate::model::events::BuildTool;

const HELPER: &str = "import org.apache.commons.lang3.StringUtils;\n\npublic class Helper {\n    \
                      public static String shout(String s) {\n        return StringUtils.upperCase(s);\n    }\n}\n";
/// Passes only with the project's compiled `Helper` and its listed library on the classpath.
const FEATURE: &str = "Feature: real\nScenario: the project's Java code and its library\n  \
                       * match Java.type('Helper').shout('grace') == 'GRACE'\n";
const POM: &str = "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n  <modelVersion>4.0.0</modelVersion>\n  \
    <groupId>grace</groupId>\n  <artifactId>real</artifactId>\n  <version>1</version>\n  <properties>\n    \
    <maven.compiler.release>21</maven.compiler.release>\n    \
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>\n  </properties>\n  <dependencies>\n    \
    <dependency>\n      <groupId>org.apache.commons</groupId>\n      <artifactId>commons-lang3</artifactId>\n      \
    <version>3.17.0</version>\n      <scope>test</scope>\n    </dependency>\n  </dependencies>\n</project>\n";
const BUILD_GRADLE: &str = "plugins { id 'java' }\nrepositories { mavenCentral() }\ndependencies { \
                            testImplementation 'org.apache.commons:commons-lang3:3.17.0' }\n";

fn lists_builds_and_runs(tool: BuildTool, files: &[(&str, &str)]) {
    let lookup: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    let t = setup_with(Setup {
        lookup_env: lookup.clone(),
        ..Setup::default()
    });
    let (dir, root) = project(files);
    write(
        dir.path(),
        &[
            ("src/test/java/Helper.java", HELPER),
            ("src/test/java/real.feature", FEATURE),
        ],
    );
    println!("{tool:?}: launcher {:?}", find_tool(dir.path(), tool, &lookup));

    let started = Instant::now();
    t.runner.open(&root).unwrap();
    let data = project_dir(&t.data_dir, &root);
    println!("{tool:?}: opened in {:?}, notices {:?}", started.elapsed(), t.notices());
    let kept = read_kept(&data).expect("a list kept");
    println!("{tool:?}: listed {:?}", kept.jars);
    assert!(kept.jars.iter().any(|j| j.contains("commons-lang3")));
    assert!(t.last_notice().contains("is not built"));
    assert_eq!(t.last("notice")["build"], true);

    let before = t.len();
    let started = Instant::now();
    t.runner.build().unwrap();
    println!("{tool:?}: built in {:?}", started.elapsed());
    let after = &t.json()[before..];
    assert!(after.iter().all(|e| e["type"] != "notice"), "{after:?}");
    assert!(read_built(&data).is_some());

    t.runner.run("d:").unwrap();
    let summary = t.last("runFinished")["summary"].clone();
    println!("{tool:?}: run {summary}");
    assert_match(&summary, json!({ "scenarioCount": 1, "scenariosPassed": 1 }));
}

#[test]
#[ignore = "needs a real Maven and a JDK"]
fn real_maven() {
    lists_builds_and_runs(BuildTool::Maven, &[("pom.xml", POM)]);
}

#[test]
#[ignore = "needs a real Gradle and a JDK"]
fn real_gradle() {
    lists_builds_and_runs(
        BuildTool::Gradle,
        &[("build.gradle", BUILD_GRADLE), ("settings.gradle", "")],
    );
}
