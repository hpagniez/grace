//! What GPUI's `div` cannot say to a screen reader (#269): a live region. GPUI 0.3.11 has no `aria_live`, so this
//! element wraps another one and adds `live` to the AccessKit node it writes. AccessKit then sends an announcement
//! each time the node's name changes (AT-SPI `object:announcement`, UIA `Notification`).
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
        self.inner.prepaint(id, inspector_id, bounds, request_layout, window, cx)
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
        self.inner.paint(id, inspector_id, bounds, request_layout, prepaint, window, cx)
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
