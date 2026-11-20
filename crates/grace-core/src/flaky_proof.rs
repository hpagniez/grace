//! `FlakyProof.tsx`'s rules: what the boxes of a flaky test's proof say, one per environment
//! (docs/specs/2026-10-26-flaky-tests.md R5). `grace::flaky_proof` only draws it.
use crate::model::flaky::{FlakyEnv, FlakyStatus, flaky_counts, flaky_heading, order_envs, threads_label};
use crate::model::history::when_label;

/// One proof run: a row of a box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofRun {
    pub id: String,
    /// "Today 14:32", as the History tab writes it.
    pub when: String,
    pub status: FlakyStatus,
    /// The word that names the result: "Passed" or "Failed".
    pub result: &'static str,
    pub threads: String,
    /// Whether its Report button takes a press: the run has a report and nothing is running (R5.3).
    pub report: bool,
}

/// One environment's box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofBox {
    /// The environment it is about, None for none.
    pub env: Option<String>,
    pub heading: String,
    pub counts: String,
    pub runs: Vec<ProofRun>,
}

/// The boxes of a test flaky in `envs`, the `current` environment first (R5.1). `now` is the clock for the rows' times;
/// `locked` is `history_list::locked`.
pub fn proof_boxes(envs: &[FlakyEnv], current: Option<&str>, now: f64, locked: bool) -> Vec<ProofBox> {
    order_envs(envs, current)
        .into_iter()
        .map(|e| ProofBox {
            env: e.env.clone(),
            heading: flaky_heading(e.env.as_deref()),
            counts: flaky_counts(e.passed, e.failed),
            runs: e
                .runs
                .iter()
                .map(|r| ProofRun {
                    id: r.id.clone(),
                    when: when_label(r.started_at, now),
                    status: r.status,
                    result: match r.status {
                        FlakyStatus::Passed => "Passed",
                        FlakyStatus::Failed => "Failed",
                    },
                    threads: threads_label(r.threads),
                    report: !locked && r.report_available,
                })
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
