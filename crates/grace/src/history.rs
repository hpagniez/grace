//! `History.tsx`: the History tab, a row per listed run of the open project, newest first, and the Clear history
//! button and its dialog (docs/specs/2026-10-25-run-history.md R3 to R8). What a row says and which buttons take a press
//! are `grace_core::history_list`'s; this draws them.
//!
//! The runs arrive with the project and after each run (the runner sends the list); the rows are worked out when the list
//! changes and read their clock once, as Electron Grace's rows did, so an idle History tab costs nothing. At most 20
//! runs are listed, so the rows are plain elements, not a virtual list. Export and the trends strip come with slices 6
//! and 7 (P3 plan).
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::WindowExt as _;
use gpui_component::dialog::DialogFooter;
use gpui_component::scroll::ScrollableElement as _;
use grace_core::history_list::{
    Actions, EMPTY, ResultMark, RowText, Tag, actions, can_clear, clear_prompt, locked, row_text,
};
use grace_core::model::history::HistoryEntry;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{
    ButtonKind, ButtonSize, MarkStatus, Off as _, button, icon, icon_button, icon_text_button, status_mark,
};
use crate::theme::{METRICS, MONO, SANS, Theme};
use crate::tooltip::{Side, tooltip};

/// The list's accessible name.
pub const LIST: &str = "Run history";
pub const CLEAR: &str = "Clear history";
pub const DELETE: &str = "Delete";
pub const CANCEL: &str = "Cancel";
/// px, a row (`h-7`).
pub const ROW_HEIGHT: f32 = 28.;
/// The dialog's width: Tailwind's `w-[28rem]`.
pub const DIALOG_WIDTH: f32 = 448.;

/// The columns' widths in px; "Tags" takes what the others leave and is cut when it is short.
const WHEN: f32 = 104.;
const ENV: f32 = 56.;
const THREADS: f32 = 56.;
const RESULT: f32 = 208.;
const DURATION: f32 = 64.;
/// "What ran" (`max-w-56` in Electron Grace, which grew to its text): cut here, its tooltip says the path. It gives up
/// width when the row is short of it, after Tags.
const WHAT: f32 = 136.;
/// What a row needs beside Tags and its buttons: its padding (`px-3`), its seven gaps (`gap-3`) and the fixed columns.
const BESIDE: f32 = 24. + 7. * 12. + WHEN + ENV + THREADS + WHAT + RESULT + DURATION;
/// px, between the buttons (`gap-1`).
const BUTTON_GAP: f32 = 4.;
/// px, a labelled small button around its text: `px-2`, the 14 px icon and the 6 px before the text.
const BUTTON_FRAME: f32 = 8. + 14. + 6. + 8.;
/// What a press of a row's button asks of the app, with the run's id.
type Press = fn(&mut Grace, String, &mut Context<Grace>);
/// A row's buttons, in their order: what the tests call each, its icon, its words and what it does.
const BUTTONS: [(&str, Icon, &str, Press); 3] = [
    ("report", Icon::FileText, "Report", Grace::open_run_report),
    ("rerun", Icon::Play, "Re-run", Grace::rerun),
    ("rerun-failed", Icon::RotateCcw, "Re-run failed", Grace::rerun_failed_of),
];

fn clock() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0., |d| d.as_millis() as f64)
}

fn mark(m: ResultMark) -> MarkStatus {
    match m {
        ResultMark::Passed => MarkStatus::Passed,
        ResultMark::Failed => MarkStatus::Failed,
        ResultMark::Cancelled => MarkStatus::Cancelled,
    }
}

/// The History tab.
pub struct History {
    grace: Entity<Grace>,
    /// What the rows below were worked out from.
    built: Vec<HistoryEntry>,
    rows: Rc<Vec<RowText>>,
    /// How many times the rows were worked out: not on a redraw (idle costs nothing).
    pub builds: u64,
    /// "Now" for a row's labels, read once as the row is made.
    pub clock: Rc<dyn Fn() -> f64>,
}

impl History {
    pub fn new(grace: Entity<Grace>) -> Self {
        Self {
            grace,
            built: Vec::new(),
            rows: Rc::default(),
            builds: 0,
            clock: Rc::new(clock),
        }
    }

