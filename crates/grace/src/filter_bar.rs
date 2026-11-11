//! `FilterBar.tsx`: the tag chips of the open project above the tree (tag filter spec), kept to one line (compact
//! filter bar spec, #127). A chip cycles its tag off, included, excluded and off again; Clear and the counter show
//! while a filter is on, and Hide non-matching sits at the right. The window never edits the filter itself: it asks
//! the runner, which sends the filter back.
//!
//! The chips wrap in a strip one line tall, as in Electron Grace; where the browser told it which chips went past the
//! line (`IntersectionObserver`), GPUI hands the strip its children's places after each layout, and the bar keeps the
//! tags whose chip is below the first line (`on_children_prepainted`). It costs a comparison per frame drawn and
//! nothing while idle, when no frame is drawn. The fold chip `+N` opens a panel of every tag over the tree.
use crate::elements::Off as _;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::accesskit::{Live, Role};
use gpui::{
    AnyElement, ClickEvent, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement as _, Pixels, Point, Render, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, deferred, div, prelude::FluentBuilder as _, px,
};
use gpui_component::RoleOverride;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_component::checkbox::Checkbox;
use grace_core::model::node::TestNode;
use grace_core::model::tags::{FilterState, chip_tags, cycle_tag, filter_on, matching_leaves};
use grace_core::rows::leaves;
use grace_core::state::AppState;

use crate::a11y::{FocusNext, FocusPrevious};
use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, button, icon};
use crate::theme::{METRICS, Theme};
use crate::tooltip::tooltip;

/// A chip's height, and so the strip's one line (R1.1).
pub const CHIP_HEIGHT: f32 = 24.;
/// Between chips, across and down: the second line starts below the strip's clip.
const GAP: f32 = 6.;
/// Room around the strip's line for a chip's focus ring, which the clip would cut (R1.4).
const RING: f32 = 4.;
/// The bar: the line, its 6 px above and below, and the 1 px line under it.
pub const BAR_HEIGHT: f32 = CHIP_HEIGHT + 12. + 1.;

/// A chip's button over the whole of its frame, the frame's border included, so that it takes the click on all of
/// the chip's 24 px (PU-24), its text 7 px in from the chip's edge: the 1 px border and `px-1.5`.
trait Target {
    fn target(self) -> Self;
}

impl Target for Button {
    fn target(self) -> Self {
        self.h(px(CHIP_HEIGHT)).m(px(-1.)).px(px(7.)).rounded(px(4.))
    }
}

pub const CLEAR: &str = "Clear";
pub const HIDE_NON_MATCHING: &str = "Hide non-matching";
pub const SHOW_ALL_TAGS: &str = "Show all tags";
pub const ALL_TAGS: &str = "All tags";

/// A tag's place in the filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipState {
    Off,
    Include,
    Exclude,
}

impl ChipState {
    pub fn of(f: &FilterState, tag: &str) -> Self {
        if f.include.iter().any(|t| t == tag) {
            Self::Include
        } else if f.exclude.iter().any(|t| t == tag) {
            Self::Exclude
        } else {
            Self::Off
        }
    }

    /// The next click, in the chip's tooltip (`HINT`).
    pub fn hint(self) -> &'static str {
        match self {
            Self::Off => "Not filtered. Click to include.",
            Self::Include => "Included. Click to exclude.",
            Self::Exclude => "Excluded. Click to clear.",
        }
    }

    /// The mark beside the tag, so a state shows by shape and not only by colour (R7.3): a check, a minus, none.
    pub fn mark(self) -> Option<Icon> {
        match self {
            Self::Off => None,
            Self::Include => Some(Icon::Check),
            Self::Exclude => Some(Icon::Minus),
        }
    }

    /// An excluded tag is struck through, beside its minus.
    pub fn struck(self) -> bool {
        self == Self::Exclude
    }

    /// Colour and strike-through are invisible to screen readers: the state goes in the name (`SPOKEN`).
    pub fn spoken(self) -> &'static str {
        match self {
            Self::Off => "not filtered",
            Self::Include => "included",
            Self::Exclude => "excluded",
        }
    }
}

