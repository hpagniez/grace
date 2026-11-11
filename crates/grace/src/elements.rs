//! The small pieces every view shares (premium UI spec): icons, status marks and buttons. Each one is built
//! here once, as `Icon`, `StatusMark` and `buttons.ts` are in Electron Grace (R9.1).

use gpui::accesskit::Role;
use gpui::{
    App, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, prelude::FluentBuilder as _, px, svg,
};
use gpui_component::button::{Button, ButtonVariants as _};

use crate::assets::Icon;
use crate::theme::Theme;

/// A Lucide icon at one of Grace's sizes (12, 14, 16 or 24 px), in the given colour. Decorative: a control that
/// holds only an icon names itself.
pub fn icon(icon: Icon, size: f32, color: Hsla) -> gpui::Svg {
    svg().path(icon.path()).size(px(size)).flex_none().text_color(color)
}

/// What a test is, plus the two outcomes that are not a test's status, and the test a debug run is paused in
/// (`MarkStatus` of `StatusMark.tsx`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkStatus {
    NotRun,
    Pending,
    Running,
    Passed,
    Failed,
    Interrupted,
    Cancelled,
    Skipped,
    Paused,
}

impl MarkStatus {
    pub const ALL: [MarkStatus; 9] = [
        Self::NotRun,
        Self::Pending,
        Self::Running,
        Self::Passed,
        Self::Failed,
        Self::Interrupted,
        Self::Cancelled,
        Self::Skipped,
        Self::Paused,
    ];

    /// Its shape (spec R7.1: never colour alone).
    pub fn icon(self) -> Icon {
        match self {
            Self::NotRun => Icon::Circle,
            Self::Pending => Icon::CircleDashed,
            Self::Running => Icon::LoaderCircle,
            Self::Passed => Icon::CircleCheck,
            Self::Failed => Icon::CircleX,
            Self::Interrupted | Self::Cancelled => Icon::CircleSlash,
            Self::Skipped => Icon::CircleMinus,
            Self::Paused => Icon::Pause,
        }
    }

    /// Its colour token.
    pub fn color(self, theme: &Theme) -> u32 {
        let t = theme.tokens;
        match self {
            Self::NotRun | Self::Pending | Self::Skipped => t.skipped,
            Self::Running | Self::Paused => t.running,
            Self::Passed => t.passed,
            Self::Failed => t.failed,
            Self::Interrupted | Self::Cancelled => t.cancelled,
        }
    }

    /// Its accessible name, the words of `STATUS_LABEL` in `nodes.ts`.
    pub fn label(self) -> &'static str {
        match self {
            Self::NotRun => "Not run",
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Passed => "Passed",
            Self::Failed => "Failed",
            Self::Interrupted => "Interrupted",
            Self::Cancelled => "Cancelled",
            Self::Skipped => "Skipped",
            Self::Paused => "Paused",
        }
    }
}

/// Whether a mark turns: only a running one, and never with reduced motion (ADR 0030).
fn turns(status: MarkStatus, reduce_motion: bool) -> bool {
    status == MarkStatus::Running && !reduce_motion
}

/// A status by shape, colour and accessible name. `decorative` (beside the word that names the status) leaves
/// it out of the accessibility tree. A running mark turns, which only happens during a run (ADR 0030), and not
/// when reduced motion is on.
pub fn status_mark(
    id: impl Into<ElementId>,
    status: MarkStatus,
    size: f32,
    decorative: bool,
    reduce_motion: bool,
    cx: &App,
) -> impl IntoElement {
    let color = Theme::color(status.color(Theme::global(cx)));
    let mark = icon(status.icon(), size, color);
    let mark = if turns(status, reduce_motion) {
        crate::spin::turning(mark).into_any_element()
    } else {
        mark.into_any_element()
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .when(!decorative, |d| {
            d.role(Role::Image).aria_label(SharedString::from(status.label()))
        })
        .child(mark)
}

/// A button's look (`buttons.ts`): bordered, filled green, filled red, or borderless.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    Plain,
    Primary,
    Danger,
    Ghost,
}

/// A button's size: 28 px with 13 px text, 24 px with 12 px text, or a dialog's 28 px with 14 px text (R2.2,
/// R3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonSize {
    Medium,
    Small,
    Dialog,
}

