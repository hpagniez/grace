//! `Tree.tsx` and `VirtualList.tsx`: the project's tests, one row per folder, feature, outline, scenario and example
//! row, on GPUI's `uniform_list`, which draws only the rows in view (VL-01).
//!
//! The tree is one tab stop (VL-06). Electron Grace moved the DOM focus from row to row; here the keyboard focus
//! stays on the tree and the row it is on reports itself as the focused node to assistive technology
//! (`aria_active_descendant`), so a screen reader follows the rows as it did. The rows' buttons are out of the tab
//! order, as they were (VL-11).
//!
//! The debug button, the breakpoint dot (phase P4), "Open in VS Code" from a row's menu (P3) and the sub-folder
//! buttons of an empty project (P4) come with their phases (P2 plan, Redesigns).
use crate::elements::Off as _;
use gpui_component::scroll::ScrollableElement as _;
use std::collections::HashSet;
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement,
    Render, ScrollStrategy, SharedString, StatefulInteractiveElement as _, Styled, Subscription,
    UniformListScrollHandle, Window, div, point, prelude::FluentBuilder as _, px, uniform_list,
};
use gpui_component::button::{Button, ButtonVariants as _};
use grace_core::model::flaky::FLAKY_TITLE;
use grace_core::model::node::{NodeKind, Status};
use grace_core::model::status::derive_status;
use grace_core::model::tags::FilterState;
use grace_core::rows::{
    TreeRow, Turned, View, ancestors, anchored_scroll_top, feature_of, flaky_chip, label_of, leaves, page_rows,
    row_matches, row_name, status_label, survivor, tree_rows,
};
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{MarkStatus, icon, status_mark};
use crate::theme::{METRICS, Theme};
use crate::tooltip::{Side, cut, tooltip};

/// px, a row (VL-01).
pub const ROW_HEIGHT: f32 = 24.;
/// Under "No feature files found", where Grace looks.
const EMPTY_HINT: &str = "Grace looks for .feature files under the project folder.";
/// px before the first row and after the last.
pub const PAD: f32 = 8.;
/// px a level indents its rows.
pub const INDENT: f32 = 16.;
/// What a row's keys do beside moving, said to assistive technology (VL-22).
pub const SHORTCUTS: &str = "Control+Enter";

/// The run in progress, or the last: the features it may no longer open, and how many failed keys it has read
/// (#224 R3).
#[derive(Default)]
struct RunTrack {
    id: Option<String>,
    done: HashSet<String>,
    read: usize,
}

pub struct Tree {
    grace: Entity<Grace>,
    pub focus: FocusHandle,
    scroll: UniformListScrollHandle,
    turned: Turned,
    /// Grows with each turn, so the rows are built again.
    turns: u64,
    /// The open project's folder: another project starts afresh (TO-04).
    root: Option<String>,
    rows: Rc<Vec<TreeRow>>,
    keys: Rc<Vec<String>>,
    /// What `rows` were built from: the state's tree count, the filter and the turns.
    built: Option<(u64, FilterState, u64)>,
    /// The row last focused or moved to; the focus row is it, else the selected row, else the first (VL-06).
    pub(crate) active: Option<String>,
    /// The focus row came there from the keyboard, so it shows the ring.
    keyboard: bool,
    /// The row under the pointer: its run button shows green, as on the selected and the focus row.
    hovered: Option<String>,
    run: RunTrack,
    seen_reveals: u32,
    seen_pause: (Option<String>, u32),
    seen_filter: Option<FilterState>,
    /// The selected row scrolls into view once the rows are built (VL-20, a pause, a reveal).
    reveal_selected: bool,
    _observe: Subscription,
}

