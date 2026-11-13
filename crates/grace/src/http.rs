//! `HttpCalls.tsx`: the HTTP tab, status-group chips over one row per call of the tests under the tree selection
//! (docs/specs/2026-10-25-http-inspector.md R3), on GPUI's `uniform_list`, which draws only the rows in view.
//!
//! The list is the tree's kind of list (P3 plan, Porting rules 1): one tab stop, whose keyboard focus stays on the
//! list while the row it is on reports itself as the focused node to assistive technology (`aria_active_descendant`);
//! the scroll rules are `grace_core::rows`'s. Its rows are built only while the tab shows, and again only when the
//! calls, the tree, the selection or the chips change (#124): a closed tab costs a run the tab's count alone
//! (`grace_core::http_list::shown_count`).
//!
//! A call's details and Copy as cURL come with the next slice: selecting a call sets `selected_call`.
use std::rc::Rc;

use gpui::accesskit::{Role, Toggled};
use gpui::{
    AnyElement, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement,
    Render, ScrollStrategy, SharedString, StatefulInteractiveElement as _, Styled, Subscription,
    UniformListScrollHandle, Window, div, point, prelude::FluentBuilder as _, px, uniform_list,
};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_component::scroll::ScrollableElement as _;
use grace_core::http_list::{
    GroupCounts, call_name, empty_text, group_counts, group_label, narrowed_calls, shown_calls,
};
use grace_core::model::http::{HTTP_GROUPS, HttpCall, HttpGroup, http_group};
use grace_core::model::node::TestNode;
use grace_core::rows::{View, anchored_scroll_top, node_name, page_rows, survivor};
use grace_core::state::{AppState, CallRef, Panel};

use crate::app::Grace;
use crate::assets::Icon;
use crate::details::format_millis;
use crate::elements::icon;
use crate::theme::{METRICS, MONO, Theme, Tokens};
use crate::tooltip::{Side, cut, tooltip};

/// px, a row (virtual lists, Constraints).
pub const ROW_HEIGHT: f32 = 28.;
/// The list's accessible name (virtual lists R5.3).
pub const LIST: &str = "HTTP calls";

/// The colour a badge takes: a status group's colour everywhere else in Grace (`GROUP_TINT`, `TINT`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tint {
    Skipped,
    Passed,
    Running,
    Cancelled,
    Failed,
}

impl Tint {
    /// The fill and the ink: the status colour on its own solid tint, never white on a fill.
    pub fn colours(self, t: &Tokens) -> (u32, u32) {
        match self {
            Tint::Skipped => (t.skipped_tint, t.skipped),
            Tint::Passed => (t.passed_tint, t.passed),
            Tint::Running => (t.running_tint, t.running),
            Tint::Cancelled => (t.cancelled_tint, t.cancelled),
            Tint::Failed => (t.failed_tint, t.failed),
        }
    }
}

pub fn group_tint(g: HttpGroup) -> Tint {
    match g {
        HttpGroup::Informational => Tint::Skipped,
        HttpGroup::Success => Tint::Passed,
        HttpGroup::Redirect => Tint::Running,
        HttpGroup::ClientError => Tint::Cancelled,
        HttpGroup::ServerError | HttpGroup::NoResponse => Tint::Failed,
    }
}

/// The tint of a status; a code outside 100–599 is neutral.
pub fn status_tint(status: Option<i32>) -> Tint {
    http_group(status).map_or(Tint::Skipped, group_tint)
}

/// What a row reads, left to right.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowText {
    /// The code, or "—" for no response.
    pub status: String,
    pub method: String,
    /// The URL; with none, the first line of the error.
    pub place: String,
    /// What the place's tooltip says when it is cut: the URL, or the whole error.
    pub place_whole: String,
    pub test: String,
    pub attempt: Option<String>,
    pub via: Option<String>,
    /// The response time, or "—".
    pub time: String,
}

pub fn row_text(call: &HttpCall, test: &TestNode) -> RowText {
    let first_line = call.error.as_deref().and_then(|e| e.lines().next());
    RowText {
        status: call.status.map_or_else(|| "—".to_owned(), |s| s.to_string()),
        method: call.method.clone(),
        place: call.url.as_deref().or(first_line).unwrap_or_default().to_owned(),
        place_whole: call
            .url
            .as_deref()
            .or(call.error.as_deref())
            .unwrap_or_default()
            .to_owned(),
        test: node_name(test),
        attempt: call.attempt.map(|a| format!("attempt {} of {}", a.n, a.of)),
        via: call.via.as_ref().map(|v| format!("via {v}")),
        time: call.millis.map_or_else(|| "—".to_owned(), format_millis),
    }
}

