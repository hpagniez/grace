//! `src/shared/env.ts`: rules for the run environment and the run properties
//! (docs/specs/2026-10-25-environment-switcher.md, docs/specs/2026-11-04-env-var-properties.md).
use super::js;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fmt;

pub const MAX_VALUE_LENGTH: usize = 10_000;

pub const MAX_ROWS: usize = 500;
pub const MAX_ENVS: usize = 100;
const MAX_NAME_LENGTH: usize = 200;

/// System properties Karate 2.1.4 reads (found by scanning the bundled jar); a run property must not replace them.
pub const RESERVED_KARATE_KEYS: [&str; 6] = [
    "karate.env",
    "karate.options",
    "karate.config.dir",
    "karate.output.dir",
    "karate.debug.port",
    "karate.js.slotFrames",
];

/// The JVM's own properties a run property must not replace, beside `RESERVED_KARATE_KEYS`.
const RESERVED_JVM_KEYS: [&str; 8] = [
    "user.dir",
    "user.home",
    "file.encoding",
    "native.encoding",
    "stdout.encoding",
    "stderr.encoding",
    "line.separator",
    "path.separator",
];
const RESERVED_PREFIXES: [&str; 3] = ["java.", "jdk.", "sun."];

/// The rule a name broke. Its `Display` is the text the properties dialog shows, as in Electron Grace.
///
/// Differs from the TypeScript, whose `propertyNameError` returns the text or `null`: the reason is a value to match on
/// and `None` is a valid name; `to_string()` gives the TypeScript's text, which `edit_errors` and its tests compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameError {
    /// Not the pattern of a name of its kind.
    Invalid,
    /// A name Karate, the JVM or Grace keeps for itself.
    Reserved,
}

impl fmt::Display for NameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameError::Invalid => "Invalid name",
            NameError::Reserved => "Reserved name",
        })
    }
}

/// The rule a value broke (`propertyValueError`); `Display` is Electron Grace's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueError {
    TooLong,
    LineBreaks,
    /// Not printable ASCII: Java reads the argument file in the Windows ANSI code page, and only printable ASCII is the
    /// same in all of them.
    CannotPass,
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ValueError::TooLong => "Too long",
            ValueError::LineBreaks => "No line breaks",
            ValueError::CannotPass => "This value cannot be passed to Java exactly.",
        })
    }
}

