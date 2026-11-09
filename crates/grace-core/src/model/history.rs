//! `src/shared/history.ts`: what a finished run stores and how the History tab words it
//! (docs/specs/2026-10-25-run-history.md).
use super::env::valid_env_name;
use super::events::RunOutcome;
use super::js;
use super::node::NodeKind;
use super::tags::{FilterState, parse_filter_state};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HISTORY_LIMIT: usize = 20;

/// `RUN_ID`, `/^\d{8}-\d{6}-\d{3}(-\d+)?$/` (`\d` is ASCII only): the folder name of a run, its local start time
/// `yyyyMMdd-HHmmss-SSS`, plus `-2`, `-3`… if taken. Nothing else is a run's folder: not `..`, not a path.
pub fn valid_run_id(id: &str) -> bool {
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    let mut parts = id.split('-');
    let [Some(date), Some(time), Some(millis), taken, None] =
        [parts.next(), parts.next(), parts.next(), parts.next(), parts.next()]
    else {
        return false;
    };
    digits(date, 8)
        && digits(time, 6)
        && digits(millis, 3)
        && taken.is_none_or(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
}

/// `FINGERPRINT`, `/^[0-9a-f]{64}$/`: a project fingerprint, SHA-256 as lowercase hex.
pub fn valid_fingerprint(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A test's status once its run has ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestStatusFinal {
    Passed,
    Failed,
    NotRun,
    Interrupted,
}

/// `scenario` or `row`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LeafKind {
    Scenario,
    Row,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTest {
    pub kind: LeafKind,
    pub file: String,
    pub ref_id: String,
    pub name: String,
    pub line: Option<u32>,
    pub key: String,
    pub status: TestStatusFinal,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "crate::model::js_num_opt")]
    pub duration_millis: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredSelection {
    pub kind: NodeKind,
    pub file: String,
    pub name: String,
    pub line: Option<u32>,
    pub key: String,
}

/// `run.json`. Never holds a property value, an error text, a step log or a console line. Its keys are written in
/// the order of the object `runner.ts` writes: `version` to `tests` as below, then `debug` last, only when set
/// (`history::tests::run_json_text` checks the text against `JSON.stringify`'s).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    /// Always 1.
    pub version: u32,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    #[serde(with = "crate::model::js_num")]
    pub duration_millis: f64,
    pub env: Option<String>,
    /// The count passed with `-T`; None for a run stored before F12.
    pub threads: Option<u32>,
    /// SHA-256 of the project files' paths, sizes and times, the same before Java started and after it exited (F15).
    pub fingerprint: Option<String>,
    pub filter: FilterState,
    /// None for a Re-run failed, whose `failed_of` is then the `started_at` of its source run.
    pub selection: Option<StoredSelection>,
    #[serde(with = "crate::model::js_num_opt")]
    pub failed_of: Option<f64>,
    pub outcome: RunOutcome,
    /// What Java exited with. A JavaScript number, which on Windows can be an unsigned 32-bit code such as
    /// 3221225786, so it is an `i64` and not an `i32`.
    pub exit_code: Option<i64>,
    pub tests: Vec<StoredTest>,
    /// A step debugger run; absent for any other. Flaky detection leaves it out (step debugger R2.7). Written last:
    /// `runner.ts` adds it after `tests`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub debug: bool,
}

/// A listed run as the window gets it: the record without `tests`, with counts and its folder id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    #[serde(with = "crate::model::js_num")]
    pub started_at: f64,
    #[serde(with = "crate::model::js_num")]
    pub duration_millis: f64,
    pub env: Option<String>,
    pub threads: Option<u32>,
    pub filter: FilterState,
    pub selection: Option<StoredSelection>,
    #[serde(with = "crate::model::js_num_opt")]
    pub failed_of: Option<f64>,
    pub outcome: RunOutcome,
    /// As `RunRecord::exit_code`.
    pub exit_code: Option<i64>,
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
    pub report_available: bool,
    /// Electron Grace's `toEntry` keeps `debug` from the record.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub debug: bool,
}

/// A string member. The TypeScript's `str`.
fn string(x: Option<&Value>) -> Option<String> {
    x?.as_str().map(String::from)
}

