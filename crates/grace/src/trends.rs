//! `Trends.tsx`: the trend strip at the top of the History tab: per stored run of the picked environment, the pass rate
//! and the time per test of the tree selection (docs/specs/2026-11-04-trends.md). What it says and how tall each bar
//! is are `grace_core::trends`'; this asks the runner and draws the boxes.
//!
//! It is made when the tab shows and dropped when it does not (`History::close`), so nothing is asked or held for a
//! closed tab (R5.3). While it is there it asks the runner for the selection's trend when the tab shows and when the
//! selection, the history, the tree or the environment changes, one question at a time: a change that comes while one is
//! out is asked once that one is answered, for the selection then current, and the answer to a question a newer one has
//! replaced is not drawn (R5.1, R5.2). Nothing runs while nothing changes: no timer, poll or animation (ADR 0030).
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement,
    Render, SharedString, StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use grace_core::state::AppState;
use grace_core::trends::{Column, GAP, NONE, ROW, Strip, strip, trend_caption};

use crate::app::Grace;
use crate::elements::{ring, ring_shown};
use crate::theme::{METRICS, Theme};
use crate::tooltip::{cut, tooltip};

/// px, the strip: 8 above, the caption's line, 4, the two 24 px rows with 4 between them, 8 below (R1.4).
pub const STRIP_HEIGHT: f32 = 88.;
/// px, the caption's line (`text-xs`).
const CAPTION: f32 = 16.;
/// px, a column (`w-3`) and the air between two (`gap-1`).
const COLUMN: f32 = 12.;
/// px, the label beside the columns (`w-36`).
const LABELS: f32 = 144.;
/// px, the rows' height with the air between them: what a column is tall.
const COLUMN_HEIGHT: f32 = 2. * ROW + 4.;

/// A click or Enter on a column: the History tab shows this run's row.
pub struct Picked(pub String);

/// An answer, with the selection it is for and what it says.
pub struct Shown {
    /// The node the answer is for; `None` for the whole tree.
    pub key: Option<String>,
    /// What the strip covers, as the tree named it when the answer came.
    pub caption: String,
    pub strip: Strip,
}

/// What an answer depends on: when any of it changes, the strip asks again.
#[derive(Clone, PartialEq)]
struct Wanted {
    selected: Option<String>,
    runs: Vec<String>,
    trees: u64,
    env: Option<String>,
}

impl Wanted {
    fn of(s: &AppState) -> Self {
        Self {
            selected: s.selected.clone(),
            runs: s.history.iter().map(|e| e.id.clone()).collect(),
            trees: s.trees,
            env: s.env.as_ref().and_then(|e| e.env.clone()),
        }
    }
}

pub struct Trends {
    grace: Entity<Grace>,
    /// What the last question was asked for.
    wanted: Option<Wanted>,
    /// A question is out; the runner is asked one at a time.
    busy: bool,
    /// The selection changed while a question was out.
    again: bool,
    shown: Option<Shown>,
    /// "Now" for a column's tooltip, read when the answer comes.
    clock: Rc<dyn Fn() -> f64>,
}

impl EventEmitter<Picked> for Trends {}

impl Trends {
    pub fn new(grace: Entity<Grace>, clock: Rc<dyn Fn() -> f64>) -> Self {
        Self {
            grace,
            wanted: None,
            busy: false,
            again: false,
            shown: None,
            clock,
        }
    }

    /// The last answer.
    pub fn shown(&self) -> Option<&Shown> {
        self.shown.as_ref()
    }

