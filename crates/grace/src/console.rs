//! `Console.tsx` and `Banner.tsx`: Karate's output under the banner, its find bar and its error stepping (console
//! banner spec, console search spec). The search lives here, in the `Console` the bottom panel keeps for as long as
//! the project view shows, so its text and current match outlive a tab switch and a run (R3.12).
//!
//! Electron Grace drew every line and painted the matches over them with the CSS highlight API, so no row redrew
//! (R4.1). GPUI draws only the lines in view (`gpui::list`, which measures wrapped lines), so painting a match is
//! drawing its line's few runs again: the cost is the view's, not the console's. The list follows the bottom while it
//! is at the bottom, as the scroller's `pinned` did (`FollowMode::Tail`).
use gpui_component::scroll::ScrollableElement as _;
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;
use std::time::Duration;

use gpui::accesskit::Role;
use gpui::{
    App, AppContext as _, Bounds, Context, Entity, FocusHandle, FollowMode, HighlightStyle, InteractiveElement as _,
    IntoElement, KeyBinding, ListAlignment, ListState, ParentElement as _, Pixels, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, StyledText, Subscription, Window, canvas, div, list,
    prelude::FluentBuilder as _, px,
};
use gpui_component::input::{InputEvent, InputState};
use grace_core::banner::{BannerState, banner_state, tests_text};
use grace_core::console_find::{Match, error_lines, find_matches, is_error_line, start_index, step_index};
use grace_core::state::{AppState, Panel};

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::icon;
use crate::theme::{METRICS, Theme};
use crate::tooltip::tooltip;

pub const FIND: &str = "Find";
pub const FIND_IN_CONSOLE: &str = "Find in console";
pub const PREVIOUS_MATCH: &str = "Previous match";
pub const NEXT_MATCH: &str = "Next match";
pub const CLOSE_FIND: &str = "Close find";
pub const ERRORS: &str = "Errors";
pub const PREVIOUS_ERROR: &str = "Previous error";
pub const NEXT_ERROR: &str = "Next error";
pub const NO_RESULTS: &str = "No results";
/// The console's accessible name: the lines are its children, in view.
pub const CONSOLE: &str = "Console";

/// The versions the banner names, from `package.json` and `scripts/vendor.json` (`build.rs`).
pub const VERSIONS: (&str, &str, &str) = (
    env!("GRACE_VERSION"),
    env!("GRACE_KARATE_VERSION"),
    env!("GRACE_JAVA_VERSION"),
);

/// The key context of the project view: Ctrl+F and F8 work anywhere in it (R1, R3.1), not in a dialog, whose focus
/// is outside it, and not on the Settings page, which replaces it.
pub const CONTEXT: &str = "ProjectView";

gpui::actions!(console, [FindInConsole, NextError, PreviousError]);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-f", FindInConsole, Some(CONTEXT)),
        KeyBinding::new("f8", NextError, Some(CONTEXT)),
        KeyBinding::new("shift-f8", PreviousError, Some(CONTEXT)),
    ]);
}

/// A place in the console by line number, the dropped lines plus its index, so it keeps its line when older lines
/// drop at the cap (R5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Place {
    line: u64,
    start: usize,
}

/// What the matches and errors were found in: the lines, the text, and whether the bar is open and the tab shows.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Found {
    dropped: u64,
    len: usize,
    query: String,
    open: bool,
    shown: bool,
}

pub struct Console {
    grace: Entity<Grace>,
    /// The banner, then one item per line.
    pub list: ListState,
    pub field: Entity<InputState>,
    open: bool,
    held: Option<Place>,
    /// A line number.
    held_error: Option<u64>,
    matches: Vec<Match>,
    errors: Vec<usize>,
    found: Option<Found>,
    /// The lines the list was last told of: the dropped count and the length.
    told: (u64, usize),
    /// What had the focus before the bar opened, given back when it closes (R3.6).
    back: Option<FocusHandle>,
    /// Where the lines show, in the window, after each layout: a match already in view is not scrolled to (R3.3).
    view: Rc<Cell<Bounds<Pixels>>>,
    _subscriptions: Vec<Subscription>,
}

