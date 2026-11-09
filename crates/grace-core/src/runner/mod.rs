//! `src/main/runner.ts`: the `Runner`, which owns the open project, its test tree, its tag filter, its thread count,
//! its environment and run properties, and the one Karate process (dry run or run) at a time. It reports what happens
//! as `AppEvent`s to the `emit` it is given; the `Grace` entity of phase P2 holds it and turns them into state.
//!
//! # Threads
//!
//! A `Runner` is a cheap handle (`Clone`, `Send`, `Sync`). Its methods block: `open`, `refresh`, `set_env`, `run`,
//! `rerun` and `rerun_failed` for as long as their dry run and run take, on the thread that calls them, which is never
//! GPUI's main thread (P2 calls them from a background task or a thread of its own). While one of them works, the
//! runner is busy: any other of them, and every change that a run would see, is refused with "A run is already in
//! progress", as in Electron Grace; `cancel`, `replay`, `current` and the other reads answer at once from any thread.
//! Nothing of the runner runs or wakes while it is idle (ADR 0030): a run's thread reads its events file every 100 ms
//! only while Java runs.
//!
//! `emit` is called on the thread that does the work (a run's events on the run's caller, the recent list on a short
//! thread of its own), never under one of the runner's locks: it may call `cancel`, as a test of Electron Grace's
//! does. It should only queue the event (`batch::batched`) and return.
//!
//! # Where this differs from a line-for-line port
//!
//! - The TypeScript's single event loop made each step between two `await`s atomic. Here the state is behind one lock,
//!   and a change refused while busy is checked and made under it; a cancel cannot fall between the check for a cancel
//!   and the start of Java, which take the process lock together (`spawn`).
//! - Busy is a `Phase` that carries the folder being opened: "opening while idle" cannot be written.
//! - The run's thread waits for Java's lines on a channel and reads the events file at each 100 ms deadline in
//!   between, in place of a readline callback and a `setInterval`: one thread emits a run's events, in order.
//! - Errors are `io::Result`s where the TypeScript threw; P2 shows them as notices, as `main.ts`'s `handle` did.
//! - The window's untyped requests (`setThreads(n: unknown)`) keep their checks where the type allows them.
//! - Phase P4's parts are seams or left out: the libraries, the test code notice, pom.xml's environment and the
//!   sub-folder offer are `ProjectSetup`'s methods (`NoSetup` until then); watch mode, the step debugger, Build and the
//!   editor's files are left out, each place they act marked `P4`. The events stay those of a project that has none of
//!   them: `watch` (as saved), `breakpoints` (empty), no `listing`.
mod run;
mod setup;
#[cfg(test)]
mod tests;

pub use setup::{JavaCode, NoSetup, PomStart, ProjectSetup, pom_start};

use crate::flaky::find_flaky;
use crate::history::{self, ListedRun, clear_history, list_runs, read_run_record};
#[cfg(test)]
use crate::karate::command::Env;
use crate::karate::command::{
    BuildArgs, OUT_OF_MEMORY, ProjectLayout, absolute, build_args, child_env, detect_layout, extension_jars, locate,
    out_of_memory, tag_args,
};
use crate::karate::environments::{feature_tag_envs, overlay_envs};
use crate::karate::events::read_all_events;
use crate::karate::process::{KarateProcess, SpawnOptions, spawn_karate};
use crate::karate::tree::{build_tree, leaf_keys, parse_ref_id, row_lines};
use crate::model::env::{
    EnvView, PlainRow, PropertiesEdit, PropertyListsView, RowEdit, RowView, SecretRowView, SecretState, edit_errors,
    property_value_error, sorted_unique, valid_env_name,
};
use crate::model::events::{AppEvent, Breakpoint, Project};
use crate::model::history::{HistoryEntry, to_entry};
use crate::model::js;
use crate::model::tags::{FilterState, no_filter, project_tags, prune_filter};
use crate::model::threads::{THREADS_ERROR, parse_threads};
use crate::model::trends::TrendPoint;
use crate::model::{NodeKind, TestNode};
use crate::recent::RecentSender;
use crate::secrets::SecretBox;
use crate::settings::{
    EnvSettings, SavedControls, StoredLists, StoredRow, StoredValue, forget_last_project, load_project_env,
    load_project_filter, load_project_threads, load_project_watch, load_recent_projects, remove_recent_project,
    save_opened_project, save_project_env, save_project_filter, save_project_properties, save_project_threads,
    saved_controls, skipped_properties,
};
use crate::trends::find_trend;
use crate::vendor::VendorPaths;
use indexmap::IndexMap;
use run::{OldNode, RunTarget};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const BUSY: &str = "A run is already in progress";
const INVALID: &str = "Invalid request";

/// What the runner reports to: the `Grace` entity in the app, a list in the tests.
pub type Emit = Arc<dyn Fn(AppEvent) + Send + Sync>;
/// The project fingerprint's walk: `fingerprint::project_fingerprint`, or a test's.
pub type Walk = Arc<dyn Fn(&Path) -> Option<String> + Send + Sync>;

/// `projectDirOf`: `<data_dir>/projects/<hash>`, the run folders, the dry run folder and the argument files of a
/// project; `<hash>` is the first 12 hex digits of the SHA-1 of its root, as Electron Grace names it.
pub fn project_dir(data_dir: &Path, root: &str) -> PathBuf {
    let hash = sha1_smol::Sha1::from(root).digest().to_string();
    data_dir.join("projects").join(&hash[..12])
}

/// Karate's own words for a failed dry run: lines naming an exception or a feature, else the tail.
pub fn dry_run_error<S: AsRef<str>>(lines: &[S]) -> String {
    if out_of_memory(lines) {
        return OUT_OF_MEMORY.to_string();
    }
    let hits: Vec<&str> = lines
        .iter()
        .map(AsRef::as_ref)
        .filter(|l| l.contains("Exception") || l.contains("feature"))
        .collect();
    if hits.is_empty() {
        let tail = lines.len().saturating_sub(20);
        lines[tail..].iter().map(AsRef::as_ref).collect::<Vec<_>>().join("\n")
    } else {
        hits.join("\n")
    }
}

