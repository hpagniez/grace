//! `Toolbar.tsx`: the row above the sheet (toolbar spec, premium UI R13.1). Home, the project's name and layout,
//! Env, Threads, Properties…, Refresh, Watch, Open…, Report, Re-run failed with its count, Cancel and Run all, in
//! that order, Run all last and the only primary button. Below 1280 px the labels of Properties…, Open…, Report, Re-run failed and
//! Cancel go from sight, not from the buttons' names, and say themselves in a tooltip instead (`narrow.ts`).
//!
//! While a debug run goes on, Run all gives way to Pause, and while it waits to Continue, Step over and Step into
//! (step debugger R6), icons below 1280 px too; their keys, F5, F10 and F11, are the window's.
//!
//! The environment and threads boxes are `env_picker`'s and `threads_box`'s, the Properties dialog `properties`'.
use crate::elements::Off as _;
use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Entity, ImageSource, InteractiveElement as _, IntoElement, ParentElement, Resource, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, img, prelude::FluentBuilder as _, px,
};
use gpui_component::button::{Button, ButtonVariants as _};
use grace_core::model::events::DebugCommand;
use grace_core::model::node::Layout;
use grace_core::model::tags::any_matching_leaf;
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::{Icon, LOGO_PATH};
use crate::elements::{ButtonKind, ButtonSize, icon_text_button};
use crate::theme::{METRICS, Theme};
use crate::tooltip::{cut_middle, tooltip};

pub const HOME: &str = "Home";
pub const REFRESH: &str = "Refresh";
pub const WATCH: &str = "Watch";
pub const WATCH_TIP: &str = "Watch: run a feature when it is saved";
pub const OPEN: &str = "Open…";
pub const REPORT: &str = "Report";
pub const RERUN_FAILED: &str = "Re-run failed";
pub const CANCEL: &str = "Cancel";
pub const RUN_ALL: &str = "Run all";
pub const PAUSE: &str = "Pause";

/// The step debugger's controls while paused, in order, each with its button's id, label, icon and key (R6.3).
pub struct DebugStep {
    pub command: DebugCommand,
    pub id: &'static str,
    pub label: &'static str,
    pub icon: Icon,
    /// As GPUI names it in a keystroke.
    pub key: &'static str,
    /// As assistive technology is told it.
    pub shortcut: &'static str,
}

pub const DEBUG_STEPS: [DebugStep; 3] = [
    DebugStep {
        command: DebugCommand::Continue,
        id: "debug-continue",
        label: "Continue",
        icon: Icon::Play,
        key: "f5",
        shortcut: "F5",
    },
    DebugStep {
        command: DebugCommand::Over,
        id: "debug-over",
        label: "Step over",
        icon: Icon::RedoDot,
        key: "f10",
        shortcut: "F10",
    },
    DebugStep {
        command: DebugCommand::Into,
        id: "debug-into",
        label: "Step into",
        icon: Icon::ArrowDownToDot,
        key: "f11",
        shortcut: "F11",
    },
];

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
    let can_run = state.tree.as_ref().is_some_and(|t| any_matching_leaf(t, &state.filter));
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

/// The Watch toggle (watch mode R1): on and off differ by shape, not only by fill. It is on during a run, as it
/// touches only the watcher, and off while a project is being opened.
#[derive(Clone, Copy)]
pub struct WatchToggle {
    pub icon: Icon,
    pub pressed: bool,
    pub enabled: bool,
}

