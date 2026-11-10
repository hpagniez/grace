//! End-to-end tests of GPUI Grace through the accessibility tree (migration guide R6.3, ADR 0031). A test starts
//! the built app with a fresh data folder, reads its tree by role and name, and presses what a person would press.
//! One driver, two backends: AT-SPI on Linux, UI Automation on Windows. It replaces Playwright's `_electron`
//! (ADR 0004) at the cut-over.
//!
//! The tree is read as a snapshot (`Driver::tree`) and searched in plain Rust, so each backend only has to walk
//! and press. Like the Playwright tests, every test waits for what it expects (`App::wait_for`) rather than sleeping.
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
mod atspi_driver;
#[cfg(windows)]
mod uia_driver;

/// What a node is. The roles Grace uses; any other is `Other` with the platform's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Application,
    Window,
    Group,
    Button,
    CheckBox,
    RadioButton,
    Entry,
    ComboBox,
    SpinButton,
    Slider,
    Tab,
    TabList,
    Tree,
    TreeItem,
    List,
    ListItem,
    Link,
    MenuItem,
    Status,
    ProgressBar,
    Image,
    Text,
    Dialog,
    Tooltip,
    Other(String),
}

impl Role {
    /// A control a person acts on: it must have a name (guide R8.2).
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            Role::Button
                | Role::CheckBox
                | Role::RadioButton
                | Role::Entry
                | Role::ComboBox
                | Role::SpinButton
                | Role::Slider
                | Role::Tab
                | Role::TreeItem
                | Role::ListItem
                | Role::Link
                | Role::MenuItem
        )
    }
}

/// The states the tests read. On Linux the pinned AccessKit adapter reports every button enabled and drops
/// `expanded` (guide R8.4), so tests must not rely on those two there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct States {
    pub enabled: bool,
    pub focusable: bool,
    pub focused: bool,
    pub selected: bool,
    /// A check box or a radio button that is on.
    pub checked: bool,
    pub expanded: Option<bool>,
}

/// Where a node is, in pixels: its left, top, width and height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn centre(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }
}

/// A key the tests press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Escape,
    Enter,
    Space,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// Ctrl+Enter.
    CtrlEnter,
    /// Ctrl+A, which selects what a text box holds.
    SelectAll,
    Backspace,
    /// Shift+Enter.
    ShiftEnter,
    /// Ctrl+F.
    CtrlF,
    F8,
    /// Shift+F8.
    ShiftF8,
    /// Alt+Left.
    AltLeft,
    /// Alt+Right.
    AltRight,
}

/// One node of the tree, as read. `path` is its position under the app's window, which `Driver::press` finds again.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub role: Role,
    pub name: String,
    pub description: String,
    pub value: Option<f64>,
    /// What a text box holds.
    pub text: Option<String>,
    pub states: States,
    /// Where it is, when the platform says: on the screen through UI Automation, in its window through AT-SPI.
    /// `Driver::hover` reads it as its backend wrote it.
    pub rect: Option<Rect>,
    pub path: Vec<usize>,
    pub children: Vec<Node>,
}

impl Node {
    /// This node and every node under it, depth first.
    pub fn all(&self) -> Vec<&Node> {
        let mut out = vec![self];
        for c in &self.children {
            out.extend(c.all());
        }
        out
    }

    /// The first node with this role and name.
    pub fn find(&self, role: &Role, name: &str) -> Option<&Node> {
        self.all().into_iter().find(|n| &n.role == role && n.name == name)
    }

    /// Every interactive node without a name, as `role at path`: what the tree check fails on (guide R8.2).
    pub fn unnamed_controls(&self) -> Vec<String> {
        self.all()
            .into_iter()
            .filter(|n| n.role.is_interactive() && n.name.trim().is_empty())
            .map(|n| format!("{:?} at {:?}", n.role, n.path))
            .collect()
    }

    /// The tree as indented text, for a failing test's message.
    pub fn dump(&self) -> String {
        fn line(n: &Node, depth: usize, out: &mut String) {
            out.push_str(&format!("{}{:?} {:?}", "  ".repeat(depth), n.role, n.name));
            if !n.description.is_empty() {
                out.push_str(&format!(" desc={:?}", n.description));
            }
            if let Some(v) = n.value {
                out.push_str(&format!(" value={v}"));
            }
            if let Some(t) = &n.text {
                out.push_str(&format!(" text={t:?}"));
            }
            out.push('\n');
            for c in &n.children {
                line(c, depth + 1, out);
            }
        }
        let mut out = String::new();
        line(self, 0, &mut out);
        out
    }
}

