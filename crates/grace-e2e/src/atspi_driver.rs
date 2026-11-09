//! The Linux backend: AT-SPI over D-Bus, which GPUI's AccessKit adapter serves (prototype a11y-linux/). Needs a
//! session bus with the AT-SPI bus launcher, as `scripts/e2e-linux.sh` starts them.
use atspi::proxy::accessible::{AccessibleProxy, ObjectRefExt as _};
use atspi::proxy::action::ActionProxy;
use atspi::proxy::component::ComponentProxy;
use atspi::proxy::text::TextProxy;
use atspi::proxy::value::ValueProxy;
use atspi::{AccessibilityConnection, CoordType, Interface, ObjectRefOwned, State};
use futures_lite::future::block_on;

use crate::{Driver, Key, Node, Rect, Result, Role, States};

pub struct AtspiDriver {
    conn: AccessibilityConnection,
    pid: u32,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Turns accessibility on for the session, so AccessKit in the app serves its tree. Before the app starts.
pub fn prepare() -> Result<()> {
    block_on(atspi::connection::set_session_accessibility(true)).map_err(err)
}

impl AtspiDriver {
    pub fn new(pid: u32) -> Result<Self> {
        let conn = block_on(AccessibilityConnection::new()).map_err(err)?;
        Ok(Self { conn, pid })
    }

    /// The app's first window: the registry's application whose bus connection belongs to our process.
    async fn window(&self) -> Result<ObjectRefOwned> {
        let bus = self.conn.connection();
        let dbus = atspi::zbus::fdo::DBusProxy::new(bus).await.map_err(err)?;
        let root = self.conn.root_accessible_on_registry().await.map_err(err)?;
        for app in root.get_children().await.map_err(err)? {
            let Some(name) = app.name() else { continue };
            let Ok(pid) = dbus.get_connection_unix_process_id(name.clone().into()).await else {
                continue;
            };
            if pid != self.pid {
                continue;
            }
            let app = app.as_accessible_proxy(bus).await.map_err(err)?;
            if let Some(window) = app.get_children().await.map_err(err)?.into_iter().next() {
                return Ok(window);
            }
        }
        Err(format!(
            "process {} has no window in the accessibility tree yet",
            self.pid
        ))
    }

    async fn proxy(&self, at: &ObjectRefOwned) -> Result<AccessibleProxy<'_>> {
        at.clone()
            .into_accessible_proxy(self.conn.connection())
            .await
            .map_err(err)
    }

    async fn read(&self, at: &ObjectRefOwned, path: Vec<usize>) -> Result<Node> {
        let p = self.proxy(at).await?;
        let role = role(p.get_role().await.map_err(err)?);
        let state = p.get_state().await.map_err(err)?;
        let interfaces = p.get_interfaces().await.map_err(err)?;
        let value = if interfaces.contains(Interface::Value) {
            let v = ValueProxy::builder(self.conn.connection())
                .destination(p.inner().destination().to_owned())
                .map_err(err)?
                .path(p.inner().path().to_owned())
                .map_err(err)?
                .build()
                .await
                .map_err(err)?;
            v.current_value().await.ok()
        } else {
            None
        };
        let text = if interfaces.contains(Interface::Text) {
            let t = TextProxy::builder(self.conn.connection())
                .destination(p.inner().destination().to_owned())
                .map_err(err)?
                .path(p.inner().path().to_owned())
                .map_err(err)?
                .build()
                .await
                .map_err(err)?;
            t.get_text(0, -1).await.ok()
        } else {
            None
        };
        let rect = if interfaces.contains(Interface::Component) {
            let c = ComponentProxy::builder(self.conn.connection())
                .destination(p.inner().destination().to_owned())
                .map_err(err)?
                .path(p.inner().path().to_owned())
                .map_err(err)?
                .build()
                .await
                .map_err(err)?;
            c.get_extents(CoordType::Window)
                .await
                .ok()
                .map(|(x, y, width, height)| Rect { x, y, width, height })
        } else {
            None
        };
        let mut children = Vec::new();
        for (i, child) in p.get_children().await.map_err(err)?.into_iter().enumerate() {
            let mut at = path.clone();
            at.push(i);
            children.push(Box::pin(self.read(&child, at)).await?);
        }
        Ok(Node {
            role,
            name: p.name().await.map_err(err)?,
            description: p.description().await.unwrap_or_default(),
            value,
            text,
            states: States {
                enabled: state.contains(State::Enabled),
                focusable: state.contains(State::Focusable),
                focused: state.contains(State::Focused),
                selected: state.contains(State::Selected),
                expanded: state
                    .contains(State::Expandable)
                    .then(|| state.contains(State::Expanded)),
            },
            rect,
            path,
            children,
        })
    }
}

impl AtspiDriver {
    /// The app's X window, by its process.
    fn x_window(&self) -> Result<String> {
        let found = std::process::Command::new("xdotool")
            .args(["search", "--onlyvisible", "--pid", &self.pid.to_string()])
            .output()
            .map_err(|e| format!("cannot run xdotool (apt install xdotool): {e}"))?;
        String::from_utf8_lossy(&found.stdout)
            .lines()
            .next()
            .map(str::to_owned)
            .ok_or_else(|| format!("process {} shows no X window", self.pid))
    }
}

impl Driver for AtspiDriver {
    fn tree(&self) -> Result<Node> {
        block_on(async {
            let window = self.window().await?;
            self.read(&window, vec![]).await
        })
    }

