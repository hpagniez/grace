//! The window through the accessibility tree: the empty window of phase P0, then `e2e/window.spec.ts`, where the app
//! places its window and saves its bounds. These tests start the real app, so they are ignored by
//! `cargo test --workspace`; `scripts/e2e-gpui.sh` builds Grace with the `e2e` feature and runs them.
//!
//! The window's bounds are read as Grace saves them on close, in GPUI's own coordinates, as `window.spec.ts` read
//! Electron's `getBounds`. Maximizing and minimizing need a window manager, which Xvfb has none of: those tests run on
//! Windows only.
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use grace_e2e::{App, Area, Node, Role, SavedWindow, fresh_data_dir};
use serde_json::json;

/// The window, once Grace's own root (named Grace, inside the window named Grace) is in it.
fn grace_inside(tree: &Node) -> bool {
    tree.children.iter().any(|c| c.all().iter().any(|n| n.name == "Grace"))
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn opens_one_window_named_grace() {
    let app = App::launch().unwrap();
    let tree = app
        .wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    assert_eq!(tree.role, Role::Window, "{}", tree.dump());
    assert_eq!(tree.name, "Grace", "{}", tree.dump());
}

#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn has_a_live_region_for_announcements() {
    let app = App::launch().unwrap();
    let tree = app
        .wait_for("the live region", Duration::from_secs(15), |t| {
            t.all().iter().any(|n| n.role == Role::Status)
        })
        .unwrap();
    assert!(tree.all().iter().any(|n| n.role == Role::Status), "{}", tree.dump());
}

/// Guide R8.2: every interactive element of the window has a name.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn names_every_control() {
    let app = App::launch().unwrap();
    let tree = app
        .wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    assert_eq!(tree.unnamed_controls(), Vec::<String>::new(), "{}", tree.dump());
}

// ----- window.spec.ts: where the app places its window -----

/// These tests check where the app itself places its window: on the main screen, never moved to another.
const MAIN: &[(&str, &str)] = &[("GRACE_E2E_SCREEN", "main")];

/// Grace on the main screen with `data` as its data folder, once its window shows.
fn launch(data: PathBuf) -> App {
    let app = App::launch_in(data, MAIN).unwrap();
    app.wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    app
}

/// A fresh data folder whose settings.json holds this window state.
fn with_saved(window: serde_json::Value) -> PathBuf {
    let data = fresh_data_dir().unwrap();
    std::fs::write(data.join("settings.json"), json!({ "window": window }).to_string()).unwrap();
    data
}

/// Closes the window and returns what Grace saved of it.
fn closed(mut app: App) -> SavedWindow {
    app.close_and_wait().unwrap();
    app.saved_window().expect("the window saved on close")
}

/// The bounds of a first start with a fresh data folder, and the main screen's work area.
fn first_start() -> (Area, Area) {
    static FIRST: OnceLock<(Area, Area)> = OnceLock::new();
    *FIRST.get_or_init(|| {
        let app = launch(fresh_data_dir().unwrap());
        let work_area = app.work_area().unwrap();
        (closed(app).bounds, work_area)
    })
}

/// 800 by 500 (the window's minimum width is 760), 40 px right and 30 px down from the work area's corner.
fn inside(wa: Area) -> Area {
    Area {
        x: wa.x + 40.,
        y: wa.y + 30.,
        width: 800.,
        height: 500.,
    }
}

fn near(actual: Area, want: Area) {
    for (k, a, w) in [
        ("x", actual.x, want.x),
        ("y", actual.y, want.y),
        ("width", actual.width, want.width),
        ("height", actual.height, want.height),
    ] {
        assert!((a - w).abs() <= 1., "{k}: {a} ≈ {w} ({actual:?}, want {want:?})");
    }
}

fn is_near(actual: Area, want: Area) -> bool {
    [
        (actual.x, want.x),
        (actual.y, want.y),
        (actual.width, want.width),
        (actual.height, want.height),
    ]
    .iter()
    .all(|(a, w)| (a - w).abs() <= 1.)
}

fn place(app: &App, r: Area) {
    app.set_window_rect(r.x as i32, r.y as i32, r.width as i32, r.height as i32)
        .unwrap();
}

fn saved(window: Area, maximized: bool) -> serde_json::Value {
    json!({ "x": window.x, "y": window.y, "width": window.width, "height": window.height, "maximized": maximized })
}

/// E-window-01, WB-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_window_comes_back_where_it_was() {
    let app = launch(fresh_data_dir().unwrap());
    let want = inside(app.work_area().unwrap());
    place(&app, want);
    app.wait_until("the move saved", Duration::from_secs(10), || {
        app.saved_window().is_some_and(|s| is_near(s.bounds, want))
    })
    .unwrap();
    let app = app.relaunch(MAIN).unwrap();
    app.wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    let s = closed(app);
    near(s.bounds, want);
    assert!(!s.maximized);
}

/// E-window-03, WB-03: a move is saved once the window keeps its bounds, before any close.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_move_by_the_person_is_saved_before_the_app_closes() {
    let (_, wa) = first_start();
    let app = launch(fresh_data_dir().unwrap());
    place(&app, inside(wa));
    app.wait_until("the move saved", Duration::from_secs(10), || {
        app.saved_window()
            .is_some_and(|s| is_near(s.bounds, inside(wa)) && !s.maximized)
    })
    .unwrap();
}

