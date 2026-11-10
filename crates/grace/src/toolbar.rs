//! `Toolbar.tsx`: the row above the sheet (toolbar spec, premium UI R13.1). Home, the project's name and layout,
//! Env, Threads, Properties…, Refresh, Open…, Report, Re-run failed with its count, Cancel and Run all, in that order,
//! Run all last and the only primary button. Below 1280 px the labels of Properties…, Open…, Report, Re-run failed and
//! Cancel go from sight, not from the buttons' names, and say themselves in a tooltip instead (`narrow.ts`).
//!
//! The environment and threads boxes are `env_picker`'s and `threads_box`'s, the Properties dialog `properties`'; the
//! Watch toggle and the step debugger's buttons come with the runner's watch and debug runs in phase P4 (P2 plan).
use crate::elements::Off as _;
use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Entity, ImageSource, InteractiveElement as _, IntoElement, ParentElement, Resource, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, img, prelude::FluentBuilder as _, px,
};
use gpui_component::button::{Button, ButtonVariants as _};
use grace_core::model::node::Layout;
use grace_core::model::tags::matching_leaves;
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::{Icon, LOGO_PATH};
use crate::elements::{ButtonKind, ButtonSize, button};
use crate::theme::{METRICS, Theme};
use crate::tooltip::{cut_middle, tooltip};

pub const HOME: &str = "Home";
pub const REFRESH: &str = "Refresh";
pub const OPEN: &str = "Open…";
pub const REPORT: &str = "Report";
pub const RERUN_FAILED: &str = "Re-run failed";
pub const CANCEL: &str = "Cancel";
pub const RUN_ALL: &str = "Run all";

/// Below this window width the toolbar shows four of its buttons as icons (`NARROW_QUERY`).
pub const NARROW_BELOW: f32 = 1280.;
/// A cut name keeps this many characters of its end in sight; they fit the 760 px window (R1.2).
pub const NAME_TAIL: usize = 12;

/// The chip beside the name.
pub fn layout_label(layout: Layout) -> &'static str {
    match layout {
        Layout::Maven => "Maven",
        Layout::Gradle => "Gradle",
        Layout::Standalone => "Standalone",
    }
}

/// A folder's name: its last segment, whichever slash it uses and with a trailing one; the whole path when it has
/// none (`folderName`).
pub fn folder_name(path: &str) -> &str {
    path.split(['\\', '/']).rfind(|s| !s.is_empty()).unwrap_or(path)
}

/// The name in two parts: the start, which gives way when the name is cut, and its last `NAME_TAIL` characters,
/// which stay. A name of `NAME_TAIL` characters or fewer is all end.
pub fn name_parts(name: &str) -> (&str, &str) {
    let at = name.char_indices().rev().nth(NAME_TAIL - 1).map_or(0, |(i, _)| i);
    name.split_at(at)
}

/// Which buttons take a press, and the count on Re-run failed: `Toolbar.tsx`'s rules. A run or a discovery holds
/// everything but Cancel; Run all needs a test the filter lets through (WS-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actions {
    pub home: bool,
    pub refresh: bool,
    pub open: bool,
    pub report: bool,
    pub rerun_failed: bool,
    pub cancel: bool,
    pub run_all: bool,
    /// Properties…: a project is open and nothing runs.
    pub properties: bool,
    /// The newest run's failed tests; the pill shows only above 0.
    pub failed: u32,
}

pub fn actions(state: &AppState) -> Actions {
    let busy = state.run.active || state.busy;
    // the runner sends the newest run first
    let failed = state.history.first().map_or(0, |h| h.failed);
    let can_run = state
        .tree
        .as_ref()
        .is_some_and(|t| !matching_leaves(t, &state.filter).is_empty());
    Actions {
        home: !busy,
        refresh: !busy,
        open: !busy,
        report: !busy && state.run.report_available,
        rerun_failed: !busy && failed > 0,
        cancel: busy,
        run_all: !busy && can_run,
        properties: !busy && state.env.is_some(),
        failed,
    }
}

/// Whether the window is narrow enough for the icon-only buttons.
pub fn narrow(window: &Window) -> bool {
    f32::from(window.viewport_size().width) < NARROW_BELOW
}

