//! The runner's tests, `src/main/runner.*.test.ts` test for test, with real Java and Karate on copies of
//! `test/fixtures/projects/*`. This module is `test/runner.ts`: each test makes its own runner, data folder, home
//! folder and fixture copy, all removed when it ends, so the tests run in parallel.
use super::{Runner, RunnerOptions, Spawn, Walk, lock, project_dir};
use crate::history::{create_run_dir, write_run_record};
use crate::karate::events::read_all_events;
use crate::karate::testing::copy_dir;
use crate::karate::tree::leaf_keys;
use crate::model::events::AppEvent;
use crate::model::history::{LeafKind, RunRecord, StoredSelection, StoredTest, TestStatusFinal};
use crate::model::tags::no_filter;
use crate::model::{NodeKind, TestNode};
use crate::secrets::{FakeSecretBox, SecretBox};
use crate::vendor::{VendorPaths, vendor_paths};
use chrono::TimeZone;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tempfile::TempDir;

mod discovery;
mod editor;
mod env;
mod fingerprint;
mod flaky;
mod history;
mod home;
mod properties;
mod rerun;
mod runs;
mod tags;
mod threads;
mod trends;

pub(super) const SECRET: &str = "s3cret-token-123";

/// `vendorPaths(path.resolve("vendor"))`: on Linux the `java` of `GRACE_JAVA` or `PATH`.
pub(super) fn vendor() -> VendorPaths {
    vendor_paths(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("vendor"),
    )
}

/// A temporary folder, prefixed as the TypeScript's `mkdtempSync(path.join(tmpdir(), prefix))`.
pub(super) fn temp(prefix: &str) -> TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap()
}

/// `copyFixture(name)`: a fresh copy of a fixture project, removed when dropped. `broken` is `clean` plus
/// `broken-extra`.
pub(super) struct Fixture {
    pub root: String,
    _dir: TempDir,
}

impl Fixture {
    pub fn path(&self) -> &Path {
        Path::new(&self.root)
    }

    pub fn join(&self, rel: &str) -> PathBuf {
        self.path().join(rel)
    }
}

pub(super) fn fixture(name: &str) -> Fixture {
    let dir = temp(&format!("kk-{name}-"));
    let projects = crate::karate::testing::fixture("x").parent().unwrap().to_path_buf();
    copy_dir(
        &projects.join(if name == "broken" { "clean" } else { name }),
        dir.path(),
    );
    if name == "broken" {
        copy_dir(&projects.join("broken-extra"), dir.path());
    }
    Fixture {
        root: dir.path().to_string_lossy().into_owned(),
        _dir: dir,
    }
}

type Listener = Arc<dyn Fn(&AppEvent) + Send + Sync>;

/// What the runner emitted, with each event's JSON, as the window would receive it.
#[derive(Default)]
pub(super) struct Sink {
    events: Mutex<Vec<(AppEvent, Value)>>,
    listener: Mutex<Option<Listener>>,
}

impl Sink {
    fn push(&self, e: AppEvent) {
        let json = serde_json::to_value(&e).unwrap();
        lock(&self.events).push((e.clone(), json));
        // outside the events' lock: a listener may read them
        let listener = lock(&self.listener).clone();
        if let Some(f) = listener {
            f(&e);
        }
    }
}

/// What `setup` makes a runner with; `Default` is the TypeScript's `setup()`.
#[derive(Default)]
pub(super) struct Setup {
    /// The data folder of another runner, shared.
    pub data: Option<Arc<TempDir>>,
    pub secrets: Option<Arc<dyn SecretBox>>,
    pub fingerprint: Option<Walk>,
}

/// `setup()`'s result. Dropping it disposes the runner and removes its folders (`cleanUp`).
pub(super) struct TestRunner {
    pub runner: Runner,
    pub data: Arc<TempDir>,
    pub data_dir: PathBuf,
    sink: Arc<Sink>,
    _home: TempDir,
}

pub(super) fn setup() -> TestRunner {
    setup_with(Setup::default())
}

pub(super) fn setup_with(o: Setup) -> TestRunner {
    let data = o.data.unwrap_or_else(|| Arc::new(temp("kk-data-")));
    // never the developer's own ~/.karate/ext
    let home = temp("kk-home-");
    let sink = Arc::new(Sink::default());
    let emit = {
        let sink = Arc::clone(&sink);
        Arc::new(move |e| sink.push(e))
    };
    let mut options = RunnerOptions::new(
        data.path().to_path_buf(),
        vendor(),
        o.secrets.unwrap_or_else(|| Arc::new(FakeSecretBox::new(true))),
        emit,
    );
    options.fingerprint = o.fingerprint;
    options.home = Some(home.path().to_path_buf());
    TestRunner {
        runner: Runner::with_options(options),
        data_dir: data.path().to_path_buf(),
        data,
        sink,
        _home: home,
    }
}

