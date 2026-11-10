//! Grace's window (`main.ts`'s `createWindow` and `App.tsx`'s frame): the ground, in the theme, from the first frame
//! (#90); the accessibility root and the live region that announces a run's start and end (guide R8.1); its bounds
//! saved and placed on the screens there are (window bounds spec); and the home screen or the project view.
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::accesskit::{Live, Role};
use gpui::{
    App, AppContext as _, Bounds, Context, Entity, InteractiveElement as _, IntoElement, ParentElement, Render,
    SharedString, StatefulInteractiveElement as _, Styled, Task, TitlebarOptions, Window, WindowBounds, WindowHandle,
    WindowOptions, div, point, prelude::FluentBuilder as _, px, size,
};
use gpui_component::Root;
use grace_core::settings::{WindowState, load_window_state, save_window_state};
use grace_core::window::{Rectangle, place_window};

use crate::a11y;
use crate::app::Grace;
use crate::probe;
use crate::theme::Theme;
use crate::ui::{NoticeKind, notice};

/// The window's smallest size: the toolbar fits down to 760 px wide, and 480 px leaves room for the bars,
/// 120 px of tests and 96 px of bottom panel (`src/main.ts`).
pub const MIN_SIZE: (f32, f32) = (760., 480.);
/// Its first size, before `settings.json` remembers one (window bounds spec).
pub const FIRST_SIZE: (f32, f32) = (1200., 800.);
/// A move or a resize is saved once the window has kept its bounds this long: Electron's `moved` and `resized` came at
/// the end of one, which GPUI does not tell.
const SAVE_AFTER: Duration = Duration::from_millis(500);

/// Grace's window: the home screen or the project view, on the `Grace` entity's state.
pub struct GraceWindow {
    pub grace: Option<Entity<Grace>>,
    /// What the live region says; a new text is announced.
    pub announcement: SharedString,
    tips: Option<Entity<crate::tooltip::Tips>>,
    /// Held by the window's root when nothing inside has the focus, so Tab reaches it (`a11y::tab_order`).
    pub focus: gpui::FocusHandle,
    /// Open folder…'s: removing a recent folder moves the focus there.
    pub open_focus: gpui::FocusHandle,
    /// The first frame was asked for: the probe's `first_frame` follows it (`crate::probe`).
    painted: bool,
    /// The pending save of the window's bounds; a newer move replaces it.
    save: Option<Task<()>>,
    /// The progress section, which keeps the run's clock.
    progress: Option<Entity<crate::progress::Progress>>,
    /// The tree, which keeps the rows a person opened and closed.
    pub tree: Option<Entity<crate::tree::Tree>>,
    pub filter_bar: Option<Entity<crate::filter_bar::FilterBar>>,
    pub env: Option<Entity<crate::env_picker::EnvPicker>>,
    pub threads: Option<Entity<crate::threads_box::ThreadsBox>>,
    /// The bottom panel, which keeps the console's search, its height and its tabs' order.
    pub panel: Option<Entity<crate::bottom_panel::BottomPanel>>,
    /// The selected test's or group's details, right of the tree.
    pub details: Option<Entity<crate::details::Details>>,
    /// The Settings page while it covers the view; made afresh each time it opens, as Electron Grace's was.
    pub settings: Option<Entity<crate::settings::SettingsPage>>,
    _motion: Option<gpui::Subscription>,
}

impl GraceWindow {
    pub fn new(cx: &mut App) -> Self {
        Self {
            grace: None,
            announcement: SharedString::default(),
            tips: None,
            focus: cx.focus_handle(),
            open_focus: cx.focus_handle(),
            painted: false,
            save: None,
            progress: None,
            tree: None,
            filter_bar: None,
            env: None,
            threads: None,
            panel: None,
            details: None,
            settings: None,
            _motion: None,
        }
    }

