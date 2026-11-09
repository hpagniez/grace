//! `src/main/karate/pom.ts`: what Grace takes from a project's `karate-pom.json` (docs/specs/2026-11-02-project-setup.md
//! R1, docs/specs/2026-11-04-pom-defaults.md R2). `command::detect_layout` reads it for the feature roots, the config
//! folder and the open notice.
//!
//! Only `readPom` ports in phase P1, with its tests. `pomStart`, the starting values of a project, also reads pom.xml's
//! `karate.env` through `maven.ts`'s `mavenEnv`; both port in phase P4 with the rest of project setup, and its five
//! tests with them.
use super::js;
use crate::model::env::valid_env_name;
use crate::model::tags::{FilterState, filterable, parse_filter_state};
use crate::model::threads::parse_threads;
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// karate-pom.json's `env` (docs/specs/2026-11-04-pom-defaults.md R2.1).
// Rust-native: three variants in place of `string | null | undefined`. Parity: `PomEnv::NotGiven` is `undefined`,
// `Unused` is `null`; `takes_env_tags_and_threads`, `names_an_env_or_a_thread_count_grace_cannot_take` and
// `reads_an_empty_env_as_none_given` check each.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PomEnv {
    /// The file gives none (or an empty one), so pom.xml's may.
    NotGiven,
    /// Given, but not a name Grace takes: named in `unused`.
    Unused,
    /// The environment the project starts with.
    Given(String),
}

/// What Grace takes from a project's `karate-pom.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pom {
    /// As written; None when absent or empty.
    pub paths: Option<Vec<String>>,
    /// The `-g` value, as written.
    pub config_dir: Option<String>,
    /// `output.junitXml`: Karate also writes JUnit XML (docs/specs/2026-11-03-export-reports.md R1).
    pub junit_xml: bool,
    pub env: PomEnv,
    /// The `tags` entries the filter can show (R2.3); None when none. Electron Grace's `TagFilter`, so
    /// `hide_non_matching` is always false.
    pub filter: Option<FilterState>,
    /// `threads` when Grace can take it (R2.2).
    pub threads: Option<u32>,
    /// The other keys, in the file's order, `output`'s as `output.<key>`: Grace does not use them; an unused `env`,
    /// `tags` entry or `threads` as `<key> <JSON>`.
    pub unused: Vec<String>,
    /// Why the file could not be read; the other fields are then empty.
    pub error: Option<String>,
}

pub const NO_POM: Pom = Pom {
    paths: None,
    config_dir: None,
    junit_xml: false,
    env: PomEnv::NotGiven,
    filter: None,
    threads: None,
    unused: Vec::new(),
    error: None,
};

fn broken(error: impl Into<String>) -> Pom {
    Pom {
        error: Some(error.into()),
        ..NO_POM
    }
}

/// The value as a list of strings, if it is one.
fn strings(v: &Value) -> Option<Vec<&str>> {
    v.as_array()?.iter().map(Value::as_str).collect()
}

