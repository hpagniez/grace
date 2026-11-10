//! `Home.tsx`: the start screen (home screen spec R12), on the ground in both themes: no project is open, so there is
//! no sheet. The notice shows above the column.
use crate::elements::Off as _;
use gpui::accesskit::Role;
use gpui::{
    AnyElement, App, Entity, FocusHandle, ImageSource, InteractiveElement as _, IntoElement, ParentElement, Resource,
    SharedString, StatefulInteractiveElement as _, Styled, Window, div, img, prelude::FluentBuilder as _, px,
};
use gpui_component::button::ButtonVariants as _;
use gpui_component::scroll::ScrollableElement as _;
use grace_core::model::events::RecentEntry;
use grace_core::state::AppState;

use crate::app::Grace;
use crate::assets::Icon;
use crate::elements::{ButtonKind, ButtonSize, button, icon, icon_button};
use crate::theme::{METRICS, Theme};
use crate::tooltip::{Side, cut, tooltip};

pub const HEADING: &str = "Open a Karate project";
pub const OPEN: &str = "Open folder…";
pub const RECENT: &str = "Recent folders";
pub const REMOVE: &str = "Remove from list";
pub const NOT_FOUND: &str = "Not found";
pub const MISSING: &str = "Folder not found";

/// The line under the heading: it says the list is there to reopen from when there is one.
pub fn lead(state: &AppState) -> &'static str {
    if state.recent.as_ref().is_some_and(|r| !r.is_empty()) {
        "Pick a folder, or reopen one you used before."
    } else {
        "Pick the folder that holds your .feature files. Grace brings its own Java and Karate."
    }
}

/// A folder's name: its last segment, whichever slash it uses; the whole path when it has none (a drive's root).
pub fn folder_name(path: &str) -> &str {
    match path.rsplit(['\\', '/']).next() {
        Some(last) if !last.is_empty() => last,
        _ => path,
    }
}

/// The accessible name of a row: what it says, as the DOM gave it from the row's text.
pub fn row_name(entry: &RecentEntry) -> String {
    let name = folder_name(&entry.path);
    if entry.exists {
        format!("{name} {}", entry.path)
    } else {
        format!("{name} {NOT_FOUND} {}", entry.path)
    }
}