pub type Result<T> = std::result::Result<T, String>;

/// A platform's way into an app's accessibility tree.
pub trait Driver {
    /// The tree of the app's first window, or an error while the app has none yet.
    fn tree(&self) -> Result<Node>;
    /// Does the node's default action (a button's press).
    fn press(&self, node: &Node) -> Result<()>;
    /// Moves the pointer to the middle of the node, as a person resting the mouse on it.
    fn hover(&self, node: &Node) -> Result<()>;
    /// Presses a key on the keyboard, for the window under the pointer or with the focus.
    fn key(&self, key: Key) -> Result<()>;
    /// Clicks the pointer on the node's centre, as a person does to put the focus in a text box.
    fn click(&self, node: &Node) -> Result<()>;
    /// Types `text` on the keyboard, into the control with the focus.
    fn type_text(&self, text: &str) -> Result<()>;
    /// Asks the window to close, as its close button does: Grace then saves, stops a run and quits (Playwright's
    /// `app.close()`).
    fn close(&self) -> Result<()>;
    /// Where the system says the keyboard focus is, for a failure's message: the element and the foreground window.
    fn focus_report(&self) -> String {
        String::new()
    }
    /// Moves and sizes the window, as a person dragging it and its edge does (Electron's `setBounds`).
    fn set_window_rect(&self, x: i32, y: i32, width: i32, height: i32) -> Result<()>;
    /// Maximizes, minimizes or restores the window, as its title bar's buttons do. Only on Windows: Xvfb has no
    /// window manager to do it.
    fn show_window(&self, _show: Show) -> Result<()> {
        Err("no window manager to maximize or minimize the window".into())
    }
}

/// How the window shows, for `Driver::show_window`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Show {
    Normal,
    Maximized,
    Minimized,
}

/// A rectangle in GPUI's own coordinates: the saved window's bounds, or a screen's work area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The window as `settings.json` keeps it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SavedWindow {
    pub bounds: Area,
    pub maximized: bool,
}

/// The built `Grace` binary: `GRACE_EXE` when set, else the workspace's debug build.
pub fn grace_exe() -> PathBuf {
    if let Ok(exe) = std::env::var("GRACE_EXE") {
        return PathBuf::from(exe);
    }
    let name = if cfg!(windows) { "Grace.exe" } else { "Grace" };
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/debug")
        .join(name)
}

/// A running Grace with its own data folder, killed and cleaned up when dropped.
pub struct App {
    child: Child,
    data: PathBuf,
    driver: Box<dyn Driver>,
    /// Leave the data folder when dropped, for a second start on it (`App::relaunch`).
    keep: bool,
}

/// A fresh, empty data folder for one test.
pub fn fresh_data_dir() -> Result<PathBuf> {
    let data = std::env::temp_dir().join(format!("grace-e2e-{}-{}", std::process::id(), unique()));
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    Ok(data)
}

/// A copy of a fixture project of `test/fixtures/projects`, in a folder of its own, as `e2e/helpers.ts` `project()`
/// makes one: a test may change it.
pub fn project(name: &str) -> Result<PathBuf> {
    fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            let target = to.join(entry.file_name());
            if entry.file_type()?.is_dir() {
                copy(&entry.path(), &target)?;
            } else {
                std::fs::copy(entry.path(), target)?;
            }
        }
        Ok(())
    }
    let from = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/projects")
        .join(name);
    let to = std::env::temp_dir()
        .join(format!("grace-e2e-project-{}-{}", std::process::id(), unique()))
        .join(name);
    copy(&from, &to).map_err(|e| format!("cannot copy {}: {e}", from.display()))?;
    Ok(to)
}

impl App {
    /// Starts Grace with a fresh data folder (`GRACE_USER_DATA`), its window on a screen other than the main one
    /// (`GRACE_E2E_SCREEN`, read by the `e2e` feature), as `e2e/helpers.ts` `launch()` does.
    pub fn launch() -> Result<App> {
        Self::launch_with(&[])
    }

    pub fn launch_with(env: &[(&str, &str)]) -> Result<App> {
        Self::launch_in(fresh_data_dir()?, env)
    }

    /// Starts Grace with `settings.json` already holding `settings`, as `e2e/helpers.ts` `launchWithSettings()`.
    pub fn launch_with_settings(settings: &serde_json::Value, env: &[(&str, &str)]) -> Result<App> {
        let data = fresh_data_dir()?;
        std::fs::write(data.join("settings.json"), settings.to_string()).map_err(|e| e.to_string())?;
        Self::launch_in(data, env)
    }

