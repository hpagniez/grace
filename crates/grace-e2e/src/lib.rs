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
}

/// One node of the tree, as read. `path` is its position under the app's window, which `Driver::press` finds again.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub role: Role,
    pub name: String,
    pub description: String,
    pub value: Option<f64>,
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
}

impl App {
    /// Starts Grace with a fresh data folder (`GRACE_USER_DATA`), its window on a screen other than the main one
    /// (`GRACE_E2E_SCREEN`, read by the `e2e` feature), as `e2e/helpers.ts` `launch()` does.
    pub fn launch() -> Result<App> {
        Self::launch_with(&[])
    }

    pub fn launch_with(env: &[(&str, &str)]) -> Result<App> {
        let exe = grace_exe();
        #[cfg(target_os = "linux")]
        atspi_driver::prepare()?;
        let data = std::env::temp_dir().join(format!("grace-e2e-{}-{}", std::process::id(), unique()));
        std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&exe);
        cmd.env("GRACE_USER_DATA", &data).env("GRACE_E2E_SCREEN", "other");
        for (k, v) in env {
            cmd.env(k, v);
        }
        let child = cmd
            .spawn()
            .map_err(|e| format!("cannot start {}: {e}", exe.display()))?;
        let driver = driver_for(child.id())?;
        Ok(App { child, data, driver })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn tree(&self) -> Result<Node> {
        self.driver.tree()
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

    pub fn hover(&self, node: &Node) -> Result<()> {
        self.driver.hover(node)
    }

    pub fn key(&self, key: Key) -> Result<()> {
        self.driver.key(key)
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
        let _ = std::fs::remove_dir_all(&self.data);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(role: Role, name: &str, children: Vec<Node>) -> Node {
        Node {
            role,
            name: name.into(),
            description: String::new(),
            value: None,
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
