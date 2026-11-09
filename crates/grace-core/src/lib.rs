//! Everything of GPUI Grace without a window (migration guide, Architecture). No GPUI dependency, so its tests run
//! in plain `cargo test`. Phase P1 (#280) ports `src/main` and `src/shared` here; the plan is
//! `docs/plans/2026-11-09-gpui-p1-core.md`.
pub mod fingerprint;
pub mod karate;
pub mod model;
pub mod paths;
pub mod properties_file;
pub mod secrets;
pub mod window;
