//! `src/renderer/reduce.ts` and `store.ts`: what the window shows, built from the runner's `AppEvent`s.
//!
//! Electron Grace kept this state in the renderer's store, fed over IPC. GPUI Grace has one process: the `Grace`
//! entity owns an `AppState` and applies each batch of the runner's events with [`AppState::reduce`] (P2 plan,
//! Redesigns). The rules stay those of `reduce.ts`, test for test.
use std::collections::{HashMap, VecDeque};

use serde_json::Value;

use crate::model::env::EnvView;
use crate::model::events::{
    AppEvent, Breakpoint, BuildTool, DebugVar, Paused, Project, RecentEntry, RunOutcome, Step, Summary,
};
use crate::model::flaky::FlakyEnv;
use crate::model::history::HistoryEntry;
use crate::model::http::{HttpCall, HttpGroup};
use crate::model::node::{NodeKind, Status, TestNode};
use crate::model::tags::{FilterState, no_filter};

/// The console keeps this many lines; older ones are dropped.
pub const CONSOLE_LIMIT: usize = 10_000;

/// What a run reported about one test, beyond its status.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TestResult {
    pub duration_millis: Option<f64>,
    pub error: Option<String>,
    /// Where Karate's reason starts in `error` (#229 R2.1).
    pub reason_at: Option<usize>,
}

/// The last notice; `success` reports something done, not a problem; `build` offers the Build button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoticeView {
    pub text: String,
    pub success: bool,
    pub build: bool,
}

/// A call of a test: the test's key and the call's place in that test's list.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CallRef {
    pub key: String,
    pub index: usize,
}

/// The tab shown in the bottom panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Panel {
    #[default]
    Console,
    History,
    Http,
    Variables,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunView {
    pub active: bool,
    pub total: u32,
    pub done: u32,
    pub passed: u32,
    pub failed: u32,
    /// Tests the run left running when it was cancelled; not part of `done`.
    pub interrupted: u32,
    pub started_at: Option<f64>,
    pub outcome: Option<RunOutcome>,
    pub summary: Option<Summary>,
    pub report_available: bool,
    /// The run folder's name, the key of its entry in the history.
    pub run_id: Option<String>,
    /// The environment the run in progress, or the last run, used.
    pub env: Option<String>,
    /// A step debugger run.
    pub debug: bool,
    /// The debug run's pauses so far: the tree scrolls to the paused test on each.
    pub pauses: u32,
    /// The key the run started from: the tree's root for Run all and a rerun of the failed tests.
    pub selection: Option<String>,
    /// The run's tests that failed, in the order they did; the tree opens their features (#224 R3.3).
    pub failed_keys: Vec<String>,
}

