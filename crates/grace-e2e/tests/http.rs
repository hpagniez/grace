//! `e2e/http.spec.ts`'s list half through the accessibility tree: the calls of a real run listed, grouped and narrowed
//! (HI-01 to HI-03, HI-05, HI-06, HI-11), the list box's keys (VL-13, VL-25), a call's details and Copy as cURL (HI-04,
//! HI-07 to HI-10), and that a secret never shows in a call (E-http-03, on Windows, which has the secret storage). What
//! the rows draw, their tooltips and the scroll rules are the headless tests' in `grace::http`.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(120);
const VIA: &str = "via tests/common/create-user.feature:8";
const COPY: &str = "Copy as cURL";

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
    open_dir(&project(name).unwrap(), 520)
}

/// The bottom panel `panel` px tall: a short one leaves the details pane the room its Copy as cURL button needs, which
/// a press in the tree clicks at the centre of (a button the pane has scrolled out of view is not clicked).
fn open_dir(dir: &std::path::Path, panel: u32) -> App {
    let settings = json!({ "lastProject": dir.to_string_lossy(), "panelHeight": panel });
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

/// A node anywhere whose name is this text, of any role: a label's name is its text.
fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

/// A label anywhere whose text holds this text: the details pane's parts, not a row or a heading that names them.
fn holding<'a>(tree: &'a Node, text: &str) -> Option<&'a Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::Text && n.name.contains(text))
}

/// The heading a call's row opens: its method and URL, from the row's name (status, method, URL, test).
fn heading_of(call: &Node) -> String {
    let parts: Vec<_> = call.name.split(", ").collect();
    format!("{} {}", parts[1], parts[2])
}

fn selected_row(tree: &Node) -> Option<String> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.states.selected)
        .map(|n| n.name.clone())
}

