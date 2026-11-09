//! The draft's rules from `PropertiesDialog.test.tsx`: what the dialog lists, what blocks Save and what Save sends.
//! The views' own tests (focus, the look, the dialog's life) are `grace::properties::tests`.
use super::*;
use crate::import_plan::import_plan;
use crate::model::env::{ImportEntry, PlainRow, PropertyListsView, SecretRowView};
use indexmap::IndexMap;

fn plain(name: &str, value: &str) -> RowView {
    RowView::Plain(PlainRow {
        name: name.into(),
        value: value.into(),
        env_var: None,
    })
}

fn secret(name: &str, state: SecretState) -> RowView {
    RowView::Secret(SecretRowView {
        name: name.into(),
        secret: true,
        state,
        env_var: None,
    })
}

fn view(env: Option<&str>, shared: Vec<RowView>, by_env: Vec<(&str, Vec<RowView>)>) -> EnvView {
    EnvView {
        env: env.map(Into::into),
        suggestions: vec![],
        overlays: vec![],
        properties: PropertyListsView {
            shared,
            by_env: by_env.into_iter().map(|(e, r)| (e.to_string(), r)).collect(),
        },
        secrets_available: true,
    }
}

/// `saved` of the TypeScript: dev chosen, lists qa and prod.
fn saved() -> EnvView {
    view(
        Some("dev"),
        vec![plain("timeout", "5000")],
        vec![
            (
                "qa",
                vec![plain("timeout", "9000"), secret("api.token", SecretState::Set)],
            ),
            ("prod", vec![]),
        ],
    )
}

fn names(d: &Draft) -> Vec<String> {
    d.visible().map(|r| r.name.clone()).collect()
}

fn ids(d: &Draft) -> Vec<u64> {
    d.visible().map(|r| r.id).collect()
}

/// Adds a row to the list shown, named and valued; its id.
fn add(d: &mut Draft, name: &str, value: &str) -> u64 {
    let id = d.add_row().unwrap();
    d.set_name(id, name);
    d.set_value(id, value);
    id
}

fn error(d: &Draft, id: u64) -> Option<String> {
    d.errors().remove(&id)
}

fn value(name: &str, value: &str, secret: bool, env_var: bool) -> RowEdit {
    RowEdit::Value {
        name: name.into(),
        value: value.into(),
        secret,
        env_var: env_var.then_some(true),
    }
}

fn keep(name: &str, env_var: bool) -> RowEdit {
    RowEdit::Keep {
        name: name.into(),
        env_var: env_var.then_some(true),
    }
}

fn by_env(lists: Vec<(&str, Vec<RowEdit>)>) -> IndexMap<String, Vec<RowEdit>> {
    lists.into_iter().map(|(e, r)| (e.to_string(), r)).collect()
}

#[test]
fn lists_scopes_and_rows() {
    let mut d = Draft::new(&saved());
    // All environments, every environment with a list, then the current one if it has none
    assert_eq!(d.scopes(), ["", "qa", "prod", "dev"]);
    assert_eq!(d.scope, "");
    assert_eq!(names(&d), ["timeout"]);
    assert!(d.errors().is_empty());
    d.scope = "qa".into();
    assert_eq!(names(&d), ["timeout", "api.token"]);
    let token = d.visible().nth(1).unwrap();
    assert_eq!((token.saved, token.value.as_str()), (Some(SecretState::Set), ""));
    d.scope = "prod".into();
    assert_eq!(names(&d), Vec::<String>::new());
    d.scope = "dev".into();
    let id = d.add_row().unwrap();
    assert_eq!(error(&d, id).as_deref(), Some("Enter a name"));
    d.delete_row(id);
    assert!(d.visible().next().is_none());
    assert!(d.errors().is_empty());
}

