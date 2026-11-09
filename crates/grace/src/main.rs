// No console window on Windows in a release build.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    grace::run();
}
