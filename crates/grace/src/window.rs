//! Grace's window. In phase P0 it is empty: the ground, in the theme, from the first frame (#90), with the
//! accessibility root and the live region that announces a run's start and end (guide R8.1).
use gpui::accesskit::{Live, Role};
use gpui::{
    App, AppContext as _, Bounds, Context, InteractiveElement as _, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, TitlebarOptions, Window, WindowBounds, WindowHandle, WindowOptions, div,
    prelude::FluentBuilder as _, px, size,
};
use gpui_component::Root;

use crate::a11y;
use crate::probe;
use crate::theme::Theme;

/// The window's smallest size: the toolbar fits down to 760 px wide, and 480 px leaves room for the bars,
/// 120 px of tests and 96 px of bottom panel (`src/main.ts`).
pub const MIN_SIZE: (f32, f32) = (760., 480.);
/// Its first size, before `window.json` remembers one (window bounds spec; phase P2).
pub const FIRST_SIZE: (f32, f32) = (1200., 800.);

/// The empty Grace window.
pub struct GraceWindow {
    /// What the live region says; a new text is announced.
    pub announcement: SharedString,
    /// The first frame was asked for: the probe's `first_frame` follows it (`crate::probe`).
    painted: bool,
}

impl GraceWindow {
    pub fn new() -> Self {
        Self {
            announcement: SharedString::default(),
            painted: false,
        }
    }
}

impl Default for GraceWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for GraceWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.painted {
            self.painted = true;
            if probe::on() {
                window.on_next_frame(|_, _| probe::mark("first_frame"));
            }
        }
        let t = Theme::global(cx).tokens;
        div()
            .id("grace")
            .role(Role::Application)
            .aria_label("Grace")
            .size_full()
            .bg(Theme::color(t.ground))
            .text_color(Theme::color(t.ink))
            .child(a11y::live(
                div()
                    .id("announcer")
                    .role(Role::Status)
                    .size_0()
                    .when(!self.announcement.is_empty(), |d| {
                        d.aria_label(self.announcement.clone())
                    }),
                Live::Polite,
            ))
    }
}

pub fn options(cx: &App) -> WindowOptions {
    WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Grace".into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            e2e_display(cx),
            size(px(FIRST_SIZE.0), px(FIRST_SIZE.1)),
            cx,
        ))),
        window_min_size: Some(size(px(MIN_SIZE.0), px(MIN_SIZE.1))),
        app_id: Some("Grace".into()),
        ..Default::default()
    }
}

/// The screen an end-to-end test's window opens on: with `GRACE_E2E_SCREEN=other`, the first screen that is not
/// the main one, out of the way of the person working, as `e2e/helpers.ts` `launch()` does. Only in a build with
/// the `e2e` feature; the release build always opens on the main screen.
#[cfg(feature = "e2e")]
fn e2e_display(cx: &App) -> Option<gpui::DisplayId> {
    if std::env::var("GRACE_E2E_SCREEN").as_deref() != Ok("other") {
        return None;
    }
    let main = cx.primary_display().map(|d| d.id());
    cx.displays().into_iter().map(|d| d.id()).find(|id| Some(*id) != main)
}

#[cfg(not(feature = "e2e"))]
fn e2e_display(_cx: &App) -> Option<gpui::DisplayId> {
    None
}

/// Opens Grace's window: GPUI Kit's `Root` (dialogs, tooltips, text selection) around it, the theme following
/// the system's.
pub fn open(cx: &mut App) -> WindowHandle<Root> {
    cx.open_window(options(cx), |window, cx| {
        crate::theme::follow_system(window, cx);
        #[cfg(feature = "e2e")]
        if crate::e2e::wants_gallery() {
            let gallery = cx.new(|_| crate::e2e::Gallery::default());
            return cx.new(|cx| Root::new(gallery, window, cx));
        }
        let grace = cx.new(|_| GraceWindow::new());
        cx.new(|cx| Root::new(grace, window, cx))
    })
    .expect("the window opens")
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn opens_an_empty_grace_window_in_the_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            crate::init(cx);
            crate::theme::apply(true, None, cx);
        });
        let window = cx.add_window(|_, _| GraceWindow::new());
        cx.run_until_parked();
        window
            .update(cx, |grace, _, cx| {
                assert!(grace.announcement.is_empty());
                assert!(Theme::global(cx).dark);
                assert_eq!(
                    gpui_component::Theme::global(cx).colors.background,
                    Theme::color(Theme::global(cx).tokens.surface)
                );
            })
            .unwrap();
    }
}
