//! `e2e/examples.spec.ts` (E-examples-01) and DD-05 through the accessibility tree: an outline selected shows its
//! Examples as a list box named "Examples" whose options are named "Row <n>: <column> <value>, ..., <status>[, <time>]".
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Node, Role, fresh_data_dir, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(120);

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

/// A tree row, "<label>, <status>...".
fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

/// The lines of the table, in order.
fn lines(tree: &Node) -> Vec<&Node> {
    tree.find(&Role::List, "Examples")
        .map(|l| l.all().into_iter().filter(|n| n.role == Role::ListItem).collect())
        .unwrap_or_default()
}

fn names(tree: &Node) -> Vec<String> {
    lines(tree).iter().map(|n| n.name.clone()).collect()
}

/// Grace on a copy of the fixture project `name`, ready, with `feature` expanded and the outline `label` selected.
fn open_outline(name: &str, feature: &str, label: &str) -> App {
    let dir = project(name).unwrap();
    let data = fresh_data_dir().unwrap();
    std::fs::write(
        data.join("settings.json"),
        json!({ "lastProject": dir.to_string_lossy() }).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
    app.press_named(Role::Button, &format!("Expand {feature}")).unwrap();
    let t = app.wait_for(label, WAIT, |t| row(t, label).is_some()).unwrap();
    app.click(row(&t, label).unwrap()).unwrap();
    app
}

/// E-examples-01, DD-01, DD-02, DD-03: the table before a run, the result of each line after it, and a click that
/// opens a row, which the tree then shows selected.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn an_outlines_examples_as_a_table() {
    let app = open_outline("clean", "outline.feature", "add <a> and <b>");
    let t = app
        .wait_for("the table", WAIT, |t| lines(t).len() == 3)
        .unwrap_or_else(|e| panic!("{e}: {}", app.tree().unwrap().dump()));
    assert_eq!(
        names(&t),
        [
            "Row 1: a 1, b 2, expected 3, Not run",
            "Row 2: a 2, b 3, expected 5, Not run",
            "Row 3: a 5, b 5, expected 10, Not run",
        ]
    );

    app.press_named(Role::Button, "Run all").unwrap();
    let t = app
        .wait_for("every line passed", RUN, |t| {
            let n = names(t);
            n.len() == 3
                && n.iter()
                    .zip(["a 1, b 2, expected 3", "a 2, b 3, expected 5", "a 5, b 5, expected 10"])
                    .enumerate()
                    .all(|(i, (name, values))| {
                        name.starts_with(&format!("Row {}: {values}, Passed, ", i + 1)) && name.ends_with(" ms")
                    })
        })
        .unwrap();

    app.click(lines(&t)[1]).unwrap();
    let t = app
        .wait_for("the row opened", WAIT, |t| {
            t.all().iter().any(|n| n.name == "add 2 and 3") && row(t, "add 2 and 3").is_some_and(|r| r.states.selected)
        })
        .unwrap();
    assert!(lines(&t).is_empty(), "the table is the outline's, not the row's");
}

/// DD-05: a dynamic Examples table has the column its data gives.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_dynamic_examples_table() {
    let app = open_outline("edge", "rows.feature", "dynamic <a>");
    let t = app
        .wait_for("the table", WAIT, |t| lines(t).len() == 2)
        .unwrap_or_else(|e| panic!("{e}: {}", app.tree().unwrap().dump()));
    assert_eq!(names(&t), ["Row 1: a 1, Not run", "Row 2: a 2, Not run"]);
}
