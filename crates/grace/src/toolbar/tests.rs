//! `Toolbar.test.tsx`, for the toolbar of P2's second PR: order and shape, the narrow window, Home and Refresh, the
//! project and the actions. Its Env, Threads and Properties tests come with those boxes (P2 PR 4), its step debugger
//! and Watch tests with phase P4.
use super::*;
use crate::testing::{Harness, open, project, run};
use gpui::{Bounds, Modifiers, Pixels, TestAppContext, VisualTestContext, point};
use std::time::Duration;

const WIDE: f32 = 1400.;
const NARROW: f32 = 1000.;
const BUTTONS: [&str; 7] = [
    "home-button",
    "refresh-button",
    "open-button",
    "report-button",
    "rerun-failed-button",
    "cancel-button",
    "run-all-button",
];

fn toolbar(cx: &mut TestAppContext, width: f32, state: AppState) -> (Harness, VisualTestContext) {
    open(cx, width, state, |grace, window, cx| {
        let state = grace.read(cx).state.clone();
        render(grace, &state, vec![], window, cx).into_any_element()
    })
}

fn at(cx: &mut VisualTestContext, id: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(id).unwrap_or_else(|| panic!("{id} is drawn"))
}

fn click(cx: &mut VisualTestContext, id: &'static str) {
    let b = at(cx, id);
    cx.simulate_click(b.center(), Modifiers::default());
    cx.run_until_parked();
}

/// The tooltip after the pointer rests on `id`.
fn hover(h: &Harness, cx: &mut VisualTestContext, id: &'static str) -> Option<SharedString> {
    let b = at(cx, id);
    cx.simulate_mouse_move(b.center(), None, Modifiers::none());
    cx.executor()
        .advance_clock(crate::tooltip::DELAY + Duration::from_millis(1));
    cx.run_until_parked();
    let tip = h.tip(cx);
    cx.simulate_mouse_move(point(px(1.), px(600.)), None, Modifiers::none());
    cx.run_until_parked();
    tip
}

fn idle() -> AppState {
    project("D:\\work\\api", Layout::Maven, 3)
}

mod order_and_shape_r13_1 {
    use super::*;

    #[gpui::test]
    fn is_one_44_px_row_on_the_ground(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        assert_eq!(at(&mut cx, "toolbar").size.height, px(44.));
    }

    #[gpui::test]
    fn puts_the_controls_in_the_order_of_the_spec_run_all_last(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        let mut ids = BUTTONS.to_vec();
        ids.insert(1, "project");
        let xs: Vec<Pixels> = ids.iter().map(|id| at(&mut cx, id).origin.x).collect();
        assert!(xs.windows(2).all(|w| w[0] < w[1]), "{ids:?} {xs:?}");
    }

    #[gpui::test]
    fn separates_the_project_from_refresh_and_open_from_report_with_1_px_dividers(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        let first = at(&mut cx, "divider-actions");
        let second = at(&mut cx, "divider-run");
        assert_eq!((first.size.width, first.size.height), (px(1.), px(20.)));
        assert_eq!(second.size.width, px(1.));
        assert!(at(&mut cx, "project").right() <= first.left());
        assert!(first.right() <= at(&mut cx, "refresh-button").left());
        assert!(at(&mut cx, "open-button").right() <= second.left());
        assert!(second.right() <= at(&mut cx, "report-button").left());
    }

    #[gpui::test]
    fn every_button_is_28_px_tall_and_the_icon_buttons_square(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        for id in BUTTONS {
            assert_eq!(at(&mut cx, id).size.height, px(28.), "{id} {:?}", at(&mut cx, id));
        }
        for id in ["home-button", "refresh-button"] {
            assert_eq!(at(&mut cx, id).size.width, px(28.), "{id}");
        }
    }
}

mod a_narrow_window {
    use super::*;

    const SHRUNK: [&str; 3] = ["open-button", "report-button", "cancel-button"];