/// One notice of several sentences: the window shows only the last notice.
fn join_notices<S: AsRef<str>>(said: &[S]) -> String {
    said.iter()
        .map(|m| {
            let m = m.as_ref();
            if said.len() > 1 && !m.ends_with('.') {
                format!("{m}.")
            } else {
                m.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The text of a path, as Grace keeps project folders: a JSON string.
fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The error with the path it is about, as Node's messages name it.
fn at(path: &Path) -> impl Fn(io::Error) -> io::Error + '_ {
    move |e| io::Error::new(e.kind(), format!("{}: {e}", path.display()))
}

/// `Date.now()`.
fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

/// The folder's name, as `path.basename` gives it.
fn basename(root: &str) -> String {
    Path::new(root)
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

/// Whether the runner is idle, or busy discovering or running; while it opens a folder, that folder.
// Rust-native: one enum for Electron Grace's `busy` and `opening`, which are set and cleared together. Parity: every
// `replay` test (`replay_restores_a_window_that_loads_during_discovery`, `replay_during_an_open_carries_...`).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Busy { opening: Option<String> },
}

/// The last `project` event but its tree, which is always the runner's `tree`: the tree is held once.
#[derive(Clone, Debug)]
struct LastProject {
    project: Option<Project>,
    error: Option<String>,
    subfolders: Option<Vec<String>>,
}

struct State {
    project: Option<Project>,
    tree: Option<Arc<TestNode>>,
    last: Option<LastProject>,
    filter: FilterState,
    threads: u32,
    envs: EnvSettings,
    /// Environments of overlay files and env tags, scanned at each discovery of open, refresh and set_env.
    overlays: Vec<String>,
    tag_envs: Vec<String>,
    phase: Phase,
    report: Option<PathBuf>,
    /// The open project's breakpoints, in memory only (step debugger R3.1); always empty until P4 ports the debugger.
    breakpoints: Vec<Breakpoint>,
    /// Watch mode as saved for the open project (watch mode R1); the watcher itself ports in P4.
    watching: bool,
}

impl State {
    fn new() -> Self {
        Self {
            project: None,
            tree: None,
            last: None,
            filter: no_filter(),
            threads: 1,
            envs: EnvSettings::default(),
            overlays: Vec::new(),
            tag_envs: Vec::new(),
            phase: Phase::Idle,
            report: None,
            breakpoints: Vec::new(),
            watching: false,
        }
    }

    fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }

    fn opening(&self) -> Option<String> {
        match &self.phase {
            Phase::Busy { opening } => opening.clone(),
            Phase::Idle => None,
        }
    }

    fn last_event(&self) -> Option<AppEvent> {
        let last = self.last.as_ref()?;
        Some(AppEvent::Project {
            project: last.project.clone(),
            tree: self.tree.as_deref().cloned(),
            error: last.error.clone(),
            subfolders: last.subfolders.clone(),
        })
    }
}

/// What a cancel reaches: the flag each start of Java checks, and the process to kill.
#[derive(Default)]
struct Control {
    cancelled: bool,
    /// Set by `dispose` alone (`cancel` leaves it): the app is quitting, so a run skips its second walk.
    disposing: bool,
    proc: Option<Arc<KarateProcess>>,
}

/// The values a discovery of open and refresh starts with (pom defaults R1).
struct Start {
    filter: FilterState,
    envs: EnvSettings,
    threads: u32,
}

/// How a discovery ended: cancelled (nothing published), or with the tree it published (None after an error).
enum Discovery {
    Cancelled,
    Found(Option<Arc<TestNode>>),
}

/// The folder and proposed zip name of a run's report (export spec R2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportSource {
    pub dir: PathBuf,
    pub name: String,
}

/// What a `Runner` is made with. `new` gives the app's; the other fields are seams.
pub struct RunnerOptions {
    pub data_dir: PathBuf,
    pub vendor: VendorPaths,
    pub secrets: Arc<dyn SecretBox>,
    pub emit: Emit,
    /// The project fingerprint's walk; `None` is `fingerprint::project_fingerprint`. A test seam.
    pub fingerprint: Option<Walk>,
    /// Where `~/.karate/ext` is; `None` is the person's home folder. A test seam.
    pub home: Option<PathBuf>,
    /// Phase P4's parts; `NoSetup` until then.
    pub setup: Arc<dyn ProjectSetup>,
}

impl RunnerOptions {
    pub fn new(data_dir: PathBuf, vendor: VendorPaths, secrets: Arc<dyn SecretBox>, emit: Emit) -> Self {
        Self {
            data_dir,
            vendor,
            secrets,
            emit,
            fingerprint: None,
            home: None,
            setup: Arc::new(NoSetup),
        }
    }
}

type LoadRecent = Box<dyn Fn() -> Vec<String> + Send + Sync>;
type EmitRecent = Box<dyn Fn(AppEvent) + Send + Sync>;

/// A test's look at each start of Java: its program, arguments and environment, which it may change.
#[cfg(test)]
pub(crate) struct Spawn {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub env: Env,
    pub dry: bool,
}

#[cfg(test)]
pub(crate) type SpawnHook = Arc<dyn Fn(&mut Spawn) + Send + Sync>;

struct Inner {
    data_dir: PathBuf,
    vendor: VendorPaths,
    secrets: Arc<dyn SecretBox>,
    emit: Emit,
    walk: Walk,
    home: PathBuf,
    setup: Arc<dyn ProjectSetup>,
    pid_file: PathBuf,
    recent: Arc<RecentSender<LoadRecent, EmitRecent>>,
    state: Mutex<State>,
    /// Signalled when the phase goes back to idle, for `dispose`.
    idle: Condvar,
    ctl: Mutex<Control>,
    /// The environment Java's is made from; None is Grace's own. Tests set it, never `std::env::set_var`.
    #[cfg(test)]
    parent_env: Mutex<Option<Env>>,
    #[cfg(test)]
    spawn_hook: Mutex<Option<SpawnHook>>,
}

/// The runner: see the module's documentation.
#[derive(Clone)]
pub struct Runner(Arc<Inner>);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Runner {
    /// The app's runner: the project fingerprint of `fingerprint.rs`, `~/.karate/ext` in the person's home, no P4 setup.
    pub fn new(
        data_dir: impl Into<PathBuf>,
        vendor: VendorPaths,
        secrets: Arc<dyn SecretBox>,
        emit: impl Fn(AppEvent) + Send + Sync + 'static,
    ) -> Self {
        Self::with_options(RunnerOptions::new(data_dir.into(), vendor, secrets, Arc::new(emit)))
    }

    pub fn with_options(o: RunnerOptions) -> Self {
        let load: LoadRecent = {
            let dir = o.data_dir.clone();
            Box::new(move || load_recent_projects(&dir))
        };
        let emit_recent: EmitRecent = {
            let emit = Arc::clone(&o.emit);
            Box::new(move |e| emit(e))
        };
        Self(Arc::new(Inner {
            pid_file: o.data_dir.join("running.pid"),
            data_dir: o.data_dir,
            vendor: o.vendor,
            secrets: o.secrets,
            emit: o.emit,
            walk: o
                .fingerprint
                .unwrap_or_else(|| Arc::new(crate::fingerprint::project_fingerprint)),
            home: o.home.or_else(std::env::home_dir).unwrap_or_default(),
            setup: o.setup,
            recent: Arc::new(RecentSender::new(load, emit_recent)),
            state: Mutex::new(State::new()),
            idle: Condvar::new(),
            ctl: Mutex::new(Control::default()),
            #[cfg(test)]
            parent_env: Mutex::new(None),
            #[cfg(test)]
            spawn_hook: Mutex::new(None),
        }))
    }

    /// True only if the folder is now the open project, which is then saved first in the recent list and as the last
    /// project. Sends `opening` around the discovery and the recent list after it.
    pub fn open(&self, root: &str) -> io::Result<bool> {
        self.0.open(root)
    }

    /// Opens a folder of the recent list; refused if not listed, ignored while it is the one being opened.
    pub fn open_recent(&self, root: &str) -> io::Result<()> {
        self.0.open_recent(root)
    }

    /// Drops a folder of the recent list; refused if not listed. Allowed while busy: it touches only settings.json.
    pub fn remove_recent(&self, root: &str) {
        self.0.remove_recent(root);
    }

    /// Checks the saved recent list and emits it as a `recent` event; the app calls it after the replay of a window
    /// that loads. Blocks up to a second for a folder on a share that does not answer (`recent::TIMEOUT`).
    pub fn send_recent(&self) {
        self.0.recent.send();
    }

    /// Back to the home screen: forgets the project and its state in memory, not its saved settings. Refused while
    /// busy.
    pub fn close(&self) {
        self.0.close();
    }

    /// Discovers the open project again; nothing with no project.
    pub fn refresh(&self) -> io::Result<()> {
        self.0.refresh()
    }

    /// Discovers again with the new environment (env tags change which tests exist), and saves it once that discovery
    /// ends, also on a parse error; a cancel keeps the environment before and saves nothing. Refused while busy or for
    /// an invalid name (the environment in force is re-sent).
    pub fn set_env(&self, name: Option<&str>) -> io::Result<()> {
        self.0.set_env(name)
    }

    /// Refused while busy (the filter in force is re-sent); ignored with no project. Applied even if it cannot be
    /// saved.
    pub fn set_filter(&self, f: FilterState) {
        self.0.set_filter(f);
    }

    /// Refused while busy or for anything but a whole number from 1 to 32 (the count in force is re-sent); ignored
    /// with no project. Applied even if it cannot be saved, as the filter is. Takes the window's value as it comes
    /// (`threads.ts` checks a JSON value).
    pub fn set_threads(&self, n: &Value) {
        self.0.set_threads(n);
    }

    /// Replaces the run properties with a validated edit; a `Keep` row keeps the ciphertext of the saved secret of the
    /// same scope and name. Applied even if it cannot be saved. True once applied; false, after a notice, if refused,
    /// so the window keeps its draft.
    pub fn save_properties(&self, edit: &PropertiesEdit) -> bool {
        self.0.save_properties(edit)
    }

    /// Runs the node `key` of the tree, after a discovery that finds it again.
    pub fn run(&self, key: &str) -> io::Result<()> {
        self.0.run(key)
    }

    /// Re-run (R5): the run's selection on its environment and filter; a Re-run failed run's tests, all of them.
    pub fn rerun(&self, id: &str) -> io::Result<()> {
        self.0.rerun(id, false)
    }

    /// Re-run failed (R6): the run's failed tests on its environment and filter.
    pub fn rerun_failed(&self, id: &str) -> io::Result<()> {
        self.0.rerun(id, true)
    }

    /// Deletes the open project's run folders (R7); the tree keeps its statuses. Refused while busy.
    pub fn clear_history(&self) {
        self.0.clear_history();
    }

    /// The trend of `key` (the whole tree when None) on the environment picked now; none with no project.
    pub fn trend(&self, key: Option<&str>) -> Vec<TrendPoint> {
        self.0.trend(key)
    }

    /// The report of a listed run of the open project (R4); None if it has none. Never takes a path.
    pub fn run_report_path(&self, id: &str) -> Option<PathBuf> {
        self.0.run_report_path(id)
    }

    /// The folder and proposed zip name of a listed run's report (export spec R2.3); None if it has none.
    pub fn export_source(&self, id: &str) -> Option<ExportSource> {
        let report = self.0.run_report_path(id)?;
        let root = lock(&self.0.state).project.as_ref()?.root.clone();
        Some(ExportSource {
            dir: report.parent()?.to_path_buf(),
            name: format!("{}-report-{id}.zip", basename(&root)),
        })
    }

    /// Stops the dry run or run going on, and anything after it in the same operation. Blocks until Java has ended
    /// (`process::KarateProcess::kill`).
    pub fn cancel(&self) {
        self.0.cancel();
    }

    /// True while discovering or running: an update would kill it (R6.4).
    pub fn is_busy(&self) -> bool {
        lock(&self.0.state).busy()
    }

    /// The last finished run's report, while it exists.
    pub fn report_path(&self) -> Option<PathBuf> {
        lock(&self.0.state).report.clone().filter(|r| r.exists())
    }

    /// The last `project` event.
    pub fn current(&self) -> Option<AppEvent> {
        lock(&self.0.state).last_event()
    }

    /// What a window that just loaded needs: events sent before it subscribed are lost.
    pub fn replay(&self) -> Vec<AppEvent> {
        self.0.replay()
    }

    /// Kills a run and waits for it to record itself, 10 s at most altogether, then removes the pid file. The app
    /// quits after it whatever happens.
    pub fn dispose(&self) {
        self.0.dispose();
    }

    /// A test's environment for Java in place of Grace's own.
    #[cfg(test)]
    pub(crate) fn set_parent_env(&self, env: Env) {
        *lock(&self.0.parent_env) = Some(env);
    }

    /// A test's look at each start of Java.
    #[cfg(test)]
    pub(crate) fn set_spawn_hook(&self, hook: SpawnHook) {
        *lock(&self.0.spawn_hook) = Some(hook);
    }
}

/// The plan of one start of Java: `spawn`'s arguments in `runner.ts`.
struct SpawnPlan<'a> {
    root: &'a str,
    layout: &'a ProjectLayout,
    out_dir: &'a Path,
    dry_run: bool,
    targets: &'a [String],
    env: Option<&'a str>,
    arg_file: Option<&'a Path>,
    /// Required: a run that forgot it would run unfiltered.
    filter: &'a FilterState,
    /// Required: a run that forgot it would run on one thread.
    threads: u32,
    /// Set on Java's environment; none for a dry run.
    vars: &'a [PlainRow],
}