/// A status group's chip: All (`group` `None`) or a group, with its count, and whether it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip {
    pub group: Option<HttpGroup>,
    pub text: String,
    pub pressed: bool,
}

/// The seven chips, zeros included, so "5xx Server error 0" says there was none (R3). All is on when no group is.
pub fn chips(counts: &GroupCounts, groups: &[HttpGroup]) -> Vec<Chip> {
    let all = Chip {
        group: None,
        text: format!("All {}", counts.get(None)),
        pressed: groups.is_empty(),
    };
    std::iter::once(all)
        .chain(HTTP_GROUPS.iter().map(|g| Chip {
            group: Some(g.group),
            text: format!("{} {}", g.label, counts.get(Some(g.group))),
            pressed: groups.contains(&g.group),
        }))
        .collect()
}

/// The chip's name in debug selectors: `all`, `1xx` … `none`.
pub fn chip_id(group: Option<HttpGroup>) -> &'static str {
    group.map_or("all", HttpGroup::as_str)
}

/// One call of the list, built once per change of what the list shows.
#[derive(Clone, Debug)]
pub struct Row {
    pub at: CallRef,
    pub status: Option<i32>,
    pub text: RowText,
    /// Its accessible name: status, method, URL and test (R5.4).
    pub name: String,
}

/// What the rows were built from: the tree, the selection, the calls and the chips.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Built {
    trees: u64,
    selected: Option<String>,
    calls: u64,
    groups: Vec<HttpGroup>,
}

/// A row's place as a key, for the scroll rules: a place, not a call, so a row outlives its test's calls being
/// replaced.
fn place_key(at: &CallRef) -> String {
    format!("{}\n{}", at.key, at.index)
}

/// An id unique to a row's part.
fn part(what: &str, at: &CallRef) -> SharedString {
    SharedString::from(format!("{what}:{}#{}", at.key, at.index))
}

pub struct HttpCalls {
    grace: Entity<Grace>,
    pub focus: FocusHandle,
    scroll: UniformListScrollHandle,
    rows: Rc<Vec<Row>>,
    keys: Rc<Vec<String>>,
    counts: GroupCounts,
    built: Option<Built>,
    /// How many times the rows were built: a run's calls build none while the tab is closed (#124).
    pub builds: u64,
    /// The call last focused or moved to; the focus row is it, else the selected call, else the first (VL-13).
    pub(crate) active: Option<CallRef>,
    /// The focus row came there from the keyboard, so it shows the ring (E-virtual-08).
    keyboard: bool,
    /// The selected call scrolls into view once the rows are built, after a chip the person pressed (R4.7).
    reveal_selected: bool,
    /// The tab was closed, or another project opened, since the list was drawn: it starts afresh, at the top with no
    /// focus row of its own, as Electron Grace's list mounted again.
    stale: bool,
    root: Option<String>,
    _observe: Subscription,
}

