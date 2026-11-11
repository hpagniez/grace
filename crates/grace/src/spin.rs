//! A mark that turns while its state lasts (a running test, a run going on), drawn a step at a time.
//!
//! GPUI's `with_animation` asks for a frame 60 times a second, and each frame builds the view the mark is in and
//! paints the whole window again: during a 2,000-test run the turning marks cost more CPU than Electron Grace's
//! CSS spinner, which the compositor turns alone (#320). A mark here turns in `STEPS` steps a turn, one turn a
//! second as `animate-spin`, and asks for its view's next frame only when its step changes. Nothing is asked once
//! no mark is drawn, so no timer outlives the run (ADR 0030).
use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui::{
    App, EntityId, Global, IntoElement, ParentElement as _, Styled as _, Svg, Transformation, Window, canvas, div,
};

/// The steps of one turn: 20 a second, a third of the frames of a 60 Hz screen.
pub const STEPS: u32 = 20;
/// One turn, as Tailwind's `animate-spin`.
pub const TURN: Duration = Duration::from_secs(1);

fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Where in its turn a mark is at `elapsed`, in whole steps: from 0 up to, not including, 1.
pub fn phase_at(elapsed: Duration, steps: u32, period: Duration) -> f32 {
    let step = period.as_nanos() / u128::from(steps);
    let n = (elapsed.as_nanos() / step) % u128::from(steps);
    n as f32 / steps as f32
}

/// How long from `elapsed` to the next step.
pub fn until_next_step(elapsed: Duration, steps: u32, period: Duration) -> Duration {
    let step = period.as_nanos() / u128::from(steps);
    let left = step - elapsed.as_nanos() % step;
    Duration::from_nanos(left as u64)
}

/// The views that have a frame asked for at their next step.
#[derive(Default)]
struct Asked(HashSet<EntityId>);
impl Global for Asked {}

/// How many views have a frame asked for at their next step: none while nothing turns.
#[cfg(test)]
pub fn asked(cx: &App) -> usize {
    cx.try_global::<Asked>().map_or(0, |a| a.0.len())
}

/// Forgets the steps asked so far, so a test sees only those the next frames ask for.
#[cfg(test)]
pub fn forget_asked(cx: &mut App) {
    cx.default_global::<Asked>().0.clear();
}

/// Asks for the current view's frame at the next step of `steps` a `period`, once per view.
fn ask_next_step(steps: u32, period: Duration, window: &mut Window, cx: &mut App) {
    let view = window.current_view();
    if !cx.default_global::<Asked>().0.insert(view) {
        return;
    }
    let wait = until_next_step(epoch().elapsed(), steps, period);
    cx.spawn(async move |cx| {
        cx.background_executor().timer(wait).await;
        cx.update(|cx| {
            cx.default_global::<Asked>().0.remove(&view);
            cx.notify(view);
        });
    })
    .detach();
}

/// The phase of something drawn in `steps` steps a `period`, asking for the frame of its next step: for a mark
/// that changes otherwise than by turning, such as the console's blinking cursor.
pub fn stepped(steps: u32, period: Duration) -> (f32, impl IntoElement) {
    let phase = phase_at(epoch().elapsed(), steps, period);
    let ask = canvas(
        move |_, window, cx| ask_next_step(steps, period, window, cx),
        |_, _, _, _| {},
    )
    .absolute()
    .size_0();
    (phase, ask)
}

/// `mark` turning, a step at a time.
pub fn turning(mark: Svg) -> impl IntoElement {
    let (phase, ask) = stepped(STEPS, TURN);
    div()
        .relative()
        .flex()
        .flex_none()
        .child(mark.with_transformation(Transformation::rotate(gpui::percentage(phase))))
        .child(ask)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #320: a mark turns once a second in 20 steps, and its next frame is asked for when its step changes.
    #[test]
    fn a_mark_turns_once_a_second_in_twenty_steps() {
        let ms = Duration::from_millis;
        assert_eq!(phase_at(ms(0), STEPS, TURN), 0.);
        assert_eq!(phase_at(ms(49), STEPS, TURN), 0.);
        assert_eq!(phase_at(ms(50), STEPS, TURN), 0.05);
        assert_eq!(phase_at(ms(999), STEPS, TURN), 0.95);
        assert_eq!(phase_at(ms(1000), STEPS, TURN), 0.);
        assert_eq!(until_next_step(ms(0), STEPS, TURN), ms(50));
        assert_eq!(until_next_step(ms(30), STEPS, TURN), ms(20));
        assert_eq!(until_next_step(ms(1050), STEPS, TURN), ms(50));
        // the console's cursor: shown half a second, hidden half a second
        assert_eq!(phase_at(ms(400), 2, TURN), 0.);
        assert_eq!(phase_at(ms(600), 2, TURN), 0.5);
    }
}