/// A chip's accessible name: `@smoke, included`.
pub fn chip_name(tag: &str, state: ChipState) -> String {
    format!("@{tag}, {}", state.spoken())
}

/// The fold chip's text: `+N`, or `+N · K on` when K of the folded tags are on (R2.1).
pub fn fold_text(folded: usize, on: usize) -> String {
    if on > 0 {
        format!("+{folded} · {on} on")
    } else {
        format!("+{folded}")
    }
}

/// The fold chip's accessible name: `N more tags`, `1 more tag`, then `, K on` (R2.2).
pub fn fold_name(folded: usize, on: usize) -> String {
    let tags = if folded == 1 { "tag" } else { "tags" };
    if on > 0 {
        format!("{folded} more {tags}, {on} on")
    } else {
        format!("{folded} more {tags}")
    }
}

/// `2 of 9 tests match`.
pub fn counter(tree: &TestNode, f: &FilterState) -> String {
    format!(
        "{} of {} tests match",
        matching_leaves(tree, f).len(),
        leaves(tree).len()
    )
}

/// The tags whose chip lies below the strip's first line, from the chips' places in tag order.
pub fn folded_tags(tags: &[String], places: &[gpui::Bounds<Pixels>]) -> Vec<String> {
    let Some(first) = places.first() else {
        return Vec::new();
    };
    let line = first.top() + px(1.);
    tags.iter()
        .zip(places)
        .filter(|(_, b)| b.top() > line)
        .map(|(t, _)| t.clone())
        .collect()
}

/// The filter bar of the open project.
pub struct FilterBar {
    grace: Entity<Grace>,
    /// The tags whose chip went past the line, in tag order.
    folded: Vec<String>,
    /// Whether the panel of every tag is open.
    pub(crate) open: bool,
    /// Where the press was that closed the panel from outside: when it was on the fold chip, its click must not open
    /// the panel again (a popover's light dismiss).
    closed_at: Option<Point<Pixels>>,
    /// The focus handles of the chips on the line, the fold chip's and the panel's chips', from the last frame.
    chips: Rc<RefCell<HashMap<String, FocusHandle>>>,
    more: Rc<RefCell<Option<FocusHandle>>>,
    panel: Rc<RefCell<Vec<FocusHandle>>>,
    _observe: Subscription,
}

