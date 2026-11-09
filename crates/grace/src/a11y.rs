//! What GPUI's `div` cannot say to a screen reader (#269): a live region. GPUI 0.3.11 has no `aria_live`, so `live`
//! wraps an element that has an id, a role and a label, and adds `live` to the AccessKit node it writes. AccessKit
//! then announces each new label (AT-SPI `object:announcement`, UIA `Notification`), as `aria-live` does in
//! Electron Grace. From the prototype (docs/research/2026-11-07-gpui-prototype/src/a11y.rs).
use std::panic::Location;

use gpui::accesskit::{Live, Node, Role};
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Window,
};

pub struct Announcer<E> {
    inner: E,
    live: Live,
}

/// Wraps an element that has an id, a role and a label: each new label is announced.
pub fn live<E: Element>(inner: E, live: Live) -> Announcer<E> {
    Announcer { inner, live }
}

impl<E: Element> IntoElement for Announcer<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for Announcer<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.inner
            .prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner
            .paint(id, inspector_id, bounds, request_layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<Role> {
        self.inner.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut Node) {
        self.inner.write_a11y_info(node);
        node.set_live(self.live);
    }

    fn a11y_synthetic_children(&mut self, prepaint: &mut Self::PrepaintState, builder: &mut A11ySubtreeBuilder) {
        self.inner.a11y_synthetic_children(prepaint, builder)
    }
}

gpui::actions!(grace, [FocusNext, FocusPrevious]);

/// The key context of a window's root, where Tab and Shift+Tab move the focus.
const WINDOW: &str = "GraceWindow";

/// Tab and Shift+Tab, as a browser gives Electron Grace for free (guide R4): bound once at start.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        gpui::KeyBinding::new("tab", FocusNext, Some(WINDOW)),
        gpui::KeyBinding::new("shift-tab", FocusPrevious, Some(WINDOW)),
    ]);
}

/// Makes `root` the window's root for the keyboard: it holds `focus` when nothing inside does, so the keys reach it,
/// and Tab and Shift+Tab move through the tab stops in order. A dialog keeps its own Tab inside it. `also` adds a key
/// context of the root's own, such as the project view's: an element has one context, which may name several.
pub fn tab_order(
    root: gpui::Stateful<gpui::Div>,
    focus: &gpui::FocusHandle,
    also: Option<&'static str>,
) -> gpui::Stateful<gpui::Div> {
    use gpui::InteractiveElement as _;
    let mut context = gpui::KeyContext::default();
    context.add(WINDOW);
    if let Some(also) = also {
        context.add(also);
    }
    root.track_focus(focus)
        .key_context(context)
        .on_action(|_: &FocusNext, window, cx| window.focus_next(cx))
        .on_action(|_: &FocusPrevious, window, cx| window.focus_prev(cx))
}

/// Whether the system asks for reduced motion, as CSS's `prefers-reduced-motion` read it for Electron Grace: on
/// Windows, Settings > Accessibility > Visual effects > Animation effects (`SPI_GETCLIENTAREAANIMATION`). Linux, not
/// released (guide R5), reads GNOME's `enable-animations` through no API and is taken as moving. `GRACE_REDUCE_MOTION`
/// (`1` or `0`) overrides it, for tests.
pub fn system_reduces_motion() -> bool {
    if let Ok(v) = std::env::var("GRACE_REDUCE_MOTION") {
        return v == "1";
    }
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
        };
        use windows::core::BOOL;
        let mut animate = BOOL(1);
        // SAFETY: the call writes one BOOL to the pointer it is given, which lives across the call.
        let read = unsafe {
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                Some(&mut animate as *mut BOOL as *mut core::ffi::c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        read.is_ok() && !animate.as_bool()
    }
    #[cfg(not(windows))]
    false
}

/// GPUI's animations and the text boxes' caret (`theme::cursor_blink`) follow `reduce`.
pub fn set_reduce_motion(reduce: bool, cx: &mut App) {
    if cx.reduce_motion() == reduce {
        return;
    }
    cx.set_reduce_motion(reduce);
    gpui_component::Theme::update(cx, |k| k.cursor_blink = crate::theme::cursor_blink(reduce));
}

/// Applies the system's reduced motion to GPUI's animations, which then draw their end (a one-shot) or their start (a
/// loop) and ask for no frame. Read at start and each time the window becomes active, so a change in Settings counts
/// from the next return to Grace without anything polling (ADR 0030).
pub fn follow_reduced_motion<T: 'static>(window: &mut Window, cx: &mut gpui::Context<T>) -> gpui::Subscription {
    set_reduce_motion(system_reduces_motion(), cx);
    cx.observe_window_activation(window, |_, window, cx| {
        if window.is_window_active() {
            set_reduce_motion(system_reduces_motion(), cx);
        }
    })
}

/// The interactivity of a control that is not a stateful `div`, such as GPUI Kit's `Button`, seen as one: GPUI's
/// AccessKit setters (`aria_expanded`, `aria_description`, …) are on `StatefulInteractiveElement`, which those
/// controls do not implement.
pub struct Aria<'a>(&'a mut gpui::Interactivity);

impl gpui::InteractiveElement for Aria<'_> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.0
    }
}

impl gpui::StatefulInteractiveElement for Aria<'_> {}

/// Sets AccessKit states on `control` through [`Aria`]: `aria(button, |a| a.aria_expanded(open))`.
pub fn aria<E: gpui::InteractiveElement>(mut control: E, set: impl FnOnce(Aria<'_>) -> Aria<'_>) -> E {
    let _ = set(Aria(control.interactivity()));
    control
}
