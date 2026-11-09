//! `src/shared/trends.ts`: what the window gets for the trend strip (docs/specs/2026-11-04-trends.md).
use serde::{Deserialize, Serialize};

/// One stored run as the trend strip shows it, for the selection asked about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    /// The run folder's name.
    pub id: String,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    /// Counted tests (stored `passed` or `failed`, under the selection) that passed.
    pub passed: u32,
    pub failed: u32,
    /// The tests under the selection in the current tree.
    pub total: u32,
    /// The mean `durationMillis` of the counted tests that have one; None if none has.
    #[serde(with = "crate::model::js_num_opt")]
    pub mean_millis: Option<f64>,
}
