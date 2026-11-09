//! The shared elements (grace::elements) through the accessibility tree, in the `e2e` build's gallery window.
//! Ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Role};

fn gallery() -> App {
    App::launch_with(&[("GRACE_E2E_VIEW", "elements")]).unwrap()
}

/// Guide R8.2: every interactive element has a name.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn names_every_control() {
    let app = gallery();
    app.wait_for_node(Role::Button, "Pressed 0 times").unwrap();
    let tree = app.tree().unwrap();
    assert!(
        tree.all().iter().filter(|n| n.role == Role::Button).count() >= 15,
        "{}",
        tree.dump()
    );
    assert_eq!(tree.unnamed_controls(), Vec::<String>::new(), "{}", tree.dump());
}

/// PU-08: each status reads as its name, not its colour.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn names_each_status_mark() {
    let app = gallery();
    let tree = app
        .wait_for("the status marks", Duration::from_secs(15), |t| {
            t.find(&Role::Image, "Paused").is_some()
        })
        .unwrap();
    for name in [
        "Not run",
        "Pending",
        "Running",
        "Passed",
        "Failed",
        "Interrupted",
        "Cancelled",
        "Skipped",
        "Paused",
    ] {
        assert!(tree.find(&Role::Image, name).is_some(), "{name}\n{}", tree.dump());
    }
}

/// An icon-only button is named by its label, as `btnIcon`'s aria-label does.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn names_icon_buttons_by_their_label() {
    let app = gallery();
    app.wait_for_node(Role::Button, "Settings").unwrap();
    app.wait_for_node(Role::Button, "Delete").unwrap();
}

/// The driver presses a button as a person would, and the app sees it.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn presses_a_button() {
    let app = gallery();
    let counter = app.wait_for_node(Role::Button, "Pressed 0 times").unwrap();
    app.press(&counter).unwrap();
    app.wait_for_node(Role::Button, "Pressed 1 time").unwrap();
}
