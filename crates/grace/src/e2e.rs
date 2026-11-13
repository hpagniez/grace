//! Test hooks for crates/grace-e2e, only in a build with the `e2e` feature (migration guide, Architecture).
//!
//! `GRACE_E2E_VIEW=elements` opens a window with every shared element instead of Grace's view, so the tree check
//! (guide R8.2), the press test and the tooltip tests (PU-19 to PU-21) have a subject before the real views are ported
//! (phases P2 to P4).
use gpui::accesskit::Role;
use gpui::{
    Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, px,
};
use gpui_component::Disableable as _;
use gpui_component::WindowExt as _;

use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, MarkStatus, button, icon_button, status_mark};
use crate::theme::Theme;
use crate::tooltip::{Tips, cut, tooltip};

/// With `GRACE_E2E_KEYS` set, every keystroke a window takes and the action it ran, on stderr: what a test that
/// sends keys prints when a key seems lost.
pub fn log_keys(cx: &mut gpui::App) {
    if std::env::var_os("GRACE_E2E_KEYS").is_none() {
        return;
    }
    cx.observe_keystrokes(|event, _, _| {
        let action = event.action.as_ref().map(|a| a.name());
        eprintln!("E2E key {:?} -> {action:?}", event.keystroke);
    })
    .detach();
}

/// Writes the main screen's work area to `<data folder>/e2e-screen.json`, in GPUI's coordinates, the window's
/// bounds' own: the window tests place their windows from it, as `window.spec.ts` read Electron's `workArea`.
pub fn record_screen(data_dir: &std::path::Path, cx: &gpui::App) {
    let Some(display) = cx.primary_display() else {
        return;
    };
    let b = display.visible_bounds();
    let n = |p: gpui::Pixels| f64::from(f32::from(p));
    let area = serde_json::json!({
        "x": n(b.origin.x),
        "y": n(b.origin.y),
        "width": n(b.size.width),
        "height": n(b.size.height),
    });
    let _ = std::fs::write(data_dir.join("e2e-screen.json"), area.to_string());
}

/// Which view the window shows.
pub fn wants_gallery() -> bool {
    std::env::var("GRACE_E2E_VIEW").as_deref() == Ok("elements")
}

/// Every shared element, and one button that counts its presses.
pub struct Gallery {
    presses: usize,
    tips: Option<Entity<Tips>>,
    pub focus: gpui::FocusHandle,
}

impl Gallery {
    pub fn new(cx: &mut gpui::App) -> Self {
        Self {
            presses: 0,
            tips: None,
            focus: cx.focus_handle(),
        }
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_rem_size(gpui::px(crate::window::REM));
        let tips = self
            .tips
            .get_or_insert_with(|| crate::tooltip::install(window, cx))
            .clone();
        let t = Theme::global(cx).tokens;
        let pressed: SharedString = match self.presses {
            1 => "Pressed 1 time".into(),
            n => format!("Pressed {n} times").into(),
        };
        let kinds = [
            ("Plain", ButtonKind::Plain),
            ("Primary", ButtonKind::Primary),
            ("Danger", ButtonKind::Danger),
            ("Ghost", ButtonKind::Ghost),
        ];
        let sizes = [
            ("medium", ButtonSize::Medium),
            ("small", ButtonSize::Small),
            ("dialog", ButtonSize::Dialog),
        ];
        let mut buttons = div().flex().flex_wrap().gap_2();
        for (kind_name, kind) in kinds {
            for (size_name, size) in sizes {
                let label = format!("{kind_name} {size_name}");
                buttons = buttons.child(button(SharedString::from(label.clone()), label, kind, size));
            }
        }
        let mut marks = div().flex().gap_2();
        for (i, status) in MarkStatus::ALL.into_iter().enumerate() {
            marks = marks.child(status_mark(("mark", i), status, 16., false, false, cx));
        }
        crate::a11y::tab_order(div().id("gallery"), &self.focus, None)
            .role(Role::Group)
            .aria_label("Elements")
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .bg(Theme::color(t.surface))
            .child(buttons)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(tooltip("tip-settings", Some("Settings".into())).focusable(|f| {
                        icon_button("icon-medium", Icon::Settings, 16., "Settings", false).track_focus(f)
                    }))
                    .child(
                        tooltip("tip-delete", Some("Delete".into()))
                            .focusable(|f| icon_button("icon-small", Icon::Trash2, 16., "Delete", true).track_focus(f)),
                    ),
            )
            // PU-20: a disabled button shows its reason on hover; a line too long for its room shows whole
            .child(
                div()
                    .flex()
                    .gap_2()
                    .items_center()
                    .child(
                        tooltip("tip-run", Some("Open a project first".into())).child(
                            button("run-all", "Run all", ButtonKind::Primary, ButtonSize::Medium).disabled(true),
                        ),
                    )
                    .child(
                        div()
                            .w(px(120.))
                            .child(cut("cut-line", "A project name far too long to fit in its row")),
                    ),
            )
            // PU-21: Escape in a dialog closes the tooltip first
            .child(
                button("open-dialog", "Open dialog", ButtonKind::Plain, ButtonSize::Medium).on_click(
                    |_, window, cx| {
                        window.open_dialog(cx, |dialog, _, _| {
                            dialog.title("Dialog").child(
                                tooltip("tip-in-dialog", Some("Does nothing".into())).focusable(|f| {
                                    button("in-dialog", "In dialog", ButtonKind::Plain, ButtonSize::Dialog)
                                        .track_focus(f)
                                }),
                            )
                        });
                    },
                ),
            )
            .child(marks)
            .child(
                button("counter", pressed, ButtonKind::Primary, ButtonSize::Medium).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.presses += 1;
                        cx.notify();
                    },
                )),
            )
            .child(tips)
    }
}

