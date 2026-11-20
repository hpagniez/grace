//! The rules of `Examples.test.tsx` that are not drawing: `exampleColumns` and the line's name.
use super::*;
use crate::model::node::NodeKind;
use indexmap::IndexMap;

fn row(n: usize, values: &[(&str, &str)]) -> TestNode {
    TestNode {
        key: format!("s:o.feature#1.{n}"),
        kind: NodeKind::Row,
        name: format!("row {n}"),
        file: "o.feature".into(),
        line: Some(7 + n as u32),
        ref_id: None,
        tags: vec![],
        example: (!values.is_empty()).then(|| {
            values
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<IndexMap<_, _>>()
        }),
        children: vec![],
    }
}

fn outline(rows: Vec<TestNode>) -> TestNode {
    TestNode {
        key: "o:o.feature#1".into(),
        kind: NodeKind::Outline,
        name: "add <a> and <b>".into(),
        file: "o.feature".into(),
        line: Some(3),
        ref_id: None,
        tags: vec![],
        example: None,
        children: rows,
    }
}

#[test]
fn gives_the_columns_of_all_rows_in_the_order_first_met() {
    let t = table(&outline(vec![
        row(1, &[("a", "1"), ("b", "2")]),
        row(2, &[("a", "1"), ("c", "3")]),
        row(3, &[]),
    ]));
    assert_eq!(t.columns, ["a", "b", "c"]);
    // a row without a column has an empty cell there (DD-06), and a row without values only empty cells
    assert_eq!(t.lines[1].values, ["1", "", "3"]);
    assert_eq!(t.lines[2].values, ["", "", ""]);
}

#[test]
fn names_a_line_by_its_number_values_status_and_time() {
    let t = table(&outline(vec![
        row(1, &[("a", "1"), ("b", "2")]),
        row(2, &[("a", "3"), ("c", "4")]),
        row(3, &[]),
    ]));
    assert_eq!(line_name(&t, 0, "Not run", ""), "Row 1: a 1, b 2, Not run");
    assert_eq!(line_name(&t, 1, "Passed", "6 ms"), "Row 2: a 3, c 4, Passed, 6 ms");
    assert_eq!(line_name(&t, 2, "Not run", ""), "Row 3: Not run");
}

#[test]
fn is_never_shorter_than_8_lines_or_its_line_count() {
    assert_eq!(min_height(3), 4. * ROW_HEIGHT + 2.);
    assert_eq!(min_height(8), 9. * ROW_HEIGHT + 2.);
    assert_eq!(min_height(10_000), 9. * ROW_HEIGHT + 2.);
}
