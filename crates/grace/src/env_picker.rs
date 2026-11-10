//! `EnvPicker.tsx`: the project's environment, a name typed or picked, committed through the runner. The box shows
//! only what the runner last said; the runner sends the environment in force back, also when it refused a change.
//!
//! Electron Grace's box was an `<input list>` with a `<datalist>` of suggestions. GPUI has neither, so the box is GPUI
//! Kit's `Input` read as a combo box, and its suggestions a list under it while it has the focus (plan, Redesigns).
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription,
    Window, deferred, div, prelude::FluentBuilder as _, px,
};
use gpui_base::ElementExt as _;
use gpui_component::input::{Escape, InputEvent, InputState, MoveDown, MoveUp};
use grace_core::model::env::{EnvView, valid_env_name};
use grace_core::state::AppState;

use crate::app::Grace;
use crate::theme::{METRICS, Theme};
use crate::tooltip::{Side, tooltip};
use crate::ui::{FieldKind, field_message};

/// No environment, as the box and its first suggestion say it.
pub const NONE: &str = "(none)";
pub const LABEL: &str = "Env";
pub const SUGGESTIONS: &str = "Environments";
pub use grace_core::model::env::INVALID_ENV;
/// The box's width: Tailwind's `w-28`.
pub const WIDTH: f32 = 112.;

/// The line under the box when the environment has no overlay file of its own; the file is found whatever its letter
/// case, as Windows finds it.
pub fn hint(view: &EnvView) -> Option<String> {
    let env = view.env.as_deref()?;
    let lower = env.to_lowercase();
    (!view.overlays.iter().any(|o| o.to_lowercase() == lower))
        .then(|| format!("No karate-config-{env}.js found. Fine if karate-config.js handles it."))
}

/// The suggestions for what the box holds: `(none)` then the project's environments, those that hold the text, in
/// any letter case. An empty box offers them all; Chromium's datalist filtered its list the same way.
pub fn suggestions(view: &EnvView, text: &str) -> Vec<String> {
    let text = text.trim().to_lowercase();
    std::iter::once(NONE.to_string())
        .chain(view.suggestions.iter().cloned())
        .filter(|s| s.to_lowercase().contains(&text))
        .collect()
}

/// What a commit of `value` does, with `current` the environment in force and `sent` the name sent and not yet
/// answered (`EnvPicker.tsx`'s `commit`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Commit {
    /// Ask the runner for this environment.
    Send(Option<String>),
    /// Already on its way: keep the text until the runner answers.
    Pending,
    /// Not a name the runner takes: say why under the box.
    Invalid,
    /// The environment in force: nothing to send, the box shows it again.
    Restore,
}

pub fn commit(value: &str, current: Option<&str>, sent: Option<&Option<String>>) -> Commit {
    let name = value.trim();
    let next = (!name.is_empty() && name != NONE).then(|| name.to_string());
    if sent == Some(&next) {
        Commit::Pending
    } else if next.as_deref().is_some_and(|n| !valid_env_name(n)) {
        Commit::Invalid
    } else if next.as_deref() != current {
        Commit::Send(next)
    } else {
        Commit::Restore
    }
}

/// What an empty box says: the environment in force, `(none)` for none; nothing with no project's view.
pub fn placeholder(state: &AppState) -> String {
    match &state.env {
        Some(v) => v.env.clone().unwrap_or_else(|| NONE.to_string()),
        None => String::new(),
    }
}

pub struct EnvPicker {
    grace: Entity<Grace>,
    pub input: Entity<InputState>,
    error: Option<&'static str>,
    /// Typed into since the focus: leaving an untouched box puts the environment back.
    typed: bool,
    /// The name sent until the runner's next view; `None` when none is on its way. A pick commits at once, and the
    /// Enter or blur after it can come before the runner's busy echo locks the box.
    sent: Option<Option<String>>,
    /// The text last seen in the box.
    text: String,
    /// The suggestions show.
    open: bool,
    /// The suggestion the arrow keys are on.
    highlight: Option<usize>,
    /// The runner's view last shown: the box resets on each new one.
    seen: Option<(u64, bool)>,
    /// Where the suggestions were last drawn: a press there picks, so it does not commit what was typed.
    list: Rc<std::cell::Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
    _subscriptions: Vec<Subscription>,
}

