//! Screenshots of GPUI Grace for a pull request: each test drives the app to a state and saves the window as a PNG
//! in `GRACE_SCREENS`. Run on Linux with `GRACE_SCREENS="$PWD/target/screens" sh scripts/e2e-gpui.sh screens_`: the
//! `screens` feature, which the e2e suite leaves out, since a screenshot checks nothing.
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use grace_e2e::{App, Key, Node, Role, project};
use serde_json::json;

const WAIT: Duration = Duration::from_secs(30);
const RUN: Duration = Duration::from_secs(90);

fn out() -> PathBuf {
    let dir = PathBuf::from(std::env::var("GRACE_SCREENS").expect("GRACE_SCREENS, the folder for the screenshots"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Saves Grace's window, once the tree has settled, as `name`.png.
fn shot(app: &App, name: &str) {
    // the frame that follows the last change: read the tree twice, the window is drawn by then
    let _ = app.tree();
    let _ = app.tree();
    capture(&out().join(format!("{name}.png")));
}

#[cfg(not(windows))]
fn capture(path: &std::path::Path) {
    let id = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "^Grace$"])
        .output()
        .unwrap();
    let id = String::from_utf8_lossy(&id.stdout).lines().next().unwrap().to_string();
    let ok = Command::new("import")
        .args(["-window", &id, path.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(ok.success());
}

/// On Windows, with the owner's go (docs/process/owner-machine.md): the screen under the window "Grace", in its
/// frame's own pixels, copied by PowerShell.
#[cfg(windows)]
fn capture(path: &std::path::Path) {
    const SCRIPT: &str = r#"
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class Win {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindowW(string c, string t);
  [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int size);
  public static int[] Frame(IntPtr h) {
    R r;
    DwmGetWindowAttribute(h, 9, out r, 16);
    return new int[] { r.L, r.T, r.Rt - r.L, r.B - r.T };
  }
}
'@
[Win]::SetProcessDPIAware() | Out-Null
$h = [Win]::FindWindowW([NullString]::Value, 'Grace')
$f = [Win]::Frame($h)
$bmp = New-Object System.Drawing.Bitmap $f[2], $f[3]
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($f[0], $f[1], 0, 0, $bmp.Size)
$bmp.Save($args[0], [System.Drawing.Imaging.ImageFormat]::Png)
"#;
    let script = path.with_extension("ps1");
    std::fs::write(&script, SCRIPT).unwrap();
    let ok = Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .arg(path)
        .status()
        .unwrap();
    let _ = std::fs::remove_file(&script);
    assert!(ok.success());
}

/// `name`, or `name-dark` for the dark look.
fn themed(name: &str, dark: bool) -> String {
    if dark { format!("{name}-dark") } else { name.to_string() }
}

/// `settings` with the dark look in it when `dark`.
fn look(mut settings: serde_json::Value, dark: bool) -> serde_json::Value {
    if dark {
        settings["appearance"] = json!({ "theme": "dark" });
    }
    settings
}

fn starting<'a>(tree: &'a Node, start: &str) -> Option<&'a Node> {
    tree.all().into_iter().find(|n| n.name.starts_with(start))
}

fn open(dir: &std::path::Path, dark: bool) -> App {
    let mut settings = json!({ "lastProject": dir.to_string_lossy() });
    if dark {
        settings["appearance"] = json!({ "theme": "dark" });
    }
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| starting(t, "tests ").is_some())
        .unwrap();
    app
}

fn run_all(app: &App) {
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
}

