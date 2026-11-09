//! `src/shared/history.ts`: what a finished run stores and how the History tab words it
//! (docs/specs/2026-10-25-run-history.md).
use super::events::RunOutcome;
use super::node::NodeKind;
use super::tags::FilterState;
use serde::{Deserialize, Serialize};

pub const HISTORY_LIMIT: usize = 20;

/// A test's status once its run has ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestStatusFinal {
    Passed,
    Failed,
    NotRun,
    Interrupted,
}

/// `scenario` or `row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeafKind {
    Scenario,
    Row,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTest {
    pub kind: LeafKind,
    pub file: String,
    pub ref_id: String,
    pub name: String,
    pub line: Option<u32>,
    pub key: String,
    pub status: TestStatusFinal,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "crate::model::js_num_opt")]
    pub duration_millis: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSelection {
    pub kind: NodeKind,
    pub file: String,
    pub name: String,
    pub line: Option<u32>,
    pub key: String,
}

/// `run.json`. Never holds a property value, an error text, a step log or a console line. Its keys are written in
/// Electron Grace's order: `debug` (when set) first, then as below.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    /// A step debugger run; absent for any other. Flaky detection leaves it out (step debugger R2.7).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub debug: bool,
    /// Always 1.
    pub version: u32,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    #[serde(with = "crate::model::js_num")]
    pub duration_millis: f64,
    pub env: Option<String>,
    /// The count passed with `-T`; None for a run stored before F12.
    pub threads: Option<u32>,
    /// SHA-256 of the project files' paths, sizes and times, the same before Java started and after it exited (F15).
    pub fingerprint: Option<String>,
    pub filter: FilterState,
    /// None for a Re-run failed, whose `failed_of` is then the `started_at` of its source run.
    pub selection: Option<StoredSelection>,
    #[serde(with = "crate::model::js_num_opt")]
    pub failed_of: Option<f64>,
    pub outcome: RunOutcome,
    pub exit_code: Option<i32>,
    pub tests: Vec<StoredTest>,
}

/// A listed run as the window gets it: the record without `tests`, with counts and its folder id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    #[serde(with = "crate::model::js_num")]
    pub duration_millis: f64,
    pub env: Option<String>,
    pub threads: Option<u32>,
    pub filter: FilterState,
    pub selection: Option<StoredSelection>,
    #[serde(with = "crate::model::js_num_opt")]
    pub failed_of: Option<f64>,
    pub outcome: RunOutcome,
    pub exit_code: Option<i32>,
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub report_available: bool,
    /// Electron Grace's `toEntry` keeps `debug` from the record.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub debug: bool,
}