impl FilterBar {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&grace, |_, _, cx| cx.notify());
        Self {
            grace,
            folded: Vec::new(),
            open: false,
            closed_at: None,
            chips: Rc::default(),
            more: Rc::default(),
            panel: Rc::default(),
            _observe: observe,
        }
    }

    pub fn folded(&self) -> &[String] {
        &self.folded
    }

    /// The strip's report after a layout: a chip that went past the line while it had the focus hands it to the fold
    /// chip, which now holds that tag; nothing left past the line closes the panel (R3.5).
    fn set_folded(&mut self, folded: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
        if folded == self.folded {
            return;
        }
        let lost_focus = {
            let chips = self.chips.borrow();
            folded
                .iter()
                .filter(|t| !self.folded.contains(t))
                .any(|t| chips.get(t).is_some_and(|f| f.is_focused(window)))
        };
        self.folded = folded;
        if self.folded.is_empty() {
            self.open = false;
        }
        cx.notify();
        if lost_focus {
            // the fold chip shows from this frame on: focus it once it is drawn
            let more = self.more.clone();
            cx.on_next_frame(window, move |_, window, cx| {
                if let Some(f) = more.borrow().as_ref() {
                    window.focus(f, cx);
                }
            });
        }
    }

    fn set_filter(&self, filter: FilterState, cx: &mut Context<Self>) {
        self.grace.update(cx, |g, cx| g.set_filter(filter, cx));
    }

    /// A click on the fold chip, which keeps the focus, so Escape reaches the bar.
    fn toggle_panel(&mut self, e: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let closed_here = match e {
            ClickEvent::Mouse(m) => self.closed_at == Some(m.down.position),
            _ => false,
        };
        self.closed_at = None;
        if !closed_here {
            self.open = !self.open;
        }
        if let Some(f) = self.more.borrow().as_ref() {
            window.focus(f, cx);
        }
        cx.notify();
    }

    fn close_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if let Some(f) = self.more.borrow().as_ref() {
            window.focus(f, cx);
        }
        cx.notify();
    }

    /// One chip: on the line, `folded` past it (drawn the same, so it keeps its place, but hidden, out of the Tab
    /// order and out of the accessibility tree, R1.3), or in the panel.
    #[allow(clippy::too_many_arguments)]
    fn chip(
        &self,
        id: SharedString,
        tag: &str,
        filter: &FilterState,
        locked: bool,
        folded: bool,
        focus: Rc<dyn Fn(&FocusHandle)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let state = ChipState::of(filter, tag);
        let (border, bg, fg, hover) = match state {
            ChipState::Off => (t.line, t.surface, t.muted, Some(t.hover)),
            ChipState::Include => (t.accent, t.accent, t.on_accent, None),
            ChipState::Exclude => (t.control, t.surface, t.ink, None),
        };
        let mark = state.mark();
        let next = cycle_tag(filter.clone(), tag);
        let me = cx.entity();
        let name = chip_name(tag, state);
        let label = SharedString::from(format!("@{tag}"));
        let tip = (!folded).then(|| SharedString::from(state.hint()));
        let variant = ButtonCustomVariant::new(cx)
            .color(Theme::clear())
            .foreground(Theme::color(fg))
            .hover(Theme::clear())
            .active(Theme::clear());
        let button_id = id.clone();
        let tip_id = SharedString::from(format!("tip-{id}"));
        let target = tooltip(tip_id, tip).focusable(move |f| {
            focus(f);
            Button::new(button_id)
                .custom(variant)
                .target()
                .gap_1()
                .text_size(px(METRICS.text_2xs.size))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(Theme::color(fg))
                .when(state.struck(), |b| b.line_through())
                .track_focus(f)
                .off(locked)
                .map(|b| {
                    if folded {
                        b.role(RoleOverride::Presentational).tab_stop(false)
                    } else {
                        crate::a11y::aria(b.accessibility_label(name), |a| a.aria_description(state.hint()))
                    }
                })
                .children(mark.map(|m| icon(m, 14., Theme::color(fg))))
                .child(label)
                .on_click(move |_, _, cx| me.update(cx, |bar, cx| bar.set_filter(next.clone(), cx)))
        });
        // the border, the fill and the hover on a frame under the button, which GPUI Kit's custom buttons colour
        // themselves
        div()
            .debug_selector(move || id.to_string())
            .flex()
            .flex_none()
            .h(px(CHIP_HEIGHT))
            .rounded(px(4.))
            .border_1()
            .border_color(Theme::color(border))
            .bg(Theme::color(bg))
            .when_some(hover.filter(|_| !locked), |d, h| {
                d.hover(move |s| s.bg(Theme::color(h)))
            })
            .when(locked, |d| d.opacity(0.4))
            .when(folded, |d| d.invisible())
            .child(target)
            .into_any_element()
    }
}

