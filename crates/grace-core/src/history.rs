//! `src/main/history.ts`: run history on disk. `<data dir>/projects/<hash>/runs/<id>/` holds Karate's output and our
//! `run.json`, one folder per run, the newest 20 kept (ADR 0010, docs/specs/2026-10-25-run-history.md). The caller
//! gives the project's folder (`<data dir>/projects/<hash>`); this module does not know how it is named.
//!
//! Where it differs from a line-for-line port (plan P1, Redesigns):
//!
//! - A run folder is claimed with `create_dir`, which fails when the name is taken, so two runs that start in the same
//!   millisecond cannot get one id; the TypeScript asks `existsSync` and then creates, with a gap between the two.
//! - A record is read through one open file, whose size is checked on the handle; the TypeScript stats the path, then
//!   reads it.
//! - `list_history` consumes the listed runs: no record is copied into its entry.
//! - Deleting a folder is a seam (`clear_history_with`, `prune_history_with`) so a test can hold a folder the way
//!   Windows does, which Linux does not do (a process's working folder can be deleted there).
use crate::model::history::{HISTORY_LIMIT, HistoryEntry, RunRecord, parse_run_record, to_entry, valid_run_id};
use chrono::{Datelike, Timelike};
use std::cmp::Ordering;
use std::fs::{self, File};
use std::io::{self, ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// About 200 bytes per test: room for 300k tests, still a bound on a hand-made file.
const MAX_RECORD_BYTES: u64 = 64 * 1024 * 1024;

const RECORD_FILE: &str = "run.json";
const REPORT_FILE: &str = "karate-summary.html";

/// Retries of Node's `rmSync` for a person who asked for the deletion and waits for it (`removeDir`'s default).
const DELETE_RETRIES: u32 = 5;

fn runs_dir(project_dir: &Path) -> PathBuf {
    project_dir.join("runs")
}

/// The run folder names of a project in name order (oldest first); other names are not ours. Never fails: a missing
/// or unreadable `runs` folder lists nothing, and an entry that cannot be read is skipped (the TypeScript's
/// `readdirSync` would give nothing at all). A run id is ASCII, so the byte order of the names is JavaScript's `sort()`.
fn run_ids(project_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(runs_dir(project_dir)) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = entries
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| valid_run_id(name))
        .collect();
    ids.sort();
    ids
}

/// Platform seam (plan P1, rule 5): whether a failed delete is tried again. Node's `rmSync` retries `EBUSY`,
/// `EMFILE`, `ENFILE`, `ENOTEMPTY` and `EPERM`; on Windows, where antivirus or the indexer can hold a file a moment
/// and the errors come in many forms, every error but "not found" is retried; elsewhere only a busy file and a folder
/// that is not yet empty.
fn worth_retrying(e: &io::Error) -> bool {
    cfg!(windows) || matches!(e.kind(), ErrorKind::ResourceBusy | ErrorKind::DirectoryNotEmpty)
}

/// Runs `attempt`, and again up to `max_retries` times, waiting 100 ms, 200 ms, 300 ms… before each retry, as Node's
/// `rmSync` does. `pause` is how it waits, a seam so the test does not sleep.
fn with_retries(
    max_retries: u32,
    mut attempt: impl FnMut() -> io::Result<()>,
    mut pause: impl FnMut(Duration),
) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match attempt() {
            Err(e) if tries < max_retries && worth_retrying(&e) => {
                tries += 1;
                pause(Duration::from_millis(100 * u64::from(tries)));
            }
            result => return result,
        }
    }
}

/// Node's `rmSync(path, { recursive: true, force: true })`: a folder with everything in it, a file or a link; a path
/// that is not there is deleted already.
fn remove_path(path: &Path) -> io::Result<()> {
    let removed = match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => fs::remove_file(path),
        // a link is removed itself, not what it points to
        Ok(_) => fs::remove_dir_all(path),
        Err(e) => Err(e),
    };
    match removed {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// `removeDir`: deletes `dir`, trying again `max_retries` times while it fails for a reason a moment can cure. A
/// background cleanup passes 0. False if the folder stayed. This blocks for up to 1.5 s with 5 retries.
fn remove_dir(dir: &Path, max_retries: u32) -> bool {
    #[cfg(test)]
    if held::is_held(dir) {
        return false;
    }
    remove_retrying(dir, max_retries).is_ok()
}

/// `rmSync(path, { recursive: true, force: true, maxRetries })` with its error: the runner empties its dry run folder
/// with it, and a failure there ends the discovery.
pub(crate) fn remove_retrying(path: &Path, max_retries: u32) -> io::Result<()> {
    with_retries(max_retries, || remove_path(path), thread::sleep)
}

/// `rmSync(file, { force: true, maxRetries })`: a file, never a folder (which is an error); one that is not there is
/// deleted already. The runner deletes a run's argument files with it.
pub(crate) fn remove_file_retrying(file: &Path, max_retries: u32) -> io::Result<()> {
    let attempt = || match fs::remove_file(file) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        removed => removed,
    };
    with_retries(max_retries, attempt, thread::sleep)
}

/// Folders a test of the runner holds the way Windows holds a folder a process runs in: `remove_dir` leaves them and
/// fails. Linux lets such a folder go, so the runner's tests hold it here, on the thread that asks for the delete (the
/// runner deletes the history on its caller's thread).
#[cfg(test)]
pub(crate) mod held {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};

    thread_local! {
        static HELD: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    }

    pub(crate) fn hold(dir: &Path) {
        HELD.with(|h| h.borrow_mut().push(dir.to_path_buf()));
    }

    pub(crate) fn release(dir: &Path) {
        HELD.with(|h| h.borrow_mut().retain(|d| d != dir));
    }

    pub(super) fn is_held(dir: &Path) -> bool {
        HELD.with(|h| h.borrow().iter().any(|d| d == dir))
    }
}

