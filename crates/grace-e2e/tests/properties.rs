//! `e2e/env.spec.ts`'s tests of the run properties (EV-06 to EV-11, E-env-06 to E-env-12, EP-26), through the
//! accessibility tree. The dialog's environments are tabs in a tab list named "Environments"; each property is a
//! list item named by its name in the list "Properties".
//!
//! A secret needs the system's secret storage, which Grace has on Windows only (migration guide R5.3): the tests that
//! save one are Windows', and on Linux `without_secret_storage_no_secret_can_be_saved` checks the dialog says so.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::path::{Path, PathBuf};
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(60);
const ENV: &str = "Env";
/// `SECRET` of `e2e/helpers.ts`: a value that must never reach the screen, a command line or the settings file.
#[cfg(windows)]
const SECRET: &str = "s3cret-token-123";

fn row<'a>(tree: &'a Node, label: &str) -> Option<&'a Node> {
    let start = format!("{label}, ");
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with(&start))
}

fn says(tree: &Node, label: &str, status: &str) -> bool {
    row(tree, label)
        .is_some_and(|r| r.name == format!("{label}, {status}") || r.name.starts_with(&format!("{label}, {status}, ")))
}

/// A node anywhere with this name, of any role: a label's name is its text.
fn named<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name == name)
}

/// The dialog's property row named `name`.
fn property<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::ListItem && n.name == name)
}

/// A control inside a node: its first descendant with this name.
fn inside<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    node.all().into_iter().skip(1).find(|n| n.name == name)
}

fn open_env(settings: serde_json::Value) -> (PathBuf, App) {
    let dir = project("env").unwrap();
    let mut s = settings;
    s["lastProject"] = json!(dir.to_string_lossy());
    let app = App::launch_with_settings(&s, &[]).unwrap();
    open_features(&app);
    (dir, app)
}

fn open_features(app: &App) {
    app.wait_for("the tests found", WAIT, |t| {
        t.find(&Role::Button, "Expand env.feature").is_some()
    })
    .unwrap();
    app.press_named(Role::Button, "Expand env.feature").unwrap();
    app.wait_for("the feature open", WAIT, |t| row(t, "always").is_some())
        .unwrap();
}

fn saved(app: &App, dir: &Path) -> serde_json::Value {
    app.settings()["projects"][dir.to_string_lossy().as_ref()].clone()
}

fn choose_env(app: &App, dir: &Path, name: &str) {
    app.fill(Role::ComboBox, ENV, name).unwrap();
    app.key(Key::Enter).unwrap();
    let want = if name == "(none)" { json!(null) } else { json!(name) };
    app.wait_until(&format!("{name} saved"), WAIT, || saved(app, dir)["env"] == want)
        .unwrap();
}

fn open_properties(app: &App) {
    app.press_named(Role::Button, "Properties…").unwrap();
    app.wait_for("the dialog", WAIT, |t| t.find(&Role::TabList, "Environments").is_some())
        .unwrap();
}

/// Shows the list of `scope`, "All environments" or an environment.
fn choose_scope(app: &App, scope: &str) {
    app.press_named(Role::Tab, scope).unwrap();
    app.wait_for(&format!("{scope} chosen"), WAIT, |t| {
        t.find(&Role::Tab, scope).is_some_and(|n| n.states.selected)
    })
    .unwrap();
}

/// Adds a row to the list of `scope` (`addProperty`). Add property puts the focus in the new row's name (the view's
/// tests prove it), but only in an active window, and a window that only had actions pressed through the tree may not
/// be one under Xvfb: a click on the name makes it so.
fn add_property(app: &App, scope: &str, name: &str, value: &str, secret: bool) {
    choose_scope(app, scope);
    app.press_named(Role::Button, "Add property").unwrap();
    let tree = app
        .wait_for("the new row", WAIT, |t| property(t, "New property").is_some())
        .unwrap();
    let name_box = inside(property(&tree, "New property").unwrap(), "Property name")
        .unwrap()
        .clone();
    app.click(&name_box).unwrap();
    app.wait_for("the new row's name to take the focus", WAIT, |t| {
        property(t, "New property")
            .and_then(|r| inside(r, "Property name"))
            .is_some_and(|n| n.states.focused)
    })
    .unwrap();
    app.type_text(name).unwrap();
    let tree = app
        .wait_for(&format!("the row {name}"), WAIT, |t| property(t, name).is_some())
        .unwrap();
    let r = property(&tree, name).unwrap();
    if secret {
        app.press(inside(r, "Secret").unwrap()).unwrap();
    }
    let value_box = inside(r, "Property value").unwrap().clone();
    app.click(&value_box).unwrap();
    app.wait_for("the value box to take the focus", WAIT, |t| {
        property(t, name)
            .and_then(|r| inside(r, "Property value"))
            .is_some_and(|n| n.states.focused)
    })
    .unwrap();
    app.type_text(value).unwrap();
}

