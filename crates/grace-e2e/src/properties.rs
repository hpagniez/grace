//! The Properties dialog as the tests that fill it share it (`env.spec.ts`'s `addProperty` and its neighbours): the
//! dialog's environments are tabs in a tab list named "Environments"; each property is a list item named by its name.
use std::time::Duration;

use crate::{App, Node, Role};

const WAIT: Duration = Duration::from_secs(30);

/// The dialog's property row named `name`.
pub fn property<'a>(tree: &'a Node, name: &str) -> Option<&'a Node> {
    tree.all()
        .into_iter()
        .find(|n| n.role == Role::ListItem && n.name == name)
}

/// A control inside a node: its first descendant with this name.
pub fn inside<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    node.all().into_iter().skip(1).find(|n| n.name == name)
}

pub fn open_properties(app: &App) {
    app.press_named(Role::Button, "Properties…").unwrap();
    app.wait_for("the dialog", WAIT, |t| t.find(&Role::TabList, "Environments").is_some())
        .unwrap();
}

/// Shows the list of `scope`, "All environments" or an environment.
pub fn choose_scope(app: &App, scope: &str) {
    app.press_named(Role::Tab, scope).unwrap();
    app.wait_for(&format!("{scope} chosen"), WAIT, |t| {
        t.find(&Role::Tab, scope).is_some_and(|n| n.states.selected)
    })
    .unwrap();
}

/// Adds a row to the list of `scope` (`addProperty`). Add property puts the focus in the new row's name (the view's
/// tests prove it), but only in an active window, and a window that only had actions pressed through the tree may not
/// be one under Xvfb: a click on the name makes it so.
pub fn add_property(app: &App, scope: &str, name: &str, value: &str, secret: bool) {
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

pub fn save_properties(app: &App) {
    app.press_named(Role::Button, "Save").unwrap();
    app.wait_for("the dialog closed", WAIT, |t| {
        t.find(&Role::TabList, "Environments").is_none()
    })
    .unwrap();
}
