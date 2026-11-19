//! `FlakyProof.tsx`: the runs that prove a test flaky, one box per environment, drawn by `Details` after the test's
//! place, time and tags and before its error (docs/specs/2026-10-26-flaky-tests.md R5). What the boxes say is
//! `grace_core::flaky_proof`'s; a Report opens that run's report through the runner and the `open_path` seam (FL-04).
use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use grace_core::flaky_proof::{ProofBox, ProofRun};
use grace_core::model::flaky::FlakyStatus;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, MarkStatus, Off as _, button, icon, status_mark};
use crate::theme::{METRICS, Theme};

/// What a test names an environment by: `-` for none.
fn name(b: &ProofBox) -> &str {
    b.env.as_deref().unwrap_or("-")
}

fn row(grace: &Entity<Grace>, env: &str, r: &ProofRun, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    let (id, shown) = (r.id.clone(), format!("{}, {}, {}", r.when, r.result, r.threads));
    let click = {
        let (grace, id) = (grace.clone(), r.id.clone());
        move |_: &gpui::ClickEvent, _: &mut gpui::Window, cx: &mut App| {
            grace.update(cx, |g, cx| g.open_run_report(id.clone(), cx))
        }
    };
    let mark = match r.status {
        FlakyStatus::Passed => MarkStatus::Passed,
        FlakyStatus::Failed => MarkStatus::Failed,
    };
    let cell = |text: String| div().flex_none().child(text);
    div()
        .id(SharedString::from(format!("flaky-run-{env}-{id}")))
        .debug_selector({
            let (env, id) = (env.to_string(), id.clone());
            move || format!("flaky-run:{env}:{id}")
        })
        .role(Role::ListItem)
        .aria_label(SharedString::from(shown))
        .flex()
        .flex_wrap()
        .items_center()
        .gap_x_3()
        .gap_y_1()
        .text_size(px(METRICS.text_xs.size))
        .text_color(Theme::color(t.ink))
        .child(cell(r.when.clone()))
        .child(
            div()
                .debug_selector({
                    let (env, id) = (env.to_string(), id.clone());
                    move || format!("flaky-run-mark:{env}:{id}")
                })
                .flex()
                .flex_none()
                .items_center()
                .gap_1()
                // the word names the result, so the mark is decorative
                .child(status_mark(
                    SharedString::from(format!("flaky-mark-{env}-{id}")),
                    mark,
                    14.,
                    true,
                    cx.reduce_motion(),
                    cx,
                ))
                .child(r.result),
        )
        .child(cell(r.threads.clone()))
        .child(
            div()
                .debug_selector({
                    let (env, id) = (env.to_string(), id.clone());
                    move || format!("flaky-report:{env}:{id}")
                })
                .child(
                    button(
                        SharedString::from(format!("flaky-report-{env}-{id}")),
                        "Report",
                        ButtonKind::Plain,
                        ButtonSize::Small,
                    )
                    .off(!r.report)
                    .on_click(click),
                ),
        )
        .into_any_element()
}

/// The boxes of `boxes`, in order.
pub fn view(grace: &Entity<Grace>, boxes: &[ProofBox], cx: &App) -> Vec<AnyElement> {
    let t = Theme::global(cx).tokens;
    boxes
        .iter()
        .map(|b| {
            let env = name(b).to_string();
            div()
                .id(SharedString::from(format!("flaky-proof-{env}")))
                .debug_selector({
                    let env = env.clone();
                    move || format!("flaky-proof:{env}")
                })
                .role(Role::Group)
                .aria_label(SharedString::from(b.heading.clone()))
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded_lg()
                .bg(Theme::color(t.cancelled_tint))
                .child(
                    div()
                        .id(SharedString::from(format!("flaky-heading-{env}")))
                        .role(Role::Heading)
                        .aria_level(3)
                        .aria_label(SharedString::from(b.heading.clone()))
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .text_size(px(METRICS.text_sm.size))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(
                            div()
                                .debug_selector({
                                    let env = env.clone();
                                    move || format!("flaky-proof-icon:{env}")
                                })
                                .flex()
                                .child(icon(Icon::Shuffle, 14., Theme::color(t.ink))),
                        )
                        .child(b.heading.clone()),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("flaky-counts-{env}")))
                        .role(Role::Label)
                        .aria_value(SharedString::from(b.counts.clone()))
                        .text_size(px(METRICS.text_xs.size))
                        .text_color(Theme::color(t.ink))
                        .child(b.counts.clone()),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("flaky-runs-{env}")))
                        .role(Role::List)
                        .aria_label("Proof runs")
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(b.runs.iter().map(|r| row(grace, &env, r, cx))),
                )
                .into_any_element()
        })
        .collect()
}

#[cfg(test)]
mod tests;
