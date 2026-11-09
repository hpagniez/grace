//! `src/main/recent.ts`: tells which of the recent project folders still exist, for the home screen's list.
//!
//! A folder on a disconnected network share can take 20 s or more to answer a `stat`, so each folder is checked on a
//! thread of its own and the caller waits for all of them at most `TIMEOUT` altogether: an unknown folder counts as
//! existing, and must not be shown as gone. The TypeScript's timer (`within`) and its `Promise.all` are a condition
//! variable's timed wait here: nothing polls, and a thread lives only as long as its `stat` (ADR 0030).
//!
//! A hung `stat` holds its thread until the operating system gives up, so a later call joins the check that has not
//! settled instead of starting another (as the TypeScript does with its table of pending checks): hung threads never
//! pile up however often the list is shown.
//!
//! The calls block for `TIMEOUT` at most. Call them off the UI thread.
use crate::model::events::{AppEvent, RecentEntry};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// How long the answer for a folder is waited for.
pub const TIMEOUT: Duration = Duration::from_millis(1000);

/// `stat` as a seam, so a test makes it fail or never return: whether the path is a directory.
pub type Stat = dyn Fn(&str) -> io::Result<bool> + Send + Sync;

/// A directory, after following links, as Node's `stat(path).isDirectory()` says.
fn fs_stat(path: &str) -> io::Result<bool> {
    fs::metadata(path).map(|m| m.is_dir())
}

/// A lock whose holder panicked still holds consistent data here: take it anyway.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One folder's check, shared by every call that asks while it runs.
#[derive(Default)]
struct Check {
    /// `None` until the check settled; then whether the folder exists.
    answer: Mutex<Option<bool>>,
    settled: Condvar,
}

impl Check {
    fn settle(&self, exists: bool) {
        *lock(&self.answer) = Some(exists);
        self.settled.notify_all();
    }

    /// The answer, waiting for it until `deadline`; `None` if it has not come.
    fn wait(&self, deadline: Instant) -> Option<bool> {
        let timeout = deadline.saturating_duration_since(Instant::now());
        let (answer, _) = self
            .settled
            .wait_timeout_while(lock(&self.answer), timeout, |a| a.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        *answer
    }
}

type Pending = Arc<Mutex<HashMap<String, Arc<Check>>>>;

/// Ends a check, on a normal end of its thread and on a panic of the `stat`: the folder counts as not found, the
/// check is forgotten, and whoever waits is let go. It forgets before it settles, so a caller that has the answer
/// finds no stale check.
struct Settler {
    check: Arc<Check>,
    path: String,
    pending: Pending,
    exists: bool,
}

impl Drop for Settler {
    fn drop(&mut self) {
        lock(&self.pending).remove(&self.path);
        self.check.settle(self.exists);
    }
}

/// Checks whether folders exist, at most `timeout` for all of them together.
///
/// The TypeScript's pending checks are kept by `stat` function in a `WeakMap`; here each checker owns its own.
#[derive(Clone)]
pub struct RecentChecker {
    stat: Arc<Stat>,
    timeout: Duration,
    pending: Pending,
}

impl Default for RecentChecker {
    fn default() -> Self {
        Self::with(Arc::new(fs_stat), TIMEOUT)
    }
}

impl RecentChecker {
    pub fn new() -> Self {
        Self::default()
    }

    /// A checker with its own `stat` and time bound.
    pub fn with(stat: Arc<Stat>, timeout: Duration) -> Self {
        Self {
            stat,
            timeout,
            pending: Arc::default(),
        }
    }

    /// The check of `path` that is running, or a new one on a thread of its own. `None` if no thread could start: the
    /// folder is then unknown.
    fn start(&self, path: &str) -> Option<Arc<Check>> {
        let mut pending = lock(&self.pending);
        if let Some(check) = pending.get(path) {
            return Some(Arc::clone(check));
        }
        let check = Arc::new(Check::default());
        // Entered before the thread starts, which cannot forget it before this lock is let go.
        pending.insert(path.to_string(), Arc::clone(&check));
        let spawned = thread::Builder::new().name("recent-stat".to_string()).spawn({
            let (stat, check, path, pending) = (
                Arc::clone(&self.stat),
                Arc::clone(&check),
                path.to_string(),
                Arc::clone(&self.pending),
            );
            move || {
                // made here, not before, so a thread that never starts does not drop it under the caller's lock
                let mut end = Settler {
                    check,
                    path,
                    pending,
                    exists: false,
                };
                // a failing `stat` is a folder that is not there
                end.exists = stat(&end.path).unwrap_or(false);
            }
        });
        if spawned.is_err() {
            pending.remove(path);
            return None;
        }
        Some(check)
    }

