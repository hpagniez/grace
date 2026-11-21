//! `Examples.tsx`: an outline's Examples as a table in Details, a line per row with its values, status and time
//! (docs/specs/2026-11-03-data-driven-view.md). What the table holds is `grace_core::examples`'s.
//!
//! The lines are a `uniform_list`, which draws only the lines in view (DD-07), and a list box of the tree's kind: one
//! tab stop whose keyboard focus stays on the list while its focus line reports itself as the focused node to
//! assistive technology. The arrow keys move the focus line without opening it; a click, Enter or Space opens the
//! row (DD-03, DD-04), which is the tree's selection.
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, Context, Entity, FocusHandle, FontWeight, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement as _, Render, ScrollStrategy, SharedString, StatefulInteractiveElement as _, Styled as _,
    UniformListScrollHandle, Window, div, point, prelude::FluentBuilder as _, px, uniform_list,
};
use gpui_component::scroll::ScrollableElement as _;
use grace_core::examples::{ROW_HEIGHT, Table, line_name, min_height, table};
use grace_core::karate::tree::find_node;
use grace_core::model::node::{NodeKind, Status};
use grace_core::rows::{View, page_rows, status_label};
use grace_core::state::AppState;

use crate::app::Grace;
use crate::details::format_millis;
use crate::elements::status_mark;
use crate::theme::{METRICS, MONO, Theme};
use crate::tooltip::{Side, cut, tooltip};

/// The list's accessible name.
pub const LIST: &str = "Examples";
/// px, the status mark's column, the number's and the time's; the values share what is left.
const MARK: f32 = 24.;
const NUMBER: f32 = 40.;
const TIME: f32 = 72.;

pub struct Examples {
    grace: Entity<Grace>,
    pub focus: FocusHandle,
    scroll: UniformListScrollHandle,
    table: Rc<Table>,
    /// What the table was made from: the tree and the outline.
    built: Option<(u64, String)>,
    /// The row last focused or moved to; the focus line is it, else the first.
    active: Option<String>,
    /// The focus line came there from the keyboard, so it shows the ring.
    keyboard: bool,
}

