//! `BottomPanel.tsx` and `Splitter.tsx`: the Console, History and HTTP tabs under the tree and details, and Variables
//! during a debug run, one height for all, with the splitter on top that resizes it (UI polish R1, R18) and tabs that
//! move (reorder panel tabs). P2 draws the Console; the History and HTTP tabs' content comes with P3 and Variables'
//! with P4 (P2 plan), so their panels stay empty until then.
//!
//! Electron Grace read the panel's and the main area's heights from the layout (`getBoundingClientRect`, a
//! `ResizeObserver`); GPUI tells them after each layout, which the panel keeps (`canvas`), at no cost while idle,
//! when no frame is drawn.
use std::cell::Cell;
use std::rc::Rc;

use gpui::accesskit::{Orientation, Role};
use gpui::{
    App, AppContext as _, Context, CursorStyle, DragMoveEvent, Empty, Entity, FocusHandle, InteractiveElement as _,
    IntoElement, KeyDownEvent, MouseButton, ParentElement as _, Pixels, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, canvas, div, prelude::FluentBuilder as _, px,
};
use grace_core::panel_height::{PANEL_MIN, PanelKey, clamp_panel, key_height, max_panel};
use grace_core::panel_tabs::{TabId, move_tab};
use grace_core::settings::{load_panel_height, load_tab_order, save_panel_height, save_tab_order};
use grace_core::state::{AppState, Panel};

use crate::app::Grace;
use crate::assets::Icon;
use crate::console::{Console, FIND};
use crate::elements::icon;
use crate::theme::Theme;
use crate::tooltip::tooltip;

pub const TABS: &str = "Console, run history and HTTP calls";
pub const RESIZE: &str = "Resize bottom panel";

/// A tab's panel, icon and label.
fn tab_look(id: TabId) -> (Panel, Icon, &'static str) {
    match id {
        TabId::Console => (Panel::Console, Icon::SquareTerminal, "Console"),
        TabId::History => (Panel::History, Icon::History, "History"),
        TabId::Http => (Panel::Http, Icon::Globe, "HTTP"),
    }
}

/// The count a tab shows in its pill: History's runs. HTTP's calls come with its panel, in P3.
fn tab_count(id: TabId, state: &AppState) -> Option<usize> {
    match id {
        TabId::History => Some(state.history.len()),
        _ => None,
    }
}

/// A tab's accessible name: the brackets are spoken only, the tab reads "History (2)" (R18.2).
pub fn tab_name(id: TabId, state: &AppState) -> SharedString {
    let label = tab_look(id).2;
    match tab_count(id, state) {
        Some(n) => format!("{label} ({n})").into(),
        None => label.into(),
    }
}

/// A tab dragged to a new place.
#[derive(Clone, Copy)]
struct TabDrag(TabId);

/// The splitter being dragged.
#[derive(Clone, Copy)]
struct SplitDrag;

/// A drag of the splitter: where it started, and the height it asks for now.
#[derive(Clone, Copy)]
struct Drag {
    y: Pixels,
    start: f32,
    max: f32,
    h: f32,
}

pub struct BottomPanel {
    grace: Entity<Grace>,
    pub console: Entity<Console>,
    pub order: [TabId; 3],
    /// The saved height; the panel shows it clamped to the room there is.
    pub height: f32,
    /// The panel's height and the room it shares with the main area, after the last layout.
    shown: Rc<Cell<f32>>,
    pub room: Rc<Cell<f32>>,
    drag: Option<Drag>,
    /// The tab being dragged, and the tab it is over.
    from: Option<TabId>,
    over: Option<TabId>,
    pub splitter: FocusHandle,
    tabs: [FocusHandle; 3],
    _subscriptions: Vec<Subscription>,
}

impl BottomPanel {
    pub fn new(grace: Entity<Grace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let data_dir = grace.read(cx).data_dir().to_path_buf();
        let console = cx.new(|cx| Console::new(grace.clone(), window, cx));
        let subscriptions = vec![cx.observe(&grace, |_, _, cx| cx.notify())];
        Self {
            grace,
            console,
            // read before the first paint (R4.2)
            order: load_tab_order(&data_dir),
            height: load_panel_height(&data_dir),
            shown: Rc::new(Cell::new(0.)),
            room: Rc::new(Cell::new(0.)),
            drag: None,
            from: None,
            over: None,
            splitter: cx.focus_handle().tab_stop(true),
            tabs: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            _subscriptions: subscriptions,
        }
    }

    fn state(&self, cx: &App) -> Rc<AppState> {
        self.grace.read(cx).state.clone()
    }

    fn data_dir(&self, cx: &App) -> std::path::PathBuf {
        self.grace.read(cx).data_dir().to_path_buf()
    }

    /// The height drawn and the largest the panel may take, from the last layout.
    pub fn sizes(&self) -> (f32, f32) {
        (self.shown.get().round(), max_panel(self.room.get()))
    }

    fn set_height(&mut self, h: f32, cx: &mut Context<Self>) {
        self.height = h;
        cx.notify();
    }

    pub fn save_height(&self, cx: &App) {
        let _ = save_panel_height(&self.data_dir(cx), self.height); // a height that cannot be saved is only forgotten
    }