impl Drop for TestRunner {
    fn drop(&mut self) {
        // a listener may hold the runner
        *lock(&self.sink.listener) = None;
        self.runner.dispose();
    }
}

/// An event's `type`.
fn type_of(json: &Value) -> &str {
    json["type"].as_str().unwrap_or_default()
}

impl TestRunner {
    /// Every event so far.
    pub fn events(&self) -> Vec<AppEvent> {
        lock(&self.sink.events).iter().map(|(e, _)| e.clone()).collect()
    }

    /// Every event so far, as JSON.
    pub fn json(&self) -> Vec<Value> {
        lock(&self.sink.events).iter().map(|(_, j)| j.clone()).collect()
    }

    pub fn len(&self) -> usize {
        lock(&self.sink.events).len()
    }

    /// `all(type)`, as JSON.
    pub fn all(&self, kind: &str) -> Vec<Value> {
        lock(&self.sink.events)
            .iter()
            .filter(|(_, j)| type_of(j) == kind)
            .map(|(_, j)| j.clone())
            .collect()
    }

    pub fn count(&self, kind: &str) -> usize {
        lock(&self.sink.events)
            .iter()
            .filter(|(_, j)| type_of(j) == kind)
            .count()
    }

    /// `last(type)`, as JSON.
    pub fn last(&self, kind: &str) -> Value {
        self.all(kind).pop().unwrap_or_else(|| panic!("no {kind} event"))
    }

    /// `last(type)`, the event itself.
    pub fn last_event(&self, kind: &str) -> AppEvent {
        lock(&self.sink.events)
            .iter()
            .rev()
            .find(|(_, j)| type_of(j) == kind)
            .map(|(e, _)| e.clone())
            .unwrap_or_else(|| panic!("no {kind} event"))
    }

    /// The messages of every notice.
    pub fn notices(&self) -> Vec<String> {
        self.all("notice")
            .iter()
            .map(|n| n["message"].as_str().unwrap().to_string())
            .collect()
    }

    pub fn last_notice(&self) -> String {
        self.notices().pop().expect("a notice")
    }

    /// The tree of the last `project` event.
    pub fn tree(&self) -> TestNode {
        match self.last_event("project") {
            AppEvent::Project { tree: Some(tree), .. } => tree,
            e => panic!("no tree in {e:?}"),
        }
    }

    /// The key of the node named `name` in the last tree.
    pub fn key(&self, name: &str) -> String {
        by_name(&self.tree(), name).key
    }

    /// `onEvent(f)`: calls `f` with each event as it is emitted, before the runner goes on.
    pub fn on_event(&self, f: impl Fn(&AppEvent) + Send + Sync + 'static) {
        *lock(&self.sink.listener) = Some(Arc::new(f));
    }

    /// Records each start of Java: its program, arguments and environment (the TypeScript's mocked `spawnKarate`).
    pub fn record_spawns(&self) -> Arc<Mutex<Vec<Started>>> {
        let starts: Arc<Mutex<Vec<Started>>> = Arc::default();
        let sink = Arc::clone(&starts);
        self.runner.set_spawn_hook(Arc::new(move |s: &mut Spawn| {
            lock(&sink).push(Started {
                dry: s.dry,
                args: s.args.clone(),
                env: s.env.clone(),
            });
        }));
        starts
    }

    /// True while a Java the runner started runs.
    pub fn java_up(&self) -> bool {
        lock(&self.runner.0.ctl).proc.is_some()
    }

    /// Runs `op` on the runner from a thread of its own, as a call the test does not await yet.
    pub fn bg<T: Send + 'static>(&self, op: impl FnOnce(&Runner) -> T + Send + 'static) -> JoinHandle<T> {
        let runner = self.runner.clone();
        thread::spawn(move || op(&runner))
    }
}

/// A start of Java, as `record_spawns` saw it.
#[derive(Clone, Debug)]
pub(super) struct Started {
    pub dry: bool,
    pub args: Vec<String>,
    pub env: Vec<(OsString, OsString)>,
}

impl Started {
    /// The value of the variable `name` exactly; None if it is not set.
    pub fn var(&self, name: &str) -> Option<String> {
        self.env
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.to_string_lossy().into_owned())
    }

    /// The variables named `name` in any letter case, as Windows sees one.
    pub fn vars_any_case(&self, name: &str) -> Vec<(String, String)> {
        self.env
            .iter()
            .filter(|(k, _)| k.to_string_lossy().to_uppercase() == name.to_uppercase())
            .map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned()))
            .collect()
    }
}

