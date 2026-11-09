//! Small shared views: `Notice.tsx` and `EmptyState.tsx`.
use gpui::accesskit::{Live, Role};
use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, prelude::FluentBuilder as _, px,
};

use crate::assets::Icon;
use crate::elements::icon;
use crate::theme::{METRICS, Theme};

/// A notice's kind: a success or a warning is a status, an error an alert (premium UI R20.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Success,
    Warning,
    Error,
}

impl NoticeKind {
    /// Its tint, its colour and its icon (`Notice.tsx`'s `KIND`).
    pub fn look(self, cx: &App) -> (u32, u32, Icon) {
        let t = Theme::global(cx).tokens;
        match self {
            NoticeKind::Success => (t.passed_tint, t.passed, Icon::CircleCheck),
            NoticeKind::Warning => (t.cancelled_tint, t.cancelled, Icon::TriangleAlert),
            NoticeKind::Error => (t.failed_tint, t.failed, Icon::CircleAlert),
        }
    }

    pub fn role(self) -> Role {
        match self {
            NoticeKind::Error => Role::Alert,
            _ => Role::Status,
        }
    }
}

/// A tinted message with its icon. `children`, such as a button, follow the text in the same box. Its text wraps, so a
/// long path never overflows. It is a live region: a screen reader says a new notice.
pub fn notice(
    id: impl Into<ElementId>,
    kind: NoticeKind,
    text: impl Into<SharedString>,
    children: Vec<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let (tint, tone, mark) = kind.look(cx);
    let t = Theme::global(cx).tokens;
    let text = text.into();
    crate::a11y::live(
        div()
            .id(id)
            .debug_selector(|| "notice".into())
            .role(kind.role())
            .aria_label(text.clone())
            .flex()
            .items_start()
            .gap_2()
            .rounded(px(METRICS.radius_lg))
            .px_3()
            .py_2()
            .text_size(px(METRICS.text_sm.size))
            .bg(Theme::color(tint))
            .child(div().mt(px(2.)).child(icon(mark, 16., Theme::color(tone))))
            .child(div().min_w_0().flex_1().text_color(Theme::color(t.ink)).child(text))
            .children(children),
        if kind == NoticeKind::Error {
            Live::Assertive
        } else {
            Live::Polite
        },
    )
}