fn one_feature(text: &str) -> PathBuf {
    let dir = grace_e2e::fresh_data_dir().unwrap().join("checkout");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("checkout.feature"), text).unwrap();
    dir
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_console() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    shot(&app, "console-ready");
    run_all(&app);
    shot(&app, "console-after-a-run");
    app.key(Key::CtrlF).unwrap();
    app.fill(Role::Entry, "Find in console", "INFO").unwrap();
    app.wait_for("matches", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.role == Role::Status && n.name.contains(" of "))
    })
    .unwrap();
    app.key(Key::Enter).unwrap();
    shot(&app, "console-find");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_console_errors() {
    let dir = one_feature(
        "Feature: checkout\n\nScenario: totals add up\n  * def total = 2 + 2\n  * match total == 5\n\nScenario: payment logs a problem\n  * karate.logger.error('card declined')\n  * print 'retrying'\n",
    );
    let app = open(&dir, false);
    run_all(&app);
    app.wait_for("the errors", WAIT, |t| starting(t, "2 errors").is_some())
        .unwrap();
    app.key(Key::F8).unwrap();
    app.wait_for("an error current", WAIT, |t| {
        starting(t, "1 of 2").is_some() || starting(t, "2 of 2").is_some()
    })
    .unwrap();
    shot(&app, "console-errors");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_console_dark() {
    let dir = project("clean").unwrap();
    let app = open(&dir, true);
    run_all(&app);
    shot(&app, "console-dark");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_panel_resized() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    run_all(&app);
    for _ in 0..80 {
        let t = app.wait_for("the tree", WAIT, |_| true).unwrap();
        if t.find(&Role::Other("Separator".into()), "Resize bottom panel")
            .is_some_and(|n| n.states.focused)
        {
            break;
        }
        app.key(Key::Tab).unwrap();
    }
    for _ in 0..12 {
        app.key(Key::Up).unwrap();
    }
    app.wait_for("a taller panel", WAIT, |t| {
        t.find(&Role::Other("Separator".into()), "Resize bottom panel")
            .and_then(|n| n.value)
            .is_some_and(|v| v >= 380.)
    })
    .unwrap();
    app.key(Key::Tab).unwrap();
    app.key(Key::AltRight).unwrap();
    app.key(Key::AltRight).unwrap();
    app.wait_for("Console last", WAIT, |t| {
        let tabs: Vec<_> = t
            .all()
            .into_iter()
            .filter(|n| n.role == Role::Tab)
            .map(|n| n.name.clone())
            .collect();
        tabs.last().is_some_and(|n| n == "Console")
    })
    .unwrap();
    shot(&app, "panel-taller-tabs-moved");
}

/// The HTTP tab after a run of the http sample project, then narrowed to one test, then the calls that went wrong.
/// The steps before the chips work on a build without the list too, for the "before" shots.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_http() {
    for dark in [false, true] {
        http(dark);
    }
}