/// A text button. GPUI Kit's `Button` gives it focus, the focus ring, the keyboard and its accessible name.
pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>, kind: ButtonKind, size: ButtonSize) -> Button {
    let m = crate::theme::METRICS;
    let (height, pad, text) = match size {
        ButtonSize::Medium => (m.control_height, 12., m.text_sm.size),
        ButtonSize::Small => (m.control_height_small, 8., m.text_xs.size),
        ButtonSize::Dialog => (m.control_height, 12., m.text_base.size),
    };
    let b = Button::new(id)
        .cursor_pointer()
        .label(label)
        .h(px(height))
        .px(px(pad))
        .text_size(px(text))
        .rounded(px(m.radius));
    match kind {
        ButtonKind::Plain => b.outline(),
        ButtonKind::Primary => b.primary(),
        ButtonKind::Danger => b.danger(),
        ButtonKind::Ghost => b.ghost(),
    }
}

/// An icon-only, borderless button, 28 px square or 24 px when `small` (R10.3). `label` is its accessible name;
/// give it the same tooltip with `tooltip::tooltip(..).focusable(|f| icon_button(..).track_focus(f))`.
pub fn icon_button(id: impl Into<ElementId>, icon: Icon, label: impl Into<SharedString>, small: bool) -> Button {
    let label = label.into();
    let side = if small {
        crate::theme::METRICS.control_height_small
    } else {
        crate::theme::METRICS.control_height
    };
    // sized by hand: GPUI Kit's own square size would draw the icon at three quarters of it, not Lucide's 16 px
    Button::new(id)
        .cursor_pointer()
        .icon(icon)
        .ghost()
        .w(px(side))
        .h(px(side))
        .px_0()
        .accessibility_label(label)
}

/// A button's state, with the cursor that says it (`buttons.ts`): the hand while it can be pressed, the
/// not-allowed sign while it cannot. GPUI Kit's own `disabled` leaves the arrow.
pub trait Off: gpui_component::Disableable + Styled + Sized {
    fn off(self, off: bool) -> Self {
        let b = self.disabled(off);
        if off {
            b.cursor(gpui::CursorStyle::OperationNotAllowed)
        } else {
            b
        }
    }
}

impl Off for Button {}
impl Off for gpui_component::checkbox::Checkbox {}

/// Whether one of Grace's own focusable elements shows the focus ring now: it has the focus and the last input was a
/// key, as a browser's `:focus-visible` (PU-23). GPUI Kit's controls show theirs after a click too.
pub fn ring_shown(focus: &gpui::FocusHandle, window: &gpui::Window) -> bool {
    focus.is_focused(window) && window.last_input_was_keyboard()
}

/// GPUI Kit's focus ring, the one its buttons and boxes draw, on one of Grace's own elements while `shown`: the
/// panel's tabs and its splitter. Its colour is the theme's focus colour at half strength, 3 px outside the element
/// (plan, Redesigns).
pub fn ring<E: Styled + ParentElement>(el: E, shown: bool, window: &gpui::Window, cx: &App) -> E {
    use gpui_component::ThemeStyled as _;
    if shown { el.focus_ring_style(window, cx) } else { el }
}

