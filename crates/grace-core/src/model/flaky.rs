//! `src/shared/flaky.ts`: what the window gets about flaky tests (docs/specs/2026-10-26-flaky-tests.md).
use serde::{Deserialize, Serialize};

/// `passed` or `failed`: the statuses that count for flaky detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlakyStatus {
    Passed,
    Failed,
}

/// One run that proves a test flaky: the test `passed` or `failed` in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakyRun {
    pub id: String,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    pub status: FlakyStatus,
    pub threads: Option<u32>,
    pub report_available: bool,
}

/// The proof of one environment (None: none): the runs with the same project files, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakyEnv {
    pub env: Option<String>,
    pub passed: u32,
    pub failed: u32,
    pub runs: Vec<FlakyRun>,
}

/// A test of the tree last sent that is flaky, by its key in that tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakyTest {
    pub key: String,
    pub envs: Vec<FlakyEnv>,
}