impl EnvPicker {
    pub fn new(grace: Entity<Grace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx));
        let subscriptions = vec![
            cx.observe_in(&grace, window, |this, _, window, cx| this.sync(window, cx)),
            cx.subscribe_in(&input, window, Self::on_input),
        ];
        let mut me = Self {
            grace,
            input,
            error: None,
            typed: false,
            sent: None,
            text: String::new(),
            open: false,
            highlight: None,
            seen: None,
            list: Rc::default(),
            _subscriptions: subscriptions,
        };
        me.sync(window, cx);
        me
    }

    fn state(&self, cx: &App) -> Rc<AppState> {
        self.grace.read(cx).state.clone()
    }

    fn locked(state: &AppState) -> bool {
        state.run.active || state.busy
    }

    /// The box takes no input with no project's view, nor while a run or a discovery holds the runner.
    pub fn disabled(state: &AppState) -> bool {
        Self::locked(state) || state.env.is_none()
    }

    /// The environment in force, as the box shows it: empty for none, with `(none)` as its placeholder.
    fn shown(state: &AppState) -> String {
        state.env.as_ref().and_then(|v| v.env.clone()).unwrap_or_default()
    }

    /// Every new view resets the box, so a refused change never leaves a name the runner is not using.
    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state(cx);
        let seen = (state.env_generation, state.env.is_some());
        if self.seen == Some(seen) {
            return;
        }
        self.seen = Some(seen);
        self.sent = None;
        self.error = None;
        let shown = Self::shown(&state);
        let placeholder = placeholder(&state);
        self.set_text(shown, window, cx);
        self.input
            .update(cx, |i, cx| i.set_placeholder(placeholder, window, cx));
        cx.notify();
    }

    /// Puts `text` in the box, which reports no change for it.
    fn set_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.text = text.clone();
        self.input.update(cx, |i, cx| i.set_value(text, window, cx));
    }

    fn on_input(&mut self, _: &Entity<InputState>, event: &InputEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            // focus empties the box, with the environment as its placeholder, so every suggestion shows
            InputEvent::Focus => {
                self.typed = false;
                self.error = None;
                self.open = true;
                self.highlight = None;
                self.set_text(String::new(), window, cx);
            }
            InputEvent::Change => {
                // GPUI Kit also reports a change that leaves the text as it was, such as after Enter
                let text = self.input.read(cx).value().to_string();
                if text == self.text {
                    return;
                }
                self.text = text;
                self.typed = true;
                self.error = None;
                self.open = true;
                self.highlight = None;
            }
            InputEvent::PressEnter { .. } => match self.highlighted(cx) {
                Some(pick) => self.pick(pick, window, cx),
                None => self.leave(window, cx),
            },
            InputEvent::Blur => {
                self.open = false;
                self.leave(window, cx);
            }
        }
        cx.notify();
    }

    fn options(&self, cx: &App) -> Vec<String> {
        let state = self.state(cx);
        let text = self.input.read(cx).value();
        state.env.as_ref().map(|v| suggestions(v, &text)).unwrap_or_default()
    }

    fn highlighted(&self, cx: &App) -> Option<String> {
        if !self.open {
            return None;
        }
        self.options(cx).get(self.highlight?).cloned()
    }

    /// Puts the environment in force back in the box.
    fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.typed = false;
        self.error = None;
        let shown = Self::shown(&self.state(cx));
        self.set_text(shown, window, cx);
    }

    fn leave(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.typed {
            let value = self.input.read(cx).value().to_string();
            self.commit(&value, window, cx);
        } else {
            self.restore(window, cx);
        }
    }

    /// A suggestion picked with the pointer or Enter: in the box, and sent at once.
    fn pick(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.highlight = None;
        self.typed = true;
        self.set_text(name.clone(), window, cx);
        self.commit(&name, window, cx);
    }

    fn commit(&mut self, value: &str, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state(cx);
        if Self::locked(&state) {
            return;
        }
        let current = state.env.as_ref().and_then(|v| v.env.as_deref());
        match commit(value, current, self.sent.as_ref()) {
            Commit::Pending => {}
            Commit::Invalid => self.error = Some(INVALID_ENV),
            Commit::Send(next) => {
                self.sent = Some(next.clone());
                self.grace.update(cx, |g, cx| g.set_env(next, cx));
            }
            Commit::Restore => self.restore(window, cx),
        }
        cx.notify();
    }

    /// The arrow keys move through the suggestions, opening them; Escape puts the environment back and closes them.
    fn step(&mut self, down: bool, cx: &mut Context<Self>) {
        let n = self.options(cx).len();
        if n == 0 {
            return;
        }
        self.open = true;
        self.highlight = Some(match (self.highlight, down) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        });
        cx.notify();
    }

    fn render_list(&self, options: Vec<String>, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::global(cx).tokens;
        let me = cx.entity();
        let rows = options.into_iter().enumerate().map(|(i, name)| {
            let on = self.highlight == Some(i);
            let me = me.clone();
            let picked = name.clone();
            div()
                .id(("env-option", i))
                .debug_selector(move || format!("env-option-{i}"))
                .role(Role::ListBoxOption)
                .aria_label(SharedString::from(name.clone()))
                .aria_selected(on)
                .px_2()
                .py_1()
                .rounded(px(METRICS.radius))
                .cursor_pointer()
                .when(on, |d| d.bg(Theme::color(t.selected)))
                .hover(|d| d.bg(Theme::color(t.hover)))
                .text_color(Theme::color(t.ink))
                .child(name)
                // on the press, before the box would lose the focus
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    cx.stop_propagation();
                    let picked = picked.clone();
                    me.update(cx, |p, cx| p.pick(picked, window, cx));
                })
        });
        let list = self.list.clone();
        deferred(
            div()
                .id("env-suggestions")
                .on_prepaint(move |b, _, _| list.set(Some(b)))
                .debug_selector(|| "env-suggestions".into())
                .role(Role::ListBox)
                .aria_label(SUGGESTIONS)
                .absolute()
                .top(px(METRICS.control_height + 4.))
                .left_0()
                .w(px(WIDTH))
                .occlude()
                .p_1()
                .rounded(px(METRICS.radius_lg))
                .border_1()
                .border_color(Theme::color(t.line))
                .bg(Theme::color(t.raised))
                .shadow(crate::theme::float_shadow(cx))
                .children(rows),
        )
        .with_priority(1)
        .into_any_element()
    }
}

