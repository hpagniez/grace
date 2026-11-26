//! The starting values a project's pom files give (`pom_start`), and the part of opening a project that phase P4 has
//! still to port: the sub-folder offer (`karate/subfolders.ts`). The runner calls it where `runner.ts` does, through
//! `ProjectSetup`; until it is ported, `NoSetup` has nothing to say.
use crate::karate::maven::maven_env;
use crate::karate::pom::{PomEnv, read_pom};
use crate::model::tags::FilterState;
use std::path::Path;

/// What phase P4 plugs into the runner. Every method has the answer of a project that needs none of it.
pub trait ProjectSetup: Send + Sync {
    /// `karateFolders`: folders below the project that look like a Karate project, offered when the tree has no test
    /// (sub-folder offer R4).
    fn karate_folders(&self, _root: &Path) -> Vec<String> {
        Vec::new()
    }
}

/// Phase P1's setup: no sub-folder offer.
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

/// `pomStart`: karate-pom.json's values, and pom.xml's `karate.env` when karate-pom.json gives no `env`.
pub fn pom_start(root: &Path) -> PomStart {
    let pom = read_pom(root);
    PomStart {
        env: match pom.env {
            PomEnv::NotGiven => maven_env(root),
            PomEnv::Unused => None,
            PomEnv::Given(env) => Some(env),
        },
        filter: pom.filter,
        threads: pom.threads,
    }
}

#[cfg(test)]
mod tests {
    /// `pom.test.ts`'s `pomStart`.
    mod pom_start {
        use super::super::*;
        use std::fs;
        use tempfile::TempDir;

        const MAVEN_DEV: &str = "<project><properties><karate.env>dev</karate.env></properties></project>";

        /// A project folder with this `karate-pom.json` and `pom.xml`, or without.
        fn project(pom: Option<&str>, pom_xml: Option<&str>) -> TempDir {
            let d = tempfile::Builder::new().prefix("kk-pom-").tempdir().unwrap();
            for (name, text) in [("karate-pom.json", pom), ("pom.xml", pom_xml)] {
                if let Some(text) = text {
                    fs::write(d.path().join(name), text).unwrap();
                }
            }
            d
        }

        fn start(pom: Option<&str>, pom_xml: Option<&str>) -> PomStart {
            pom_start(project(pom, pom_xml).path())
        }

        #[test]
        fn takes_pom_xmls_karate_env_without_a_karate_pom_json() {
            assert_eq!(
                start(None, Some(MAVEN_DEV)),
                PomStart {
                    env: Some("dev".into()),
                    filter: None,
                    threads: None,
                }
            );
        }

        #[test]
        fn puts_karate_pom_jsons_env_first() {
            assert_eq!(
                start(Some(r#"{"env":"qa"}"#), Some(MAVEN_DEV)).env.as_deref(),
                Some("qa")
            );
        }

        #[test]
        fn keeps_pom_xml_out_when_karate_pom_jsons_env_is_unused() {
            assert_eq!(start(Some(r#"{"env":"x y"}"#), Some(MAVEN_DEV)).env, None);
        }

        #[test]
        fn reads_a_karate_pom_json_that_cannot_be_read_as_absent() {
            assert_eq!(start(Some("not json"), Some(MAVEN_DEV)).env.as_deref(), Some("dev"));
        }

        #[test]
        fn takes_the_filter_and_the_thread_count() {
            assert_eq!(
                start(Some(r#"{"tags":["~@slow"],"threads":2}"#), None),
                PomStart {
                    env: None,
                    filter: Some(FilterState {
                        include: vec![],
                        exclude: vec!["slow".into()],
                        hide_non_matching: false,
                    }),
                    threads: Some(2),
                }
            );
        }
    }
}
