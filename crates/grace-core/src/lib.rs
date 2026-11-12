//! Everything of GPUI Grace without a window (migration guide, Architecture). No GPUI dependency, so its tests run
//! in plain `cargo test`. Phase P1 (#280) ports `src/main` and `src/shared` here; the plan is
//! `docs/plans/2026-11-09-gpui-p1-core.md`.
pub mod banner;
pub mod batch;
pub mod console_find;
pub mod editor;
pub mod fingerprint;
pub mod flaky;
pub mod history;
pub mod import_plan;
pub mod karate;
pub mod mask;
pub mod model;
pub mod panel_height;
pub mod panel_tabs;
pub mod paths;
pub mod properties_draft;
pub mod properties_file;
pub mod recent;
pub mod rows;
pub mod runner;
pub mod secrets;
pub mod settings;
pub mod state;
pub mod trends;
pub mod vendor;
pub mod window;
