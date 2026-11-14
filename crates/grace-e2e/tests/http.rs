//! `e2e/http.spec.ts`'s list half through the accessibility tree: the calls of a real run listed, grouped and narrowed
//! (HI-01 to HI-03, HI-05, HI-06, HI-11), and the list box's keys (VL-13, VL-25). A call's details and Copy as cURL
//! (E-http-01's other steps, E-http-03) come with the next slice. What the rows draw, their tooltips and the scroll
//! rules are the headless tests' in `grace::http`.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(120);
const VIA: &str = "via tests/common/create-user.feature:8";

fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

/// The HTTP tab, whose name carries its count.
fn tab(tree: &Node) -> Option<&Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Tab && n.name.starts_with("HTTP"))
}

fn tab_is(tree: &Node, name: &str) -> bool {
    tab(tree).is_some_and(|t| t.name == name)
}

/// The calls the list box shows, in order.
fn calls(tree: &Node) -> Vec<&Node> {
    tree.find(&Role::List, "HTTP calls")
        .map(|l| l.all().into_iter().filter(|n| n.role == Role::ListItem).collect())
        .unwrap_or_default()
}

/// A call's status, the first part of its name: "404", or "no status".
fn status(call: &Node) -> &str {
    call.name.split(", ").next().unwrap_or_default()
}

fn statuses(tree: &Node) -> Vec<String> {
    let mut s: Vec<_> = calls(tree).iter().map(|c| status(c).to_owned()).collect();
    s.sort();
    s
}

/// A project open in a fresh app, with a bottom panel tall enough for every call of a run (13 rows of 28 px).
fn open(name: &str) -> App {
    let dir = project(name).unwrap();
    let settings = json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 520 });
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| {
        starting(t, "tests ").is_some() && tab(t).is_some()
    })
    .unwrap();
    app
}

fn run_all(app: &App) {
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
}

fn show_http(app: &App) {
    let t = app.tree().unwrap();
    app.press(tab(&t).unwrap()).unwrap();
    app.wait_for("the HTTP tab", WAIT, |t| {
        tab(t).is_some_and(|n| n.states.selected) && starting(t, "All ").is_some()
    })
    .unwrap();
}

/// Clicks a test in the tree, which narrows the list to its calls.
fn select(app: &App, label: &str) {
    let t = app.tree().unwrap();
    app.click(row(&t, label).unwrap_or_else(|| panic!("{label} in {}", t.dump())))
        .unwrap();
}

fn press_chip(app: &App, name: &str) {
    app.press_named(Role::Button, name).unwrap();
}

