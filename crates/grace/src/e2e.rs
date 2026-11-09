//! Test hooks for crates/grace-e2e, only in a build with the `e2e` feature (migration guide, Architecture).
//!
//! `GRACE_E2E_VIEW=elements` opens a window with every shared element instead of Grace's view, so the tree check
//! (guide R8.2) and the press test have a subject before the real views are ported (phases P2 to P4).
use gpui::accesskit::Role;
use gpui::{
    Context, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div,
};

use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, MarkStatus, button, icon_button, status_mark};
use crate::theme::Theme;

/// Which view the window shows.
pub fn wants_gallery() -> bool {
    std::env::var("GRACE_E2E_VIEW").as_deref() == Ok("elements")
}

/// Every shared element, and one button that counts its presses.
#[derive(Default)]
pub struct Gallery {
    presses: usize,
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        div()
            .id("gallery")
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
                    .child(icon_button("icon-medium", Icon::Settings, "Settings", false))
                    .child(icon_button("icon-small", Icon::Trash2, "Delete", true)),
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
    }
}