fn http(dark: bool) {
    let dir = project("http").unwrap();
    let settings = look(
        json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 400 }),
        dark,
    );
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    app.wait_for("the tests found", WAIT, |t| starting(t, "tests ").is_some())
        .unwrap();
    run_all(&app);
    let tab = |t: &Node| {
        t.all()
            .into_iter()
            .find(|n| n.role == Role::Tab && n.name.starts_with("HTTP"))
            .cloned()
    };
    let t = app.wait_for("the tabs", WAIT, |t| tab(t).is_some()).unwrap();
    app.press(&tab(&t).unwrap()).unwrap();
    app.wait_for("the HTTP tab", WAIT, |t| tab(t).is_some_and(|n| n.states.selected))
        .unwrap();
    shot(&app, &themed("http-after-a-run", dark));
    let t = app.tree().unwrap();
    let retry = t
        .all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with("retry, "))
        .cloned()
        .unwrap();
    app.click(&retry).unwrap();
    app.wait_for("retry selected", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.role == Role::TreeItem && n.name.starts_with("retry, ") && n.states.selected)
    })
    .unwrap();
    shot(&app, &themed("http-narrowed", dark));
    app.click(
        &t.all()
            .into_iter()
            .find(|n| n.name.starts_with("tests, "))
            .cloned()
            .unwrap(),
    )
    .unwrap();
    for chip in ["4xx Client error 3", "5xx Server error 1", "No response 2"] {
        app.press_named(Role::Button, chip).unwrap();
    }
    app.wait_for("six calls", WAIT, |t| tab(t).is_some_and(|n| n.name == "HTTP (6)"))
        .unwrap();
    shot(&app, &themed("http-went-wrong", dark));
    // a call opened: its details in place of the test's (before this slice the pane keeps the test)
    let t = app.tree().unwrap();
    let call = t
        .all()
        .into_iter()
        .find(|n| n.role == Role::ListItem && n.name.starts_with("500, "))
        .cloned()
        .unwrap();
    app.click(&call).unwrap();
    app.wait_for("the call selected", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.role == Role::ListItem && n.name.starts_with("500, ") && n.states.selected)
    })
    .unwrap();
    shot(&app, &themed("http-call-selected", dark));
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_home() {
    let clean = project("clean").unwrap();
    let tags = project("tags").unwrap();
    let gone = std::env::temp_dir().join("grace-screens-gone/payments-api");
    let app = App::launch_with_settings(
        &json!({ "recentProjects": [clean.to_string_lossy(), tags.to_string_lossy(), gone.to_string_lossy()] }),
        &[],
    )
    .unwrap();
    app.wait_for("the home screen", WAIT, |t| starting(t, "Open folder").is_some())
        .unwrap();
    shot(&app, "home");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_toolbar_during_a_run() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    app.press_named(Role::Button, "Run all").unwrap();
    app.wait_for("the run half way", RUN, |t| {
        t.all()
            .into_iter()
            .any(|n| n.name.len() >= 5 && n.name.ends_with(" / 9") && ["3", "4", "5", "6", "7"].contains(&&n.name[..1]))
    })
    .unwrap();
    shot(&app, "toolbar-during-a-run");
    app.wait_for("the run over", RUN, |t| starting(t, "Finished").is_some())
        .unwrap();
    shot(&app, "toolbar-after-a-run");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_tree() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    run_all(&app);
    for f in ["Expand outline.feature", "Expand slow.feature"] {
        app.press_named(Role::Button, f).unwrap();
    }
    app.wait_for("the features open", WAIT, |t| {
        starting(t, "Collapse slow.feature").is_some()
    })
    .unwrap();
    shot(&app, "tree");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_filter_bar() {
    for dark in [false, true] {
        let dir = project("tags").unwrap();
        let app = open(&dir, dark);
        let tree = app.tree().unwrap();
        let chip = tree
            .all()
            .into_iter()
            .find(|n| n.role == Role::Button && n.name.ends_with(", not filtered"))
            .unwrap()
            .name
            .clone();
        app.press_named(Role::Button, &chip).unwrap();
        app.wait_for("a chip included", WAIT, |t| {
            t.all().into_iter().any(|n| n.name.ends_with(", included"))
        })
        .unwrap();
        app.press_named(Role::CheckBox, "Hide non-matching").unwrap();
        app.wait_for("Hide non-matching checked", WAIT, |t| {
            t.find(&Role::CheckBox, "Hide non-matching")
                .is_some_and(|n| n.states.checked)
        })
        .unwrap();
        shot(&app, if dark { "filter-bar-dark" } else { "filter-bar" });
    }
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_filter_bar_many_tags() {
    let dir = grace_e2e::fresh_data_dir().unwrap().join("modules");
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    for i in 0..30 {
        std::fs::write(
            dir.join("tests").join(format!("module-{i:02}.feature")),
            format!("@module-{i:02}\nFeature: module {i}\n\nScenario: scenario {i}\n  * def x = {i}\n"),
        )
        .unwrap();
    }
    let app = open(&dir, false);
    let tree = app
        .wait_for("the fold chip", WAIT, |t| {
            t.all()
                .into_iter()
                .any(|n| n.role == Role::Button && n.name.contains(" more tag"))
        })
        .unwrap();
    let more = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.contains(" more tag"))
        .unwrap()
        .name
        .clone();
    app.press_named(Role::Button, &more).unwrap();
    app.wait_for("the panel", WAIT, |t| t.find(&Role::Group, "All tags").is_some())
        .unwrap();
    shot(&app, "filter-bar-all-tags");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_env_suggestions() {
    let dir = project("env").unwrap();
    let app = open(&dir, false);
    let env = app.wait_for_node(Role::ComboBox, "Env").unwrap();
    app.click(&env).unwrap();
    app.wait_for("the suggestions", WAIT, |t| {
        t.find(&Role::List, "Environments").is_some()
    })
    .unwrap();
    shot(&app, "env-suggestions");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_properties() {
    let dir = project("env").unwrap();
    std::fs::write(
        dir.join("grace-shots.properties"),
        "baseUrl=https://qa.example.test\ntimeout=5000\n",
    )
    .unwrap();
    let app = open(&dir, false);
    app.press_named(Role::Button, "Properties…").unwrap();
    app.wait_for("the dialog", WAIT, |t| t.find(&Role::TabList, "Environments").is_some())
        .unwrap();
    app.press_named(Role::Button, "Add property").unwrap();
    let tree = app
        .wait_for("the new row", WAIT, |t| {
            t.find(&Role::ListItem, "New property").is_some()
        })
        .unwrap();
    let name_box = tree
        .find(&Role::ListItem, "New property")
        .unwrap()
        .all()
        .into_iter()
        .find(|n| n.name == "Property name")
        .unwrap()
        .clone();
    app.click(&name_box).unwrap();
    app.type_text("baseUrl").unwrap();
    let tree = app
        .wait_for("the row", WAIT, |t| t.find(&Role::ListItem, "baseUrl").is_some())
        .unwrap();
    let value_box = tree
        .find(&Role::ListItem, "baseUrl")
        .unwrap()
        .all()
        .into_iter()
        .find(|n| n.name == "Property value")
        .unwrap()
        .clone();
    app.click(&value_box).unwrap();
    app.type_text("https://qa.example.test").unwrap();
    shot(&app, "properties");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_details() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    run_all(&app);
    let tree = app.tree().unwrap();
    let row = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with("failing match, "))
        .unwrap()
        .clone();
    app.click(&row).unwrap();
    let tree = app
        .wait_for("the steps", WAIT, |t| t.find(&Role::List, "Steps").is_some())
        .unwrap();
    let toggle = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::Button && n.name.starts_with("Show log of method get"))
        .unwrap()
        .name
        .clone();
    app.press_named(Role::Button, &toggle).unwrap();
    app.wait_for("the log", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.name.starts_with("Hide log of method get"))
    })
    .unwrap();
    shot(&app, "details-failed-test");
    let tree = app.tree().unwrap();
    let feature = tree
        .all()
        .into_iter()
        .find(|n| n.role == Role::TreeItem && n.name.starts_with("basic.feature, "))
        .unwrap()
        .clone();
    app.click(&feature).unwrap();
    app.wait_for("the group", WAIT, |t| {
        t.all().into_iter().any(|n| n.name.ends_with(" scenarios"))
    })
    .unwrap();
    shot(&app, "details-group");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_settings() {
    let dir = project("clean").unwrap();
    for (dark, name) in [(false, "settings-appearance"), (true, "settings-appearance-dark")] {
        let app = open(&dir, dark);
        app.press_named(Role::Button, "Settings").unwrap();
        app.wait_for("the page", WAIT, |t| t.find(&Role::Group, "Theme").is_some())
            .unwrap();
        shot(&app, name);
    }
}