    pub fn rows(&self) -> Rc<Vec<RowText>> {
        Rc::clone(&self.rows)
    }

    /// The rows again, when the list changed.
    fn build(&mut self, history: &[HistoryEntry]) {
        if self.built == history {
            return;
        }
        self.built = history.to_vec();
        let now = (self.clock)();
        self.rows = Rc::new(history.iter().map(|e| row_text(e, now)).collect());
        self.builds += 1;
    }

    fn header(&self, actions: f32, cx: &App) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let head = |text: &'static str, w: Option<f32>, right: bool| {
            div()
                .when_some(w, |d, w| d.w(px(w)).flex_none())
                .when(w.is_none(), |d| d.flex_1().min_w_0())
                .when(right, |d| d.text_right())
                .child(text)
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_3()
            .h(px(ROW_HEIGHT))
            .px_3()
            .bg(Theme::color(t.surface))
            .border_b_1()
            .border_color(Theme::color(t.line))
            .text_size(px(METRICS.text_xs.size))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(Theme::color(t.muted))
            .child(head("When", Some(WHEN), false))
            .child(head("Env", Some(ENV), false))
            .child(head("Threads", Some(THREADS), true))
            .child(head("Tags", None, false))
            .child(head("What ran", Some(WHAT), false).flex_shrink(1.).min_w_0().truncate())
            .child(head("Result", Some(RESULT), false))
            .child(head("Duration", Some(DURATION), true))
            .child(div().w(px(actions)).flex_none())
    }

    fn row(&self, i: usize, e: &HistoryEntry, locked: bool, look: Look, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let text = &self.rows[i];
        let a = actions(e, locked);
        let id = e.id.clone();
        // widgets are keyed by the run, not its place: the focus stays with a run when a new one pushes it down
        let rid = id.clone();
        let cell = |what: &'static str, value: String| {
            div()
                .id(SharedString::from(format!("history-{what}-{rid}")))
                .debug_selector(move || format!("history-{what}:{}", i + 1))
                .role(Role::Label)
                .aria_value(value.clone())
                .child(value)
        };
        let tags: AnyElement = if text.tags.is_empty() {
            cell("tags", "—".into()).into_any_element()
        } else {
            div()
                .debug_selector(move || format!("history-tags:{}", i + 1))
                .flex()
                .gap_1()
                .children(text.tags.iter().enumerate().map(|(k, tag)| tag_chip(&rid, k, tag, cx)))
                .into_any_element()
        };
        let what = div().w(px(WHAT)).min_w_0().overflow_hidden().child(
            tooltip(
                SharedString::from(format!("history-what-tip-{rid}")),
                text.path.clone().map(SharedString::from),
            )
            .side(Side::Over)
            .child(
                div()
                    .id(SharedString::from(format!("history-what-text-{rid}")))
                    .debug_selector(move || format!("history-what:{}", i + 1))
                    .role(Role::Label)
                    .aria_value(text.what.clone())
                    .truncate()
                    .child(text.what.clone()),
            ),
        );
        let result = div()
            .flex_none()
            .w(px(RESULT))
            .flex()
            .items_center()
            .gap(px(6.))
            .child(status_mark(
                SharedString::from(format!("history-mark-{rid}")),
                mark(text.mark),
                14.,
                true,
                cx.reduce_motion(),
                cx,
            ))
            .child(cell("result", text.result.clone()));
        div()
            .id(SharedString::from(format!("history-row-{rid}")))
            .debug_selector(move || format!("history-row:{}", i + 1))
            .role(Role::ListItem)
            .aria_label(format!("{}, {}, {}", text.when, text.what, text.result))
            .flex()
            .flex_none()
            .items_center()
            .gap_3()
            .h(px(ROW_HEIGHT))
            .px_3()
            .border_b_1()
            .border_color(Theme::color(t.line))
            .text_size(px(METRICS.text_xs.size))
            .text_color(Theme::color(t.ink))
            .child(cell("when", text.when.clone()).flex_none().w(px(WHEN)))
            .child(cell("env", text.env.clone()).flex_none().w(px(ENV)))
            .child(
                cell("threads", text.threads.clone())
                    .flex_none()
                    .w(px(THREADS))
                    .text_right(),
            )
            .child(div().flex_1().min_w_0().overflow_hidden().child(tags))
            .child(what)
            .child(result)
            .child(
                cell("duration", text.duration.clone())
                    .flex_none()
                    .w(px(DURATION))
                    .text_right()
                    .font_family(MONO),
            )
            .child(self.buttons(i, id, a, look))
            .into_any_element()
    }

    fn buttons(&self, i: usize, id: String, a: Actions, look: Look) -> impl IntoElement {
        let on = [a.report, a.rerun, a.rerun_failed];
        div()
            .debug_selector(move || format!("history-actions:{}", i + 1))
            .flex()
            .flex_none()
            .w(px(look.width))
            .gap(px(BUTTON_GAP))
            .children(BUTTONS.iter().zip(on).map(|(&(what, icon, label, press), on)| {
                let (g, run) = (self.grace.clone(), id.clone());
                let click = move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
                    g.update(cx, |g, cx| press(g, run.clone(), cx));
                };
                let bid = SharedString::from(format!("history-{what}-{id}"));
                let button = if look.icons {
                    tooltip(
                        SharedString::from(format!("history-{what}-tip-{id}")),
                        Some(label.into()),
                    )
                    .side(Side::Left)
                    .focusable(move |f| {
                        icon_button(bid, icon, 14., label, true)
                            .track_focus(f)
                            .off(!on)
                            .on_click(click)
                    })
                    .into_any_element()
                } else {
                    icon_text_button(bid, icon, label, ButtonKind::Ghost, ButtonSize::Small)
                        .off(!on)
                        .on_click(click)
                        .into_any_element()
                };
                // each button in a frame that names it, for the tests' clicks
                div()
                    .debug_selector(move || format!("history-{what}:{}", i + 1))
                    .child(button)
            }))
    }
}