/// `<root>/karate-pom.json`, or `NO_POM` when there is none. Never fails.
///
/// The file is read as UTF-8 with a bad byte replaced, as Node's `"utf8"` does. When it cannot be read or parsed,
/// `error` is the system's or `serde_json`'s message, which reads differently from Node's and V8's.
pub fn read_pom(root: &Path) -> Pom {
    let text = match fs::read(root.join("karate-pom.json")) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return NO_POM,
        Err(err) => return broken(err.to_string()),
    };
    let json: Value = match serde_json::from_str(&text) {
        Ok(json) => json,
        Err(err) => return broken(err.to_string()),
    };
    let Value::Object(json) = json else {
        return broken("not a JSON object");
    };
    let paths = match json.get("paths") {
        None => None,
        Some(v) => match strings(v) {
            Some(paths) => Some(paths),
            None => return broken("paths is not a list of strings"),
        },
    };
    let config_dir = match json.get("configDir") {
        None => None,
        Some(Value::String(dir)) => Some(dir.clone()),
        Some(_) => return broken("configDir is not a string"),
    };
    // an output that is not an object turns nothing on and is named whole
    let output = json.get("output").and_then(Value::as_object);
    let junit_xml = match output.and_then(|o| o.get("junitXml")) {
        // `output?.junitXml ?? false`: absent and `null` are both off
        None | Some(Value::Null) => false,
        Some(Value::Bool(on)) => *on,
        Some(_) => return broken("output.junitXml is not true or false"),
    };
    // `json.env === "" ? undefined : json.env`
    let env = json.get("env").filter(|e| e.as_str() != Some(""));
    let used_env = env.and_then(Value::as_str).filter(|e| valid_env_name(e));
    let threads = json.get("threads").and_then(parse_threads);
    let tags = read_tags(json.get("tags"));
    let value = |key: &str, v: &Value| format!("{key} {}", js::json_stringify(v));
    // the keys Grace takes a starting value from (`env`, `threads`, `tags`) are named only with the values it cannot
    // take (R2.4); `output`'s keys but `junitXml` are named one by one when it is an object
    let named = |key: &str, v: &Value| -> Vec<String> {
        match (key, output) {
            ("output", Some(output)) => keys(output)
                .filter(|o| *o != "junitXml")
                .map(|o| format!("output.{o}"))
                .collect(),
            ("env", _) => match env {
                Some(env) if used_env.is_none() => vec![value("env", env)],
                _ => vec![],
            },
            ("threads", _) if threads.is_some() => vec![],
            ("threads", _) => vec![value("threads", v)],
            ("tags", _) => tags.unused.iter().map(|t| value("tags", t)).collect(),
            _ => vec![key.to_string()],
        }
    };
    let unused = js::entries(&json)
        .into_iter()
        .filter(|(k, _)| *k != "paths" && *k != "configDir")
        .flat_map(|(k, v)| named(k, v))
        .collect();
    Pom {
        paths: paths
            .filter(|p| !p.is_empty())
            .map(|p| p.into_iter().map(String::from).collect()),
        config_dir,
        junit_xml,
        env: match (env, used_env) {
            (None, _) => PomEnv::NotGiven,
            (Some(_), Some(name)) => PomEnv::Given(name.to_string()),
            (Some(_), None) => PomEnv::Unused,
        },
        filter: tags.filter,
        threads,
        unused,
        error: None,
    }
}

/// An object's keys in the order `Object.keys` gives them.
fn keys(map: &Map<String, Value>) -> impl Iterator<Item = &str> {
    js::entries(map).into_iter().map(|(k, _)| k.as_str())
}

struct Tags {
    filter: Option<FilterState>,
    unused: Vec<Value>,
}

