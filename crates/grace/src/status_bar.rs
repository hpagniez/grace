//! `StatusBar.tsx`: the 24 px bar along the bottom of every view, on the ground in muted 12 px text (spec R21). It
//! starts with the project's path while its view shows (toolbar spec R2), cut so its last folder stays in sight. The
//! update marker, the running version and Settings join with the Settings page in P2's fifth PR and the updater in
//! phase P5 (P2 plan).
use gpui::{App, InteractiveElement as _, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder as _, px};
use grace_core::state::AppState;

use crate::theme::{METRICS, MONO, Theme};
use crate::tooltip::cut_middle;

pub const HEIGHT: f32 = 24.;

/// A path in two parts: the folders before the project's, which give way when it is cut, and the project's folder
/// with the separator after it if it has one, which stays (`splitPath`).
pub fn split_path(path: &str) -> (&str, &str) {
    let trimmed = path.strip_suffix(['\\', '/']).unwrap_or(path);
    let at = trimmed.rfind(['\\', '/']).map_or(0, |i| i + 1);
    path.split_at(at)
}

/// The path the bar shows: the open project's, or the folder opening; none on the home screen.
pub fn shown_path(state: &AppState) -> Option<&str> {
    state
        .project
        .as_ref()
        .map(|p| p.root.as_str())
        .or(state.opening.as_deref())
}

pub fn render(state: &AppState, cx: &App) -> impl IntoElement {
    let t = Theme::global(cx).tokens;
    div()
        .id("status-bar")
        .debug_selector(|| "status-bar".into())
        .flex()
        .flex_none()
        .items_center()
        .justify_end()
        .gap_1()
        .h(px(HEIGHT))
        .px_2()
        .bg(Theme::color(t.ground))
        .text_size(px(METRICS.text_xs.size))
        .text_color(Theme::color(t.muted))
        .when_some(shown_path(state), |d, path| {
            let (head, tail) = split_path(path);
            d.child(
                div()
                    .debug_selector(|| "project-path".into())
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .px(px(6.))
                    .font_family(MONO)
                    .child(cut_middle("project-path", head.to_string(), tail.to_string())),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{open, project};
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use grace_core::model::node::Layout;
    use std::time::Duration;

    fn bar(cx: &mut TestAppContext, width: f32, state: AppState) -> (crate::testing::Harness, VisualTestContext) {
        open(cx, width, state, |grace, _, cx| {
            let state = grace.read(cx).state.clone();
            render(&state, cx).into_any_element()
        })
    }

    /// TB-06: the path while a project is open or opening, none on the home screen; a 24 px bar.
    #[gpui::test]
    fn shows_the_path_while_a_project_is_open_and_none_on_the_home_screen(cx: &mut TestAppContext) {
        let (h, mut cx) = bar(cx, 1200., project("D:\\work\\api", Layout::Maven, 1));
        assert_eq!(cx.debug_bounds("status-bar").unwrap().size.height, px(HEIGHT));
        assert!(cx.debug_bounds("project-path").is_some());
        // the open project stays shown while another opens
        h.update(&mut cx, |s| s.opening = Some("E:\\next".into()));
        assert_eq!(
            shown_path(&h.grace.read_with(&cx, |g, _| g.state.clone())),
            Some("D:\\work\\api")
        );
        h.update(&mut cx, |s| {
            s.project = None;
            s.opening = Some("E:\\next".into());
        });
        assert_eq!(
            shown_path(&h.grace.read_with(&cx, |g, _| g.state.clone())),
            Some("E:\\next")
        );
        h.update(&mut cx, |s| s.opening = None);
        assert!(cx.debug_bounds("project-path").is_none());
    }

    /// TB-07: a cut path says itself whole in a tooltip; one that fits says nothing.
    #[gpui::test]
    fn explains_a_cut_path_in_a_tooltip_and_says_nothing_while_it_fits(cx: &mut TestAppContext) {
        let long = format!("D:\\{}\\api", "deep\\".repeat(60));
        let (h, mut cx) = bar(cx, 760., project(&long, Layout::Maven, 1));
        let hover = |cx: &mut VisualTestContext| {
            let b = cx.debug_bounds("project-path").unwrap();
            cx.simulate_mouse_move(b.center(), None, Modifiers::none());
            cx.executor()
                .advance_clock(crate::tooltip::DELAY + Duration::from_millis(1));
            cx.run_until_parked();
            let tip = h.tip(cx);
            cx.simulate_mouse_move(gpui::point(px(1.), px(600.)), None, Modifiers::none());
            cx.run_until_parked();
            tip
        };
        assert_eq!(hover(&mut cx).as_deref(), Some(long.as_str()));
        h.set(&mut cx, project("D:\\api", Layout::Maven, 1));
        assert_eq!(hover(&mut cx), None);
    }

    #[test]
    fn splits_a_path_at_the_project_folder() {
        assert_eq!(split_path("D:\\work\\api"), ("D:\\work\\", "api"));
        assert_eq!(split_path("/home/me/api"), ("/home/me/", "api"));
        assert_eq!(split_path("/home/me/api/"), ("/home/me/", "api/"));
        assert_eq!(split_path("D:\\work\\api\\"), ("D:\\work\\", "api\\"));
        assert_eq!(split_path("api"), ("", "api"));
    }
}