impl HttpCalls {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&grace, |list: &mut HttpCalls, grace, cx| {
            let s = &grace.read(cx).state;
            let root = s.project.as_ref().map(|p| &p.root);
            if s.panel != Panel::Http || root != list.root.as_ref() {
                list.stale = true;
            }
        });
        Self {
            stale: false,
            root: grace.read(cx).state.project.as_ref().map(|p| p.root.clone()),
            _observe: observe,
            grace,
            // the list is one tab stop (VL-13)
            focus: cx.focus_handle().tab_stop(true),
            scroll: UniformListScrollHandle::new(),
            rows: Rc::default(),
            keys: Rc::default(),
            counts: GroupCounts::default(),
            built: None,
            builds: 0,
            active: None,
            keyboard: false,
            reveal_selected: false,
        }
    }

    pub fn rows(&self) -> Rc<Vec<Row>> {
        Rc::clone(&self.rows)
    }

    pub fn counts(&self) -> GroupCounts {
        self.counts
    }

    /// The rows again, when what they show changed. Rows added or removed above the view keep the rows in view
    /// still (VL-19); a focus row that left hands over to the nearest row above.
    fn build(&mut self, state: &AppState) {
        if std::mem::take(&mut self.stale) {
            self.root = state.project.as_ref().map(|p| p.root.clone());
            self.active = None;
            self.keyboard = false;
            self.keys = Rc::default(); // nothing to keep in view
            self.scroll.0.borrow().base_handle.set_offset(point(px(0.), px(0.)));
        }
        let from = Built {
            trees: state.trees,
            selected: state.selected.clone(),
            calls: state.calls_changes,
            groups: state.http_groups.clone(),
        };
        if self.built.as_ref() != Some(&from) {
            self.built = Some(from);
            self.builds += 1;
            let narrowed = narrowed_calls(state);
            self.counts = group_counts(&narrowed);
            let rows: Vec<Row> = shown_calls(narrowed, &state.http_groups)
                .into_iter()
                .map(|l| Row {
                    status: l.call.status,
                    text: row_text(l.call, l.test),
                    name: call_name(l.call, l.test),
                    at: l.at,
                })
                .collect();
            let before = Rc::clone(&self.keys);
            self.keys = Rc::new(rows.iter().map(|r| place_key(&r.at)).collect());
            self.rows = Rc::new(rows);
            let view = self.view();
            let top = anchored_scroll_top(view, &before, &self.keys);
            if top != view.scroll_top {
                self.scroll.0.borrow().base_handle.set_offset(point(px(0.), px(-top)));
            }
            if let Some(a) = self.active.clone()
                && self.index_of(&a).is_none()
            {
                let selected_kept = state.selected_call.as_ref().is_some_and(|s| self.index_of(s).is_some());
                self.active = if selected_kept {
                    None
                } else {
                    survivor(&place_key(&a), &before, &self.keys).and_then(|k| {
                        let i = self.keys.iter().position(|x| *x == k)?;
                        Some(self.rows[i].at.clone())
                    })
                };
            }
        }
        if self.reveal_selected {
            self.reveal_selected = false;
            if let Some(i) = state.selected_call.as_ref().and_then(|s| self.index_of(s)) {
                self.scroll.scroll_to_item(i, ScrollStrategy::Nearest);
            }
        }
    }

    fn index_of(&self, at: &CallRef) -> Option<usize> {
        self.rows.iter().position(|r| &r.at == at)
    }

    /// Where the list is scrolled and how tall it is.
    fn view(&self) -> View {
        let handle = &self.scroll.0.borrow().base_handle;
        View {
            scroll_top: -f32::from(handle.offset().y),
            height: f32::from(handle.bounds().size.height),
            row_height: ROW_HEIGHT,
            pad: 0.,
        }
    }

    /// The focus row's index: the call last focused if it is there, else the selected one, else the first.
    fn focus_index(&self, state: &AppState) -> Option<usize> {
        self.active
            .as_ref()
            .and_then(|a| self.index_of(a))
            .or_else(|| state.selected_call.as_ref().and_then(|s| self.index_of(s)))
            .or_else(|| (!self.rows.is_empty()).then_some(0))
    }

    fn select(&self, at: CallRef, cx: &mut Context<Self>) {
        self.grace.update(cx, |g, cx| g.select_call(at, cx));
    }

    /// Moves to row `index` as an arrow key does: in view, the focus row from the keyboard, selected (VL-13).
    fn move_to(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(at) = self.rows.get(index).map(|r| r.at.clone()) else {
            return;
        };
        self.scroll.scroll_to_item(index, ScrollStrategy::Nearest);
        self.active = Some(at.clone());
        self.keyboard = true;
        self.select(at, cx);
        cx.notify();
    }

    fn on_key(&mut self, e: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        // a borrow, not a clone of the `Rc`: one held while selecting would make `state_mut` copy the whole state
        let Some(f) = self.focus_index(&self.grace.read(cx).state) else {
            return;
        };
        let k = &e.keystroke;
        let m = k.modifiers;
        if m.control || m.platform || m.alt {
            return;
        }
        let last = self.rows.len() - 1;
        let page = page_rows(self.view());
        let to = match k.key.as_str() {
            "down" => Some((f + 1).min(last)),
            "up" => Some(f.saturating_sub(1)),
            "home" => Some(0),
            "end" => Some(last),
            "pagedown" => Some((f + page).min(last)),
            "pageup" => Some(f.saturating_sub(page)),
            "enter" | "space" => {
                cx.stop_propagation();
                // a key that does not move still brings a scrolled-away row back
                self.scroll.scroll_to_item(f, ScrollStrategy::Nearest);
                let at = self.rows[f].at.clone();
                self.select(at, cx);
                return;
            }
            _ => return,
        };
        // at an end too, so nothing else takes the key
        cx.stop_propagation();
        if let Some(to) = to.filter(|to| *to != f) {
            self.move_to(to, cx);
        }
    }

    /// A click on a row: the row is the focus row, not from the keyboard, and the list takes the focus.
    fn clicked(&mut self, at: CallRef, window: &mut Window, cx: &mut Context<Self>) {
        self.active = Some(at.clone());
        self.keyboard = false;
        window.focus(&self.focus, cx);
        self.select(at, cx);
        cx.notify();
    }

    /// A click on a chip; the selected call then comes into view if the chips still show it (R4.7).
    fn toggle(&mut self, group: Option<HttpGroup>, cx: &mut Context<Self>) {
        self.reveal_selected = true;
        self.grace.update(cx, |g, cx| g.toggle_group(group, cx));
        cx.notify();
    }

    fn chip(&self, chip: Chip, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let (bg, fg, hover) = if chip.pressed {
            (t.selected, t.ink, None)
        } else {
            (t.surface, t.muted, Some(t.hover))
        };
        let variant = ButtonCustomVariant::new(cx)
            .color(Theme::clear())
            .foreground(Theme::color(fg))
            .hover(Theme::clear())
            .active(Theme::clear());
        let id = chip_id(chip.group);
        let me = cx.entity();
        let group = chip.group;
        let button = Button::new(SharedString::from(format!("http-group-{id}")))
            .custom(variant)
            .cursor_pointer()
            .h(px(24.))
            .m(px(-1.))
            .px(px(5.))
            .rounded(px(4.))
            .accessibility_label(chip.text.clone())
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(Theme::color(fg))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(METRICS.text_2xs.size))
                    .whitespace_nowrap()
                    // the check shows a pressed chip by shape, not only by fill
                    .when(chip.pressed, |d| d.child(icon(Icon::Check, 14., Theme::color(fg))))
                    .child(chip.text),
            )
            .on_click(move |_, _, cx| me.update(cx, |list, cx| list.toggle(group, cx)));
        let button = crate::a11y::aria(button, |a| {
            a.aria_toggled(if chip.pressed { Toggled::True } else { Toggled::False })
        });
        // the border, the fill and the hover on a frame under the button, which GPUI Kit's custom buttons colour
        // themselves (as the tag chips)
        div()
            .debug_selector(move || format!("http-group-{id}"))
            .flex()
            .flex_none()
            .h(px(24.))
            .rounded(px(4.))
            .border_1()
            .border_color(Theme::color(t.line))
            .bg(Theme::color(bg))
            .when_some(hover, |d, h| d.hover(move |s| s.bg(Theme::color(h))))
            .child(button)
            .into_any_element()
    }
}