    #[gpui::test]
    fn below_1280_px_shrinks_open_report_and_cancel_to_a_28_px_icon(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, NARROW, idle());
        for id in SHRUNK {
            assert_eq!(at(&mut cx, id).size.width, px(28.), "{id}");
        }
        cx.simulate_resize(gpui::size(px(WIDE), px(700.)));
        cx.run_until_parked();
        for id in SHRUNK {
            assert!(at(&mut cx, id).size.width > px(40.), "{id} shows its label");
        }
    }

    #[gpui::test]
    fn keeps_the_count_pill_of_re_run_failed(cx: &mut TestAppContext) {
        let mut s = idle();
        s.history = vec![run("r1", 1, 3)];
        let (_, mut cx) = toolbar(cx, NARROW, s);
        let button = at(&mut cx, "rerun-failed-button");
        let pill = at(&mut cx, "rerun-failed-count");
        assert!(button.contains(&pill.center()));
        assert!(button.size.width > px(28.) && button.size.width < px(80.), "{button:?}");
    }

    #[gpui::test]
    fn keeps_run_alls_label_in_sight(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, NARROW, idle());
        assert!(at(&mut cx, "run-all-button").size.width > px(60.));
    }

    #[test]
    fn asks_for_the_width_the_electron_classes_use() {
        assert_eq!(NARROW_BELOW, 1280.);
    }

    #[gpui::test]
    fn gives_each_a_tooltip_with_its_label_while_the_label_is_hidden_also_when_disabled(cx: &mut TestAppContext) {
        let (h, mut cx) = toolbar(cx, NARROW, idle());
        // idle: Cancel and Report are disabled, Open is not
        assert_eq!(hover(&h, &mut cx, "open-button").as_deref(), Some(OPEN));
        assert_eq!(hover(&h, &mut cx, "report-button").as_deref(), Some(REPORT));
        assert_eq!(hover(&h, &mut cx, "cancel-button").as_deref(), Some(CANCEL));
        assert_eq!(hover(&h, &mut cx, "rerun-failed-button").as_deref(), Some(RERUN_FAILED));
    }

    #[gpui::test]
    fn gives_them_no_tooltip_while_the_label_shows(cx: &mut TestAppContext) {
        let (h, mut cx) = toolbar(cx, WIDE, idle());
        for id in ["open-button", "report-button", "cancel-button", "rerun-failed-button"] {
            assert_eq!(hover(&h, &mut cx, id), None, "{id}");
        }
    }

    #[gpui::test]
    fn follows_the_window_as_it_is_resized(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        assert!(at(&mut cx, "open-button").size.width > px(40.));
        cx.simulate_resize(gpui::size(px(NARROW), px(700.)));
        cx.run_until_parked();
        assert_eq!(at(&mut cx, "open-button").size.width, px(28.));
    }
}

mod home_and_refresh_the_icon_only_buttons {
    use super::*;

    #[gpui::test]
    fn home_holds_the_logo_and_shows_home_in_a_tooltip(cx: &mut TestAppContext) {
        let (h, mut cx) = toolbar(cx, WIDE, idle());
        assert_eq!(hover(&h, &mut cx, "home-button").as_deref(), Some(HOME));
        let logo = gpui::AssetSource::load(&crate::assets::Assets, LOGO_PATH)
            .unwrap()
            .unwrap();
        assert!(logo.starts_with(b"<svg"));
    }

    #[gpui::test]
    fn home_goes_home_and_is_off_while_busy(cx: &mut TestAppContext) {
        let (h, mut cx) = toolbar(cx, WIDE, idle());
        click(&mut cx, "home-button");
        assert_eq!(h.backend.calls(), ["close"]);
        let mut busy = idle();
        busy.busy = true;
        assert!(!actions(&busy).home);
    }

    #[gpui::test]
    fn refresh_shows_its_word_in_a_tooltip_also_while_disabled(cx: &mut TestAppContext) {
        let mut busy = idle();
        busy.busy = true;
        let (h, mut cx) = toolbar(cx, WIDE, busy);
        assert_eq!(hover(&h, &mut cx, "refresh-button").as_deref(), Some(REFRESH));
        click(&mut cx, "refresh-button");
        assert_eq!(h.backend.calls(), Vec::<String>::new());
        h.update(&mut cx, |s| s.busy = false);
        click(&mut cx, "refresh-button");
        assert_eq!(h.backend.calls(), ["refresh"]);
    }
}