/// HI-01, HI-02, HI-03, HI-06, VL-13, VL-25 (E-http-01's list steps): one Run all of the http sample project.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn reading_the_calls_of_one_run_all() {
    let app = open("http");
    run_all(&app);

    // HI-01: every call of a run is listed and grouped
    app.wait_for("HTTP (13)", WAIT, |t| tab_is(t, "HTTP (13)")).unwrap();
    show_http(&app);
    let chips = [
        "All 13",
        "1xx Informational 0",
        "2xx Success 6",
        "3xx Redirect 1",
        "4xx Client error 3",
        "5xx Server error 1",
        "No response 2",
    ];
    let t = app
        .wait_for("the chips and the calls", WAIT, |t| {
            chips.iter().all(|c| t.find(&Role::Button, c).is_some()) && calls(t).len() == 13
        })
        .unwrap();
    assert!(t.find(&Role::Button, "All 13").unwrap().states.checked, "All is on");

    // VL-25: calls are options of a list box named "HTTP calls"; none is selected yet
    assert!(calls(&t).iter().all(|c| !c.states.selected), "{}", t.dump());

    // HI-02: showing only the calls that went wrong
    for chip in ["4xx Client error 3", "5xx Server error 1", "No response 2"] {
        press_chip(&app, chip);
    }
    let t = app
        .wait_for("six calls that went wrong", WAIT, |t| {
            tab_is(t, "HTTP (6)") && calls(t).len() == 6
        })
        .unwrap();
    assert_eq!(statuses(&t), ["404", "404", "404", "500", "no status", "no status"]);
    assert!(!t.find(&Role::Button, "All 13").unwrap().states.checked);
    press_chip(&app, "All 13"); // turns the others off
    app.wait_for("all 13 again", WAIT, |t| tab_is(t, "HTTP (13)") && calls(t).len() == 13)
        .unwrap();

    // VL-13: a click selects a call; the arrows move to the next, select it and keep the focus there
    let t = app.tree().unwrap();
    app.click(calls(&t)[0]).unwrap();
    app.wait_for("the first call selected", WAIT, |t| {
        calls(t).first().is_some_and(|c| c.states.selected)
    })
    .unwrap();
    app.key(Key::Down).unwrap();
    app.wait_for("the second call selected and focused", WAIT, |t| {
        let c = calls(t);
        c.len() == 13 && !c[0].states.selected && c[1].states.selected && c[1].states.focused
    })
    .unwrap();
    // one tab stop: Tab leaves the list rather than moving on to the next call
    app.key(Key::Tab).unwrap();
    app.wait_for("the focus out of the list", WAIT, |t| {
        let c = calls(t);
        c.len() == 13 && c[1].states.selected && c.iter().all(|o| !o.states.focused)
    })
    .unwrap();

    // HI-03: the tree selection narrows the list
    select(&app, "retry");
    let t = app
        .wait_for("the three attempts", WAIT, |t| {
            tab_is(t, "HTTP (3)") && calls(t).len() == 3
        })
        .unwrap();
    assert_eq!(statuses(&t), ["404", "404", "404"]);
    let attempts: Vec<_> = calls(&t).iter().map(|c| c.description.clone()).collect();
    assert_eq!(attempts, ["attempt 1 of 3", "attempt 2 of 3", "attempt 3 of 3"]);

    // HI-06: a shared setup call shows under the scenario Karate ran it in
    // the panel is tall: close http.feature so shared.feature's rows are in view, and so in the accessibility tree
    if row(&app.tree().unwrap(), "first").is_none() {
        app.press_named(Role::Button, "Collapse http.feature").unwrap();
        app.wait_for("http.feature closed", WAIT, |t| row(t, "retry").is_none())
            .unwrap();
        app.press_named(Role::Button, "Expand shared.feature").unwrap();
        app.wait_for("the shared scenarios", WAIT, |t| row(t, "first").is_some())
            .unwrap();
    }
    select(&app, "first");
    let t = app
        .wait_for("the shared call", WAIT, |t| {
            tab_is(t, "HTTP (1)") && calls(t).len() == 1
        })
        .unwrap();
    let call = calls(&t)[0];
    assert!(call.name.starts_with("201, POST, "), "{}", call.name);
    assert_eq!(call.description, VIA);
    select(&app, "second");
    app.wait_for("no calls in second", WAIT, |t| {
        tab_is(t, "HTTP (0)") && starting(t, "No HTTP calls in second").is_some()
    })
    .unwrap();
}

/// HI-05, E-http-02: a call made in a called feature.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_call_made_in_a_called_feature() {
    let app = open("clean");
    run_all(&app);
    show_http(&app);
    if row(&app.tree().unwrap(), "calls helper feature").is_none() {
        app.press_named(Role::Button, "Expand basic.feature").unwrap();
        app.wait_for("the scenarios", WAIT, |t| row(t, "calls helper feature").is_some())
            .unwrap();
    }
    select(&app, "calls helper feature");
    let t = app
        .wait_for("the helper's call", WAIT, |t| {
            calls(t).len() == 1 && tab_is(t, "HTTP (1)")
        })
        .unwrap();
    let call = calls(&t)[0];
    assert!(call.name.starts_with("201, POST, "), "{}", call.name);
    assert_eq!(call.description, VIA);
}

/// HI-11, E-http-04: a run of one test replaces only its calls, and the HTTP tab stays open through it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_run_of_one_test_replaces_only_its_calls() {
    let app = open("http");
    run_all(&app);
    app.wait_for("HTTP (13)", WAIT, |t| tab_is(t, "HTTP (13)")).unwrap();
    show_http(&app);
    if row(&app.tree().unwrap(), "ok").is_none() {
        app.press_named(Role::Button, "Expand http.feature").unwrap();
        app.wait_for("the scenarios", WAIT, |t| row(t, "ok").is_some()).unwrap();
    }
    app.press_named(Role::Button, "Run ok").unwrap();
    // a stored run has ended: the first Run all is the first entry
    app.wait_for("the second run over", RUN, |t| {
        t.find(&Role::Tab, "History (2)").is_some() && starting(t, "Finished").is_some()
    })
    .unwrap();
    let t = app
        .wait_for("13 calls still", WAIT, |t| {
            tab_is(t, "HTTP (13)") && calls(t).len() == 13
        })
        .unwrap();
    // starting the run kept the HTTP tab open
    assert!(tab(&t).unwrap().states.selected);
    // "ok" has its new call, and another test kept its calls of Run all
    select(&app, "ok");
    let t = app
        .wait_for("the call of ok", WAIT, |t| tab_is(t, "HTTP (1)") && calls(t).len() == 1)
        .unwrap();
    assert!(calls(&t)[0].name.starts_with("200, GET, "), "{}", calls(&t)[0].name);
    select(&app, "retry");
    app.wait_for("the calls of retry", WAIT, |t| {
        tab_is(t, "HTTP (3)") && statuses(t) == ["404", "404", "404"]
    })
    .unwrap();
}