/// The JSON of an event.
pub(super) fn json(e: &AppEvent) -> Value {
    serde_json::to_value(e).unwrap()
}

/// `toMatchObject`: every key of `expected` is in `actual` with a matching value, at any depth; arrays match element
/// for element, with the same length.
pub(super) fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(a), Value::Object(e)) => e
            .iter()
            .all(|(k, ev)| a.get(k).is_some_and(|av| matches_object(av, ev))),
        (Value::Array(a), Value::Array(e)) => {
            a.len() == e.len() && a.iter().zip(e).all(|(av, ev)| matches_object(av, ev))
        }
        (Value::Number(a), Value::Number(e)) => a.as_f64() == e.as_f64(),
        _ => actual == expected,
    }
}

#[track_caller]
pub(super) fn assert_match(actual: &Value, expected: Value) {
    assert!(
        matches_object(actual, &expected),
        "expected\n{}\nto match\n{}",
        serde_json::to_string_pretty(actual).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap()
    );
}

/// Every console line of `events`.
pub(super) fn printed(events: &[AppEvent]) -> Vec<String> {
    events
        .iter()
        .flat_map(|e| match e {
            AppEvent::Console { lines } => lines.clone(),
            _ => Vec::new(),
        })
        .collect()
}

pub(super) fn printed_has(events: &[AppEvent], text: &str) -> bool {
    printed(events).iter().any(|l| l.contains(text))
}

/// The leaves of a tree; -1 for none.
pub(super) fn leaves(tree: Option<&TestNode>) -> i64 {
    tree.map_or(-1, |t| leaf_keys(t).len() as i64)
}

pub(super) fn by_name(tree: &TestNode, name: &str) -> TestNode {
    fn find<'a>(n: &'a TestNode, name: &str) -> Option<&'a TestNode> {
        if n.name == name {
            return Some(n);
        }
        n.children.iter().find_map(|c| find(c, name))
    }
    find(tree, name)
        .unwrap_or_else(|| panic!("no node named {name}"))
        .clone()
}

/// Waits until `check` holds, looking every 50 ms; panics after `ms`.
#[track_caller]
pub(super) fn until_within(check: impl Fn() -> bool, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while !check() {
        assert!(Instant::now() <= end, "timed out waiting");
        thread::sleep(Duration::from_millis(50));
    }
}

#[track_caller]
pub(super) fn until(check: impl Fn() -> bool) {
    until_within(check, 60_000);
}

/// Each file of the tree under `root` and the SHA-1 of its content, by its path with `/`.
pub(super) fn hash_tree(root: &Path) -> BTreeMap<String, String> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(
                    rel,
                    sha1_smol::Sha1::from(fs::read(&path).unwrap()).digest().to_string(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// The events file of `<project dir>/<dir>`.
pub(super) fn events_file(data_dir: &Path, root: &str, dir: &str) -> PathBuf {
    project_dir(data_dir, root)
        .join(dir)
        .join("karate-json")
        .join("karate-events.jsonl")
}

/// The `-T` a Karate run was given, as its first SUITE_ENTER records it.
pub(super) fn suite_threads(file: &Path) -> u64 {
    let events = read_all_events(file).unwrap();
    let enter = events.iter().find(|e| e.kind.as_str() == "SUITE_ENTER").unwrap();
    enter.data["threads"].as_u64().unwrap()
}

/// Command lines of the running Java processes of the project at `root`.
#[cfg(not(windows))]
pub(super) fn java_command_lines(root: &str) -> Vec<String> {
    let Ok(procs) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    procs
        .flatten()
        .filter_map(|p| fs::read(p.path().join("cmdline")).ok())
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .trim_end_matches('\0')
                .replace('\0', " ")
        })
        .filter(|l| l.contains("io.karatelabs.Main") && l.contains(root))
        .collect()
}

/// Command lines of the running java.exe processes of the project at `root`.
#[cfg(windows)]
pub(super) fn java_command_lines(root: &str) -> Vec<String> {
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-CimInstance Win32_Process -Filter \"Name='java.exe'\").CommandLine",
        ])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains(root))
        .map(String::from)
        .collect()
}

/// `holdSlow`: makes the `env` fixture's `slow` scenario wait for `release` instead of sleeping 3 s, so a test can
/// read the JVM's command line before the JVM exits.
pub(super) struct Gate(PathBuf);