/// Selects a test in the tree with its arrow keys, which select the row they move to and scroll it into view: for a
/// row the tree has not drawn, and for one a click could miss while the tree scrolls. A click on the selected row
/// gives the tree the focus first.
fn select_with(app: &App, label: &str, key: Key) {
    let prefix = format!("{label}, ");
    let t = app.tree().unwrap();
    if let Some(r) = t
        .all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.states.selected)
    {
        app.click(r).unwrap();
    }
    for _ in 0..12 {
        let before = selected_row(&app.tree().unwrap());
        if before.as_deref().is_some_and(|r| r.starts_with(&prefix)) {
            return;
        }
        app.key(key).unwrap();
        app.wait_for("the tree's selection to move", WAIT, |t| selected_row(t) != before)
            .unwrap();
    }
    panic!("{label} was not reached");
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

    // HI-04: a call's request and response
    select_with(&app, "xml", Key::Down);
    let t = app
        .wait_for("the xml call", WAIT, |t| tab_is(t, "HTTP (1)") && calls(t).len() == 1)
        .unwrap();
    app.click(calls(&t)[0]).unwrap();
    let t = app
        .wait_for("the call's details", WAIT, |t| named(t, "200 · 2xx Success").is_some())
        .unwrap();
    let heading = heading_of(calls(&t)[0]);
    assert!(
        heading.starts_with("POST http://") && heading.ends_with("/echo"),
        "{heading}"
    );
    assert!(t.all().iter().any(|n| n.name == heading), "the heading names the call");
    // the body is indented, which a label's text keeps
    assert!(
        holding(
            &t,
            "
  <name>bob</name>
"
        )
        .is_some(),
        "{}",
        t.dump()
    );

    // HI-07: a call that got no response, its error in place of a response and Copy as cURL off
    select_with(&app, "refused", Key::Up);
    let t = app
        .wait_for("the refused call", WAIT, |t| {
            tab_is(t, "HTTP (1)") && calls(t).len() == 1
        })
        .unwrap();
    let call = calls(&t)[0];
    assert!(
        call.name.starts_with("no status, GET, ") && call.name.contains("Connection refused"),
        "{}",
        call.name
    );
    app.click(call).unwrap();
    let t = app
        .wait_for("the error", WAIT, |t| {
            holding(t, "Connection refused").is_some() && named(t, "No response").is_some()
        })
        .unwrap();
    // a button's disabled state reaches UI Automation only; through AT-SPI a disabled button offers no action
    if cfg!(windows) {
        assert!(!t.find(&Role::Button, COPY).unwrap().states.enabled);
    } else {
        assert!(app.press_named(Role::Button, COPY).is_err());
    }

    // HI-10: a click on a test gives the details pane back
    select_with(&app, "ok", Key::Up);
    let t = app
        .wait_for("the ok call", WAIT, |t| tab_is(t, "HTTP (1)") && calls(t).len() == 1)
        .unwrap();
    app.click(calls(&t)[0]).unwrap();
    app.wait_for("the call's details", WAIT, |t| t.find(&Role::Button, COPY).is_some())
        .unwrap();
    select(&app, "ok");
    let t = app
        .wait_for("the test's details", WAIT, |t| {
            t.find(&Role::Button, COPY).is_none() && named(t, "tests/http.feature:6").is_some()
        })
        .unwrap();
    // the background's `url` and the scenario's three steps
    let steps = t
        .find(&Role::List, "Steps")
        .map(|l| l.all().into_iter().filter(|n| n.role == Role::ListItem).count());
    assert_eq!(steps, Some(4), "{}", t.dump());

    // the arrow keys open the next call, and the pane follows
    select_with(&app, "http.feature", Key::Up);
    let t = app
        .wait_for("the feature's calls", WAIT, |t| {
            tab_is(t, "HTTP (12)") && calls(t).len() == 12
        })
        .unwrap();
    app.click(calls(&t)[0]).unwrap();
    let t = app
        .wait_for("the first call's details", WAIT, |t| {
            calls(t).first().is_some_and(|c| c.states.selected) && t.find(&Role::Button, COPY).is_some()
        })
        .unwrap();
    let first = heading_of(calls(&t)[0]);
    assert!(t.all().iter().any(|n| n.name == first));
    app.key(Key::Down).unwrap();
    let t = app
        .wait_for("the second call selected", WAIT, |t| {
            calls(t).get(1).is_some_and(|c| c.states.selected)
        })
        .unwrap();
    let second = heading_of(calls(&t)[1]);
    assert!(
        t.all().iter().any(|n| n.name == second),
        "the pane follows the arrow: {second}
{}",
        t.dump()
    );

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

/// HI-08: Copy as cURL puts the call on the clipboard, without the headers curl sets itself, and a press on a call
/// with no request copied nothing before it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn copy_a_call_as_curl() {
    let app = open_dir(&project("http").unwrap(), 220);
    run_all(&app);
    show_http(&app);
    // the call with no request first: Copy as cURL is off, and a press copies nothing
    select(&app, "refused");
    let t = app
        .wait_for("the refused call", WAIT, |t| {
            tab_is(t, "HTTP (1)") && calls(t).len() == 1
        })
        .unwrap();
    app.click(calls(&t)[0]).unwrap();
    let t = app
        .wait_for("its details", WAIT, |t| t.find(&Role::Button, COPY).is_some())
        .unwrap();
    // a button's disabled state reaches UI Automation only; through AT-SPI a disabled button offers no action
    if cfg!(windows) {
        assert!(!t.find(&Role::Button, COPY).unwrap().states.enabled);
    } else {
        assert!(app.press_named(Role::Button, COPY).is_err());
    }

    select(&app, "ok");
    let t = app
        .wait_for("the ok call", WAIT, |t| tab_is(t, "HTTP (1)") && calls(t).len() == 1)
        .unwrap();
    app.click(calls(&t)[0]).unwrap();
    app.wait_for("the request headers", WAIT, |t| holding(t, "Host:").is_some())
        .unwrap();
    app.press_named(Role::Button, COPY).unwrap();
    // "Copied" lasts two seconds, which a read of the whole tree can miss; the copy itself stays
    app.wait_until("the copy", WAIT, || !app.copies().is_empty()).unwrap();
    let copies = app.copies();
    assert_eq!(copies.len(), 1, "{copies:?}");
    let curl = &copies[0];
    assert!(curl.starts_with("curl -X GET 'http://localhost:"), "{curl}");
    assert!(curl.contains(" -H 'User-Agent: "), "{curl}");
    for set in ["Host", "Content-Length", "Connection"] {
        assert!(
            !curl.to_lowercase().contains(&format!("-h '{}:", set.to_lowercase())),
            "{curl}"
        );
    }
}