impl Render for HttpCalls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.grace.read(cx).state.clone();
        self.build(&state);
        let t = Theme::global(cx).tokens;
        let chips: Vec<_> = chips(&self.counts, &state.http_groups)
            .into_iter()
            .map(|c| self.chip(c, cx))
            .collect();
        let bar = div()
            .flex()
            .flex_none()
            .flex_wrap()
            .gap_2()
            .border_b_1()
            .border_color(Theme::color(t.line))
            .px_3()
            .py_2()
            .children(chips);
        let body = if self.rows.is_empty() {
            div()
                .id("http-calls")
                .debug_selector(|| "http-calls".into())
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(crate::ui::empty_state(
                    "http-empty",
                    Icon::Globe,
                    empty_text(&state),
                    None,
                    cx,
                ))
                .into_any_element()
        } else {
            let focused = self.focus.is_focused(window);
            let focus_at = self.focus_index(&state).map(|i| self.rows[i].at.clone());
            let me = cx.entity();
            div()
                .id("http-calls")
                .debug_selector(|| "http-calls".into())
                .role(Role::ListBox)
                .aria_label(LIST)
                .track_focus(&self.focus)
                .on_key_down(cx.listener(Self::on_key))
                .relative()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(
                    uniform_list("http-rows", self.rows.len(), move |range, window, cx| {
                        me.update(cx, |list, cx| {
                            let state = list.grace.read(cx).state.clone();
                            range
                                .map(|i| list.row(i, &state, focused, focus_at.as_ref(), window, cx))
                                .collect::<Vec<_>>()
                        })
                    })
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .min_h_0(),
                )
                .vertical_scrollbar(&self.scroll)
                .into_any_element()
        };
        div()
            .id("http")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(bar)
            .child(body)
    }
}