/// The seams' answers for an end-to-end test (migration guide, Architecture). `GRACE_E2E_SEAMS` names a JSON file the
/// test writes, read at each ask so a test can change the answer between two: `{"folder": "<path>" | null, "file": "<path>" | null, "vscode": false}`. Each ask
/// is appended to `<data folder>/e2e-seams.jsonl`, `{"seam": "folder", "start": "<path>" | null}`, as the Playwright
/// tests recorded `dialog.showOpenDialog`'s options.
pub struct E2eSeams {
    answers: std::path::PathBuf,
    log: std::path::PathBuf,
}

impl E2eSeams {
    pub fn from_env() -> Option<Self> {
        let answers = std::env::var_os("GRACE_E2E_SEAMS")?.into();
        let log = grace_core::paths::data_dir()?.join("e2e-seams.jsonl");
        Some(Self { answers, log })
    }

    fn record(&self, line: serde_json::Value) {
        use std::io::Write as _;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.log) {
            let _ = writeln!(f, "{line}");
        }
    }
}

impl crate::seams::Seams for E2eSeams {
    fn pick_folder(
        &self,
        start: Option<std::path::PathBuf>,
        _cx: &mut gpui::AsyncApp,
    ) -> crate::seams::Answer<Option<std::path::PathBuf>> {
        self.record(serde_json::json!({ "seam": "folder", "start": start }));
        let answer = std::fs::read_to_string(&self.answers)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("folder")?.as_str().map(std::path::PathBuf::from));
        Box::pin(async move { answer })
    }

    /// `{"file": "<path>" | null}`: the file Import from file… picks.
    fn pick_file(&self, _cx: &mut gpui::AsyncApp) -> crate::seams::Answer<Option<std::path::PathBuf>> {
        self.record(serde_json::json!({ "seam": "file" }));
        let answer = std::fs::read_to_string(&self.answers)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("file")?.as_str().map(std::path::PathBuf::from));
        Box::pin(async move { answer })
    }

    /// `{"vscode": false}`: no program is registered for a link, as on a machine without VS Code (OE-11); nothing is
    /// recorded then. Otherwise the link is recorded, as `shell.openExternal` was stubbed, never opened.
    fn open_url(&self, url: &str, _cx: &mut gpui::App) -> bool {
        let installed = std::fs::read_to_string(&self.answers)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("vscode")?.as_bool())
            .unwrap_or(true);
        if installed {
            self.record(serde_json::json!({ "seam": "url", "url": url }));
        }
        installed
    }

    /// Records the path instead of opening a browser on the test machine.
    fn open_path(&self, path: &std::path::Path, _cx: &mut gpui::App) {
        self.record(serde_json::json!({ "seam": "open", "path": path }));
    }
}