impl Tree {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&grace, |tree: &mut Tree, grace, cx| {
            let state = grace.read(cx).state.clone();
            tree.follow(&state);
            cx.notify();
        });
        let state = grace.read(cx).state.clone();
        let mut tree = Self {
            grace,
            // the tree is one tab stop (VL-06)
            focus: cx.focus_handle().tab_stop(true),
            scroll: UniformListScrollHandle::new(),
            turned: Turned::default(),
            turns: 0,
            root: None,
            rows: Rc::default(),
            keys: Rc::default(),
            built: None,
            active: None,
            keyboard: false,
            hovered: None,
            run: RunTrack::default(),
            seen_reveals: state.reveals,
            seen_pause: (None, 0),
            seen_filter: None,
            reveal_selected: false,
            _observe: observe,
        };
        tree.follow(&state);
        tree
    }

    /// The keys of the rows, in order.
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// The rows, shared: the same `Rc` while nothing they are built from changed.
    pub fn rows(&self) -> Rc<Vec<TreeRow>> {
        Rc::clone(&self.rows)
    }

    pub fn is_open(&self, key: &str) -> bool {
        self.turned.is_open(key)
    }

    fn set_open(&mut self, key: &str, open: bool) {
        if self.turned.is_open(key) != open {
            self.turned.set_open(key, open);
            self.turns += 1;
        }
    }

    /// Opens the rows above `key`, so the list can scroll to the selected row (TO-16).
    fn show(&mut self, state: &AppState, key: &str) {
        let Some(tree) = &state.tree else { return };
        for a in ancestors(tree, key) {
            self.set_open(&a, true);
        }
        self.reveal_selected = true;
    }

    /// What `Tree.tsx`'s effects did on a new state, then the rows built again when what they read changed.
    fn follow(&mut self, state: &AppState) {
        let root = state.project.as_ref().map(|p| p.root.clone());
        if root != self.root {
            self.root = root;
            self.turned = Turned::default();
            self.turns += 1;
            self.active = None;
            self.run = RunTrack::default();
        }
        let pause = (state.paused.as_ref().and_then(|p| p.key.clone()), state.run.pauses);
        if pause != self.seen_pause {
            self.seen_pause = pause.clone();
            if let Some(key) = pause.0 {
                self.show(state, &key);
            }
        }
        if state.reveals != self.seen_reveals {
            self.seen_reveals = state.reveals;
            if let Some(key) = state.selected.clone() {
                self.show(state, &key);
            }
        }
        self.follow_run(state);
        // by value: the runner sends the filter in force again after every refresh, which must not scroll (VL-20)
        if self.seen_filter.as_ref() != Some(&state.filter) {
            if self.seen_filter.is_some() {
                self.reveal_selected = true;
            }
            self.seen_filter = Some(state.filter.clone());
        }
        self.build(state);
    }

    /// A run opens the feature it started in, and each feature where one of its tests fails, once (#224 R3). The
    /// rows still read no status: they change only when a feature opens.
    fn follow_run(&mut self, state: &AppState) {
        let Some(id) = &state.run.run_id else { return };
        if self.run.id.as_ref() != Some(id) {
            self.run = RunTrack {
                id: Some(id.clone()),
                ..RunTrack::default()
            };
            self.open_for_run(feature_of(state.run.selection.as_deref()));
        }
        while self.run.read < state.run.failed_keys.len() {
            let key = &state.run.failed_keys[self.run.read];
            self.open_for_run(feature_of(Some(key)));
            self.run.read += 1;
        }
    }

    fn open_for_run(&mut self, feature: Option<String>) {
        if let Some(f) = feature
            && self.run.done.insert(f.clone())
        {
            self.set_open(&f, true);
        }
    }

    /// The rows again, when the tree, the filter or a turn changed. Rows added or removed above the view keep the
    /// rows in view still (VL-18, TO-18); a focus row that left hands over to the nearest row above (VL-17).
    fn build(&mut self, state: &AppState) {
        let from = (state.trees, state.filter.clone(), self.turns);
        if self.built.as_ref() != Some(&from) {
            self.built = Some(from);
            let before = Rc::clone(&self.keys);
            let rows = match &state.tree {
                Some(t) => tree_rows(t, |k| self.turned.is_open(k), &state.filter),
                None => Vec::new(),
            };
            self.keys = Rc::new(rows.iter().map(|r| r.key.clone()).collect());
            self.rows = Rc::new(rows);
            let view = self.view();
            let top = anchored_scroll_top(view, &before, &self.keys);
            if top != view.scroll_top {
                self.scroll.0.borrow().base_handle.set_offset(point(px(0.), px(-top)));
            }
            if let Some(a) = self.active.clone()
                && !self.keys.contains(&a)
            {
                // an unfocused list falls back to the selected row while it is there, else to the nearest row
                // above (VL-17)
                let selected_kept = state.selected.as_ref().is_some_and(|s| self.keys.contains(s));
                self.active = if selected_kept {
                    None
                } else {
                    survivor(&a, &before, &self.keys)
                };
            }
        }
        if self.reveal_selected {
            self.reveal_selected = false;
            if let Some(i) = state
                .selected
                .as_ref()
                .and_then(|s| self.keys.iter().position(|k| k == s))
            {
                self.scroll.scroll_to_item(i, ScrollStrategy::Nearest);
            }
        }
    }

    /// Where the list is scrolled and how tall it is.
    fn view(&self) -> View {
        let handle = &self.scroll.0.borrow().base_handle;
        View {
            scroll_top: -f32::from(handle.offset().y),
            height: f32::from(handle.bounds().size.height),
            row_height: ROW_HEIGHT,
            pad: PAD,
        }
    }

    /// The focus row's index: the row last focused if it is there, else the selected one, else the first.
    fn focus_index(&self, state: &AppState) -> Option<usize> {
        let at = |k: &String| self.keys.iter().position(|x| x == k);
        self.active
            .as_ref()
            .and_then(at)
            .or_else(|| state.selected.as_ref().and_then(at))
            .or_else(|| (!self.keys.is_empty()).then_some(0))
    }

    /// Opens or closes a row. A feature the person closes during a run stays closed until it ends (TO-12).
    pub fn toggle(&mut self, key: &str, cx: &mut Context<Self>) {
        let open = self.is_open(key);
        if open && self.grace.read(cx).state.run.active {
            self.run.done.insert(key.to_owned());
        }
        self.set_open(key, !open);
        let state = self.grace.read(cx).state.clone();
        self.build(&state);
        cx.notify();
    }

    fn select(&self, key: &str, cx: &mut Context<Self>) {
        self.grace.update(cx, |g, cx| g.select(key, cx));
    }

    /// Moves to row `index` as an arrow key does: in view, the focus row from the keyboard, selected (VL-07).
    fn move_to(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(key) = self.keys.get(index).cloned() else {
            return;
        };
        self.scroll.scroll_to_item(index, ScrollStrategy::Nearest);
        self.active = Some(key.clone());
        self.keyboard = true;
        self.select(&key, cx);
        cx.notify();
    }

    fn move_to_key(&mut self, key: &str, cx: &mut Context<Self>) {
        if let Some(i) = self.keys.iter().position(|k| k == key) {
            self.move_to(i, cx);
        }
    }

    fn on_key(&mut self, e: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let state = self.grace.read(cx).state.clone();
        let Some(f) = self.focus_index(&state) else { return };
        let k = &e.keystroke;
        let m = k.modifiers;
        let plain = !m.control && !m.platform && !m.alt;
        let last = self.keys.len() - 1;
        let page = page_rows(self.view());
        let to = match k.key.as_str() {
            "down" => Some(f + 1),
            "up" => f.checked_sub(1),
            "home" => Some(0),
            "end" => Some(last),
            "pagedown" => Some((f + page).min(last)),
            "pageup" => Some(f.saturating_sub(page)),
            _ => None,
        };
        let moves = matches!(k.key.as_str(), "down" | "up" | "home" | "end" | "pagedown" | "pageup");
        if plain && moves {
            // at an end too, so nothing else takes the key (VL-09)
            cx.stop_propagation();
            if let Some(to) = to
                && to <= last
                && to != f
            {
                self.move_to(to, cx);
            }
        } else if plain && matches!(k.key.as_str(), "enter" | "space") {
            cx.stop_propagation();
            // a key that does not move still brings a scrolled-away row back
            self.scroll.scroll_to_item(f, ScrollStrategy::Nearest);
            let key = self.keys[f].clone();
            self.select(&key, cx);
        } else if self.row_key(e, f, &state, cx) {
            cx.stop_propagation();
            if self.focus_index(&self.grace.read(cx).state.clone()) == Some(f) {
                self.scroll.scroll_to_item(f, ScrollStrategy::Nearest);
            }
        }
    }

    /// The tree's own keys on the focus row: Ctrl+Enter runs it (VL-10), Right and Left open, close and move to a
    /// child or the parent (VL-08). True when handled.
    fn row_key(&mut self, e: &KeyDownEvent, f: usize, state: &AppState, cx: &mut Context<Self>) -> bool {
        let k = &e.keystroke;
        let m = k.modifiers;
        let row = self.rows[f].clone();
        let Some(tree) = &state.tree else { return false };
        let node = row.node(tree);
        if m.control && !m.shift && k.key == "enter" {
            // as the run button does: nothing where it is missing or disabled
            if !busy(state) && row_matches(node, &state.filter) {
                self.grace.update(cx, |g, cx| g.run(row.key.clone(), cx));
            }
            return true;
        }
        if m.control || m.platform || m.alt {
            return false;
        }
        let kids = !node.children.is_empty();
        let open = kids && self.is_open(&row.key);
        match k.key.as_str() {
            "right" => {
                if kids && !open {
                    self.toggle(&row.key, cx);
                } else if let Some(child) = self.rows.get(f + 1)
                    && child.parent_key.as_deref() == Some(row.key.as_str())
                {
                    let key = child.key.clone();
                    self.move_to_key(&key, cx);
                }
                true
            }
            "left" => {
                if open {
                    self.toggle(&row.key, cx);
                } else if let Some(parent) = row.parent_key {
                    self.move_to_key(&parent, cx);
                }
                true
            }
            _ => false,
        }
    }

    /// A click on a row: the row is the focus row, not from the keyboard, and the tree takes the focus.
    fn clicked(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.active = Some(key.to_owned());
        self.keyboard = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }
}

