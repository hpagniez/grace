//! The Windows backend: UI Automation, which GPUI's AccessKit adapter serves. Reads the raw view, so nothing the
//! app exposes is filtered out.
use uiautomation::inputs::{Keyboard, Mouse};
use uiautomation::patterns::{
    UIExpandCollapsePattern, UIInvokePattern, UILegacyIAccessiblePattern, UIRangeValuePattern, UISelectionItemPattern,
    UITogglePattern, UIValuePattern,
};
use uiautomation::types::{ControlType, ExpandCollapseState, Point};
use uiautomation::{UIAutomation, UIElement, UITreeWalker};

use crate::{Driver, Key, Node, Rect, Result, Role, States};

pub struct UiaDriver {
    automation: UIAutomation,
    walker: UITreeWalker,
    pid: u32,
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Before a launch: a key press, F24, which no window acts on. Windows lets a process take the foreground only if the
/// one that started it had input of late; without it the first Grace a test process starts may open behind another
/// window, take keys only once `to_foreground` forces it, and never tell UI Automation where its focus is (#301).
pub fn prepare() -> Result<()> {
    Keyboard::new().send_keys("{F24}").map_err(err)
}

impl UiaDriver {
    pub fn new(pid: u32) -> Result<Self> {
        let automation = UIAutomation::new().map_err(err)?;
        let walker = automation.get_raw_view_walker().map_err(err)?;
        Ok(Self {
            automation,
            walker,
            pid,
        })
    }

    /// The app's first top-level window: a child of the desktop that belongs to our process.
    fn window(&self) -> Result<UIElement> {
        let root = self.automation.get_root_element().map_err(err)?;
        for top in self.children(&root) {
            if top.get_process_id().ok() == Some(self.pid) {
                return Ok(top);
            }
        }
        Err(format!(
            "process {} has no window in the accessibility tree yet",
            self.pid
        ))
    }

    /// Brings the window to the foreground and checks that it has the focus. Windows' foreground lock can refuse a
    /// process that had no input of late, and `SetFocus` then still answers success while the keys would go to
    /// another window; a key press counts as input and lifts the lock, so the next try takes. The key is F24, which no
    /// window acts on: a tap of Alt, which this sent before, put a window that had the focus after all in its menu
    /// mode, and the Tabs after it never reached Grace's controls (#301). A window that has the focus already is left
    /// alone: `SetFocus` on it would move the focus to the window from the control that holds it.
    fn to_foreground(&self) -> Result<()> {
        let ours = || {
            let focused = self.automation.get_focused_element().ok();
            focused.and_then(|f| f.get_process_id().ok()) == Some(self.pid)
        };
        if ours() {
            return Ok(());
        }
        let window = self.window()?;
        for _ in 0..5 {
            window
                .set_focus()
                .map_err(|e| format!("cannot bring the window to the foreground: {e}"))?;
            if ours() {
                return Ok(());
            }
            Keyboard::new().send_keys("{F24}").map_err(err)?;
        }
        Err(format!("process {}'s window does not take the focus", self.pid))
    }

    fn children(&self, of: &UIElement) -> Vec<UIElement> {
        let mut out = Vec::new();
        let mut next = self.walker.get_first_child(of).ok();
        while let Some(e) = next {
            next = self.walker.get_next_sibling(&e).ok();
            out.push(e);
        }
        out
    }

