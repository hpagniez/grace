//! `Progress.tsx`: the bar at the top of the sheet and the line under it (premium UI R14, loading spec). Before any
//! run the line counts the project's tests; during and after one it says done / total, passed, failed, the time and
//! the outcome. While Grace finds tests, and while a run waits for its first result, a stripe slides along the empty
//! track in place of the segments (LV-01 to LV-08).
//!
//! What moves, and when (ADR 0030): the stripe only while it shows, the spinner only while Grace works, the time
//! once a second only during a run that is not paused, and the segments only while a result moves their ends. The
//! stripe, the spinner and the segments move a step of the marks' clock at a time (`spin`, #320). With the system's
//! reduced motion nothing slides, turns or eases (`a11y::follow_reduced_motion`).
use std::time::{Duration, Instant};

use gpui::accesskit::Role;
use gpui::{
    App, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString, SpringConfig,
    SpringState, StatefulInteractiveElement as _, Styled, Task, Window, div, ease_in_out, prelude::FluentBuilder as _,
    px, relative,
};
use grace_core::model::events::RunOutcome;
use grace_core::model::tags::{matching_leaves, no_filter};
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{MarkStatus, icon, status_mark};
use crate::theme::{METRICS, MONO, Theme};
use crate::tooltip::tooltip;

pub const BAR: &str = "Run progress";
pub const ENV_TIP: &str = "Environment of the run";

/// What the line says about the run, with its icon and colour (R14.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub text: String,
    pub icon: Icon,
    pub tone: Tone,
    /// The icon turns: Grace is working.
    pub turning: bool,
}

/// An outcome's colour token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Running,
    Muted,
    Cancelled,
    Failed,
}

impl Tone {
    /// Its colour token.
    fn token(self, cx: &App) -> u32 {
        let t = Theme::global(cx).tokens;
        match self {
            Tone::Running => t.running,
            Tone::Muted => t.muted,
            Tone::Cancelled => t.cancelled,
            Tone::Failed => t.failed,
        }
    }
}

/// What Grace says while it discovers: the build tool building the Java code or listing the libraries first, if
/// any (`findingText`).
pub fn finding_text(state: &AppState) -> String {
    match state.listing {
        None => "Finding tests…".into(),
        Some(tool) if state.building => format!("Building the project's Java code with {tool:?}…"),
        Some(tool) => format!("Listing the project's libraries with {tool:?}…"),
    }
}

/// The outcome of the run, or what the runner is doing; none before anything happened.
pub fn outcome(state: &AppState) -> Option<Outcome> {
    let working = |text: String| Outcome {
        text,
        icon: Icon::LoaderCircle,
        tone: Tone::Running,
        turning: true,
    };
    if state.run.active {
        // a debug run waiting on the person: nothing turns (step debugger R7.2)
        return Some(if state.paused.is_some() {
            Outcome {
                text: "Paused".into(),
                icon: Icon::Pause,
                tone: Tone::Running,
                turning: false,
            }
        } else {
            working("Running…".into())
        });
    }
    if state.busy {
        return Some(working(finding_text(state)));
    }
    let (text, icon, tone) = match state.run.outcome? {
        RunOutcome::Finished => ("Finished", Icon::CircleCheck, Tone::Muted),
        RunOutcome::Cancelled => ("Cancelled", Icon::CircleSlash, Tone::Cancelled),
        RunOutcome::Failed => ("Run failed", Icon::CircleAlert, Tone::Failed),
    };
    Some(Outcome {
        text: text.into(),
        icon,
        tone,
        turning: false,
    })
}

/// Grace is finding tests, or a run has no result yet: nothing in the bar would move, so the stripe shows.
pub fn waiting(state: &AppState) -> bool {
    (state.busy && !state.run.active) || (state.run.active && state.run.done == 0 && state.paused.is_none())
}

/// No run in this view yet: a new project or environment starts one with none.
pub fn idle(state: &AppState) -> bool {
    !state.run.active && state.run.run_id.is_none() && state.run.outcome.is_none()
}

