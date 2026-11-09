//! `src/shared/env.ts`: rules for the run environment and the run properties
//! (docs/specs/2026-10-25-environment-switcher.md, docs/specs/2026-11-04-env-var-properties.md).
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PropertiesEdit {
    pub shared: Vec<RowEdit>,
    pub by_env: IndexMap<String, Vec<RowEdit>>,
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
