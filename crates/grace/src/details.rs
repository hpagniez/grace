//! `Details.tsx`, its P2 part (WS-06): the selected test's name, status, place, time, tags, error and steps with
//! their logs, or a group's status and counts per status, or the empty state. The failure groups and their stepping
//! (#229), Open in VS Code, the flaky proof and the Examples table come with P3; the
//! file's steps before a run, breakpoints and the paused step with P4 (the guide's phases).
use gpui_component::scroll::ScrollableElement as _;
use std::collections::HashSet;

use gpui::accesskit::Role;
use gpui::{
    App, Context, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use grace_core::karate::tree::find_node;
use grace_core::model::events::Step;
use grace_core::model::node::{NodeKind, Status, TestNode};
use grace_core::model::status::derive_status;
use grace_core::rows::{leaves, node_name, status_label};
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, MarkStatus, icon, status_mark};
use crate::theme::{METRICS, MONO, Theme};
use crate::tooltip::{cut, tooltip};

pub const NOTHING_SELECTED: &str = "Select a test to see its details";
pub const STEPS_LATER: &str = "Steps are available when the feature finishes";
pub const ERROR: &str = "Error";
pub const STEPS: &str = "Steps";

/// `formatMillis`: under a second in whole ms, else seconds with two decimals.
pub fn format_millis(ms: f64) -> String {
    if ms < 1000. {
        format!("{} ms", ms.round())
    } else {
        format!("{:.2} s", ms / 1000.)
    }
}

/// The tint a status takes in its badge: not run and pending take a skipped step's grey.
fn badge_tint(s: Status, t: &crate::theme::Tokens) -> (u32, u32) {
    match s {
        Status::NotRun | Status::Pending => (t.skipped_tint, t.skipped),
        Status::Running => (t.running_tint, t.running),
        Status::Passed => (t.passed_tint, t.passed),
        Status::Failed => (t.failed_tint, t.failed),
        Status::Interrupted => (t.cancelled_tint, t.cancelled),
    }
}

/// A step's mark: a status Karate has not shown us (the fixtures hold passed, failed and skipped) takes the amber
/// mark of an interrupted one.
pub fn step_mark(status: &str) -> MarkStatus {
    match status {
        "passed" => MarkStatus::Passed,
        "failed" => MarkStatus::Failed,
        "skipped" => MarkStatus::Skipped,
        _ => MarkStatus::Interrupted,
    }
}

/// A group's tests per status, in the order their first test comes in the tree.
pub fn counts(node: &TestNode, state: &AppState) -> Vec<(Status, usize)> {
    let mut out: Vec<(Status, usize)> = Vec::new();
    for l in leaves(node) {
        let s = state.statuses.get(&l.key).copied().unwrap_or(Status::NotRun);
        match out.iter_mut().find(|(k, _)| *k == s) {
            Some((_, n)) => *n += 1,
            None => out.push((s, 1)),
        }
    }
    out
}

/// The place a test or group is shown at: its file and line, its file alone, or `/` for the project's folder.
pub fn location(node: &TestNode) -> String {
    match (node.kind, node.line) {
        (NodeKind::Scenario | NodeKind::Row, Some(line)) => format!("{}:{line}", node.file),
        _ if node.file.is_empty() => "/".into(),
        _ => node.file.clone(),
    }
}

/// The log button's name: what a press does, and to which step.
pub fn log_toggle_name(text: &str, open: bool) -> String {
    format!("{} log of {text}", if open { "Hide" } else { "Show" })
}

/// What Details shows for the state: nothing selected, a test, or a group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Shown {
    Nothing,
    Test(String),
    Group(String),
}

pub fn shown(state: &AppState) -> Shown {
    let node = state
        .selected
        .as_ref()
        .zip(state.tree.as_ref())
        .and_then(|(key, tree)| find_node(tree, key));
    match node {
        None => Shown::Nothing,
        Some(n) if matches!(n.kind, NodeKind::Scenario | NodeKind::Row) => Shown::Test(n.key.clone()),
        Some(n) => Shown::Group(n.key.clone()),
    }
}