impl Examples {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        Self {
            grace,
            // the table is one tab stop (R3.2)
            focus: cx.focus_handle().tab_stop(true),
            scroll: UniformListScrollHandle::new(),
            table: Rc::default(),
            built: None,
            active: None,
            keyboard: false,
        }
    }

    /// The table drawn now.
    pub fn table(&self) -> Rc<Table> {
        Rc::clone(&self.table)
    }

    /// The table again when the tree changed. Another outline starts at the top with no focus line of its own; the
    /// same one, refreshed, keeps its place.
    fn build(&mut self, state: &AppState) {
        let Some(outline) = state
            .selected
            .as_ref()
            .zip(state.tree.as_ref())
            .and_then(|(k, tree)| find_node(tree, k))
            .filter(|n| n.kind == NodeKind::Outline)
        else {
            return;
        };
        let from = (state.trees, outline.key.clone());
        if self.built.as_ref() == Some(&from) {
            return;
        }
        if self.built.as_ref().is_none_or(|(_, key)| *key != from.1) {
            self.active = None;
            self.keyboard = false;
            self.scroll.0.borrow().base_handle.set_offset(point(px(0.), px(0.)));
        }
        self.built = Some(from);
        self.table = Rc::new(table(outline));
        if self.active.as_ref().is_some_and(|a| self.index_of(a).is_none()) {
            self.active = None;
        }
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        self.table.lines.iter().position(|l| l.key == key)
    }

    fn view(&self) -> View {
        let handle = &self.scroll.0.borrow().base_handle;
        View {
            scroll_top: -f32::from(handle.offset().y),
            height: f32::from(handle.bounds().size.height),
            row_height: ROW_HEIGHT,
            pad: 0.,
        }
    }

    fn focus_index(&self) -> Option<usize> {
        self.active
            .as_ref()
            .and_then(|a| self.index_of(a))
            .or_else(|| (!self.table.lines.is_empty()).then_some(0))
    }

    /// Opens a row: the details pane shows it and the tree has it selected (DD-03).
    fn open(&self, key: &str, cx: &mut Context<Self>) {
        let key = key.to_owned();
        self.grace.update(cx, |g, cx| g.select(&key, cx));
    }

    fn on_key(&mut self, e: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(f) = self.focus_index() else {
            return;
        };
        let m = e.keystroke.modifiers;
        if m.control || m.platform || m.alt {
            return;
        }
        let last = self.table.lines.len() - 1;
        let page = page_rows(self.view());
        let to = match e.keystroke.key.as_str() {
            "down" => (f + 1).min(last),
            "up" => f.saturating_sub(1),
            "home" => 0,
            "end" => last,
            "pagedown" => (f + page).min(last),
            "pageup" => f.saturating_sub(page),
            "enter" | "space" => {
                cx.stop_propagation();
                let key = self.table.lines[f].key.clone();
                self.open(&key, cx);
                return;
            }
            _ => return,
        };
        // at an end too, so nothing else takes the key
        cx.stop_propagation();
        if to != f {
            self.scroll.scroll_to_item(to, ScrollStrategy::Nearest);
            self.active = Some(self.table.lines[to].key.clone());
            self.keyboard = true;
            cx.notify();
        }
    }

    fn clicked(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        self.active = Some(key.clone());
        self.keyboard = false;
        window.focus(&self.focus, cx);
        self.open(&key, cx);
    }

    /// Line `i`.
    fn line(
        &self,
        i: usize,
        state: &AppState,
        focused: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let line = &self.table.lines[i];
        let status = state.statuses.get(&line.key).copied().unwrap_or(Status::NotRun);
        let time = state
            .results
            .get(&line.key)
            .and_then(|r| r.duration_millis)
            .map(format_millis)
            .unwrap_or_default();
        let is_focus_row = self.focus_index() == Some(i);
        // from the arrow keys, or by Tab after a click, as `:focus-visible` in Electron Grace
        let ring = focused && is_focus_row && (self.keyboard || window.last_input_was_keyboard());
        let n = i + 1;
        let (me, key) = (cx.entity(), line.key.clone());
        let tip = SharedString::from(format!("example-tip:{}", line.key));
        div()
            .id(SharedString::from(format!("example:{}", line.key)))
            .debug_selector(move || format!("example:{n}"))
            .role(Role::ListBoxOption)
            .aria_label(SharedString::from(line_name(
                &self.table,
                i,
                status_label(status),
                &time,
            )))
            .aria_selected(false)
            .aria_size_of_set(self.table.lines.len())
            .aria_position_in_set(n)
            .when(is_focus_row && focused, |d| d.aria_active_descendant())
            .cursor_pointer()
            .relative()
            .w_full()
            .h(px(ROW_HEIGHT))
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .text_size(px(METRICS.text_xs.size))
            .hover(|s| s.bg(Theme::color(t.hover)))
            // the ring is a child over the line, as the HTTP list's: a border would move the text
            .when(ring, |d| {
                d.child(
                    div()
                        .debug_selector(move || format!("example-ring:{n}"))
                        .absolute()
                        .inset_0()
                        .border_2()
                        .border_color(Theme::color(t.focus)),
                )
            })
            .on_click(move |_, window, cx| {
                let key = key.clone();
                me.update(cx, |list, cx| list.clicked(key, window, cx));
            })
            .child(
                tooltip(tip, Some(SharedString::from(status_label(status))))
                    .side(Side::Right)
                    .child(div().flex_none().w(px(MARK)).flex().child(status_mark(
                        SharedString::from(format!("example-mark:{}", line.key)),
                        crate::tree::mark(status),
                        14.,
                        true,
                        cx.reduce_motion(),
                        cx,
                    ))),
            )
            .child(
                div()
                    .flex_none()
                    .w(px(NUMBER))
                    .text_color(Theme::color(t.muted))
                    .child(n.to_string()),
            )
            .children(line.values.iter().enumerate().map(|(c, v)| {
                div()
                    .debug_selector(move || format!("example-value:{n}:{c}"))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .font_family(MONO)
                    .when(!v.is_empty(), |d| {
                        let id = SharedString::from(format!("example-cut:{}:{c}", line.key));
                        d.child(cut(id, v.clone()).side(Side::Over))
                    })
            }))
            .child(
                div()
                    .debug_selector(move || format!("example-time:{n}"))
                    .flex_none()
                    .w(px(TIME))
                    .flex()
                    .justify_end()
                    .font_family(MONO)
                    .text_color(Theme::color(t.muted))
                    .child(time),
            )
            .into_any_element()
    }
}

impl Render for Examples {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.grace.read(cx).state.clone();
        self.build(&state);
        let t = Theme::global(cx).tokens;
        let head = div()
            .debug_selector(|| "examples-head".into())
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .px_2()
            .h(px(ROW_HEIGHT))
            .border_b_1()
            .border_color(Theme::color(t.line))
            .text_size(px(METRICS.text_xs.size))
            .font_weight(FontWeight::MEDIUM)
            .text_color(Theme::color(t.muted))
            .child(div().flex_none().w(px(MARK)))
            .child(div().flex_none().w(px(NUMBER)).child("#"))
            .children(self.table.columns.iter().enumerate().map(|(c, name)| {
                div()
                    .debug_selector(move || format!("example-column:{c}"))
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .child(cut(SharedString::from(format!("example-column-cut:{c}")), name.clone()).side(Side::Over))
            }))
            .child(div().flex_none().w(px(TIME)).flex().justify_end().child("Time"));
        let focused = self.focus.is_focused(window);
        let me = cx.entity();
        let list = div()
            .id("examples")
            .debug_selector(|| "examples".into())
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
                uniform_list("example-lines", self.table.lines.len(), move |range, window, cx| {
                    me.update(cx, |list, cx| {
                        let state = list.grace.read(cx).state.clone();
                        range
                            .map(|i| list.line(i, &state, focused, window, cx))
                            .collect::<Vec<_>>()
                    })
                })
                .track_scroll(&self.scroll)
                .flex_1()
                .min_h_0(),
            )
            .vertical_scrollbar(&self.scroll);
        div()
            .id("examples-table")
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(min_height(self.table.lines.len())))
            .overflow_hidden()
            .rounded_lg()
            .border_1()
            .border_color(Theme::color(t.line))
            .child(head)
            .child(list)
    }
}

#[cfg(test)]
mod tests;