/// How a row's buttons are drawn: with their words while the row fits the window, their icons only below that, so
/// every button stays inside the window down to its least width (a press clicks a button's centre).
#[derive(Clone, Copy)]
struct Look {
    /// The Actions column's width, the header's included: what its buttons take.
    width: f32,
    icons: bool,
}

impl Look {
    fn of(window: &Window) -> Self {
        let mut style = window.text_style();
        style.font_family = SANS.into();
        let words: f32 = BUTTONS
            .iter()
            .map(|(_, _, label, _)| {
                let line = window.text_system().shape_line(
                    SharedString::new_static(label),
                    px(METRICS.text_xs.size),
                    &[style.to_run(label.len())],
                    None,
                );
                f32::from(line.width).ceil() + BUTTON_FRAME
            })
            .sum();
        let gaps = BUTTON_GAP * (BUTTONS.len() - 1) as f32;
        // the tab spans the sheet, inside its inset and side on both edges
        let room = f32::from(window.viewport_size().width) - 2. * (METRICS.sheet_inset + METRICS.sheet_side);
        if BESIDE + words + gaps <= room {
            Self {
                width: words + gaps,
                icons: false,
            }
        } else {
            Self {
                width: METRICS.control_height_small * BUTTONS.len() as f32 + gaps,
                icons: true,
            }
        }
    }
}

/// A tag of the run's filter, drawn by its shape: included with a check on the accent, excluded with a minus and a
/// strike on the surface (`chipIncluded`, `chipExcluded`).
fn tag_chip(rid: &str, k: usize, tag: &Tag, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    let (border, bg, fg, glyph) = if tag.excluded {
        (t.control, t.surface, t.ink, Icon::Minus)
    } else {
        (t.accent, t.accent, t.on_accent, Icon::Check)
    };
    div()
        .id(SharedString::from(format!("history-tag-{rid}-{k}")))
        .role(Role::Label)
        .aria_value(SharedString::from(tag.text.clone()))
        .flex()
        .flex_none()
        .items_center()
        .gap_1()
        .h(px(24.))
        .px(px(7.))
        .rounded(px(4.))
        .border_1()
        .border_color(Theme::color(border))
        .bg(Theme::color(bg))
        .text_size(px(METRICS.text_2xs.size))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(Theme::color(fg))
        .whitespace_nowrap()
        .when(tag.excluded, |d| d.line_through())
        .child(icon(glyph, 14., Theme::color(fg)))
        .child(tag.text.clone())
        .into_any_element()
}