impl Console {
    pub fn new(grace: Entity<Grace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = cx.new(|cx| InputState::new(window, cx).placeholder(FIND));
        let list = ListState::new(1, ListAlignment::Top, px(200.));
        list.set_follow_mode(FollowMode::Tail);
        let subscriptions = vec![
            cx.observe(&grace, |this, _, cx| this.sync(cx)),
            cx.subscribe_in(&field, window, Self::on_field),
        ];
        let mut me = Self {
            grace,
            list,
            field,
            open: false,
            held: None,
            held_error: None,
            matches: Vec::new(),
            errors: Vec::new(),
            found: None,
            told: (0, 0),
            back: None,
            view: Rc::default(),
            _subscriptions: subscriptions,
        };
        me.sync(cx);
        me
    }

    fn state(&self, cx: &App) -> Rc<AppState> {
        self.grace.read(cx).state.clone()
    }

    pub fn query(&self, cx: &App) -> String {
        self.field.read(cx).value().to_string()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn matches(&self) -> &[Match] {
        &self.matches
    }

    pub fn errors(&self) -> &[usize] {
        &self.errors
    }

    /// The current match's index; None for none.
    pub fn current(&self, cx: &App) -> Option<usize> {
        let p = self.held?;
        let at = p.line.checked_sub(self.state(cx).console_dropped)? as usize;
        self.matches.iter().position(|m| m.line == at && m.start == p.start)
    }

    pub fn current_error(&self, cx: &App) -> Option<usize> {
        let line = self.held_error?.checked_sub(self.state(cx).console_dropped)? as usize;
        self.errors.iter().position(|e| *e == line)
    }

    /// Tells the list of new and dropped lines, finds again what changed, and keeps a match current (R3.10).
    fn sync(&mut self, cx: &mut Context<Self>) {
        let state = self.state(cx);
        let (dropped, len) = (state.console_dropped, state.console.len());
        let (was_dropped, was_len) = self.told;
        let gone = dropped.wrapping_sub(was_dropped) as usize;
        // a closed project clears the console and its count, which may have been 0 already: fewer lines, none dropped
        if dropped < was_dropped || gone > was_len || len < was_len - gone {
            self.list.reset(1 + len); // a new run or a closed project cleared the console
        } else if (dropped, len) != self.told {
            // drops the oldest lines, then adds the new ones at the end; item 0 is the banner
            self.list.splice(1..1 + gone, 0);
            let kept = was_len - gone;
            self.list.splice(1 + kept..1 + kept, len - kept);
        }
        self.told = (dropped, len);
        self.find(cx);
        cx.notify();
    }

    fn find(&mut self, cx: &mut Context<Self>) {
        let state = self.state(cx);
        let found = Found {
            dropped: state.console_dropped,
            len: state.console.len(),
            query: self.query(cx),
            open: self.open,
            shown: state.panel == Panel::Console,
        };
        if self.found.as_ref() == Some(&found) {
            return;
        }
        // nothing is searched while the bar is closed, nor while another tab shows: nothing would show them (R3.11)
        self.matches = if found.open && found.shown {
            find_matches(state.console.iter(), &found.query)
        } else {
            Vec::new()
        };
        // only while the Console tab shows: the count is on its tab row (R2.1)
        self.errors = if found.shown {
            error_lines(state.console.iter())
        } else {
            Vec::new()
        };
        self.found = Some(found);
        if !self.matches.is_empty() && self.current(cx).is_none() {
            let lines: Vec<usize> = self.matches.iter().map(|m| m.line).collect();
            if let Some(i) = start_index(&lines, self.top(), true) {
                self.hold(i, cx);
            }
        }
    }

    fn hold(&mut self, i: usize, cx: &App) {
        let m = self.matches[i];
        self.held = Some(Place {
            line: m.line as u64 + self.state(cx).console_dropped,
            start: m.start,
        });
    }

    /// The first line in view.
    fn top(&self) -> usize {
        self.list.logical_scroll_top().item_ix.saturating_sub(1)
    }

    /// Brings line `i` into view; with `only_if_hidden`, only when it is not in view already. Either way the list
    /// stops following the bottom, as a scroll away from it does (R3.13).
    fn show(&self, i: usize, only_if_hidden: bool) {
        let item = i + 1;
        let view = self.view.get();
        let in_view = self
            .list
            .bounds_for_item(item)
            .is_some_and(|b| b.top() >= view.top() && b.bottom() <= view.bottom());
        if only_if_hidden && in_view {
            return;
        }
        self.list.pause_following_tail();
        self.list.scroll_to_reveal_item(item);
    }

    fn on_field(&mut self, _: &Entity<InputState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                // GPUI Kit's box tells of a change on Enter too: only a new text picks a match again
                if self.found.as_ref().is_some_and(|f| f.query == self.query(cx)) {
                    return;
                }
                self.held = None;
                self.find(cx);
                if let Some(i) = self.current(cx) {
                    self.show(self.matches[i].line, true);
                }
                cx.notify();
            }
            InputEvent::PressEnter { shift, .. } => self.step(!shift, cx),
            _ => {}
        }
    }

    pub fn step(&mut self, down: bool, cx: &mut Context<Self>) {
        if self.matches.is_empty() {
            return;
        }
        let next = match self.current(cx) {
            Some(i) => step_index(self.matches.len(), i, down),
            None => {
                let lines: Vec<usize> = self.matches.iter().map(|m| m.line).collect();
                start_index(&lines, self.top(), down).unwrap_or(0)
            }
        };
        self.hold(next, cx);
        self.show(self.matches[next].line, false);
        cx.notify();
    }

    pub fn step_error(&mut self, down: bool, cx: &mut Context<Self>) {
        if self.errors.is_empty() {
            return;
        }
        let next = match self.current_error(cx) {
            Some(i) => step_index(self.errors.len(), i, down),
            None => start_index(&self.errors, self.top(), down).unwrap_or(0),
        };
        self.held_error = Some(self.errors[next] as u64 + self.state(cx).console_dropped);
        self.show(self.errors[next], false);
        cx.notify();
    }

    /// Ctrl+F and the Find button: the Console tab, the bar, its field focused with its text selected (R3.1).
    pub fn open_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            self.back = window.focused(cx);
        }
        self.grace.update(cx, |g, cx| g.show_panel(Panel::Console, cx));
        self.open = true;
        self.find(cx);
        self.field.update(cx, |f, cx| {
            f.focus(window, cx);
            f.select_all(window, cx);
        });
        cx.notify();
    }

    /// Escape and the close button: the text stays, the focus goes back (R3.6).
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.held = None; // reopened, the bar picks from the view again (R3.10)
        self.find(cx);
        if let Some(back) = self.back.take() {
            window.focus(&back, cx);
        }
        cx.notify();
    }

    /// The match count, R3.4: "2 of 5", "No results", nothing for no text.
    pub fn count(&self, cx: &App) -> String {
        if self.query(cx).is_empty() {
            return String::new();
        }
        match (self.matches.len(), self.current(cx)) {
            (0, _) => NO_RESULTS.into(),
            (n, Some(i)) => format!("{} of {n}", i + 1),
            (n, None) => format!("0 of {n}"),
        }
    }

    /// The errors group's count, R2.1: "3 errors", "2 of 3 errors", "1 error".
    pub fn error_count(&self, cx: &App) -> String {
        let n = self.errors.len();
        let noun = if n == 1 { "error" } else { "errors" };
        match self.current_error(cx) {
            Some(i) => format!("{} of {n} {noun}", i + 1),
            None => format!("{n} {noun}"),
        }
    }

    /// The find bar, over the console's top right corner.
    fn find_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let this = cx.entity();
        let small = |id: &'static str, glyph: Icon, label: &'static str, tip: &'static str, down: Option<bool>| {
            let this = this.clone();
            tooltip(format!("tip-{id}"), Some(SharedString::from(tip))).focusable(move |f| {
                crate::elements::icon_button(id, glyph, label, true)
                    .track_focus(f)
                    .on_click(move |_, window, cx| {
                        this.update(cx, |c, cx| match down {
                            Some(down) => c.step(down, cx),
                            None => c.close(window, cx),
                        })
                    })
            })
        };
        let this_key = this.clone();
        div()
            .id("find-bar")
            .debug_selector(|| "find-bar".into())
            .absolute()
            .top_1()
            .right_3()
            .flex()
            .items_center()
            .gap_1()
            .p_1()
            .rounded(px(METRICS.radius))
            .border_1()
            .border_color(Theme::color(t.line))
            .bg(Theme::color(t.raised))
            .on_key_down(move |e, window, cx| {
                if e.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this_key.update(cx, |c, cx| c.close(window, cx));
                }
            })
            .child(
                div().w(px(192.)).child(
                    crate::elements::input(&self.field, METRICS.control_height_small).aria_label(FIND_IN_CONSOLE),
                ),
            )
            .child(
                div()
                    .id("find-count")
                    .role(Role::Status)
                    .aria_label(self.count(cx))
                    .min_w(px(64.))
                    .px_1()
                    .text_xs()
                    .text_color(Theme::color(t.muted))
                    .child(self.count(cx)),
            )
            .child(small(
                "find-prev",
                Icon::ArrowUp,
                PREVIOUS_MATCH,
                "Previous match (Shift+Enter)",
                Some(false),
            ))
            .child(small(
                "find-next",
                Icon::ArrowDown,
                NEXT_MATCH,
                "Next match (Enter)",
                Some(true),
            ))
            .child(small("find-close", Icon::X, CLOSE_FIND, "Close (Escape)", None))
    }

    /// The errors group on the Console tab's row (R2); None with no error.
    pub fn errors_group(this: &Entity<Self>, cx: &App) -> Option<impl IntoElement + use<>> {
        let c = this.read(cx);
        if c.errors.is_empty() {
            return None;
        }
        let t = Theme::global(cx).tokens;
        let count = c.error_count(cx);
        let step = |id: &'static str, glyph: Icon, label: &'static str, tip: &'static str, down: bool| {
            let this = this.clone();
            tooltip(format!("tip-{id}"), Some(SharedString::from(tip))).focusable(move |f| {
                crate::elements::icon_button(id, glyph, label, true)
                    .track_focus(f)
                    .on_click(move |_, _, cx| this.update(cx, |c, cx| c.step_error(down, cx)))
            })
        };
        Some(
            div()
                .id("console-errors")
                .debug_selector(|| "console-errors".into())
                .role(Role::Group)
                .aria_label(ERRORS)
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .px_1()
                        .text_xs()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(Theme::color(t.failed))
                        .child(icon(Icon::CircleX, 14., Theme::color(t.failed)))
                        .child(
                            div()
                                .id("errors-count")
                                .role(Role::Status)
                                .aria_label(count.clone())
                                .child(count),
                        ),
                )
                .child(step(
                    "error-prev",
                    Icon::ArrowUp,
                    PREVIOUS_ERROR,
                    "Previous error (Shift+F8)",
                    false,
                ))
                .child(step("error-next", Icon::ArrowDown, NEXT_ERROR, "Next error (F8)", true)),
        )
    }

    /// Line `i`'s highlights: its matches, the current one brighter, or the whole line as the current error.
    fn highlights(
        &self,
        i: usize,
        len: usize,
        current: Option<usize>,
        error: Option<usize>,
        cx: &App,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let t = Theme::global(cx).tokens;
        let bg = |c: u32| HighlightStyle {
            background_color: Some(Theme::color(c)),
            ..Default::default()
        };
        if error.is_some_and(|e| self.errors[e] == i) {
            return vec![(0..len, bg(t.failed_tint))];
        }
        let first = self.matches.partition_point(|m| m.line < i);
        self.matches[first..]
            .iter()
            .enumerate()
            .take_while(|(_, m)| m.line == i)
            .map(|(k, m)| {
                let colour = if current == Some(first + k) {
                    t.found_current
                } else {
                    t.found
                };
                (m.start..m.end, bg(colour))
            })
            .collect()
    }
}