/// The window's state, as `reduce.ts`'s `AppState`.
#[derive(Clone, Debug, PartialEq)]
pub struct AppState {
    pub project: Option<Project>,
    pub tree: Option<TestNode>,
    pub error: Option<String>,
    /// The folders the last `project` event offered (sub-folder offer R3).
    pub subfolders: Vec<String>,
    pub statuses: HashMap<String, Status>,
    pub results: HashMap<String, TestResult>,
    pub steps: HashMap<String, Vec<Step>>,
    /// Each test's HTTP calls, in run order; cleared exactly as `steps` is.
    pub calls: HashMap<String, Vec<HttpCall>>,
    /// The last `CONSOLE_LIMIT` lines. A `VecDeque`, so the oldest go without copying the rest.
    pub console: VecDeque<String>,
    /// The console lines dropped at `CONSOLE_LIMIT` or cleared by a run since the project opened: a line's number is
    /// this plus its index, and no two lines share one.
    pub console_dropped: u64,
    pub run: RunView,
    pub notice: Option<NoticeView>,
    pub selected: Option<String>,
    /// Counts the selections the tree must show: it opens the rows above the selected test and scrolls to it.
    pub reveals: u32,
    /// Counts the trees the runner sent: a view that builds on the tree builds again only when this or what else it
    /// reads changes, without comparing trees.
    pub trees: u64,
    /// The open project's tag filter, as the runner holds it.
    pub filter: FilterState,
    /// The open project's environment and run properties; `None` with no project.
    pub env: Option<EnvView>,
    /// How many `environment` events came for the open project: the box resets on each, even when the view is the one
    /// it held (a refused change re-sends it; `EnvPicker.tsx` reset on each new view object).
    pub env_generation: u64,
    /// The open project's thread count, and how many `threads` events came: the box resets on each, even when the
    /// count is the one it held (`reduce.ts` used a fresh object for that).
    pub threads: Option<ThreadsView>,
    /// Whether watch mode is on for the open project.
    pub watch: bool,
    /// The runner holds its one-at-a-time slot: discovering, or running.
    pub busy: bool,
    /// The open project's listed runs, newest first.
    pub history: Vec<HistoryEntry>,
    /// Proof per flaky test key of the tree shown.
    pub flaky: HashMap<String, Vec<FlakyEnv>>,
    /// The status groups switched on in the HTTP tab; none means All. Lasts for the session.
    pub http_groups: Vec<HttpGroup>,
    /// The call shown in the details pane in place of the node; `None` shows the node.
    pub selected_call: Option<CallRef>,
    pub panel: Panel,
    pub breakpoints: Vec<Breakpoint>,
    /// Where the debug run waits; `None` while it runs or with no debug run.
    pub paused: Option<Paused>,
    /// The last pause's variables, kept while the run goes on; `None` before a debug run's first pause.
    pub vars: Option<Vec<DebugVar>>,
    /// The folder the runner is opening.
    pub opening: Option<String>,
    /// The build tool the runner runs to list the project's libraries, or to build its Java code when `building`.
    pub listing: Option<BuildTool>,
    pub building: bool,
    /// The runner has said what is open or opening; until then the window cannot tell home from a folder opening.
    pub loaded: bool,
    /// The saved recent folders, newest first; `None` until the runner sends them.
    pub recent: Option<Vec<RecentEntry>>,
    /// The updater's view, release and progress (phase P5 types them).
    pub updates: Option<Value>,
    pub update_prompt: Option<Value>,
    pub update_progress: Option<Value>,
}

/// The thread count the box shows; `generation` grows with each `threads` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThreadsView {
    pub count: u32,
    pub generation: u64,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            project: None,
            tree: None,
            error: None,
            subfolders: Vec::new(),
            statuses: HashMap::new(),
            results: HashMap::new(),
            steps: HashMap::new(),
            calls: HashMap::new(),
            console: VecDeque::new(),
            console_dropped: 0,
            run: RunView::default(),
            notice: None,
            selected: None,
            reveals: 0,
            trees: 0,
            filter: no_filter(),
            env: None,
            env_generation: 0,
            threads: None,
            watch: false,
            busy: false,
            history: Vec::new(),
            flaky: HashMap::new(),
            http_groups: Vec::new(),
            selected_call: None,
            panel: Panel::Console,
            breakpoints: Vec::new(),
            paused: None,
            vars: None,
            opening: None,
            listing: None,
            building: false,
            loaded: false,
            recent: None,
            updates: None,
            update_prompt: None,
            update_progress: None,
        }
    }
}

