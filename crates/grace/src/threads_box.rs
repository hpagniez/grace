//! `Threads.tsx`: the project's thread count, a number committed through the runner. The box shows only what the runner
//! last said; the runner sends the count in force back, also when it refused a change.
//!
//! Electron Grace's box was an `<input type=number>`. It is GPUI Kit's `Input` here, read as a spin button: Enter or
//! leaving the box sends a changed value, and the arrow keys step it by one between 1 and 32 and send it at once, as
//! Chromium's `change` event did (plan, Redesigns).
use std::rc::Rc;

use gpui::accesskit::Role;
use gpui::{
    App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_component::input::{Input, InputEvent, InputState, MoveDown, MoveUp};
use grace_core::model::threads::MAX_THREADS;
use grace_core::state::AppState;
use serde_json::Value;

use crate::app::Grace;
use crate::theme::{METRICS, Theme};
use crate::tooltip::{Side, tooltip};

pub const LABEL: &str = "Threads";
/// The box's width: Tailwind's `w-14`.
pub const WIDTH: f32 = 56.;

/// The box's text as the runner takes it: a number as written, `null` for anything else (`valueAsNumber`'s `NaN`).
/// The runner checks it.
pub fn value(text: &str) -> Value {
    let text = text.trim();
    if let Ok(n) = text.parse::<i64>() {
        return Value::from(n);
    }
    text.parse::<f64>()
        .ok()
        .and_then(serde_json::Number::from_f64)
        .map_or(Value::Null, Value::Number)
}

/// One step of the arrow keys from `text`, kept between 1 and 32 as `min` and `max` kept Chromium's; `None` when the
/// step changes nothing. An empty or unreadable box steps to 1.
pub fn step(text: &str, up: bool) -> Option<u32> {
    let max = f64::from(MAX_THREADS);
    let next = match text.trim().parse::<f64>() {
        Ok(v) if v.is_finite() => {
            let n = if up { v.floor() + 1. } else { v.ceil() - 1. };
            n.clamp(1., max)
        }
        _ => 1.,
    };
    let same = text.trim().parse::<f64>().is_ok_and(|v| v == next);
    (!same).then_some(next as u32)
}

/// What a box may hold: Chromium's number box takes only these characters.
fn numeric(text: &str) -> bool {
    text.chars().all(|c| c.is_ascii_digit() || ".eE+-".contains(c))
}

pub struct ThreadsBox {
    grace: Entity<Grace>,
    pub input: Entity<InputState>,
    /// The text last sent or shown from the runner: Enter or leaving sends only a different one.
    committed: String,
    /// The runner's view last shown: the box resets on each new one.
    seen: Option<Option<u64>>,
    _subscriptions: Vec<Subscription>,
}

impl ThreadsBox {
    pub fn new(grace: Entity<Grace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).validate(|t, _| numeric(t)));
        let subscriptions = vec![
            cx.observe_in(&grace, window, |this, _, window, cx| this.sync(window, cx)),
            cx.subscribe_in(&input, window, Self::on_input),
        ];
        let mut me = Self {
            grace,
            input,
            committed: String::new(),
            seen: None,
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

    /// Every `threads` event resets the box, so a refused count never stays in it; with no project it shows 1.
    fn sync(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state(cx);
        let seen = state.threads.map(|t| t.generation);
        if self.seen == Some(seen) {
            return;
        }
        self.seen = Some(seen);
        let shown = state.threads.map_or(1, |t| t.count).to_string();
        self.committed = shown.clone();
        self.input.update(cx, |i, cx| i.set_value(shown, window, cx));
        cx.notify();
    }

    fn on_input(&mut self, _: &Entity<InputState>, event: &InputEvent, _: &mut Window, cx: &mut Context<Self>) {
        if let InputEvent::PressEnter { .. } | InputEvent::Blur = event {
            self.commit(cx);
        }
    }

    /// Sends what the box holds, if it changed since the last send or the runner's last count.
    fn commit(&mut self, cx: &mut Context<Self>) {
        let text = self.input.read(cx).value().to_string();
        if text != self.committed {
            self.send(text, cx);
        }
    }

    fn send(&mut self, text: String, cx: &mut Context<Self>) {
        let state = self.state(cx);
        if Self::locked(&state) || state.threads.is_none() {
            return;
        }
        let n = value(&text);
        self.committed = text;
        self.grace.update(cx, |g, cx| g.set_threads(n, cx));
    }

    fn step(&mut self, up: bool, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state(cx);
        if Self::locked(&state) || state.threads.is_none() {
            return;
        }
        let text = self.input.read(cx).value().to_string();
        if let Some(n) = step(&text, up) {
            let text = n.to_string();
            self.input.update(cx, |i, cx| i.set_value(text.clone(), window, cx));
            self.send(text, cx);
        }
    }
}

impl Render for ThreadsBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.state(cx);
        let t = Theme::global(cx).tokens;
        let disabled = Self::locked(&state) || state.threads.is_none();
        let narrow = crate::toolbar::narrow(window);
        let tip = narrow.then(|| SharedString::from(LABEL));
        let boxed = tooltip("tip-threads", tip).side(Side::Left).child(
            div().debug_selector(|| "threads-input".into()).w(px(WIDTH)).child(
                Input::new(&self.input)
                    .role(Role::SpinButton)
                    .aria_label(LABEL)
                    .h(px(METRICS.control_height))
                    .disabled(disabled),
            ),
        );
        div()
            .id("threads")
            .debug_selector(|| "threads".into())
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .text_color(Theme::color(t.muted))
            // a press elsewhere commits before its click, as the DOM's blur came before it: Run all then runs with the
            // count just typed. GPUI tells the box it lost the focus only at the next frame, after the click.
            .on_mouse_down_out(cx.listener(|b, _, window, cx| {
                if b.input.read(cx).focus_handle(cx).is_focused(window) {
                    b.commit(cx);
                }
            }))
            .capture_action(cx.listener(|b, _: &MoveUp, window, cx| b.step(true, window, cx)))
            .capture_action(cx.listener(|b, _: &MoveDown, window, cx| b.step(false, window, cx)))
            .when(!narrow, |d| d.child(div().flex_none().child(LABEL)))
            .child(boxed)
    }
}

#[cfg(test)]
mod tests;