/// The tests of the tree, whatever the filter: the run's total is 0 until a run starts.
pub fn tests(state: &AppState) -> usize {
    state
        .tree
        .as_ref()
        .map_or(0, |t| matching_leaves(t, &no_filter()).len())
}

/// The line's first words: the tests, and "Not run yet" for a project that never ran; done / total once it has.
pub fn count_text(state: &AppState) -> String {
    if idle(state) {
        let n = tests(state);
        let never = if state.history.is_empty() {
            " · Not run yet"
        } else {
            ""
        };
        format!("{n} {}{never}", if n == 1 { "test" } else { "tests" })
    } else {
        format!("{} / {}", state.run.done, state.run.total)
    }
}

/// The passed, failed and interrupted segments' widths, in percent of the track.
pub fn widths(state: &AppState) -> [f32; 3] {
    let r = &state.run;
    let pct = |n: u32| {
        if r.total > 0 {
            (n as f32 / r.total as f32 * 100.).min(100.)
        } else {
            0.
        }
    };
    [pct(r.passed), pct(r.failed), pct(r.interrupted)]
}

/// Minutes and seconds, each on two digits.
pub fn mmss(ms: u64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Now, in the milliseconds since 1970 the runner's `started_at` is in.
fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0., |d| d.as_millis() as f64)
}

/// The slide of the stripe: its period, as `animate-slide`, and its steps, those of the marks' clock (#320).
pub const SLIDE: Duration = Duration::from_millis(1400);
pub const SLIDE_STEPS: u32 = 28;

/// The bar's springs.
const SPRING: SpringConfig = SpringConfig::new(170., 26., 1.);
/// How near its end a segment rests, in percent of the track.
const EPSILON: f32 = 0.05;

/// The passed, failed and interrupted segments, each a spring that keeps its place and speed when a new result moves
/// its end (LV-06).
///
/// Rust-native (#320): GPUI's `with_spring` asks for a frame 60 times a second while a segment moves, the whole run
/// long on a big one; these springs move by the time since the view's last frame and ask for their next frame at the
/// next step of the marks' clock (`spin::next_step`), so the bar moves in the frames the marks and the results draw.
#[derive(Default)]
struct Bar {
    springs: [SpringState; 3],
    targets: [f32; 3],
    at: Option<Instant>,
}

impl Bar {
    /// Moves each segment on by the time since the last step toward the end it had then, and takes the new ends: true
    /// while a segment still moves. A first step, and every step with reduced motion, puts each segment at its end.
    fn step(&mut self, to: [f32; 3], now: Instant, still: bool) -> bool {
        let dt = self.at.map(|at| now.saturating_duration_since(at).as_secs_f32());
        self.at = Some(now);
        let mut moving = false;
        for (i, end) in to.into_iter().enumerate() {
            let s = match dt {
                Some(dt) if !still => SPRING.step(self.springs[i], self.targets[i], dt),
                _ => SpringState {
                    position: end,
                    velocity: 0.,
                },
            };
            self.springs[i] = if SPRING.is_settled(s, end, EPSILON) {
                SpringState {
                    position: end,
                    velocity: 0.,
                }
            } else {
                moving = true;
                s
            };
            self.targets[i] = end;
        }
        moving
    }
}

/// The progress section, its own view: it holds the run's clock and the bar's springs.
pub struct Progress {
    grace: Entity<Grace>,
    bar: Bar,
    /// The run's time at the last tick.
    ticked: u64,
    /// The run the clock follows, and whether it is paused: a change restarts or stops the clock.
    clock_for: Option<(f64, bool)>,
    clock: Option<Task<()>>,
}

impl Progress {
    pub fn new(grace: Entity<Grace>, cx: &mut Context<Self>) -> Self {
        cx.observe(&grace, |this, _, cx| this.follow(cx)).detach();
        let mut me = Self {
            grace,
            bar: Bar::default(),
            ticked: 0,
            clock_for: None,
            clock: None,
        };
        me.follow(cx);
        me
    }