/// A folder's or a feature's icon beside its name; a scenario, an outline and its rows have none.
fn kind_icon(kind: NodeKind, open: bool) -> Option<Icon> {
    match kind {
        NodeKind::Feature => Some(Icon::FileText),
        NodeKind::Folder if open => Some(Icon::FolderOpen),
        NodeKind::Folder => Some(Icon::Folder),
        _ => None,
    }
}

/// A row's run button is muted, and green on the selected row, the row under the pointer and the focus row
/// (`Tree.tsx`'s `group-hover`, `group-focus-within`).
fn run_is_green(selected: bool, hovered: bool, focus_row: bool) -> bool {
    selected || hovered || focus_row
}

fn busy(state: &AppState) -> bool {
    state.run.active || state.busy
}

/// A status as its mark shows it.
pub fn mark(s: Status) -> MarkStatus {
    match s {
        Status::NotRun => MarkStatus::NotRun,
        Status::Pending => MarkStatus::Pending,
        Status::Running => MarkStatus::Running,
        Status::Passed => MarkStatus::Passed,
        Status::Failed => MarkStatus::Failed,
        Status::Interrupted => MarkStatus::Interrupted,
    }
}

/// An id unique to a row's part.
fn part(what: &str, key: &str) -> SharedString {
    SharedString::from(format!("{what}:{key}"))
}