/// `SECRET` of `e2e/helpers.ts`: a value that must never reach the screen or the clipboard.
#[cfg(windows)]
const SECRET: &str = "s3cret-token-123";

/// HI-09, E-http-03: a secret property the run sent in a header shows masked in the call and in the cURL text. A secret
/// needs the system's secret storage, which Grace has on Windows only (migration guide R5.3).
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_secret_never_shows_in_a_call() {
    use grace_e2e::properties::{add_property, open_properties, save_properties};
    let dir = project("http").unwrap();
    let app = open_dir(&dir, 220);
    open_properties(&app);
    add_property(&app, "All environments", "api.token", SECRET, true);
    save_properties(&app);
    app.wait_until("the secret saved", WAIT, || {
        let shared = &app.settings()["projects"][dir.to_string_lossy().as_ref()]["properties"]["shared"];
        shared[0]["name"] == "api.token" && shared[0]["secret"].is_string()
    })
    .unwrap();

    // the secret header scenario is the last of its feature: its row comes into view with the tree's arrow keys
    if row(&app.tree().unwrap(), "ok").is_none() {
        app.press_named(Role::Button, "Expand http.feature").unwrap();
        app.wait_for("the scenarios", WAIT, |t| row(t, "ok").is_some()).unwrap();
    }
    select(&app, "ok");
    select_with(&app, "secret header", Key::Down);
    app.press_named(Role::Button, "Run secret header").unwrap();
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
    show_http(&app);
    let t = app
        .wait_for("the call", WAIT, |t| tab_is(t, "HTTP (1)") && calls(t).len() == 1)
        .unwrap();
    app.click(calls(&t)[0]).unwrap();
    app.wait_for("the masked header", WAIT, |t| holding(t, "X-Token: ****").is_some())
        .unwrap();
    app.press_named(Role::Button, COPY).unwrap();
    // "Copied" lasts two seconds, which a read of the whole tree can miss; the copy itself stays
    app.wait_until("the copy", WAIT, || !app.copies().is_empty()).unwrap();

    let copies = app.copies();
    assert_eq!(copies.len(), 1, "{copies:?}");
    assert!(copies[0].contains("-H 'X-Token: ****'"), "{copies:?}");
    assert!(!copies[0].contains(SECRET), "{copies:?}");
    // every name and text in the window: the tree is what Grace shows, to a person and to a screen reader
    assert!(!app.tree().unwrap().dump().contains(SECRET));
}

/// The HTTP list reports its focus row to assistive technology only while it has the focus. With the focus on the
/// window's root, above this list and the tree both, two rows claimed it, which GPUI refuses: a panic in a debug
/// build. Headless windows build no accessibility tree, so only a real window can show it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_click_outside_the_lists_leaves_the_app_running() {
    let app = open("http");
    run_all(&app);
    show_http(&app);
    let t = app.wait_for("the calls", WAIT, |t| calls(t).len() == 13).unwrap();
    // the list has the focus, and a focus row
    app.click(calls(&t)[0]).unwrap();
    let t = app
        .wait_for("the first call selected", WAIT, |t| {
            calls(t).first().is_some_and(|c| c.states.selected)
        })
        .unwrap();
    // text takes no focus: a click on it leaves the focus on the window's root
    app.click(starting(&t, "Finished").unwrap()).unwrap();
    app.wait_for("the window answering", WAIT, |t| {
        calls(t).len() == 13 && starting(t, "Finished").is_some()
    })
    .unwrap();
}