    fn read(&self, e: &UIElement, path: Vec<usize>) -> Result<Node> {
        let value = e
            .get_pattern::<UIRangeValuePattern>()
            .ok()
            .and_then(|p| p.get_value().ok());
        let selected = e
            .get_pattern::<UISelectionItemPattern>()
            .ok()
            .and_then(|p| p.is_selected().ok());
        let expanded = e
            .get_pattern::<UIExpandCollapsePattern>()
            .ok()
            .and_then(|p| p.get_state().ok())
            .and_then(|s| match s {
                ExpandCollapseState::Expanded | ExpandCollapseState::PartiallyExpanded => Some(true),
                ExpandCollapseState::Collapsed => Some(false),
                _ => None,
            });
        let description = e
            .get_pattern::<UILegacyIAccessiblePattern>()
            .ok()
            .and_then(|p| p.get_description().ok())
            .filter(|d| !d.is_empty())
            .or_else(|| e.get_help_text().ok())
            .unwrap_or_default();
        // a text box's text, through the Value pattern a button or a check box does not have
        let text = matches!(
            e.get_control_type(),
            Ok(ControlType::Edit | ControlType::ComboBox | ControlType::Spinner)
        )
        .then(|| e.get_pattern::<UIValuePattern>().ok()?.get_value().ok())
        .flatten();
        let children = self
            .children(e)
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut at = path.clone();
                at.push(i);
                self.read(c, at)
            })
            .collect::<Result<Vec<_>>>()?;
        let rect = e.get_bounding_rectangle().ok().map(|r| Rect {
            x: r.get_left(),
            y: r.get_top(),
            width: r.get_width(),
            height: r.get_height(),
        });
        Ok(Node {
            // AccessKit gives a dialog UI Automation's Window type, as Chromium does: one inside the window is a dialog
            role: match role(e.get_control_type().map_err(err)?) {
                Role::Window if !path.is_empty() => Role::Dialog,
                r => r,
            },
            name: e.get_name().unwrap_or_default(),
            description,
            value,
            text,
            states: States {
                enabled: e.is_enabled().unwrap_or(false),
                focusable: e.is_keyboard_focusable().unwrap_or(false),
                focused: e.has_keyboard_focus().unwrap_or(false),
                selected: selected.unwrap_or(false),
                expanded,
            },
            rect,
            path,
            children,
        })
    }
}

impl Driver for UiaDriver {
    fn tree(&self) -> Result<Node> {
        let window = self.window()?;
        self.read(&window, vec![])
    }

    fn press(&self, node: &Node) -> Result<()> {
        let mut at = self.window()?;
        for &i in &node.path {
            at = self
                .children(&at)
                .into_iter()
                .nth(i)
                .ok_or_else(|| format!("{:?} is gone", node.path))?;
        }
        // a click brings its window to the foreground first: a dialog opened in a window that is not active never
        // gets the focus, and the keys after it went nowhere (#301)
        self.to_foreground()?;
        // as a click would: AccessKit gives Invoke to a button, Toggle to a check box, and neither to a button that
        // opens something (ExpandCollapse) or to a tree item; a tree item takes a click of the pointer on its centre
        if let Ok(invoke) = at.get_pattern::<UIInvokePattern>() {
            return invoke.invoke().map_err(err);
        }
        if let Ok(toggle) = at.get_pattern::<UITogglePattern>() {
            return toggle.toggle().map_err(err);
        }
        // a button that opens something takes the focus and Space, as from the keyboard: its centre can be under
        // another part of the window when its scroll area clips it (a step's log button below a long error)
        if node.role == Role::Button && at.get_pattern::<UIExpandCollapsePattern>().is_ok() {
            at.set_focus().map_err(err)?;
            return Keyboard::new().send_keys("{space}").map_err(err);
        }
        let r = at
            .get_bounding_rectangle()
            .map_err(|e| format!("{:?} {:?}: {e}", node.role, node.name))?;
        let centre = Point::new(r.get_left() + r.get_width() / 2, r.get_top() + r.get_height() / 2);
        Mouse::new().click(&centre).map_err(err)
    }

    fn hover(&self, node: &Node) -> Result<()> {
        let (x, y) = node
            .rect
            .ok_or_else(|| format!("{:?} {:?} has no place on the screen", node.role, node.name))?
            .centre();
        Mouse::new().move_to(&Point::new(x, y)).map_err(err)
    }