pub struct Details {
    grace: Entity<Grace>,
    /// The steps whose log shows, by index, for the test shown; a new selection closes them, as Electron Grace's
    /// rows were made afresh.
    open_logs: HashSet<usize>,
    for_key: Option<String>,
    /// It showed that nothing is selected: drawn again only once something is.
    nothing: bool,
    _subscriptions: Vec<Subscription>,
}

impl Details {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&grace, |this, _, cx| {
            let state = this.grace.read(cx).state.clone();
            if state.selected != this.for_key {
                this.for_key = state.selected.clone();
                this.open_logs.clear();
            }
            // #320: with nothing selected the pane says so whatever else changes, so a run's results do not draw it
            let nothing = shown(&state) == Shown::Nothing;
            if !std::mem::replace(&mut this.nothing, nothing) || !nothing {
                cx.notify();
            }
        })];
        let state = grace.read(cx).state.clone();
        Self {
            grace,
            open_logs: HashSet::new(),
            for_key: state.selected.clone(),
            nothing: shown(&state) == Shown::Nothing,
            _subscriptions: subscriptions,
        }
    }

    pub fn log_open(&self, step: usize) -> bool {
        self.open_logs.contains(&step)
    }

    pub fn toggle_log(&mut self, step: usize, cx: &mut Context<Self>) {
        if !self.open_logs.remove(&step) {
            self.open_logs.insert(step);
        }
        cx.notify();
    }

    fn heading(name: String) -> impl IntoElement {
        div()
            .id("details-heading")
            .role(Role::Heading)
            .aria_level(2)
            .aria_label(SharedString::from(name.clone()))
            .text_size(px(METRICS.text_lg.size))
            .font_weight(FontWeight::SEMIBOLD)
            .child(name)
    }

    /// The status as a tinted badge: its mark and its word, which names it, so the mark is decorative (R7.1).
    fn badge(status: Status, cx: &App) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let (bg, fg) = badge_tint(status, t);
        let word = status_label(status);
        div().flex().child(
            div()
                .id("details-status")
                .debug_selector(|| "details-status".into())
                .role(Role::Label)
                .aria_value(SharedString::from(word))
                .flex()
                .items_center()
                .gap_1()
                .h(px(22.))
                .px_2()
                .rounded_full()
                .bg(Theme::color(bg))
                .text_color(Theme::color(fg))
                .text_size(px(METRICS.text_xs.size))
                .font_weight(FontWeight::MEDIUM)
                .child(status_mark(
                    "details-status-mark",
                    crate::tree::mark(status),
                    14.,
                    true,
                    cx.reduce_motion(),
                    cx,
                ))
                .child(word),
        )
    }

    fn place(text: String, cx: &App) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        div()
            .id("details-location")
            .debug_selector(|| "details-location".into())
            .role(Role::Label)
            .aria_value(SharedString::from(text.clone()))
            .min_w_0()
            .max_w_full()
            .font_family(MONO)
            .text_color(Theme::color(t.muted))
            .child(cut("details-location-cut", text))
    }

    fn test(&self, this: &Entity<Self>, node: &TestNode, state: &AppState, cx: &App) -> Vec<gpui::AnyElement> {
        let t = Theme::global(cx).tokens;
        let status = state.statuses.get(&node.key).copied().unwrap_or(Status::NotRun);
        let result = state.results.get(&node.key);
        let duration = result.and_then(|r| r.duration_millis).map(format_millis);
        let meta = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_3()
            .gap_y_1()
            .text_size(px(METRICS.text_xs.size))
            .text_color(Theme::color(t.muted))
            .child(Self::place(location(node), cx))
            .children(duration.map(|d| {
                div()
                    .id("details-duration")
                    .debug_selector(|| "details-duration".into())
                    .role(Role::Label)
                    .aria_value(SharedString::from(d.clone()))
                    .font_family(MONO)
                    .child(d)
            }))
            .children(node.tags.iter().enumerate().map(|(i, tag)| {
                crate::ui::tag_chip(tag.clone(), cx).debug_selector(move || format!("details-tag-{i}"))
            }));
        let error = result
            .and_then(|r| r.error.clone())
            .filter(|_| status == Status::Failed)
            .map(|e| {
                div()
                    .id("details-error-box")
                    .role(Role::Group)
                    .aria_label(ERROR)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded_lg()
                    .bg(Theme::color(t.failed_tint))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .text_size(px(METRICS.text_sm.size))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(Theme::color(t.failed))
                            .child(icon(Icon::CircleAlert, 14., Theme::color(t.failed)))
                            .child(ERROR),
                    )
                    .child(
                        div()
                            .id("details-error")
                            .debug_selector(|| "details-error".into())
                            .role(Role::Label)
                            .aria_value(SharedString::from(e.clone()))
                            .font_family(MONO)
                            .text_size(px(METRICS.text_xs.size))
                            .text_color(Theme::color(t.ink))
                            .child(e),
                    )
            });
        let steps = match state.steps.get(&node.key) {
            Some(steps) => self.steps(this, steps, cx).into_any_element(),
            None => {
                // the spinner turns only while the run lasts, and not with reduced motion (ADR 0030)
                let spins = state.run.active && status != Status::NotRun;
                let spinner = spins.then(|| {
                    let mark = icon(Icon::LoaderCircle, 16., Theme::color(t.muted));
                    if cx.reduce_motion() {
                        mark.into_any_element()
                    } else {
                        crate::spin::turning(mark).into_any_element()
                    }
                });
                div()
                    .id("steps-later")
                    .debug_selector(|| "steps-later".into())
                    .role(Role::Label)
                    .aria_value(STEPS_LATER)
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(METRICS.text_sm.size))
                    .text_color(Theme::color(t.muted))
                    .children(spinner.map(|s| div().debug_selector(|| "steps-spinner".into()).flex().child(s)))
                    .child(STEPS_LATER)
                    .into_any_element()
            }
        };
        vec![
            Self::heading(node_name(node)).into_any_element(),
            Self::badge(status, cx).into_any_element(),
            meta.into_any_element(),
        ]
        .into_iter()
        .chain(error.map(|e| e.into_any_element()))
        .chain(std::iter::once(steps))
        .collect()
    }

    fn steps(&self, this: &Entity<Self>, steps: &[Step], cx: &App) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        div()
            .id("steps")
            .role(Role::List)
            .aria_label(STEPS)
            .flex()
            .flex_col()
            .children(steps.iter().enumerate().map(|(i, step)| {
                let open = self.log_open(i);
                let whole = format!("{} {}", step.keyword, step.text);
                let mark = step_mark(&step.status);
                let comments = step.comments.clone().unwrap_or_default();
                let toggle = step.log.as_ref().map(|_| {
                    let this = this.clone();
                    let name = log_toggle_name(&step.text, open);
                    crate::a11y::aria(
                        crate::elements::icon_text_button(
                            ("step-toggle", i),
                            if open { Icon::ChevronDown } else { Icon::ChevronRight },
                            "Log",
                            ButtonKind::Ghost,
                            ButtonSize::Small,
                        )
                        .accessibility_label(SharedString::from(name))
                        .debug_selector(move || format!("step-toggle-{i}")),
                        |a| a.aria_expanded(open),
                    )
                    .on_click(move |_, _, cx| this.update(cx, |d, cx| d.toggle_log(i, cx)))
                });
                let log = step.log.clone().filter(|_| open).map(|log| {
                    div()
                        .id(("step-log", i))
                        .debug_selector(move || format!("step-log-{i}"))
                        .role(Role::Label)
                        .aria_value(SharedString::from(log.clone()))
                        .mt_1()
                        .ml_6()
                        .p_2()
                        .rounded(px(METRICS.radius))
                        .border_1()
                        .border_color(Theme::color(t.line))
                        .bg(Theme::color(t.sunken))
                        .font_family(MONO)
                        .text_size(px(METRICS.text_xs.size))
                        .child(log)
                });
                div()
                    .id(("step", i))
                    .role(Role::ListItem)
                    .aria_label(SharedString::from(whole.clone()))
                    .py_1()
                    .when(i > 0, |d| d.border_t_1().border_color(Theme::color(t.line)))
                    .when(!comments.is_empty(), |d| {
                        d.child(
                            div()
                                .debug_selector(move || format!("step-comments-{i}"))
                                .ml_6()
                                .font_family(MONO)
                                .text_size(px(METRICS.text_xs.size))
                                .text_color(Theme::color(t.muted))
                                .children(comments.into_iter().map(|c| div().child(c))),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .min_h(px(24.))
                            .text_size(px(METRICS.text_sm.size))
                            .child(
                                tooltip(("tip-step-mark", i), Some(SharedString::from(mark.label())))
                                    .child(status_mark(("step-mark", i), mark, 14., false, cx.reduce_motion(), cx)),
                            )
                            .when(step.background, |d| {
                                d.child(
                                    crate::ui::tag_chip("Background", cx)
                                        .debug_selector(move || format!("step-background-{i}")),
                                )
                            })
                            .child(
                                div()
                                    .debug_selector(move || format!("step-text-{i}"))
                                    .flex_1()
                                    .min_w_0()
                                    .font_family(MONO)
                                    .child(cut(("step-text", i), whole).side(crate::tooltip::Side::Over)),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .font_family(MONO)
                                    .text_size(px(METRICS.text_xs.size))
                                    .text_color(Theme::color(t.muted))
                                    .child(format_millis(step.millis)),
                            )
                            .children(toggle),
                    )
                    .children(log)
            }))
    }

    fn group(&self, node: &TestNode, state: &AppState, cx: &App) -> Vec<gpui::AnyElement> {
        let status = derive_status(node, &state.statuses);
        let total = leaves(node).len();
        let counts = div()
            .id("details-counts")
            .debug_selector(|| "details-counts".into())
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_4()
            .gap_y_1()
            .text_size(px(METRICS.text_sm.size))
            .child(
                div()
                    .id("details-total")
                    .role(Role::Label)
                    .aria_value(SharedString::from(format!("{total} scenarios")))
                    .child(format!("{total} scenarios")),
            )
            .children(counts(node, state).into_iter().map(|(s, n)| {
                let words = format!("{n} {}", status_label(s));
                div()
                    .id(SharedString::from(format!("count-{}", status_label(s))))
                    .role(Role::Label)
                    .aria_value(SharedString::from(words))
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(status_mark(
                        SharedString::from(format!("count-mark-{}", status_label(s))),
                        crate::tree::mark(s),
                        14.,
                        true,
                        cx.reduce_motion(),
                        cx,
                    ))
                    .child(n.to_string())
                    .child(status_label(s))
            }));
        vec![
            Self::heading(node.name.clone()).into_any_element(),
            Self::badge(status, cx).into_any_element(),
            div()
                .flex()
                .text_size(px(METRICS.text_xs.size))
                .child(Self::place(location(node), cx))
                .into_any_element(),
            counts.into_any_element(),
        ]
    }
}

impl Render for Details {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let this = cx.entity();
        let grace = self.grace.clone();
        // borrowed, never copied: the state holds every test's results, steps and the whole tree
        let state = &grace.read(cx).state;
        let node = |key: &str| state.tree.as_ref().and_then(|tr| find_node(tr, key));
        let content: Vec<gpui::AnyElement> = match shown(state) {
            Shown::Nothing => vec![
                crate::ui::empty_state("details-empty", Icon::MousePointerClick, NOTHING_SELECTED, None, cx)
                    .into_any_element(),
            ],
            Shown::Test(key) => node(&key).map_or_else(Vec::new, |n| self.test(&this, n, state, cx)),
            Shown::Group(key) => node(&key).map_or_else(Vec::new, |n| self.group(n, state, cx)),
        };
        div()
            .id("details")
            .debug_selector(|| "details".into())
            .role(Role::Group)
            .aria_label("Details")
            .size_full()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .gap_3()
            .p_4()
            .border_l_1()
            .border_color(Theme::color(t.line))
            .overflow_y_scrollbar()
            .children(content)
    }
}

#[cfg(test)]
mod tests;
