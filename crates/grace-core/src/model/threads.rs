//! `src/shared/threads.ts`: rules for the thread count (docs/specs/2026-10-26-thread-count.md).
use super::js;
use serde_json::Value;

pub const MAX_THREADS: u32 = 32;

/// The thread count `n` is, when it is a whole number from 1 to `MAX_THREADS`.
fn in_range(n: f64) -> Option<u32> {
    (n >= 1.0 && n <= f64::from(MAX_THREADS)).then_some(n as u32)
}

/// The thread count a JSON value holds: a whole number from 1 to `MAX_THREADS`, else `None`.
///
/// The TypeScript's `validThreads` answers true or false; this gives the count itself, typed, which is what every
/// caller goes on to use. `valid_threads` is the TypeScript's.
pub fn parse_threads(n: &Value) -> Option<u32> {
    js::integer(n).and_then(in_range)
}

/// True only for a whole number from 1 to `MAX_THREADS`.
pub fn valid_threads(n: &Value) -> bool {
    parse_threads(n).is_some()
}

pub const THREADS_ERROR: &str = "Threads must be a whole number from 1 to 32";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod valid_threads {
        use super::*;

        /// One test per value, as the TypeScript's `it.each`.
        macro_rules! accepts {
            ($($name:ident: $n:expr,)*) => {
                $( #[test] fn $name() { assert!(valid_threads(&json!($n))); } )*
            };
        }

        macro_rules! rejects {
            ($($name:ident: $n:expr,)*) => {
                $( #[test] fn $name() { assert!(!valid_threads(&$n)); } )*
            };
        }

        accepts! {
            accepts_1: 1,
            accepts_8: 8,
            accepts_32: 32,
        }

        rejects! {
            rejects_0: json!(0),
            rejects_33: json!(33),
            rejects_2_5: json!(2.5),
            rejects_minus_1: json!(-1),
            rejects_the_string_4: json!("4"),
            rejects_null: Value::Null,
            // `undefined` has no JSON value: a member that is absent reads as `null` here
            rejects_undefined: Value::Null,
        }

        // `NaN` and `Infinity` are no JSON values (`json!(f64::NAN)` is `null`, and `serde_json` refuses to read one):
        // they are checked on the number the rule is made of.
        #[test]
        fn rejects_nan() {
            assert_eq!(in_range(f64::NAN), None);
            assert!(!valid_threads(&json!(f64::NAN)));
        }

        #[test]
        fn rejects_infinity() {
            assert_eq!(in_range(f64::INFINITY), None);
            assert!(!valid_threads(&json!(f64::INFINITY)));
        }

        #[test]
        fn the_refusal_notice_names_the_range() {
            assert_eq!(MAX_THREADS, 32);
            assert_eq!(THREADS_ERROR, "Threads must be a whole number from 1 to 32");
        }

        /// Not in the TypeScript: the typed count, and a whole number written with a fraction.
        #[test]
        fn parse_threads_gives_the_count() {
            assert_eq!(parse_threads(&json!(8)), Some(8));
            assert_eq!(parse_threads(&json!(8.0)), Some(8));
            assert_eq!(parse_threads(&json!(33)), None);
        }
    }
}