/// The properties of a run with secrets decrypted, and the secret values to mask.
struct Props {
    rows: Vec<PlainRow>,
    secrets: Vec<String>,
}

impl Inner {
    fn state(&self) -> MutexGuard<'_, State> {
        lock(&self.state)
    }

    fn ctl(&self) -> MutexGuard<'_, Control> {
        lock(&self.ctl)
    }

    fn emit(&self, e: AppEvent) {
        (self.emit)(e);
    }

    fn notice(&self, message: impl Into<String>) {
        self.emit(AppEvent::notice(message));
    }

    fn cancelled(&self) -> bool {
        self.ctl().cancelled
    }

    fn disposing(&self) -> bool {
        self.ctl().disposing
    }

    fn clear_proc(&self) {
        self.ctl().proc = None;
    }

    fn project_dir_of(&self, root: &str) -> PathBuf {
        project_dir(&self.data_dir, root)
    }

    /// The open project; every caller runs with one.
    fn project(&self) -> Project {
        self.state().project.clone().expect("a project is open")
    }

    /// Runs `work` as the one operation of the runner, with `busy` events around it; refused with a notice while
    /// another runs. `Ok(None)` when refused.
    fn guarded<T>(&self, work: impl FnOnce() -> io::Result<T>) -> io::Result<Option<T>> {
        {
            let mut s = self.state();
            if s.busy() {
                drop(s);
                self.notice(BUSY);
                return Ok(None);
            }
            s.phase = Phase::Busy { opening: None };
            // under the state lock, which `cancel` takes first: a cancel of this operation cannot come before it
            self.ctl().cancelled = false;
        }
        self.emit(AppEvent::Busy { busy: true });
        // idle again however `work` ends, a panic included
        struct Idle<'a>(&'a Inner);
        impl Drop for Idle<'_> {
            fn drop(&mut self) {
                self.0.state().phase = Phase::Idle;
                self.0.idle.notify_all();
                self.0.emit(AppEvent::Busy { busy: false });
                // drain(): watch mode's saves waiting, P4
            }
        }
        let _idle = Idle(self);
        work().map(Some)
    }

    fn set_opening(&self, root: Option<String>) {
        if let Phase::Busy { opening } = &mut self.state().phase {
            *opening = root;
        }
    }

    fn open(&self, root: &str) -> io::Result<bool> {
        let mut opened = false;
        self.guarded(|| {
            let root = text(&absolute(Path::new(root)));
            if !Path::new(&root).is_dir() {
                {
                    let mut s = self.state();
                    s.project = None;
                    s.tree = None;
                    s.watching = false;
                }
                // arm(): watch mode, P4
                self.send_watch();
                self.publish(LastProject {
                    project: None,
                    error: Some(format!("Folder not found: {root}")),
                    subfolders: None,
                });
                return Ok(());
            }
            self.set_opening(Some(root.clone()));
            self.emit(AppEvent::Opening {
                root: Some(root.clone()),
            });
            let opening = self.open_body(&root);
            self.set_opening(None);
            self.emit(AppEvent::Opening { root: None });
            opened = opening?;
            Ok(())
        })?;
        self.send_recent_later();
        Ok(opened)
    }

    fn open_body(&self, root: &str) -> io::Result<bool> {
        let report = self.state().report.take(); // a report of the project open before
        let skipped = skipped_properties(&self.data_dir, root);
        let layout = detect_layout(Path::new(root)).layout;
        let libraries = self.setup.list_libraries(Path::new(root), layout);
        let start = self.start(
            root,
            // unreadable: none saved, as the loaders read it
            saved_controls(&self.data_dir, root).unwrap_or(SavedControls {
                env: false,
                filter: false,
                threads: false,
            }),
            load_project_filter(&self.data_dir, root),
            load_project_env(&self.data_dir, root),
            load_project_threads(&self.data_dir, root),
        );
        let project = Project {
            root: root.to_string(),
            layout,
        };
        if !self.rediscover(project, start)? {
            self.state().report = report;
            return Ok(false);
        }
        self.send_breakpoints(Vec::new()); // another project's lines mean nothing here
        // waiting.clear(): watch mode's saves of the project open before, P4
        let watching = load_project_watch(&self.data_dir, root);
        self.state().watching = watching;
        // arm(): watch mode, P4
        self.send_watch();
        let code = self
            .setup
            .java_code(Path::new(root), layout, &self.project_dir_of(root));
        let mut notice = Vec::new();
        if !skipped.is_empty() {
            notice.push(format!(
                "Some saved properties were not loaded and are kept in settings.json: {}",
                skipped.join(", ")
            ));
        }
        notice.extend(detect_layout(Path::new(root)).pom_notice);
        notice.extend(libraries);
        notice.extend(code.said);
        if !notice.is_empty() {
            self.say(&notice, code.build);
        }
        if let Err(e) = save_opened_project(&self.data_dir, root) {
            self.recent_notice(&e);
        }
        Ok(true)
    }

    fn open_recent(&self, root: &str) -> io::Result<()> {
        let resolved = text(&absolute(Path::new(root)));
        if self.state().opening().is_some_and(|o| o == resolved) {
            return Ok(());
        }
        if !load_recent_projects(&self.data_dir).iter().any(|r| r == root) {
            self.notice(INVALID);
            return Ok(());
        }
        self.open(root).map(|_| ())
    }

    fn remove_recent(&self, root: &str) {
        if !load_recent_projects(&self.data_dir).iter().any(|r| r == root) {
            self.notice(INVALID);
            return;
        }
        if let Err(e) = remove_recent_project(&self.data_dir, root) {
            self.recent_notice(&e);
            return;
        }
        self.send_recent_later();
    }

    /// `void this.sendRecent()`: the check of the folders runs on a short thread of its own, so a share that does not
    /// answer never holds up the operation that changed the list.
    fn send_recent_later(&self) {
        let recent = Arc::clone(&self.recent);
        let started = thread::Builder::new()
            .name("grace-recent".into())
            .spawn(move || recent.send());
        if started.is_err() {
            self.recent.send();
        }
    }

    fn recent_notice(&self, err: &io::Error) {
        self.notice(format!("The recent list could not be saved: {err}"));
    }

    fn close(&self) {
        {
            let mut s = self.state();
            if s.busy() {
                drop(s);
                self.notice(BUSY);
                return;
            }
            s.project = None;
            s.tree = None;
            s.last = None;
            s.report = None;
            s.filter = no_filter();
            s.threads = 1;
            s.envs = EnvSettings::default();
            s.overlays.clear();
            s.tag_envs.clear();
            s.watching = false;
        }
        // not `publish`: `last` stays None
        self.emit(AppEvent::Project {
            project: None,
            tree: None,
            error: None,
            subfolders: None,
        });
        self.send_breakpoints(Vec::new());
        // arm(): watch mode, P4
        self.send_watch();
        if let Err(e) = forget_last_project(&self.data_dir) {
            // after the project event: the window clears its notice on every project event
            self.notice(format!("The last project could not be forgotten: {e}"));
        }
        self.send_recent_later();
    }

    fn refresh(&self) -> io::Result<()> {
        let Some(project) = self.state().project.clone() else {
            return Ok(());
        };
        self.guarded(|| self.refresh_body(project)).map(|_| ())
    }

    fn refresh_body(&self, project: Project) -> io::Result<()> {
        let root = Path::new(&project.root);
        let libraries = self.setup.list_libraries(root, project.layout);
        let (filter, envs, threads) = {
            let s = self.state();
            (s.filter.clone(), s.envs.clone(), s.threads)
        };
        let start = self.start(
            &project.root,
            // unreadable: keep every value in force rather than guess
            saved_controls(&self.data_dir, &project.root).unwrap_or(SavedControls {
                env: true,
                filter: true,
                threads: true,
            }),
            filter,
            envs,
            threads,
        );
        let (root, layout) = (project.root.clone(), project.layout);
        if !self.rediscover(project, start)? {
            return Ok(());
        }
        // arm(): karate-pom.json may have changed the feature roots (watch mode R2.4), P4
        let code = self
            .setup
            .java_code(Path::new(&root), layout, &self.project_dir_of(&root));
        let said: Vec<String> = libraries.into_iter().chain(code.said).collect();
        if !said.is_empty() {
            self.say(&said, code.build);
        }
        Ok(())
    }

    /// One notice of `said`, with the Build button when `build`.
    fn say(&self, said: &[String], build: bool) {
        self.emit(AppEvent::Notice {
            message: join_notices(said),
            success: None,
            build,
        });
    }

    fn set_env(&self, name: Option<&str>) -> io::Result<()> {
        let (busy, current, filter) = {
            let s = self.state();
            if s.project.is_none() {
                return Ok(());
            }
            (s.busy(), s.envs.env.clone(), s.filter.clone())
        };
        let invalid = name.is_some_and(|n| !valid_env_name(n));
        if busy || invalid {
            self.notice(if busy { BUSY } else { "Invalid environment name" });
            self.emit(self.env_event());
            return Ok(());
        }
        if name == current.as_deref() {
            return Ok(());
        }
        let name = name.map(String::from);
        self.guarded(|| self.switch_env(name, filter)).map(|_| ())
    }

    fn set_filter(&self, f: FilterState) {
        let root = {
            let mut s = self.state();
            if s.busy() {
                drop(s);
                self.notice(BUSY);
                self.send_filter();
                return;
            }
            let Some(project) = &s.project else {
                return;
            };
            let root = project.root.clone();
            s.filter = f.clone();
            root
        };
        self.save_filter(&root, &f);
        self.send_filter();
    }

    fn set_threads(&self, n: &Value) {
        let (root, count) = {
            let mut s = self.state();
            if s.busy() {
                drop(s);
                self.notice(BUSY);
                self.send_threads();
                return;
            }
            let Some(project) = &s.project else {
                return;
            };
            let root = project.root.clone();
            let Some(count) = parse_threads(n) else {
                drop(s);
                self.notice(THREADS_ERROR);
                self.send_threads();
                return;
            };
            s.threads = count;
            (root, count)
        };
        if let Err(e) = save_project_threads(&self.data_dir, &root, count) {
            self.notice(format!("The thread count could not be saved: {e}"));
        }
        self.send_threads();
    }

    fn save_properties(&self, edit: &PropertiesEdit) -> bool {
        let saved = {
            let mut s = self.state();
            if s.busy() {
                drop(s);
                self.notice(BUSY);
                return false;
            }
            let Some(project) = &s.project else {
                return false;
            };
            let root = project.root.clone();
            match self.stored_properties(&s.envs.properties, edit) {
                Ok(properties) => {
                    s.envs.properties = properties.clone();
                    Ok((root, properties))
                }
                Err(message) => Err(message),
            }
        };
        let (root, properties) = match saved {
            Ok(saved) => saved,
            Err(message) => {
                self.notice(message);
                return false;
            }
        };
        if let Err(e) = save_project_properties(&self.data_dir, &root, &properties) {
            self.notice(format!("The properties could not be saved: {e}"));
        }
        self.emit(self.env_event());
        true
    }

    /// The lists an edit makes from the saved ones, or the notice that refuses it.
    fn stored_properties(&self, saved: &StoredLists, edit: &PropertiesEdit) -> Result<StoredLists, String> {
        let cipher = |scope: Option<&str>, name: &str| -> Option<String> {
            let rows: &[StoredRow] = match scope {
                None => &saved.shared,
                Some(env) => saved.by_env.get(env).map_or(&[], Vec::as_slice),
            };
            rows.iter().find(|r| r.name == name).and_then(|r| match &r.value {
                StoredValue::Secret(c) => Some(c.clone()),
                StoredValue::Plain(_) => None,
            })
        };
        if let Some(error) = edit_errors(edit, |scope, name| cipher(scope, name).is_some()).first() {
            return Err(format!("Properties not saved: {error}"));
        }
        let new_secret = |r: &RowEdit| matches!(r, RowEdit::Value { secret: true, .. });
        let all = edit.shared.iter().chain(edit.by_env.values().flatten());
        if !self.secrets.available() && all.clone().any(new_secret) {
            return Err("Secret storage is not available on this system".into());
        }
        let stored = |scope: Option<&str>, rows: &[RowEdit]| -> Result<Vec<StoredRow>, String> {
            rows.iter()
                .map(|r| {
                    let value = match r {
                        // `edit_errors` checked the secret is saved
                        RowEdit::Keep { name, .. } => StoredValue::Secret(cipher(scope, name).unwrap_or_default()),
                        RowEdit::Value {
                            value, secret: true, ..
                        } => StoredValue::Secret(self.secrets.encrypt(value).map_err(|e| e.to_string())?),
                        RowEdit::Value { value, .. } => StoredValue::Plain(value.clone()),
                    };
                    Ok(StoredRow {
                        name: crate::model::env::Named::name(r).to_string(),
                        value,
                        env_var: r.is_env_var(),
                    })
                })
                .collect()
        };
        let mut by_env = IndexMap::new();
        // `Object.fromEntries`: an environment named like an index comes first, as JavaScript lists an object's keys
        for env in js::key_order(edit.by_env.keys()) {
            by_env.insert(env.clone(), stored(Some(env), &edit.by_env[env])?);
        }
        Ok(StoredLists {
            shared: stored(None, &edit.shared)?,
            by_env,
        })
    }

    fn run(&self, key: &str) -> io::Result<()> {
        if self.state().project.is_none() {
            return Ok(());
        }
        self.guarded(|| {
            let (old, filter) = {
                let s = self.state();
                let old = s
                    .tree
                    .as_deref()
                    .and_then(|t| crate::karate::tree::find_node(t, key))
                    .map(OldNode::from);
                (old, s.filter.clone())
            };
            self.run_target(
                RunTarget::Node {
                    key: key.to_string(),
                    old,
                    gone: "This test changed; the tree was refreshed",
                },
                filter,
            )
        })
        .map(|_| ())
    }

    fn rerun(&self, id: &str, failed_only: bool) -> io::Result<()> {
        if self.state().project.is_none() {
            return Ok(());
        }
        self.guarded(|| self.rerun_guarded(id, failed_only)).map(|_| ())
    }

    fn clear_history(&self) {
        {
            let s = self.state();
            if s.busy() {
                drop(s);
                self.notice(BUSY);
                return;
            }
            let Some(project) = &s.project else {
                return;
            };
            // under the lock: no run can start its folder meanwhile. The toolbar Report stays: `report_path` checks
            // the file, which a held folder can keep.
            let cleared = clear_history(&self.project_dir_of(&project.root));
            drop(s);
            if !cleared {
                self.notice("Some runs could not be deleted");
            }
        }
        self.send_history();
    }

    fn trend(&self, key: Option<&str>) -> Vec<TrendPoint> {
        let (project, tree, env) = {
            let s = self.state();
            (s.project.clone(), s.tree.clone(), s.envs.env.clone())
        };
        find_trend(&self.listed(project.as_ref()), tree.as_deref(), key, env.as_deref())
    }

    fn run_report_path(&self, id: &str) -> Option<PathBuf> {
        let root = self.state().project.as_ref()?.root.clone();
        let dir = self.project_dir_of(&root);
        read_run_record(&dir, id)?; // also checks the id
        let report = dir.join("runs").join(id).join("karate-summary.html");
        report.exists().then_some(report)
    }

    fn cancel(&self) {
        // waiting.clear(): Cancel means stop (watch mode R4.2), P4
        let proc = {
            let s = self.state();
            if !s.busy() {
                return;
            }
            // also between two JVMs of one operation: each start checks it (a re-run's switch, then its run)
            let mut c = self.ctl();
            c.cancelled = true;
            c.proc.clone()
        };
        if let Some(proc) = proc {
            proc.kill();
        }
    }

    fn replay(&self) -> Vec<AppEvent> {
        let (head, project, tree, breakpoints, opening, busy) = {
            let s = self.state();
            let head = s.last_event().map(|last| {
                [
                    last,
                    AppEvent::Filter {
                        filter: s.filter.clone(),
                    },
                    AppEvent::Threads { count: s.threads },
                    AppEvent::Watch { on: s.watching },
                    self.env_event_of(&s),
                ]
            });
            (
                head,
                s.project.clone(),
                s.tree.clone(),
                s.breakpoints.clone(),
                s.opening(),
                s.busy(),
            )
        };
        let opening = AppEvent::Opening { root: opening };
        // `listing` while a build tool runs: P4
        let busy = AppEvent::Busy { busy };
        let Some(head) = head else {
            return vec![opening, busy];
        };
        let mut events = Vec::from(head);
        events.extend(self.history_events(project.as_ref(), tree.as_deref()));
        events.push(AppEvent::Breakpoints { breakpoints });
        events.push(opening);
        events.push(busy);
        events
    }

    fn dispose(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        let proc = {
            let mut c = self.ctl();
            c.disposing = true;
            c.cancelled = true;
            c.proc.clone()
        };
        // the watcher and the saves waiting: P4
        // The kill ends the run's process; the run then writes its record before the operation ends.
        if let Some(proc) = proc {
            proc.kill();
        }
        let s = self.state();
        let wait = deadline.saturating_duration_since(Instant::now());
        drop(
            self.idle
                .wait_timeout_while(s, wait, |s| s.busy())
                .unwrap_or_else(PoisonError::into_inner),
        );
        let _ = fs::remove_file(&self.pid_file);
    }

    fn send_breakpoints(&self, breakpoints: Vec<Breakpoint>) {
        self.state().breakpoints.clone_from(&breakpoints);
        // the debug session's breakpoints: P4
        self.emit(AppEvent::Breakpoints { breakpoints });
    }

    /// Sets the last project event, and sends it and the flaky tests of its tree.
    fn publish(&self, last: LastProject) {
        let (event, project, tree) = {
            let mut s = self.state();
            s.last = Some(last);
            (s.last_event(), s.project.clone(), s.tree.clone())
        };
        if let Some(event) = event {
            self.emit(event);
        }
        // the marks follow the tree that is shown
        let runs = self.listed(project.as_ref());
        self.emit(AppEvent::Flaky {
            tests: find_flaky(&runs, tree.as_deref()),
        });
    }

    fn send_filter(&self) {
        let filter = self.state().filter.clone();
        self.emit(AppEvent::Filter { filter });
    }

    fn send_threads(&self) {
        let count = self.state().threads;
        self.emit(AppEvent::Threads { count });
    }

    fn send_watch(&self) {
        let on = self.state().watching;
        self.emit(AppEvent::Watch { on });
    }

    /// The plain value of a saved secret; None if it cannot be decrypted or Java cannot take it exactly.
    fn read_secret(&self, cipher: &str) -> Option<String> {
        if !self.secrets.available() {
            return None;
        }
        self.secrets
            .decrypt(cipher)
            .filter(|value| property_value_error(value).is_none())
    }

    fn env_event(&self) -> AppEvent {
        let s = self.state();
        self.env_event_of(&s)
    }

    /// A secret row carries only whether it can be read, never its value.
    fn env_event_of(&self, s: &State) -> AppEvent {
        let EnvSettings { env, properties } = &s.envs;
        let env_var = |r: &StoredRow| r.env_var.then_some(true);
        let row = |r: &StoredRow| match &r.value {
            StoredValue::Secret(cipher) => RowView::Secret(SecretRowView {
                name: r.name.clone(),
                secret: true,
                state: if self.read_secret(cipher).is_some() {
                    SecretState::Set
                } else {
                    SecretState::Unreadable
                },
                env_var: env_var(r),
            }),
            StoredValue::Plain(value) => RowView::Plain(PlainRow {
                name: r.name.clone(),
                value: value.clone(),
                env_var: env_var(r),
            }),
        };
        AppEvent::Environment {
            view: EnvView {
                env: env.clone(),
                suggestions: sorted_unique(&[
                    s.overlays.clone(),
                    s.tag_envs.clone(),
                    properties.by_env.keys().cloned().collect(),
                    env.iter().cloned().collect(),
                ]),
                overlays: s.overlays.clone(),
                properties: PropertyListsView {
                    shared: properties.shared.iter().map(row).collect(),
                    by_env: properties
                        .by_env
                        .iter()
                        .map(|(e, rows)| (e.clone(), rows.iter().map(row).collect()))
                        .collect(),
                },
                secrets_available: self.secrets.available(),
            },
        }
    }

    /// The project's listed runs, newest first; none with no project.
    fn listed(&self, project: Option<&Project>) -> Vec<ListedRun> {
        project.map_or_else(Vec::new, |p| list_runs(&self.project_dir_of(&p.root)))
    }

    /// `history`, then `flaky`, from one read of the runs.
    fn history_events(&self, project: Option<&Project>, tree: Option<&TestNode>) -> [AppEvent; 2] {
        let runs = self.listed(project);
        let flaky = find_flaky(&runs, tree);
        let entries: Vec<HistoryEntry> = runs
            .into_iter()
            .map(|r| to_entry(&r.id, r.record, r.report_available))
            .collect();
        [AppEvent::History { runs: entries }, AppEvent::Flaky { tests: flaky }]
    }

    fn send_history(&self) {
        let (project, tree) = {
            let s = self.state();
            (s.project.clone(), s.tree.clone())
        };
        for e in self.history_events(project.as_ref(), tree.as_deref()) {
            self.emit(e);
        }
    }

    /// Open and Refresh (pom defaults R1): each control's value when `saved`, else the pom's, else Grace's default.
    fn start(&self, root: &str, saved: SavedControls, filter: FilterState, envs: EnvSettings, threads: u32) -> Start {
        let pom = pom_start(Path::new(root), self.setup.as_ref());
        Start {
            filter: if saved.filter {
                filter
            } else {
                pom.filter.unwrap_or_default()
            },
            envs: if saved.env {
                envs
            } else {
                EnvSettings { env: pom.env, ..envs }
            },
            threads: if saved.threads {
                threads
            } else {
                pom.threads.unwrap_or(1)
            },
        }
    }

    /// Applies `f` even if it cannot be saved: a disk error must not undo the change.
    fn store_filter(&self, f: FilterState) {
        let root = {
            let mut s = self.state();
            s.filter = f.clone();
            s.project.as_ref().map(|p| p.root.clone())
        };
        if let Some(root) = root {
            self.save_filter(&root, &f);
        }
    }

    fn save_filter(&self, root: &str, f: &FilterState) {
        if let Err(e) = save_project_filter(&self.data_dir, root, f) {
            self.notice(format!("The filter could not be saved: {e}"));
        }
    }

    /// Discovery of open, refresh and set_env with `start.envs`, then `start.filter` without the tags the new tree
    /// lacks (a failed dry run drops nothing), then `start.threads`, then `environment`. The values replace those in
    /// force only once the discovery succeeded, so nothing shows or uses them before. Cancel puts the project and tree
    /// from before back and re-sends them, the filter and the thread count, which it leaves unchanged, and the
    /// environment.
    fn rediscover(&self, project: Project, start: Start) -> io::Result<bool> {
        let before = {
            let mut s = self.state();
            (s.project.replace(project.clone()), s.tree.clone())
        };
        let restore = |(project, tree): (Option<Project>, Option<Arc<TestNode>>)| {
            let mut s = self.state();
            s.project = project;
            s.tree = tree;
        };
        let found = match self.discover(start.envs.env.as_deref()) {
            Ok(found) => found,
            Err(e) => {
                restore(before);
                self.emit(self.env_event()); // the window may still show the name it sent
                return Err(e);
            }
        };
        // Rust-native: the discovery says whether it was cancelled, in place of a second look at the flag. A cancel
        // that comes after it published its tree has nothing left to stop here.
        let Discovery::Found(tree) = found else {
            restore(before);
            let last = self.state().last_event().unwrap_or(AppEvent::Project {
                project: None,
                tree: None,
                error: None,
                subfolders: None,
            });
            self.emit(last);
            self.send_filter();
            self.send_threads();
            self.emit(self.env_event());
            self.send_history();
            self.notice("Cancelled");
            return Ok(false);
        };
        let Start { filter, envs, threads } = start;
        let pruned = tree.as_deref().map(|t| prune_filter(filter.clone(), &project_tags(t)));
        {
            let mut s = self.state();
            s.envs = envs;
            s.filter = filter.clone();
            s.threads = threads;
        }
        if let Some(pruned) = pruned
            && (pruned.include.len() != filter.include.len() || pruned.exclude.len() != filter.exclude.len())
        {
            // a filter from the pom stays unsaved (pom defaults R1.2)
            if saved_controls(&self.data_dir, &project.root).is_none_or(|c| c.filter) {
                self.store_filter(pruned);
            } else {
                self.state().filter = pruned;
            }
        }
        self.send_filter();
        self.send_threads();
        let layout = detect_layout(Path::new(&project.root));
        let overlays = overlay_envs(&layout.config_dirs);
        // from the files: the tree lacks other environments' tests
        let tag_envs = feature_tag_envs(&layout.feature_roots);
        {
            let mut s = self.state();
            s.overlays = overlays;
            s.tag_envs = tag_envs;
        }
        self.emit(self.env_event());
        self.send_history();
        Ok(true)
    }

    /// Environment switcher R1: discovery with `env` and `filter`, then the environment saved, also after a parse
    /// error. False if cancelled, which keeps the environment before and saves nothing.
    fn switch_env(&self, env: Option<String>, filter: FilterState) -> io::Result<bool> {
        let (project, envs, threads) = {
            let s = self.state();
            let envs = EnvSettings {
                env,
                properties: s.envs.properties.clone(),
            };
            (s.project.clone().expect("a project is open"), envs, s.threads)
        };
        let start = Start {
            filter,
            envs: envs.clone(),
            threads,
        };
        let root = project.root.clone();
        if !self.rediscover(project, start)? {
            return Ok(false);
        }
        self.state().report = None; // a result from another environment
        if let Err(e) = save_project_env(&self.data_dir, &root, &envs) {
            self.notice(format!("The environment could not be saved: {e}"));
        }
        Ok(true)
    }

    /// Empties and returns `<project dir>/dryrun`, so no file of an earlier dry run is ever read.
    fn dry_run_dir(&self, root: &str) -> io::Result<PathBuf> {
        let dir = self.project_dir_of(root).join("dryrun");
        history::remove_retrying(&dir, 5).map_err(at(&dir))?;
        fs::create_dir_all(&dir).map_err(at(&dir))?;
        Ok(dir)
    }

    /// Starts Java for a dry run or a run, unless the operation was cancelled: the check and the start are one step
    /// for `cancel`, which then kills it. None when cancelled.
    fn spawn(
        &self,
        p: SpawnPlan<'_>,
        on_lines: impl FnMut(Vec<String>) + Send + 'static,
    ) -> Option<Arc<KarateProcess>> {
        let root = Path::new(p.root);
        // discovery lists every test; a run gets the filter
        let tags = if p.dry_run { Vec::new() } else { tag_args(p.filter) };
        let mut classpath = p.layout.classpath.clone();
        classpath.extend(extension_jars(&self.home, root));
        classpath.extend(self.setup.kept_jars(&self.project_dir_of(p.root), p.layout.layout));
        let args = build_args(&BuildArgs {
            jar: &self.vendor.jar,
            classpath: &classpath,
            root,
            out_dir: p.out_dir,
            dry_run: p.dry_run,
            tags: &tags,
            targets: p.targets,
            env: p.env,
            arg_file: p.arg_file,
            threads: p.threads,
            config_dir: p.layout.config_dir.as_deref(),
            debug: None,                                 // the step debugger: P4
            junit_xml: !p.dry_run && p.layout.junit_xml, // the dry run's reports are never read
        });
        #[cfg(test)]
        let parent: Env = lock(&self.parent_env)
            .clone()
            .unwrap_or_else(|| std::env::vars_os().collect());
        #[cfg(not(test))]
        let parent = std::env::vars_os();
        let env = child_env(parent, p.vars);
        #[allow(unused_mut)]
        let (mut java, mut args, mut env) = (self.vendor.java.clone(), args, env);
        #[cfg(test)]
        let hook = lock(&self.spawn_hook).clone();
        #[cfg(test)]
        if let Some(hook) = hook {
            let mut spawn = Spawn {
                java,
                args,
                env,
                dry: p.dry_run,
            };
            hook(&mut spawn);
            (java, args, env) = (spawn.java, spawn.args, spawn.env);
        }
        let mut c = self.ctl();
        if c.cancelled {
            return None;
        }
        let options = SpawnOptions {
            env: Some(env),
            ..SpawnOptions::default()
        };
        let proc = Arc::new(spawn_karate(&java, &args, root, on_lines, options));
        c.proc = Some(Arc::clone(&proc));
        Some(proc)
    }

    /// Dry run on all feature roots with `env`; publishes the new tree, or the error.
    fn discover(&self, env: Option<&str>) -> io::Result<Discovery> {
        if self.cancelled() {
            return Ok(Discovery::Cancelled); // cancelled before it started
        }
        let (project, threads) = {
            let s = self.state();
            (s.project.clone().expect("a project is open"), s.threads)
        };
        let root = Path::new(&project.root);
        let layout = detect_layout(root);
        let root_name = basename(&project.root);
        let (tree, error) = if layout.feature_roots.is_empty() {
            // Karate with no paths would load a pom or print help
            (build_tree(&[], &root_name).ok(), None)
        } else {
            let out_dir = self.dry_run_dir(&project.root)?;
            let lines = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&lines);
            let targets: Vec<String> = layout.feature_roots.iter().map(|r| text(r)).collect();
            let plan = SpawnPlan {
                root: &project.root,
                layout: &layout,
                out_dir: &out_dir,
                dry_run: true,
                targets: &targets,
                env,
                arg_file: None,
                filter: &no_filter(),
                threads,   // build_args passes 1 to a dry run
                vars: &[], // dry runs do not execute config (environment switcher R2)
            };
            let Some(proc) = self.spawn(plan, move |l| lock(&sink).extend(l)) else {
                return Ok(Discovery::Cancelled);
            };
            let code = proc.wait();
            self.clear_proc();
            if self.cancelled() {
                return Ok(Discovery::Cancelled); // killed on purpose: keep the last tree
            }
            if code != Some(0) {
                (None, Some(dry_run_error(&lock(&lines))))
            } else {
                match read_tree(&out_dir, &root_name, &layout.bases) {
                    Ok(tree) => (Some(tree), None),
                    Err(error) => (None, Some(error)),
                }
            }
        };
        let subfolders = match &tree {
            Some(t) if leaf_keys(t).is_empty() => self.setup.karate_folders(root),
            _ => Vec::new(),
        };
        if self.cancelled() {
            return Ok(Discovery::Cancelled); // cancelled during the walk: keep the last tree
        }
        let tree = tree.map(Arc::new);
        self.state().tree = tree.clone();
        self.publish(LastProject {
            project: Some(project),
            error,
            subfolders: (!subfolders.is_empty()).then_some(subfolders),
        });
        Ok(Discovery::Found(tree))
    }
}