mod the_project {
    use super::*;

    #[test]
    fn shows_the_folder_name_and_the_layout_and_leaves_the_path_to_the_status_bar_r1_1() {
        assert_eq!(folder_name("D:\\work\\api"), "api");
        assert_eq!(layout_label(Layout::Maven), "Maven");
        assert_eq!(layout_label(Layout::Gradle), "Gradle");
        assert_eq!(layout_label(Layout::Standalone), "Standalone");
    }

    #[gpui::test]
    fn draws_the_name_and_the_chip(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        let name = at(&mut cx, "project-name");
        let chip = at(&mut cx, "project-layout");
        assert!(name.right() <= chip.left());
        assert_eq!(chip.size.height, px(20.));
    }

    #[test]
    fn names_a_folder_opened_with_forward_slashes_too_and_one_with_a_trailing_separator() {
        assert_eq!(folder_name("D:/work/api"), "api");
        assert_eq!(folder_name("D:\\work\\api\\"), "api");
        assert_eq!(folder_name("/home/me/api/"), "api");
    }

    #[test]
    fn cuts_a_long_name_in_the_middle_its_start_gives_way_its_last_12_characters_stay_r1_2() {
        let name = "a-very-long-project-folder-name-v2";
        let (head, tail) = name_parts(name);
        assert_eq!(tail, "lder-name-v2");
        assert_eq!(tail.chars().count(), NAME_TAIL);
        assert_eq!(format!("{head}{tail}"), name);
        // characters, not bytes
        let (h, t) = name_parts("projet-très-lointain-éé");
        assert_eq!(t.chars().count(), 12);
        assert_eq!(format!("{h}{t}"), "projet-très-lointain-éé");
    }

    #[test]
    fn keeps_a_name_of_12_characters_or_fewer_in_its_end_part() {
        assert_eq!(name_parts("api"), ("", "api"));
        assert_eq!(name_parts("twelve-chars"), ("", "twelve-chars"));
    }

    #[gpui::test]
    fn explains_a_cut_name_in_a_tooltip_and_says_nothing_while_it_fits_r1_3(cx: &mut TestAppContext) {
        let long = format!(
            "D:\\work\\{}",
            "a-project-name-long-enough-to-be-cut-in-a-narrow-window-".repeat(4)
        );
        let name = folder_name(&long).to_string();
        let (h, mut cx) = toolbar(cx, 760., project(&long, Layout::Maven, 3));
        assert_eq!(hover(&h, &mut cx, "project-name").as_deref(), Some(name.as_str()));
        h.set(&mut cx, idle());
        cx.simulate_resize(gpui::size(px(WIDE), px(700.)));
        cx.run_until_parked();
        assert_eq!(hover(&h, &mut cx, "project-name"), None);
    }

    #[gpui::test]
    fn lets_the_chip_go_whole_before_the_name_is_cut_r1_2(cx: &mut TestAppContext) {
        let long = format!("D:\\work\\{}", "x".repeat(60));
        let (_, mut cx) = toolbar(cx, 760., project(&long, Layout::Standalone, 3));
        let project = at(&mut cx, "project");
        // the chip wrapped to the hidden second line
        let chip = at(&mut cx, "project-layout");
        assert!(chip.top() >= project.bottom(), "{chip:?} {project:?}");
    }
}

mod the_actions {
    use super::*;

    #[gpui::test]
    fn run_all_runs_the_trees_root(cx: &mut TestAppContext) {
        let (h, mut cx) = toolbar(cx, WIDE, idle());
        click(&mut cx, "run-all-button");
        assert_eq!(h.backend.calls(), ["run d:"]);
    }

    #[test]
    fn shows_the_newest_runs_failed_count_on_re_run_failed() {
        let mut s = idle();
        s.history = vec![run("new", 2, 3), run("old", 0, 9)];
        let a = actions(&s);
        assert_eq!(a.failed, 3);
        assert!(a.rerun_failed);
    }