    /// Starts Grace on an existing data folder.
    pub fn launch_in(data: PathBuf, env: &[(&str, &str)]) -> Result<App> {
        let exe = grace_exe();
        #[cfg(target_os = "linux")]
        atspi_driver::prepare()?;
        #[cfg(windows)]
        uia_driver::prepare()?;
        let mut cmd = Command::new(&exe);
        cmd.env("GRACE_USER_DATA", &data)
            .env("GRACE_E2E_SCREEN", "other")
            .env("GRACE_E2E_SEAMS", seams_file(&data));
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
        let driver = driver_for(child.id())?;
        Ok(App {
            child,
            data,
            driver,
            keep: false,
        })
    }

    /// Ends this Grace, keeping its data folder, and starts another on it with `env`: Electron's restart.
    pub fn relaunch(mut self, env: &[(&str, &str)]) -> Result<App> {
        self.keep = true;
        let data = self.data.clone();
        drop(self);
        Self::launch_in(data, env)
    }

    /// `settings.json` as Grace wrote it, or `null` when there is none.
    pub fn settings(&self) -> serde_json::Value {
        std::fs::read_to_string(self.data.join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(serde_json::Value::Null)
    }

    /// Answers the next folder dialogs with `folder` (the app's `e2e` seams, `GRACE_E2E_SEAMS`).
    pub fn answer_folder(&self, folder: Option<&Path>) -> Result<()> {
        self.answer("folder", folder)
    }

    /// Answers the next file dialogs, Import from file…'s, with `file`.
    pub fn answer_file(&self, file: Option<&Path>) -> Result<()> {
        self.answer("file", file)
    }

    /// Sets one seam's answer and keeps the others'.
    fn answer(&self, seam: &str, path: Option<&Path>) -> Result<()> {
        let file = seams_file(&self.data);
        let mut answers = std::fs::read_to_string(&file)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .filter(|v| v.is_object())
            .unwrap_or_else(|| serde_json::json!({}));
        answers[seam] = serde_json::json!(path);
        std::fs::write(file, answers.to_string()).map_err(|e| e.to_string())
    }

    /// Where each folder dialog started, in order (`e2e-seams.jsonl`).
    pub fn folder_dialogs(&self) -> Vec<Option<String>> {
        std::fs::read_to_string(self.data.join("e2e-seams.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| v["seam"] == "folder")
            .map(|v| v["start"].as_str().map(str::to_string))
            .collect()
    }

    /// The files Grace opened with their program, in order (`e2e-seams.jsonl`): Report's HTML.
    pub fn opened_paths(&self) -> Vec<String> {
        std::fs::read_to_string(self.data.join("e2e-seams.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| v["seam"] == "open")
            .filter_map(|v| v["path"].as_str().map(str::to_string))
            .collect()
    }

    /// The Java processes this Grace started: their command line holds its data folder, where a run's output and
    /// argument file go (`ourJavaPids` of `e2e/helpers.ts`, which looked for the bundled JRE).
    pub fn java_pids(&self) -> Vec<u32> {
        java_under(&self.data).into_iter().map(|(pid, _)| pid).collect()
    }

    /// The command lines of the Java processes this Grace started (`ourJavaCommandLines` of `e2e/helpers.ts`).
    pub fn java_command_lines(&self) -> Vec<String> {
        java_under(&self.data).into_iter().map(|(_, cmd)| cmd).collect()
    }

    /// Asks the window to close, as its close button does.
    pub fn close(&self) -> Result<()> {
        self.driver.close()
    }

    /// Asks the window to close and waits for Grace to end: its bounds are then saved.
    pub fn close_and_wait(&mut self) -> Result<()> {
        self.driver.close()?;
        if self.exited(Duration::from_secs(30)) {
            Ok(())
        } else {
            Err("Grace did not end within 30 s of its window's close".into())
        }
    }

    /// See `Driver::set_window_rect`.
    pub fn set_window_rect(&self, x: i32, y: i32, width: i32, height: i32) -> Result<()> {
        self.driver.set_window_rect(x, y, width, height)
    }

    /// See `Driver::show_window`.
    pub fn show_window(&self, show: Show) -> Result<()> {
        self.driver.show_window(show)
    }

    /// The main screen's work area, as the `e2e` build records it at start (`grace::e2e::record_screen`).
    pub fn work_area(&self) -> Result<Area> {
        let file = self.data.join("e2e-screen.json");
        self.wait_until("the screen recorded", Duration::from_secs(15), || file.exists())?;
        let v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&file).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        area(&v).ok_or_else(|| format!("e2e-screen.json: {v}"))
    }

    /// The window saved in `settings.json`, or None.
    pub fn saved_window(&self) -> Option<SavedWindow> {
        let w = self.settings().get("window")?.clone();
        Some(SavedWindow {
            bounds: area(&w)?,
            maximized: w.get("maximized")?.as_bool()?,
        })
    }

    /// Waits for Grace's process to end, up to `timeout`; true if it did.
    pub fn exited(&mut self, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Ok(Some(_)) = self.child.try_wait() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Waits until `ok` holds, checking every 50 ms, up to `timeout`.
    pub fn wait_until(&self, what: &str, timeout: Duration, ok: impl Fn() -> bool) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(format!("timed out waiting for {what}"))
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn tree(&self) -> Result<Node> {
        self.driver.tree()
    }

    /// Where the system says the keyboard focus is (Windows only), for a failure's message.
    pub fn focus_report(&self) -> String {
        self.driver.focus_report()
    }

    /// Waits until the tree satisfies `ok`, reading it every 50 ms, up to `timeout`; returns that tree.
    pub fn wait_for(&self, what: &str, timeout: Duration, ok: impl Fn(&Node) -> bool) -> Result<Node> {
        let start = Instant::now();
        let mut last = Err("no tree read yet".to_string());
        while start.elapsed() < timeout {
            last = self.driver.tree();
            if let Ok(tree) = &last
                && ok(tree)
            {
                return last;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(match last {
            Ok(tree) => format!("timed out waiting for {what}; the tree:\n{}", tree.dump()),
            Err(e) => format!("timed out waiting for {what}: {e}"),
        })
    }

    /// Waits for a node with this role and name, and returns it.
    pub fn wait_for_node(&self, role: Role, name: &str) -> Result<Node> {
        let tree = self.wait_for(&format!("{role:?} {name:?}"), Duration::from_secs(10), |t| {
            t.find(&role, name).is_some()
        })?;
        Ok(tree.find(&role, name).cloned().expect("found above"))
    }

    pub fn press(&self, node: &Node) -> Result<()> {
        self.driver.press(node)
    }

    /// Presses the node with this role and name, reading the tree afresh: a node is pressed by its place in the
    /// tree, and a tree that changed between the read and the press (a run's first event, a button's new state) moves
    /// it. Tries again for up to 5 s.
    pub fn press_named(&self, role: Role, name: &str) -> Result<()> {
        let start = Instant::now();
        loop {
            let tried = self
                .driver
                .tree()
                .and_then(|t| {
                    t.find(&role, name)
                        .cloned()
                        .ok_or_else(|| format!("no {role:?} {name:?}"))
                })
                .and_then(|n| self.driver.press(&n));
            match tried {
                Ok(()) => return Ok(()),
                Err(e) if start.elapsed() > Duration::from_secs(5) => return Err(e),
                Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }

    pub fn hover(&self, node: &Node) -> Result<()> {
        self.driver.hover(node)
    }

    pub fn key(&self, key: Key) -> Result<()> {
        self.driver.key(key)
    }

    pub fn click(&self, node: &Node) -> Result<()> {
        self.driver.click(node)
    }

    pub fn type_text(&self, text: &str) -> Result<()> {
        self.driver.type_text(text)
    }

    /// Clicks into the text box with this role and name and types `text` over what it holds (Playwright's `fill`).
    pub fn fill(&self, role: Role, name: &str, text: &str) -> Result<()> {
        let node = self.wait_for_node(role.clone(), name)?;
        self.click(&node)?;
        self.wait_for(
            &format!("{role:?} {name:?} to take the focus"),
            Duration::from_secs(5),
            |t| {
                t.all()
                    .iter()
                    .any(|n| n.role == role && n.name == name && n.states.focused)
            },
        )
        .map_err(|e| format!("{e}\n{}", self.focus_report()))?;
        self.key(Key::SelectAll)?;
        self.key(Key::Backspace)?;
        if text.is_empty() { Ok(()) } else { self.type_text(text) }
    }

    /// The text box with this role and name, once it holds `text`. Through AT-SPI the pinned AccessKit adapter gives
    /// a combo box or a spin button no text (it has no text runs) and no other way to its value, so on Linux this
    /// only finds the box; UI Automation's Value pattern reads it on Windows.
    pub fn wait_for_text(&self, role: Role, name: &str, text: &str) -> Result<Node> {
        let tree = self.wait_for(
            &format!("{role:?} {name:?} to hold {text:?}"),
            Duration::from_secs(30),
            |t| {
                t.find(&role, name)
                    .is_some_and(|n| cfg!(target_os = "linux") || n.text.as_deref().unwrap_or_default() == text)
            },
        )?;
        Ok(tree.find(&role, name).cloned().expect("found above"))
    }

    /// Waits until no node has this role and name.
    pub fn wait_for_no_node(&self, role: Role, name: &str) -> Result<Node> {
        self.wait_for(&format!("no {role:?} {name:?}"), Duration::from_secs(10), |t| {
            t.find(&role, name).is_none()
        })
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.data);
        }
    }
}

/// Java processes whose command line names `dir`.
#[cfg(target_os = "linux")]
fn java_under(dir: &Path) -> Vec<(u32, String)> {
    let dir = dir.to_string_lossy();
    let Ok(procs) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    procs
        .flatten()
        .filter_map(|p| {
            let pid: u32 = p.file_name().to_str()?.parse().ok()?;
            let cmd = std::fs::read(p.path().join("cmdline")).ok()?;
            let cmd = String::from_utf8_lossy(&cmd).replace('\0', " ");
            let first = cmd.split(' ').next()?;
            (first.ends_with("java") && cmd.contains(dir.as_ref())).then_some((pid, cmd))
        })
        .collect()
}

#[cfg(windows)]
fn java_under(dir: &Path) -> Vec<(u32, String)> {
    let script = "Get-CimInstance Win32_Process -Filter \"Name='java.exe'\" | ForEach-Object { \"$($_.ProcessId) $($_.CommandLine)\" }";
    let Ok(out) = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
    else {
        return Vec::new();
    };
    let dir = dir.to_string_lossy().to_lowercase();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.to_lowercase().contains(&dir))
        .filter_map(|l| {
            let (pid, cmd) = l.split_once(' ')?;
            Some((pid.parse().ok()?, cmd.to_string()))
        })
        .collect()
}

/// The file the app reads its seams' answers from; a test writes it with `App::answer_folder`.
fn seams_file(data: &Path) -> PathBuf {
    data.join("e2e-answers.json")
}

fn unique() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn driver_for(pid: u32) -> Result<Box<dyn Driver>> {
    Ok(Box::new(atspi_driver::AtspiDriver::new(pid)?))
}

#[cfg(windows)]
fn driver_for(pid: u32) -> Result<Box<dyn Driver>> {
    Ok(Box::new(uia_driver::UiaDriver::new(pid)?))
}

#[cfg(not(any(target_os = "linux", windows)))]
fn driver_for(_pid: u32) -> Result<Box<dyn Driver>> {
    Err("no accessibility driver on this platform".into())
}

fn area(v: &serde_json::Value) -> Option<Area> {
    let n = |k: &str| v.get(k).and_then(serde_json::Value::as_f64);
    Some(Area {
        x: n("x")?,
        y: n("y")?,
        width: n("width")?,
        height: n("height")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: Role, name: &str, children: Vec<Node>) -> Node {
        Node {
            role,
            name: name.into(),
            description: String::new(),
            value: None,
            text: None,
            states: States::default(),
            rect: None,
            path: vec![],
            children,
        }
    }

    #[test]
    fn finds_a_node_by_role_and_name_at_any_depth() {
        let tree = node(
            Role::Window,
            "Grace",
            vec![node(Role::Group, "", vec![node(Role::Button, "Run all", vec![])])],
        );
        assert!(tree.find(&Role::Button, "Run all").is_some());
        assert!(tree.find(&Role::Button, "Cancel").is_none());
        assert!(tree.find(&Role::Status, "Run all").is_none());
    }

    #[test]
    fn names_the_unnamed_controls_and_only_those() {
        let tree = node(
            Role::Window,
            "Grace",
            vec![
                node(Role::Button, "Open…", vec![]),
                node(Role::Button, " ", vec![]),
                node(Role::Group, "", vec![node(Role::TreeItem, "", vec![])]),
                node(Role::Status, "", vec![]),
                node(Role::Image, "", vec![]),
            ],
        );
        assert_eq!(tree.unnamed_controls().len(), 2);
        assert!(tree.unnamed_controls()[0].starts_with("Button"));
        assert!(tree.unnamed_controls()[1].starts_with("TreeItem"));
    }
}