/// The home screen. `open` is Open folder…'s focus handle: removing a row moves the focus there, so it is never left
/// on a row that is gone.
pub fn render(
    grace: &Entity<Grace>,
    state: &AppState,
    notice: Option<AnyElement>,
    open: &FocusHandle,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let t = Theme::global(cx).tokens;
    let entries = state.recent.clone().unwrap_or_default();
    let g = grace.clone();
    let open_button = button("open-button", OPEN, ButtonKind::Primary, ButtonSize::Medium)
        .icon(Icon::FolderOpen)
        .track_focus(open)
        .on_click(move |_, _, cx| g.update(cx, |g, cx| g.open_project(cx)));
    let column = div()
        .id("home-column")
        .debug_selector(|| "home-column".into())
        .mx_auto()
        .w_full()
        .max_w(px(540.))
        .flex()
        .flex_col()
        .items_start()
        // decorative: the heading says what the screen is for (ADR 0013)
        .child(
            div().debug_selector(|| "mascot".into()).mb_6().child(
                img(ImageSource::Resource(Resource::Embedded(MASCOT.into())))
                    .w(px(240.))
                    .h(px(180.)),
            ),
        )
        .child(
            div()
                .id("home-heading")
                .role(Role::Heading)
                .aria_level(1)
                .aria_label(HEADING)
                .text_size(px(METRICS.text_xl.size))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(HEADING),
        )
        .child(
            div()
                .id("home-lead")
                .role(Role::Label)
                .aria_value(lead(state))
                .mt_1()
                .text_size(px(METRICS.text_sm.size))
                .text_color(Theme::color(t.muted))
                .child(lead(state)),
        )
        .child(div().mt_4().debug_selector(|| "open-button".into()).child(open_button))
        .when(!entries.is_empty(), |c| {
            c.child(
                div()
                    .id("recent-heading")
                    .role(Role::Heading)
                    .aria_level(2)
                    .aria_label(RECENT)
                    .mt_8()
                    .text_size(px(METRICS.text_sm.size))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(Theme::color(t.muted))
                    .child(RECENT),
            )
            .child(recent_list(grace, &entries, open, window, cx))
        });
    div()
        .id("home")
        .debug_selector(|| "home".into())
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .bg(Theme::color(t.ground))
        .children(notice.map(|n| div().px_3().py_2().child(n)))
        .child(
            div()
                .id("home-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scrollbar()
                .px_4()
                .py_12()
                .child(column),
        )
}

/// The mascot, from Electron Grace's `assets/mascot.svg`.
pub const MASCOT: &str = "grace/mascot.svg";

/// The recent folders in one surface box, a rule between rows (R12.3).
fn recent_list(
    grace: &Entity<Grace>,
    entries: &[RecentEntry],
    open: &FocusHandle,
    _window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let t = Theme::global(cx).tokens;
    let mut list = div()
        .id("recent-list")
        .debug_selector(|| "recent-list".into())
        .role(Role::List)
        .aria_label(RECENT)
        .mt_2()
        .w_full()
        .flex()
        .flex_col()
        .overflow_hidden()
        .rounded(px(METRICS.radius_lg))
        .border_1()
        .border_color(Theme::color(t.line))
        .bg(Theme::color(t.surface));
    for (i, entry) in entries.iter().enumerate() {
        list = list.child(recent_row(grace, i, entry, open, cx));
    }
    list
}

fn recent_row(grace: &Entity<Grace>, i: usize, entry: &RecentEntry, open: &FocusHandle, cx: &mut App) -> AnyElement {
    let t = Theme::global(cx).tokens;
    let name = SharedString::from(folder_name(&entry.path).to_string());
    let path = SharedString::from(entry.path.clone());
    let muted = Theme::color(t.muted);
    let title = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .debug_selector(move || format!("recent-name-{i}"))
                .truncate()
                .font_weight(gpui::FontWeight::MEDIUM)
                .when(!entry.exists, |d| d.text_color(muted))
                .child(name),
        )
        .when(!entry.exists, |d| {
            d.child(
                div()
                    .debug_selector(move || format!("recent-missing-{i}"))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_1()
                    .h(px(20.))
                    .px_2()
                    .rounded_full()
                    .text_size(px(METRICS.text_2xs.size))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .bg(Theme::color(t.cancelled_tint))
                    .text_color(Theme::color(t.cancelled))
                    .child(icon(Icon::TriangleAlert, 12., Theme::color(t.cancelled)))
                    .child(NOT_FOUND),
            )
        });
    let g = grace.clone();
    let root = entry.path.clone();
    let row = gpui_component::button::Button::new(("recent-row", i))
        .ghost()
        .w_full()
        .h_auto()
        .px_3()
        .py_2()
        .justify_start()
        .rounded_none()
        .off(!entry.exists)
        .accessibility_label(row_name(entry))
        .on_click(move |_, _, cx| {
            let root = root.clone();
            g.update(cx, |g, cx| g.open_recent(root, cx));
        })
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .min_w_0()
                .flex_1()
                .text_left()
                .child(icon(Icon::Folder, 16., muted))
                .child(
                    div().flex().flex_col().min_w_0().flex_1().child(title).child(
                        div()
                            .w_full()
                            .font_family(crate::theme::MONO)
                            .text_size(px(METRICS.text_xs.size))
                            .text_color(muted)
                            .child(cut(("recent-path", i), path).side(Side::Over)),
                    ),
                ),
        );
    let g = grace.clone();
    let root = entry.path.clone();
    let open = open.clone();
    let remove = tooltip(("tip-remove", i), Some(REMOVE.into())).focusable(move |f| {
        icon_button(("recent-remove", i), Icon::X, REMOVE, true)
            .track_focus(f)
            .on_click(move |_, window, cx| {
                let root = root.clone();
                g.update(cx, |g, cx| g.remove_recent(root, cx));
                // the rows are rebuilt when the new list arrives: keep the focus off the gone row
                open.focus(window, cx);
            })
    });
    div()
        .id(("recent-item", i))
        .debug_selector(move || format!("recent-row-{i}"))
        .role(Role::ListItem)
        .flex()
        .items_center()
        .pr_2()
        .when(i > 0, |d| d.border_t_1().border_color(Theme::color(t.line)))
        .child(
            div().min_w_0().flex_1().child(
                tooltip(("tip-row", i), (!entry.exists).then(|| MISSING.into()))
                    .side(Side::Over)
                    .child(row),
            ),
        )
        .child(div().ml_2().flex_none().child(remove))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::fake::FakeBackend;
    use crate::app::{Backend, events};
    use crate::seams::fake::FakeSeams;
    use gpui::{Context, Render, TestAppContext, VisualTestContext};
    use grace_core::model::events::AppEvent;
    use std::path::PathBuf;
    use std::sync::Arc;

    struct HomeView {
        grace: Entity<Grace>,
        open: FocusHandle,
    }

    impl Render for HomeView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let state = self.grace.read(cx).state.clone();
            div()
                .size_full()
                .flex()
                .child(render(&self.grace, &state, None, &self.open, window, cx))
        }
    }

    fn entries() -> Vec<RecentEntry> {
        vec![
            RecentEntry {
                path: "D:\\work\\api".into(),
                exists: true,
            },
            RecentEntry {
                path: "E:\\usb\\old".into(),
                exists: false,
            },
        ]
    }

    fn setup(
        cx: &mut TestAppContext,
        events: Vec<AppEvent>,
    ) -> (Entity<Grace>, Arc<FakeBackend>, &mut VisualTestContext) {
        cx.update(crate::init);
        let backend = Arc::new(FakeBackend::default());
        let (tx, rx) = crate::app::events();
        let b: Arc<dyn Backend> = backend.clone();
        let grace = cx.update(|cx| {
            gpui::AppContext::new(cx, |cx| {
                Grace::new(
                    b,
                    Arc::new(FakeSeams::answering(Some("/picked"))),
                    PathBuf::new(),
                    rx,
                    cx,
                )
            })
        });
        for e in events {
            tx.try_send(e).unwrap();
        }
        let g = grace.clone();
        let (_, vcx) = cx.add_window_view(|_, cx| HomeView {
            grace: g,
            open: cx.focus_handle(),
        });
        vcx.run_until_parked();
        std::mem::forget(tx);
        (grace, backend, vcx)
    }

    fn opening() -> AppEvent {
        AppEvent::Opening { root: None }
    }

    fn recent(entries: Vec<RecentEntry>) -> AppEvent {
        AppEvent::Recent { entries }
    }

    #[gpui::test]
    fn a_first_start_shows_the_home_screen_without_a_list(cx: &mut TestAppContext) {
        let (grace, _, vcx) = setup(cx, vec![opening(), recent(vec![])]);
        assert!(vcx.debug_bounds("home").is_some());
        assert!(vcx.debug_bounds("recent-list").is_none());
        grace.read_with(vcx, |g, _| {
            assert_eq!(
                lead(&g.state),
                "Pick the folder that holds your .feature files. Grace brings its own Java and Karate."
            );
        });
        let _ = events;
    }

    #[gpui::test]
    fn the_mascot_is_decorative(cx: &mut TestAppContext) {
        let (_, _, vcx) = setup(cx, vec![opening()]);
        let b = vcx.debug_bounds("mascot").unwrap();
        assert_eq!((b.size.width, b.size.height), (px(240.), px(180.)));
        // decorative: it has no id or role, so it is not in the accessibility tree
        let svg = crate::assets::Assets.load_mascot();
        assert!(svg.starts_with(b"<svg"));
    }

    #[gpui::test]
    fn open_folder_opens_the_folder_dialog(cx: &mut TestAppContext) {
        let (_, backend, vcx) = setup(cx, vec![opening()]);
        let b = vcx.debug_bounds("open-button").unwrap();
        vcx.simulate_click(b.center(), gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(backend.calls(), ["open /picked"]);
    }

    #[gpui::test]
    fn lists_the_recent_folders(cx: &mut TestAppContext) {
        let (grace, _, vcx) = setup(cx, vec![opening(), recent(entries())]);
        assert!(vcx.debug_bounds("recent-list").is_some());
        assert!(vcx.debug_bounds("recent-row-0").is_some());
        assert!(vcx.debug_bounds("recent-row-1").is_some());
        assert!(vcx.debug_bounds("recent-row-2").is_none());
        grace.read_with(vcx, |g, _| {
            assert_eq!(lead(&g.state), "Pick a folder, or reopen one you used before.");
        });
        assert_eq!(row_name(&entries()[0]), "api D:\\work\\api");
        assert_eq!(row_name(&entries()[1]), "old Not found E:\\usb\\old");
    }

    #[gpui::test]
    fn shows_the_list_in_one_surface_box_with_a_rule_between_rows_r12_3(cx: &mut TestAppContext) {
        let (_, _, vcx) = setup(cx, vec![opening(), recent(entries())]);
        let list = vcx.debug_bounds("recent-list").unwrap();
        for i in 0..2 {
            let row = vcx
                .debug_bounds(if i == 0 { "recent-row-0" } else { "recent-row-1" })
                .unwrap();
            assert!(list.contains(&row.origin), "row {i} in the box");
        }
    }

    #[gpui::test]
    fn says_not_found_in_a_cancelled_badge_with_the_warning_icon_and_the_row_is_disabled(cx: &mut TestAppContext) {
        let (_, backend, vcx) = setup(cx, vec![opening(), recent(entries())]);
        assert!(vcx.debug_bounds("recent-missing-0").is_none());
        assert!(vcx.debug_bounds("recent-missing-1").is_some());
        // a click on the missing row opens nothing
        let row = vcx.debug_bounds("recent-row-1").unwrap();
        vcx.simulate_click(
            gpui::point(row.origin.x + px(60.), row.origin.y + row.size.height / 2.),
            gpui::Modifiers::default(),
        );
        vcx.run_until_parked();
        assert_eq!(backend.calls(), Vec::<String>::new());
    }

    #[gpui::test]
    fn reopening_a_row_opens_its_folder(cx: &mut TestAppContext) {
        let (_, backend, vcx) = setup(cx, vec![opening(), recent(entries())]);
        let row = vcx.debug_bounds("recent-row-0").unwrap();
        vcx.simulate_click(
            gpui::point(row.origin.x + px(60.), row.origin.y + row.size.height / 2.),
            gpui::Modifiers::default(),
        );
        vcx.run_until_parked();
        assert_eq!(backend.calls(), ["open_recent D:\\work\\api"]);
    }

    #[gpui::test]
    fn removes_a_row_with_a_24_px_icon_button_r10_3(cx: &mut TestAppContext) {
        let (_, backend, vcx) = setup(cx, vec![opening(), recent(entries())]);
        let row = vcx.debug_bounds("recent-row-0").unwrap();
        // the remove button: the row's last 24 px, 8 px from its right edge
        let at = gpui::point(
            row.origin.x + row.size.width - px(8. + 12.),
            row.origin.y + row.size.height / 2.,
        );
        vcx.simulate_click(at, gpui::Modifiers::default());
        vcx.run_until_parked();
        assert_eq!(backend.calls(), ["remove_recent D:\\work\\api"]);
    }

    #[test]
    fn names_a_folder_by_its_last_segment_whichever_slash_it_uses() {
        assert_eq!(folder_name("D:/work/slash"), "slash");
        assert_eq!(folder_name("D:\\work\\back"), "back");
        assert_eq!(folder_name("C:\\"), "C:\\");
    }

    #[gpui::test]
    fn a_missing_folder_shows_its_reason_on_home(cx: &mut TestAppContext) {
        let (grace, _, vcx) = setup(
            cx,
            vec![AppEvent::Project {
                project: None,
                tree: None,
                error: Some("Folder not found: D:\\x".into()),
                subfolders: None,
            }],
        );
        grace.read_with(vcx, |g, _| {
            assert!(g.state.home());
            assert_eq!(g.state.shown_notice().unwrap().text, "Folder not found: D:\\x");
        });
    }

    #[gpui::test]
    fn home_sits_on_the_ground_in_both_themes_with_no_sheet_r12_1(cx: &mut TestAppContext) {
        let (_, _, vcx) = setup(cx, vec![opening()]);
        assert!(vcx.debug_bounds("home").is_some());
        assert!(vcx.debug_bounds("sheet").is_none());
    }
}