impl Render for FilterBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state: Rc<AppState> = self.grace.read(cx).state.clone();
        let tree = state.tree.as_ref().filter(|_| state.project.is_some());
        let tags = if tree.is_some() {
            chip_tags(tree, &state.filter)
        } else {
            Vec::new()
        };
        if tags.is_empty() {
            self.chips.borrow_mut().clear();
            *self.more.borrow_mut() = None;
            self.panel.borrow_mut().clear();
            return div().id("filter-bar-none").into_any_element();
        }
        let t = Theme::global(cx).tokens;
        let filter = &state.filter;
        let locked = state.run.active || state.busy;
        let on = filter_on(filter);
        self.folded.retain(|t| tags.contains(t));
        let folded_on = self
            .folded
            .iter()
            .filter(|t| ChipState::of(filter, t) != ChipState::Off)
            .count();

        // the chips on the line
        self.chips.borrow_mut().clear();
        let chips: Vec<AnyElement> = tags
            .iter()
            .map(|tag| {
                let store = self.chips.clone();
                let key = tag.clone();
                let folded = self.folded.contains(tag);
                self.chip(
                    SharedString::from(format!("tag-chip-{tag}")),
                    tag,
                    filter,
                    locked,
                    folded,
                    Rc::new(move |f: &FocusHandle| {
                        store.borrow_mut().insert(key.clone(), f.clone());
                    }),
                    cx,
                )
            })
            .collect();
        let me = cx.weak_entity();
        let seen = self.folded.clone();
        let order = tags.clone();
        let strip = div()
            .on_children_prepainted(move |places, window, cx| {
                let folded = folded_tags(&order, &places);
                if folded != seen {
                    let me = me.clone();
                    window.defer(cx, move |window, cx| {
                        me.update(cx, |bar, cx| bar.set_folded(folded, window, cx)).ok();
                    });
                }
            })
            .debug_selector(|| "tag-strip".into())
            .flex()
            .flex_wrap()
            .gap(px(GAP))
            .min_w_0()
            // one line, with room for the focus ring that the clip would cut; the second line starts past it
            .h(px(CHIP_HEIGHT + 2. * RING))
            .p(px(RING))
            .m(px(-RING))
            .overflow_hidden()
            .children(chips);

        // the fold chip and the panel of every tag
        let more = (!self.folded.is_empty()).then(|| {
            let me = cx.entity();
            let tab = cx.entity();
            let store = self.more.clone();
            let panel = self.panel.clone();
            let open = self.open;
            let text = fold_text(self.folded.len(), folded_on);
            let name = fold_name(self.folded.len(), folded_on);
            let variant = ButtonCustomVariant::new(cx)
                .color(Theme::clear())
                .foreground(Theme::color(t.muted))
                .hover(Theme::clear())
                .active(Theme::clear());
            tooltip("tip-tag-more", Some(SHOW_ALL_TAGS.into())).focusable(move |f| {
                *store.borrow_mut() = Some(f.clone());
                div()
                    .flex()
                    .flex_none()
                    .h(px(CHIP_HEIGHT))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(Theme::color(t.line))
                    .bg(Theme::color(t.surface))
                    .hover(|s| s.bg(Theme::color(t.hover)))
                    .child(crate::a11y::aria(
                        Button::new("tag-more")
                            .custom(variant)
                            .target()
                            .text_size(px(METRICS.text_2xs.size))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(Theme::color(t.muted))
                            .accessibility_label(name)
                            .track_focus(f)
                            .child(text)
                            .on_click(move |e, window, cx| me.update(cx, |bar, cx| bar.toggle_panel(e, window, cx)))
                            // the panel's chips follow the fold chip in the Tab order (R3.4)
                            .on_action(move |_: &FocusNext, window, cx| {
                                let first = panel.borrow().first().cloned();
                                match first.filter(|_| tab.read(cx).open) {
                                    Some(first) => window.focus(&first, cx),
                                    None => cx.propagate(),
                                }
                            }),
                        |a| a.aria_expanded(open),
                    ))
            })
        });
        let panel = (self.open && !self.folded.is_empty()).then(|| self.panel_view(&tags, filter, locked, window, cx));

        let me = cx.entity();
        let clear = on.then(|| {
            let cleared = FilterState {
                include: vec![],
                exclude: vec![],
                hide_non_matching: filter.hide_non_matching,
            };
            div().debug_selector(|| "tag-clear".into()).flex_none().child(
                button("tag-clear", CLEAR, ButtonKind::Ghost, ButtonSize::Small)
                    .off(locked)
                    .on_click(move |_, _, cx| me.update(cx, |bar, cx| bar.set_filter(cleared.clone(), cx))),
            )
        });
        // always there: a live region added together with its text is often not announced
        let count = on.then(|| tree.map(|tree| counter(tree, filter))).flatten();
        let counter = crate::a11y::live(
            div()
                .id("tag-counter")
                .debug_selector(|| "tag-counter".into())
                .role(Role::Status)
                .when_some(count.clone(), |d, c| d.aria_label(c))
                .flex_none()
                .whitespace_nowrap()
                .text_size(px(METRICS.text_xs.size))
                .text_color(Theme::color(t.muted))
                .children(count),
            Live::Polite,
        );
        let me = cx.entity();
        let hide = filter.clone();
        let hide_non_matching = div()
            .debug_selector(|| "hide-non-matching".into())
            .ml_auto()
            .flex()
            .flex_none()
            .items_center()
            .h(px(CHIP_HEIGHT))
            .child(
                Checkbox::new("hide-non-matching")
                    .cursor_pointer()
                    .label(HIDE_NON_MATCHING)
                    .checked(filter.hide_non_matching)
                    .off(locked)
                    .text_size(px(METRICS.text_xs.size))
                    .whitespace_nowrap()
                    .on_click(move |checked, _, cx| {
                        let next = FilterState {
                            hide_non_matching: *checked,
                            ..hide.clone()
                        };
                        me.update(cx, |bar, cx| bar.set_filter(next, cx));
                    }),
            );

        let me = cx.entity();
        div()
            .id("filter-bar")
            .debug_selector(|| "filter-bar".into())
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(GAP))
            .h(px(BAR_HEIGHT))
            .px_3()
            .border_b_1()
            .border_color(Theme::color(t.line))
            .on_key_down(move |e: &KeyDownEvent, window, cx| {
                if e.keystroke.key == "escape" && me.read(cx).open {
                    me.update(cx, |bar, cx| bar.close_panel(window, cx));
                    cx.stop_propagation();
                }
            })
            .child(strip)
            .children(more.map(|m| div().debug_selector(|| "tag-more".into()).flex_none().child(m)))
            .children(panel)
            .children(clear)
            .child(counter)
            .child(hide_non_matching)
            .into_any_element()
    }
}