    /// A key on the splitter: Up and Down step, Home and End jump (R1.3).
    pub fn key(&mut self, key: PanelKey, cx: &mut Context<Self>) {
        let (shown, max) = self.sizes();
        let h = key_height(key, shown, max);
        if h == shown {
            return; // at a limit: keep the saved height
        }
        self.set_height(h, cx);
        self.save_height(cx);
    }

    fn start_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let (shown, max) = self.sizes();
        self.drag = Some(Drag {
            y,
            start: shown,
            max,
            h: shown,
        });
        cx.notify();
    }

    fn move_drag(&mut self, y: Pixels, cx: &mut Context<Self>) {
        let Some(d) = self.drag.as_mut() else { return };
        d.h = clamp_panel(d.start + f32::from(d.y - y), d.max);
        let h = d.h;
        self.set_height(h, cx);
    }

    /// Release, wherever it happens: only the first end counts.
    fn end_drag(&mut self, cx: &mut Context<Self>) {
        let Some(d) = self.drag.take() else { return };
        if d.h != d.start {
            self.save_height(cx);
        }
        cx.notify();
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Moves `id` to place `to` and saves the order; nothing if it stays (R2.1, R3.1).
    pub fn place(&mut self, id: TabId, to: isize, cx: &mut Context<Self>) {
        if let Some(next) = move_tab(&self.order, id, to) {
            self.order = next;
            let _ = save_tab_order(&self.data_dir(cx), &next); // an order that cannot be saved is only forgotten
            cx.notify();
        }
    }

    fn index(&self, id: TabId) -> isize {
        self.order.iter().position(|t| *t == id).unwrap_or(0) as isize
    }

    /// The side of `id` a dragged tab will land on: it takes that tab's place.
    fn mark(&self, id: TabId, from: Option<TabId>) -> Option<bool> {
        let from = from?;
        (self.over == Some(id) && from != id).then(|| self.index(from) < self.index(id))
    }

    fn splitter_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let (shown, max) = self.sizes();
        let this = cx.entity();
        let pressed = self.drag.is_some();
        div()
            .id("splitter")
            .debug_selector(|| "splitter".into())
            .role(Role::Splitter)
            .aria_label(RESIZE)
            .aria_orientation(Orientation::Horizontal)
            .aria_min_numeric_value(f64::from(PANEL_MIN))
            .aria_max_numeric_value(f64::from(max))
            .aria_numeric_value(f64::from(shown))
            .track_focus(&self.splitter)
            .relative()
            .h(px(1.))
            .flex_none()
            .bg(Theme::color(t.line))
            .cursor(CursorStyle::ResizeUpDown)
            .on_key_down({
                let this = this.clone();
                move |e: &KeyDownEvent, _, cx| {
                    let key = match e.keystroke.key.as_str() {
                        "up" => PanelKey::Up,
                        "down" => PanelKey::Down,
                        "home" => PanelKey::Home,
                        "end" => PanelKey::End,
                        _ => return,
                    };
                    cx.stop_propagation();
                    this.update(cx, |p, cx| p.key(key, cx));
                }
            })
            // the 6 px hit area, centred on the line, and the 2 px accent line on hover and press (R18.1)
            .child(
                div()
                    .id("splitter-hit")
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(-3.))
                    .h(px(6.))
                    .cursor(CursorStyle::ResizeUpDown)
                    .group("splitter")
                    .on_mouse_down(MouseButton::Left, {
                        let this = this.clone();
                        move |e, window, cx| {
                            window.prevent_default(); // no text selection
                            this.update(cx, |p, cx| p.start_drag(e.position.y, cx));
                        }
                    })
                    .on_drag(SplitDrag, |_, _, _, cx| cx.new(|_| Empty))
                    .child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(px(3.))
                            .h(px(2.))
                            .bg(Theme::color(t.accent))
                            .opacity(if pressed { 1. } else { 0. })
                            .group_hover("splitter", |s| s.opacity(1.)),
                    ),
            )
    }

    fn tab_view(
        &self,
        id: TabId,
        k: usize,
        from: Option<TabId>,
        state: &AppState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let (panel, glyph, label) = tab_look(id);
        let active = state.panel == panel;
        let count = tab_count(id, state);
        let name = tab_name(id, state);
        let this = cx.entity();
        let grace = self.grace.clone();
        let drop_side = self.mark(id, from);
        div()
            .id(SharedString::from(format!("tab-{}", id.name())))
            .debug_selector(move || format!("tab-{}", id.name()))
            .role(Role::Tab)
            .aria_label(name)
            .aria_selected(active)
            .track_focus(&self.tabs[k])
            .flex()
            .items_center()
            .gap_1p5()
            .h(px(32.))
            .px_2()
            .border_b_2()
            .border_color(if active { Theme::color(t.accent) } else { Theme::clear() })
            .text_sm()
            .text_color(Theme::color(if active { t.ink } else { t.muted }))
            .cursor_pointer()
            .hover(|s| s.text_color(Theme::color(t.ink)))
            .when_some(drop_side, |d, right| {
                if right {
                    d.border_r_2().border_color(Theme::color(t.accent))
                } else {
                    d.border_l_2().border_color(Theme::color(t.accent))
                }
            })
            .on_click(move |_, _, cx| grace.update(cx, |g, cx| g.show_panel(panel, cx)))
            .on_key_down({
                let this = this.clone();
                move |e: &KeyDownEvent, window, cx| {
                    let m = e.keystroke.modifiers;
                    if !m.alt || m.control || m.shift || m.platform {
                        return;
                    }
                    let step = match e.keystroke.key.as_str() {
                        "left" => -1,
                        "right" => 1,
                        _ => return,
                    };
                    cx.stop_propagation();
                    this.update(cx, |p, cx| {
                        let to = p.index(id) + step;
                        p.place(id, to, cx);
                        // the moved tab keeps the focus
                        let k = p.order.iter().position(|t| *t == id).unwrap_or(0);
                        p.tabs[k].focus(window, cx);
                    });
                }
            })
            .on_drag(TabDrag(id), {
                let this = this.clone();
                move |_, _, _, cx| {
                    this.update(cx, |p, _| p.from = Some(id));
                    cx.new(|_| Empty)
                }
            })
            .on_drag_move::<TabDrag>({
                let this = this.clone();
                move |e: &DragMoveEvent<TabDrag>, _, cx| {
                    let inside = e.bounds.contains(&e.event.position);
                    this.update(cx, |p, cx| {
                        if inside && p.over != Some(id) {
                            p.over = Some(id);
                            cx.notify();
                        } else if !inside && p.over == Some(id) {
                            p.over = None;
                            cx.notify();
                        }
                    });
                }
            })
            .on_drop({
                let this = this.clone();
                move |d: &TabDrag, _, cx| {
                    this.update(cx, |p, cx| {
                        let to = p.index(id);
                        p.place(d.0, to, cx);
                        p.from = None;
                        p.over = None;
                        cx.notify();
                    })
                }
            })
            .child(icon(glyph, 14., Theme::color(if active { t.ink } else { t.muted })))
            .child(label)
            .children(count.map(|n| {
                div()
                    .px_1p5()
                    .rounded_full()
                    .bg(Theme::color(t.control))
                    .text_xs()
                    .text_color(Theme::color(t.muted))
                    .child(n.to_string())
            }))
    }
}