/// E-window-05, WB-06: settings.json is a folder, so the save on close fails; the window still closes.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_failed_save_does_not_stop_the_app_from_closing() {
    let (_, wa) = first_start();
    let data = fresh_data_dir().unwrap();
    std::fs::create_dir(data.join("settings.json")).unwrap();
    let mut app = launch(data);
    place(&app, inside(wa));
    app.close_and_wait().unwrap();
    assert!(app.saved_window().is_none());
}

/// E-window-06, WB-08
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_short_saved_window_is_held_at_the_minimum_height() {
    let (_, wa) = first_start();
    let short = Area {
        height: 300.,
        ..inside(wa)
    };
    let s = closed(launch(with_saved(saved(short, false))));
    assert_eq!(s.bounds.height, 480.);
}

/// E-window-07, WB-09
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_broken_saved_state_opens_the_window_as_on_a_first_start() {
    let (first, _) = first_start();
    for window in [
        json!("abc"),
        json!({ "x": 10, "y": 10, "width": -5, "height": 500, "maximized": false }),
        json!({ "x": "a", "y": 10, "width": 700, "height": 500, "maximized": false }),
        json!({ "x": 10, "y": 10, "width": 700, "height": 500 }),
    ] {
        let s = closed(launch(with_saved(window)));
        near(s.bounds, first);
        assert!(!s.maximized);
    }
}

/// E-window-08, WB-10
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_window_partly_off_the_screen_is_pulled_onto_it() {
    let (_, wa) = first_start();
    let off = Area {
        x: wa.x + wa.width - 400.,
        ..inside(wa)
    };
    let s = closed(launch(with_saved(saved(off, false))));
    near(
        s.bounds,
        Area {
            x: wa.x + wa.width - 800.,
            ..off
        },
    );
}

/// E-window-09: snapped to the left, Windows' invisible resize border sticks out left and below.
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_window_flush_with_the_screen_edges_keeps_its_size_and_place() {
    let (_, wa) = first_start();
    let flush = Area {
        x: wa.x - 7.,
        y: wa.y,
        width: 800.,
        height: wa.height + 7.,
    };
    near(closed(launch(with_saved(saved(flush, false)))).bounds, flush);
}

/// E-window-10, WB-11
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_window_bigger_than_the_screen_is_shrunk_to_fit() {
    let (_, wa) = first_start();
    let big = Area {
        x: wa.x,
        y: wa.y,
        width: wa.width + 500.,
        height: 500.,
    };
    near(
        closed(launch(with_saved(saved(big, false)))).bounds,
        Area { width: wa.width, ..big },
    );
}

/// E-window-11, WB-12
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_window_on_a_screen_that_is_gone_opens_on_the_main_screen() {
    let (first, _) = first_start();
    let gone = Area {
        x: -30000.,
        y: -30000.,
        width: 800.,
        height: 500.,
    };
    near(closed(launch(with_saved(saved(gone, false)))).bounds, first);
}

/// E-window-02, WB-02
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_maximized_window_comes_back_maximized() {
    use grace_e2e::Show;
    let (_, wa) = first_start();
    let app = launch(fresh_data_dir().unwrap());
    place(&app, inside(wa));
    app.show_window(Show::Maximized).unwrap();
    app.wait_until("maximized saved", Duration::from_secs(10), || {
        app.saved_window().is_some_and(|s| s.maximized)
    })
    .unwrap();
    let again = app.relaunch(MAIN).unwrap();
    again
        .wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    again.show_window(Show::Normal).unwrap();
    again
        .wait_until("restored to its bounds", Duration::from_secs(10), || {
            again
                .saved_window()
                .is_some_and(|s| !s.maximized && is_near(s.bounds, inside(wa)))
        })
        .unwrap();
}

/// E-window-04, WB-05
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_window_closed_while_minimized_opens_normal() {
    use grace_e2e::Show;
    let (_, wa) = first_start();
    let app = launch(fresh_data_dir().unwrap());
    place(&app, inside(wa));
    app.show_window(Show::Maximized).unwrap();
    app.show_window(Show::Minimized).unwrap();
    let mut app = app;
    app.close_and_wait().unwrap();
    let again = app.relaunch(MAIN).unwrap();
    again
        .wait_for("the window", Duration::from_secs(15), grace_inside)
        .unwrap();
    let s = closed(again);
    assert!(!s.maximized);
    near(s.bounds, inside(wa));
}

/// E-window-12, WB-13
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_maximized_window_on_a_screen_that_is_gone_opens_maximized_on_the_main_screen() {
    let (_, wa) = first_start();
    let gone = Area {
        x: -30000.,
        y: -30000.,
        width: 800.,
        height: 500.,
    };
    let s = closed(launch(with_saved(saved(gone, true))));
    assert!(s.maximized);
    // the bounds it restores to are on the main screen
    assert!(s.bounds.x >= wa.x && s.bounds.x < wa.x + wa.width, "{s:?} on {wa:?}");
}