    /// The window on `grace`, redrawn when its state changes, its bounds saved in `data_dir`.
    pub fn with_grace(grace: Entity<Grace>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe(&grace, |_, _, cx| cx.notify()).detach();
        let data_dir = grace.read(cx).data_dir().to_path_buf();
        cx.observe_window_bounds(window, move |this: &mut Self, window, cx| {
            let state = window_state(window);
            let dir = data_dir.clone();
            this.save = Some(cx.spawn(async move |_, cx| {
                cx.background_executor().timer(SAVE_AFTER).await;
                save_bounds(&dir, &state);
            }));
        })
        .detach();
        let mut me = Self::new(cx);
        me.progress = Some(cx.new(|cx| crate::progress::Progress::new(grace.clone(), cx)));
        me.tree = Some(cx.new(|cx| crate::tree::Tree::new(grace.clone(), cx)));
        me.filter_bar = Some(cx.new(|cx| crate::filter_bar::FilterBar::new(grace.clone(), cx)));
        me.env = Some(cx.new(|cx| crate::env_picker::EnvPicker::new(grace.clone(), window, cx)));
        me.threads = Some(cx.new(|cx| crate::threads_box::ThreadsBox::new(grace.clone(), window, cx)));
        me.panel = Some(cx.new(|cx| crate::bottom_panel::BottomPanel::new(grace.clone(), window, cx)));
        me.details = Some(cx.new(|cx| crate::details::Details::new(grace.clone(), cx)));
        me._motion = Some(a11y::follow_reduced_motion(window, cx));
        me.grace = Some(grace);
        me
    }

    /// Covers the view with the Settings page (app-updates R6.1); the view's entities stay, so Back finds them as
    /// they were.
    pub fn open_settings(&mut self, cx: &mut Context<Self>) {
        let Some(grace) = self.grace.clone() else {
            return;
        };
        if self.settings.is_none() {
            let me = cx.entity().downgrade();
            self.settings = Some(cx.new(|cx| {
                crate::settings::SettingsPage::new(
                    grace,
                    move |window, cx| {
                        let _ = me.update(cx, |w, cx| w.close_settings(window, cx));
                    },
                    cx,
                )
            }));
            cx.notify();
        }
    }

    pub fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }
}

/// The window's normal bounds and whether it is maximized, as `main.ts` saved them: a minimized window is saved
/// normal, so it opens normal (WB-05).
pub fn window_state(window: &Window) -> WindowState {
    let maximized = window.is_maximized();
    let b = window.window_bounds().get_bounds();
    WindowState {
        x: f64::from(f32::from(b.origin.x)),
        y: f64::from(f32::from(b.origin.y)),
        width: f64::from(f32::from(b.size.width)),
        height: f64::from(f32::from(b.size.height)),
        maximized,
    }
}

