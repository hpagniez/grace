//! `History.tsx`'s rules: what a listed run's row says and which of its buttons take a press
//! (docs/specs/2026-10-25-run-history.md R3, R8). `grace::history` only draws it.
use crate::model::events::RunOutcome;
use crate::model::history::{HistoryEntry, StoredSelection, result_label, what_ran_label, when_label};
use crate::model::node::NodeKind;
use crate::state::AppState;

/// What the tab says with no runs.
pub const EMPTY: &str = "No runs yet. Runs appear here when they finish.";

/// "18s", or "1m 12s" from a minute on; a run under one second is "0s".
pub fn duration_label(millis: f64) -> String {
    let s = (millis / 1000.).floor();
    if s >= 60. {
        format!("{}m {:02}s", (s / 60.).floor(), s % 60.)
    } else {
        format!("{s}s")
    }
}

/// The tooltip of "What ran": where the selected node sits. Nothing for All and for a Re-run failed run.
pub fn selection_path(s: Option<&StoredSelection>) -> Option<String> {
    let s = s?;
    let path = match s.kind {
        NodeKind::Folder | NodeKind::Feature => s.file.clone(),
        _ => format!("{} › {}", s.file, s.name),
    };
    (!path.is_empty()).then_some(path)
}

/// The mark of a run's result: a failure beats a cancel (R7.1); the words next to it say which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultMark {
    Passed,
    Failed,
    Cancelled,
}

pub fn result_mark(e: &HistoryEntry) -> ResultMark {
    if e.outcome == RunOutcome::Failed || e.failed > 0 {
        ResultMark::Failed
    } else if e.outcome == RunOutcome::Cancelled {
        ResultMark::Cancelled
    } else {
        ResultMark::Passed
    }
}

/// A tag of the run's filter: "@smoke", or "not @slow" struck through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub text: String,
    pub excluded: bool,
}

/// What a row reads, left to right. `now` is one clock for every label of the row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowText {
    pub when: String,
    /// "—" for none.
    pub env: String,
    /// The thread count, "—" for a run stored before it was kept.
    pub threads: String,
    /// Empty for no filter: the row says "—".
    pub tags: Vec<Tag>,
    pub what: String,
    /// What "What ran" adds in a tooltip.
    pub path: Option<String>,
    pub result: String,
    pub mark: ResultMark,
    pub duration: String,
}

pub fn row_text(e: &HistoryEntry, now: f64) -> RowText {
    let tags = e
        .filter
        .include
        .iter()
        .map(|t| Tag {
            text: format!("@{t}"),
            excluded: false,
        })
        .chain(e.filter.exclude.iter().map(|t| Tag {
            text: format!("not @{t}"),
            excluded: true,
        }))
        .collect();
    RowText {
        when: when_label(e.started_at, now),
        env: e.env.clone().unwrap_or_else(|| "—".into()),
        threads: e.threads.map_or_else(|| "—".into(), |n| n.to_string()),
        tags,
        what: what_ran_label(e, now),
        path: selection_path(e.selection.as_ref()),
        result: result_label(e),
        mark: result_mark(e),
        duration: duration_label(e.duration_millis),
    }
}

/// While a run goes on or the runner is busy, every History action waits (R8).
pub fn locked(state: &AppState) -> bool {
    state.run.active || state.busy
}

/// Which of a row's buttons take a press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actions {
    pub report: bool,
    /// Export follows Report (ER-05).
    pub export: bool,
    pub rerun: bool,
    pub rerun_failed: bool,
}

pub fn actions(e: &HistoryEntry, locked: bool) -> Actions {
    Actions {
        report: !locked && e.report_available,
        export: !locked && e.report_available,
        rerun: !locked,
        rerun_failed: !locked && e.failed > 0,
    }
}

/// Whether Clear history takes a press.
pub fn can_clear(state: &AppState) -> bool {
    !locked(state) && !state.history.is_empty()
}

/// What Clear history asks, with the count of the moment it was opened.
pub fn clear_prompt(count: usize) -> String {
    if count == 1 {
        "Delete the 1 run of this project and its report?".into()
    } else {
        format!("Delete the {count} runs of this project and their reports?")
    }
}

#[cfg(test)]
mod tests;