impl FilterBar {
    /// Under the bar, its width, over the tree (R3.1): every tag's chip, wrapped, scrolling past half the window.
    fn panel_view(
        &self,
        tags: &[String],
        filter: &FilterState,
        locked: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let m = METRICS;
        self.panel.borrow_mut().clear();
        let last = tags.len().saturating_sub(1);
        let chips: Vec<AnyElement> = tags
            .iter()
            .enumerate()
            .map(|(i, tag)| {
                let store = self.panel.clone();
                let chip = self.chip(
                    SharedString::from(format!("tag-panel-chip-{tag}")),
                    tag,
                    filter,
                    locked,
                    false,
                    Rc::new(move |f: &FocusHandle| store.borrow_mut().push(f.clone())),
                    cx,
                );
                let more = self.more.clone();
                // Tab past the last goes on from the fold chip; Shift+Tab before the first goes back to it
                div()
                    .when(i == last, |d| {
                        let more = more.clone();
                        d.on_action(move |_: &FocusNext, window, cx| {
                            if let Some(f) = more.borrow().as_ref() {
                                window.focus(f, cx);
                                window.focus_next(cx);
                            }
                        })
                    })
                    .when(i == 0, |d| {
                        d.on_action(move |_: &FocusPrevious, window, cx| {
                            if let Some(f) = more.borrow().as_ref() {
                                window.focus(f, cx);
                            }
                        })
                    })
                    .child(chip)
                    .into_any_element()
            })
            .collect();
        let me = cx.entity();
        let shadow = crate::theme::float_shadow(cx);
        deferred(
            div()
                .id("tag-panel")
                .debug_selector(|| "tag-panel".into())
                .role(Role::Group)
                .aria_label(ALL_TAGS)
                .absolute()
                .top(px(BAR_HEIGHT))
                .left_0()
                .right_0()
                .max_h(window.viewport_size().height * 0.5)
                .overflow_y_scroll()
                .occlude()
                .p_2()
                .rounded_b(px(m.radius_lg))
                .border_1()
                .border_color(Theme::color(t.line))
                .bg(Theme::color(t.raised))
                .shadow(shadow)
                .on_mouse_down_out(move |e, _, cx| {
                    me.update(cx, |bar, cx| {
                        bar.open = false;
                        bar.closed_at = Some(e.position);
                        cx.notify();
                    })
                })
                .child(div().flex().flex_wrap().gap(px(GAP)).children(chips)),
        )
        .with_priority(1)
        .into_any_element()
    }
}

#[cfg(test)]
mod tests;
