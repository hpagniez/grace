//! `panelTabs.test.ts`, test for test. Its storage tests are `settings::tests::panel_tabs`; "leaves the list it was
//! given as it was" is the borrow: `move_tab` takes the order by reference.
use super::*;
use TabId::{Console, History, Http};
use serde_json::{Value, json};

const ORDER: [TabId; 3] = [Console, History, Http];

#[test]
fn puts_the_tab_in_the_place_asked_and_shifts_the_ones_between() {
    assert_eq!(move_tab(&ORDER, Console, 2), Some([History, Http, Console]));
    assert_eq!(move_tab(&ORDER, Http, 0), Some([Http, Console, History]));
    assert_eq!(move_tab(&ORDER, History, 0), Some([History, Console, Http]));
}

#[test]
fn stops_at_the_ends_and_returns_nothing_when_nothing_moves() {
    assert_eq!(move_tab(&ORDER, Console, -1), None);
    assert_eq!(move_tab(&ORDER, Http, 3), None);
    assert_eq!(move_tab(&ORDER, History, 1), None);
}

#[test]
fn reads_a_saved_order() {
    assert_eq!(
        parse_tab_order(Some(&json!(["http", "console", "history"]))),
        [Http, Console, History]
    );
}

#[test]
fn falls_back_to_the_default() {
    for saved in [
        Value::Null,
        json!("not JSON"),
        json!(["history", "console"]),
        json!(["history", "console", "http", "x"]),
        json!(["history", "history", "http"]),
        json!(["history", "console", "logs"]),
        json!(["history", "console", "variables"]),
        json!({ "console": 0 }),
        json!(7),
    ] {
        assert_eq!(parse_tab_order(Some(&saved)), ORDER, "{saved}");
    }
    assert_eq!(parse_tab_order(None), ORDER);
}
