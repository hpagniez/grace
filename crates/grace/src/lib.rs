//! GPUI Grace (ADR 0031). Phase P0 (#279) built the foundation: the theme, the embedded assets, the shared elements,
//! accessibility and the window. Phase P2 (#281) builds the main screen on `grace-core` (plan
//! `docs/plans/2026-11-09-gpui-p2-main-screen.md`); P3 and P4 the panels and the project setup.
pub mod a11y;
pub mod app;
pub mod assets;
pub mod bottom_panel;
pub mod console;
#[cfg(feature = "e2e")]
pub mod e2e;
pub mod elements;
pub mod env_picker;
pub mod filter_bar;
pub mod home;
pub mod instance;
pub mod probe;
pub mod progress;
pub mod properties;
pub mod seams;
pub mod status_bar;
#[cfg(test)]
pub mod testing;
pub mod theme;
pub mod threads_box;
pub mod toolbar;
pub mod tooltip;
pub mod tree;
pub mod ui;
pub mod window;

use std::path::PathBuf;
use std::sync::Arc;

use gpui::App;

/// What every window of the app needs: GPUI Kit, IBM Plex and the theme.
pub fn init(cx: &mut App) {
    gpui_component::init(cx);
    assets::load_fonts(cx);
    a11y::bind_keys(cx);
    console::bind_keys(cx);
    theme::apply(false, None, cx);
}

/// Where the bundled Java and Karate are: `resources` beside `Grace.exe` in the release zip (ADR 0019), else the
/// repository's `vendor/` in development (`cargo run`).
fn vendor_dir() -> PathBuf {
    let packaged = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("resources")));
    match packaged {
        Some(dir) if dir.join("karate.jar").is_file() => dir,
        _ => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor")),
    }
}

/// The platform seams, or the e2e build's answers when a test gives them.
fn seams() -> Arc<dyn seams::Seams> {
    #[cfg(feature = "e2e")]
    if let Some(s) = e2e::E2eSeams::from_env() {
        return Arc::new(s);
    }
    Arc::new(seams::Platform)
}

/// Grace's start (`main.ts`): one Grace per data folder; the Java a crash left behind killed and the leftovers of
/// runs removed; the window in the saved Appearance and bounds; the last project reopened.
pub fn run() {
    probe::init();
    let Some(data_dir) = grace_core::paths::data_dir() else {
        eprintln!("Grace has no data folder: set GRACE_USER_DATA or APPDATA");
        std::process::exit(1);
    };
    let primary = match instance::start(&data_dir) {
        Ok(instance::Start::Primary(p)) => Some(p),
        Ok(instance::Start::Secondary) => return, // the first Grace comes to the front instead
        Err(e) => {
            eprintln!("the single-instance lock failed, going on without it: {e}");
            None
        }
    };
    let vendor = grace_core::vendor::vendor_paths(&vendor_dir());
    grace_core::karate::process::kill_orphan(&data_dir.join("running.pid"), &vendor.jre_dir);
    grace_core::karate::argfile::remove_leftover_arg_files(&data_dir);
    grace_core::history::remove_leftover_runs(&data_dir);
    let appearance = grace_core::settings::load_appearance(&data_dir);
    gpui_platform::application().with_assets(assets::Assets).run(move |cx| {
        init(cx);
        // before the window, so its first frame is in the saved theme (premium UI R8.3)
        cx.set_global(theme::SavedAppearance(appearance));
        let grace = app::new_app(data_dir.clone(), vendor, seams(), cx);
        let handle = window::open(grace.clone(), data_dir, cx);
        if let Some(primary) = primary {
            let (tx, rx) = async_channel::unbounded::<()>();
            primary.listen(move || {
                let _ = tx.try_send(());
            });
            // A second start on the folder brings this window to the front.
            cx.spawn(async move |cx| {
                while rx.recv().await.is_ok() {
                    let _ = handle.update(cx, |_, window, _| window.activate_window());
                }
            })
            .detach();
        }
        grace.update(cx, |g, cx| g.start(cx));
        cx.activate(true);
    });
}