/// Saves the bounds; a failed save is logged and never stops anything, as `main.ts` did (WB-06).
pub fn save_bounds(data_dir: &Path, state: &WindowState) {
    if let Err(e) = save_window_state(data_dir, state) {
        eprintln!("the window's bounds could not be saved: {e}");
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
        let tips = self
            .tips
            .get_or_insert_with(|| crate::tooltip::install(window, cx))
            .clone();
        let t = Theme::global(cx).tokens;
        // Ctrl+F and F8 work anywhere in the project view, the window's root included: the focus rests there after a
        // click on nothing, and a dialog keeps its own (console search R1, R3.1)
        let settings = self.settings.clone();
        let console = self
            .grace
            .as_ref()
            .filter(|g| settings.is_none() && g.read(cx).state.loaded && !g.read(cx).state.home())
            .and(self.panel.as_ref())
            .map(|p| p.read(cx).console.clone());
        let also = if settings.is_some() {
            Some(crate::settings::CONTEXT)
        } else {
            console.as_ref().map(|_| crate::console::CONTEXT)
        };
        let me = cx.entity().downgrade();
        let open_settings = move |_: &gpui::ClickEvent, _: &mut Window, cx: &mut App| {
            let _ = me.update(cx, |w, cx| w.open_settings(cx));
        };
        let view = self.grace.clone().map(|grace| {
            let state = grace.read(cx).state.clone();
            if !state.loaded {
                // the ground alone until the runner says what is open, so home never flashes (HS-05)
                return div().into_any_element();
            }
            let shown = state.shown_notice().map(|n| {
                notice(
                    "notice",
                    if n.success {
                        NoticeKind::Success
                    } else {
                        NoticeKind::Warning
                    },
                    n.text,
                    vec![],
                    cx,
                )
                .into_any_element()
            });
            let view = if let Some(page) = settings.clone() {
                page.into_any_element()
            } else if state.home() {
                crate::home::render(&grace, &state, shown, &self.open_focus, window, cx).into_any_element()
            } else {
                let boxes = [
                    self.env.clone().map(gpui::AnyView::from),
                    self.threads.clone().map(gpui::AnyView::from),
                ]
                .into_iter()
                .flatten()
                .map(|b| b.into_any_element())
                .collect();
                let toolbar = crate::toolbar::render(&grace, &state, boxes, window, cx).into_any_element();
                project_view(self, toolbar, shown, cx).into_any_element()
            };
            // the view cuts off at the status bar, so a short window never hides the bar
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(div().flex().flex_col().flex_1().min_h_0().overflow_hidden().child(view))
                .child(crate::status_bar::render(&state, open_settings, cx))
                .into_any_element()
        });
        a11y::tab_order(div().id("grace"), &self.focus, also)
            .role(Role::Application)
            .aria_label("Grace")
            .size_full()
            .flex()
            .flex_col()
            .bg(Theme::color(t.ground))
            .text_color(Theme::color(t.ink))
            .text_size(px(crate::theme::METRICS.text_sm.size))
            .when_some(settings, |d, page| {
                d.on_action(move |_: &crate::settings::GoBack, window, cx| {
                    let back = page.read(cx).back();
                    back(window, cx)
                })
            })
            .when_some(console, |d, c| {
                let (find, next, prev) = (c.clone(), c.clone(), c);
                d.on_action(move |_: &crate::console::FindInConsole, w, cx| find.update(cx, |c, cx| c.open_bar(w, cx)))
                    .on_action(move |_: &crate::console::NextError, _, cx| {
                        next.update(cx, |c, cx| c.step_error(true, cx))
                    })
                    .on_action(move |_: &crate::console::PreviousError, _, cx| {
                        prev.update(cx, |c, cx| c.step_error(false, cx))
                    })
            })
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
            .children(view)
            .child(tips)
    }
}

/// The project view: the toolbar, then the sheet (spec R3.3) with the progress section, the notice, the filter bar,
/// then the tree and the details side by side above the bottom panel (P2
/// plan).
fn project_view(
    window: &GraceWindow,
    toolbar: gpui::AnyElement,
    shown: Option<gpui::AnyElement>,
    cx: &App,
) -> impl IntoElement {
    let (progress, filter_bar, tree, panel, details) = (
        window.progress.clone(),
        window.filter_bar.clone(),
        window.tree.clone(),
        window.panel.clone(),
        window.details.clone(),
    );
    let t = Theme::global(cx).tokens;
    let m = crate::theme::METRICS;
    let room = panel.as_ref().map(|p| p.read(cx).room.clone());
    div()
        .id("project-view")
        .debug_selector(|| "project-view".into())
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(toolbar)
        .child(
            div()
                .id("sheet")
                .debug_selector(|| "sheet".into())
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .mx(px(m.sheet_inset))
                .mb(px(m.sheet_inset))
                .overflow_hidden()
                .rounded(px(m.sheet_radius))
                .border_t_1()
                .border_color(Theme::color(t.line))
                .bg(Theme::color(t.surface))
                .children(progress)
                .children(shown.map(|n| div().px_3().py_2().child(n)))
                .children(filter_bar)
                .child(
                    // the main area and the bottom panel share what is left; the panel's limits are read from it
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h_0()
                        .children(room.map(|room| {
                            gpui::canvas(move |b, _, _| room.set(f32::from(b.size.height)), |_, _, _, _| {})
                                .absolute()
                                .size_full()
                        }))
                        .child(
                            div()
                                .id("main")
                                // `<main>`: the main landmark
                                .role(Role::Main)
                                .debug_selector(|| "main".into())
                                .flex()
                                .flex_1()
                                .min_h(px(grace_core::panel_height::MAIN_MIN))
                                .child(div().flex().flex_col().w_1_2().min_h_0().children(tree))
                                .child(div().flex().flex_col().w_1_2().min_h_0().children(details)),
                        )
                        .children(panel),
                ),
        )
}