impl Render for BottomPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let state = self.state(cx);
        let (_, max) = self.sizes();
        // the saved height clamped to the room there is; flexbox shrinks it further in a short window (UP-07)
        let h = if self.room.get() > 0. {
            self.height.min(max).max(PANEL_MIN)
        } else {
            self.height
        };
        let shown = self.shown.clone();
        let this = cx.entity();
        if !cx.has_active_drag() {
            // the drag ended outside the tabs, or with Escape: nothing moves
            self.from = None;
            self.over = None;
        }
        let from = self.from;
        let tabs: Vec<_> = self
            .order
            .into_iter()
            .enumerate()
            .map(|(k, id)| self.tab_view(id, k, from, &state, cx).into_any_element())
            .collect();
        let console = self.console.clone();
        let find = tooltip("tip-find", Some(SharedString::from("Find (Ctrl+F)"))).focusable(move |f| {
            crate::elements::icon_button("find-button", Icon::Search, FIND, true)
                .track_focus(f)
                .on_click(move |_, window, cx| console.update(cx, |c, cx| c.open_bar(window, cx)))
        });
        let tools = (state.panel == Panel::Console).then(|| {
            div()
                .ml_auto()
                .flex()
                .items_center()
                .gap_1()
                .children(Console::errors_group(&self.console, cx))
                .child(find)
        });
        let _ = window;
        div()
            .id("bottom-panel")
            .debug_selector(|| "bottom-panel".into())
            .flex()
            .flex_col()
            .flex_shrink(1.)
            .h(px(h))
            .min_h(px(PANEL_MIN))
            .on_drag_move::<SplitDrag>({
                let this = this.clone();
                move |e: &DragMoveEvent<SplitDrag>, _, cx| this.update(cx, |p, cx| p.move_drag(e.event.position.y, cx))
            })
            .on_mouse_up(MouseButton::Left, {
                let this = this.clone();
                move |_, _, cx| this.update(cx, |p, cx| p.end_drag(cx))
            })
            .on_mouse_up_out(MouseButton::Left, {
                let this = this.clone();
                move |_, _, cx| this.update(cx, |p, cx| p.end_drag(cx))
            })
            .child(
                canvas(move |b, _, _| shown.set(f32::from(b.size.height)), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .child(self.splitter_view(cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(Theme::color(t.line))
                    .px_2()
                    .child(
                        div()
                            .id("panel-tabs")
                            .role(Role::TabList)
                            .aria_label(TABS)
                            .flex()
                            .items_center()
                            .gap_1()
                            .children(tabs),
                    )
                    .children(tools),
            )
            .child(
                div()
                    .id("tab-panel")
                    .role(Role::TabPanel)
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .when(state.panel == Panel::Console, |d| d.child(self.console.clone())),
            )
    }
}

#[cfg(test)]
mod tests;
