//! `src/main/karate/debug.ts`, only what the Karate command line needs in phase P1: the listener's class name, which
//! `command::build_args` passes to a debug run. The step debugger itself (its socket and protocol) ports in phase P4
//! with the rest of `debug.ts`.

/// The class of the step debugger's listener (`listener/grace/DebugListener.java`), passed with `--listener`.
pub const DEBUG_LISTENER: &str = "grace.DebugListener";