/// Where the window opens: the saved bounds placed on the screens there are now, maximized when saved so; else
/// 1200 by 800 in the middle of the main screen, as on a first start (WB-09, WB-12, WB-13).
pub fn bounds(saved: Option<WindowState>, cx: &App) -> WindowBounds {
    let areas: Vec<Rectangle> = cx
        .displays()
        .iter()
        .map(|d| {
            let b = d.visible_bounds();
            Rectangle::new(
                f64::from(f32::from(b.origin.x)),
                f64::from(f32::from(b.origin.y)),
                f64::from(f32::from(b.size.width)),
                f64::from(f32::from(b.size.height)),
            )
        })
        .collect();
    let placed = saved.and_then(|s| {
        place_window(Rectangle::new(s.x, s.y, s.width, s.height), &areas).map(|r| {
            Bounds::new(
                point(px(r.x as f32), px(r.y as f32)),
                size(
                    px((r.width as f32).max(MIN_SIZE.0)),
                    px((r.height as f32).max(MIN_SIZE.1)),
                ),
            )
        })
    });
    let first = || Bounds::centered(e2e_display(cx), size(px(FIRST_SIZE.0), px(FIRST_SIZE.1)), cx);
    match (placed, saved.is_some_and(|s| s.maximized)) {
        (Some(b), true) => WindowBounds::Maximized(b),
        (Some(b), false) => WindowBounds::Windowed(b),
        // a maximized window whose screen is gone opens maximized on the main screen (WB-13)
        (None, true) => WindowBounds::Maximized(first()),
        (None, false) => WindowBounds::Windowed(first()),
    }
}

/// The screen a window with these bounds opens on: the one that holds their middle, or else the main one (or the
/// e2e screen). Windows' GPUI opens a window with no screen named on the main one and moves bounds that lie on
/// another there, so a window saved on a second screen came back on the main one (WB-15).
pub fn display_for(bounds: &WindowBounds, cx: &App) -> Option<gpui::DisplayId> {
    let middle = bounds.get_bounds().center();
    cx.displays()
        .into_iter()
        .find(|d| d.bounds().contains(&middle))
        .map(|d| d.id())
        .or_else(|| e2e_display(cx))
}

