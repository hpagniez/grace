//! `Examples.tsx`'s rules: an outline's Examples as a table, one line per row with its values (docs/specs/
//! 2026-11-03-data-driven-view.md R2, R3). `grace::examples` only draws it.
use crate::model::node::TestNode;

/// px, a line of the table (R2.6).
pub const ROW_HEIGHT: f32 = 24.;
/// The fewest lines the table shows before the details pane scrolls instead (R2.7).
pub const MIN_LINES: usize = 8;

/// One row of the outline: its key and its values, one per column of the table ("" where it has none).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub key: String,
    pub values: Vec<String>,
}

/// The table of an outline: the columns of all its rows in the order first met (R2.2), and a line per row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Table {
    pub columns: Vec<String>,
    pub lines: Vec<Line>,
}

pub fn table(outline: &TestNode) -> Table {
    let mut columns: Vec<String> = Vec::new();
    for r in &outline.children {
        for c in r.example.iter().flat_map(|e| e.keys()) {
            if !columns.contains(c) {
                columns.push(c.clone());
            }
        }
    }
    let lines = outline
        .children
        .iter()
        .map(|r| Line {
            key: r.key.clone(),
            values: columns
                .iter()
                .map(|c| r.example.as_ref().and_then(|e| e.get(c)).cloned().unwrap_or_default())
                .collect(),
        })
        .collect();
    Table { columns, lines }
}

/// A line's accessible name: number, values by column, status, time (R3.3).
pub fn line_name(t: &Table, index: usize, status: &str, time: &str) -> String {
    let values = t
        .columns
        .iter()
        .zip(&t.lines[index].values)
        .filter(|(_, v)| !v.is_empty())
        .map(|(c, v)| format!("{c} {v}"));
    let parts: Vec<String> = values
        .chain([status.to_owned(), time.to_owned()])
        .filter(|p| !p.is_empty())
        .collect();
    format!("Row {}: {}", index + 1, parts.join(", "))
}

/// The least height of the table: its header and `MIN_LINES` lines or its line count, whichever is fewer, and the
/// border (R2.7).
pub fn min_height(lines: usize) -> f32 {
    (lines.min(MIN_LINES) + 1) as f32 * ROW_HEIGHT + 2.
}

#[cfg(test)]
mod tests;