/// Each node's kind and name by key.
fn by_key(n: Option<&TestNode>) -> HashMap<&str, (NodeKind, &str)> {
    fn walk<'a>(n: &'a TestNode, out: &mut HashMap<&'a str, (NodeKind, &'a str)>) {
        out.insert(&n.key, (n.kind, &n.name));
        for c in &n.children {
            walk(c, out);
        }
    }
    let mut out = HashMap::new();
    if let Some(n) = n {
        walk(n, &mut out);
    }
    out
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// What a project for another folder, or none, clears.
    fn reset_project_view(&mut self) {
        self.reset_results();
        self.filter = no_filter(); // the runner's `filter` event follows
        self.env = None; // and its `environment` event
        self.threads = None; // and its `threads` event
        self.history.clear(); // its `history` follows a project that opens
        self.flaky.clear(); // and its `flaky` follows the tree
        self.paused = None;
        self.vars = None;
        if self.panel == Panel::Variables {
            self.panel = Panel::Console;
        }
    }

    /// What a result from another environment or project must not pass for.
    fn reset_results(&mut self) {
        self.statuses.clear();
        self.results.clear();
        self.steps.clear();
        self.calls.clear();
        self.selected = None;
        self.selected_call = None;
        self.run = RunView::default();
    }

    /// Applies one event, as `reduce.ts`'s `reduce`.
    pub fn reduce(&mut self, e: &AppEvent) {
        match e {
            AppEvent::Project {
                project,
                tree,
                error,
                subfolders,
            } => {
                let other = match (project, &self.project) {
                    (Some(p), Some(open)) => p.root != open.root,
                    _ => true,
                };
                if other {
                    self.reset_project_view();
                    if project.is_none() {
                        // a closed project leaves no console behind
                        self.console.clear();
                        self.console_dropped = 0;
                    }
                }
                // A key only says where a test sits: inserting a scenario gives the next one its key.
                {
                    let before = by_key(self.tree.as_ref());
                    let after = by_key(tree.as_ref());
                    let same = |key: &str| match (before.get(key), after.get(key)) {
                        (Some(a), Some(b)) => a == b,
                        _ => false,
                    };
                    if self.selected.as_deref().is_some_and(|k| !same(k)) {
                        self.selected = None;
                    }
                    if self.selected_call.as_ref().is_some_and(|c| !same(&c.key)) {
                        self.selected_call = None;
                    }
                    self.statuses.retain(|k, _| same(k));
                    self.results.retain(|k, _| same(k));
                    self.steps.retain(|k, _| same(k));
                    self.calls.retain(|k, _| same(k));
                    self.flaky.retain(|k, _| same(k));
                }
                self.project = project.clone();
                self.tree = tree.clone();
                self.trees += 1;
                self.error = error.clone();
                self.subfolders = subfolders.clone().unwrap_or_default();
                self.notice = None;
                self.loaded = true;
            }
            AppEvent::RunStarted {
                selection,
                run_id,
                keys,
                total,
                started_at,
                env,
                debug,
            } => {
                self.run = RunView {
                    active: true,
                    total: *total,
                    started_at: Some(*started_at),
                    run_id: Some(run_id.clone()),
                    env: env.clone(),
                    debug: *debug,
                    selection: Some(selection.clone()),
                    ..RunView::default()
                };
                self.paused = None;
                self.vars = None;
                // the bottom tab stays as it is, so the HTTP tab can be watched filling during the run
                if self.selected_call.as_ref().is_some_and(|c| keys.contains(&c.key)) {
                    self.selected_call = None;
                }
                for key in keys {
                    self.statuses.insert(key.clone(), Status::Pending);
                    self.results.remove(key);
                    self.steps.remove(key);
                    self.calls.remove(key);
                }
                self.console_dropped += self.console.len() as u64;
                self.console.clear();
            }
            AppEvent::Scenarios { updates } => {
                for u in updates {
                    let before = self.statuses.get(&u.key).copied();
                    let is_final = matches!(before, Some(Status::Passed | Status::Failed));
                    self.statuses.insert(u.key.clone(), u.status);
                    let r = self.results.entry(u.key.clone()).or_default();
                    if u.duration_millis.is_some() {
                        r.duration_millis = u.duration_millis;
                    }
                    if u.error.is_some() {
                        r.error = u.error.clone();
                    }
                    if u.reason_at.is_some() {
                        r.reason_at = u.reason_at;
                    }
                    if u.status == Status::Interrupted && before != Some(Status::Interrupted) {
                        self.run.interrupted += 1;
                    }
                    if !is_final && matches!(u.status, Status::Passed | Status::Failed) {
                        self.run.done += 1;
                        if u.status == Status::Passed {
                            self.run.passed += 1;
                        } else {
                            self.run.failed += 1;
                            self.run.failed_keys.push(u.key.clone());
                        }
                    }
                }
            }
            AppEvent::Steps { key, steps } => {
                self.steps.insert(key.clone(), steps.clone());
            }
            AppEvent::Calls { key, calls } => {
                self.calls.insert(key.clone(), calls.clone());
            }
            AppEvent::Console { lines } => {
                self.console.extend(lines.iter().cloned());
                let over = self.console.len().saturating_sub(CONSOLE_LIMIT);
                self.console.drain(..over);
                self.console_dropped += over as u64;
            }
            AppEvent::RunFinished {
                outcome,
                summary,
                report_available,
                ..
            } => {
                self.run.active = false;
                self.run.outcome = Some(*outcome);
                self.run.summary = summary.clone();
                self.run.report_available = *report_available;
                self.busy = false; // nothing follows a run; avoids "Finding tests…" until the busy event arrives
                self.paused = None;
                if self.panel == Panel::Variables {
                    self.panel = Panel::Console; // the tab shows only during a debug run
                }
            }
            AppEvent::Breakpoints { breakpoints } => self.breakpoints = breakpoints.clone(),
            AppEvent::DebugPaused { paused } => {
                if self.vars.is_none() {
                    self.panel = Panel::Variables; // the first pause only: later the user's tab stays
                }
                self.paused = Some(paused.clone());
                self.vars = Some(paused.vars.clone());
                self.run.pauses += 1;
                if let Some(key) = &paused.key {
                    self.selected = Some(key.clone());
                    self.selected_call = None;
                }
            }
            AppEvent::DebugResumed => self.paused = None,
            AppEvent::Filter { filter } => self.filter = filter.clone(),
            AppEvent::History { runs } => {
                self.history = runs.clone();
                // the toolbar Report opens a run's report: it needs the run to still be listed with one
                let listed = runs
                    .iter()
                    .any(|r| Some(&r.id) == self.run.run_id.as_ref() && r.report_available);
                if !listed {
                    self.run.report_available = false;
                }
            }
            AppEvent::Flaky { tests } => {
                self.flaky = tests.iter().map(|t| (t.key.clone(), t.envs.clone())).collect();
            }
            AppEvent::Environment { view } => {
                if self.project.is_none() {
                    return; // the runner can still hold the previous project's view
                }
                // a result from another environment must not pass for the state of this one
                if self.env.as_ref().is_some_and(|e| e.env != view.env) {
                    self.reset_results();
                }
                self.env = Some(view.clone());
                self.env_generation += 1;
            }
            AppEvent::Threads { count } => {
                if self.project.is_none() {
                    return; // the runner can still hold the previous project's count
                }
                let generation = self.threads.map_or(0, |t| t.generation + 1);
                self.threads = Some(ThreadsView {
                    count: *count,
                    generation,
                });
            }
            AppEvent::Watch { on } => self.watch = *on,
            AppEvent::Busy { busy } => self.busy = *busy,
            AppEvent::Listing { tool, build } => {
                self.listing = *tool;
                self.building = *build;
            }
            AppEvent::Notice {
                message,
                success,
                build,
            } => {
                self.notice = Some(NoticeView {
                    text: message.clone(),
                    success: success.unwrap_or(false),
                    build: *build,
                });
            }
            AppEvent::Opening { root } => {
                self.opening = root.clone();
                self.loaded = true;
                if root.is_some() && self.project.is_none() {
                    self.error = None; // the last open's "Folder not found" is not this one's
                }
            }
            AppEvent::Recent { entries } => self.recent = Some(entries.clone()),
            AppEvent::Updates { view } => self.updates = Some(view.clone()),
            AppEvent::UpdatePrompt { release } => {
                self.update_prompt = (!release.is_null()).then(|| release.clone());
            }
            AppEvent::UpdateProgress { progress } => self.update_progress = progress.clone(),
        }
    }

    // `store.ts`'s actions.

    pub fn select(&mut self, key: &str) {
        self.selected = Some(key.to_string());
        self.selected_call = None; // a tree click gives the details pane back
    }

    /// Selects a test the tree may not show: the tree opens the rows above it and scrolls to it.
    pub fn reveal(&mut self, key: &str) {
        self.select(key);
        self.reveals += 1;
    }

    pub fn show_panel(&mut self, panel: Panel) {
        self.panel = panel;
        if panel != Panel::Http {
            self.selected_call = None;
        }
    }

    pub fn select_call(&mut self, call: CallRef) {
        self.selected_call = Some(call);
    }

    pub fn dismiss_prompt(&mut self) {
        self.update_prompt = None;
    }

    /// The home screen shows: no project is open or opening.
    pub fn home(&self) -> bool {
        self.project.is_none() && self.opening.is_none()
    }

    /// The notice to show: the last one, or on the home screen the open's error ("Folder not found" and the like
    /// arrive as an error with no project), as `App.tsx` shows it.
    pub fn shown_notice(&self) -> Option<NoticeView> {
        self.notice.clone().or_else(|| match (&self.error, self.home()) {
            (Some(text), true) => Some(NoticeView {
                text: text.clone(),
                success: false,
                build: false,
            }),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests;