impl Render for EnvPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state(cx);
        let t = Theme::global(cx).tokens;
        let disabled = Self::disabled(&state);
        let narrow = crate::toolbar::narrow(window);
        let focused = self.input.read(cx).focus_handle(cx).is_focused(window);
        let options = if self.open && focused && !disabled {
            self.options(cx)
        } else {
            Vec::new()
        };
        let message = match (self.error, state.env.as_ref().and_then(hint)) {
            (Some(error), _) => Some(field_message("env-error", FieldKind::Error, error, cx)),
            (None, Some(hint)) => Some(field_message("env-hint", FieldKind::Hint, hint, cx)),
            _ => None,
        };
        let input = self.input.clone();
        // the word Env goes from sight below 1280 px, and the box's tooltip says it beside the box
        let tip = narrow.then(|| SharedString::from(LABEL));
        let boxed = tooltip("tip-env", tip).side(Side::Left).child(
            div().debug_selector(|| "env-input".into()).w(px(WIDTH)).child(
                crate::elements::input(&input)
                    .role(Role::EditableComboBox)
                    .aria_label(LABEL)
                    .h(px(METRICS.control_height))
                    .disabled(disabled),
            ),
        );
        div()
            .id("env-picker")
            .debug_selector(|| "env-picker".into())
            .flex()
            .flex_col()
            .flex_none()
            // a press elsewhere commits before its click, as the DOM's blur came before it: GPUI tells the box it lost
            // the focus only at the next frame, after the click. A press on a suggestion picks it instead.
            .on_mouse_down_out(cx.listener(|p, e: &gpui::MouseDownEvent, window, cx| {
                let on_list = p.open && p.list.get().is_some_and(|b| b.contains(&e.position));
                if !on_list && p.input.read(cx).focus_handle(cx).is_focused(window) {
                    p.open = false;
                    p.leave(window, cx);
                    cx.notify();
                }
            }))
            .capture_action(cx.listener(|p, _: &MoveDown, _, cx| p.step(true, cx)))
            .capture_action(cx.listener(|p, _: &MoveUp, _, cx| p.step(false, cx)))
            .capture_action(cx.listener(|p, _: &Escape, window, cx| {
                p.open = false;
                p.highlight = None;
                p.restore(window, cx);
                cx.notify();
            }))
            .child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_color(Theme::color(t.muted))
                    .when(!narrow, |d| d.child(div().flex_none().child(LABEL)))
                    .child(boxed)
                    .when(!options.is_empty(), |d| d.child(self.render_list(options, cx))),
            )
            .children(message)
    }
}

#[cfg(test)]
mod tests;