    fn press(&self, node: &Node) -> Result<()> {
        block_on(async {
            let mut at = self.window().await?;
            for &i in &node.path {
                let p = self.proxy(&at).await?;
                at = p.get_child_at_index(i as i32).await.map_err(err)?;
            }
            let p = self.proxy(&at).await?;
            let action = ActionProxy::builder(self.conn.connection())
                .destination(p.inner().destination().to_owned())
                .map_err(err)?
                .path(p.inner().path().to_owned())
                .map_err(err)?
                .build()
                .await
                .map_err(err)?;
            match action.do_action(0).await.map_err(err)? {
                true => Ok(()),
                false => Err(format!("{:?} {:?} did not take its action", node.role, node.name)),
            }
        })
    }

    /// Through `xdotool`, on the X display the app is on: AT-SPI has no pointer of its own. The node's place is
    /// read in the window's coordinates, since the app cannot know where an X window without a window manager is.
    fn hover(&self, node: &Node) -> Result<()> {
        let (x, y) = node
            .rect
            .ok_or_else(|| format!("{:?} {:?} has no place in the window", node.role, node.name))?
            .centre();
        let window = self.x_window()?;
        xdotool(&["mousemove", "--window", &window, &x.to_string(), &y.to_string()])
    }

    /// To the app's window, given the keyboard focus first: with no window manager, nothing else gives it.
    fn key(&self, key: Key) -> Result<()> {
        let window = self.x_window()?;
        xdotool(&["windowfocus", "--sync", &window])?;
        xdotool(&[
            "key",
            match key {
                Key::Tab => "Tab",
                Key::Escape => "Escape",
                Key::Enter => "Return",
                Key::Space => "space",
                Key::Up => "Up",
                Key::Down => "Down",
                Key::Left => "Left",
                Key::Right => "Right",
                Key::Home => "Home",
                Key::End => "End",
                Key::PageUp => "Prior",
                Key::PageDown => "Next",
                Key::CtrlEnter => "ctrl+Return",
                Key::SelectAll => "ctrl+a",
                Key::Backspace => "BackSpace",
                Key::ShiftEnter => "shift+Return",
                Key::CtrlF => "ctrl+f",
                Key::F8 => "F8",
                Key::ShiftF8 => "shift+F8",
                Key::AltLeft => "alt+Left",
                Key::AltRight => "alt+Right",
            },
        ])
    }

    fn click(&self, node: &Node) -> Result<()> {
        self.hover(node)?;
        let window = self.x_window()?;
        xdotool(&["windowfocus", "--sync", &window])?;
        xdotool(&["click", "1"])
    }

    fn type_text(&self, text: &str) -> Result<()> {
        let window = self.x_window()?;
        xdotool(&["windowfocus", "--sync", &window])?;
        // "--" ends the options, so a text that starts with "-" is typed, not read as one
        xdotool(&["type", "--delay", "20", "--", text])
    }

    fn close(&self) -> Result<()> {
        let window = self.x_window()?;
        ask_to_close(window.parse().map_err(err)?)
    }
}

/// `WM_DELETE_WINDOW`, what a window manager sends when the close button is pressed. `xdotool windowclose` would
/// destroy the window instead, which the app never hears of. The request is checked, a round trip, before the
/// connection closes: a connection closed right after its last request could lose it, and Grace then never heard
/// of the close (about one run in six under Xvfb).
fn ask_to_close(window: u32) -> Result<()> {
    use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt as _, EventMask};
    let (conn, _) = x11rb::connect(None).map_err(err)?;
    let atom =
        |name: &[u8]| -> Result<u32> { Ok(conn.intern_atom(false, name).map_err(err)?.reply().map_err(err)?.atom) };
    let (protocols, delete) = (atom(b"WM_PROTOCOLS")?, atom(b"WM_DELETE_WINDOW")?);
    let event = ClientMessageEvent::new(32, window, protocols, [delete, x11rb::CURRENT_TIME, 0, 0, 0]);
    conn.send_event(false, window, EventMask::NO_EVENT, event)
        .map_err(err)?
        .check()
        .map_err(err)
}

fn xdotool(args: &[&str]) -> Result<()> {
    let out = std::process::Command::new("xdotool")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run xdotool (apt install xdotool): {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "xdotool {args:?}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// AT-SPI's roles to the driver's. GPUI's AccessKit adapter answers `GetRole` but not `GetRoleName`.
fn role(role: atspi::Role) -> Role {
    use atspi::Role as A;
    match role {
        // AccessKit gives an `Application` node AT-SPI's `Embedded` role
        A::Application | A::Embedded => Role::Application,
        A::Frame | A::Window => Role::Window,
        A::Panel | A::Section | A::Filler | A::Grouping => Role::Group,
        A::Button | A::ToggleButton => Role::Button,
        A::CheckBox => Role::CheckBox,
        A::RadioButton => Role::RadioButton,
        A::Entry | A::PasswordText => Role::Entry,
        A::ComboBox => Role::ComboBox,
        A::SpinButton => Role::SpinButton,
        A::Slider => Role::Slider,
        A::PageTab => Role::Tab,
        A::PageTabList => Role::TabList,
        A::Tree | A::TreeTable => Role::Tree,
        A::TreeItem => Role::TreeItem,
        A::List | A::ListBox => Role::List,
        A::ListItem => Role::ListItem,
        A::Link => Role::Link,
        A::MenuItem => Role::MenuItem,
        A::StatusBar => Role::Status,
        A::ProgressBar => Role::ProgressBar,
        A::Image | A::Icon => Role::Image,
        A::Label | A::Static | A::Text => Role::Text,
        A::Dialog | A::Alert => Role::Dialog,
        A::ToolTip => Role::Tooltip,
        other => Role::Other(format!("{other:?}")),
    }
}