impl Render for Tree {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.grace.read(cx).state.clone();
        let t = Theme::global(cx).tokens;
        let message = |child: AnyElement| {
            div()
                .id("tree-message")
                .debug_selector(|| "tree-message".into())
                .size_full()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .p_2()
                .child(child)
                .into_any_element()
        };
        if let Some(error) = &state.error {
            return message(
                crate::ui::notice("tree-error", crate::ui::NoticeKind::Error, error.clone(), vec![], cx)
                    .into_any_element(),
            );
        }
        let Some(tree) = state.tree.as_ref().filter(|_| state.project.is_some()) else {
            return message(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .text_size(px(METRICS.text_sm.size))
                    .text_color(Theme::color(t.muted))
                    .child(icon(Icon::LoaderCircle, 16., Theme::color(t.muted)))
                    .child(
                        div()
                            .id("tree-finding")
                            .role(Role::Label)
                            .aria_value(crate::progress::finding_text(&state))
                            .child(crate::progress::finding_text(&state)),
                    )
                    .into_any_element(),
            );
        };
        if leaves(tree).is_empty() {
            return message(
                crate::ui::empty_state(
                    "tree-empty",
                    Icon::FileSearch,
                    "No feature files found",
                    Some(
                        // a label, so a screen reader reads the hint as it read Electron Grace's (PU-28)
                        div()
                            .id("tree-empty-hint")
                            .role(Role::Label)
                            .aria_value(EMPTY_HINT)
                            .text_size(px(METRICS.text_xs.size))
                            .text_color(Theme::color(t.muted))
                            .child(EMPTY_HINT)
                            .into_any_element(),
                    ),
                    cx,
                )
                .into_any_element(),
            );
        }

        let focused = self.focus.is_focused(window);
        let focus_key = self.focus_index(&state).map(|i| self.keys[i].clone());
        let count = self.rows.len();
        let me = cx.entity();
        div()
            .id("tree")
            .debug_selector(|| "tree".into())
            .role(Role::Tree)
            .aria_label("Tests")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(
                uniform_list("tree-rows", count, move |range, window, cx| {
                    me.update(cx, |tree, cx| {
                        let state = tree.grace.read(cx).state.clone();
                        range
                            .map(|i| tree.row(i, &state, focused, focus_key.as_deref(), window, cx))
                            .collect::<Vec<_>>()
                    })
                })
                .track_scroll(&self.scroll)
                .py(px(PAD))
                .flex_1()
                .min_h_0(),
            )
            .vertical_scrollbar(&self.scroll)
            .into_any_element()
    }
}