/// A line of the TypeScript's `numOrNull`, which takes any finite number; the model's lines are `u32`, so a fraction, a
/// negative number or one past `u32::MAX` is refused here (Grace writes none: they are Karate's line numbers).
/// `Some(None)` is a `null` line.
fn line(x: Option<&Value>) -> Option<Option<u32>> {
    match x? {
        Value::Null => Some(None),
        n => js::unsigned(n, u64::from(u32::MAX)).map(|n| Some(n as u32)),
    }
}

/// The TypeScript's `numOrNull`, for a time: `Some(None)` is a `null`.
fn finite_or_null(x: Option<&Value>) -> Option<Option<f64>> {
    match x? {
        Value::Null => Some(None),
        n => js::finite(n).map(Some),
    }
}

/// The TypeScript's `numOrNull`, for an exit code, which the model keeps as a whole number (`i64`): a fraction is
/// refused here.
fn exit_code(x: Option<&Value>) -> Option<Option<i64>> {
    match x? {
        Value::Null => Some(None),
        n => js::integer(n).filter(|n| n.abs() < 9.0e18).map(|n| Some(n as i64)),
    }
}

fn parse_test(x: &Value) -> Option<StoredTest> {
    let o = x.as_object()?;
    let kind = match o.get("kind")?.as_str()? {
        "scenario" => LeafKind::Scenario,
        "row" => LeafKind::Row,
        _ => return None,
    };
    let status = match o.get("status")?.as_str()? {
        "passed" => TestStatusFinal::Passed,
        "failed" => TestStatusFinal::Failed,
        "notRun" => TestStatusFinal::NotRun,
        "interrupted" => TestStatusFinal::Interrupted,
        _ => return None,
    };
    // present, `durationMillis` must be a number (`null` is not)
    let duration_millis = match o.get("durationMillis") {
        None => None,
        Some(d) => Some(js::finite(d)?),
    };
    Some(StoredTest {
        kind,
        file: string(o.get("file"))?,
        ref_id: string(o.get("refId"))?,
        name: string(o.get("name"))?,
        line: line(o.get("line"))?,
        key: string(o.get("key"))?,
        status,
        duration_millis,
    })
}

fn parse_selection(x: &Value) -> Option<StoredSelection> {
    let o = x.as_object()?;
    let kind = match o.get("kind")?.as_str()? {
        "folder" => NodeKind::Folder,
        "feature" => NodeKind::Feature,
        "outline" => NodeKind::Outline,
        "scenario" => NodeKind::Scenario,
        "row" => NodeKind::Row,
        _ => return None,
    };
    Some(StoredSelection {
        kind,
        file: string(o.get("file"))?,
        name: string(o.get("name"))?,
        line: line(o.get("line"))?,
        key: string(o.get("key"))?,
    })
}

/// Validates a `run.json` read from disk; `None` for anything that is not a version 1 record.
///
/// Every key of the TypeScript's validator is a rule here, with these narrowings to the model's types: a `line` and a
/// `threads` count must fit `u32`, and an `exitCode` must be a whole number (the TypeScript takes any finite number).
pub fn parse_run_record(x: &Value) -> Option<RunRecord> {
    let o = x.as_object()?;
    if o.get("version")?.as_f64() != Some(1.0) {
        return None;
    }
    let started_at = js::finite(o.get("startedAt")?)?;
    let duration_millis = js::finite(o.get("durationMillis")?)?;
    let failed_of = finite_or_null(o.get("failedOf"))?;
    let exit_code = exit_code(o.get("exitCode"))?;
    let env = match o.get("env")? {
        Value::Null => None,
        Value::String(s) if valid_env_name(s) => Some(s.clone()),
        _ => return None,
    };
    // a record stored before F12 has no `threads`
    let threads = match o.get("threads") {
        None | Some(Value::Null) => None,
        Some(t) => Some(js::unsigned(t, u64::from(u32::MAX)).filter(|n| *n >= 1)? as u32),
    };
    // a record stored before F15 has no `fingerprint`
    let fingerprint = match o.get("fingerprint") {
        None | Some(Value::Null) => None,
        Some(Value::String(f)) if valid_fingerprint(f) => Some(f.clone()),
        Some(_) => return None,
    };
    let outcome = match o.get("outcome")?.as_str()? {
        "finished" => RunOutcome::Finished,
        "cancelled" => RunOutcome::Cancelled,
        "failed" => RunOutcome::Failed,
        _ => return None,
    };
    let filter = parse_filter_state(o.get("filter")?)?;
    let selection = match o.get("selection")? {
        Value::Null => None,
        s => Some(parse_selection(s)?),
    };
    // every entry must parse: a hole in a sparse array is `null` once it is JSON, and `null` is no test
    let tests = o
        .get("tests")?
        .as_array()?
        .iter()
        .map(parse_test)
        .collect::<Option<Vec<_>>>()?;
    let debug = match o.get("debug") {
        None => false,
        Some(Value::Bool(true)) => true,
        Some(_) => return None,
    };
    Some(RunRecord {
        debug,
        version: 1,
        started_at,
        duration_millis,
        env,
        threads,
        fingerprint,
        filter,
        selection,
        failed_of,
        outcome,
        exit_code,
        tests,
    })
}