fn save_properties(app: &App) {
    app.press_named(Role::Button, "Save").unwrap();
    app.wait_for("the dialog closed", WAIT, |t| {
        t.find(&Role::TabList, "Environments").is_none()
    })
    .unwrap();
}

fn run_always(app: &App) {
    app.press_named(Role::Button, "Run always").unwrap();
    app.wait_for("always ran", RUN, |t| says(t, "always", "passed"))
        .unwrap();
}

/// Karate's log of the newest run, from its events file: the console comes with P2's fifth PR.
fn run_log(app: &App) -> String {
    let events = newest_events(app.data_dir()).expect("a run's events");
    std::fs::read_to_string(events).unwrap_or_default()
}

fn newest_events(dir: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.file_name().is_some_and(|n| n == "karate-events.jsonl") {
                out.push(p);
            }
        }
    }
    let mut all = Vec::new();
    walk(dir, &mut all);
    all.into_iter()
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}

/// EV-06, E-env-06
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn shared_and_environment_properties_environment_wins() {
    let (dir, app) = open_env(json!({}));
    choose_env(&app, &dir, "qa");
    open_properties(&app);
    add_property(&app, "All environments", "timeout", "5000", false);
    add_property(&app, "qa", "timeout", "9000", false);
    save_properties(&app);
    app.wait_until("the properties saved", WAIT, || {
        saved(&app, &dir)["properties"]
            == json!({
                "shared": [{ "name": "timeout", "value": "5000" }],
                "byEnv": { "qa": [{ "name": "timeout", "value": "9000" }] },
            })
    })
    .unwrap();

    run_always(&app);
    assert!(run_log(&app).contains("timeout = 9000"));

    choose_env(&app, &dir, "(none)");
    app.wait_for("qa's tests gone", WAIT, |t| row(t, "qa only").is_none())
        .unwrap();
    run_always(&app);
    let log = run_log(&app);
    assert!(log.contains("timeout = 5000") && !log.contains("timeout = 9000"));
}

/// EV-09, E-env-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn reserved_names_are_refused() {
    let (dir, app) = open_env(json!({}));
    open_properties(&app);
    add_property(&app, "All environments", "karate.options", "-t @x", false);
    app.wait_for("the error", WAIT, |t| named(t, "Reserved name").is_some())
        .unwrap();
    // a button's disabled state reaches UI Automation only; through AT-SPI a disabled button offers no action
    if cfg!(windows) {
        let tree = app.tree().unwrap();
        assert!(!tree.find(&Role::Button, "Save").unwrap().states.enabled);
    } else {
        assert!(app.press_named(Role::Button, "Save").is_err());
    }
    assert!(app.tree().unwrap().find(&Role::TabList, "Environments").is_some());
    assert!(saved(&app, &dir)["properties"].is_null());
}

/// EV-10, E-env-11: a saved secret that cannot be read blocks runs before any Java starts
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn an_unreadable_secret_blocks_runs() {
    let dir = project("env").unwrap();
    let settings = json!({
        "lastProject": dir.to_string_lossy(),
        "projects": {
            dir.to_string_lossy(): {
                "env": "qa",
                "properties": {
                    "shared": [],
                    "byEnv": { "qa": [{ "name": "api.token", "secret": "bm90LWEtcmVhbC1jaXBoZXJ0ZXh0" }] },
                },
            },
        },
    });
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    open_features(&app);
    app.wait_for_text(Role::ComboBox, ENV, "qa").unwrap();

    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the notice", WAIT, |t| {
        named(t, "Secret api.token (qa) can't be read. Replace it in Properties.").is_some()
    })
    .unwrap();
    assert!(says(&app.tree().unwrap(), "always", "not run"));
    assert!(app.java_pids().is_empty(), "refused before the pre-run dry run");

    open_properties(&app);
    choose_scope(&app, "qa");
    app.wait_for("the row says so", WAIT, |t| {
        property(t, "api.token").is_some_and(|r| inside(r, "Can't be read, enter it again").is_some())
    })
    .unwrap();
}

/// EP-25 on Linux, which has no secret storage yet (guide R5.3): no row can become a secret, no file be imported
#[cfg(not(windows))]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn without_secret_storage_no_secret_can_be_saved() {
    let (_, app) = open_env(json!({}));
    open_properties(&app);
    app.wait_for("the warning", WAIT, |t| {
        named(t, "Secret storage is not available on this system").is_some()
    })
    .unwrap();
}