    /// Asks the runner for the trend of the selection now; once the answer in flight is in when there is one.
    fn ask(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            self.again = true;
            return;
        }
        let key = self.wanted.as_ref().and_then(|w| w.selected.clone());
        self.busy = true;
        let task = self.grace.update(cx, |g, cx| g.trend(key.clone(), cx));
        cx.spawn(async move |this, cx| {
            let points = task.await;
            let _ = this.update(cx, |t, cx| {
                t.busy = false;
                if std::mem::take(&mut t.again) {
                    t.ask(cx); // the answer is for an older selection: not drawn
                    return;
                }
                let state = t.grace.read(cx).state.clone();
                t.shown = Some(Shown {
                    caption: trend_caption(state.tree.as_ref(), key.as_deref()),
                    key,
                    strip: strip(&points, (t.clock)()),
                });
                cx.notify();
            });
        })
        .detach();
    }

    /// One run's column: a button named by its run, its pass bar over its time bar.
    fn column(&self, c: &Column, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let id = c.id.clone();
        let focus = window
            .use_keyed_state(SharedString::from(format!("trend-focus-{id}")), cx, |_, cx| {
                cx.focus_handle().tab_stop(true)
            })
            .read(cx)
            .clone();
        let keyboard = ring_shown(&focus, window);
        let strip = cx.entity();
        let pick = {
            let (strip, id) = (strip.clone(), id.clone());
            move |cx: &mut App| strip.update(cx, |_, cx| cx.emit(Picked(id.clone())))
        };
        let top = |bar: gpui::Div| bar.rounded_tl(px(4.)).rounded_tr(px(4.));
        let part = |what: &'static str, height: f32, colour: u32| {
            let id = id.clone();
            div()
                .debug_selector(move || format!("trend-{what}:{id}"))
                .flex_none()
                .w_full()
                .h(px(height))
                .bg(Theme::color(colour))
        };
        // the pass bar: failed above, passed on the baseline, a gap between them; a share of 0 draws nothing
        let pass = top(div().flex().flex_col().gap(px(GAP)).flex_none().w_full().h(px(ROW)))
            .overflow_hidden()
            .when(c.failed > 0., |d| d.child(part("failed", c.failed, t.failed)))
            .when(c.passed > 0., |d| d.child(part("passed", c.passed, t.passed)));
        let time = div()
            .flex()
            .items_end()
            .flex_none()
            .w_full()
            .h(px(ROW))
            .when_some(c.time.filter(|h| *h > 0.), |d, h| {
                d.child(top(part("time", h, t.muted)))
            });
        let button = ring(
            div()
                .id(SharedString::from(format!("trend-run-{id}")))
                .debug_selector({
                    let id = id.clone();
                    move || format!("trend-run:{id}")
                })
                .role(Role::Button)
                .aria_label(c.tip.clone())
                .track_focus(&focus)
                .flex()
                .flex_col()
                .gap(px(4.))
                .w(px(COLUMN))
                .h(px(COLUMN_HEIGHT))
                .cursor_pointer()
                .on_click({
                    let pick = pick.clone();
                    move |_, _, cx| pick(cx)
                })
                .on_key_down(move |e: &KeyDownEvent, _, cx| {
                    if matches!(e.keystroke.key.as_str(), "enter" | "space") {
                        cx.stop_propagation();
                        pick(cx);
                    }
                })
                .child(pass)
                .child(time),
            keyboard,
            window,
            cx,
        );
        div()
            .flex_none()
            .w(px(COLUMN))
            .child(
                tooltip(
                    SharedString::from(format!("trend-tip-{id}")),
                    Some(SharedString::from(c.tip.clone())),
                )
                .keyboard(keyboard)
                .child(button),
            )
            .into_any_element()
    }

    /// "Pass rate " in muted text, then the newest column's value.
    fn label(id: &'static str, name: &'static str, value: &str, cx: &App) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        div()
            .id(id)
            .debug_selector(move || id.into())
            .role(Role::Label)
            .aria_value(format!("{name}{value}"))
            .flex()
            .flex_none()
            .h(px(ROW))
            .items_center()
            .whitespace_nowrap()
            .child(div().text_color(Theme::color(t.muted)).child(name))
            .child(value.to_owned())
    }
}

impl Render for Trends {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.grace.read(cx).state.clone();
        // `History` drops the strip with the last run, so there is always a run here
        let wanted = Wanted::of(&state);
        if self.wanted.as_ref() != Some(&wanted) {
            self.wanted = Some(wanted);
            self.ask(cx);
        }
        let t = Theme::global(cx).tokens;
        let Some(shown) = self.shown.as_ref() else {
            // asked, and not answered yet: the strip keeps its place
            return div()
                .debug_selector(|| "trends-pending".into())
                .flex_none()
                .h(px(STRIP_HEIGHT))
                .into_any_element();
        };
        let caption = shown.caption.clone();
        let columns: Vec<AnyElement> = shown.strip.columns.iter().map(|c| self.column(c, window, cx)).collect();
        let none = shown.strip.columns.is_empty();
        let line = |top: f32| {
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(top))
                .h(px(1.))
                .bg(Theme::color(t.line))
        };
        let right = div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .text_color(Theme::color(t.muted))
            .child(div().h(px(ROW)))
            // a node only where there is a value to read
            .when(!shown.strip.max.is_empty(), |d| {
                d.child(
                    div()
                        .id("trends-time-max")
                        .debug_selector(|| "trends-time-max".into())
                        .role(Role::Label)
                        .aria_value(shown.strip.max.clone())
                        .h(px(ROW))
                        .child(shown.strip.max.clone()),
                )
            });
        div()
            .id("trends")
            .debug_selector(|| "trends".into())
            .role(Role::Group)
            .aria_label(format!("Trend of {caption}"))
            .flex()
            .flex_col()
            .flex_none()
            .h(px(STRIP_HEIGHT))
            .px(px(12.))
            .py(px(8.))
            .text_size(px(METRICS.text_xs.size))
            .text_color(Theme::color(t.ink))
            .child(
                div()
                    .flex_none()
                    .h(px(CAPTION))
                    .max_w_full()
                    .text_color(Theme::color(t.muted))
                    .child(cut("trends-caption", caption)),
            )
            .child(
                div()
                    .mt(px(4.))
                    .flex()
                    .gap(px(12.))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_none()
                            .gap(px(4.))
                            .w(px(LABELS))
                            .child(Self::label("trends-pass-label", "Pass rate ", &shown.strip.pass, cx))
                            .child(Self::label(
                                "trends-time-label",
                                "Time per test ",
                                &shown.strip.time,
                                cx,
                            )),
                    )
                    .when(none, |d| {
                        d.child(
                            div()
                                .id("trends-none")
                                .debug_selector(|| "trends-none".into())
                                .role(Role::Label)
                                .aria_value(NONE)
                                .flex()
                                .items_center()
                                .h(px(COLUMN_HEIGHT))
                                .text_color(Theme::color(t.muted))
                                .child(NONE),
                        )
                    })
                    .when(!none, |d| {
                        // the two baselines run under every column; the bars sit on them
                        d.child(
                            div()
                                .relative()
                                .flex()
                                .gap(px(4.))
                                .h(px(COLUMN_HEIGHT))
                                .child(line(ROW))
                                .child(line(COLUMN_HEIGHT))
                                .children(columns),
                        )
                        .child(right)
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests;
