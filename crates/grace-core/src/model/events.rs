//! `src/shared/events.ts`: what a run reports. Electron Grace sent these over IPC as `AppEvent`s; GPUI Grace has no
//! IPC (migration guide, Architecture), and the runner reports the same events to the `Grace` entity instead.
use super::node::{Layout, Status};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// Gherkin prefix as written: Given, When, Then, And, or `*`.
    pub keyword: String,
    /// Karate keyword and its argument, e.g. `match response.message == 'x'`.
    pub text: String,
    pub line: u32,
    /// Karate's `result.status` as-is.
    pub status: String,
    #[serde(with = "crate::model::js_num")]
    pub millis: f64,
    pub background: bool,
    pub log: Option<String>,
    /// The `#` lines Karate attached to the step, trimmed; absent when it has none (step comments R1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<Vec<String>>,
}

/// A step of a test as its feature file has it, before any run (step debugger R4.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStep {
    pub keyword: String,
    pub text: String,
    pub line: u32,
    pub background: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<Vec<String>>,
}

/// A step debugger breakpoint: the tree's `file` (Karate's feature path) and a step line.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Breakpoint {
    pub file: String,
    pub line: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DebugCommand {
    Continue,
    Over,
    Into,
    Pause,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VarKind {
    String,
    Number,
    Boolean,
    Null,
    Json,
    Xml,
    Function,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugVar {
    pub name: String,
    pub kind: VarKind,
    /// The Java class's simple name, for `other` only.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    pub value: String,
    /// Characters left out after the first 65,536.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cut: Option<u32>,
}

/// Where a debug run waits: the step it has not run yet, and the scenario's variables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paused {
    /// The paused test's key in the tree; None if the tree has no such test.
    pub key: Option<String>,
    pub file: String,
    pub line: u32,
    /// 0 for the test's own steps, 1 or more inside a called feature.
    pub depth: u32,
    pub prefix: String,
    pub keyword: String,
    pub text: String,
    pub vars: Vec<DebugVar>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioUpdate {
    pub key: String,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "crate::model::js_num_opt")]
    pub duration_millis: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Where Karate's reason starts in `error`, in UTF-16 units as Electron Grace counted (#229 R2.1); None: the whole
    /// error is the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_at: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub scenario_count: u32,
    pub scenarios_passed: u32,
    pub scenarios_failed: u32,
    #[serde(with = "crate::model::js_num")]
    pub duration_millis: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunOutcome {
    Finished,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub root: String,
    pub layout: Layout,
}

/// A saved recent folder and whether it is still a folder on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    pub exists: bool,
}