impl Gate {
    pub fn release(&self) {
        fs::write(&self.0, "").unwrap();
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) fn hold_slow(root: &str) -> Gate {
    let gate = PathBuf::from(format!("{root}.release")); // outside the project, so no walk sees it
    let feature = Path::new(root).join("tests").join("env.feature");
    let text = fs::read_to_string(&feature).unwrap();
    let held = text.replace(
        "java.lang.Thread.sleep(3000)",
        &format!(
            "eval while (!new java.io.File('{}').exists()) java.lang.Thread.sleep(50)",
            gate.to_string_lossy().replace('\\', "/")
        ),
    );
    assert_ne!(held, text, "no slow step in {}", feature.display());
    fs::write(&feature, held).unwrap();
    Gate(gate)
}

/// The run folder names of the project, oldest first.
pub(super) fn folders(data_dir: &Path, root: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(project_dir(data_dir, root).join("runs")) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

pub(super) fn record_file(data_dir: &Path, root: &str, id: &str) -> PathBuf {
    project_dir(data_dir, root).join("runs").join(id).join("run.json")
}

/// A run's `run.json`, as JSON.
pub(super) fn record(data_dir: &Path, root: &str, id: &str) -> Value {
    serde_json::from_slice(&fs::read(record_file(data_dir, root, id)).unwrap()).unwrap()
}

/// Run All records, oldest first, with `over` applied to each.
pub(super) fn seed(data_dir: &Path, root: &str, count: u32, over: impl Fn(&mut RunRecord)) -> Vec<String> {
    (0..count)
        .map(|i| {
            let at = chrono::Local.with_ymd_and_hms(2020, 1, 1, 0, 0, i).unwrap();
            let run = create_run_dir(&project_dir(data_dir, root), &at).unwrap();
            let mut record = RunRecord {
                version: 1,
                started_at: at.timestamp_millis() as f64,
                duration_millis: 1000.0,
                env: None,
                threads: Some(1),
                fingerprint: None,
                filter: no_filter(),
                selection: Some(StoredSelection {
                    kind: NodeKind::Folder,
                    file: String::new(),
                    name: Path::new(root).file_name().unwrap().to_string_lossy().into_owned(),
                    line: None,
                    key: "d:".into(),
                }),
                failed_of: None,
                outcome: crate::model::events::RunOutcome::Finished,
                exit_code: Some(0),
                tests: Vec::new(),
                debug: false,
            };
            over(&mut record);
            write_run_record(&run.dir, &record).unwrap();
            run.id
        })
        .collect()
}

pub(super) fn stored(n: &TestNode, status: TestStatusFinal) -> StoredTest {
    StoredTest {
        kind: if n.kind == NodeKind::Row {
            LeafKind::Row
        } else {
            LeafKind::Scenario
        },
        file: n.file.clone(),
        ref_id: n.ref_id.clone().unwrap(),
        name: n.name.clone(),
        line: n.line,
        key: n.key.clone(),
        status,
        duration_millis: None,
    }
}

/// The message of an error writing a file where a folder is (`EISDIR` in Node's words): what a notice about it says.
pub(super) fn folder_write_error() -> String {
    let dir = temp("kk-eisdir-");
    fs::write(dir.path(), "").unwrap_err().to_string()
}

/// The filter of the window, `{ include, exclude, hideNonMatching }`.
pub(super) fn filter(include: &[&str], exclude: &[&str], hide_non_matching: bool) -> crate::model::tags::FilterState {
    crate::model::tags::FilterState {
        include: include.iter().map(|s| s.to_string()).collect(),
        exclude: exclude.iter().map(|s| s.to_string()).collect(),
        hide_non_matching,
    }
}

/// `{ type: "project", project: null, tree: null, error: null }`.
pub(super) fn no_project() -> AppEvent {
    AppEvent::Project {
        project: None,
        tree: None,
        error: None,
        subfolders: None,
    }
}

/// The replay of a window that loads while the project is open and the runner idle.
pub(super) fn idle_replay(t: &TestRunner, filter: crate::model::tags::FilterState, threads: u32) -> Vec<AppEvent> {
    vec![
        t.runner.current().unwrap(),
        AppEvent::Filter { filter },
        AppEvent::Threads { count: threads },
        AppEvent::Watch { on: false },
        t.last_event("environment"),
        t.last_event("history"),
        t.last_event("flaky"),
        AppEvent::Breakpoints {
            breakpoints: Vec::new(),
        },
        AppEvent::Opening { root: None },
        AppEvent::Busy { busy: false },
    ]
}
