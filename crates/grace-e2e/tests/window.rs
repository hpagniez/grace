//! The empty window of phase P0, through the accessibility tree. These tests start the real app, so they are
//! ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` builds Grace with the `e2e` feature and runs them.
use std::time::Duration;

use grace_e2e::{App, Node, Role};

/// The window, once Grace's own root (named Grace, inside the window named Grace) is in it.
fn grace_inside(tree: &Node) -> bool {
    tree.children.iter().any(|c| c.all().iter().any(|n| n.name == "Grace"))
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn opens_one_window_named_grace() {
    let app = App::launch().unwrap();
    let tree = app
        .wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    assert_eq!(tree.role, Role::Window, "{}", tree.dump());
    assert_eq!(tree.name, "Grace", "{}", tree.dump());
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn has_a_live_region_for_announcements() {
    let app = App::launch().unwrap();
    let tree = app
        .wait_for("the live region", Duration::from_secs(15), |t| {
            t.all().iter().any(|n| n.role == Role::Status)
        })
        .unwrap();
    assert!(tree.all().iter().any(|n| n.role == Role::Status), "{}", tree.dump());
}

/// Guide R8.2: every interactive element of the window has a name.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn names_every_control() {
    let app = App::launch().unwrap();
    let tree = app
        .wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    assert_eq!(tree.unnamed_controls(), Vec::<String>::new(), "{}", tree.dump());
}
