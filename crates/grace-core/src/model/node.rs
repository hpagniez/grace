//! `src/shared/model.ts`: the test tree.
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Layout {
    Maven,
    Gradle,
    Standalone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Folder,
    Feature,
    Outline,
    Scenario,
    Row,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    NotRun,
    Pending,
    Running,
    Passed,
    Failed,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestNode {
    /// `d:<file>` folder, `f:<file>` feature, `o:<file>#<section>` outline, `s:<file>#<section>[.<n>]` scenario or row.
    pub key: String,
    pub kind: NodeKind,
    pub name: String,
    /// Folder or feature path relative to the project, '/' separators; '' for the root.
    pub file: String,
    /// Line to run: scenario line, outline line, or row line; None for folders and features.
    pub line: Option<u32>,
    /// Karate refId (`[section:line]`, `[section.example:line]`); scenarios and rows only, None otherwise.
    pub ref_id: Option<String>,
    /// Without '@'.
    pub tags: Vec<String>,
    /// Rows only: the row's values by Examples column, as Karate's dry run gives them (data-driven view R1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<IndexMap<String, String>>,
    pub children: Vec<TestNode>,
}

impl TestNode {
    /// A scenario or a row: a test that runs.
    pub fn is_leaf(&self) -> bool {
        matches!(self.kind, NodeKind::Scenario | NodeKind::Row)
    }
}