#[test]
fn adds_and_deletes_environment_lists() {
    let mut d = Draft::new(&saved());
    assert!(!d.can_delete_env(), "All environments");
    d.scope = "dev".into();
    assert!(!d.can_delete_env(), "no list yet");
    assert_eq!(d.add_env("not valid!"), Err(EnvError::Invalid));
    assert!(EnvError::Invalid.to_string().contains("letters, digits"));
    assert_eq!(d.add_env(" staging "), Ok(()));
    assert_eq!(d.scopes(), ["", "qa", "prod", "staging", "dev"]);
    assert_eq!(d.scope, "staging");
    add(&mut d, "a.b", "v");
    // naming an environment that has a list just selects it
    assert_eq!(d.add_env("qa"), Ok(()));
    assert_eq!((d.scope.as_str(), names(&d).len()), ("qa", 2));
    d.scope = "prod".into();
    assert!(d.can_delete_env());
    d.delete_env();
    assert_eq!(d.scopes(), ["", "qa", "staging", "dev"]);
    assert_eq!(d.scope, "");
    assert_eq!(
        d.edit(),
        PropertiesEdit {
            shared: vec![value("timeout", "5000", false, false)],
            by_env: by_env(vec![
                (
                    "qa",
                    vec![value("timeout", "9000", false, false), keep("api.token", false)]
                ),
                ("staging", vec![value("a.b", "v", false, false)]),
            ]),
        }
    );
}

#[test]
fn rows_added_under_the_current_environment_which_has_no_list_yet_are_saved() {
    let mut d = Draft::new(&saved());
    d.scope = "dev".into();
    add(&mut d, "dev.only", "1");
    assert!(d.can_delete_env());
    let edit = d.edit();
    assert_eq!(edit.by_env["dev"], [value("dev.only", "1", false, false)]);
    assert_eq!(edit.by_env.keys().collect::<Vec<_>>(), ["qa", "prod", "dev"]);
}

#[test]
fn an_invalid_row_under_the_current_environment_marks_its_scope() {
    let mut d = Draft::new(&saved());
    d.scope = "dev".into();
    let id = add(&mut d, "karate.env", "v");
    assert_eq!(error(&d, id).as_deref(), Some("Reserved name"));
    assert!(d.invalid_scopes().contains("dev"));
    d.delete_row(id);
    assert!(d.errors().is_empty());
}

#[test]
fn a_list_of_more_than_500_rows_is_an_error() {
    let many = (0..501).map(|i| plain(&format!("p{i}"), "v")).collect();
    let mut d = Draft::new(&view(None, many, vec![]));
    let rows = ids(&d);
    assert_eq!(error(&d, rows[499]), None);
    assert_eq!(error(&d, rows[500]).as_deref(), Some("At most 500 properties per list"));
    assert!(d.invalid_scopes().contains(""));
    d.delete_row(rows[0]);
    assert!(d.errors().is_empty());
}

#[test]
fn a_101st_environment_is_refused() {
    let lists = (0..100).map(|i| (format!("e{i}"), vec![])).collect::<Vec<_>>();
    let mut d = Draft::new(&view(
        Some("dev"),
        vec![],
        lists.iter().map(|(e, r)| (e.as_str(), r.clone())).collect(),
    ));
    assert_eq!(d.add_env("e100"), Err(EnvError::TooMany));
    assert_eq!(EnvError::TooMany.to_string(), "At most 100 environments");
    assert_eq!(d.scopes().len(), 102); // All environments, e0-e99, dev
    // nor by the first row of the current environment, which has no list yet
    d.scope = "dev".into();
    assert_eq!(d.add_row(), Err(EnvError::TooMany));
    assert!(d.visible().next().is_none());
    // an environment that has a list can still be chosen
    assert_eq!(d.add_env("e5"), Ok(()));
    assert_eq!(d.scope, "e5");
}

#[test]
fn replace_asks_for_a_new_value_of_a_saved_secret() {
    let mut d = Draft::new(&saved());
    d.scope = "qa".into();
    let token = ids(&d)[1];
    d.replace(token);
    let r = d.row(token).unwrap();
    assert_eq!((r.saved, r.secret, r.value.as_str()), (None, true, ""));
    assert_eq!(error(&d, token).as_deref(), Some("Enter the secret value"));
}

#[test]
fn an_unreadable_secret_can_be_replaced() {
    let mut d = Draft::new(&view(
        Some("qa"),
        vec![],
        vec![("qa", vec![secret("t", SecretState::Unreadable)])],
    ));
    d.scope = "qa".into();
    let t = ids(&d)[0];
    assert_eq!(d.row(t).unwrap().saved, Some(SecretState::Unreadable));
    assert_eq!(error(&d, t), None);
    d.replace(t);
    assert_eq!(d.row(t).unwrap().saved, None);
}