/// The banner's label, padded to nine characters as `padEnd(9)` did, in the muted colour.
fn label(text: &str, cx: &App) -> impl IntoElement {
    div()
        .flex_none()
        .text_color(Theme::color(Theme::global(cx).tokens.muted))
        .child(format!("{text:<9}"))
}

/// What the banner's state line says (F24 R2.1).
pub fn state_text(st: BannerState, state: &AppState) -> SharedString {
    match st {
        BannerState::None => "Open a Karate project folder".into(),
        BannerState::Finding => crate::progress::finding_text(state).into(),
        BannerState::Starting => "Starting Karate…".into(),
        BannerState::Ready => "Ready. Press Run all, or ▶ on a test.".into(),
    }
}

/// Whether the banner's cursor blinks: only while Grace works, and never with reduced motion (CB-08). Idle costs
/// nothing, where a blink draws the window twice a second.
pub fn blinks(st: Option<BannerState>, reduce_motion: bool) -> bool {
    matches!(st, Some(BannerState::Finding | BannerState::Starting)) && !reduce_motion
}

/// A line's colour: ERROR lines and failed tests in the failed colour, WARN lines in the cancelled one.
pub fn line_colour(text: &str, t: &crate::theme::Tokens) -> Option<u32> {
    if is_error_line(text) {
        Some(t.failed)
    } else if text.contains(" WARN ") {
        Some(t.cancelled)
    } else {
        None
    }
}