/// A text box, with the text cursor over it, as a browser's `<input>` has. GPUI Kit gives it only inside an
/// `InputGroup`.
/// A one-line text box `height` px high (`ui.ts`'s `input`, 28 px; 24 px in the console's find bar). GPUI Kit's
/// `Input::h` sizes only a box of many lines, so the height goes through `Styled`, over the small size's 24 px frame.
pub fn input(state: &gpui::Entity<gpui_component::input::InputState>, height: f32) -> gpui_component::input::Input {
    use gpui_component::Sizable as _;
    gpui::Styled::h(
        gpui_component::input::Input::new(state)
            .cursor_text()
            .with_size(gpui_component::Size::Small),
        px(height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UP-13, E-cursor-01: the hand over a button that can be pressed, the not-allowed sign over one that cannot.
    #[test]
    fn a_button_shows_the_hand_and_a_disabled_one_the_not_allowed_sign() {
        use gpui::CursorStyle;
        let cursor = |mut b: Button| b.style().mouse_cursor;
        let plain = || button("b", "Run all", ButtonKind::Primary, ButtonSize::Medium);
        assert_eq!(cursor(plain()), Some(CursorStyle::PointingHand));
        assert_eq!(cursor(plain().off(false)), Some(CursorStyle::PointingHand));
        assert_eq!(cursor(plain().off(true)), Some(CursorStyle::OperationNotAllowed));
        let icon = || icon_button("i", Icon::Play, "Run", true);
        assert_eq!(cursor(icon()), Some(CursorStyle::PointingHand));
        assert_eq!(cursor(icon().off(true)), Some(CursorStyle::OperationNotAllowed));
    }

    /// `StatusMark.tsx`'s `MARK` table, row by row: icon, colour and label.
    #[test]
    fn marks_each_status_as_electron_grace_does() {
        let theme = Theme::new(false);
        let t = theme.tokens;
        let expected = [
            (MarkStatus::NotRun, Icon::Circle, t.skipped, "Not run"),
            (MarkStatus::Pending, Icon::CircleDashed, t.skipped, "Pending"),
            (MarkStatus::Running, Icon::LoaderCircle, t.running, "Running"),
            (MarkStatus::Passed, Icon::CircleCheck, t.passed, "Passed"),
            (MarkStatus::Failed, Icon::CircleX, t.failed, "Failed"),
            (MarkStatus::Interrupted, Icon::CircleSlash, t.cancelled, "Interrupted"),
            (MarkStatus::Cancelled, Icon::CircleSlash, t.cancelled, "Cancelled"),
            (MarkStatus::Skipped, Icon::CircleMinus, t.skipped, "Skipped"),
            (MarkStatus::Paused, Icon::Pause, t.running, "Paused"),
        ];
        for (status, icon, color, label) in expected {
            assert_eq!(
                (status.icon(), status.color(&theme), status.label()),
                (icon, color, label),
                "{status:?}"
            );
        }
        assert_eq!(MarkStatus::ALL.len(), expected.len());
    }

    /// `StatusMark.test.tsx`'s "turns only while running".
    #[test]
    fn turns_only_while_running_and_never_with_reduced_motion() {
        for status in MarkStatus::ALL {
            assert_eq!(turns(status, false), status == MarkStatus::Running, "{status:?}");
            assert!(!turns(status, true), "{status:?}");
        }
    }

    /// Never by colour alone: two statuses that share a colour differ in shape or name.
    #[test]
    fn tells_statuses_apart_by_shape_or_name() {
        for a in MarkStatus::ALL {
            for b in MarkStatus::ALL {
                if a != b {
                    assert!(a.icon() != b.icon() || a.label() != b.label(), "{a:?} {b:?}");
                }
            }
        }
    }

    /// `buttons.test.ts`, `Icon.test.tsx` and `StatusMark.test.tsx`'s sizes: a dialog button is 28 px high with
    /// 14 px text, a small icon button 24 × 24, an icon 16 px unless asked, a status mark the size asked.
    #[gpui::test]
    fn draws_buttons_icons_and_marks_at_their_sizes(cx: &mut gpui::TestAppContext) {
        use gpui::VisualTestContext;
        struct View;
        impl gpui::Render for View {
            fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
                let at = |name: &'static str| div().flex().debug_selector(move || name.into());
                div()
                    .flex()
                    .flex_col()
                    .items_start()
                    .child(at("dialog").child(button("d", "Save", ButtonKind::Primary, ButtonSize::Dialog)))
                    .child(at("small").child(icon_button("s", Icon::Play, "Run", true)))
                    .child(at("icon").child(icon(Icon::Play, 16., Theme::color(Theme::global(cx).tokens.ink))))
                    .child(at("mark-14").child(status_mark("m14", MarkStatus::Passed, 14., false, true, cx)))
                    .child(at("mark-16").child(status_mark("m16", MarkStatus::Passed, 16., false, true, cx)))
            }
        }
        cx.update(crate::init);
        let window = cx.add_window(|_, _| View);
        let mut cx = VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        let size = |cx: &mut VisualTestContext, name: &'static str| {
            let b = cx.debug_bounds(name).unwrap();
            (f32::from(b.size.width), f32::from(b.size.height))
        };
        assert_eq!(size(&mut cx, "dialog").1, crate::theme::METRICS.control_height);
        assert_eq!(size(&mut cx, "small"), (24., 24.));
        assert_eq!(size(&mut cx, "icon"), (16., 16.));
        assert_eq!(size(&mut cx, "mark-14"), (14., 14.));
        assert_eq!(size(&mut cx, "mark-16"), (16., 16.));
        assert_eq!(crate::theme::METRICS.text_base.size, 14., "a dialog button's text");
    }
}