#[test]
fn a_reserved_name_is_an_error_in_any_letter_case() {
    let mut d = Draft::new(&saved());
    let id = add(&mut d, "karate.options", "-t @x");
    assert_eq!(error(&d, id).as_deref(), Some("Reserved name"));
    d.set_name(id, "KARATE.ENV");
    assert_eq!(error(&d, id).as_deref(), Some("Reserved name"));
    d.set_name(id, "my.own");
    assert_eq!(error(&d, id), None);
}

#[test]
fn a_name_that_repeats_within_a_list_is_an_error_in_another_list_it_is_not() {
    let mut d = Draft::new(&saved());
    let dup = add(&mut d, "timeout", "v"); // shared already has timeout
    let first = ids(&d)[0];
    assert_eq!(error(&d, first).as_deref(), Some("Duplicate name"));
    assert_eq!(error(&d, dup).as_deref(), Some("Duplicate name"));
    assert_eq!(d.invalid_scopes(), HashSet::from([String::new()]));
    d.delete_row(dup);
    assert!(d.invalid_scopes().is_empty());
}

#[test]
fn a_value_breaks_the_value_rules_with_their_words() {
    let mut d = Draft::new(&saved());
    let id = add(&mut d, "greeting", "h\u{e9}llo");
    assert_eq!(
        error(&d, id).as_deref(),
        Some("This value cannot be passed to Java exactly.")
    );
    d.set_value(id, "hello");
    assert_eq!(error(&d, id), None);
    d.set_value(id, &"x".repeat(10_001));
    assert_eq!(error(&d, id).as_deref(), Some("Too long"));
}

#[test]
fn save_sends_keep_for_an_untouched_secret_and_the_new_value_after_replace() {
    let mut d = Draft::new(&saved());
    let qa = || {
        vec![
            (
                "qa",
                vec![value("timeout", "9000", false, false), keep("api.token", false)],
            ),
            ("prod", vec![]),
        ]
    };
    assert_eq!(
        d.edit(),
        PropertiesEdit {
            shared: vec![value("timeout", "5000", false, false)],
            by_env: by_env(qa()),
        }
    );
    d.scope = "qa".into();
    let token = ids(&d)[1];
    d.replace(token);
    d.set_value(token, "s3cret-token-123");
    assert!(d.errors().is_empty());
    assert_eq!(
        d.edit().by_env["qa"][1],
        value("api.token", "s3cret-token-123", true, false)
    );
}

#[test]
fn renaming_or_unticking_a_saved_secret_clears_it_and_asks_for_a_value() {
    let mut d = Draft::new(&saved());
    d.scope = "qa".into();
    let token = ids(&d)[1];
    d.set_name(token, "api.token2");
    assert_eq!(d.row(token).unwrap().saved, None);
    assert_eq!(error(&d, token).as_deref(), Some("Enter the secret value"));
    d.set_value(token, "abcdef");
    assert!(d.errors().is_empty());

    let mut d = Draft::new(&saved());
    d.scope = "qa".into();
    let token = ids(&d)[1];
    d.set_secret(token, false);
    let r = d.row(token).unwrap();
    assert_eq!((r.secret, r.saved, r.value.as_str()), (false, None, ""));
    assert_eq!(error(&d, token), None);
    d.set_value(token, "plain");
    assert_eq!(d.edit().by_env["qa"][1], value("api.token", "plain", false, false));
}

#[test]
fn env_variable_picks_the_name_rule_and_save_sends_it() {
    let mut v = saved();
    v.properties = PropertyListsView {
        shared: vec![RowView::Plain(PlainRow {
            name: "LOG_LEVEL".into(),
            value: "debug".into(),
            env_var: Some(true),
        })],
        by_env: IndexMap::new(),
    };
    let mut d = Draft::new(&v);
    assert!(d.visible().next().unwrap().env_var);
    let id = add(&mut d, "db.user", "v");
    assert_eq!(error(&d, id), None);
    d.set_env_var(id, true);
    assert_eq!(error(&d, id).as_deref(), Some("Invalid name"));
    d.set_name(id, "DB_USER");
    assert_eq!(error(&d, id), None);
    assert_eq!(
        d.edit(),
        PropertiesEdit {
            shared: vec![
                value("LOG_LEVEL", "debug", false, true),
                value("DB_USER", "v", false, true)
            ],
            by_env: IndexMap::new(),
        }
    );
}