/// The console's first lines: versions, project, tests found, and what the app is doing (F24).
pub fn banner(state: &AppState, cx: &App) -> impl IntoElement {
    let t = Theme::global(cx).tokens;
    let (app, karate, java) = VERSIONS;
    let line = |id: &'static str| div().id(id).debug_selector(move || id.into()).flex().role(Role::Label);
    let project = state.project.as_ref().map_or("none".to_string(), |p| p.root.clone());
    let tests = tests_text(state);
    let st = banner_state(state);
    let moving = blinks(st, cx.reduce_motion());
    div()
        .id("console-banner")
        .debug_selector(|| "console-banner".into())
        .child(
            line("banner-versions")
                .aria_value(format!("Grace {app} · Karate {karate} · Java {java}"))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(Theme::color(t.accent))
                        .child("Grace"),
                )
                .child(format!(" {app} · Karate {karate} · Java {java}")),
        )
        .child(
            line("banner-project")
                .aria_value(format!("project {project}"))
                .child(label("project", cx))
                .child(div().min_w_0().child(project)),
        )
        .children(tests.map(|tests| {
            line("banner-tests")
                .aria_value(format!("tests {tests}"))
                .child(label("tests", cx))
                .child(tests)
        }))
        .children(st.map(|st| {
            let text = state_text(st, state);
            let cursor = div()
                .id("console-cursor")
                .ml_1()
                .w(px(7.))
                .h(px(12.))
                .bg(Theme::color(t.ink));
            // shown half a second, hidden half a second: two frames a second, not 60
            let cursor = if moving {
                let (phase, ask) = crate::spin::stepped(2, Duration::from_secs(1));
                div()
                    .relative()
                    .child(cursor.opacity(if phase < 0.5 { 1. } else { 0. }))
                    .child(ask)
                    .into_any_element()
            } else {
                cursor.into_any_element()
            };
            let words = match st {
                BannerState::Ready => div()
                    .flex()
                    .child(div().text_color(Theme::color(t.passed)).child("Ready."))
                    .child("\u{a0}Press Run all, or ▶ on a test.")
                    .into_any_element(),
                _ => div().child(text.clone()).into_any_element(),
            };
            line("console-state")
                .aria_value(text)
                .items_center()
                .text_color(Theme::color(t.muted))
                .child(words)
                .child(cursor)
        }))
}