/// `ENV_NAME`, `/^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/`: an environment's name. Written as a check of bytes rather than
/// a regular expression: it only accepts ASCII, so a byte is a character.
pub fn valid_env_name(name: &str) -> bool {
    let b = name.as_bytes();
    (1..=64).contains(&b.len())
        && b[0].is_ascii_alphanumeric()
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

/// `PROPERTY_NAME`, `/^[A-Za-z_][A-Za-z0-9_.-]{0,199}$/`: the pattern of a system property's name, before the reserved
/// names.
pub fn valid_property_name(name: &str) -> bool {
    let b = name.as_bytes();
    (1..=200).contains(&b.len())
        && (b[0].is_ascii_alphabetic() || b[0] == b'_')
        && b[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
}

/// Variables Grace removes from Karate's environment, as they override the app's values; a run property must not set
/// them.
pub const STRIPPED_ENV_VARS: [&str; 6] = [
    "KARATE_OPTIONS",
    "KARATE_ENV",
    "KARATE_CONFIG_DIR",
    "JAVA_TOOL_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "_JAVA_OPTIONS",
];

/// `ENV_VAR_NAME`, `/^[A-Za-z_][A-Za-z0-9_]{0,199}$/`.
fn valid_env_var_name(name: &str) -> bool {
    let b = name.as_bytes();
    (1..=200).contains(&b.len())
        && (b[0].is_ascii_alphabetic() || b[0] == b'_')
        && b[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

pub fn env_var_name_error(name: &str) -> Option<NameError> {
    if !valid_env_var_name(name) {
        return Some(NameError::Invalid);
    }
    // the name is ASCII here, so an ASCII comparison is the TypeScript's `toUpperCase()` one
    STRIPPED_ENV_VARS
        .iter()
        .any(|v| v.eq_ignore_ascii_case(name))
        .then_some(NameError::Reserved)
}

/// The name rule of a row's kind: an environment variable's, or a system property's.
pub fn name_error(name: &str, env_var: bool) -> Option<NameError> {
    if env_var {
        env_var_name_error(name)
    } else {
        property_name_error(name)
    }
}

/// Two rows of a list clash when these intersect: the same name, or two environment variables that differ only in
/// letter case, which Windows sees as one.
pub fn name_keys(name: &str, env_var: bool) -> Vec<String> {
    if env_var {
        vec![name.to_string(), format!("\0{}", name.to_uppercase())]
    } else {
        vec![name.to_string()]
    }
}

/// An imported key's variable name: the prefix as typed, then the key in upper case, other characters as `_`.
///
/// The TypeScript's `replace(/[^A-Z0-9_]/g, "_")` works on UTF-16 units: a character above U+FFFF, two units, becomes
/// two `_`. So it does here.
pub fn env_var_from_key(prefix: &str, key: &str) -> String {
    let mut name = String::from(prefix);
    for c in key.to_uppercase().chars() {
        if matches!(c, 'A'..='Z' | '0'..='9' | '_') {
            name.push(c);
        } else {
            name.extend(std::iter::repeat_n('_', c.len_utf16()));
        }
    }
    name
}

/// `PREFIX`, `/^[A-Za-z0-9_]*$/`: what an import prefix may hold.
pub fn valid_prefix(prefix: &str) -> bool {
    prefix.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

pub fn property_name_error(name: &str) -> Option<NameError> {
    if !valid_property_name(name) {
        return Some(NameError::Invalid);
    }
    // the name is ASCII here, so the TypeScript's `toLowerCase()` comparisons are ASCII case-insensitive ones
    let reserved = RESERVED_KARATE_KEYS
        .iter()
        .chain(&RESERVED_JVM_KEYS)
        .any(|k| k.eq_ignore_ascii_case(name))
        || RESERVED_PREFIXES.iter().any(|p| {
            name.as_bytes()
                .get(..p.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(p.as_bytes()))
        });
    reserved.then_some(NameError::Reserved)
}

/// Java reads the argument file in the Windows ANSI code page; only printable ASCII is the same in all of them.
pub fn property_value_error(value: &str) -> Option<ValueError> {
    if js::utf16_len(value) > MAX_VALUE_LENGTH {
        return Some(ValueError::TooLong);
    }
    if value.contains(['\r', '\n']) {
        return Some(ValueError::LineBreaks);
    }
    value
        .chars()
        .any(|c| !(' '..='~').contains(&c))
        .then_some(ValueError::CannotPass)
}

/// A row with a name: what `effective_rows` needs of a plain row, a saved secret as the window sees it, or an edit.
pub trait Named {
    fn name(&self) -> &str;
}

/// A plain run property, or one set on the Karate process's environment (`env_var`) rather than passed as `-D`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlainRow {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_var: Option<bool>,
}

/// Whether a saved secret can be read back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretState {
    Set,
    Unreadable,
}

/// A saved secret as the window sees it: never its value. `secret` is always true.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecretRowView {
    pub name: String,
    pub secret: bool,
    pub state: SecretState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_var: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RowView {
    Secret(SecretRowView),
    Plain(PlainRow),
}

impl Named for PlainRow {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Named for SecretRowView {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Named for RowView {
    fn name(&self) -> &str {
        match self {
            RowView::Secret(r) => &r.name,
            RowView::Plain(r) => &r.name,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyListsView {
    pub shared: Vec<RowView>,
    pub by_env: IndexMap<String, Vec<RowView>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvView {
    pub env: Option<String>,
    pub suggestions: Vec<String>,
    /// Environments that have a karate-config-<env>.js overlay file.
    pub overlays: Vec<String>,
    pub properties: PropertyListsView,
    pub secrets_available: bool,
}

/// A row sent by the window: a new value, or `Keep` for a saved secret whose value stays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowEdit {
    Value {
        name: String,
        value: String,
        secret: bool,
        env_var: Option<bool>,
    },
    Keep {
        name: String,
        env_var: Option<bool>,
    },
}

impl RowEdit {
    /// Whether the row sets an environment variable rather than a system property (`envVar === true`).
    pub fn is_env_var(&self) -> bool {
        match self {
            RowEdit::Value { env_var, .. } | RowEdit::Keep { env_var, .. } => *env_var == Some(true),
        }
    }
}

impl Named for RowEdit {
    fn name(&self) -> &str {
        match self {
            RowEdit::Value { name, .. } | RowEdit::Keep { name, .. } => name,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PropertiesEdit {
    pub shared: Vec<RowEdit>,
    pub by_env: IndexMap<String, Vec<RowEdit>>,
}

/// One row of an edit. `None` is the TypeScript's `null`: anything but an object of exactly the shapes of `RowEdit`.
fn parse_row(x: &Value) -> Option<RowEdit> {
    let o = x.as_object()?;
    let name = o
        .get("name")?
        .as_str()
        .filter(|n| js::utf16_len(n) <= MAX_NAME_LENGTH)?
        .to_string();
    // `envVar` absent is `None`; present, it must be a boolean (`null` is refused, as `undefined` is the only absence)
    let env_var = match o.get("envVar") {
        None => None,
        Some(v) => Some(v.as_bool()?),
    };
    let secret = o.get("secret");
    if let Some(keep) = o.get("keep") {
        return (keep == &Value::Bool(true) && secret == Some(&Value::Bool(true)))
            .then_some(RowEdit::Keep { name, env_var });
    }
    let value = o
        .get("value")?
        .as_str()
        .filter(|v| js::utf16_len(v) <= MAX_VALUE_LENGTH)?
        .to_string();
    Some(RowEdit::Value {
        name,
        value,
        secret: secret?.as_bool()?,
        env_var,
    })
}

/// A list of at most `MAX_ROWS` rows. Every entry must parse: a hole in a JavaScript array (the TypeScript walks them
/// with `Array.from`, since `map` skips holes) is `null` once it is JSON, and `null` is no row.
fn parse_rows(x: &Value) -> Option<Vec<RowEdit>> {
    x.as_array()
        .filter(|a| a.len() <= MAX_ROWS)?
        .iter()
        .map(parse_row)
        .collect()
}

/// Validates the shape of untrusted input (the window); the rules of each row are `edit_errors`.
///
/// The environments come out in the order JavaScript lists an object's keys (`js::key_order`), as in Electron Grace.
pub fn parse_properties_edit(x: &Value) -> Option<PropertiesEdit> {
    let o = x.as_object()?;
    let by_env = o.get("byEnv")?.as_object()?;
    let shared = parse_rows(o.get("shared")?)?;
    if by_env.len() > MAX_ENVS || !by_env.keys().all(|e| valid_env_name(e)) {
        return None;
    }
    let mut out = PropertiesEdit {
        shared,
        by_env: IndexMap::new(),
    };
    for env in js::key_order(by_env.keys()) {
        out.by_env.insert(env.clone(), parse_rows(&by_env[env.as_str()])?);
    }
    Some(out)
}

/// Every broken rule of the edit, as `<scope>: <name>: <reason>`. Scope `None` is "All environments".
///
/// `stored_secrets(scope, name)` says whether a secret of that name is saved in that scope.
pub fn edit_errors(e: &PropertiesEdit, stored_secrets: impl Fn(Option<&str>, &str) -> bool) -> Vec<String> {
    let mut errors = Vec::new();
    let mut check = |scope: Option<&str>, rows: &[RowEdit]| {
        let mut seen: HashSet<String> = HashSet::new();
        for row in rows {
            let name = row.name();
            let env_var = row.is_env_var();
            let keys = name_keys(name, env_var);
            let mut report = |reason: &dyn fmt::Display| {
                errors.push(format!("{}: {}: {}", scope.unwrap_or("All environments"), name, reason));
            };
            if let Some(reason) = name_error(name, env_var) {
                report(&reason);
            }
            if keys.iter().any(|k| seen.contains(k)) {
                report(&"Duplicate name");
            }
            match row {
                RowEdit::Keep { .. } => {
                    if !stored_secrets(scope, name) {
                        report(&"No saved secret to keep");
                    }
                }
                RowEdit::Value { value, .. } => {
                    if let Some(reason) = property_value_error(value) {
                        report(&reason);
                    }
                }
            }
            seen.extend(keys);
        }
    };
    check(None, &e.shared);
    for (env, rows) in &e.by_env {
        check(Some(env.as_str()), rows);
    }
    errors
}

/// R4: environment rows win by name; shared order first, then rows only the environment has.
///
/// The rows are borrowed from the lists, where the TypeScript returns the same objects.
pub fn effective_rows<'a, T: Named>(
    shared: &'a [T],
    by_env: &'a IndexMap<String, Vec<T>>,
    env: Option<&str>,
) -> Vec<&'a T> {
    let own: &[T] = env.and_then(|e| by_env.get(e)).map_or(&[], Vec::as_slice);
    let wins: HashMap<&str, &T> = own.iter().map(|r| (r.name(), r)).collect();
    let in_shared: HashSet<&str> = shared.iter().map(Named::name).collect();
    shared
        .iter()
        .map(|r| wins.get(r.name()).copied().unwrap_or(r))
        .chain(own.iter().filter(|r| !in_shared.contains(r.name())))
        .collect()
}

/// Environment names in `env=` and `envnot=` tags (written without `@`), e.g. `env=dev,qa`.
///
/// The names are borrowed from the tags, where the TypeScript copies them.
pub fn env_tag_values(tags: &[String]) -> Vec<&str> {
    tags.iter()
        .filter_map(|t| t.strip_prefix("envnot=").or_else(|| t.strip_prefix("env=")))
        // the TypeScript's `(.*)` does not match a line terminator, so a tag with one is no env tag
        .filter(|v| !v.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
        .flat_map(|v| v.split(','))
        .filter(|n| valid_env_name(n))
        .collect()
}

/// The union of the lists, without repeats, sorted as JavaScript's `sort()` does (by UTF-16 unit).
pub fn sorted_unique(lists: &[Vec<String>]) -> Vec<String> {
    let mut all: Vec<String> = lists
        .iter()
        .flatten()
        .collect::<HashSet<_>>()
        .into_iter()
        .cloned()
        .collect();
    js::sort_utf16(&mut all);
    all
}

/// A key of an imported `.properties` file, with its value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportEntry {
    pub key: String,
    pub value: String,
}

/// The answer to Import from file…: the file's name and keys, or a message. Electron Grace's `null` (no file picked)
/// is `Option::None` around it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportResult {
    File { file: String, entries: Vec<ImportEntry> },
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CANNOT_PASS: &str = "This value cannot be passed to Java exactly.";

    fn plain(name: &str, value: &str) -> RowEdit {
        RowEdit::Value {
            name: name.into(),
            value: value.into(),
            secret: false,
            env_var: None,
        }
    }

    fn secret(name: &str, value: &str) -> RowEdit {
        RowEdit::Value {
            name: name.into(),
            value: value.into(),
            secret: true,
            env_var: None,
        }
    }

    fn keep(name: &str) -> RowEdit {
        RowEdit::Keep {
            name: name.into(),
            env_var: None,
        }
    }

    fn edit(shared: Vec<RowEdit>, by_env: Vec<(&str, Vec<RowEdit>)>) -> PropertiesEdit {
        PropertiesEdit {
            shared,
            by_env: by_env.into_iter().map(|(e, rows)| (e.to_string(), rows)).collect(),
        }
    }

    fn no_secrets(_: Option<&str>, _: &str) -> bool {
        false
    }

    /// The JSON of a row as the window sends it.
    fn row_json(name: &str, value: &str, secret: bool) -> Value {
        json!({ "name": name, "value": value, "secret": secret })
    }

    mod env_names {
        use super::*;

        #[test]
        fn accepts_ordinary_names() {
            for n in ["qa", "prod-2", "eu.west_1", "1", "A"] {
                assert!(valid_env_name(n), "{n}");
            }
            assert!(valid_env_name(&"a".repeat(64)));
        }

        #[test]
        fn rejects_empty_path_like_spaced_long_and_dash_leading_names() {
            for n in ["", "../x", "a b", &"a".repeat(65), "-x", ".x", "_x", "é"] {
                assert!(!valid_env_name(n), "{n}");
            }
        }
    }

    mod property_names {
        use super::*;

        #[test]
        fn reserves_karate_and_jvm_names_in_any_case() {
            for n in [
                "karate.env",
                "KARATE.OPTIONS",
                "java.home",
                "Sun.x",
                "user.dir",
                "JDK.x",
            ] {
                assert_eq!(property_name_error(n), Some(NameError::Reserved), "{n}");
                assert_eq!(property_name_error(n).unwrap().to_string(), "Reserved name");
            }
            for n in RESERVED_KARATE_KEYS {
                assert_eq!(property_name_error(&n.to_uppercase()), Some(NameError::Reserved), "{n}");
            }
        }

        #[test]
        fn knows_the_keys_karate_2_1_4_reads() {
            for k in [
                "karate.env",
                "karate.options",
                "karate.config.dir",
                "karate.output.dir",
                "karate.debug.port",
            ] {
                assert!(RESERVED_KARATE_KEYS.contains(&k), "{k}");
            }
        }

        #[test]
        fn allows_other_names_including_other_karate_ones() {
            for n in ["karate.mything", "timeout", "api.token", "_x", "a-b.c_d"] {
                assert_eq!(property_name_error(n), None, "{n}");
            }
        }

        #[test]
        fn rejects_names_that_do_not_match_the_pattern() {
            for n in ["", "1x", "a b", "a=b", &"a".repeat(201), "é"] {
                assert_eq!(property_name_error(n), Some(NameError::Invalid), "{n}");
                assert_eq!(property_name_error(n).unwrap().to_string(), "Invalid name");
            }
            assert_eq!(property_name_error(&"a".repeat(200)), None);
        }
    }

    mod value_rules {
        use super::*;

        #[test]
        fn rejects_too_long_values() {
            assert_eq!(property_value_error(&"x".repeat(MAX_VALUE_LENGTH)), None);
            let long = property_value_error(&"x".repeat(MAX_VALUE_LENGTH + 1));
            assert_eq!(long, Some(ValueError::TooLong));
            assert_eq!(long.unwrap().to_string(), "Too long");
        }

        #[test]
        fn rejects_line_breaks() {
            assert_eq!(property_value_error("a\nb"), Some(ValueError::LineBreaks));
            assert_eq!(property_value_error("a\r\nb"), Some(ValueError::LineBreaks));
            assert_eq!(ValueError::LineBreaks.to_string(), "No line breaks");
        }

        #[test]
        fn rejects_anything_that_is_not_printable_ascii_with_the_exact_copy() {
            for v in ["é", "\t", "a\u{0}b", "\u{7f}", "日本", "\u{2028}"] {
                assert_eq!(property_value_error(v), Some(ValueError::CannotPass), "{v:?}");
                assert_eq!(property_value_error(v).unwrap().to_string(), CANNOT_PASS);
            }
        }

        #[test]
        fn accepts_spaces_quotes_backslashes_and_percent_signs() {
            assert_eq!(property_value_error("x y \"q\" C:\\Users\\me %PATH%"), None);
            assert_eq!(property_value_error(""), None);
        }
    }

    mod effective_rows {
        use super::*;

        #[derive(Debug, PartialEq)]
        struct Row {
            name: &'static str,
            value: &'static str,
        }

        impl Named for Row {
            fn name(&self) -> &str {
                self.name
            }
        }

        fn row(name: &'static str, value: &'static str) -> Row {
            Row { name, value }
        }

        fn lists() -> (Vec<Row>, IndexMap<String, Vec<Row>>) {
            let shared = vec![row("timeout", "5000"), row("a", "1")];
            let by_env = IndexMap::from([("qa".to_string(), vec![row("timeout", "9000"), row("b", "2")])]);
            (shared, by_env)
        }

        #[test]
        fn environment_wins_by_name_shared_order_first_then_environment_only_rows() {
            let (shared, by_env) = lists();
            assert_eq!(
                effective_rows(&shared, &by_env, Some("qa")),
                [&row("timeout", "9000"), &row("a", "1"), &row("b", "2")]
            );
        }

        #[test]
        fn gives_the_shared_rows_for_no_environment_or_one_without_a_list() {
            let (shared, by_env) = lists();
            let all: Vec<&Row> = shared.iter().collect();
            assert_eq!(effective_rows(&shared, &by_env, None), all);
            assert_eq!(effective_rows(&shared, &by_env, Some("prod")), all);
            // the TypeScript's `Object.hasOwn` guard: a name on `Object.prototype` is no list. A map has no prototype.
            assert_eq!(effective_rows(&shared, &by_env, Some("constructor")), all);
        }
    }

    mod env_tag_values {
        use super::*;

        fn tags(list: &[&str]) -> Vec<String> {
            list.iter().map(|t| t.to_string()).collect()
        }

        #[test]
        fn splits_values_of_env_and_envnot_tags() {
            assert_eq!(
                env_tag_values(&tags(&["env=dev,qa", "envnot=prod", "smoke", "env"])),
                ["dev", "qa", "prod"]
            );
        }

        #[test]
        fn drops_invalid_names_and_ignores_look_alike_tags() {
            assert_eq!(
                env_tag_values(&tags(&["env=a b,,../x,ok", "environment=x", "envnot="])),
                ["ok"]
            );
        }

        /// Not in the TypeScript: its `.` does not match a line terminator, so such a tag is no env tag.
        #[test]
        fn ignores_a_tag_with_a_line_terminator() {
            assert!(env_tag_values(&tags(&["env=dev\nqa", "env=dev\r", "env=a\u{2028}b"])).is_empty());
        }
    }

    mod sorted_unique {
        use super::*;

        #[test]
        fn merges_de_duplicates_and_sorts() {
            let lists = [
                vec!["qa".to_string(), "dev".to_string()],
                vec!["prod".to_string(), "qa".to_string()],
                vec![],
            ];
            assert_eq!(sorted_unique(&lists), ["dev", "prod", "qa"]);
        }
    }

    mod parse_properties_edit {
        use super::*;

        fn ok_json() -> Value {
            json!({
                "shared": [row_json("a", "1", false)],
                "byEnv": { "qa": [row_json("t", "x", true), { "name": "k", "secret": true, "keep": true }] }
            })
        }

        #[test]
        fn accepts_a_well_formed_edit_and_keeps_only_known_fields() {
            assert_eq!(
                parse_properties_edit(&ok_json()),
                Some(edit(
                    vec![plain("a", "1")],
                    vec![("qa", vec![secret("t", "x"), keep("k")])]
                ))
            );
            let extra = json!({ "shared": [{ "name": "a", "value": "1", "secret": false, "extra": 1 }], "byEnv": {} });
            assert_eq!(parse_properties_edit(&extra), Some(edit(vec![plain("a", "1")], vec![])));
        }

        /// Each is a test of its own in the TypeScript (`it.each`); `rows` is 501 rows.
        macro_rules! rejects {
            ($($name:ident: $x:expr,)*) => {
                $(
                    #[test]
                    fn $name() {
                        let x: Value = $x;
                        assert_eq!(parse_properties_edit(&x), None, "{x}");
                    }
                )*
            };
        }

        fn rows(n: usize) -> Vec<Value> {
            (0..n).map(|i| row_json(&format!("p{i}"), "1", false)).collect()
        }

        // A sparse array, `[ <empty item> ]`, cannot be built from JSON: `serde_json::Value` has no holes, and
        // `JSON.stringify` writes one as `null`. The three sparse cases are checked as the closest thing there is, a
        // `null` where a row should be, which no row is.
        rejects! {
            rejects_null: Value::Null,
            rejects_a_string: json!("x"),
            rejects_an_array: json!([]),
            rejects_an_empty_object: json!({}),
            rejects_shared_not_an_array: json!({ "shared": {}, "byEnv": {} }),
            rejects_by_env_an_array: json!({ "shared": [], "byEnv": [] }),
            rejects_an_environment_list_not_an_array: json!({ "shared": [], "byEnv": { "qa": {} } }),
            rejects_a_row_without_a_name: json!({ "shared": [{ "value": "1", "secret": false }], "byEnv": {} }),
            rejects_a_non_string_value: json!({ "shared": [{ "name": "a", "value": 1, "secret": false }], "byEnv": {} }),
            rejects_a_row_without_secret: json!({ "shared": [{ "name": "a", "value": "1" }], "byEnv": {} }),
            rejects_keep_on_a_non_secret_row: json!({ "shared": [{ "name": "a", "secret": false, "keep": true }], "byEnv": {} }),
            rejects_keep_false: json!({ "shared": [{ "name": "a", "secret": true, "keep": false }], "byEnv": {} }),
            rejects_a_10_001_character_value: json!({ "shared": [row_json("a", &"x".repeat(MAX_VALUE_LENGTH + 1), false)], "byEnv": {} }),
            rejects_a_201_character_name: json!({ "shared": [row_json(&"a".repeat(201), "1", false)], "byEnv": {} }),
            rejects_an_invalid_environment_name: json!({ "shared": [], "byEnv": { "../x": [] } }),
            rejects_an_own_proto_environment: serde_json::from_str(r#"{"shared":[],"byEnv":{"__proto__":[]}}"#).unwrap(),
            rejects_501_shared_rows: json!({ "shared": rows(501), "byEnv": {} }),
            rejects_501_environment_rows: json!({ "shared": [], "byEnv": { "qa": rows(501) } }),
            rejects_101_environments: json!({
                "shared": [],
                "byEnv": (0..101).map(|i| (format!("e{i}"), json!([]))).collect::<serde_json::Map<_, _>>()
            }),
            rejects_a_sparse_shared_list: json!({ "shared": [null], "byEnv": {} }),
            rejects_a_sparse_environment_list: json!({ "shared": [], "byEnv": { "qa": [null, null] } }),
            rejects_a_sparse_list_after_structured_clone: json!({ "shared": [null, null, null], "byEnv": {} }),
        }

        #[test]
        fn accepts_the_limits_themselves() {
            let shared: Vec<Value> = (0..500)
                .map(|i| row_json(&format!("p{i}"), &"x".repeat(MAX_VALUE_LENGTH), false))
                .collect();
            let by_env: serde_json::Map<_, _> = (0..100).map(|i| (format!("e{i}"), json!([]))).collect();
            assert!(parse_properties_edit(&json!({ "shared": shared, "byEnv": by_env })).is_some());
        }

        /// Not in the TypeScript: a length counts UTF-16 units, and the environments come out as JavaScript lists keys.
        #[test]
        fn counts_utf16_units_and_lists_environments_as_javascript_does() {
            let emoji = |n: usize| json!({ "shared": [row_json("a", &"😀".repeat(n), false)], "byEnv": {} });
            assert!(parse_properties_edit(&emoji(5000)).is_some()); // 10,000 units
            assert!(parse_properties_edit(&emoji(5001)).is_none());
            let x: Value = serde_json::from_str(r#"{"shared":[],"byEnv":{"b":[],"10":[],"2":[],"a":[]}}"#).unwrap();
            let order: Vec<String> = parse_properties_edit(&x).unwrap().by_env.into_keys().collect();
            assert_eq!(order, ["2", "10", "b", "a"]);
        }
    }

    mod edit_errors {
        use super::*;

        #[test]
        fn is_empty_for_a_valid_edit() {
            let e = edit(vec![plain("a", "1")], vec![("qa", vec![secret("t", "x")])]);
            assert_eq!(edit_errors(&e, no_secrets), Vec::<String>::new());
        }

        #[test]
        fn reports_duplicates_and_reserved_names_per_scope() {
            let e = edit(
                vec![plain("a", "1"), plain("a", "2"), plain("karate.env", "x")],
                vec![(
                    "qa",
                    vec![plain("1bad", "x"), plain("timeout", "1"), plain("timeout", "2")],
                )],
            );
            assert_eq!(
                edit_errors(&e, no_secrets),
                [
                    "All environments: a: Duplicate name",
                    "All environments: karate.env: Reserved name",
                    "qa: 1bad: Invalid name",
                    "qa: timeout: Duplicate name",
                ]
            );
        }

        #[test]
        fn reports_value_problems_secret_values_included() {
            let e = edit(
                vec![plain("a", "é"), secret("b", "x\ny")],
                vec![("qa", vec![plain("c", &"x".repeat(10_001))])],
            );
            assert_eq!(
                edit_errors(&e, no_secrets),
                [
                    format!("All environments: a: {CANNOT_PASS}"),
                    "All environments: b: No line breaks".to_string(),
                    "qa: c: Too long".to_string(),
                ]
            );
        }

        #[test]
        fn keep_needs_a_stored_secret_of_the_same_name_in_the_same_scope() {
            let stored: HashSet<(Option<&str>, &str)> = HashSet::from([(None, "token"), (Some("qa"), "api")]);
            let has = |scope: Option<&str>, name: &str| stored.contains(&(scope, name));
            let e = edit(
                vec![keep("token"), keep("api")],
                vec![("qa", vec![keep("api"), keep("token")])],
            );
            assert_eq!(
                edit_errors(&e, has),
                [
                    "All environments: api: No saved secret to keep",
                    "qa: token: No saved secret to keep"
                ]
            );
        }
    }

    mod environment_variables {
        use super::*;

        fn variable(name: &str) -> RowEdit {
            RowEdit::Value {
                name: name.into(),
                value: "v".into(),
                secret: false,
                env_var: Some(true),
            }
        }

        #[test]
        fn follow_their_own_name_rule() {
            assert_eq!(env_var_name_error("APP_X"), None);
            assert_eq!(env_var_name_error("_x9"), None);
            for bad in ["db.user", "9A", "", "a-b", &"A".repeat(201)] {
                assert_eq!(env_var_name_error(bad), Some(NameError::Invalid), "{bad}");
            }
        }

        #[test]
        fn cannot_be_one_grace_strips_from_karates_environment_in_any_case() {
            for name in [
                "KARATE_OPTIONS",
                "karate_env",
                "Karate_Config_Dir",
                "java_tool_options",
                "JDK_JAVA_OPTIONS",
                "_JAVA_OPTIONS",
            ] {
                assert_eq!(env_var_name_error(name), Some(NameError::Reserved), "{name}");
            }
        }

        #[test]
        fn the_kind_picks_the_rule() {
            assert_eq!(name_error("db.user", false), None);
            assert_eq!(name_error("db.user", true), Some(NameError::Invalid));
            assert_eq!(name_error("java.home", false), Some(NameError::Reserved));
        }

        #[test]
        fn names_a_key_the_prefix_then_the_key_in_upper_case_with_other_characters_as_underscore() {
            assert_eq!(env_var_from_key("APP_", "db.user"), "APP_DB_USER");
            assert_eq!(env_var_from_key("", "café-x"), "CAF__X");
            assert_eq!(env_var_from_key("p", "a b"), "pA_B");
        }

        #[test]
        fn an_edit_keeps_the_kind_and_refuses_a_malformed_one() {
            let with_kind = |row: Value| json!({ "shared": [row], "byEnv": {} });
            let parsed = parse_properties_edit(&with_kind(
                json!({ "name": "A", "value": "v", "secret": false, "envVar": true }),
            ));
            assert_eq!(parsed.unwrap().shared[0], variable("A"));
            let kept = parse_properties_edit(&with_kind(
                json!({ "name": "A", "secret": true, "keep": true, "envVar": true }),
            ));
            assert_eq!(
                kept.unwrap().shared[0],
                RowEdit::Keep {
                    name: "A".into(),
                    env_var: Some(true)
                }
            );
            let bad = parse_properties_edit(&with_kind(
                json!({ "name": "A", "value": "1", "secret": false, "envVar": "yes" }),
            ));
            assert_eq!(bad, None);
        }

        #[test]
        fn are_checked_by_their_rule_on_save() {
            assert_eq!(
                edit_errors(&edit(vec![variable("java_tool_options")], vec![]), no_secrets),
                ["All environments: java_tool_options: Reserved name"]
            );
            assert_eq!(
                edit_errors(&edit(vec![variable("db.user")], vec![]), no_secrets),
                ["All environments: db.user: Invalid name"]
            );
        }

        #[test]
        fn two_variables_that_differ_only_in_case_are_duplicates_a_property_beside_them_is_not() {
            assert_eq!(
                edit_errors(
                    &edit(vec![variable("App_Token"), variable("APP_TOKEN")], vec![]),
                    no_secrets
                ),
                ["All environments: APP_TOKEN: Duplicate name"]
            );
            assert_eq!(
                edit_errors(
                    &edit(vec![variable("APP_TOKEN"), plain("app_token", "1")], vec![]),
                    no_secrets
                ),
                Vec::<String>::new()
            );
            assert_eq!(
                edit_errors(&edit(vec![plain("X", "1"), variable("X")], vec![]), no_secrets),
                ["All environments: X: Duplicate name"]
            );
        }

        /// Not in the TypeScript: the replace of `env_var_from_key` works on UTF-16 units, so an emoji is two.
        #[test]
        fn a_character_above_u_ffff_is_two_underscores() {
            assert_eq!(env_var_from_key("", "a😀"), "A__");
        }
    }

    /// Not in the TypeScript, whose patterns are regular expressions: the hand-written checks of the same patterns.
    #[test]
    fn the_name_checks_are_the_patterns() {
        assert!(valid_prefix("") && valid_prefix("APP_1") && !valid_prefix("a-b") && !valid_prefix("é"));
        assert!(valid_property_name("a.b-c_d") && !valid_property_name("a=b") && !valid_property_name("-a"));
        assert!(!valid_env_name("a\u{0}b") && !valid_env_name("a\n"));
    }
}