/// A tree longer than the window, its scrollbar, and a cut label shown whole on keyboard focus.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_long_tree() {
    for dark in [false, true] {
        let dir = grace_e2e::fresh_data_dir().unwrap().join("big");
        std::fs::create_dir_all(dir.join("tests")).unwrap();
        let long =
            "feature-000-the-checkout-keeps-the-basket-when-the-session-ends-and-the-customer-signs-in-again-later";
        for name in (1..120).map(|i| format!("feature-{i:03}")).chain([long.to_owned()]) {
            std::fs::write(
                dir.join("tests").join(format!("{name}.feature")),
                format!("Feature: {name}\n\nScenario: works\n  * def x = 1\n"),
            )
            .unwrap();
        }
        let app = open(&dir, dark);
        let tests = app
            .wait_for("the features", WAIT, |t| starting(t, "feature-001.feature").is_some())
            .unwrap();
        let folder = tests
            .all()
            .into_iter()
            .find(|n| n.role == Role::TreeItem && n.name.starts_with("tests, "))
            .unwrap()
            .clone();
        app.press(&folder).unwrap();
        app.key(Key::Down).unwrap();
        app.wait_for_node(Role::Tooltip, &format!("{long}.feature · {long}"))
            .unwrap();
        shot(&app, if dark { "long-tree-dark" } else { "long-tree" });
    }
}

/// The toolbar in a window narrower than 1280 px: four buttons as icons, their names in tooltips.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_narrow_toolbar() {
    let dir = project("clean").unwrap();
    let app = open(&dir, false);
    app.set_window_rect(0, 0, 1000, 640).unwrap();
    let open = app
        .wait_for("the narrow toolbar", WAIT, |t| {
            t.find(&Role::Button, "Open…")
                .and_then(|b| b.rect)
                .is_some_and(|r| r.width < 40)
        })
        .unwrap();
    app.hover(open.find(&Role::Button, "Open…").unwrap()).unwrap();
    app.wait_for_node(Role::Tooltip, "Open…").unwrap();
    shot(&app, "toolbar-narrow");
}

#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_open_in_vs_code() {
    for dark in [false, true] {
        open_in_vs_code(dark);
    }
}

