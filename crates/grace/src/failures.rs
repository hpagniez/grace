//! `ConfigBanner.tsx`, and the failure groups' parts of `Details.tsx` (docs/specs/2026-11-04-group-failures.md): the
//! banner that says once that `karate-config.js` failed, the stepper under a failed test's error, and a folder's
//! failures by error. What they say is worked out in `grace_core::failures`; these only draw it.
use gpui::accesskit::{Live, Role};
use gpui::{
    AnyElement, App, Entity, FontWeight, InteractiveElement as _, IntoElement as _, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, div, px,
};
use grace_core::failures::{
    FailureGroup, config_failure, failure_groups, group_of, largest_first, reason_of, shown_leaves, step,
};
use grace_core::model::node::TestNode;
use grace_core::rows::leaves;
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, button, icon, icon_button, icon_text_button};
use crate::theme::{METRICS, MONO, Theme};
use crate::tooltip::tooltip;

pub const OPEN_IN_EDITOR: &str = "Open in VS Code";
pub const PREVIOUS: &str = "Previous test with this error";
pub const NEXT: &str = "Next test with this error";
pub const SAME_ERROR: &str = "Failed with the same error";

/// The failures the tree shows, grouped by error, once a run has ended (R3.2): none while a run goes on, so nothing is
/// worked out then.
///
/// shortcut: one pass over the tests each time the pane is drawn, which an idle window does not; cache it by the
/// tree, the filter and the results if a pane of 10,000 failed tests is ever slow.
pub fn groups(state: &AppState, under: Option<&TestNode>) -> Vec<FailureGroup> {
    let Some(node) = under.or(state.tree.as_ref()).filter(|_| !state.run.active) else {
        return Vec::new();
    };
    failure_groups(&shown_leaves(node, &state.filter), &state.statuses, &state.results)
}

pub fn headline(file: &str, count: usize) -> String {
    format!(
        "{file} failed for {count} {}",
        if count == 1 { "test" } else { "tests" }
    )
}

/// The banner under the notice: `karate-config.js` (or an overlay) failed the last run's tests (R6). None while a run
/// goes on, and none for ordinary failures.
pub fn banner(grace: &Entity<Grace>, state: &AppState, cx: &App) -> Option<AnyElement> {
    let tree = state.tree.as_ref().filter(|_| !state.run.active)?;
    let failure = config_failure(&leaves(tree), &state.run.failed_keys, &state.results)?;
    let t = Theme::global(cx).tokens;
    let head = headline(&failure.file, failure.count);
    let (grace, file, line) = (grace.clone(), failure.file.clone(), failure.line);
    let open = icon_text_button(
        "config-failure-open",
        Icon::ExternalLink,
        OPEN_IN_EDITOR,
        ButtonKind::Ghost,
        ButtonSize::Small,
    )
    .accessibility_label(SharedString::from(format!("Open {} in VS Code", failure.file)))
    .debug_selector(|| "config-failure-open".into())
    .on_click(move |_, _, cx| grace.update(cx, |g, cx| g.open_in_editor(file.clone(), line, cx)));
    let reason = div()
        .id("config-failure-reason")
        .debug_selector(|| "config-failure-reason".into())
        .role(Role::Label)
        .aria_value(SharedString::from(failure.reason.clone()))
        .max_h(px(15. * METRICS.text_xs.line_height))
        .overflow_y_scroll()
        .font_family(MONO)
        .text_size(px(METRICS.text_xs.size))
        .text_color(Theme::color(t.ink))
        .child(failure.reason);
    Some(
        crate::a11y::live(
            div()
                .id("config-failure")
                .debug_selector(|| "config-failure".into())
                .role(Role::Alert)
                .aria_label(SharedString::from(head.clone()))
                .flex()
                .flex_col()
                .gap_1()
                .rounded(px(METRICS.radius_lg))
                .p_3()
                .bg(Theme::color(t.failed_tint))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_x_1p5()
                        .gap_y_1()
                        .text_size(px(METRICS.text_sm.size))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icon(Icon::CircleAlert, 14., Theme::color(t.failed)))
                        .child(div().text_color(Theme::color(t.ink)).child(head))
                        .child(open),
                )
                .child(reason),
            Live::Assertive,
        )
        .into_any_element(),
    )
}