/// The listed form of a stored run: its counts, and no `tests` or `fingerprint`.
///
/// Takes the record, where the TypeScript copies what it keeps from it: its filter, selection and env move into the
/// entry.
pub fn to_entry(id: &str, r: RunRecord, report_available: bool) -> HistoryEntry {
    let count = |s: TestStatusFinal| r.tests.iter().filter(|t| t.status == s).count() as u32;
    let (passed, failed, total) = (
        count(TestStatusFinal::Passed),
        count(TestStatusFinal::Failed),
        r.tests.len() as u32,
    );
    HistoryEntry {
        id: id.to_string(),
        started_at: r.started_at,
        duration_millis: r.duration_millis,
        env: r.env,
        threads: r.threads,
        filter: r.filter,
        selection: r.selection,
        failed_of: r.failed_of,
        outcome: r.outcome,
        exit_code: r.exit_code,
        passed,
        failed,
        total,
        report_available,
        debug: r.debug,
    }
}

/// The local date and time of a JavaScript time value in milliseconds; `None` where `new Date(ms)` is an Invalid Date
/// (not a number, or past 8.64e15 ms) and, differing from JavaScript, where `chrono` has no date: past the year
/// 262,142, or before -262,143, where JavaScript's range runs to the year 275,760. No run is dated in them. The
/// fraction of a millisecond is dropped, as JavaScript drops it.
fn local(ms: f64) -> Option<DateTime<Local>> {
    if ms.is_nan() || ms.abs() > 8.64e15 {
        return None;
    }
    Local.timestamp_millis_opt(ms.trunc() as i64).single()
}

/// `yyyy-MM-dd` of a local date as the TypeScript writes it (the year as JavaScript prints it); an Invalid Date is
/// `NaN-NaN-NaN`, as `String(NaN)` makes it.
fn day(d: Option<NaiveDate>) -> String {
    d.map_or_else(
        || "NaN-NaN-NaN".to_string(),
        |d| format!("{}-{:02}-{:02}", d.year(), d.month(), d.day()),
    )
}

/// "Today 14:32", "Yesterday 17:20", else "2026-10-21 09:05", in local time. `started_at` and `now` are JavaScript
/// times: milliseconds since the epoch.
pub fn when_label(started_at: f64, now: f64) -> String {
    let d = local(started_at);
    let today = local(now).map(|t| t.date_naive());
    let yesterday = today.and_then(|t| t.pred_opt());
    let date = day(d.map(|d| d.date_naive()));
    let time = d.map_or_else(
        || "NaN:NaN".to_string(),
        |d| format!("{:02}:{:02}", d.hour(), d.minute()),
    );
    if date == day(today) {
        return format!("Today {time}");
    }
    format!(
        "{} {time}",
        if date == day(yesterday) {
            "Yesterday"
        } else {
            date.as_str()
        }
    )
}

/// What ran: "All", a folder or feature path, a test's name, or "Failed tests" of a re-run.
pub fn what_ran_label(e: &HistoryEntry, now: f64) -> String {
    match &e.selection {
        None => match e.failed_of {
            None => "Failed tests".to_string(),
            Some(at) => format!("Failed tests of {}", when_label(at, now)),
        },
        Some(s) => match s.kind {
            NodeKind::Folder if s.file.is_empty() => "All".to_string(),
            NodeKind::Folder | NodeKind::Feature => s.file.clone(),
            _ => s.name.clone(),
        },
    }
}