#[test]
fn two_variables_that_differ_only_in_case_are_duplicates_a_property_beside_them_is_not() {
    let mut d = Draft::new(&view(Some("dev"), vec![], vec![]));
    let rows: Vec<u64> = ["App_Token", "APP_TOKEN", "app_token"]
        .iter()
        .map(|n| add(&mut d, n, "v"))
        .collect();
    d.set_env_var(rows[0], true);
    d.set_env_var(rows[1], true);
    assert_eq!(error(&d, rows[0]).as_deref(), Some("Duplicate name"));
    assert_eq!(error(&d, rows[1]).as_deref(), Some("Duplicate name"));
    assert_eq!(error(&d, rows[2]), None);
}

#[test]
fn ticking_env_variable_on_a_saved_secret_keeps_it() {
    let mut d = Draft::new(&view(Some("dev"), vec![secret("API_TOKEN", SecretState::Set)], vec![]));
    let id = ids(&d)[0];
    d.set_env_var(id, true);
    assert_eq!(d.row(id).unwrap().saved, Some(SecretState::Set));
    assert_eq!(error(&d, id), None);
    assert_eq!(d.edit().shared, [keep("API_TOKEN", true)]);
}

#[test]
fn a_secret_shorter_than_4_characters_is_short() {
    let mut d = Draft::new(&saved());
    let id = add(&mut d, "p", "abc");
    d.set_secret(id, true);
    assert!(d.row(id).unwrap().short_secret());
    assert!(d.has_secret());
    d.set_value(id, "abcd");
    assert!(!d.row(id).unwrap().short_secret());
}

fn entries(pairs: &[(&str, &str)]) -> Vec<ImportEntry> {
    pairs
        .iter()
        .map(|(k, v)| ImportEntry {
            key: (*k).into(),
            value: (*v).into(),
        })
        .collect()
}

#[test]
fn import_puts_secret_variables_in_the_draft_replacing_in_place() {
    let mut d = Draft::new(&view(
        Some("dev"),
        vec![plain("APP_DB_USER", "old"), plain("keep.me", "1")],
        vec![],
    ));
    let first = ids(&d)[0];
    let plan = import_plan(
        &entries(&[("db.user", "u"), ("db.password", "p")]),
        true,
        "APP_",
        &d.existing(""),
    );
    assert_eq!(plan.replaces, ["APP_DB_USER"]);
    d.import("", true, &plan.rows).unwrap();
    assert_eq!(ids(&d)[0], first, "replaced in place");
    assert_eq!(
        d.edit().shared,
        [
            value("APP_DB_USER", "u", true, true),
            value("keep.me", "1", false, false),
            value("APP_DB_PASSWORD", "p", true, true),
        ]
    );
}

#[test]
fn imports_into_the_chosen_environments_new_list() {
    let mut d = Draft::new(&saved());
    let plan = import_plan(&entries(&[("token", "t0k3n")]), true, "", &d.existing("dev"));
    d.import("dev", true, &plan.rows).unwrap();
    assert_eq!(d.scope, "dev");
    assert_eq!(d.lists(), ["qa", "prod", "dev"]);
    assert_eq!(d.edit().by_env["dev"], [value("TOKEN", "t0k3n", true, true)]);
}

#[test]
fn import_is_refused_into_a_101st_list() {
    let lists = (0..100).map(|i| (format!("e{i}"), vec![])).collect::<Vec<_>>();
    let mut d = Draft::new(&view(
        Some("dev"),
        vec![],
        lists.iter().map(|(e, r)| (e.as_str(), r.clone())).collect(),
    ));
    assert!(!d.can_add_list("dev"));
    assert!(d.can_add_list("") && d.can_add_list("e1"));
    let plan = import_plan(&entries(&[("a", "b")]), true, "", &[]);
    assert_eq!(d.import("dev", true, &plan.rows), Err(EnvError::TooMany));
}
