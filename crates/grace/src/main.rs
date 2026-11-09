// No console window on Windows in a release build.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    gpui_platform::application()
        .with_assets(grace::assets::Assets)
        .run(|cx| {
            grace::init(cx);
            grace::window::open(cx);
            cx.activate(true);
        });
}