/// `openWithSecret`: the env project in qa with the secret api.token saved for it.
#[cfg(windows)]
fn open_with_secret() -> (PathBuf, App) {
    let (dir, app) = open_env(json!({}));
    choose_env(&app, &dir, "qa");
    open_properties(&app);
    add_property(&app, "qa", "api.token", SECRET, true);
    save_properties(&app);
    app.wait_until("the secret saved", WAIT, || {
        let qa = &saved(&app, &dir)["properties"]["byEnv"]["qa"];
        qa[0]["name"] == "api.token" && qa[0]["secret"].is_string()
    })
    .unwrap();
    (dir, app)
}

/// Every name in the window: the tree is what Grace shows, to a person and to a screen reader.
#[cfg(windows)]
fn everything(app: &App) -> String {
    app.tree().unwrap().dump()
}

/// The qa list's api.token row shows a saved secret: dots, Replace, no value box.
#[cfg(windows)]
fn shows_a_saved_secret(app: &App) {
    open_properties(app);
    choose_scope(app, "qa");
    let tree = app
        .wait_for("api.token", WAIT, |t| property(t, "api.token").is_some())
        .unwrap();
    let r = property(&tree, "api.token").unwrap();
    assert!(inside(r, "Saved secret").is_some(), "{}", r.dump());
    assert!(inside(r, "Replace").is_some());
    assert!(inside(r, "Property value").is_none());
    assert!(inside(r, "Can't be read, enter it again").is_none());
}

/// EV-07, E-env-07
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_secret_is_used_but_never_shown() {
    let (_, app) = open_with_secret();
    run_always(&app);
    // Karate's own log holds what it printed; the console masks it, which `runner::tests::properties` checks and the
    // console's e2e test with P2's fifth PR
    assert!(
        run_log(&app).contains(&format!("api.token = {SECRET}")),
        "the run got it"
    );
    assert!(!everything(&app).contains(SECRET));
    shows_a_saved_secret(&app);
    assert!(!everything(&app).contains(SECRET));
}

/// EV-08, E-env-09
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_secret_stays_off_the_command_line_and_out_of_the_settings_file() {
    let (_, app) = open_with_secret();
    app.press_named(Role::Button, "Run slow").unwrap();
    // the pre-run dry run has no argument file; the run itself has one
    let lines = std::cell::RefCell::new(Vec::new());
    app.wait_until("the run's Java", WAIT, || {
        let now = app.java_command_lines();
        let found = now.iter().any(|l| l.contains("run-args.txt"));
        if found {
            *lines.borrow_mut() = now;
        }
        found
    })
    .unwrap();
    for line in lines.borrow().iter() {
        assert!(!line.contains(SECRET), "{line}");
    }
    let settings = std::fs::read_to_string(app.data_dir().join("settings.json")).unwrap();
    assert!(!settings.contains(SECRET));
    app.wait_for("slow ran", RUN, |t| says(t, "slow", "passed")).unwrap();
}

/// EV-11's properties, E-env-12
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_properties_survive_a_restart() {
    let (_, app) = open_with_secret();
    let app = app.relaunch(&[]).unwrap();
    app.wait_for_text(Role::ComboBox, ENV, "qa").unwrap();
    open_features(&app);
    shows_a_saved_secret(&app);
}

/// EP-26, E-env-08
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn import_a_secrets_file_and_run_with_it() {
    let (dir, app) = open_env(json!({}));
    choose_env(&app, &dir, "qa");
    let folder = grace_e2e::fresh_data_dir().unwrap();
    let file = folder.join("secrets-qa.properties");
    let text = format!("# kept out of the repository\napi.token={SECRET}\n");
    std::fs::write(&file, &text).unwrap();
    app.answer_file(Some(&file)).unwrap();

    open_properties(&app);
    choose_scope(&app, "qa"); // the step starts on the list shown
    app.press_named(Role::Button, "Import from file…").unwrap();
    app.wait_for("the import step", WAIT, |t| {
        named(t, "Import secrets-qa.properties").is_some()
    })
    .unwrap();
    app.fill(Role::Entry, "Prefix", "APP_").unwrap();
    app.wait_for("the preview", WAIT, |t| named(t, "api.token → APP_API_TOKEN").is_some())
        .unwrap();
    app.press_named(Role::Button, "Import").unwrap();
    app.wait_for("the row", WAIT, |t| property(t, "APP_API_TOKEN").is_some())
        .unwrap();
    save_properties(&app);
    app.wait_until("the secret saved", WAIT, || {
        let qa = &saved(&app, &dir)["properties"]["byEnv"]["qa"];
        qa[0]["name"] == "APP_API_TOKEN" && qa[0]["secret"].is_string() && qa[0]["envVar"] == true
    })
    .unwrap();
    let settings = std::fs::read_to_string(app.data_dir().join("settings.json")).unwrap();
    assert!(!settings.contains(SECRET));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);

    run_always(&app);
    assert!(
        run_log(&app).contains(&format!("APP_API_TOKEN = {SECRET}")),
        "the run got it"
    );
    assert!(!everything(&app).contains(SECRET));
}