/// The dry run's tree: its events, then each row's line from its feature file. An error is the message the project
/// event shows.
fn read_tree(out_dir: &Path, root_name: &str, bases: &[PathBuf]) -> Result<TestNode, String> {
    let file = out_dir.join("karate-json").join("karate-events.jsonl");
    let events = read_all_events(&file).map_err(|e| e.to_string())?;
    let mut tree = build_tree(&events, root_name).map_err(|e| e.to_string())?;
    set_row_lines(&mut tree, bases)?;
    Ok(tree)
}

/// Sets each row's line from its feature file; None when the table has no such row (dynamic Examples), since running
/// a guessed line could run another row or nothing.
fn set_row_lines(tree: &mut TestNode, bases: &[PathBuf]) -> Result<(), String> {
    fn walk(n: &mut TestNode, bases: &[PathBuf], texts: &mut HashMap<String, String>) -> Result<(), String> {
        if n.kind == NodeKind::Row {
            let id = parse_ref_id(n.ref_id.as_deref().unwrap_or_default()).map_err(|e| e.to_string())?;
            if !texts.contains_key(&n.file) {
                let file = locate(bases, &n.file);
                let bytes = fs::read(&file).map_err(|e| format!("{}: {e}", file.display()))?;
                texts.insert(n.file.clone(), String::from_utf8_lossy(&bytes).into_owned());
            }
            let examples = u32::try_from(id.line).unwrap_or(u32::MAX);
            let lines = row_lines(&texts[&n.file], examples);
            n.line = id
                .example
                .and_then(|e| usize::try_from(e).ok()?.checked_sub(1))
                .and_then(|i| lines.get(i).copied());
        }
        n.children.iter_mut().try_for_each(|c| walk(c, bases, texts))
    }
    walk(tree, bases, &mut HashMap::new())
}