pub fn options(saved: Option<WindowState>, cx: &App) -> WindowOptions {
    let window_bounds = bounds(saved, cx);
    WindowOptions {
        display_id: display_for(&window_bounds, cx),
        titlebar: Some(TitlebarOptions {
            title: Some("Grace".into()),
            ..Default::default()
        }),
        window_bounds: Some(window_bounds),
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

/// Opens Grace's window on `grace`: GPUI Kit's `Root` (dialogs, tooltips, text selection) around it, the theme
/// following the saved Appearance and the system's. Closing it saves its bounds, cancels a run and waits for Java to
/// end, then quits (WS-09, WB-03).
pub fn open(grace: Entity<Grace>, data_dir: PathBuf, cx: &mut App) -> WindowHandle<Root> {
    let saved = load_window_state(&data_dir);
    #[cfg(feature = "e2e")]
    crate::e2e::record_screen(&data_dir, cx);
    cx.open_window(options(saved, cx), move |window, cx| {
        crate::theme::follow_system(window, cx);
        #[cfg(feature = "e2e")]
        if crate::e2e::wants_gallery() {
            let gallery = cx.new(|cx| crate::e2e::Gallery::new(cx));
            gallery.read(cx).focus.clone().focus(window, cx);
            return cx.new(|cx| Root::new(gallery, window, cx));
        }
        let closing = grace.clone();
        let dir = data_dir.clone();
        window.on_window_should_close(cx, move |window, cx| {
            save_bounds(&dir, &window_state(window));
            let dispose = closing.read(cx).dispose(cx);
            cx.spawn(async move |cx| {
                dispose.await;
                cx.update(|cx| cx.quit());
            })
            .detach();
            false
        });
        let view = cx.new(|cx| GraceWindow::with_grace(grace, window, cx));
        view.read(cx).focus.clone().focus(window, cx);
        cx.new(|cx| Root::new(view, window, cx))
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
        let window = cx.add_window(|_, cx| GraceWindow::new(cx));
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

    fn state(x: f64, y: f64, width: f64, height: f64, maximized: bool) -> WindowState {
        WindowState {
            x,
            y,
            width,
            height,
            maximized,
        }
    }

    /// `Home.test.tsx`: the project view sits on the sheet under the toolbar, the status bar below; home has no
    /// sheet. The progress section is the sheet's first part.
    #[gpui::test]
    fn the_project_view_sits_on_the_sheet_under_the_toolbar(cx: &mut TestAppContext) {
        use grace_core::model::node::Layout;
        let (h, _) = crate::testing::open(
            cx,
            1200.,
            crate::testing::project("D:\\api", Layout::Maven, 2),
            |_, _, _| div().into_any_element(),
        );
        let grace = h.grace.clone();
        let window = cx.add_window(move |window, cx| GraceWindow::with_grace(grace, window, cx));
        let mut vcx = gpui::VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(size(px(1200.), px(700.)));
        vcx.run_until_parked();
        let toolbar = vcx.debug_bounds("toolbar").unwrap();
        let sheet = vcx.debug_bounds("sheet").unwrap();
        let progress = vcx.debug_bounds("progress").unwrap();
        let status = vcx.debug_bounds("status-bar").unwrap();
        assert!(toolbar.bottom() <= sheet.top());
        assert_eq!(progress.top(), sheet.top() + px(1.)); // under the sheet's top border
        assert!(sheet.bottom() <= status.top());
        assert_eq!(status.bottom(), px(700.));
        assert!(vcx.debug_bounds("home").is_none());
    }

    /// The whole window drawn on `state`, 1200 × 700.
    fn whole_window(cx: &mut TestAppContext, state: grace_core::state::AppState) -> gpui::VisualTestContext {
        let (h, _) = crate::testing::open(cx, 1200., state, |_, _, _| div().into_any_element());
        let grace = h.grace.clone();
        let window = cx.add_window(move |window, cx| GraceWindow::with_grace(grace, window, cx));
        let vcx = gpui::VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(size(px(1200.), px(700.)));
        vcx.run_until_parked();
        vcx
    }

    /// `Home.test.tsx` "home sits on the ground, with no sheet" and "the app root is ground and ink": with no project,
    /// the window draws home alone, with no toolbar and no sheet.
    #[gpui::test]
    fn home_is_drawn_alone_with_no_toolbar_and_no_sheet(cx: &mut TestAppContext) {
        let mut state = grace_core::state::AppState::new();
        state.loaded = true;
        state.recent = Some(vec![]);
        let mut vcx = whole_window(cx, state);
        assert!(vcx.debug_bounds("home").is_some());
        assert!(vcx.debug_bounds("sheet").is_none());
        assert!(vcx.debug_bounds("toolbar").is_none());
        assert!(
            vcx.debug_bounds("status-bar").is_some(),
            "the status bar is on every view"
        );
    }

    /// `Home.test.tsx` "the project view sits on the sheet under the toolbar": the notice, the main area and the
    /// bottom panel are on the sheet; the toolbar is not.
    #[gpui::test]
    fn the_notice_the_main_area_and_the_panel_are_on_the_sheet(cx: &mut TestAppContext) {
        use grace_core::model::node::Layout;
        let mut state = crate::testing::project("D:\\api", Layout::Maven, 2);
        state.notice = Some(grace_core::state::NoticeView {
            text: "Nothing to run".into(),
            success: false,
            build: false,
        });
        let mut vcx = whole_window(cx, state);
        let sheet = vcx.debug_bounds("sheet").unwrap();
        for part in ["notice", "main", "bottom-panel"] {
            let b = vcx.debug_bounds(part).unwrap_or_else(|| panic!("{part}"));
            assert!(
                sheet.contains(&b.origin) && b.bottom() <= sheet.bottom(),
                "{part} {b:?} on the sheet {sheet:?}"
            );
        }
        assert!(vcx.debug_bounds("toolbar").unwrap().bottom() <= sheet.top());
    }

    /// `Toolbar.test.tsx` "keeps its items in order" and "the Env and Threads boxes are the controls' height": Home,
    /// the project, Env, Threads, Properties…, Refresh, Open…, Report, Re-run failed, Cancel, Run all, left to right.
    #[gpui::test]
    fn the_toolbar_keeps_its_items_in_order_and_its_boxes_at_the_controls_height(cx: &mut TestAppContext) {
        use grace_core::model::node::Layout;
        let mut vcx = whole_window(cx, crate::testing::project("D:\\api", Layout::Maven, 2));
        let order = [
            "home-button",
            "project",
            "env-input",
            "threads-input",
            "properties-button",
            "refresh-button",
            "open-button",
            "report-button",
            "rerun-failed-button",
            "cancel-button",
            "run-all-button",
        ];
        let xs: Vec<_> = order
            .iter()
            .map(|id| vcx.debug_bounds(id).unwrap_or_else(|| panic!("{id}")).left())
            .collect();
        for (pair, x) in order.windows(2).zip(xs.windows(2)) {
            assert!(x[0] < x[1], "{} before {}: {:?}", pair[0], pair[1], x);
        }
        for id in ["env-input", "threads-input"] {
            let h = vcx.debug_bounds(id).unwrap().size.height;
            assert_eq!(h, px(crate::theme::METRICS.control_height), "{id}");
        }
    }

    /// The whole window on a project whose folder sits deep, more than 150 characters in (`open.spec.ts`).
    fn long_path_window(cx: &mut TestAppContext, width: f32) -> gpui::VisualTestContext {
        use grace_core::model::node::Layout;
        let path = "C:\\Users\\someone\\OneDrive - Acme Corporation\\Documents\\source\\repos\\platform\\services\\\
                    payments-gateway\\integration-tests\\payments-gateway-contract-tests";
        assert!(path.len() > 150);
        let (h, _) = crate::testing::open(
            cx,
            width,
            crate::testing::project(path, Layout::Standalone, 3),
            |_, _, _| div().into_any_element(),
        );
        let grace = h.grace.clone();
        let window = cx.add_window(move |window, cx| GraceWindow::with_grace(grace, window, cx));
        let vcx = gpui::VisualTestContext::from_window(window.into(), cx);
        vcx.simulate_resize(size(px(width), px(700.)));
        vcx.run_until_parked();
        vcx
    }

    /// TB-04: at the default 1200 px the chip stays on the name's line, so neither is cut (the chip goes to the
    /// hidden second line before the name is cut, R1.2), and both end before the Env box.
    #[gpui::test]
    fn the_name_shows_whole_at_the_default_size(cx: &mut TestAppContext) {
        let mut vcx = long_path_window(cx, FIRST_SIZE.0);
        let project = vcx.debug_bounds("project").unwrap();
        let name = vcx.debug_bounds("project-name").unwrap();
        let chip = vcx.debug_bounds("project-layout").unwrap();
        let env = vcx.debug_bounds("env-input").unwrap();
        assert!(
            chip.top() < project.bottom(),
            "the chip on the name's line: {chip:?} {project:?}"
        );
        assert!(name.right() <= chip.left());
        assert!(chip.right() <= env.left());
    }

    /// TB-05: at the 760 px minimum every toolbar button is inside the window, the name's end shows before the Env
    /// box, the chip is whole or out of sight, and the status bar's path ends before its button.
    #[gpui::test]
    fn a_long_path_at_the_minimum_width(cx: &mut TestAppContext) {
        let mut vcx = long_path_window(cx, MIN_SIZE.0);
        for id in [
            "home-button",
            "refresh-button",
            "open-button",
            "report-button",
            "rerun-failed-button",
            "cancel-button",
            "run-all-button",
        ] {
            let b = vcx.debug_bounds(id).unwrap_or_else(|| panic!("{id}"));
            assert!(b.left() >= px(0.) && b.right() <= px(MIN_SIZE.0), "{id} at {b:?}");
        }
        let project = vcx.debug_bounds("project").unwrap();
        let name = vcx.debug_bounds("project-name").unwrap();
        let env = vcx.debug_bounds("env-input").unwrap();
        assert!(name.size.width > px(40.), "the name's last 12 characters: {name:?}");
        assert!(name.right() <= env.left());
        let chip = vcx.debug_bounds("project-layout").unwrap();
        assert!(
            chip.top() >= project.bottom() || chip.right() <= env.left(),
            "the chip whole or out of sight: {chip:?}"
        );
        let path = vcx.debug_bounds("project-path").unwrap();
        let settings = vcx.debug_bounds("settings-button").unwrap();
        assert!(
            path.left() >= px(0.) && path.right() <= settings.left(),
            "{path:?} {settings:?}"
        );
    }

    #[gpui::test]
    fn a_broken_or_missing_state_opens_as_on_a_first_start(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let first = bounds(None, cx);
            assert!(matches!(first, WindowBounds::Windowed(_)));
            assert_eq!(first.get_bounds().size, size(px(FIRST_SIZE.0), px(FIRST_SIZE.1)));
        });
    }

    #[gpui::test]
    fn the_window_comes_back_where_it_was_and_maximized_when_it_was(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let area = cx.displays()[0].visible_bounds();
            let x = f64::from(f32::from(area.origin.x)) + 50.;
            let y = f64::from(f32::from(area.origin.y)) + 40.;
            let normal = bounds(Some(state(x, y, 900., 600., false)), cx);
            assert!(matches!(normal, WindowBounds::Windowed(_)));
            assert_eq!(
                normal.get_bounds(),
                Bounds::new(point(px(x as f32), px(y as f32)), size(px(900.), px(600.)))
            );
            let max = bounds(Some(state(x, y, 900., 600., true)), cx);
            assert!(matches!(max, WindowBounds::Maximized(_)));
            assert_eq!(max.get_bounds().size, size(px(900.), px(600.)));
        });
    }

    #[gpui::test]
    fn the_window_opens_on_the_screen_that_holds_its_saved_bounds(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let screen = &cx.displays()[0];
            let area = screen.visible_bounds();
            let (x, y) = (
                f64::from(f32::from(area.origin.x)) + 50.,
                f64::from(f32::from(area.origin.y)) + 40.,
            );
            let opts = options(Some(state(x, y, 900., 600., false)), cx);
            assert_eq!(
                opts.display_id,
                Some(screen.id()),
                "the saved screen is named, not left to the platform"
            );
            assert_eq!(
                options(None, cx).display_id,
                Some(screen.id()),
                "a first start on the main screen"
            );
        });
    }

    #[gpui::test]
    fn a_short_saved_window_is_held_at_the_minimum_height(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let area = cx.displays()[0].visible_bounds();
            let (x, y) = (f64::from(f32::from(area.origin.x)), f64::from(f32::from(area.origin.y)));
            let b = bounds(Some(state(x, y, 900., 200., false)), cx).get_bounds();
            assert_eq!(b.size.height, px(MIN_SIZE.1));
        });
    }

    #[gpui::test]
    fn a_window_on_a_screen_that_is_gone_opens_on_the_main_screen(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let gone = bounds(Some(state(-50_000., -50_000., 900., 600., false)), cx);
            assert!(matches!(gone, WindowBounds::Windowed(_)));
            assert_eq!(gone.get_bounds().size, size(px(FIRST_SIZE.0), px(FIRST_SIZE.1)));
            let gone_max = bounds(Some(state(-50_000., -50_000., 900., 600., true)), cx);
            assert!(matches!(gone_max, WindowBounds::Maximized(_)));
        });
    }
}