    #[gpui::test]
    fn re_run_failed_reruns_the_newest_run(cx: &mut TestAppContext) {
        let mut s = idle();
        s.history = vec![run("new", 2, 3), run("old", 0, 9)];
        let (h, mut cx) = toolbar(cx, WIDE, s);
        assert!(cx.debug_bounds("rerun-failed-count").is_some());
        click(&mut cx, "rerun-failed-button");
        assert_eq!(h.backend.calls(), ["rerun_failed new"]);
    }

    #[gpui::test]
    fn shows_no_pill_while_there_is_nothing_to_re_run(cx: &mut TestAppContext) {
        let mut s = idle();
        s.history = vec![run("new", 4, 0)];
        let (_, mut cx) = toolbar(cx, WIDE, s.clone());
        assert!(cx.debug_bounds("rerun-failed-count").is_none());
        assert!(!actions(&s).rerun_failed);
    }

    #[test]
    fn keeps_run_all_cancel_open_and_report_as_before() {
        let s = idle();
        let a = actions(&s);
        assert!(a.run_all && a.open && !a.cancel && !a.report);
        let mut running = idle();
        running.run.active = true;
        let a = actions(&running);
        assert!(!a.run_all && !a.open && a.cancel && !a.report && !a.refresh && !a.home);
        let mut finding = idle();
        finding.busy = true;
        assert!(actions(&finding).cancel);
        let mut done = idle();
        done.run.report_available = true;
        assert!(actions(&done).report);
    }

    #[gpui::test]
    fn cancel_and_report_ask_the_runner(cx: &mut TestAppContext) {
        let mut s = idle();
        s.run.active = true;
        let (h, mut cx) = toolbar(cx, WIDE, s);
        click(&mut cx, "cancel-button");
        assert_eq!(h.backend.calls(), ["cancel"]);
        h.update(&mut cx, |s| {
            s.run.active = false;
            s.run.report_available = true;
        });
        click(&mut cx, "report-button");
        assert_eq!(h.backend.calls(), ["cancel", "report_path"]);
        assert_eq!(h.seams.opened(), [std::path::PathBuf::from("/out/karate-summary.html")]);
    }

    #[gpui::test]
    fn open_asks_for_a_folder(cx: &mut TestAppContext) {
        let (h, mut cx) = toolbar(cx, WIDE, idle());
        click(&mut cx, "open-button");
        assert_eq!(h.backend.calls(), ["open /picked"]);
    }

    /// WS-10: nothing to run, or nothing the filter lets through.
    #[test]
    fn run_all_is_disabled_with_nothing_to_run() {
        let empty = project("D:\\empty", Layout::Standalone, 0);
        assert!(!actions(&empty).run_all);
        let mut filtered = idle();
        filtered.filter.include = vec!["nothing".into()];
        assert!(!actions(&filtered).run_all);
        filtered.filter.include = vec!["smoke".into()];
        assert!(actions(&filtered).run_all);
        let mut no_tree = idle();
        no_tree.tree = None;
        assert!(!actions(&no_tree).run_all);
    }
}

mod properties {
    use super::*;

    /// `PropertiesDialog.test.tsx`: the Properties button is disabled with no project, while busy and while running.
    #[test]
    fn is_off_with_no_environment_view_while_busy_and_while_running() {
        let mut s = idle();
        assert!(!actions(&s).properties, "no environment view yet");
        s.env = Some(grace_core::model::env::EnvView {
            env: None,
            suggestions: vec![],
            overlays: vec![],
            properties: Default::default(),
            secrets_available: true,
        });
        assert!(actions(&s).properties);
        s.busy = true;
        assert!(!actions(&s).properties);
        s.busy = false;
        s.run.active = true;
        assert!(!actions(&s).properties);
    }

    #[gpui::test]
    fn sits_after_the_boxes_and_before_refresh(cx: &mut TestAppContext) {
        let (_, mut cx) = toolbar(cx, WIDE, idle());
        assert!(at(&mut cx, "project").origin.x < at(&mut cx, "properties-button").origin.x);
        assert!(at(&mut cx, "properties-button").origin.x < at(&mut cx, "refresh-button").origin.x);
    }
}
