//! `src/shared/tags.ts`: the tag filter. Tags are stored and compared without '@', case-sensitive.
use serde::{Deserialize, Serialize};

/// A project's tag filter. Electron Grace's `TagFilter` is this without `hide_non_matching`: functions that take one
/// take a `FilterState` and ignore the flag.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterState {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    /// Hide tests and tree nodes that do not match; dim them when false.
    pub hide_non_matching: bool,
}

/// No tag included or excluded, non-matching tests dimmed.
pub fn no_filter() -> FilterState {
    FilterState::default()
}
