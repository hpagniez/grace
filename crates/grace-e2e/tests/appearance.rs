//! `e2e/appearance.spec.ts`, through the accessibility tree: Settings › Appearance, the theme in force at start, a
//! change applied at once and kept after a restart, and Escape going back. On Linux the ground's colour is read off
//! the window's pixels, on Windows the title bar's dark mode, which follows Grace's theme rather than Windows' (PU-14,
//! #337); the dialog test and main's refusal of an unknown theme are the plan's Redesigns.
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

/// Whether a colour `0xrrggbb` is dark: its luma under half.
fn is_dark(rgb: u32) -> bool {
    let (r, g, b) = ((rgb >> 16) & 0xff, (rgb >> 8) & 0xff, rgb & 0xff);
    2 * r + 5 * g + b < 8 * 128
}

/// The title bar as (asked dark, drawn dark): its dark mode attribute, and the colour the screen shows in it.
fn title_bar(app: &App) -> Option<(bool, bool)> {
    Some((app.dark_title_bar().ok()??, is_dark(app.title_bar_colour().ok()??)))
}

/// Waits for the title bar to be asked and drawn dark or light, where the platform says which (Windows); elsewhere
/// skips the check. An error is a window not there yet, after a restart: the wait goes on.
fn wait_title_bar(app: &App, dark: bool) {
    if let Ok(None) = app.dark_title_bar() {
        return;
    }
    let what = if dark { "a dark title bar" } else { "a light title bar" };
    app.wait_until(what, WAIT, || title_bar(app) == Some((dark, dark)))
        .unwrap_or_else(|e| {
            panic!(
                "{e}: asked dark {:?}, drawn #{:06x}",
                app.dark_title_bar(),
                app.title_bar_colour().ok().flatten().unwrap_or(0)
            )
        });
}

/// Windows' own theme, switched as Settings › Personalisation › Colours › Choose your mode does: the registry values
/// for apps and for Windows, then the broadcast Windows sends. Put back when dropped. Only on CI's runner: a test never
/// changes the theme of a person's machine.
#[cfg(windows)]
struct WindowsTheme {
    was: [bool; 2],
}

#[cfg(windows)]
const PERSONALIZE: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

#[cfg(windows)]
const MODES: [&str; 2] = ["AppsUseLightTheme", "SystemUsesLightTheme"];

#[cfg(windows)]
impl WindowsTheme {
    /// Whether apps or Windows are dark: a value of 0, where a missing value is light.
    fn dark(name: &str) -> bool {
        let out = std::process::Command::new("reg")
            .args(["query", PERSONALIZE, "/v", name])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains("0x0")
    }

    fn is_dark() -> bool {
        Self::dark(MODES[0])
    }

    fn set(dark: [bool; 2]) {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
        };
        use windows::core::w;
        for (name, dark) in MODES.iter().zip(dark) {
            let value = if dark { "0" } else { "1" };
            let added = std::process::Command::new("reg")
                .args(["add", PERSONALIZE, "/v", name, "/t", "REG_DWORD", "/d", value, "/f"])
                .status()
                .unwrap();
            assert!(added.success(), "reg add {name}");
        }
        // SAFETY: the string is static, and the call waits at most 5 s for each window
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                WPARAM(0),
                LPARAM(w!("ImmersiveColorSet").as_ptr() as isize),
                SMTO_ABORTIFHUNG,
                5000,
                None,
            );
        }
    }

    /// Switches both to the other mode of the apps'.
    fn switch() -> Self {
        let was = MODES.map(Self::dark);
        Self::set([!was[0]; 2]);
        WindowsTheme { was }
    }
}

#[cfg(windows)]
impl Drop for WindowsTheme {
    fn drop(&mut self) {
        Self::set(self.was);
    }
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
    wait_title_bar(&app, true);
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
    wait_title_bar(&app, true);
    app.wait_for("Dark checked", WAIT, |t| checked(t) == [false, false, true])
        .unwrap();
    app.wait_until("Dark saved", WAIT, || {
        app.settings()["appearance"] == json!({ "theme": "dark" })
    })
    .unwrap();

    choose(&app, "Light");
    wait_ground(&app, LIGHT_GROUND);
    wait_title_bar(&app, false);
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
    wait_title_bar(&again, false);
    let tree = open_appearance(&again);
    assert_eq!(checked(&tree), [false, true, false]);
}

/// PU-14 with Windows' own theme switched while Grace is open: GPUI sets the title bar from Windows' new theme, and
/// Grace's own must win, as Electron Grace's does. Only on CI's Windows runner, where the test may switch the machine's
/// theme.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_switch_of_windows_theme_leaves_graces_on_the_title_bar() {
    #[cfg(windows)]
    if std::env::var("CI").as_deref() == Ok("true") {
        windows_switches_under_grace();
    }
}

#[cfg(windows)]
fn windows_switches_under_grace() {
    // Grace on the theme Windows is switched away from
    let to_dark = !WindowsTheme::is_dark();
    let grace_dark = !to_dark;
    let theme = if grace_dark { "dark" } else { "light" };
    let app = App::launch_with_settings(&json!({ "appearance": { "theme": theme } }), &[]).unwrap();
    app.wait_for("the home screen", WAIT, |t| {
        t.all().into_iter().any(|n| n.name.starts_with("Open folder"))
    })
    .unwrap();
    wait_title_bar(&app, grace_dark);

    let switched = std::time::Instant::now();
    let _theme = WindowsTheme::switch();
    // what the title bar was asked and drew over 4 s, each change once
    let seen = std::cell::RefCell::new(Vec::<(u128, Option<bool>, Option<u32>)>::new());
    let _ = app.wait_until("4 s after the switch", Duration::from_secs(5), || {
        let now = (
            app.dark_title_bar().ok().flatten(),
            app.title_bar_colour().ok().flatten(),
        );
        let mut seen = seen.borrow_mut();
        if seen.last().map(|&(_, a, c)| (a, c)) != Some(now) {
            seen.push((switched.elapsed().as_millis(), now.0, now.1));
        }
        switched.elapsed() >= Duration::from_secs(4)
    });
    let seen = seen.into_inner();
    let report = seen
        .iter()
        .map(|(ms, a, c)| format!("{ms} ms: asked dark {a:?}, drawn #{:06x}", c.unwrap_or(0)))
        .collect::<Vec<_>>()
        .join("; ");
    let (_, asked, drawn) = *seen.last().unwrap();
    assert_eq!(
        (asked, drawn.map(is_dark)),
        (Some(grace_dark), Some(grace_dark)),
        "Windows switched to {}: {report}",
        if to_dark { "dark" } else { "light" }
    );
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
