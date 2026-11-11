//! `BottomPanel.test.tsx`'s tests and the splitter's, headless on the fake runner, with a data folder of their own
//! for the saved height and tab order. What a screen reader reads is `grace-e2e/tests/panel.rs`'s.
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{Entity, Modifiers, Point, TestAppContext, VisualTestContext, div, px};
use grace_core::model::node::Layout;
use grace_core::panel_height::{MAIN_MIN, PANEL_DEFAULT, PANEL_STEP};
use grace_core::panel_tabs::TAB_IDS;
use grace_core::settings::{load_panel_height, load_tab_order};

use super::*;
use crate::testing::{self, Harness};

/// A data folder of the test's own, emptied first.
fn data_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("grace-panel-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The panel under a main area, in a window 700 px tall, as the project view lays them out.
fn setup(cx: &mut TestAppContext, dir: PathBuf) -> (Harness, VisualTestContext, Entity<BottomPanel>) {
    let slot: Rc<RefCell<Option<Entity<BottomPanel>>>> = Rc::default();
    let s = Rc::clone(&slot);
    let state = testing::project("/p", Layout::Standalone, 3);
    let (h, mut vcx) = testing::open_in(cx, dir, 1000., state, move |grace, window, cx| {
        let p = cx.new(|cx| BottomPanel::new(grace.clone(), window, cx));
        *s.borrow_mut() = Some(p.clone());
        Box::new(move |_, _, cx| {
            let room = p.read(cx).room.clone();
            div()
                .relative()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .child(
                    canvas(move |b, _, _| room.set(f32::from(b.size.height)), |_, _, _, _| {})
                        .absolute()
                        .size_full(),
                )
                .child(div().flex_1().min_h(px(MAIN_MIN)))
                .child(p.clone())
                .into_any_element()
        })
    });
    let panel = slot.borrow().clone().unwrap();
    draw(&mut vcx);
    (h, vcx, panel)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn order(p: &Entity<BottomPanel>, cx: &mut VisualTestContext) -> [TabId; 3] {
    cx.update(|_, cx| p.read(cx).order)
}

fn shown(p: &Entity<BottomPanel>, cx: &mut VisualTestContext) -> f32 {
    cx.update(|_, cx| p.read(cx).sizes().0)
}

fn centre(cx: &mut VisualTestContext, selector: &'static str) -> Point<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} is drawn"))
        .center()
}

fn focus_tab(p: &Entity<BottomPanel>, k: usize, cx: &mut VisualTestContext) {
    let f = cx.update(|_, cx| p.read(cx).tabs[k].clone());
    cx.update(|window, cx| window.focus(&f, cx));
    draw(cx);
}

fn keys(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

mod tabs {
    use super::*;

    #[gpui::test]
    fn name_history_with_its_count(cx: &mut TestAppContext) {
        let (h, mut cx, _p) = setup(cx, data_dir("names"));
        h.update(&mut cx, |s| {
            s.history = vec![testing::run("a", 1, 0), testing::run("b", 0, 1)]
        });
        let state = cx.update(|_, cx| h.grace.read(cx).state.clone());
        assert_eq!(tab_name(TabId::Console, &state), "Console");
        assert_eq!(tab_name(TabId::History, &state), "History (2)");
        assert_eq!(tab_name(TabId::Http, &state), "HTTP");
    }

    #[gpui::test]
    fn a_click_shows_a_tab(cx: &mut TestAppContext) {
        let (h, mut cx, _p) = setup(cx, data_dir("click"));
        let at = centre(&mut cx, "tab-history");
        cx.simulate_click(at, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(cx.update(|_, cx| h.grace.read(cx).state.panel), Panel::History);
    }

    #[gpui::test]
    fn start_with_console_history_http(cx: &mut TestAppContext) {
        let (_h, mut cx, p) = setup(cx, data_dir("default-order"));
        assert_eq!(order(&p, &mut cx), TAB_IDS);
    }

    #[gpui::test]
    fn read_the_saved_order_before_the_first_paint(cx: &mut TestAppContext) {
        let dir = data_dir("saved-order");
        save_tab_order(&dir, &[TabId::Http, TabId::Console, TabId::History]).unwrap();
        let (_h, mut cx, p) = setup(cx, dir);
        assert_eq!(order(&p, &mut cx), [TabId::Http, TabId::Console, TabId::History]);
    }

    #[gpui::test]
    fn a_dragged_tab_takes_the_place_of_the_tab_it_is_dropped_on_and_is_saved(cx: &mut TestAppContext) {
        let dir = data_dir("drag");
        let (h, mut cx, p) = setup(cx, dir.clone());
        let from = centre(&mut cx, "tab-http");
        let to = centre(&mut cx, "tab-console");
        cx.simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(
            from + gpui::point(px(-10.), px(0.)),
            MouseButton::Left,
            Modifiers::none(),
        );
        cx.simulate_mouse_move(to, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        cx.simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        let moved = [TabId::Http, TabId::Console, TabId::History];
        assert_eq!(order(&p, &mut cx), moved);
        assert_eq!(load_tab_order(&dir), moved);
        // the shown tab stays
        assert_eq!(cx.update(|_, cx| h.grace.read(cx).state.panel), Panel::Console);
    }

    #[gpui::test]
    fn a_drag_that_ends_on_itself_changes_and_saves_nothing(cx: &mut TestAppContext) {
        let dir = data_dir("drag-self");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        let at = centre(&mut cx, "tab-history");
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(at + gpui::point(px(10.), px(0.)), MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
        cx.run_until_parked();
        assert_eq!(order(&p, &mut cx), TAB_IDS);
        assert!(!dir.join("settings.json").exists());
    }

    #[gpui::test]
    fn alt_right_and_alt_left_move_a_focused_tab_keep_its_focus_and_save(cx: &mut TestAppContext) {
        let dir = data_dir("keys");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        focus_tab(&p, 0, &mut cx);
        keys(&mut cx, "alt-right");
        let moved = [TabId::History, TabId::Console, TabId::Http];
        assert_eq!(order(&p, &mut cx), moved);
        assert_eq!(load_tab_order(&dir), moved);
        // Console is now second, and its slot holds the focus
        let focused = cx.update(|window, cx| p.read(cx).tabs[1].is_focused(window));
        assert!(focused);
        keys(&mut cx, "alt-left");
        assert_eq!(order(&p, &mut cx), TAB_IDS);
        assert_eq!(load_tab_order(&dir), TAB_IDS);
    }

    #[gpui::test]
    fn alt_left_stops_at_the_first_place_without_saving(cx: &mut TestAppContext) {
        let dir = data_dir("first");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        focus_tab(&p, 0, &mut cx);
        keys(&mut cx, "alt-left");
        assert_eq!(order(&p, &mut cx), TAB_IDS);
        assert!(!dir.join("settings.json").exists());
    }

    #[gpui::test]
    fn other_keys_leave_the_order_alone(cx: &mut TestAppContext) {
        let dir = data_dir("other-keys");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        focus_tab(&p, 0, &mut cx);
        keys(&mut cx, "right ctrl-alt-right alt-shift-right alt-up");
        assert_eq!(order(&p, &mut cx), TAB_IDS);
        assert!(!dir.join("settings.json").exists());
    }

    #[gpui::test]
    fn an_order_that_cannot_be_saved_is_only_forgotten(cx: &mut TestAppContext) {
        // a data folder that is a file: nothing can be written in it
        let dir = data_dir("unsaved").join("file");
        std::fs::write(&dir, "").unwrap();
        let (_h, mut cx, p) = setup(cx, dir);
        assert_eq!(order(&p, &mut cx), TAB_IDS);
        focus_tab(&p, 0, &mut cx);
        keys(&mut cx, "alt-right");
        assert_eq!(order(&p, &mut cx), [TabId::History, TabId::Console, TabId::Http]);
    }
}

mod splitter {
    use super::*;

    #[gpui::test]
    fn starts_at_the_default_height(cx: &mut TestAppContext) {
        let (_h, mut cx, p) = setup(cx, data_dir("default-height"));
        assert_eq!(shown(&p, &mut cx), PANEL_DEFAULT);
    }

    #[gpui::test]
    fn reads_the_saved_height(cx: &mut TestAppContext) {
        let dir = data_dir("saved-height");
        save_panel_height(&dir, 250.).unwrap();
        let (_h, mut cx, p) = setup(cx, dir);
        assert_eq!(shown(&p, &mut cx), 250.);
    }

    #[gpui::test]
    fn a_saved_height_taller_than_the_window_is_clamped(cx: &mut TestAppContext) {
        let dir = data_dir("tall");
        save_panel_height(&dir, 5000.).unwrap();
        let (_h, mut cx, p) = setup(cx, dir);
        let (h, max) = cx.update(|_, cx| p.read(cx).sizes());
        assert_eq!(h, max);
        assert_eq!(max, 700. - MAIN_MIN);
    }

    #[gpui::test]
    fn the_keys_step_and_jump_and_save(cx: &mut TestAppContext) {
        let dir = data_dir("splitter-keys");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        let f = cx.update(|_, cx| p.read(cx).splitter.clone());
        cx.update(|window, cx| window.focus(&f, cx));
        draw(&mut cx);
        keys(&mut cx, "up");
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), PANEL_DEFAULT + PANEL_STEP);
        assert_eq!(load_panel_height(&dir), PANEL_DEFAULT + PANEL_STEP);
        keys(&mut cx, "down");
        keys(&mut cx, "down");
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), PANEL_DEFAULT - PANEL_STEP);
        keys(&mut cx, "home");
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), PANEL_MIN);
        keys(&mut cx, "end");
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), 700. - MAIN_MIN);
        assert_eq!(load_panel_height(&dir), 700. - MAIN_MIN);
    }

    #[gpui::test]
    fn a_key_at_a_limit_keeps_the_saved_height(cx: &mut TestAppContext) {
        let dir = data_dir("limit");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        p.update(&mut cx, |p, cx| p.key(PanelKey::Home, cx));
        draw(&mut cx);
        std::fs::remove_file(dir.join("settings.json")).unwrap();
        p.update(&mut cx, |p, cx| p.key(PanelKey::Down, cx));
        assert!(!dir.join("settings.json").exists());
    }

    #[gpui::test]
    fn dragging_resizes_within_the_limits_and_saves_on_release(cx: &mut TestAppContext) {
        let dir = data_dir("drag-height");
        let (_h, mut cx, p) = setup(cx, dir.clone());
        let at = centre(&mut cx, "splitter");
        cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
        // the first move starts the drag, as a pointer's first few pixels do
        cx.simulate_mouse_move(at + gpui::point(px(0.), px(-5.)), MouseButton::Left, Modifiers::none());
        let up = at + gpui::point(px(0.), px(-50.));
        cx.simulate_mouse_move(up, MouseButton::Left, Modifiers::none());
        draw(&mut cx);
        assert!(cx.update(|_, cx| p.read(cx).is_dragging()));
        assert_eq!(shown(&p, &mut cx), PANEL_DEFAULT + 50.);
        assert!(!dir.join("settings.json").exists(), "saved only on release");
        // far above the window: the maximum
        let top = gpui::point(at.x, px(-500.));
        cx.simulate_mouse_move(top, MouseButton::Left, Modifiers::none());
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), 700. - MAIN_MIN);
        cx.simulate_mouse_up(top, MouseButton::Left, Modifiers::none());
        draw(&mut cx);
        assert!(!cx.update(|_, cx| p.read(cx).is_dragging()));
        assert_eq!(load_panel_height(&dir), 700. - MAIN_MIN);
    }

    #[gpui::test]
    fn a_smaller_window_shrinks_the_panel_without_forgetting_its_height(cx: &mut TestAppContext) {
        let dir = data_dir("shrink");
        save_panel_height(&dir, 400.).unwrap();
        let (_h, mut cx, p) = setup(cx, dir.clone());
        assert_eq!(shown(&p, &mut cx), 400.);
        cx.simulate_resize(gpui::size(px(1000.), px(400.)));
        draw(&mut cx);
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), 400. - MAIN_MIN);
        cx.simulate_resize(gpui::size(px(1000.), px(700.)));
        draw(&mut cx);
        draw(&mut cx);
        assert_eq!(shown(&p, &mut cx), 400.);
        assert_eq!(load_panel_height(&dir), 400.);
    }
}
