//! `src/shared/flaky.ts`: what the window gets about flaky tests (docs/specs/2026-10-26-flaky-tests.md).
use super::js;
use serde::{Deserialize, Serialize};

/// `passed` or `failed`: the statuses that count for flaky detection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlakyStatus {
    Passed,
    Failed,
}

/// One run that proves a test flaky: the test `passed` or `failed` in it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakyRun {
    pub id: String,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    pub status: FlakyStatus,
    pub threads: Option<u32>,
    pub report_available: bool,
}

/// The proof of one environment (None: none): the runs with the same project files, newest first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakyEnv {
    pub env: Option<String>,
    pub passed: u32,
    pub failed: u32,
    pub runs: Vec<FlakyRun>,
}

/// A test of the tree last sent that is flaky, by its key in that tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlakyTest {
    pub key: String,
    pub envs: Vec<FlakyEnv>,
}

pub const FLAKY_TITLE: &str = "Flaky: passed and failed with the same project files";

pub fn flaky_heading(env: Option<&str>) -> String {
    match env {
        None => "Flaky with no environment".to_string(),
        Some(env) => format!("Flaky on {env}"),
    }
}

fn times(n: u32) -> String {
    format!("{n} {}", if n == 1 { "time" } else { "times" })
}

pub fn flaky_counts(passed: u32, failed: u32) -> String {
    format!(
        "Passed {}, failed {} with the same project files",
        times(passed),
        times(failed)
    )
}

/// "1 thread", "8 threads", or an em dash (U+2014) for a run stored before the thread count existed.
pub fn threads_label(n: Option<u32>) -> String {
    match n {
        None => "\u{2014}".to_string(),
        Some(1) => "1 thread".to_string(),
        Some(n) => format!("{n} threads"),
    }
}

/// The current environment first, then none, then the rest by name.
///
/// Names compare as `localeCompare` does with no locale given (`js::locale_compare`: exact for the ASCII an
/// environment's name is made of). Differs from the TypeScript in giving the environments by reference, in the new
/// order: the input is untouched and nothing is copied.
pub fn order_envs<'a>(envs: &'a [FlakyEnv], current: Option<&str>) -> Vec<&'a FlakyEnv> {
    let rank = |e: &FlakyEnv| match e.env.as_deref() {
        env if env == current => 0,
        None => 1,
        Some(_) => 2,
    };
    let name = |e: &FlakyEnv| e.env.as_deref().unwrap_or("").to_string();
    let mut ordered: Vec<&FlakyEnv> = envs.iter().collect();
    // `sort_by` is stable, as JavaScript's `sort` is
    ordered.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| js::locale_compare(&name(a), &name(b)))
    });
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: Option<&str>) -> FlakyEnv {
        FlakyEnv {
            env: name.map(String::from),
            passed: 1,
            failed: 1,
            runs: vec![],
        }
    }

    mod flaky_labels {
        use super::*;

        #[test]
        fn words_the_mark() {
            assert_eq!(FLAKY_TITLE, "Flaky: passed and failed with the same project files");
        }

        #[test]
        fn heads_a_box_with_its_environment_or_says_there_is_none() {
            assert_eq!(flaky_heading(Some("qa")), "Flaky on qa");
            assert_eq!(flaky_heading(None), "Flaky with no environment");
        }

        #[test]
        fn counts_the_runs_with_singular_for_1() {
            assert_eq!(
                flaky_counts(1, 1),
                "Passed 1 time, failed 1 time with the same project files"
            );
            assert_eq!(
                flaky_counts(3, 2),
                "Passed 3 times, failed 2 times with the same project files"
            );
            assert_eq!(
                flaky_counts(1, 2),
                "Passed 1 time, failed 2 times with the same project files"
            );
        }

        #[test]
        fn words_the_thread_count() {
            assert_eq!(threads_label(Some(1)), "1 thread");
            assert_eq!(threads_label(Some(8)), "8 threads");
            assert_eq!(threads_label(None), "\u{2014}");
        }
    }

    mod order_envs {
        use super::*;

        fn names<'a>(envs: &[&'a FlakyEnv]) -> Vec<Option<&'a str>> {
            envs.iter().map(|e| e.env.as_deref()).collect()
        }

        #[test]
        fn puts_the_current_environment_first_then_none_then_the_rest_by_name() {
            let envs = [env(Some("dev")), env(None), env(Some("qa"))];
            assert_eq!(names(&order_envs(&envs, Some("qa"))), [Some("qa"), None, Some("dev")]);
            assert_eq!(names(&order_envs(&envs, None)), [None, Some("dev"), Some("qa")]);
            let other = [env(Some("qa")), env(Some("dev")), env(Some("alpha"))];
            assert_eq!(
                names(&order_envs(&other, Some("prod"))),
                [Some("alpha"), Some("dev"), Some("qa")]
            );
        }

        #[test]
        fn leaves_its_input_as_it_was() {
            let envs = [env(Some("dev")), env(None), env(Some("qa"))];
            order_envs(&envs, Some("qa"));
            let after: Vec<Option<&str>> = envs.iter().map(|e| e.env.as_deref()).collect();
            assert_eq!(after, [Some("dev"), None, Some("qa")]);
        }

        /// Not in the TypeScript: `localeCompare` ignores case first (`beta` before `Gamma`), puts lower case before
        /// upper case among equals, and ranks `_` before `-` before `.` before digits before letters.
        #[test]
        fn names_sort_as_locale_compare_does() {
            let envs = ["Gamma", "beta", "Dev", "dev", "a1", "a.b"].map(|n| env(Some(n)));
            assert_eq!(
                names(&order_envs(&envs, None)),
                [
                    Some("a.b"),
                    Some("a1"),
                    Some("beta"),
                    Some("dev"),
                    Some("Dev"),
                    Some("Gamma")
                ]
            );
        }
    }
}
