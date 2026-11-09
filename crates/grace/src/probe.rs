//! The bench's probe (migration guide R7.2): with `GRACE_PROBE` set, Grace prints `PROBE <name> <unix ms>` lines on
//! stderr for `bench/gpui.mjs`. Off, it costs one look at the environment at start. It is in the release build,
//! since that is the build the bench measures.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ON: AtomicBool = AtomicBool::new(false);

/// Reads `GRACE_PROBE`, once, before the window opens.
pub fn init() {
    ON.store(std::env::var_os("GRACE_PROBE").is_some(), Ordering::Relaxed);
}

pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Prints the probe `name` with the time now, when the probe is on.
pub fn mark(name: &str) {
    if on() {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or_default();
        eprintln!("{}", line(name, ms));
    }
}

fn line(name: &str, ms: u128) -> String {
    format!("PROBE {name} {ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_what_the_bench_reads() {
        // bench/gpui.mjs: /^PROBE (\S+) (\d+)$/
        assert_eq!(
            line("first_frame", 1_762_640_000_123),
            "PROBE first_frame 1762640000123"
        );
    }
}