    /// Whether each folder still exists, all checked in parallel, order kept. A file or a failing `stat` is not found.
    /// A check that has not settled within the time bound counts as existing.
    pub fn check(&self, paths: &[String]) -> Vec<RecentEntry> {
        let now = Instant::now();
        let deadline = now
            .checked_add(self.timeout)
            .unwrap_or(now + Duration::from_secs(365 * 24 * 60 * 60));
        // every check starts before any is waited for, so they run side by side
        let checks: Vec<Option<Arc<Check>>> = paths.iter().map(|p| self.start(p)).collect();
        paths
            .iter()
            .zip(checks)
            .map(|(path, check)| RecentEntry {
                path: path.clone(),
                exists: check.and_then(|c| c.wait(deadline)).unwrap_or(true),
            })
            .collect()
    }
}

/// Loads the recent list, checks it and emits `recent`. A call emits nothing if a later call started meanwhile, so a
/// slow old check never overwrites a newer list.
///
/// The TypeScript returns a function; this is a value with `send`, which blocks like `RecentChecker::check`.
pub struct RecentSender<L, E> {
    checker: RecentChecker,
    load: L,
    emit: E,
    /// The number of the latest call. Held while a list is emitted, so a list is never emitted after a newer one.
    latest: Mutex<u64>,
}

impl<L, E> RecentSender<L, E>
where
    L: Fn() -> Vec<String>,
    E: Fn(AppEvent),
{
    pub fn new(load: L, emit: E) -> Self {
        Self::with_checker(load, emit, RecentChecker::new())
    }

    pub fn with_checker(load: L, emit: E, checker: RecentChecker) -> Self {
        Self {
            checker,
            load,
            emit,
            latest: Mutex::new(0),
        }
    }

    /// Loads, checks and emits. `emit` runs under a lock: it must be quick and must not call `send`.
    pub fn send(&self) {
        let mine = {
            let mut latest = lock(&self.latest);
            *latest += 1;
            *latest
        };
        let entries = self.checker.check(&(self.load)());
        let latest = lock(&self.latest);
        if mine == *latest {
            (self.emit)(AppEvent::Recent { entries });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::Builder::new().prefix("kk-recent-").tempdir().unwrap()
    }

    fn entry(path: &str, exists: bool) -> RecentEntry {
        RecentEntry {
            path: path.to_string(),
            exists,
        }
    }

    fn strings(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    /// A stat that waits at a gate: until it is opened, it does not answer.
    #[derive(Clone, Default)]
    struct Gate(Arc<(Mutex<bool>, Condvar)>);

    impl Gate {
        fn wait(&self) {
            let (open, opened) = &*self.0;
            drop(opened.wait_while(lock(open), |open| !*open).unwrap());
        }

        fn open(&self) {
            let (open, opened) = &*self.0;
            *lock(open) = true;
            opened.notify_all();
        }
    }

    /// The stat of a folder that does not answer within the time bound, until the test opens `gate`.
    fn never(gate: &Gate) -> Arc<Stat> {
        let gate = gate.clone();
        Arc::new(move |_| {
            gate.wait();
            Ok(true)
        })
    }

    const SHORT: Duration = Duration::from_millis(50);
    const LONG: Duration = Duration::from_secs(10);

    mod check_recent {
        use super::*;

        #[test]
        fn checks_each_folder() {
            let dir = tmp();
            let missing = dir.path().join("missing").to_string_lossy().into_owned();
            let here = dir.path().to_string_lossy().into_owned();
            assert_eq!(
                RecentChecker::new().check(&[missing.clone(), here.clone()]),
                [entry(&missing, false), entry(&here, true)]
            );
        }

        #[test]
        fn a_file_or_a_failing_stat_counts_as_not_found() {
            let dir = tmp();
            let file = dir.path().join("a.txt");
            fs::write(&file, "x").unwrap();
            let file = file.to_string_lossy().into_owned();
            assert_eq!(
                RecentChecker::new().check(std::slice::from_ref(&file)),
                [entry(&file, false)]
            );

            let denied: Arc<Stat> = Arc::new(|_| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
            assert_eq!(
                RecentChecker::with(denied, LONG).check(&strings(&["D:\\a"])),
                [entry("D:\\a", false)]
            );
        }

        #[test]
        fn a_folder_that_does_not_answer_within_the_time_bound_counts_as_existing() {
            let gate = Gate::default();
            let checker = RecentChecker::with(never(&gate), SHORT);
            let started = Instant::now();
            let result = checker.check(&strings(&["\\\\gone\\a", "\\\\gone\\b"]));
            // together, not one after the other
            assert!(started.elapsed() < SHORT * 2 + Duration::from_millis(500));
            assert_eq!(result, [entry("\\\\gone\\a", true), entry("\\\\gone\\b", true)]);
            gate.open();
        }

        #[test]
        fn shares_a_check_that_is_still_pending_and_starts_a_new_one_once_it_settled() {
            let calls = Arc::new(Mutex::new(Vec::<String>::new()));
            let first = Gate::default();
            let a_calls = Arc::new(AtomicUsize::new(0));
            let (entered, entered_rx) = mpsc::channel();
            let stat: Arc<Stat> = {
                let (calls, first) = (Arc::clone(&calls), first.clone());
                Arc::new(move |path| {
                    lock(&calls).push(path.to_string());
                    // the first check of `D:\a` hangs; every other answers at once
                    if path == "D:\\a" && a_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                        entered.send(()).unwrap();
                        first.wait();
                    }
                    Ok(true)
                })
            };
            let checker = RecentChecker::with(stat, SHORT);
            let seen = || lock(&calls).clone();

            assert_eq!(checker.check(&strings(&["D:\\a"])), [entry("D:\\a", true)]);
            entered_rx.recv_timeout(LONG).unwrap();
            assert_eq!(
                checker.check(&strings(&["D:\\a", "D:\\b"])),
                [entry("D:\\a", true), entry("D:\\b", true)]
            );
            assert_eq!(seen(), ["D:\\a", "D:\\b"]);

            // let the hung check end, and wait for it to be forgotten
            let held = Arc::clone(lock(&checker.pending).get("D:\\a").unwrap());
            first.open();
            assert_eq!(held.wait(Instant::now() + LONG), Some(true));
            assert!(lock(&checker.pending).is_empty());
            checker.check(&strings(&["D:\\a"]));
            assert_eq!(seen(), ["D:\\a", "D:\\b", "D:\\a"]);
        }

        // Not in the TypeScript: a `stat` that panics is a folder that is not there, and is forgotten.
        #[test]
        fn a_stat_that_panics_counts_as_not_found() {
            let stat: Arc<Stat> = Arc::new(|_| panic!("stat failed"));
            let checker = RecentChecker::with(stat, LONG);
            assert_eq!(checker.check(&strings(&["D:\\a"])), [entry("D:\\a", false)]);
            assert!(lock(&checker.pending).is_empty());
        }
    }

    mod recent_sender {
        use super::*;

        #[test]
        fn emits_the_checked_list() {
            let dir = tmp();
            let here = dir.path().to_string_lossy().into_owned();
            let events = Mutex::new(Vec::new());
            RecentSender::new(|| vec![here.clone()], |e| lock(&events).push(e)).send();
            assert_eq!(
                *lock(&events),
                [AppEvent::Recent {
                    entries: vec![entry(&here, true)]
                }]
            );
        }

        #[test]
        fn drops_a_list_whose_checks_end_after_a_newer_calls() {
            let lists = Mutex::new(VecDeque::from([strings(&["D:\\a"]), vec![]]));
            let gate = Gate::default();
            let (entered, entered_rx) = mpsc::channel();
            let stat: Arc<Stat> = {
                let gate = gate.clone();
                Arc::new(move |_| {
                    entered.send(()).unwrap();
                    gate.wait();
                    Ok(true)
                })
            };
            let events = Mutex::new(Vec::new());
            let sender = RecentSender::with_checker(
                || lock(&lists).pop_front().unwrap_or_default(),
                |e| lock(&events).push(e),
                RecentChecker::with(stat, LONG),
            );
            thread::scope(|scope| {
                let first = scope.spawn(|| sender.send());
                // the first call has its number and waits at the gate
                entered_rx.recv_timeout(LONG).unwrap();
                sender.send();
                gate.open();
                first.join().unwrap();
            });
            assert_eq!(*lock(&events), [AppEvent::Recent { entries: vec![] }]);
        }
    }
}
