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
        crate::a11y::tab_order(div().id("gallery"), &self.focus)
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
                    .child(
                        tooltip("tip-settings", Some("Settings".into())).focusable(|f| {
                            icon_button("icon-medium", Icon::Settings, "Settings", false).track_focus(f)
                        }),
                    )
                    .child(
                        tooltip("tip-delete", Some("Delete".into()))
                            .focusable(|f| icon_button("icon-small", Icon::Trash2, "Delete", true).track_focus(f)),
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