    /// Starts the clock when a run starts or goes on after a pause, and stops it when the run ends or pauses; the time
    /// at a pause stays, and going on counts from the run's start again.
    fn follow(&mut self, cx: &mut Context<Self>) {
        let state = self.grace.read(cx).state.clone();
        let wanted = state
            .run
            .active
            .then_some(state.run.started_at)
            .flatten()
            .map(|at| (at, state.paused.is_some()));
        if wanted != self.clock_for {
            self.clock_for = wanted;
            self.clock = None;
            if let Some((started, paused)) = wanted {
                self.ticked = (now_ms() - started).max(0.) as u64;
                if !paused {
                    self.clock = Some(cx.spawn(async move |this, cx| {
                        loop {
                            cx.background_executor().timer(Duration::from_secs(1)).await;
                            let ticked = this.update(cx, |p, cx| {
                                p.ticked = (now_ms() - started).max(0.) as u64;
                                cx.notify();
                            });
                            if ticked.is_err() {
                                break;
                            }
                        }
                    }));
                }
            }
        }
        cx.notify();
    }

    /// The run's time: the clock's during a run, the summary's after one.
    pub fn elapsed(&self, state: &AppState) -> u64 {
        if state.run.active {
            self.ticked
        } else {
            state
                .run
                .summary
                .as_ref()
                .map_or(self.ticked, |s| s.duration_millis.max(0.) as u64)
        }
    }

    pub fn clock_running(&self) -> bool {
        self.clock.is_some()
    }
}

impl Render for Progress {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.grace.read(cx).state.clone();
        let t = Theme::global(cx).tokens;
        let still = cx.reduce_motion();
        let waiting = waiting(&state);
        let idle = idle(&state);
        let run = &state.run;

        let mut bar = div()
            .id("progress-bar")
            .debug_selector(|| "progress-bar".into())
            .role(Role::ProgressIndicator)
            .aria_label(BAR)
            .relative()
            .flex()
            .gap(px(2.))
            .h(px(6.))
            .overflow_hidden()
            .rounded_full()
            .bg(Theme::color(t.line));
        // no values while it waits: a screen reader then reads it as busy, as `aria-busy` made it
        if !waiting {
            bar = bar
                .aria_min_numeric_value(0.)
                .aria_max_numeric_value(f64::from(run.total))
                .aria_numeric_value(f64::from(run.done));
        }
        // the segments stay laid out under the stripe, holding the last run's widths, but are not drawn (LV-04)
        let colours = [t.passed, t.failed, t.cancelled];
        let ws = widths(&state);
        let moving = self.bar.step(ws, cx.background_executor().now(), still);
        // GPUI clips a child to the track's box, not to its rounded ends, where the browser's `overflow-hidden` rounds
        // the fill too: the first segment rounds its own left end, and a full bar's last segment its right end. A full
        // bar drops its empty segments with their gaps and lets the rest shrink to the track, so that end is the
        // track's; the browser draws them past it and clips the 2 or 4 px off
        let full = !waiting && ws.iter().sum::<f32>() >= 99.95;
        let first = ws.iter().position(|w| *w > 0.);
        let last = ws.iter().rposition(|w| *w > 0.);
        for (i, (w, c)) in ws.into_iter().zip(colours).enumerate() {
            if full && w <= 0. {
                continue;
            }
            let segment = div()
                .debug_selector(move || format!("progress-segment-{i}"))
                .map(|d| if full { d.flex_shrink(1.) } else { d.flex_none() })
                .h_full()
                .when(Some(i) == first, |d| d.rounded_l_full())
                .when(full && Some(i) == last, |d| d.rounded_r_full())
                .when(!waiting, |d| d.bg(Theme::color(c)));
            // a spring keeps its place and speed when a new result moves its end, so a fast run fills smoothly (LV-06)
            let w = self.bar.springs[i].position;
            bar = bar.child(segment.w(relative(w.clamp(0., 100.) / 100.)));
        }
        if moving {
            bar = bar.child(crate::spin::next_step());
        }
        if waiting {
            let stripe = div()
                .debug_selector(|| "progress-stripe".into())
                .absolute()
                .top_0()
                .bottom_0()
                .w(relative(0.3))
                .rounded_full()
                .bg(Theme::color(t.running));
            bar = if still {
                bar.child(stripe.left_0())
            } else {
                // `slide`: from one stripe's width before the track to 340 % of it along, in 1.4 s, a step of the
                // marks' clock at a time (#320)
                let (delta, ask) = crate::spin::stepped(SLIDE_STEPS, SLIDE);
                bar.child(stripe.left(relative(-0.3 + ease_in_out(delta) * 1.32)))
                    .child(ask)
            };
        }