impl Tree {
    /// Row `i`: what `Row` of `Tree.tsx` drew, as a tree item named from its status, flaky chip and tags.
    fn row(
        &self,
        i: usize,
        state: &AppState,
        focused: bool,
        focus_key: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let r = &self.rows[i];
        let Some(tree) = &state.tree else {
            return div().into_any_element();
        };
        let node = r.node(tree);
        let key = r.key.clone();
        let label = label_of(node);
        let status = derive_status(node, &state.statuses);
        let paused = state.paused.as_ref().and_then(|p| p.key.as_deref()) == Some(key.as_str());
        let flaky = flaky_chip(node, &state.flaky);
        let kids = !node.children.is_empty();
        let open = kids && self.is_open(&key);
        let selected = state.selected.as_deref() == Some(key.as_str());
        let matches = row_matches(node, &state.filter);
        let is_focus_row = focus_key == Some(key.as_str());
        // from the arrow keys, or by Tab after a click on a row, as `:focus-visible` in Electron Grace (PU-23)
        let ring = focused && is_focus_row && (self.keyboard || window.last_input_was_keyboard());
        let title = (node.kind == NodeKind::Feature && node.name != label).then(|| node.name.clone());
        let ink = if matches { t.ink } else { t.muted };
        let is_feature = node.kind == NodeKind::Feature;

        let kind_icon = kind_icon(node.kind, open);
        let hovered = self.hovered.as_deref() == Some(key.as_str());
        let me = cx.entity();
        let toggle_key = key.clone();
        let chevron = Button::new(part("toggle", &key))
            .ghost()
            .tab_stop(false)
            .w(px(ROW_HEIGHT))
            .h(px(ROW_HEIGHT))
            .px_0()
            .accessibility_label(format!("{} {label}", if open { "Collapse" } else { "Expand" }))
            // a child, not GPUI Kit's `icon`, which would draw it at 16 px
            .child(
                crate::elements::glyph(if open { Icon::ChevronDown } else { Icon::ChevronRight }, 14.)
                    .text_color(Theme::color(t.muted)),
            )
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                me.update(cx, |tree, cx| {
                    tree.clicked(&toggle_key, window, cx);
                    tree.toggle(&toggle_key, cx);
                });
            });
        // a test has no chevron, only its room, and nothing for a screen reader to read
        let chevron = if kids {
            chevron.into_any_element()
        } else {
            div().flex_none().w(px(ROW_HEIGHT)).h(px(ROW_HEIGHT)).into_any_element()
        };
        let shown_mark = if paused { MarkStatus::Paused } else { mark(status) };
        let mark_tip = if paused { "Paused" } else { status_label(status) };
        let mark = tooltip(part("mark-tip", &key), Some(SharedString::from(mark_tip)))
            .side(Side::Right)
            .child(status_mark(
                part("mark", &key),
                shown_mark,
                16.,
                false,
                cx.reduce_motion(),
                cx,
            ));
        let name = div()
            .flex()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .items_center()
            .gap_2()
            .text_size(px(METRICS.text_sm.size))
            .child(
                cut(part("label", &key), label.clone())
                    .whole(match &title {
                        Some(title) => format!("{label} · {title}"),
                        None => label.clone(),
                    })
                    .side(Side::Over)
                    .keyboard(ring),
            )
            .when_some(title, |d, title| {
                // the title gives way first, then the file name, which the person navigates by
                d.child(
                    div()
                        .min_w_0()
                        .flex_shrink(1000.)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(METRICS.text_xs.size))
                        .text_color(Theme::color(t.muted))
                        .child(title),
                )
            });
        let tags = node.tags.iter().map(|tag| crate::ui::tag_chip(tag.clone(), cx));
        let flaky_badge = flaky.clone().map(|words| {
            let badge = div()
                .debug_selector({
                    let key = key.clone();
                    move || format!("flaky-mark:{key}")
                })
                .flex()
                .flex_none()
                .items_center()
                .gap_1()
                .h(px(20.))
                .px_2()
                .rounded_full()
                .bg(Theme::color(t.cancelled_tint))
                .text_color(Theme::color(t.cancelled))
                .text_size(px(METRICS.text_2xs.size))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(icon(Icon::Shuffle, 14., Theme::color(t.cancelled)))
                .child(words);
            if node.is_leaf() {
                tooltip(part("flaky-tip", &key), Some(SharedString::from(FLAKY_TITLE)))
                    .side(Side::Left)
                    .child(
                        badge
                            .id(part("flaky-mark", &key))
                            .role(Role::Image)
                            .aria_label(FLAKY_TITLE),
                    )
                    .into_any_element()
            } else {
                badge.into_any_element()
            }
        });
        let run = matches.then(|| {
            let run_key = key.clone();
            let me = cx.entity();
            let grace = self.grace.clone();
            let tip = format!("Run {label} (Ctrl+Enter)");
            tooltip(part("run-tip", &key), Some(SharedString::from(tip)))
                .side(Side::Left)
                .child(
                    run_button(part("run", &key), &label)
                        .child(crate::elements::glyph(Icon::Play, 14.).text_color(Theme::color(
                            if run_is_green(selected, hovered, ring) {
                                t.passed
                            } else {
                                t.muted
                            },
                        )))
                        .off(busy(state))
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            me.update(cx, |tree, cx| tree.clicked(&run_key, window, cx));
                            let key = run_key.clone();
                            grace.update(cx, |g, cx| g.run(key, cx));
                        }),
                )
        });

        let me = cx.entity();
        let click_key = key.clone();
        let row = div()
            .id(part("row", &key))
            .debug_selector({
                let key = key.clone();
                move || format!("tree-row:{key}")
            })
            .cursor_pointer()
            .role(Role::TreeItem)
            .aria_label(row_name(&label, status, flaky.as_deref(), &node.tags))
            .aria_level(r.level)
            .aria_size_of_set(r.set_size)
            .aria_position_in_set(r.pos_in_set)
            .aria_selected(selected)
            .when(kids, |d| d.aria_expanded(open))
            .aria_keyshortcuts(SHORTCUTS)
            .when(is_focus_row, |d| d.aria_active_descendant())
            .h(px(ROW_HEIGHT))
            .flex_1()
            .min_w_0()
            .pr_1()
            .flex()
            .items_center()
            .gap_1()
            .rounded(px(4.))
            .text_color(Theme::color(ink))
            .map(|d| {
                if selected {
                    d.bg(Theme::color(t.selected))
                        .border_l_2()
                        .border_color(Theme::color(t.accent))
                } else {
                    d.hover(|s| s.bg(Theme::color(t.hover)))
                }
            })
            .when(ring, |d| d.border_2().border_color(Theme::color(t.focus)))
            .on_hover(cx.listener({
                let key = key.clone();
                move |tree, on: &bool, _, cx| {
                    let now = on.then(|| key.clone());
                    if *on || tree.hovered.as_ref() == Some(&key) {
                        tree.hovered = now;
                        cx.notify();
                    }
                }
            }))
            .on_click(move |_, window, cx| {
                me.update(cx, |tree, cx| {
                    tree.clicked(&click_key, window, cx);
                    tree.select(&click_key, cx);
                    // a click opens a feature, never closes it (TO-05, TO-06)
                    if is_feature && !tree.is_open(&click_key) {
                        tree.toggle(&click_key, cx);
                    }
                });
            })
            .child(chevron)
            .child(mark)
            .when_some(kind_icon, |d, k| d.child(icon(k, 14., Theme::color(t.muted))))
            .child(name)
            .children(tags)
            .children(flaky_badge)
            .children(run);
        // A list lays its rows out alone, as wide as their content: the wrapper takes the list's width, so a long
        // label is cut, with its tooltip, rather than drawn past the window.
        div()
            .w_full()
            .h(px(ROW_HEIGHT))
            .flex()
            .pl(px(8. + (r.level - 1) as f32 * INDENT))
            .pr(px(8.))
            .child(row)
            .into_any_element()
    }
}

/// A row's run button: out of the Tab order (the row's Ctrl+Enter runs it), the row's height, and the hand under
/// the pointer as on every button (`Tree.tsx`); `Off::off` turns it to the not-allowed sign during a run.
fn run_button(id: impl Into<gpui::ElementId>, label: &str) -> Button {
    Button::new(id)
        .ghost()
        .tab_stop(false)
        .w(px(ROW_HEIGHT))
        .h(px(ROW_HEIGHT))
        .px_0()
        .cursor_pointer()
        .accessibility_label(format!("Run {label}"))
}

#[cfg(test)]
mod tests;
