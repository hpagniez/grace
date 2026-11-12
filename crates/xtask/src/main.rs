//! Grace's scripts, in Rust: `cargo xtask <command>` (docs/specs/2026-11-12-way-of-working.md, section 5).
//! Each command is a module of pure functions with unit tests; `main` reads the arguments and sets the exit code.
#![forbid(unsafe_code)]

mod rules;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <command>

commands:
  check-rules  the agent rules files stay within their limits";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("check-rules") => ExitCode::from(rules::run()),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
