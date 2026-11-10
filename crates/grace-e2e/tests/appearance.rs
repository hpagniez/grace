//! `e2e/appearance.spec.ts`, through the accessibility tree: Settings › Appearance, the theme in force at start, a
//! change applied at once and kept after a restart, and Escape going back. On Linux the ground's colour is read off
//! the window's pixels; the dialog test and main's refusal of an unknown theme are the plan's Redesigns.
//! They start the real app, so they are ignored by `cargo test --workspace`; `scripts/e2e-gpui.sh` runs them.
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const THEMES: [&str; 3] = ["Follow Windows", "Light", "Dark"];
const LIGHT_GROUND: &str = "f3efe6";
const DARK_GROUND: &str = "151821";

/// Which of the three themes is checked.
fn checked(tree: &Node) -> Vec<bool> {
    THEMES
        .iter()
        .map(|name| {
            tree.find(&Role::RadioButton, name)
                .unwrap_or_else(|| panic!("{name}: {}", tree.dump()))
                .states
                .checked
        })
        .collect()
}

/// Clicks a theme: AT-SPI gives a radio button no action to press.
fn choose(app: &App, name: &str) {
    let radio = app.wait_for_node(Role::RadioButton, name).unwrap();
    app.click(&radio).unwrap();
}

fn open_appearance(app: &App) -> Node {
    app.press_named(Role::Button, "Settings").unwrap();
    app.wait_for("Settings › Appearance", WAIT, |t| {
        t.find(&Role::Group, "Theme").is_some() && THEMES.iter().all(|n| t.find(&Role::RadioButton, n).is_some())
    })
    .unwrap()
}

/// The colour of the window's bottom-left pixel, on the ground under the status bar, as `rrggbb`.
#[cfg(target_os = "linux")]
fn ground() -> Option<String> {
    use std::process::Command;
    let id = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "^Grace$"])
        .output()
        .ok()?;
    let id = String::from_utf8_lossy(&id.stdout).lines().next()?.to_string();
    let geometry = Command::new("xdotool").args(["getwindowgeometry", &id]).output().ok()?;
    let geometry = String::from_utf8_lossy(&geometry.stdout);
    let height: i32 = geometry
        .lines()
        .find_map(|l| l.trim().strip_prefix("Geometry: "))?
        .split('x')
        .nth(1)?
        .trim()
        .parse()
        .ok()?;
    let out = Command::new("import")
        .args(["-window", &id, "-crop", &format!("1x1+1+{}", height - 2), "txt:-"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
    // `srgb(243,239,230)`: the `#` form has 16 bits a channel
    let at = text.rfind("srgb(")?;
    let rgb: Vec<u8> = text[at + 5..]
        .split(')')
        .next()?
        .split(',')
        .map(|c| c.trim().parse().ok())
        .collect::<Option<_>>()?;
    Some(rgb.iter().map(|c| format!("{c:02x}")).collect())
}

#[cfg(not(target_os = "linux"))]
fn ground() -> Option<String> {
    None
}

/// Waits for the ground to take `rgb`; a platform with no pixel reader skips the check.
fn wait_ground(app: &App, rgb: &str) {
    if ground().is_none() {
        return;
    }
    app.wait_until(&format!("the ground #{rgb}"), WAIT, || ground().as_deref() == Some(rgb))
        .unwrap();
}

fn no_project() -> App {
    let app = App::launch_with_settings(&json!({}), &[]).unwrap();
    app.wait_for("the home screen", WAIT, |t| {
        t.all().into_iter().any(|n| n.name.starts_with("Open folder"))
    })
    .unwrap();
    app
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn follow_windows_is_the_default() {
    let app = no_project();
    let tree = open_appearance(&app);
    assert_eq!(checked(&tree), [true, false, false]);
    assert!(app.settings().get("appearance").is_none(), "{}", app.settings());
    // `Settings.test.tsx`: the page's structure, its entry for the open section selected
    let entry = tree
        .find(&Role::Button, "Appearance")
        .unwrap_or_else(|| panic!("{}", tree.dump()));
    if cfg!(not(windows)) {
        // UI Automation reads a selection only through a selection item, which a button is not
        assert!(entry.states.selected, "{}", tree.dump());
        // AT-SPI reads headings; the navigation and the region are landmarks there
        let named = |role: Role, name: &str| tree.all().into_iter().any(|n| n.role == role && n.name == name);
        assert!(named(Role::Other("Heading".into()), "Settings"), "{}", tree.dump());
        assert!(named(Role::Other("Heading".into()), "Appearance"), "{}", tree.dump());
        assert!(named(Role::Landmark, "Settings sections"), "{}", tree.dump());
    }
    // Xvfb has no dark preference: Follow Windows shows the light theme
    wait_ground(&app, LIGHT_GROUND);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_saved_dark_is_in_force_at_start() {
    let app = App::launch_with_settings(&json!({ "appearance": { "theme": "dark" } }), &[]).unwrap();
    app.wait_for("the home screen", WAIT, |t| {
        t.all().into_iter().any(|n| n.name.starts_with("Open folder"))
    })
    .unwrap();
    wait_ground(&app, DARK_GROUND);
    let tree = open_appearance(&app);
    assert_eq!(checked(&tree), [false, false, true]);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn choosing_a_theme_applies_at_once_with_a_project_open_and_survives_a_restart() {
    let dir = project("clean").unwrap();
    let app = App::launch_with_settings(&json!({ "lastProject": dir.to_string_lossy() }), &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| {
        t.all().into_iter().any(|n| n.name.starts_with("tests "))
    })
    .unwrap();
    // a state of the view that Back must find again
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    app.wait_for_node(Role::Button, "Collapse basic.feature").unwrap();

    open_appearance(&app);
    choose(&app, "Dark");
    wait_ground(&app, DARK_GROUND);
    app.wait_for("Dark checked", WAIT, |t| checked(t) == [false, false, true])
        .unwrap();
    app.wait_until("Dark saved", WAIT, || {
        app.settings()["appearance"] == json!({ "theme": "dark" })
    })
    .unwrap();

    choose(&app, "Light");
    wait_ground(&app, LIGHT_GROUND);
    app.wait_until("Light saved", WAIT, || {
        app.settings()["appearance"] == json!({ "theme": "light" })
    })
    .unwrap();

    app.press_named(Role::Button, "Back").unwrap();
    app.wait_for("the project as it was", WAIT, |t| {
        t.find(&Role::Group, "Theme").is_none() && t.find(&Role::Button, "Collapse basic.feature").is_some()
    })
    .unwrap();

    let again = app.relaunch(&[]).unwrap();
    wait_ground(&again, LIGHT_GROUND);
    let tree = open_appearance(&again);
    assert_eq!(checked(&tree), [false, true, false]);
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn escape_goes_back_from_the_settings_page() {
    let app = no_project();
    open_appearance(&app);
    app.key(Key::Escape).unwrap();
    app.wait_for("the home screen again", WAIT, |t| {
        t.find(&Role::Group, "Theme").is_none() && t.all().into_iter().any(|n| n.name.starts_with("Open folder"))
    })
    .unwrap();
    // and again from a radio button, which holds the focus
    open_appearance(&app);
    choose(&app, "Follow Windows");
    app.key(Key::Escape).unwrap();
    app.wait_for("the home screen again", WAIT, |t| {
        t.find(&Role::Group, "Theme").is_none()
    })
    .unwrap();
}