fn open_in_vs_code(dark: bool) {
    let dir = project("clean").unwrap();
    let app = open(&dir, dark);
    app.press_named(Role::Button, "Expand basic.feature").unwrap();
    run_all(&app);
    let tree = app.tree().unwrap();
    let row = starting(&tree, "failing match, ").unwrap().clone();
    app.click(&row).unwrap();
    app.wait_for("the details", WAIT, |t| starting(t, "tests/basic.feature:15").is_some())
        .unwrap();
    shot(&app, &themed("details-open-in-vs-code", dark));
    // the build before this change has no menu: the wait is allowed to fail, and the shot shows the tree as it is
    app.right_click(&row).unwrap();
    let _ = app.wait_for("the menu", Duration::from_secs(3), |t| {
        t.all().into_iter().any(|n| n.role == Role::MenuItem)
    });
    shot(&app, &themed("tree-row-menu", dark));
}

/// The History tab with runs of every kind: a failed one, a cancelled one on an environment with tags, a scenario's, a
/// re-run of failed tests and one from yesterday; then Clear history's dialog.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_history() {
    for dark in [false, true] {
        history(dark);
    }
}

fn history(dark: bool) {
    use grace_e2e::history::{Seed, seed_runs};
    let dir = project("clean").unwrap();
    let data = grace_e2e::fresh_data_dir().unwrap();
    let seed = |ago, record| {
        seed_runs(
            &data,
            &dir,
            1,
            &Seed {
                ago,
                record,
                report: true,
            },
        )
    };
    let scenario = json!({ "kind": "scenario", "file": "tests/basic.feature", "name": "failing match", "line": 15, "key": "s:tests/basic.feature#2" });
    seed(1500, json!({ "exitCode": 0, "durationMillis": 83_000 }));
    seed(40, json!({ "selection": scenario, "durationMillis": 400 }));
    seed(
        30,
        json!({ "selection": null, "failedOf": 1_700_000_000_000_i64, "durationMillis": 1500 }),
    );
    seed(
        6,
        json!({ "env": "qa", "outcome": "cancelled", "exitCode": null, "durationMillis": 18_000,
                "filter": { "include": ["smoke"], "exclude": ["slow"], "hideNonMatching": false } }),
    );
    seed(0, json!({}));
    std::fs::write(
        data.join("settings.json"),
        look(
            json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 340 }),
            dark,
        )
        .to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    app.wait_for("the runs", WAIT, |t| {
        t.all()
            .into_iter()
            .any(|n| n.role == Role::Tab && n.name == "History (5)")
    })
    .unwrap();
    let t = app.tree().unwrap();
    app.press(
        t.all()
            .into_iter()
            .find(|n| n.role == Role::Tab && n.name.starts_with("History"))
            .unwrap(),
    )
    .unwrap();
    app.wait_for("the rows", WAIT, |t| {
        t.find(&Role::List, "Run history")
            .is_some_and(|l| l.all().iter().filter(|n| n.role == Role::ListItem).count() == 5)
    })
    .unwrap();
    shot(&app, &themed("history-tab", dark));
    // the window at its least width, 760 (`window::MIN_SIZE`, which Xvfb does not enforce): the row's buttons as icons, a name in a tooltip
    // (#385). The build before that change has no icons: the waits may fail, and the shots show the rows as they are
    let row_button = |t: &Node| {
        t.find(&Role::List, "Run history")
            .and_then(|l| l.find(&Role::Button, "Re-run failed"))
            .cloned()
    };
    let at = app.tree().unwrap().rect.unwrap_or(grace_e2e::Rect {
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
    });
    app.set_window_rect(at.x, at.y, 760, at.height).unwrap();
    let narrow = Duration::from_secs(3);
    let _ = app.wait_for("the narrow rows", narrow, |t| {
        row_button(t).and_then(|b| b.rect).is_some_and(|r| r.width < 40)
    });
    shot(&app, &themed("history-narrow", dark));
    app.hover(&row_button(&app.tree().unwrap()).unwrap()).unwrap();
    let _ = app.wait_for("the tooltip", narrow, |t| {
        t.all().iter().any(|n| n.role == Role::Tooltip)
    });
    shot(&app, &themed("history-narrow-tooltip", dark));
    app.set_window_rect(at.x, at.y, at.width, at.height).unwrap();
    let t = app
        .wait_for("the window wide again", WAIT, |t| {
            t.rect.is_some_and(|r| r.width >= at.width)
        })
        .unwrap();
    app.click(t.find(&Role::Button, "Clear history").unwrap()).unwrap();
    app.wait_for("the dialog", WAIT, |t| t.all().iter().any(|n| n.role == Role::Dialog))
        .unwrap();
    shot(&app, &themed("history-clear-dialog", dark));
}

