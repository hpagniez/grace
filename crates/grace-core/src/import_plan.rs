//! `importPlan.ts`: what importing a file's keys into a list of the Properties dialog does (spec
//! `2026-11-04-env-var-properties.md` R5). Each key is named by its kind, checked by its rules, and matched with the
//! row it replaces. It never holds a skipped key's value: the file's values are secrets.
use std::collections::{HashMap, HashSet};

use crate::model::env::{ImportEntry, env_var_from_key, name_error, name_keys, property_value_error};

/// The reason of a key with no value. Such keys are counted, not listed: in a file that is no `.properties` file each
/// line reads as a key with no value, and that key may be a token.
pub const EMPTY: &str = "Empty value";

/// A row the import adds or replaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanRow {
    pub key: String,
    pub name: String,
    pub value: String,
}

/// A key the import leaves out, and why. Never the value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skip {
    pub key: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImportPlan {
    /// The rows to add or replace, in file order.
    pub rows: Vec<PlanRow>,
    pub adds: usize,
    /// The new names that already have a row in the list.
    pub replaces: Vec<String>,
    pub skips: Vec<Skip>,
}

/// A row already in the list the keys go to: its name and kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Existing {
    pub name: String,
    pub env_var: bool,
}

/// What importing `entries` as environment variables (`env_var`, named with `prefix`) or as system properties into a
/// list holding `existing` does.
pub fn import_plan(entries: &[ImportEntry], env_var: bool, prefix: &str, existing: &[Existing]) -> ImportPlan {
    let taken: HashSet<String> = existing.iter().flat_map(|r| name_keys(&r.name, r.env_var)).collect();
    let mut earlier: HashMap<String, &str> = HashMap::new(); // name key -> the key that took it
    let mut plan = ImportPlan::default();
    for ImportEntry { key, value } in entries {
        let name = if env_var {
            env_var_from_key(prefix, key)
        } else {
            key.clone()
        };
        let keys = name_keys(&name, env_var);
        let first = keys.iter().find_map(|k| earlier.get(k).copied());
        let reason = name_error(&name, env_var)
            .map(|e| e.to_string())
            .or_else(|| {
                if value.is_empty() {
                    Some(EMPTY.to_string())
                } else {
                    property_value_error(value).map(|e| e.to_string())
                }
            })
            .or_else(|| first.map(|k| format!("Same name as {k}")));
        if let Some(reason) = reason {
            plan.skips.push(Skip {
                key: key.clone(),
                reason,
            });
            continue;
        }
        for k in &keys {
            earlier.insert(k.clone(), key);
        }
        if keys.iter().any(|k| taken.contains(k)) {
            plan.replaces.push(name.clone());
        } else {
            plan.adds += 1;
        }
        plan.rows.push(PlanRow {
            key: key.clone(),
            name,
            value: value.clone(),
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    //! `importPlan.test.ts`.
    use super::*;

    fn keys(pairs: &[(&str, &str)]) -> Vec<ImportEntry> {
        pairs
            .iter()
            .map(|(k, v)| ImportEntry {
                key: (*k).into(),
                value: (*v).into(),
            })
            .collect()
    }

    fn file() -> Vec<ImportEntry> {
        keys(&[("db.user", "u"), ("db.password", "p"), ("9lives", "x"), ("empty", "")])
    }

    fn existing() -> Vec<Existing> {
        vec![Existing {
            name: "APP_DB_USER".into(),
            env_var: true,
        }]
    }

    fn row(key: &str, name: &str, value: &str) -> PlanRow {
        PlanRow {
            key: key.into(),
            name: name.into(),
            value: value.into(),
        }
    }

    fn skip(key: &str, reason: &str) -> Skip {
        Skip {
            key: key.into(),
            reason: reason.into(),
        }
    }

    #[test]
    fn adds_replaces_and_skips_as_variables_with_no_prefix() {
        assert_eq!(
            import_plan(&file(), true, "", &existing()),
            ImportPlan {
                rows: vec![row("db.user", "DB_USER", "u"), row("db.password", "DB_PASSWORD", "p")],
                adds: 2,
                replaces: vec![],
                skips: vec![skip("9lives", "Invalid name"), skip("empty", "Empty value")],
            }
        );
    }

    #[test]
    fn a_prefix_names_the_variables_and_finds_the_row_they_replace() {
        assert_eq!(
            import_plan(&file(), true, "APP_", &existing()),
            ImportPlan {
                rows: vec![
                    row("db.user", "APP_DB_USER", "u"),
                    row("db.password", "APP_DB_PASSWORD", "p"),
                    row("9lives", "APP_9LIVES", "x"),
                ],
                adds: 2,
                replaces: vec!["APP_DB_USER".into()],
                skips: vec![skip("empty", "Empty value")],
            }
        );
    }

    #[test]
    fn replaces_a_variable_that_differs_only_in_case_not_a_property() {
        let one = keys(&[("db.user", "u")]);
        let plan = |name: &str, env_var: bool| {
            import_plan(
                &one,
                true,
                "APP_",
                &[Existing {
                    name: name.into(),
                    env_var,
                }],
            )
            .replaces
        };
        assert_eq!(plan("App_Db_User", true), ["APP_DB_USER"]);
        assert!(plan("app_db_user", false).is_empty());
    }

    #[test]
    fn skips_a_key_that_maps_to_the_name_of_an_earlier_one() {
        let plan = import_plan(&keys(&[("db.user", "a"), ("db_user", "b")]), true, "APP_", &[]);
        assert_eq!(plan.rows, [row("db.user", "APP_DB_USER", "a")]);
        assert_eq!(plan.skips, [skip("db_user", "Same name as db.user")]);
    }

    #[test]
    fn as_system_properties_a_key_keeps_its_name_and_their_rule_applies() {
        let plan = import_plan(&file(), false, "IGNORED_", &existing());
        let names: Vec<_> = plan.rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["db.user", "db.password"]);
        assert_eq!(plan.skips[0], skip("9lives", "Invalid name"));
        assert_eq!(
            import_plan(&keys(&[("java.home", "x")]), false, "", &[]).skips,
            [skip("java.home", "Reserved name")]
        );
    }

    #[test]
    fn skips_a_value_the_value_rules_refuse_and_a_reserved_variable() {
        let plan = import_plan(
            &keys(&[("greeting", "héllo"), ("java_tool_options", "x")]),
            true,
            "",
            &[],
        );
        assert_eq!(
            plan.skips,
            [
                skip("greeting", "This value cannot be passed to Java exactly."),
                skip("java_tool_options", "Reserved name"),
            ]
        );
    }
}