/// R2.3: `~@t` excludes, the first `@a,@b` entry includes; every other entry is unused, as written.
fn read_tags(tags: Option<&Value>) -> Tags {
    let Some(tags) = tags else {
        return Tags {
            filter: None,
            unused: vec![],
        };
    };
    let Some(entries) = strings(tags) else {
        return Tags {
            filter: None,
            unused: vec![tags.clone()],
        };
    };
    let mut include: Vec<String> = Vec::new();
    let mut exclude: Vec<String> = Vec::new();
    let mut unused: Vec<Value> = Vec::new();
    for entry in entries {
        let parts: Vec<&str> = entry.split(',').map(js::trim).collect();
        let excludes = parts.len() == 1 && parts[0].starts_with("~@");
        // the names, once the entry is of its kind: `~@` or `@` cut off
        let names: Option<Vec<&str>> = if excludes {
            Some(vec![&parts[0][2..]])
        } else if include.is_empty() {
            parts.iter().map(|p| p.strip_prefix('@')).collect()
        } else {
            None
        };
        let (out, other) = if excludes {
            (&mut exclude, &include)
        } else {
            (&mut include, &exclude)
        };
        let ok = names.filter(|names| names.iter().all(|t| filterable(t) && !other.iter().any(|o| o == t)));
        match ok {
            Some(names) => {
                for (i, t) in names.iter().enumerate() {
                    if !out.iter().any(|o| o == t) && names.iter().position(|n| n == t) == Some(i) {
                        out.push(t.to_string());
                    }
                }
            }
            None => unused.push(Value::String(entry.to_string())),
        }
    }
    if parse_filter_state(&json!({ "include": include, "exclude": exclude, "hideNonMatching": false })).is_none() {
        return Tags {
            filter: None,
            unused: vec![tags.clone()],
        };
    }
    let filter = (!include.is_empty() || !exclude.is_empty()).then_some(FilterState {
        include,
        exclude,
        hide_non_matching: false,
    });
    Tags { filter, unused }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A project folder with this `karate-pom.json`, or none. Phase P4 adds the TypeScript's second argument, a
    /// `pom.xml`, with `pomStart`.
    fn project(pom: Option<&str>) -> TempDir {
        let d = tempfile::Builder::new().prefix("kk-pom-").tempdir().unwrap();
        if let Some(pom) = pom {
            fs::write(d.path().join("karate-pom.json"), pom).unwrap();
        }
        d
    }

    fn read(pom: &str) -> Pom {
        read_pom(project(Some(pom)).path())
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn filter(include: &[&str], exclude: &[&str]) -> FilterState {
        FilterState {
            include: strings(include),
            exclude: strings(exclude),
            hide_non_matching: false,
        }
    }

    mod read_pom {
        use super::*;

        #[test]
        fn is_empty_without_a_file() {
            assert_eq!(read_pom(project(None).path()), NO_POM);
        }

        #[test]
        fn keeps_paths_and_config_dir_as_written_and_lists_the_other_keys_in_file_order() {
            let pom = r#"{"paths":["tests/a"],"configDir":"conf","env":"qa","dryRun":true}"#;
            assert_eq!(
                read(pom),
                Pom {
                    paths: Some(strings(&["tests/a"])),
                    config_dir: Some("conf".into()),
                    junit_xml: false,
                    env: PomEnv::Given("qa".into()),
                    filter: None,
                    threads: None,
                    unused: strings(&["dryRun"]),
                    error: None,
                }
            );
        }

        #[test]
        fn reads_an_empty_paths_list_as_none() {
            assert_eq!(read(r#"{"paths":[]}"#).paths, None);
        }

        #[test]
        fn gives_the_parsers_message_for_a_file_that_is_not_json() {
            let pom = read("{");
            assert_eq!((pom.paths, pom.config_dir, pom.unused), (None, None, vec![]));
            assert!(pom.error.is_some_and(|e| e.chars().any(|c| !c.is_whitespace())));
        }

        #[test]
        fn refuses_a_file_that_is_not_an_object() {
            for text in ["null", "[]", "3"] {
                assert_eq!(read(text), broken("not a JSON object"), "{text}");
            }
        }

        #[test]
        fn refuses_paths_that_are_not_a_list_of_strings() {
            for text in [r#"{"paths":"tests"}"#, r#"{"paths":[1]}"#] {
                assert_eq!(read(text), broken("paths is not a list of strings"), "{text}");
            }
        }

        #[test]
        fn refuses_a_config_dir_that_is_not_a_string() {
            assert_eq!(read(r#"{"configDir":3}"#), broken("configDir is not a string"));
        }

        #[test]
        fn takes_output_junit_xml_and_names_outputs_other_keys() {
            let pom = read(r#"{"env":"qa","output":{"dir":"x","junitXml":true,"html":false}}"#);
            assert!(pom.junit_xml);
            assert_eq!(pom.unused, ["output.dir", "output.html"]);
        }

        #[test]
        fn reads_junit_xml_false_or_absent_as_off() {
            for text in [r#"{"output":{"junitXml":false}}"#, r#"{"output":{}}"#] {
                let pom = read(text);
                assert!(!pom.junit_xml, "{text}");
                assert_eq!(pom.unused, [] as [String; 0], "{text}");
            }
        }

        #[test]
        fn names_an_output_that_is_not_an_object() {
            for text in [r#"{"output":"x"}"#, r#"{"output":[true]}"#, r#"{"output":null}"#] {
                let pom = read(text);
                assert!(!pom.junit_xml, "{text}");
                assert_eq!(pom.unused, ["output"], "{text}");
            }
        }

        #[test]
        fn refuses_a_junit_xml_that_is_not_true_or_false() {
            assert_eq!(
                read(r#"{"output":{"junitXml":"yes"}}"#),
                broken("output.junitXml is not true or false")
            );
        }

        #[test]
        fn takes_env_tags_and_threads() {
            let pom = read(r#"{"env":"qa","tags":["@smoke","~@slow"],"threads":3}"#);
            assert_eq!(pom.env, PomEnv::Given("qa".into()));
            assert_eq!(pom.filter, Some(filter(&["smoke"], &["slow"])));
            assert_eq!(pom.threads, Some(3));
            assert_eq!(pom.unused, [] as [String; 0]);
        }

        /// One test per row, as the TypeScript's `it.each("shows the tag entries %j as chips")`.
        macro_rules! shows_as_chips {
            ($($name:ident: $tags:expr => $include:expr, $exclude:expr;)*) => {
                $(
                    #[test]
                    fn $name() {
                        let pom = read(&json!({ "tags": $tags }).to_string());
                        assert_eq!(pom.filter, Some(filter(&$include, &$exclude)));
                        assert_eq!(pom.unused, [] as [String; 0]);
                    }
                )*
            };
        }

        shows_as_chips! {
            shows_the_tag_entries_smoke_as_chips: ["@smoke"] => ["smoke"], [];
            shows_the_tag_entries_not_slow_as_chips: ["~@slow"] => [], ["slow"];
            shows_the_tag_entries_a_or_b_as_chips: ["@a,@b"] => ["a", "b"], [];
            shows_the_tag_entries_spaced_a_or_b_not_c_not_d_as_chips:
                [" @a , @b ", "~@c", "~@d"] => ["a", "b"], ["c", "d"];
        }

        /// One test per row, as the TypeScript's `it.each("names the tag entries of %j it cannot show")`.
        macro_rules! names_what_it_cannot_show {
            ($($name:ident: $tags:expr => $filter:expr, $unused:expr;)*) => {
                $(
                    #[test]
                    fn $name() {
                        let pom = read(&json!({ "tags": $tags }).to_string());
                        assert_eq!(pom.filter, $filter);
                        assert_eq!(pom.unused, $unused);
                    }
                )*
            };
        }

        names_what_it_cannot_show! {
            names_the_tag_entries_of_any_of_it_cannot_show:
                ["anyOf('@a','@b')"] => None, [r#"tags "anyOf('@a','@b')""#];
            names_the_tag_entries_of_a_second_include_it_cannot_show:
                ["@a", "@b"] => Some(filter(&["a"], &[])), [r#"tags "@b""#];
            names_the_tag_entries_of_two_excludes_in_one_it_cannot_show:
                ["~@a,~@b"] => None, [r#"tags "~@a,~@b""#];
            names_the_tag_entries_of_a_tag_both_included_and_excluded_it_cannot_show:
                ["@a", "~@a"] => Some(filter(&["a"], &[])), [r#"tags "~@a""#];
            names_the_tag_entries_of_a_string_it_cannot_show:
                "@a" => None, [r#"tags "@a""#];
        }

        #[test]
        fn names_an_env_or_a_thread_count_grace_cannot_take() {
            let pom = read(r#"{"env":"x y"}"#);
            assert_eq!(pom.env, PomEnv::Unused);
            assert_eq!(pom.unused, [r#"env "x y""#]);
            let pom = read(r#"{"threads":64}"#);
            assert_eq!(pom.threads, None);
            assert_eq!(pom.unused, ["threads 64"]);
            let pom = read(r#"{"threads":"4"}"#);
            assert_eq!(pom.threads, None);
            assert_eq!(pom.unused, [r#"threads "4""#]);
        }

        #[test]
        fn reads_an_empty_env_as_none_given() {
            let pom = read(r#"{"env":""}"#);
            assert_eq!(pom.env, PomEnv::NotGiven);
            assert_eq!(pom.unused, [] as [String; 0]);
        }
    }
}