/// A flaky test selected: Details shows the proof (slice 8 of P3). The build before it shows the mark and no proof.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_flaky() {
    for dark in [false, true] {
        flaky(dark);
    }
}

fn flaky(dark: bool) {
    use grace_e2e::history::{Seed, seed_runs};
    let dir = project("flaky").unwrap();
    let data = grace_e2e::fresh_data_dir().unwrap();
    let tests = |coin: &str| {
        json!([
            { "kind": "scenario", "file": "tests/coin.feature", "refId": "[1:3]", "name": "coin", "line": 3,
              "key": "s:tests/coin.feature#1", "status": coin },
            { "kind": "scenario", "file": "tests/coin.feature", "refId": "[2:7]", "name": "steady", "line": 7,
              "key": "s:tests/coin.feature#2", "status": "passed" },
        ])
    };
    for (ago, coin, threads) in [(1, "failed", 2), (0, "passed", 1)] {
        let record = json!({
            "threads": threads, "fingerprint": "f".repeat(64),
            "exitCode": if coin == "failed" { 1 } else { 0 }, "tests": tests(coin),
        });
        seed_runs(
            &data,
            &dir,
            1,
            &Seed {
                report: true,
                ago,
                record,
            },
        );
    }
    std::fs::write(
        data.join("settings.json"),
        look(json!({ "lastProject": dir.to_string_lossy() }), dark).to_string(),
    )
    .unwrap();
    let app = App::launch_in(data, &[]).unwrap();
    app.wait_for("the history", WAIT, |t| starting(t, "History (2)").is_some())
        .unwrap();
    app.press_named(Role::Button, "Expand coin.feature").unwrap();
    let t = app.wait_for("coin", WAIT, |t| starting(t, "coin, ").is_some()).unwrap();
    app.click(starting(&t, "coin, ").unwrap()).unwrap();
    // the build before the proof has no group: the wait is allowed to fail
    let _ = app.wait_for("the proof", Duration::from_secs(5), |t| {
        t.all()
            .iter()
            .any(|n| n.name.starts_with("Flaky on") || n.name.starts_with("Flaky with"))
    });
    shot(&app, &themed("flaky", dark));
}

/// An outline selected after a run: Details shows its Examples as a table (slice 9 of P3). The build before it shows
/// the outline's counts and nothing under them.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_examples() {
    for dark in [false, true] {
        examples(dark);
    }
}

fn examples(dark: bool) {
    let app = App::launch_with_settings(
        &look(
            json!({ "lastProject": project("clean").unwrap().to_string_lossy() }),
            dark,
        ),
        &[],
    )
    .unwrap();
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
    app.press_named(Role::Button, "Expand outline.feature").unwrap();
    app.wait_for("the outline", WAIT, |t| starting(t, "add <a> and <b>, ").is_some())
        .unwrap();
    // the whole run: the tree moves while it goes on, so the row is found once it is over
    run_all(&app);
    let t = app.tree().unwrap();
    app.click(starting(&t, "add <a> and <b>, ").unwrap()).unwrap();
    // the build before the table has no list: the wait is allowed to fail
    let _ = app.wait_for("the table", Duration::from_secs(5), |t| {
        t.all().iter().any(|n| n.name.starts_with("Row 1: "))
    });
    shot(&app, &themed("examples", dark));
}

/// A run that failed on a broken `karate-config.js`, with the first failed test selected: the banner that says so once
/// and the stepper under the test's error (slice 10 of P3). The build before shows each test's error and nothing more.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_failures() {
    for dark in [false, true] {
        failures(dark);
    }
}