impl Render for History {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.grace.read(cx).state.clone();
        self.build(&state.history);
        let locked = locked(&state);
        let look = Look::of(window);
        let body: AnyElement = if state.history.is_empty() {
            div()
                .id("history-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(crate::ui::empty_state("history-empty", Icon::History, EMPTY, None, cx))
                .into_any_element()
        } else {
            let rows: Vec<_> = state
                .history
                .iter()
                .enumerate()
                .map(|(i, e)| self.row(i, e, locked, look, cx))
                .collect();
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(self.header(look.width, cx))
                .child(
                    div()
                        .id("history-list")
                        .debug_selector(|| "history-list".into())
                        .role(Role::List)
                        .aria_label(LIST)
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scrollbar()
                        .children(rows),
                )
                .into_any_element()
        };
        div()
            .id("history")
            .debug_selector(|| "history".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(body)
    }
}

/// The Clear history button of the tab's header.
pub fn clear_button(grace: &Entity<Grace>, cx: &App) -> impl IntoElement {
    let state = &grace.read(cx).state;
    let g = grace.clone();
    div().debug_selector(|| "clear-history".into()).child(
        button("clear-history", CLEAR, ButtonKind::Plain, ButtonSize::Small)
            .off(!can_clear(state))
            .on_click(move |_, window, cx| {
                open_clear(&g, window, cx);
            }),
    )
}

/// Opens the dialog that confirms Clear history with the count of the runs of this moment, so main's empty list
/// arriving while the dialog fades out does not change its words. Delete first, then Cancel, which takes the focus;
/// Escape closes it (RH-10, PU-32).
/// Returns Cancel's focus handle.
pub fn open_clear(grace: &Entity<Grace>, window: &mut Window, cx: &mut App) -> FocusHandle {
    let count = grace.read(cx).state.history.len();
    let cancel: FocusHandle = cx.focus_handle();
    let kept = cancel.clone();
    let g = grace.clone();
    let first = std::rc::Rc::new(std::cell::Cell::new(true));
    window.open_dialog(cx, move |dialog, window, cx| {
        let g = g.clone();
        let cancel = cancel.clone();
        // Kit gives the dialog its own focus once it is open, after this call: Cancel takes it from there, once
        if first.replace(false) {
            let cancel = cancel.clone();
            window.defer(cx, move |window, cx| window.focus(&cancel, cx));
        }
        dialog
            .title(
                div()
                    .flex()
                    .text_size(px(METRICS.text_lg.size))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(
                        div()
                            .id("clear-history-title")
                            .debug_selector(|| "clear-history-title".into())
                            // the dialog's name, read out as the question it asks
                            .role(Role::Label)
                            .aria_value(clear_prompt(count))
                            .flex_none()
                            .child(clear_prompt(count)),
                    ),
            )
            .w(px(DIALOG_WIDTH))
            // a native dialog has no close button and a click outside does not close it: Escape and Cancel do
            .close_button(false)
            .overlay_closable(false)
            .footer(
                DialogFooter::new()
                    .child(div().debug_selector(|| "clear-history-confirm".into()).child(
                        button("clear-history-confirm", DELETE, ButtonKind::Danger, ButtonSize::Dialog).on_click(
                            move |_, window, cx| {
                                g.update(cx, |g, cx| g.clear_history(cx));
                                window.close_dialog(cx);
                            },
                        ),
                    ))
                    .child(
                        div().debug_selector(|| "clear-history-cancel".into()).child(
                            button("clear-history-cancel", CANCEL, ButtonKind::Plain, ButtonSize::Dialog)
                                .track_focus(&cancel)
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        ),
                    ),
            )
    });
    kept
}

#[cfg(test)]
mod tests;