/// "3 of 24 tests failed with this error" with Previous and Next, under a failed test's error (R4). Focus stays on the
/// pressed button: the pane is drawn again with the same ids.
pub fn stepper(grace: &Entity<Grace>, group: &FailureGroup, index: usize, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    let words = format!("{} of {} tests failed with this error", index + 1, group.keys.len());
    let go = |id: &'static str, glyph: Icon, name: &'static str, by: isize| {
        let (grace, key) = (grace.clone(), step(group, index, by).to_string());
        tooltip(SharedString::from(format!("tip-{id}")), Some(SharedString::from(name))).focusable(move |f| {
            icon_button(id, glyph, 14., name, true)
                .track_focus(f)
                .debug_selector(move || id.into())
                .on_click(move |_, _, cx| grace.update(cx, |g, cx| g.reveal(&key, cx)))
        })
    };
    div()
        .debug_selector(|| "error-group".into())
        .mt_2()
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(METRICS.text_xs.size))
        .text_color(Theme::color(t.ink))
        .child(
            // polite: the focus stays on Next, so the count is what says the selection moved (R4.4)
            crate::a11y::live(
                div()
                    .id("error-group-count")
                    .role(Role::Status)
                    .aria_label(SharedString::from(words.clone()))
                    .mr_1()
                    .child(words),
                Live::Polite,
            ),
        )
        .child(go("previous-failure", Icon::ChevronUp, PREVIOUS, -1))
        .child(go("next-failure", Icon::ChevronDown, NEXT, 1))
        .into_any_element()
}

/// A folder's own failures by error, largest first, each with its reason cut after 4 lines and Go to first (R5);
/// nothing when it has no group or a run goes on.
pub fn folder_list(grace: &Entity<Grace>, state: &AppState, node: &TestNode, cx: &App) -> Option<AnyElement> {
    let found = largest_first(groups(state, Some(node)));
    if found.is_empty() {
        return None;
    }
    let t = Theme::global(cx).tokens;
    let items = found.into_iter().enumerate().map(|(i, g)| {
        let n = g.keys.len();
        let first = g.keys[0].clone();
        let reason = reason_of(state.results.get(&first)).unwrap_or_default().to_string();
        let grace = grace.clone();
        div()
            .id(("failure-group", i))
            .debug_selector(move || format!("failure-group-{i}"))
            .role(Role::ListItem)
            .aria_label(SharedString::from(format!("{n} tests")))
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(METRICS.text_sm.size))
                    .child(format!("{n} tests"))
                    .child(
                        button(("go-to-first", i), "Go to first", ButtonKind::Ghost, ButtonSize::Small)
                            .accessibility_label(SharedString::from(format!("Go to first of {n} tests")))
                            .debug_selector(move || format!("go-to-first-{i}"))
                            .on_click(move |_, _, cx| grace.update(cx, |g, cx| g.reveal(&first, cx))),
                    ),
            )
            .child(
                // the cut is on an unpadded box inside the padded one: a padded one shows the top of line 5
                div()
                    .id(("failure-reason", i))
                    .debug_selector(move || format!("failure-reason-{i}"))
                    .role(Role::Label)
                    .aria_value(SharedString::from(reason.clone()))
                    .p_2()
                    .rounded(px(METRICS.radius))
                    .border_1()
                    .border_color(Theme::color(t.line))
                    .bg(Theme::color(t.sunken))
                    .font_family(MONO)
                    .text_size(px(METRICS.text_xs.size))
                    .child(div().text_ellipsis().line_clamp(4).child(reason)),
            )
    });
    Some(
        div()
            .id("failure-groups")
            .debug_selector(|| "failure-groups".into())
            .role(Role::Group)
            .aria_label(SAME_ERROR)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id("failure-groups-heading")
                    .role(Role::Heading)
                    .aria_level(3)
                    .aria_label(SAME_ERROR)
                    .text_size(px(METRICS.text_sm.size))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SAME_ERROR),
            )
            .child(
                div()
                    .id("failure-group-list")
                    .role(Role::List)
                    .aria_label(SAME_ERROR)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .children(items),
            )
            .into_any_element(),
    )
}

/// The group a failed test is in, and its place in it, with the stepper to draw (R4.1).
pub fn stepper_for(grace: &Entity<Grace>, state: &AppState, key: &str, cx: &App) -> Option<AnyElement> {
    let all = groups(state, None);
    let (group, index) = group_of(&all, key)?;
    Some(stepper(grace, group, index, cx))
}

#[cfg(test)]
mod tests;