fn failures(dark: bool) {
    let dir = grace_e2e::fresh_data_dir().unwrap();
    let write = |file: &str, text: &str| std::fs::write(dir.join(file), text).unwrap();
    write(
        "karate-config.js",
        "function fn() {
  karate.fail('no vpn');
  return {};
}
",
    );
    write(
        "a.feature",
        "Feature: a

Scenario: one
* print 'x'

Scenario: two
* print 'x'
",
    );
    std::fs::create_dir(dir.join("suite")).unwrap();
    write(
        "suite/b.feature",
        "Feature: b

Scenario: three
* print 'x'

Scenario: four
* print 'x'
",
    );
    // the least bottom panel and a tall window: the banner, the error and the stepper show whole, which slice 10's
    // shot cut off
    let settings = look(json!({ "lastProject": dir.to_string_lossy(), "panelHeight": 96 }), dark);
    let app = App::launch_with_settings(&settings, &[]).unwrap();
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
    let at = app.tree().unwrap().rect.unwrap_or(grace_e2e::Rect {
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
    });
    app.set_window_rect(at.x, 0, at.width, 1050).unwrap();
    app.press_named(Role::Button, "Run all").unwrap();
    let t = app
        .wait_for("the run finished", RUN, |t| starting(t, "three, failed").is_some())
        .unwrap();
    app.click(starting(&t, "three, failed").unwrap()).unwrap();
    // the build before has no banner and no stepper: the wait is allowed to fail
    let _ = app.wait_for("the banner", Duration::from_secs(5), |t| {
        t.all().iter().any(|n| n.name == "karate-config.js failed for 4 tests")
    });
    shot(&app, &themed("failures", dark));
    // a folder: its failures grouped by error, with the reason and the tests that share it
    let t = app.tree().unwrap();
    if let Some(folder) = starting(&t, "suite, ") {
        app.click(folder).unwrap();
        let _ = app.wait_for("the folder's list", Duration::from_secs(5), |t| {
            t.all().iter().any(|n| n.name == "Failed with the same error")
        });
    }
    shot(&app, &themed("failures-folder", dark));
}

/// A Maven project whose Java code is not built (slice 4 of P4): the notice with its Build button, the build going, and
/// a failed build's long notice, also at the narrowest window. The build before shows no notice.
#[test]
#[ignore = "takes screenshots; run with scripts/e2e-gpui.sh screens_"]
fn screens_build_notice() {
    for dark in [false, true] {
        build_notice(dark);
    }
}

fn build_notice(dark: bool) {
    let dir = project("maven").unwrap();
    let helper = dir.join("src/test/java/common/Helper.java");
    std::fs::create_dir_all(helper.parent().unwrap()).unwrap();
    std::fs::write(helper, "class Helper {}").unwrap();
    // a stand-in for Maven: it lists no library, and test-compile takes a while, then fails
    #[cfg(windows)]
    std::fs::write(
        dir.join("mvnw.cmd"),
        "@echo off\r\nif defined GRACE_CLASSPATH_FILE (type nul>\"%GRACE_CLASSPATH_FILE%\" & exit /b 0)\r\n\
         ping -n 6 127.0.0.1 >nul\r\necho [ERROR] COMPILATION ERROR\r\nexit /b 1\r\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = dir.join("mvnw");
        let script = "#!/bin/sh\n\
             if [ -n \"$GRACE_CLASSPATH_FILE\" ]; then : > \"$GRACE_CLASSPATH_FILE\"; exit 0; fi\n\
             sleep 5\necho '[ERROR] COMPILATION ERROR'\nexit 1\n";
        std::fs::write(&wrapper, script).unwrap();
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let app = open(&dir, dark);
    app.wait_for("the project ready", WAIT, |t| starting(t, "Ready.").is_some())
        .unwrap();
    shot(&app, &themed("build-notice", dark));
    // the build before has no notice and no button
    if app.tree().unwrap().find(&Role::Button, "Build").is_none() {
        return;
    }
    app.press_named(Role::Button, "Build").unwrap();
    app.wait_for("the build going", WAIT, |t| {
        starting(t, "Building the project's Java code").is_some()
    })
    .unwrap();
    shot(&app, &themed("build-running", dark));
    app.wait_for("the failure's notice", RUN, |t| {
        starting(t, "Grace could not build").is_some()
    })
    .unwrap();
    shot(&app, &themed("build-failed", dark));
    let at = app.tree().unwrap().rect.unwrap_or(grace_e2e::Rect {
        x: 0,
        y: 0,
        width: 1200,
        height: 800,
    });
    app.set_window_rect(at.x, at.y, 760, at.height).unwrap();
    shot(&app, &themed("build-failed-narrow", dark));
}
