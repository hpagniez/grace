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

#[cfg(windows)]
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

/// How far the bounds Grace saves can sit from the rectangle the driver gave the window. Windows moves a window by
/// its outer frame, and GPUI's bounds leave the frame out; Xvfb has no frame.
const FRAME: f64 = if cfg!(windows) { 40. } else { 1. };

fn within_frame(actual: Area, want: Area) -> bool {
    [
        (actual.x, want.x),
        (actual.y, want.y),
        (actual.width, want.width),
        (actual.height, want.height),
    ]
    .iter()
    .all(|(a, w)| (a - w).abs() <= FRAME)
}

/// Places the window at `want` and waits for Grace to save the move (WB-03); returns the bounds it saved.
fn placed(app: &App, want: Area) -> Area {
    place(app, want);
    app.wait_until("the move saved", Duration::from_secs(10), || {
        app.saved_window()
            .is_some_and(|s| within_frame(s.bounds, want) && !s.maximized)
    })
    .unwrap();
    app.saved_window().unwrap().bounds
}

fn saved(window: Area, maximized: bool) -> serde_json::Value {
    json!({ "x": window.x, "y": window.y, "width": window.width, "height": window.height, "maximized": maximized })
}

/// E-window-01, WB-01
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_window_comes_back_where_it_was() {
    let app = launch(fresh_data_dir().unwrap());
    let want = placed(&app, inside(app.work_area().unwrap()));
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
    placed(&app, inside(wa));
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

/// #346: a first start on a screen too small for 1200×800 (CI's 1024×768) keeps the whole window on it, the title bar
/// too, as Electron's `BrowserWindow` does; GPUI took the shrunk size as the client area and put half the frame above
/// the screen's top edge. The title bar reads only where the screen shows it.
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn a_first_window_keeps_its_title_bar_on_the_screen() {
    let app = launch(fresh_data_dir().unwrap());
    app.wait_until("the title bar on the screen", Duration::from_secs(15), || {
        matches!(app.title_bar_colour(), Ok(Some(_)))
    })
    .unwrap_or_else(|e| panic!("{e}: {:?}; {}", app.title_bar_colour(), app.title_bar_report()));
}

/// #321: Grace's icon in the title bar and on the taskbar, as Electron Grace's exe carries it. `Grace.exe` holds
/// `assets/icon.ico` as icon 1, each image as the file has it, and the window has an icon: GPUI loads icon 1 of the exe
/// for its window class and leaves the class without one when the exe has none.
#[cfg(windows)]
#[test]
#[ignore = "starts the app; run with scripts/e2e-gpui.sh"]
fn the_exe_and_its_window_carry_graces_icon() {
    use windows::Win32::UI::WindowsAndMessaging::{RT_GROUP_ICON, RT_ICON};

    let ico = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icon.ico")).unwrap();
    let field = |at: usize| u32::from_le_bytes(ico[at..at + 4].try_into().unwrap()) as usize;
    let exe = resources::Exe::open(&grace_e2e::grace_exe());
    let group = exe.resource(1, RT_GROUP_ICON).expect("icon 1 in Grace.exe");
    let count = usize::from(u16::from_le_bytes([ico[4], ico[5]]));
    assert_eq!(group[..6], ico[..6], "the icon's header");
    assert_eq!(group.len(), 6 + 14 * count, "one entry for each image of the file");
    for i in 0..count {
        let (entry, item) = (6 + 16 * i, &group[6 + 14 * i..6 + 14 * (i + 1)]);
        assert_eq!(item[..12], ico[entry..entry + 12], "image {}'s size and colours", i + 1);
        let id = u16::from_le_bytes([item[12], item[13]]);
        let (size, offset) = (field(entry + 8), field(entry + 12));
        let image = exe
            .resource(id, RT_ICON)
            .unwrap_or_else(|| panic!("image {} in Grace.exe", i + 1));
        assert!(
            image == ico[offset..offset + size],
            "image {} as the file holds it",
            i + 1
        );
    }
    drop(exe);

    let app = App::launch().unwrap();
    app.wait_until("the window's icon", Duration::from_secs(15), || {
        matches!(app.window_icon(), Ok(Some(true)))
    })
    .unwrap_or_else(|e| panic!("{e}: {:?}", app.window_icon()));
}

/// An exe's resources, read as Explorer and the window manager read them.
#[cfg(windows)]
mod resources {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::Win32::Foundation::{FreeLibrary, HMODULE};
    use windows::Win32::System::LibraryLoader::{
        FindResourceW, LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE, LoadLibraryExW, LoadResource,
        LockResource, SizeofResource,
    };
    use windows::core::PCWSTR;

    /// An exe loaded for its resources only: none of its code runs.
    pub struct Exe(HMODULE);

    impl Exe {
        pub fn open(path: &Path) -> Self {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
            // SAFETY: a nul-terminated path that lives across the call
            let module = unsafe {
                LoadLibraryExW(
                    PCWSTR(wide.as_ptr()),
                    None,
                    LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE,
                )
            }
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            Self(module)
        }

        /// The bytes of the resource of type `kind` numbered `id`, where the exe has one.
        pub fn resource(&self, id: u16, kind: PCWSTR) -> Option<Vec<u8>> {
            let name = PCWSTR(std::ptr::without_provenance(usize::from(id)));
            // SAFETY: the module stays loaded while `self` lives, and the bytes are copied out of it before the return
            unsafe {
                let found = FindResourceW(Some(self.0), name, kind);
                if found.is_invalid() {
                    return None;
                }
                let size = SizeofResource(Some(self.0), found) as usize;
                let data = LockResource(LoadResource(Some(self.0), found).ok()?) as *const u8;
                (!data.is_null()).then(|| std::slice::from_raw_parts(data, size).to_vec())
            }
        }
    }

    impl Drop for Exe {
        fn drop(&mut self) {
            // SAFETY: the module this value loaded, freed once
            let _ = unsafe { FreeLibrary(self.0) };
        }
    }
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
    let normal = placed(&app, inside(wa));
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
                .is_some_and(|s| !s.maximized && is_near(s.bounds, normal))
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
    let normal = placed(&app, inside(wa));
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
    near(s.bounds, normal);
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