/// `now`'s local time as `yyyyMMdd-HHmmss-SSS`: the id of a run started then, before a suffix. Takes the local
/// date and time (a `NaiveDateTime` in a test, `Local::now()` in the app) and so never reads a time zone itself.
fn base_id(now: &(impl Datelike + Timelike)) -> String {
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        // a leap second counts a nanosecond past 1e9; `getMilliseconds()` never gives 1000
        (now.nanosecond() / 1_000_000).min(999),
    )
}

/// The ids a run started at `now` may take, in order: the time, then `-2`, `-3`…
fn candidate_ids(now: &(impl Datelike + Timelike)) -> impl Iterator<Item = String> {
    let base = base_id(now);
    std::iter::once(base.clone()).chain((2u64..).map(move |n| format!("{base}-{n}")))
}

/// Local start time `yyyyMMdd-HHmmss-SSS`; `-2`, `-3`… is appended while `taken`.
pub fn run_id(now: &(impl Datelike + Timelike), taken: impl Fn(&str) -> bool) -> String {
    candidate_ids(now)
        .find(|id| !taken(id))
        .expect("the candidates never end")
}

/// A run's folder as `create_run_dir` made it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunDir {
    pub id: String,
    pub dir: PathBuf,
}

/// Creates `runs/<id>` (and the folders above it) for a run started at `now`, local time. The id is claimed by the
/// creation itself: a name that is taken (a second run in the same millisecond) gives the next suffix.
pub fn create_run_dir(project_dir: &Path, now: &(impl Datelike + Timelike)) -> io::Result<RunDir> {
    let runs = runs_dir(project_dir);
    fs::create_dir_all(&runs)?;
    for id in candidate_ids(now) {
        let dir = runs.join(&id);
        match fs::create_dir(&dir) {
            Ok(()) => return Ok(RunDir { id, dir }),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    unreachable!("the candidates never end")
}

/// Writes `run.json` into the run's folder as `JSON.stringify(record)` writes it: no indent, the keys in
/// `RunRecord`'s order (Electron Grace's).
pub fn write_run_record(dir: &Path, record: &RunRecord) -> io::Result<()> {
    let json = serde_json::to_vec(record).map_err(io::Error::from)?;
    fs::write(dir.join(RECORD_FILE), json)
}

/// The file's text as `readFileSync(file, "utf8")` reads it: an invalid byte is U+FFFD, not an error. None for a
/// file over 64 MB, judged by the size of the file that was opened.
fn read_record_text(file: &Path) -> io::Result<Option<String>> {
    let mut f = File::open(file)?;
    let size = f.metadata()?.len();
    if size > MAX_RECORD_BYTES {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(size as usize);
    f.read_to_end(&mut bytes)?;
    Ok(Some(match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }))
}

/// None for an id that is not a run id, a missing folder or file, a file over 64 MB or an invalid record.
pub fn read_run_record(project_dir: &Path, id: &str) -> Option<RunRecord> {
    if !valid_run_id(id) {
        return None; // also keeps `..` and separators out of the path
    }
    let text = read_record_text(&runs_dir(project_dir).join(id).join(RECORD_FILE)).ok()??;
    parse_run_record(&serde_json::from_str(&text).ok()?)
}

/// A listed run: its folder id, its record and whether its report exists.
#[derive(Clone, Debug, PartialEq)]
pub struct ListedRun {
    pub id: String,
    pub record: RunRecord,
    pub report_available: bool,
}

/// The valid runs, newest `started_at` first, at most 20. Never fails. Runs with the same time stay in name order.
pub fn list_runs(project_dir: &Path) -> Vec<ListedRun> {
    let mut runs: Vec<ListedRun> = run_ids(project_dir)
        .into_iter()
        .filter_map(|id| {
            let record = read_run_record(project_dir, &id)?;
            let report_available = runs_dir(project_dir).join(&id).join(REPORT_FILE).exists();
            Some(ListedRun {
                id,
                record,
                report_available,
            })
        })
        .collect();
    // times are finite (the record's parser refuses any other), so `partial_cmp` never fails; `-0` equals `0`
    runs.sort_by(|a, b| {
        b.record
            .started_at
            .partial_cmp(&a.record.started_at)
            .unwrap_or(Ordering::Equal)
    });
    runs.truncate(HISTORY_LIMIT);
    runs
}

/// `list_runs` as the window gets it.
pub fn list_history(project_dir: &Path) -> Vec<HistoryEntry> {
    list_runs(project_dir)
        .into_iter()
        .map(|r| to_entry(&r.id, r.record, r.report_available))
        .collect()
}

/// Keeps the 20 newest folders by name, valid or not. A folder that stays is tried again after the next run; no
/// retries here, they would block the caller.
pub fn prune_history(project_dir: &Path) {
    prune_history_with(project_dir, remove_dir);
}

/// `prune_history` with the way a folder is deleted given: `remove(folder, retries)` is true if it is gone.
fn prune_history_with(project_dir: &Path, remove: impl Fn(&Path, u32) -> bool) {
    let ids = run_ids(project_dir);
    let old = ids.len().saturating_sub(HISTORY_LIMIT);
    for id in &ids[..old] {
        remove(&runs_dir(project_dir).join(id), 0);
    }
}

/// Deletes a run folder; false if it stayed. A recursive delete removes the files before it fails, and the `run.json`
/// with them, which would drop the run from the list: put the record back.
fn remove_run(dir: &Path, remove: &impl Fn(&Path, u32) -> bool) -> bool {
    let file = dir.join(RECORD_FILE);
    let record = fs::read(&file).ok();
    if remove(dir, DELETE_RETRIES) {
        return true;
    }
    if let Some(record) = record
        && dir.exists()
    {
        // the folder is then unlisted and `remove_leftover_runs` deletes it at the next start
        let _ = fs::write(&file, record);
    }
    false
}

/// Deletes every run folder; false if any stayed, and those stay listed.
pub fn clear_history(project_dir: &Path) -> bool {
    clear_history_with(project_dir, remove_dir)
}

/// `clear_history` with the way a folder is deleted given, as for `prune_history_with`.
fn clear_history_with(project_dir: &Path, remove: impl Fn(&Path, u32) -> bool) -> bool {
    let mut ok = true;
    for id in run_ids(project_dir) {
        ok = remove_run(&runs_dir(project_dir).join(id), &remove) && ok;
    }
    ok
}

/// At app start: under each `<data dir>/projects/<hash>/`, deletes run folders without `run.json` (the app crashed
/// during that run) and the `run/` folder of version 1.0. Never fails.
pub fn remove_leftover_runs(data_dir: &Path) {
    let Ok(projects) = fs::read_dir(data_dir.join("projects")) else {
        return;
    };
    for project in projects.flatten() {
        let project_dir = project.path();
        if !project_dir.is_dir() {
            continue; // a stray file has no runs, and nothing under it to delete
        }
        remove_dir(&project_dir.join("run"), DELETE_RETRIES);
        for id in run_ids(&project_dir) {
            let dir = runs_dir(&project_dir).join(id);
            if !dir.join(RECORD_FILE).exists() {
                remove_dir(&dir, DELETE_RETRIES);
            }
        }
    }
}

/// What the tests of `flaky.rs` and `trends.rs` share: the TypeScript files of both open with the same `node`, `tree`
/// and `stored`, copied; here they are written once.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::ListedRun;
    use crate::model::events::RunOutcome;
    use crate::model::history::{LeafKind, RunRecord, StoredTest, TestStatusFinal};
    use crate::model::node::{NodeKind, TestNode};
    use crate::model::tags::no_filter;

    pub fn node(kind: NodeKind, key: &str, name: &str, children: Vec<TestNode>) -> TestNode {
        TestNode {
            key: key.into(),
            kind,
            name: name.into(),
            file: "tests/a.feature".into(),
            line: None,
            ref_id: None,
            tags: Vec::new(),
            example: None,
            children,
        }
    }

    /// Feature `tests/a.feature`: the given scenarios as `(name, section)`, then the outline `rows`.
    pub fn tree(scenarios: &[(&str, u32)]) -> TestNode {
        let mut children: Vec<TestNode> = scenarios
            .iter()
            .map(|(name, n)| node(NodeKind::Scenario, &format!("s:tests/a.feature#{n}"), name, Vec::new()))
            .collect();
        children.push(node(
            NodeKind::Outline,
            "o:tests/a.feature#3",
            "rows",
            vec![
                node(NodeKind::Row, "s:tests/a.feature#3.1", "rows 1", Vec::new()),
                node(NodeKind::Row, "s:tests/a.feature#3.2", "rows 2", Vec::new()),
            ],
        ));
        let feature = node(NodeKind::Feature, "f:tests/a.feature", "a", children);
        node(NodeKind::Folder, "d:", "p", vec![feature])
    }

    /// `tree()` of the TypeScript, whose default is the scenarios `one` and `two`.
    pub fn default_tree() -> TestNode {
        tree(&[("one", 1), ("two", 2)])
    }

    pub fn stored(kind: LeafKind, name: &str, section: &str, status: TestStatusFinal) -> StoredTest {
        StoredTest {
            kind,
            file: "tests/a.feature".into(),
            ref_id: format!("[{section}:1]"),
            name: name.into(),
            line: Some(1),
            key: format!("s:tests/a.feature#{section}"),
            status,
            duration_millis: None,
        }
    }

    pub fn one(status: TestStatusFinal) -> StoredTest {
        stored(LeafKind::Scenario, "one", "1", status)
    }

    pub fn two(status: TestStatusFinal) -> StoredTest {
        stored(LeafKind::Scenario, "two", "2", status)
    }

    /// A run `run-<i>` that started at `i`, with these tests, no fingerprint and no report.
    pub fn listed(i: u32, tests: Vec<StoredTest>) -> ListedRun {
        ListedRun {
            id: format!("run-{i}"),
            report_available: false,
            record: RunRecord {
                version: 1,
                started_at: f64::from(i),
                duration_millis: 1000.0,
                env: None,
                threads: Some(1),
                fingerprint: None,
                filter: no_filter(),
                selection: None,
                failed_of: None,
                outcome: RunOutcome::Finished,
                exit_code: Some(1),
                tests,
                debug: false,
            },
        }
    }

    /// The TypeScript's `{ ...run, ...over }`: `run` with `f` applied.
    pub fn with(mut run: ListedRun, f: impl FnOnce(&mut ListedRun)) -> ListedRun {
        f(&mut run);
        run
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::events::RunOutcome;
    use crate::model::history::{LeafKind, StoredSelection, StoredTest, TestStatusFinal};
    use crate::model::node::NodeKind;
    use crate::model::tags::no_filter;
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::{Value, json};
    use std::cell::Cell;
    use tempfile::TempDir;

    /// `new Date(2026, 9, 25, 14, 32, 5, 7)`: local time as plain fields, so the tests mean the same in every zone.
    fn oct_25() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 10, 25)
            .unwrap()
            .and_hms_milli_opt(14, 32, 5, 7)
            .unwrap()
    }

    /// The tests' `record(startedAt)`.
    fn record(started_at: f64) -> RunRecord {
        RunRecord {
            debug: false,
            version: 1,
            started_at,
            duration_millis: 1000.0,
            env: None,
            threads: Some(1),
            fingerprint: None,
            filter: no_filter(),
            selection: Some(StoredSelection {
                kind: NodeKind::Folder,
                file: String::new(),
                name: String::new(),
                line: None,
                key: "d:".into(),
            }),
            failed_of: None,
            outcome: RunOutcome::Finished,
            exit_code: Some(1),
            tests: vec![
                StoredTest {
                    kind: LeafKind::Scenario,
                    file: "a.feature".into(),
                    ref_id: "[1:3]".into(),
                    name: "ok".into(),
                    line: Some(3),
                    key: "s:a#1".into(),
                    status: TestStatusFinal::Passed,
                    duration_millis: None,
                },
                StoredTest {
                    kind: LeafKind::Scenario,
                    file: "a.feature".into(),
                    ref_id: "[2:6]".into(),
                    name: "bad".into(),
                    line: Some(6),
                    key: "s:a#2".into(),
                    status: TestStatusFinal::Failed,
                    duration_millis: None,
                },
            ],
        }
    }

    /// `JSON.stringify(record)`.
    fn json_of(r: &RunRecord) -> String {
        serde_json::to_string(r).unwrap()
    }

    /// `runs/<id>` with the given `run.json` text (none: no file).
    fn run_folder(project: &Path, id: &str, json: Option<&str>) -> PathBuf {
        let dir = project.join("runs").join(id);
        fs::create_dir_all(&dir).unwrap();
        if let Some(json) = json {
            fs::write(dir.join(RECORD_FILE), json).unwrap();
        }
        dir
    }

    /// The id of the n-th second of a day, so names sort like their order.
    fn id_at(n: u32) -> String {
        format!("20261025-{n:06}-000")
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }

    /// Stands for Windows holding a run folder: a process running in it, or in a subfolder of it named to come after
    /// `run.json` (`in_subfolder`), so that a recursive delete removes `run.json` before it fails. (The TypeScript
    /// holds the folder with a real process; Linux lets the working folder of a process go, so here the delete of the
    /// held folder is the failing one, and every other folder is deleted for real.)
    struct Hold {
        dir: PathBuf,
        in_subfolder: bool,
        held: Cell<bool>,
    }

    impl Hold {
        /// Holds `dir`, which gets the subfolder `zz-held` that stays when `in_subfolder`.
        fn new(dir: PathBuf, in_subfolder: bool) -> Self {
            if in_subfolder {
                fs::create_dir_all(dir.join("zz-held")).unwrap();
            }
            Self {
                dir,
                in_subfolder,
                held: Cell::new(true),
            }
        }

        fn release(&self) {
            self.held.set(false);
        }

        /// `remove_dir` while held: deletes what it reaches in name order, up to the held subfolder, and fails, with
        /// the folder still there.
        fn remove(&self, dir: &Path, _retries: u32) -> bool {
            if !self.held.get() || dir != self.dir {
                return remove_dir(dir, 0);
            }
            let mut entries: Vec<PathBuf> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
            entries.sort();
            for entry in entries {
                if self.in_subfolder && entry.ends_with("zz-held") {
                    break;
                }
                remove_path(&entry).unwrap();
            }
            false
        }
    }

    mod run_id {
        use super::*;

        #[test]
        fn formats_local_time() {
            assert_eq!(run_id(&oct_25(), |_| false), "20261025-143205-007");
        }

        #[test]
        fn appends_a_suffix_when_taken() {
            let mut taken = vec!["20261025-143205-007".to_string()];
            assert_eq!(
                run_id(&oct_25(), |id| taken.iter().any(|t| t == id)),
                "20261025-143205-007-2"
            );
            taken.push("20261025-143205-007-2".into());
            assert_eq!(
                run_id(&oct_25(), |id| taken.iter().any(|t| t == id)),
                "20261025-143205-007-3"
            );
        }

        /// Not in the TypeScript: a leap second counts a nanosecond past 1e9, which `getMilliseconds()` never gives,
        /// and an id of four digits there would not be a run id.
        #[test]
        fn is_always_a_run_id_even_in_a_leap_second() {
            let leap = NaiveDate::from_ymd_opt(2016, 12, 31)
                .unwrap()
                .and_hms_nano_opt(23, 59, 59, 1_999_999_999)
                .unwrap();
            assert_eq!(run_id(&leap, |_| false), "20161231-235959-999");
            assert!(valid_run_id(&run_id(&leap, |_| false)));
            let early = NaiveDate::from_ymd_opt(2026, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 5)
                .unwrap();
            assert_eq!(run_id(&early, |_| false), "20260102-030405-000");
        }
    }

    mod create_run_dir {
        use super::*;

        #[test]
        fn creates_runs_id_and_a_new_one_when_the_id_is_taken() {
            let project = TempDir::new().unwrap();
            let a = create_run_dir(project.path(), &oct_25()).unwrap();
            let b = create_run_dir(project.path(), &oct_25()).unwrap();
            assert_eq!(a.id, "20261025-143205-007");
            assert_eq!(a.dir, project.path().join("runs").join(&a.id));
            assert_eq!(b.id, "20261025-143205-007-2");
            assert!(a.dir.exists() && b.dir.exists());
        }

        /// Not in the TypeScript: the folders above the run's are made, and a folder that is not there to create
        /// into is an error, not a panic.
        #[test]
        fn makes_the_folders_above_it_and_fails_where_it_cannot() {
            let project = TempDir::new().unwrap();
            let deep = project.path().join("projects").join("h1");
            assert!(create_run_dir(&deep, &oct_25()).unwrap().dir.exists());
            let file = project.path().join("a file");
            fs::write(&file, "x").unwrap();
            assert!(create_run_dir(&file, &oct_25()).is_err());
        }
    }

    mod run_json {
        use super::*;

        #[test]
        fn write_then_list_round_trips() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            let RunDir { id, dir } = create_run_dir(project, &oct_25()).unwrap();
            let r = RunRecord {
                env: Some("qa".into()),
                ..record(1792937525123.0)
            };
            write_run_record(&dir, &r).unwrap();
            assert_eq!(read_run_record(project, &id), Some(r.clone()));
            assert_eq!(
                list_history(project),
                [HistoryEntry {
                    id,
                    started_at: r.started_at,
                    duration_millis: 1000.0,
                    env: Some("qa".into()),
                    threads: Some(1),
                    filter: no_filter(),
                    selection: r.selection,
                    failed_of: None,
                    outcome: RunOutcome::Finished,
                    exit_code: Some(1),
                    passed: 1,
                    failed: 1,
                    total: 2,
                    report_available: false,
                    debug: false,
                }]
            );
        }

        #[test]
        fn read_run_record_refuses_an_id_with_dotdot() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            // projectDir/run.json is what runs/../run.json would reach
            write_run_record(project, &record(1.0)).unwrap();
            assert_eq!(read_run_record(project, ".."), None);
            assert_eq!(read_run_record(project, "20261025-143205-007/../.."), None);
            assert_eq!(read_run_record(project, ""), None);
        }

        #[test]
        fn read_run_record_reads_null_for_a_missing_folder_a_folder_with_no_file_and_a_file_over_64_mb() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            assert_eq!(read_run_record(project, &id_at(1)), None);
            run_folder(project, &id_at(2), None);
            assert_eq!(read_run_record(project, &id_at(2)), None);
            let text = json_of(&record(3.0));
            let big = run_folder(project, &id_at(3), Some(&text));
            // grown with `set_len`, as the TypeScript grows it with truncate: writing 64 MB takes seconds
            let file = fs::OpenOptions::new().write(true).open(big.join(RECORD_FILE)).unwrap();
            file.set_len(64 * 1024 * 1024 + 1).unwrap();
            assert_eq!(read_run_record(project, &id_at(3)), None);
            run_folder(project, &id_at(4), Some(&text));
            assert!(read_run_record(project, &id_at(4)).is_some());
            // a suite of 25k+ tests writes more than 5 MB: still listed
            let padded = format!("{text}{}", " ".repeat(6 * 1024 * 1024));
            run_folder(project, &id_at(5), Some(&padded));
            assert!(read_run_record(project, &id_at(5)).is_some());
            let mut ids: Vec<String> = list_history(project).into_iter().map(|e| e.id).collect();
            ids.sort();
            assert_eq!(ids, [id_at(4), id_at(5)]);
        }

        /// Not in the TypeScript: `readFileSync(file, "utf8")` replaces an invalid byte with U+FFFD and goes on, so
        /// a record with one in a test's name is still read.
        #[test]
        fn reads_a_record_with_an_invalid_byte_as_javascript_does() {
            let tmp = TempDir::new().unwrap();
            let mut text = json_of(&record(1.0)).into_bytes();
            let at = text.windows(2).position(|w| w == b"ok").unwrap();
            text[at] = 0xff;
            let dir = tmp.path().join("runs").join(id_at(1));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(RECORD_FILE), text).unwrap();
            let read = read_run_record(tmp.path(), &id_at(1)).unwrap();
            assert_eq!(read.tests[0].name, "\u{fffd}k");
        }
    }

    /// Not in the TypeScript, whose tests never look at the text: `run.json` is a file Electron Grace reads, so it is
    /// written key for key as `JSON.stringify` writes the TypeScript's object. The expected texts are what Node
    /// 22 printed for the same objects, built in the key order of `runner.ts` (`version` to `tests`, then `debug`
    /// last, only when set).
    mod run_json_text {
        use super::*;

        fn full() -> RunRecord {
            RunRecord {
                threads: Some(4),
                env: Some("qa".into()),
                fingerprint: Some("ab".repeat(32)),
                failed_of: Some(1792937525123.0),
                exit_code: Some(3_221_225_786),
                filter: crate::model::tags::FilterState {
                    include: vec!["smoke".into()],
                    exclude: vec!["slow".into()],
                    hide_non_matching: true,
                },
                outcome: RunOutcome::Failed,
                tests: vec![
                    StoredTest {
                        duration_millis: Some(412.0),
                        ..record(0.0).tests.remove(0)
                    },
                    StoredTest {
                        kind: LeafKind::Row,
                        status: TestStatusFinal::Interrupted,
                        duration_millis: Some(0.052),
                        line: None,
                        ..record(0.0).tests.remove(1)
                    },
                ],
                ..record(1792937525123.0)
            }
        }

        const FULL_TEXT: &str = concat!(
            r#"{"version":1,"startedAt":1792937525123,"durationMillis":1000,"env":"qa","threads":4,"#,
            r#""fingerprint":"abababababababababababababababababababababababababababababababab","#,
            r#""filter":{"include":["smoke"],"exclude":["slow"],"hideNonMatching":true},"#,
            r#""selection":{"kind":"folder","file":"","name":"","line":null,"key":"d:"},"#,
            r#""failedOf":1792937525123,"outcome":"failed","exitCode":3221225786,"tests":["#,
            r#"{"kind":"scenario","file":"a.feature","refId":"[1:3]","name":"ok","line":3,"key":"s:a#1","#,
            r#""status":"passed","durationMillis":412},"#,
            r#"{"kind":"row","file":"a.feature","refId":"[2:6]","name":"bad","line":null,"key":"s:a#2","#,
            r#""status":"interrupted","durationMillis":0.052}]}"#,
        );

        #[test]
        fn matches_json_stringify_of_the_runners_object() {
            assert_eq!(json_of(&full()), FULL_TEXT);
        }

        #[test]
        fn leaves_out_what_the_typescript_leaves_out_and_keeps_its_nulls() {
            let text = json_of(&record(1.0));
            assert_eq!(
                text,
                concat!(
                    r#"{"version":1,"startedAt":1,"durationMillis":1000,"env":null,"threads":1,"fingerprint":null,"#,
                    r#""filter":{"include":[],"exclude":[],"hideNonMatching":false},"#,
                    r#""selection":{"kind":"folder","file":"","name":"","line":null,"key":"d:"},"#,
                    r#""failedOf":null,"outcome":"finished","exitCode":1,"tests":["#,
                    r#"{"kind":"scenario","file":"a.feature","refId":"[1:3]","name":"ok","line":3,"key":"s:a#1","status":"passed"},"#,
                    r#"{"kind":"scenario","file":"a.feature","refId":"[2:6]","name":"bad","line":6,"key":"s:a#2","status":"failed"}]}"#,
                )
            );
            let none = RunRecord {
                selection: None,
                exit_code: None,
                threads: None,
                ..record(1.0)
            };
            let text = json_of(&none);
            assert!(text.contains(r#""threads":null"#) && text.contains(r#""selection":null"#));
            assert!(text.contains(r#""exitCode":null"#));
        }

        /// A test's duration is a fraction of a millisecond, which `JSON.stringify` prints as these texts (from Node).
        /// Not covered: `serde_json` prints a number from 1e-6 up to 1e-5 as `1e-6` where JavaScript prints
        /// `0.000001`, and one from 1e16 as `1e16` where JavaScript prints all its digits (and `1e+21` from 1e21).
        /// A duration is never so small (1 ns is 1e-6 ms; a test takes longer) or so large (1e16 ms is 300,000
        /// years), and both texts read back as the same number.
        #[test]
        fn writes_a_duration_as_javascript_prints_it() {
            let text_of = |ms: f64| {
                let t = StoredTest {
                    duration_millis: Some(ms),
                    ..record(0.0).tests.remove(0)
                };
                let json = serde_json::to_string(&t).unwrap();
                json.rsplit_once("\"durationMillis\":")
                    .unwrap()
                    .1
                    .trim_end_matches('}')
                    .to_string()
            };
            for (ms, js) in [
                (0.052, "0.052"),
                (0.1 + 0.2, "0.30000000000000004"),
                (4.35, "4.35"),
                (1234567.891, "1234567.891"),
                (123456789.123, "123456789.123"),
                (0.00001, "0.00001"),
                (1e-7, "1e-7"),
                (1.23e-18, "1.23e-18"),
            ] {
                assert_eq!(text_of(ms), js, "{ms}");
            }
        }

        #[test]
        fn writes_debug_after_tests() {
            let text = json_of(&RunRecord {
                debug: true,
                ..record(1.0)
            });
            assert!(text.ends_with(r#"],"debug":true}"#), "{text}");
            assert!(text.starts_with(r#"{"version":1,"#));
        }

        #[test]
        fn is_what_write_run_record_puts_in_the_file() {
            let tmp = TempDir::new().unwrap();
            write_run_record(tmp.path(), &full()).unwrap();
            assert_eq!(fs::read_to_string(tmp.path().join(RECORD_FILE)).unwrap(), FULL_TEXT);
        }

        #[test]
        fn is_an_error_not_a_panic_where_the_folder_is_not_there() {
            let tmp = TempDir::new().unwrap();
            assert!(write_run_record(&tmp.path().join("missing"), &record(1.0)).is_err());
        }
    }

    mod list_history {
        use super::*;

        #[test]
        fn sorts_by_started_at_not_by_folder_name() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            run_folder(project, &id_at(1), Some(&json_of(&record(300.0))));
            run_folder(project, &id_at(2), Some(&json_of(&record(200.0))));
            run_folder(project, &id_at(3), Some(&json_of(&record(100.0))));
            let started: Vec<f64> = list_history(project).iter().map(|e| e.started_at).collect();
            assert_eq!(started, [300.0, 200.0, 100.0]);
        }

        #[test]
        fn skips_invalid_and_unknown_version_files() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            run_folder(project, &id_at(1), Some(&json_of(&record(100.0))));
            run_folder(project, &id_at(2), Some("{not json"));
            let with = |key: &str, value: Value| {
                let mut x = serde_json::to_value(record(300.0)).unwrap();
                x[key] = value;
                x.to_string()
            };
            run_folder(project, &id_at(3), Some(&with("version", json!(2))));
            run_folder(project, &id_at(4), Some(&with("outcome", json!("exploded"))));
            run_folder(project, &id_at(5), None); // no run.json
            run_folder(project, "notes", None); // not a run id
            fs::write(project.join("runs").join(id_at(6)), "a file named like a run").unwrap();
            let ids: Vec<String> = list_history(project).into_iter().map(|e| e.id).collect();
            assert_eq!(ids, [id_at(1)]);
        }

        #[test]
        fn is_empty_and_does_not_throw_when_there_is_no_runs_folder() {
            let tmp = TempDir::new().unwrap();
            assert!(list_history(&tmp.path().join("missing")).is_empty());
        }

        #[test]
        fn caps_at_20() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            for i in 1..=25 {
                run_folder(project, &id_at(i), Some(&json_of(&record(f64::from(i)))));
            }
            let list = list_history(project);
            assert_eq!(list.len(), HISTORY_LIMIT);
            assert_eq!(list[0].started_at, 25.0);
            assert_eq!(list.last().unwrap().started_at, 6.0);
        }

        #[test]
        fn report_available_follows_karate_summary_html() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            let with_report = run_folder(project, &id_at(1), Some(&json_of(&record(100.0))));
            run_folder(project, &id_at(2), Some(&json_of(&record(200.0))));
            fs::write(with_report.join(REPORT_FILE), "<html></html>").unwrap();
            let listed: Vec<(String, bool)> = list_history(project)
                .into_iter()
                .map(|e| (e.id, e.report_available))
                .collect();
            assert_eq!(listed, [(id_at(2), false), (id_at(1), true)]);
        }
    }

    mod list_runs {
        use super::*;

        #[test]
        fn returns_what_list_history_lists() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            let records = [
                RunRecord {
                    env: Some("qa".into()),
                    ..record(200.0)
                },
                record(300.0),
                record(100.0),
            ];
            for (i, r) in records.iter().enumerate() {
                run_folder(project, &id_at(i as u32 + 1), Some(&json_of(r)));
            }
            run_folder(project, &id_at(4), Some("{not json"));
            fs::write(project.join("runs").join(id_at(2)).join(REPORT_FILE), "<html></html>").unwrap();
            let runs = list_runs(project);
            let ids: Vec<&str> = runs.iter().map(|r| r.id.as_str()).collect();
            let listed: Vec<String> = list_history(project).into_iter().map(|e| e.id).collect();
            assert_eq!(ids, listed);
            assert_eq!(ids, [id_at(2), id_at(1), id_at(3)]);
            let kept: Vec<&RunRecord> = runs.iter().map(|r| &r.record).collect();
            assert_eq!(kept, [&records[1], &records[0], &records[2]]);
            let reports: Vec<bool> = runs.iter().map(|r| r.report_available).collect();
            assert_eq!(reports, [true, false, false]);
        }

        /// Not in the TypeScript, which relies on `sort` being stable: runs of one time stay in name order.
        #[test]
        fn keeps_name_order_between_runs_of_one_time() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            for i in 1..=3 {
                run_folder(project, &id_at(i), Some(&json_of(&record(5.0))));
            }
            let ids: Vec<String> = list_runs(project).into_iter().map(|r| r.id).collect();
            assert_eq!(ids, [id_at(1), id_at(2), id_at(3)]);
        }
    }

    mod prune_history {
        use super::*;

        #[test]
        fn keeps_the_20_newest_names_invalid_ones_included() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            // 3 is damaged and old; 12 and 24 are damaged and among the newest 20: names decide, not content
            for i in 1..=25 {
                let text = if [3, 12, 24].contains(&i) {
                    "{".to_string()
                } else {
                    json_of(&record(f64::from(i)))
                };
                run_folder(project, &id_at(i), Some(&text));
            }
            prune_history(project);
            let expected: Vec<String> = (6..=25).map(id_at).collect();
            assert_eq!(names(&project.join("runs")), expected);
        }

        #[test]
        fn keeps_what_it_should_when_there_are_fewer_than_20_and_never_throws() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            prune_history(&project.join("missing"));
            for i in 1..=3 {
                run_folder(project, &id_at(i), Some(&json_of(&record(f64::from(i)))));
            }
            prune_history(project);
            assert_eq!(names(&project.join("runs")).len(), 3);
        }

        #[test]
        fn skips_a_held_folder_and_deletes_the_others_then_tries_it_again() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            for i in 1..=23 {
                run_folder(project, &id_at(i), Some(&json_of(&record(f64::from(i)))));
            }
            let hold = Hold::new(project.join("runs").join(id_at(1)), false);
            prune_history_with(project, |dir, retries| hold.remove(dir, retries));
            // the 20 newest and the held one
            assert_eq!(names(&project.join("runs")).len(), 21);
            hold.release();
            prune_history(project);
            assert_eq!(names(&project.join("runs")).len(), 20);
        }

        /// Not in the TypeScript: the names of runs are not all folders, and a file that is named like one goes too,
        /// as `rmSync` removes a file.
        #[test]
        fn deletes_a_file_named_like_an_old_run() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            for i in 2..=21 {
                run_folder(project, &id_at(i), Some(&json_of(&record(f64::from(i)))));
            }
            fs::write(project.join("runs").join(id_at(1)), "a file named like a run").unwrap();
            prune_history(project);
            assert_eq!(names(&project.join("runs")).len(), 20);
            assert!(!project.join("runs").join(id_at(1)).exists());
        }
    }

    mod clear_history {
        use super::*;

        #[test]
        fn removes_all() {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            for i in 1..=3 {
                run_folder(project, &id_at(i), Some(&json_of(&record(f64::from(i)))));
            }
            assert!(clear_history(project));
            assert_eq!(names(&project.join("runs")), Vec::<String>::new());
            assert!(list_history(project).is_empty());
        }

        /// The TypeScript's `it.each` over a process "in the folder" and "in a subfolder".
        fn reports_a_held_folder_and_keeps_that_run_listed(in_subfolder: bool) {
            let tmp = TempDir::new().unwrap();
            let project = tmp.path();
            for i in 1..=3 {
                run_folder(project, &id_at(i), Some(&json_of(&record(f64::from(i)))));
            }
            let held = project.join("runs").join(id_at(2));
            fs::write(held.join(REPORT_FILE), "<html></html>").unwrap();
            fs::create_dir(held.join("karate-json")).unwrap();
            fs::write(held.join("karate-json").join("karate-events.jsonl"), "{}").unwrap();
            let hold = Hold::new(held, in_subfolder);
            assert!(!clear_history_with(project, |dir, retries| hold.remove(dir, retries)));
            assert_eq!(names(&project.join("runs")), [id_at(2)]);
            // its report may be gone, its record is what keeps it in the list
            assert_eq!(read_run_record(project, &id_at(2)), Some(record(2.0)));
            let ids: Vec<String> = list_history(project).into_iter().map(|e| e.id).collect();
            assert_eq!(ids, [id_at(2)]);
            hold.release();
            assert!(clear_history(project));
            assert_eq!(names(&project.join("runs")), Vec::<String>::new());
            assert!(list_history(project).is_empty());
        }

        #[test]
        fn reports_a_folder_held_by_a_process_in_the_folder_and_keeps_that_run_listed() {
            reports_a_held_folder_and_keeps_that_run_listed(false);
        }

        #[test]
        fn reports_a_folder_held_by_a_process_in_a_subfolder_and_keeps_that_run_listed() {
            reports_a_held_folder_and_keeps_that_run_listed(true);
        }

        #[test]
        fn is_true_when_there_is_nothing_to_delete() {
            let tmp = TempDir::new().unwrap();
            assert!(clear_history(&tmp.path().join("missing")));
        }
    }

    mod remove_leftover_runs {
        use super::*;

        #[test]
        fn deletes_folders_without_run_json_and_1_0_run_folders_keeps_valid_runs() {
            let tmp = TempDir::new().unwrap();
            let data = tmp.path();
            let p1 = data.join("projects").join("h1");
            let p2 = data.join("projects").join("h2");
            run_folder(&p1, &id_at(1), Some(&json_of(&record(1.0))));
            run_folder(&p1, &id_at(2), None); // a crash left it
            run_folder(&p2, &id_at(3), Some("{")); // damaged, but run.json is there: retention's business
            run_folder(&p2, &id_at(4), None);
            for p in [&p1, &p2] {
                fs::create_dir_all(p.join("run").join("karate-json")).unwrap(); // version 1.0
                fs::create_dir_all(p.join("dryrun")).unwrap();
            }
            fs::write(data.join("settings.json"), "{}").unwrap();
            fs::write(data.join("projects").join("stray-file"), "x").unwrap();

            remove_leftover_runs(data);

            assert_eq!(names(&p1.join("runs")), [id_at(1)]);
            assert_eq!(names(&p2.join("runs")), [id_at(3)]);
            for p in [&p1, &p2] {
                assert!(!p.join("run").exists());
                assert!(p.join("dryrun").exists());
            }
            assert!(data.join("settings.json").exists());
        }

        #[test]
        fn never_throws_whatever_is_there() {
            let tmp = TempDir::new().unwrap();
            let data = tmp.path();
            remove_leftover_runs(&data.join("missing"));
            fs::create_dir_all(data.join("projects").join("h1")).unwrap();
            fs::write(
                data.join("projects").join("h1").join("runs"),
                "a file where a folder is expected",
            )
            .unwrap();
            fs::write(data.join("projects").join("file"), "x").unwrap();
            remove_leftover_runs(data);
        }
    }

    /// Not in the TypeScript, where Node does the retrying inside `rmSync`: the retry loop and its platform seam.
    mod deleting {
        use super::*;

        fn error(kind: ErrorKind) -> io::Error {
            io::Error::from(kind)
        }

        #[test]
        fn retries_with_a_growing_pause_and_then_gives_up() {
            let (mut tries, mut pauses) = (0, Vec::new());
            let result = with_retries(
                5,
                || {
                    tries += 1;
                    Err(error(ErrorKind::ResourceBusy))
                },
                |d| pauses.push(d.as_millis()),
            );
            assert!(result.is_err());
            assert_eq!(tries, 6);
            assert_eq!(pauses, [100, 200, 300, 400, 500]);
        }

        #[test]
        fn stops_at_the_first_success_and_retries_nothing_at_zero() {
            let (mut tries, mut pauses) = (0, 0);
            let result = with_retries(
                5,
                || {
                    tries += 1;
                    if tries < 3 {
                        Err(error(ErrorKind::DirectoryNotEmpty))
                    } else {
                        Ok(())
                    }
                },
                |_| pauses += 1,
            );
            assert!(result.is_ok());
            assert_eq!((tries, pauses), (3, 2));
            let mut tries = 0;
            let result = with_retries(
                0,
                || {
                    tries += 1;
                    Err(error(ErrorKind::ResourceBusy))
                },
                |_| panic!("no retry"),
            );
            assert!(result.is_err());
            assert_eq!(tries, 1);
        }

        #[test]
        fn platform_decides_what_is_worth_retrying() {
            assert!(worth_retrying(&error(ErrorKind::ResourceBusy)));
            assert!(worth_retrying(&error(ErrorKind::DirectoryNotEmpty)));
            // Windows retries any failure, as Node retries `EPERM` there; Linux only a busy file or a full folder
            assert_eq!(worth_retrying(&error(ErrorKind::PermissionDenied)), cfg!(windows));
            assert_eq!(worth_retrying(&error(ErrorKind::InvalidInput)), cfg!(windows));
        }

        #[test]
        fn a_path_that_is_not_there_is_deleted_already() {
            let tmp = TempDir::new().unwrap();
            assert!(remove_dir(&tmp.path().join("missing"), 0));
            let file = tmp.path().join("file");
            fs::write(&file, "x").unwrap();
            assert!(remove_dir(&file, 0) && !file.exists());
            let tree = tmp.path().join("tree");
            fs::create_dir_all(tree.join("a").join("b")).unwrap();
            fs::write(tree.join("a").join("b").join("c"), "x").unwrap();
            assert!(remove_dir(&tree, 0) && !tree.exists());
        }

        #[cfg(unix)]
        #[test]
        fn removes_a_link_and_not_what_it_points_to() {
            let tmp = TempDir::new().unwrap();
            let target = tmp.path().join("target");
            fs::create_dir(&target).unwrap();
            fs::write(target.join("keep"), "x").unwrap();
            let link = tmp.path().join("link");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert!(remove_dir(&link, 0));
            assert!(!link.exists() && target.join("keep").exists());
        }
    }
}