pub fn watch_toggle(state: &AppState) -> WatchToggle {
    WatchToggle {
        icon: if state.watch { Icon::Eye } else { Icon::EyeOff },
        pressed: state.watch,
        enabled: state.project.is_some() && state.opening.is_none(),
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
        crate::elements::icon_button("refresh-button", Icon::RefreshCw, 16., REFRESH, false)
            .track_focus(f)
            .off(!a.refresh)
            .on_click(move |_, _, cx| g.update(cx, |g, cx| g.refresh(cx)))
    });

    let g = grace.clone();
    let w = watch_toggle(state);
    let watch = tooltip("tip-watch", Some(WATCH_TIP.into())).focusable(move |f| {
        crate::elements::icon_button("watch-button", w.icon, 16., WATCH, false)
            .track_focus(f)
            .toggled(w.pressed)
            .when(w.pressed, |b| b.bg(Theme::color(t.selected)))
            .off(!w.enabled)
            .on_click(move |_, _, cx| g.update(cx, |g, cx| g.set_watch(!w.pressed, cx)))
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
        (Icon::SlidersHorizontal, ButtonKind::Plain, None),
        crate::properties::LABEL,
        a.properties,
        narrow,
        None,
        move |window, cx| crate::properties::open(&g, window, cx),
    );
    let g = grace.clone();
    let open = labelled(
        "open-button",
        (Icon::FolderOpen, ButtonKind::Plain, None),
        OPEN,
        a.open,
        narrow,
        None,
        move |_, cx| g.update(cx, |g, cx| g.open_project(cx)),
    );
    let g = grace.clone();
    let report = labelled(
        "report-button",
        (Icon::FileText, ButtonKind::Plain, None),
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
        (Icon::RotateCcw, ButtonKind::Plain, None),
        RERUN_FAILED,
        a.rerun_failed,
        narrow,
        pill,
        move |_, cx| g.update(cx, |g, cx| g.rerun_failed(cx)),
    );
    let g = grace.clone();
    let cancel = labelled(
        "cancel-button",
        (Icon::Square, ButtonKind::Plain, None),
        CANCEL,
        a.cancel,
        narrow,
        None,
        move |_, cx| g.update(cx, |g, cx| g.cancel(cx)),
    );
    let g = grace.clone();
    let run_all = div().debug_selector(|| "run-all-button".into()).child(
        icon_text_button(
            "run-all-button",
            Icon::Play,
            RUN_ALL,
            ButtonKind::Primary,
            ButtonSize::Medium,
        )
        .off(!a.run_all)
        .on_click(move |_, _, cx| g.update(cx, |g, cx| g.run_all(cx))),
    );

    // R6: Run all gives way to the debug controls while a debug run goes on; Cancel stays
    let debugging = state.run.active && state.run.debug;
    let controls: Vec<AnyElement> = match (debugging, state.paused.is_some()) {
        (false, _) => vec![run_all.into_any_element()],
        (true, false) => {
            let g = grace.clone();
            vec![
                labelled(
                    "debug-pause",
                    (Icon::Pause, ButtonKind::Plain, None),
                    PAUSE,
                    true,
                    narrow,
                    None,
                    move |_, cx| g.update(cx, |g, cx| g.debug_command(DebugCommand::Pause, cx)),
                )
                .into_any_element(),
            ]
        }
        (true, true) => DEBUG_STEPS
            .iter()
            .map(|s| {
                let (g, command) = (grace.clone(), s.command);
                let kind = if command == DebugCommand::Continue {
                    ButtonKind::Primary
                } else {
                    ButtonKind::Plain
                };
                labelled(
                    s.id,
                    (s.icon, kind, Some(s.shortcut)),
                    s.label,
                    true,
                    narrow,
                    None,
                    move |_, cx| g.update(cx, |g, cx| g.debug_command(command, cx)),
                )
                .into_any_element()
            })
            .collect(),
    };

    div()
        .id("toolbar")
        .debug_selector(|| "toolbar".into())
        // `<header>`: the page's banner landmark
        .role(Role::Banner)
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
        .child(div().debug_selector(|| "watch-button".into()).child(watch))
        .child(open)
        .child(divider("divider-run", cx))
        .child(report)
        .child(rerun)
        .child(cancel)
        .children(controls)
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
    (icon, kind, keys): (Icon, ButtonKind, Option<&'static str>),
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
                .map(|b| match kind {
                    ButtonKind::Primary => b.primary(),
                    _ => b.outline(),
                })
                .child(crate::elements::glyph(icon, 16.))
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
            icon_text_button(id, icon, label, kind, ButtonSize::Medium)
        };
        crate::a11y::aria(b, |a| match keys {
            Some(keys) => a.aria_keyshortcuts(keys),
            None => a,
        })
        .track_focus(f)
        .off(!enabled)
        .children(extra)
        .on_click(move |_, window, cx| press(window, cx))
    });
    div().debug_selector(move || id.into()).child(wrapped)
}

#[cfg(test)]
mod tests;
