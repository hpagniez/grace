//! Tooltips (grace::tooltip) through the accessibility tree, in the `e2e` build's gallery window: parity PU-19 to
//! PU-21 and cut text. The pointer and the keys are real input, so these move the mouse on the test's display.
//! Ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role};

fn gallery() -> App {
    App::launch_with(&[("GRACE_E2E_VIEW", "elements")]).unwrap()
}

fn has_dialog(tree: &Node) -> bool {
    tree.all().iter().any(|n| n.role == Role::Dialog)
}

/// Tabs until the node with this role and name holds the focus, at most 30 times.
fn tab_to(app: &App, role: Role, name: &str) {
    let mut seen = Vec::new();
    for _ in 0..30 {
        let tree = app.tree().unwrap();
        seen.push(
            tree.all()
                .iter()
                .find(|n| n.states.focused)
                .map(|n| format!("{:?} {:?}", n.role, n.name))
                .unwrap_or_default(),
        );
        if tree
            .all()
            .iter()
            .any(|n| n.role == role && n.name == name && n.states.focused)
        {
            return;
        }
        app.key(Key::Tab).unwrap();
        // the next read waits for the focus to move, as a person would see it
        let _ = app.wait_for("the focus to move", Duration::from_millis(500), |t| {
            t.all().iter().any(|n| n.states.focused)
        });
    }
    panic!("Tab never reached {role:?} {name:?}; the focus went {seen:?}");
}

/// PU-20: a tooltip shows after a pause on hover, also on a disabled button.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn shows_on_hover_also_over_a_disabled_button() {
    let app = gallery();
    let run = app.wait_for_node(Role::Button, "Run all").unwrap();
    app.hover(&run).unwrap();
    app.wait_for_node(Role::Tooltip, "Open a project first").unwrap();
    let elsewhere = app.wait_for_node(Role::Button, "Pressed 0 times").unwrap();
    app.hover(&elsewhere).unwrap();
    app.wait_for_no_node(Role::Tooltip, "Open a project first").unwrap();
}

/// PU-19: a tooltip shows on keyboard focus and closes with Escape.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn shows_on_keyboard_focus_and_closes_with_escape() {
    let app = gallery();
    let anywhere = app.wait_for_node(Role::Button, "Pressed 0 times").unwrap();
    app.hover(&anywhere).unwrap();
    tab_to(&app, Role::Button, "Settings");
    app.wait_for_node(Role::Tooltip, "Settings").unwrap();
    app.key(Key::Escape).unwrap();
    app.wait_for_no_node(Role::Tooltip, "Settings").unwrap();
}

/// PU-21: Escape in a dialog closes the tooltip first, then the dialog.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn escape_in_a_dialog_closes_the_tooltip_first() {
    let app = gallery();
    let open = app.wait_for_node(Role::Button, "Open dialog").unwrap();
    app.hover(&open).unwrap();
    app.press(&open).unwrap();
    app.wait_for("the dialog", Duration::from_secs(10), has_dialog).unwrap();
    tab_to(&app, Role::Button, "In dialog");
    app.wait_for_node(Role::Tooltip, "Does nothing").unwrap();
    app.key(Key::Escape).unwrap();
    let tree = app.wait_for_no_node(Role::Tooltip, "Does nothing").unwrap();
    assert!(
        has_dialog(&tree),
        "the first Escape closed the dialog too:\n{}",
        tree.dump()
    );
    app.key(Key::Escape).unwrap();
    app.wait_for("the dialog to close", Duration::from_secs(10), |t| !has_dialog(t))
        .unwrap();
}

/// A line too long for its room shows whole in a tooltip (`Cut.tsx`).
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn shows_cut_text_whole() {
    let app = gallery();
    let full = "A project name far too long to fit in its row";
    let tree = app
        .wait_for("the cut line", Duration::from_secs(10), |t| {
            t.all().iter().any(|n| n.name == full && n.role != Role::Tooltip)
        })
        .unwrap();
    let line = tree
        .all()
        .into_iter()
        .find(|n| n.name == full && n.rect.is_some())
        .cloned()
        .unwrap_or_else(|| panic!("the cut line has no place on the screen:\n{}", tree.dump()));
    app.hover(&line).unwrap();
    app.wait_for_node(Role::Tooltip, full).unwrap();
}