    /// To the app's window, brought to the foreground first: keys go to the foreground window, and a test that
    /// only pressed through the tree never made it so.
    fn key(&self, key: Key) -> Result<()> {
        self.to_foreground()?;
        Keyboard::new()
            .send_keys(match key {
                Key::Tab => "{tab}",
                Key::Escape => "{esc}",
                Key::Enter => "{enter}",
                Key::Space => "{space}",
                Key::Up => "{up}",
                Key::Down => "{down}",
                Key::Left => "{left}",
                Key::Right => "{right}",
                Key::Home => "{home}",
                Key::End => "{end}",
                Key::PageUp => "{pgup}",
                Key::PageDown => "{pgdn}",
                Key::CtrlEnter => "{ctrl}{enter}",
                Key::SelectAll => "{ctrl}a",
                Key::Backspace => "{backspace}",
                Key::ShiftEnter => "{shift}{enter}",
                Key::CtrlF => "{ctrl}f",
                Key::F8 => "{F8}",
                Key::ShiftF8 => "{shift}{F8}",
                Key::AltLeft => "{alt}{left}",
                Key::AltRight => "{alt}{right}",
            })
            .map_err(err)
    }

    fn click(&self, node: &Node) -> Result<()> {
        let (x, y) = node
            .rect
            .ok_or_else(|| format!("{:?} {:?} has no place on the screen", node.role, node.name))?
            .centre();
        self.to_foreground()?;
        Mouse::new().click(&Point::new(x, y)).map_err(err)
    }

    fn type_text(&self, text: &str) -> Result<()> {
        self.to_foreground()?;
        Keyboard::new().send_text(text).map_err(err)
    }

    /// The window pattern's Close: `WM_CLOSE`, as the title bar's button sends.
    fn focus_report(&self) -> String {
        let describe = |e: &UIElement| {
            format!(
                "{:?} {:?} pid {:?} framework {:?}",
                e.get_control_type().ok(),
                e.get_name().ok(),
                e.get_process_id().ok(),
                e.get_framework_id().ok()
            )
        };
        let focused = self.automation.get_focused_element().map(|f| {
            let is_window = self
                .window()
                .ok()
                .and_then(|w| self.automation.compare_elements(&w, &f).ok());
            format!("{}, the window itself: {is_window:?}", describe(&f))
        });
        format!("UI Automation's focus: {focused:?} (our pid {})", self.pid)
    }

    fn close(&self) -> Result<()> {
        self.window()?
            .get_pattern::<uiautomation::patterns::UIWindowPattern>()
            .map_err(err)?
            .close()
            .map_err(err)
    }
}

/// UI Automation's control types to the driver's roles.
fn role(t: ControlType) -> Role {
    match t {
        ControlType::Window => Role::Window,
        ControlType::Pane | ControlType::Group | ControlType::Custom => Role::Group,
        ControlType::Button | ControlType::SplitButton => Role::Button,
        ControlType::CheckBox => Role::CheckBox,
        ControlType::RadioButton => Role::RadioButton,
        ControlType::Edit => Role::Entry,
        ControlType::ComboBox => Role::ComboBox,
        ControlType::Spinner => Role::SpinButton,
        ControlType::Slider => Role::Slider,
        ControlType::TabItem => Role::Tab,
        ControlType::Tab => Role::TabList,
        ControlType::Tree => Role::Tree,
        ControlType::TreeItem => Role::TreeItem,
        ControlType::List => Role::List,
        ControlType::ListItem | ControlType::DataItem => Role::ListItem,
        ControlType::Hyperlink => Role::Link,
        ControlType::MenuItem => Role::MenuItem,
        ControlType::StatusBar => Role::Status,
        ControlType::ProgressBar => Role::ProgressBar,
        ControlType::Image => Role::Image,
        ControlType::Text => Role::Text,
        ControlType::ToolTip => Role::Tooltip,
        other => Role::Other(format!("{other:?}")),
    }
}