impl HttpCalls {
    /// Row `i`: what `Row` of `HttpCalls.tsx` drew, as an option of the list box (VL-25).
    fn row(
        &self,
        i: usize,
        state: &AppState,
        focused: bool,
        focus_at: Option<&CallRef>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let r = &self.rows[i];
        let at = &r.at;
        let selected = state.selected_call.as_ref() == Some(at);
        let is_focus_row = focus_at == Some(at);
        // from the arrow keys, or by Tab after a click on a row, as `:focus-visible` in Electron Grace
        let ring = focused && is_focus_row && (self.keyboard || window.last_input_was_keyboard());
        let (fill, ink) = status_tint(r.status).colours(t);
        let text = &r.text;
        let badge = tooltip(part("status-tip", at), Some(SharedString::from(group_label(r.status))))
            .side(Side::Right)
            .child(
                div()
                    .id(part("status", at))
                    .debug_selector({
                        let i = i + 1;
                        move || format!("http-call-status:{i}")
                    })
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .w(px(44.))
                    .h(px(20.))
                    .rounded_full()
                    .bg(Theme::color(fill))
                    .text_color(Theme::color(ink))
                    .text_size(px(METRICS.text_2xs.size))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .font_family(MONO)
                    .child(text.status.clone()),
            );
        let muted_cut = |what: &'static str, shown: String, max: f32| {
            let n = i + 1;
            div()
                .debug_selector(move || format!("http-call-{what}:{n}"))
                .flex_none()
                .max_w(px(max))
                .min_w_0()
                .overflow_hidden()
                .text_color(Theme::color(t.muted))
                .child(cut(part(what, at), shown).side(Side::Over))
        };
        let description = [text.attempt.clone(), text.via.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
        let me = cx.entity();
        let click_at = at.clone();
        div()
            .id(part("row", at))
            .debug_selector({
                let i = i + 1;
                move || format!("http-call:{i}")
            })
            .role(Role::ListBoxOption)
            .aria_label(r.name.clone())
            .when(!description.is_empty(), |d| d.aria_description(description))
            .aria_selected(selected)
            .aria_size_of_set(self.rows.len())
            .aria_position_in_set(i + 1)
            .when(is_focus_row, |d| d.aria_active_descendant())
            .cursor_pointer()
            .relative()
            .w_full()
            .h(px(ROW_HEIGHT))
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .border_b_1()
            .border_color(Theme::color(t.line))
            .text_size(px(METRICS.text_xs.size))
            .text_color(Theme::color(t.ink))
            .map(|d| {
                if selected {
                    d.bg(Theme::color(t.selected))
                } else {
                    d.hover(|s| s.bg(Theme::color(t.hover)))
                }
            })
            // GPUI gives a box one border colour: the accent bar and the ring are children over the row, so the
            // separator below keeps its colour and the text does not move
            .when(selected, |d| {
                d.child(
                    div()
                        .debug_selector({
                            let i = i + 1;
                            move || format!("http-call-bar:{i}")
                        })
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(2.))
                        .bg(Theme::color(t.accent)),
                )
            })
            .when(ring, |d| {
                d.child(
                    div()
                        .debug_selector({
                            let i = i + 1;
                            move || format!("http-call-ring:{i}")
                        })
                        .absolute()
                        .inset_0()
                        .border_2()
                        .border_color(Theme::color(t.focus)),
                )
            })
            .on_click(move |_, window, cx| {
                let at = click_at.clone();
                me.update(cx, |list, cx| list.clicked(at, window, cx));
            })
            .child(badge)
            .child(
                div()
                    .flex_none()
                    .w(px(56.))
                    .font_family(MONO)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(text.method.clone()),
            )
            .child(
                div()
                    .debug_selector({
                        let i = i + 1;
                        move || format!("http-call-url:{i}")
                    })
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .font_family(MONO)
                    .child(
                        cut(part("place", at), text.place.clone())
                            .whole(text.place_whole.clone())
                            .side(Side::Over),
                    ),
            )
            .child(muted_cut("test", text.test.clone(), 160.))
            .when_some(text.attempt.clone(), |d, a| {
                d.child(div().flex_none().text_color(Theme::color(t.muted)).child(a))
            })
            .when_some(text.via.clone(), |d, v| d.child(muted_cut("via", v, 224.)))
            .child(
                div()
                    .flex_none()
                    .w(px(64.))
                    .flex()
                    .justify_end()
                    .font_family(MONO)
                    .child(text.time.clone()),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