impl Render for Console {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::global(cx).tokens;
        let this = cx.entity();
        let view = self.view.clone();
        let lines = div()
            .id("console")
            .debug_selector(|| "console".into())
            .role(Role::Log)
            .aria_label(CONSOLE)
            .relative()
            .flex_1()
            .min_h_0()
            .bg(Theme::color(t.sunken))
            .px_3()
            .py_2()
            .font_family(crate::theme::MONO)
            .text_xs()
            .text_color(Theme::color(t.ink))
            .child(
                list(self.list.clone(), move |ix, _window, cx| {
                    let c = this.read(cx);
                    let state = c.state(cx);
                    if ix == 0 {
                        return banner(&state, cx).into_any_element();
                    }
                    let i = ix - 1;
                    let Some(text) = state.console.get(i) else {
                        return div().into_any_element();
                    };
                    let t = Theme::global(cx).tokens;
                    let colour = line_colour(text, t);
                    let highlights = c.highlights(i, text.len(), c.current(cx), c.current_error(cx), cx);
                    div()
                        .id(("line", ix))
                        .role(Role::Label)
                        .aria_value(SharedString::from(text.clone()))
                        .when_some(colour, |d, c| d.text_color(Theme::color(c)))
                        .child(StyledText::new(text.clone()).with_highlights(highlights))
                        .into_any_element()
                })
                .size_full(),
            )
            .child(
                canvas(move |b, _, _| view.set(b), |_, _, _, _| {})
                    .absolute()
                    .size_full(),
            )
            .vertical_scrollbar(&self.list);
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(lines)
            .when(self.open, |d| d.child(self.find_bar(cx)))
    }
}

#[cfg(test)]
mod tests;
