//! The parts of opening and running a project that phase P4 ports: Maven and Gradle libraries (`karate/libraries.ts`),
//! the test code notice and Build (`karate/compiled.ts`), pom.xml's `karate.env` (`karate/maven.ts`), and the
//! sub-folder offer (`karate/subfolders.ts`). The runner calls them where `runner.ts` does, through this trait; phase
//! P1 plugs in `NoSetup`, which has nothing to say, so a Maven or Gradle project runs on the classpath roots
//! `command::detect_layout` finds, without its libraries.
use crate::karate::pom::{PomEnv, read_pom};
use crate::model::Layout;
use crate::model::tags::FilterState;
use std::path::{Path, PathBuf};

/// `javaCode`'s answer: the test code notice (build from the notice R8), and whether it offers Build (R1.1–R1.2).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JavaCode {
    pub said: Option<String>,
    pub build: bool,
}

/// What phase P4 plugs into the runner. Every method has the answer of a project that needs none of it.
// Seam for P4: `list_libraries` will also need the runner's way to run a build tool as its cancellable process, with
// `listing` events (`runTool`); P4 adds it to this trait's arguments with `libraries.rs`.
pub trait ProjectSetup: Send + Sync {
    /// `listLibraries`: lists a Maven or Gradle project's libraries with its own build when the kept list is stale
    /// (project libraries spec R2–R6). The notice of a failure; None when listed, still good, standalone or cancelled.
    fn list_libraries(&self, _root: &Path, _layout: Layout) -> Option<String> {
        None
    }

    /// `keptJars`: the libraries listed for the project, put on Java's classpath after the extension folders.
    fn kept_jars(&self, _project_dir: &Path, _layout: Layout) -> Vec<PathBuf> {
        Vec::new()
    }

    /// `javaCode`: the notice about test code that is not compiled, with the last Build's time.
    fn java_code(&self, _root: &Path, _layout: Layout, _project_dir: &Path) -> JavaCode {
        JavaCode::default()
    }

    /// `mavenEnv`: pom.xml's `karate.env`, which a project starts with when karate-pom.json gives no `env`
    /// (pom defaults R3).
    fn maven_env(&self, _root: &Path) -> Option<String> {
        None
    }

    /// `karateFolders`: folders below the project that look like a Karate project, offered when the tree has no test
    /// (sub-folder offer R4).
    fn karate_folders(&self, _root: &Path) -> Vec<String> {
        Vec::new()
    }
}

/// Phase P1's setup: no libraries, no notice, no pom.xml environment, no sub-folder offer.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSetup;

impl ProjectSetup for NoSetup {}

/// `pomStart`: the starting values the project's pom files give (pom defaults R2, R3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PomStart {
    pub env: Option<String>,
    /// karate-pom.json's tag filter; never hides (`hide_non_matching` false).
    pub filter: Option<FilterState>,
    pub threads: Option<u32>,
}

/// `pomStart`: karate-pom.json's values, and pom.xml's `karate.env` (through `setup`) when karate-pom.json gives no
/// `env`.
pub fn pom_start(root: &Path, setup: &dyn ProjectSetup) -> PomStart {
    let pom = read_pom(root);
    PomStart {
        env: match pom.env {
            PomEnv::NotGiven => setup.maven_env(root),
            PomEnv::Unused => None,
            PomEnv::Given(env) => Some(env),
        },
        filter: pom.filter,
        threads: pom.threads,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct PomXml;

    impl ProjectSetup for PomXml {
        fn maven_env(&self, _root: &Path) -> Option<String> {
            Some("from-pom-xml".into())
        }
    }

    #[test]
    fn takes_karate_pom_json_then_pom_xml() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(pom_start(dir.path(), &NoSetup), PomStart::default());
        assert_eq!(pom_start(dir.path(), &PomXml).env.as_deref(), Some("from-pom-xml"));
        fs::write(
            dir.path().join("karate-pom.json"),
            r#"{ "env": "qa", "tags": ["@smoke"], "threads": 3 }"#,
        )
        .unwrap();
        assert_eq!(
            pom_start(dir.path(), &PomXml),
            PomStart {
                env: Some("qa".into()),
                filter: Some(FilterState {
                    include: vec!["smoke".into()],
                    exclude: vec![],
                    hide_non_matching: false,
                }),
                threads: Some(3),
            }
        );
        // an env Grace cannot take is no env, and pom.xml's is not read
        fs::write(dir.path().join("karate-pom.json"), r#"{ "env": "bad name" }"#).unwrap();
        assert_eq!(pom_start(dir.path(), &PomXml).env, None);
    }
}