/// The toolbar of the open project, with `boxes` (the environment and the thread count) after the project's name.
pub fn render(
    grace: &Entity<Grace>,
    state: &AppState,
    boxes: Vec<AnyElement>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let t = Theme::global(cx).tokens;
    let a = actions(state);
    let narrow = narrow(window);
    let path = state
        .project
        .as_ref()
        .map(|p| p.root.as_str())
        .or(state.opening.as_deref())
        .unwrap_or("No project");
    let name = folder_name(path);
    let (head, tail) = name_parts(name);
    let chip = state.project.as_ref().map(|p| layout_label(p.layout));

    let g = grace.clone();
    let home = tooltip("tip-home", Some(HOME.into())).focusable(move |f| {
        Button::new("home-button")
            .ghost()
            .w(px(METRICS.control_height))
            .h(px(METRICS.control_height))
            .px_0()
            .justify_center()
            .accessibility_label(HOME)
            .track_focus(f)
            .off(!a.home)
            .child(
                img(ImageSource::Resource(Resource::Embedded(LOGO_PATH.into())))
                    .size(px(20.))
                    .flex_none(),
            )
            .on_click(move |_, _, cx| g.update(cx, |g, cx| g.close_project(cx)))
    });
    let g = grace.clone();
    let refresh = tooltip("tip-refresh", Some(REFRESH.into())).focusable(move |f| {
        crate::elements::icon_button("refresh-button", Icon::RefreshCw, REFRESH, false)
            .track_focus(f)
            .off(!a.refresh)
            .on_click(move |_, _, cx| g.update(cx, |g, cx| g.refresh(cx)))
    });

    // one 20 px line that wraps and hides the second: once the name and the chip no longer fit side by side, the
    // chip goes there, whole, and only then is the name cut (R1.2). The path is in the status bar.
    let project = div()
        .id("project")
        .debug_selector(|| "project".into())
        .flex()
        .flex_1()
        .flex_wrap()
        .items_center()
        .gap_x_2()
        .min_w_0()
        .h(px(20.))
        .overflow_hidden()
        .pr_2()
        .child(
            div()
                .debug_selector(|| "project-name".into())
                .flex()
                .min_w_0()
                .text_size(px(METRICS.text_base.size))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(cut_middle("project-name", head.to_string(), tail.to_string())),
        )
        .when_some(chip, |d, chip| {
            d.child(
                div()
                    .id("project-layout")
                    .debug_selector(|| "project-layout".into())
                    .role(Role::Label)
                    .aria_value(chip)
                    .flex_none()
                    .h(px(20.))
                    .px(px(6.))
                    .rounded(px(4.)) // Tailwind's rounded-sm
                    .bg(Theme::color(t.selected))
                    .text_size(px(METRICS.text_2xs.size))
                    .line_height(px(20.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(Theme::color(t.muted))
                    .child(chip),
            )
        });

    let g = grace.clone();
    let properties = labelled(
        "properties-button",
        Icon::SlidersHorizontal,
        crate::properties::LABEL,
        a.properties,
        narrow,
        None,
        move |window, cx| crate::properties::open(&g, window, cx),
    );
    let g = grace.clone();
    let open = labelled(
        "open-button",
        Icon::FolderOpen,
        OPEN,
        a.open,
        narrow,
        None,
        move |_, cx| g.update(cx, |g, cx| g.open_project(cx)),
    );
    let g = grace.clone();
    let report = labelled(
        "report-button",
        Icon::FileText,
        REPORT,
        a.report,
        narrow,
        None,
        move |_, cx| g.update(cx, |g, cx| g.open_report(cx)),
    );
    let g = grace.clone();
    let pill = (a.failed > 0).then(|| count_pill(a.failed, cx));
    let rerun = labelled(
        "rerun-failed-button",
        Icon::RotateCcw,
        RERUN_FAILED,
        a.rerun_failed,
        narrow,
        pill,
        move |_, cx| g.update(cx, |g, cx| g.rerun_failed(cx)),
    );
    let g = grace.clone();
    let cancel = labelled(
        "cancel-button",
        Icon::Square,
        CANCEL,
        a.cancel,
        narrow,
        None,
        move |_, cx| g.update(cx, |g, cx| g.cancel(cx)),
    );
    let g = grace.clone();
    let run_all = div().debug_selector(|| "run-all-button".into()).child(
        button("run-all-button", RUN_ALL, ButtonKind::Primary, ButtonSize::Medium)
            .icon(Icon::Play)
            .off(!a.run_all)
            .on_click(move |_, _, cx| g.update(cx, |g, cx| g.run_all(cx))),
    );

    div()
        .id("toolbar")
        .debug_selector(|| "toolbar".into())
        .flex()
        .flex_none()
        .items_center()
        .gap_2()
        .min_h(px(44.))
        .px_3()
        .bg(Theme::color(t.ground))
        .child(div().debug_selector(|| "home-button".into()).child(home))
        .child(project)
        .children(boxes)
        .child(properties)
        .child(divider("divider-actions", cx))
        .child(div().debug_selector(|| "refresh-button".into()).child(refresh))
        .child(open)
        .child(divider("divider-run", cx))
        .child(report)
        .child(rerun)
        .child(cancel)
        .child(run_all)
}

/// Between the groups of buttons (R13.1): 1 px by 20, in the line colour, not read out.
fn divider(id: &'static str, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(move || id.into())
        .flex_none()
        .w(px(1.))
        .h(px(20.))
        .bg(Theme::color(Theme::global(cx).tokens.line))
}

/// The count on Re-run failed (`countPill`).
fn count_pill(n: u32, cx: &App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    div()
        .debug_selector(|| "rerun-failed-count".into())
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(px(16.))
        .min_w(px(20.))
        .px(px(6.))
        .rounded_full()
        .bg(Theme::color(t.selected))
        .text_size(px(METRICS.text_2xs.size))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(Theme::color(t.muted))
        .child(SharedString::from(n.to_string()))
        .into_any_element()
}

/// A text button with its icon that, in a narrow window, shows only its icon (and `extra`, the count) and says its
/// label in a tooltip, also while disabled; while the label shows, no tooltip repeats it.
fn labelled(
    id: &'static str,
    icon: Icon,
    label: &'static str,
    enabled: bool,
    narrow: bool,
    extra: Option<AnyElement>,
    press: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tip = narrow.then(|| SharedString::from(label));
    let wrapped = tooltip(SharedString::from(format!("tip-{id}")), tip).focusable(move |f| {
        let b = if narrow {
            let side = METRICS.control_height;
            Button::new(id)
                .outline()
                .icon(icon)
                .h(px(side))
                .rounded(px(METRICS.radius))
                .accessibility_label(label)
                .map(|b| {
                    if extra.is_some() {
                        b.px_2()
                    } else {
                        b.w(px(side)).px_0()
                    }
                })
        } else {
            button(id, label, ButtonKind::Plain, ButtonSize::Medium).icon(icon)
        };
        b.track_focus(f)
            .off(!enabled)
            .children(extra)
            .on_click(move |_, window, cx| press(window, cx))
    });
    div().debug_selector(move || id.into()).child(wrapped)
}

#[cfg(test)]
mod tests;