        let mono = |id: &'static str, text: String| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .role(Role::Label)
                .aria_value(SharedString::from(text.clone()))
                .font_family(MONO)
                .child(text)
        };
        // read as one label, "8 passed": the mark beside the word is decorative
        let count = |status: MarkStatus, id: &'static str, n: u32, word: &'static str| {
            div()
                .id(id)
                .debug_selector(move || id.into())
                .role(Role::Label)
                .aria_value(SharedString::from(format!("{n} {word}")))
                .flex()
                .items_center()
                .gap(px(6.))
                .child(status_mark(
                    SharedString::from(format!("{id}-mark")),
                    status,
                    14.,
                    true,
                    still,
                    cx,
                ))
                .child(
                    div()
                        .font_family(MONO)
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(n.to_string()),
                )
                .child(word)
        };
        let env = run.env.clone().map(|env| {
            tooltip("tip-run-env", Some(ENV_TIP.into())).child(
                div()
                    .id("run-env")
                    .debug_selector(|| "run-env".into())
                    .role(Role::Label)
                    .aria_value(SharedString::from(env.clone()))
                    .flex()
                    .items_center()
                    .h(px(20.))
                    .px(px(6.))
                    .rounded(px(4.))
                    .bg(Theme::color(t.selected))
                    .text_size(px(METRICS.text_2xs.size))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(Theme::color(t.muted))
                    .child(env),
            )
        });
        let outcome = outcome(&state).map(|o| {
            let mark = icon(o.icon, 16., Theme::color(o.tone.token(cx)));
            let mark = if o.turning && !still {
                crate::spin::turning(mark).into_any_element()
            } else {
                mark.into_any_element()
            };
            div()
                .id("run-outcome")
                .debug_selector(|| "run-outcome".into())
                .role(Role::Label)
                .aria_value(SharedString::from(o.text.clone()))
                .flex()
                .items_center()
                .gap(px(6.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(mark)
                .child(o.text)
        });

        let text = count_text(&state);
        let line = div()
            .mt_2()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_4()
            .gap_y_1()
            .text_size(px(METRICS.text_sm.size))
            .child(if idle {
                div()
                    .id("progress-text")
                    .debug_selector(|| "progress-text".into())
                    .role(Role::Label)
                    .aria_value(SharedString::from(text.clone()))
                    .text_color(Theme::color(t.muted))
                    .child(text)
            } else {
                mono("progress-text", text)
            })
            .when(!idle, |d| {
                d.child(count(MarkStatus::Passed, "progress-passed", run.passed, "passed"))
                    .child(count(MarkStatus::Failed, "progress-failed", run.failed, "failed"))
                    .child(mono("progress-elapsed", mmss(self.elapsed(&state))))
            })
            .children(env)
            .children(outcome);

        div()
            .id("progress")
            .debug_selector(|| "progress".into())
            .flex_none()
            .border_b_1()
            .border_color(Theme::color(t.line))
            .px_3()
            .py_2()
            .child(bar)
            .child(line)
    }
}

#[cfg(test)]
mod tests;