pub fn result_label(e: &HistoryEntry) -> String {
    if e.outcome == RunOutcome::Failed {
        return match e.exit_code {
            None => "Error".to_string(),
            Some(code) => format!("Error (exit {code})"),
        };
    }
    let counts = format!("{} passed, {} failed of {}", e.passed, e.failed, e.total);
    if e.outcome == RunOutcome::Cancelled {
        format!("Cancelled, {counts}")
    } else {
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `new Date(year, month - 1, day, hour, minute).getTime()`: a local time, so the tests mean the same in every
    /// time zone. A time that does not exist (a clock change skipped it) is read an hour on, as JavaScript reads it.
    fn local_ms(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> f64 {
        let at = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, 0)
            .unwrap();
        let found = Local.from_local_datetime(&at).earliest();
        found
            .unwrap_or_else(|| {
                Local
                    .from_local_datetime(&(at + chrono::Duration::hours(1)))
                    .earliest()
                    .unwrap()
            })
            .timestamp_millis() as f64
    }

    /// The R2 example of docs/specs/2026-10-25-run-history.md.
    fn spec_example() -> RunRecord {
        RunRecord {
            debug: false,
            version: 1,
            started_at: 1792937525123.0,
            duration_millis: 72000.0,
            env: Some("qa".into()),
            threads: Some(4),
            fingerprint: Some("ab".repeat(32)),
            filter: FilterState {
                include: vec!["smoke".into()],
                exclude: vec!["slow".into()],
                hide_non_matching: false,
            },
            selection: Some(StoredSelection {
                kind: NodeKind::Feature,
                file: "tests/users.feature".into(),
                name: "users".into(),
                line: None,
                key: "f:tests/users.feature".into(),
            }),
            failed_of: None,
            outcome: RunOutcome::Finished,
            exit_code: Some(1),
            tests: vec![
                StoredTest {
                    kind: LeafKind::Scenario,
                    file: "tests/users.feature".into(),
                    ref_id: "[1:6]".into(),
                    name: "create user".into(),
                    line: Some(6),
                    key: "s:tests/users.feature#1".into(),
                    status: TestStatusFinal::Passed,
                    duration_millis: Some(412.0),
                },
                StoredTest {
                    kind: LeafKind::Row,
                    file: "tests/users.feature".into(),
                    ref_id: "[2.1:14]".into(),
                    name: "age 3".into(),
                    line: Some(21),
                    key: "s:tests/users.feature#2.1".into(),
                    status: TestStatusFinal::Failed,
                    duration_millis: Some(95.0),
                },
            ],
        }
    }

    fn spec_json() -> Value {
        serde_json::to_value(spec_example()).unwrap()
    }

    /// The spec example's JSON with `key` set to `value`.
    fn with(key: &str, value: Value) -> Value {
        let mut x = spec_json();
        x[key] = value;
        x
    }

    /// The spec example's JSON without `key`.
    fn without(key: &str) -> Value {
        let mut x = spec_json();
        x.as_object_mut().unwrap().remove(key);
        x
    }

    fn test(status: TestStatusFinal, n: u32) -> StoredTest {
        StoredTest {
            kind: LeafKind::Scenario,
            file: "tests/x.feature".into(),
            ref_id: format!("[{n}:{n}]"),
            name: format!("t{n}"),
            line: Some(n),
            key: format!("s:tests/x.feature#{n}"),
            status,
            duration_millis: None,
        }
    }

    fn entry() -> HistoryEntry {
        HistoryEntry {
            id: "20261025-143205-123".into(),
            started_at: local_ms(2026, 10, 25, 14, 32),
            duration_millis: 1000.0,
            env: None,
            threads: Some(1),
            filter: FilterState::default(),
            selection: Some(StoredSelection {
                kind: NodeKind::Folder,
                file: "".into(),
                name: "clean".into(),
                line: None,
                key: "d:".into(),
            }),
            failed_of: None,
            outcome: RunOutcome::Finished,
            exit_code: Some(0),
            passed: 41,
            failed: 2,
            total: 43,
            report_available: true,
            debug: false,
        }
    }

    mod run_id {
        use super::*;

        #[test]
        fn accepts_ids_and_rejects_traversal() {
            assert!(valid_run_id("20261025-143205-123"));
            assert!(valid_run_id("20261025-143205-123-2"));
            for bad in ["..", "20261025-143205-123\\..", "x/20261025-143205-123", ""] {
                assert!(!valid_run_id(bad), "{bad:?}");
            }
        }

        /// Not in the TypeScript: the rest of what the pattern says.
        #[test]
        fn is_the_pattern() {
            for bad in [
                "20261025-143205-123-",
                "20261025-143205-123-2-3",
                "20261025-143205-12",
                "2026102-143205-123",
                "20261025-14320-123",
                "20261025-143205-123\n",
                "20261025_143205-123",
                "٢٠٢٦١٠٢٥-143205-123",
            ] {
                assert!(!valid_run_id(bad), "{bad:?}");
            }
            assert!(valid_run_id("20261025-143205-123-10"));
        }

        #[test]
        fn fingerprint_is_64_lowercase_hex() {
            assert!(valid_fingerprint(&"ab".repeat(32)));
            for bad in [&"AB".repeat(32), "ab", &"g".repeat(64), &"a".repeat(65), ""] {
                assert!(!valid_fingerprint(bad), "{bad:?}");
            }
        }
    }

    mod parse_run_record {
        use super::*;

        #[test]
        fn accepts_the_spec_example() {
            // through JSON text, as `JSON.parse(JSON.stringify(...))`
            let text = serde_json::to_string(&spec_example()).unwrap();
            assert_eq!(
                parse_run_record(&serde_json::from_str(&text).unwrap()),
                Some(spec_example())
            );
        }

        #[test]
        fn accepts_a_re_run_failed_record_with_no_selection_and_exit_code() {
            let r = RunRecord {
                selection: None,
                failed_of: Some(1792937525123.0),
                exit_code: None,
                ..spec_example()
            };
            assert_eq!(parse_run_record(&serde_json::to_value(&r).unwrap()), Some(r));
        }

        #[test]
        fn keeps_a_debug_runs_flag_and_refuses_any_other_value() {
            assert_eq!(
                parse_run_record(&with("debug", json!(true))),
                Some(RunRecord {
                    debug: true,
                    ..spec_example()
                })
            );
            assert_eq!(parse_run_record(&with("debug", json!(false))), None);
        }

        #[test]
        fn reads_a_record_stored_before_f12_as_no_thread_count() {
            let none = RunRecord {
                threads: None,
                ..spec_example()
            };
            assert_eq!(parse_run_record(&without("threads")), Some(none.clone()));
            assert_eq!(parse_run_record(&with("threads", Value::Null)), Some(none));
        }

        #[test]
        fn reads_a_record_stored_before_f15_as_no_fingerprint() {
            let none = RunRecord {
                fingerprint: None,
                ..spec_example()
            };
            assert_eq!(parse_run_record(&without("fingerprint")), Some(none.clone()));
            assert_eq!(parse_run_record(&with("fingerprint", Value::Null)), Some(none));
        }

        #[test]
        fn rejects_bad_shapes() {
            let tests = |tests: Vec<Value>| with("tests", Value::Array(tests));
            let t = |status: TestStatusFinal, n: u32| serde_json::to_value(test(status, n)).unwrap();
            let bad = [
                Value::Null,
                json!("x"),
                with("version", json!(2)),
                with("tests", json!({})),
                tests(vec![{
                    let mut x = t(TestStatusFinal::Passed, 1);
                    x["status"] = json!("pending");
                    x
                }]),
                tests(vec![t(TestStatusFinal::Passed, 1), {
                    let mut x = t(TestStatusFinal::Passed, 2);
                    x["line"] = json!("2");
                    x
                }]),
                with("env", json!("../x")),
                with(
                    "filter",
                    json!({ "include": ["a b"], "exclude": [], "hideNonMatching": false }),
                ),
                {
                    let mut selection = spec_json()["selection"].clone();
                    selection["kind"] = json!("suite");
                    with("selection", selection)
                },
                with("outcome", json!("done")),
                with("exitCode", json!("1")),
                with("startedAt", json!("now")),
                with("threads", json!(0)),
                with("threads", json!("2")),
                with("threads", json!(1.5)),
                with("fingerprint", json!("AB".repeat(32))),
                with("fingerprint", json!("ab")),
                with("fingerprint", json!(1)),
                with("fingerprint", json!("g".repeat(64))),
                without("outcome"),
            ];
            for b in bad {
                assert_eq!(parse_run_record(&b), None, "{b}");
            }
        }

        /// Not in the TypeScript: the model's narrower types. A whole number written with a fraction (`6.0`) is the
        /// number `6`, as in JavaScript; a line or exit code that is not whole, or a line or thread count past `u32`, is
        /// refused; an exit code past 32 bits is kept.
        #[test]
        fn reads_numbers_as_javascript_does_within_the_models_types() {
            let r = parse_run_record(&with("exitCode", json!(3221225786_u64))).unwrap();
            assert_eq!(r.exit_code, Some(3221225786));
            assert_eq!(
                parse_run_record(&with("exitCode", json!(-1))).unwrap().exit_code,
                Some(-1)
            );
            assert_eq!(
                parse_run_record(&with("exitCode", json!(2.0))).unwrap().exit_code,
                Some(2)
            );
            assert_eq!(parse_run_record(&with("exitCode", json!(1.5))), None);
            assert_eq!(parse_run_record(&with("threads", json!(4.0))).unwrap().threads, Some(4));
            assert_eq!(parse_run_record(&with("threads", json!(4_294_967_296_u64))), None);
            assert_eq!(parse_run_record(&without("exitCode")), None);
            assert_eq!(parse_run_record(&without("env")), None);
            assert_eq!(parse_run_record(&without("failedOf")), None);
            assert_eq!(parse_run_record(&without("selection")), None);
            assert_eq!(parse_run_record(&without("filter")), None);
            assert_eq!(parse_run_record(&without("tests")), None);
        }

        /// Not in the TypeScript: a test row's own rules. `line` and `durationMillis` must be present and a number.
        #[test]
        fn checks_each_test_row() {
            let row = |f: &dyn Fn(&mut Value)| {
                let mut x = serde_json::to_value(test(TestStatusFinal::Passed, 1)).unwrap();
                f(&mut x);
                parse_run_record(&with("tests", json!([x])))
            };
            assert!(row(&|_| {}).is_some());
            assert!(row(&|x| x["line"] = Value::Null).is_some());
            assert!(row(&|x| x["durationMillis"] = json!(1.5)).is_some());
            assert!(row(&|x| x["durationMillis"] = Value::Null).is_none());
            assert!(row(&|x| drop(x.as_object_mut().unwrap().remove("line"))).is_none());
            assert!(row(&|x| x["kind"] = json!("outline")).is_none());
            assert!(row(&|x| x["refId"] = json!(1)).is_none());
            // a hole of a sparse list, which JSON writes as `null`, is no test
            assert_eq!(parse_run_record(&with("tests", json!([null]))), None);
        }
    }

    mod to_entry {
        use super::*;

        #[test]
        fn counts_from_tests() {
            let tests = [
                [1, 2, 3].map(|n| test(TestStatusFinal::Passed, n)).to_vec(),
                [4, 5].map(|n| test(TestStatusFinal::Failed, n)).to_vec(),
                vec![test(TestStatusFinal::NotRun, 6), test(TestStatusFinal::Interrupted, 7)],
            ]
            .concat();
            let e = to_entry(
                "20261025-143205-123",
                RunRecord {
                    tests,
                    ..spec_example()
                },
                false,
            );
            assert_eq!([e.passed, e.failed, e.total], [3, 2, 7]);
            assert_eq!(e.id, "20261025-143205-123");
            assert_eq!(e.threads, Some(4));
            assert!(!e.report_available);
            let json = serde_json::to_value(&e).unwrap();
            assert!(json.get("tests").is_none());
            assert!(json.get("fingerprint").is_none());
        }

        /// Not in the TypeScript: what the entry takes from the record, `debug` included.
        #[test]
        fn keeps_the_rest_of_the_record() {
            let r = RunRecord {
                debug: true,
                ..spec_example()
            };
            let e = to_entry("id", r.clone(), true);
            assert_eq!(
                (e.started_at, e.duration_millis, e.exit_code, e.outcome),
                (r.started_at, r.duration_millis, r.exit_code, r.outcome)
            );
            assert_eq!(
                (e.env, e.filter, e.selection, e.failed_of, e.debug),
                (r.env, r.filter, r.selection, r.failed_of, true)
            );
            assert!(e.report_available);
        }
    }

    mod when_label {
        use super::*;

        #[test]
        fn says_today_for_the_same_day() {
            let now = local_ms(2026, 10, 25, 16, 0);
            assert_eq!(when_label(local_ms(2026, 10, 25, 14, 32), now), "Today 14:32");
        }

        #[test]
        fn says_yesterday_across_midnight() {
            let just_after_midnight = local_ms(2026, 10, 25, 0, 1);
            assert_eq!(
                when_label(local_ms(2026, 10, 24, 23, 59), just_after_midnight),
                "Yesterday 23:59"
            );
        }

        #[test]
        fn gives_the_date_for_older_runs() {
            let now = local_ms(2026, 10, 25, 16, 0);
            assert_eq!(when_label(local_ms(2026, 10, 21, 9, 5), now), "2026-10-21 09:05");
        }

        /// Not in the TypeScript: an Invalid Date reads as JavaScript prints it.
        #[test]
        fn words_a_time_that_is_no_date_as_javascript_does() {
            let now = local_ms(2026, 10, 25, 16, 0);
            assert_eq!(when_label(f64::NAN, now), "NaN-NaN-NaN NaN:NaN");
            assert_eq!(when_label(9e15, now), "NaN-NaN-NaN NaN:NaN");
            assert_eq!(when_label(f64::NAN, f64::NAN), "Today NaN:NaN");
        }
    }

    mod what_ran_label {
        use super::*;

        fn sel(kind: NodeKind, file: &str, name: &str) -> HistoryEntry {
            HistoryEntry {
                selection: Some(StoredSelection {
                    kind,
                    file: file.into(),
                    name: name.into(),
                    line: None,
                    key: "k".into(),
                }),
                ..entry()
            }
        }

        #[test]
        fn names_what_ran() {
            let now = local_ms(2026, 10, 25, 16, 0);
            assert_eq!(what_ran_label(&entry(), now), "All");
            assert_eq!(
                what_ran_label(&sel(NodeKind::Folder, "tests/api", "api"), now),
                "tests/api"
            );
            assert_eq!(
                what_ran_label(&sel(NodeKind::Feature, "tests/users.feature", "users"), now),
                "tests/users.feature"
            );
            assert_eq!(
                what_ran_label(&sel(NodeKind::Row, "tests/users.feature", "age 3"), now),
                "age 3"
            );
        }

        #[test]
        fn says_failed_tests_of_when_for_a_re_run_failed() {
            let now = local_ms(2026, 10, 25, 16, 0);
            let failed_of = Some(local_ms(2026, 10, 25, 14, 32));
            let e = HistoryEntry {
                selection: None,
                failed_of,
                ..entry()
            };
            assert_eq!(what_ran_label(&e, now), "Failed tests of Today 14:32");
            assert_eq!(
                what_ran_label(&HistoryEntry { failed_of: None, ..e }, now),
                "Failed tests"
            );
        }
    }

    mod result_label {
        use super::*;

        #[test]
        fn counts_a_finished_run() {
            assert_eq!(result_label(&entry()), "41 passed, 2 failed of 43");
        }

        #[test]
        fn counts_a_cancelled_run() {
            let e = HistoryEntry {
                outcome: RunOutcome::Cancelled,
                passed: 3,
                failed: 1,
                ..entry()
            };
            assert_eq!(result_label(&e), "Cancelled, 3 passed, 1 failed of 43");
        }

        #[test]
        fn reports_an_error_with_its_exit_code() {
            let failed = |exit_code| HistoryEntry {
                outcome: RunOutcome::Failed,
                exit_code,
                ..entry()
            };
            assert_eq!(result_label(&failed(Some(3))), "Error (exit 3)");
            assert_eq!(result_label(&failed(None)), "Error");
        }
    }
}