/// An empty list or panel: one muted 24 px icon, one sentence and, when there is one, the action that fills it
/// (premium UI R6.5).
pub fn empty_state(
    id: impl Into<ElementId>,
    mark: Icon,
    text: impl Into<SharedString>,
    action: Option<AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let t = Theme::global(cx).tokens;
    let text = text.into();
    div()
        .id(id)
        .debug_selector(|| "empty-state".into())
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .p_6()
        .text_center()
        .child(icon(mark, 24., Theme::color(t.muted)))
        .child(
            div()
                .id("empty-text")
                .role(Role::Label)
                .aria_value(text.clone())
                .text_size(px(METRICS.text_sm.size))
                .text_color(Theme::color(t.muted))
                .child(text),
        )
        .when_some(action, |d, a| d.child(a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Render, TestAppContext, Window};

    struct Shows(NoticeKind, bool);

    impl Render for Shows {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let button = div()
                .id("child")
                .debug_selector(|| "child".into())
                .w(px(40.))
                .h(px(10.));
            div()
                .w(px(300.))
                .child(notice(
                    "n",
                    self.0,
                    "D:\\a\\very\\long\\path\\that\\would\\overflow\\a\\narrow\\box\\if\\it\\did\\not\\wrap",
                    vec![button.into_any_element()],
                    cx,
                ))
                .when(self.1, |d| {
                    d.child(empty_state("e", Icon::FileSearch, "No tests yet", None, cx))
                })
        }
    }

    fn draw(kind: NoticeKind, cx: &mut TestAppContext) -> &mut gpui::VisualTestContext {
        cx.update(crate::init);
        let (_, vcx) = cx.add_window_view(|_, _| Shows(kind, true));
        vcx.run_until_parked();
        vcx
    }

    #[test]
    fn a_warning_is_a_status_tinted_cancelled() {
        let cx = gpui::TestAppContext::single();
        let (tint, tone, mark) = cx.update(|cx| {
            crate::init(cx);
            NoticeKind::Warning.look(cx)
        });
        let t = cx.update(|cx| Theme::global(cx).tokens);
        assert_eq!((tint, tone, mark), (t.cancelled_tint, t.cancelled, Icon::TriangleAlert));
        assert_eq!(NoticeKind::Warning.role(), Role::Status);
    }

    #[test]
    fn an_error_is_an_alert_tinted_failed() {
        let cx = gpui::TestAppContext::single();
        let (tint, tone, mark) = cx.update(|cx| {
            crate::init(cx);
            NoticeKind::Error.look(cx)
        });
        let t = cx.update(|cx| Theme::global(cx).tokens);
        assert_eq!((tint, tone, mark), (t.failed_tint, t.failed, Icon::CircleAlert));
        assert_eq!(NoticeKind::Error.role(), Role::Alert);
    }

    #[test]
    fn a_success_is_a_status_tinted_passed() {
        let cx = gpui::TestAppContext::single();
        let (tint, tone, mark) = cx.update(|cx| {
            crate::init(cx);
            NoticeKind::Success.look(cx)
        });
        let t = cx.update(|cx| Theme::global(cx).tokens);
        assert_eq!((tint, tone, mark), (t.passed_tint, t.passed, Icon::CircleCheck));
        assert_eq!(NoticeKind::Success.role(), Role::Status);
    }

    #[gpui::test]
    fn lets_a_long_path_wrap_instead_of_overflowing(cx: &mut TestAppContext) {
        let vcx = draw(NoticeKind::Warning, cx);
        let b = vcx.debug_bounds("notice").unwrap();
        assert!(b.size.width <= px(300.), "{b:?}");
        assert!(b.size.height > px(30.), "the text wraps onto more lines: {b:?}");
    }

    #[gpui::test]
    fn shows_its_children_after_the_text_in_the_same_box(cx: &mut TestAppContext) {
        let vcx = draw(NoticeKind::Warning, cx);
        let (n, c) = (vcx.debug_bounds("notice").unwrap(), vcx.debug_bounds("child").unwrap());
        assert!(n.contains(&c.origin), "{n:?} {c:?}");
        assert!(c.origin.x > n.origin.x + px(150.), "after the text: {c:?}");
    }

    #[test]
    fn draws_a_different_icon_per_kind() {
        let cx = gpui::TestAppContext::single();
        let marks = cx.update(|cx| {
            crate::init(cx);
            [NoticeKind::Success, NoticeKind::Warning, NoticeKind::Error].map(|k| k.look(cx).2)
        });
        assert!(marks[0] != marks[1] && marks[1] != marks[2] && marks[0] != marks[2]);
    }

    // EmptyState.test.tsx
    #[gpui::test]
    fn shows_a_muted_24_px_icon_and_the_sentence(cx: &mut TestAppContext) {
        let vcx = draw(NoticeKind::Warning, cx);
        let b = vcx.debug_bounds("empty-state").unwrap();
        // the 24 px icon, the gap and a line of text, within the padding
        assert!(b.size.height > px(24. + 8. + 12.), "{b:?}");
    }

    #[gpui::test]
    fn puts_the_action_after_the_sentence(cx: &mut TestAppContext) {
        struct WithAction;
        impl Render for WithAction {
            fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
                let action = div().id("a").debug_selector(|| "action".into()).w(px(40.)).h(px(10.));
                div().w(px(300.)).child(empty_state(
                    "e",
                    Icon::FileSearch,
                    "No tests yet",
                    Some(action.into_any_element()),
                    cx,
                ))
            }
        }
        cx.update(crate::init);
        let (_, vcx) = cx.add_window_view(|_, _| WithAction);
        vcx.run_until_parked();
        let (e, a) = (
            vcx.debug_bounds("empty-state").unwrap(),
            vcx.debug_bounds("action").unwrap(),
        );
        assert!(a.origin.y > e.origin.y + px(24. + 16.), "{e:?} {a:?}");
    }
}

/// A field message's kind: a hint is muted, a warning amber, an error red (premium UI R20.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Hint,
    Warning,
    Error,
}

/// An inline message under a field (`FieldMessage.tsx`): 12 px with its 14 px icon. A screen reader reads it as the
/// field's description (the field sets it), so it is a plain label here.
pub fn field_message(id: impl Into<ElementId>, kind: FieldKind, text: impl Into<SharedString>, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    let (tone, mark) = match kind {
        FieldKind::Hint => (t.muted, Icon::Info),
        FieldKind::Warning => (t.cancelled, Icon::TriangleAlert),
        FieldKind::Error => (t.failed, Icon::CircleAlert),
    };
    let text = text.into();
    let id = id.into();
    let selector: SharedString = id.to_string().into();
    div()
        .id(id)
        .debug_selector(move || selector.to_string())
        .role(Role::Label)
        .aria_value(text.clone())
        .flex()
        .items_start()
        .gap_1()
        .text_size(px(METRICS.text_xs.size))
        .text_color(Theme::color(tone))
        .child(div().mt(px(1.)).child(icon(mark, 14., Theme::color(tone))))
        .child(div().min_w_0().child(text))
        .into_any_element()
}
