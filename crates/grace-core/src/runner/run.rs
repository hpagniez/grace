//! The run half of `runner.ts`: `runTarget`, `rerunGuarded`, `runProperties` and `runBody`, from the checks before
//! a run to its `run.json`.
use super::{Discovery, Inner, Props, SpawnPlan, at, join_notices, now_ms, text};
use crate::history::{self, RunDir, create_run_dir, prune_history, read_run_record, write_run_record};
use crate::karate::argfile::{write_arg_file, write_target_file};
use crate::karate::command::{OUT_OF_MEMORY, detect_layout, line_targets, out_of_memory, selection_targets};
use crate::karate::events::{JsonlTail, POLL};
use crate::karate::process::KarateProcess;
use crate::karate::run_state::{LeafResult, RunState};
use crate::karate::tree::{NodeRef, find_node, resolve_node, resolve_tests};
use crate::mask::Masker;
use crate::model::env::{PlainRow, effective_rows};
use crate::model::events::{AppEvent, RunOutcome};
use crate::model::history::{LeafKind, RunRecord, StoredSelection, StoredTest, TestStatusFinal};
use crate::model::tags::{FilterState, matches, matching_leaves, project_tags, prune_filter};
use crate::model::{NodeKind, TestNode};
use crate::settings::StoredValue;
use indexmap::IndexMap;
use std::fs;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Instant;

/// A node of the tree before the pre-run discovery, to find again in the new one: what `resolve_node` compares.
// Rust-native: owned, where the TypeScript held the old tree's node: the tree it came from is replaced by the
// discovery. Parity: `runs_a_node_that_moved_...` and the rerun tests (a run's stored selection).
pub(super) struct OldNode {
    key: String,
    kind: NodeKind,
    file: String,
    name: String,
}

impl From<&TestNode> for OldNode {
    fn from(n: &TestNode) -> Self {
        Self {
            key: n.key.clone(),
            kind: n.kind,
            file: n.file.clone(),
            name: n.name.clone(),
        }
    }
}

impl From<&StoredSelection> for OldNode {
    fn from(s: &StoredSelection) -> Self {
        Self {
            key: s.key.clone(),
            kind: s.kind,
            file: s.file.clone(),
            name: s.name.clone(),
        }
    }
}

impl OldNode {
    fn as_ref(&self) -> NodeRef<'_> {
        NodeRef {
            key: &self.key,
            kind: self.kind,
            file: &self.file,
            name: &self.name,
        }
    }
}

/// What a run selects: a tree node (a click or a Re-run), found again after the pre-run discovery or refused with
/// `gone`; or stored tests (Re-run failed, or a Re-run of one). Watch mode's saved features come with P4.
pub(super) enum RunTarget {
    Node {
        key: String,
        old: Option<OldNode>,
        gone: &'static str,
    },
    Tests {
        tests: Vec<StoredTest>,
        failed_of: Option<f64>,
    },
}

/// `runBody`'s `o`: what one run runs, and with what.
struct RunPlan<'t> {
    root: String,
    tree: &'t TestNode,
    leaves: Vec<&'t TestNode>,
    targets: Vec<String>,
    /// The targets go through the targets file, `@<file>` (any number fits).
    target_file: bool,
    filter: FilterState,
    threads: u32,
    env: Option<String>,
    props: Props,
    selection: Option<&'t TestNode>,
    failed_of: Option<f64>,
}

/// The tests of run.json: each selected leaf with the final status and duration its run gave it.
fn stored_tests(leaves: &[&TestNode], results: &IndexMap<String, LeafResult>) -> Vec<StoredTest> {
    leaves
        .iter()
        .map(|n| {
            let result = &results[&n.key];
            StoredTest {
                kind: if n.kind == NodeKind::Row {
                    LeafKind::Row
                } else {
                    LeafKind::Scenario
                },
                file: n.file.clone(),
                ref_id: n.ref_id.clone().unwrap_or_default(),
                name: n.name.clone(),
                line: n.line,
                key: n.key.clone(),
                status: result.status,
                duration_millis: result.duration_millis,
            }
        })
        .collect()
}

/// An exit code as Node gives it: Windows' is unsigned (`3221225786` for a crash, which Rust reads as a negative
/// `i32`), Linux's a small number.
fn js_exit_code(code: i32) -> i64 {
    if cfg!(windows) {
        i64::from(code.cast_unsigned())
    } else {
        i64::from(code)
    }
}

/// One run between its folder's creation and its end: what `runBody`'s closures shared.
struct Run<'r, 't> {
    inner: &'r Inner,
    plan: RunPlan<'t>,
    masker: Masker,
    project_dir: PathBuf,
    id: String,
    out_dir: PathBuf,
    /// The time of `runStarted`; the folder's if the run never starts.
    started_at: f64,
    /// The run reached its end events, so it gets a run.json.
    ended: bool,
    state: RunState,
}

impl Run<'_, '_> {
    /// Every event of a run goes out masked.
    fn emit(&self, mut e: AppEvent) {
        self.masker.mask_event(&mut e);
        self.inner.emit(e);
    }

    fn notice(&self, message: impl Into<String>) {
        self.emit(AppEvent::notice(message));
    }

    /// Sends the end events and returns `runFinished`'s outcome and exit code, for `store`. An exit code of None
    /// forces the outcome failed.
    fn end(&mut self, code: Option<i32>, cancelled: bool) -> (RunOutcome, Option<i32>) {
        self.ended = true;
        let report = self.out_dir.join("karate-summary.html");
        let mut done = (RunOutcome::Failed, None);
        for mut e in self.state.finish(code, cancelled, false) {
            if let AppEvent::RunFinished {
                outcome,
                exit_code,
                report_available,
                ..
            } = &mut e
            {
                let kept = (*outcome == RunOutcome::Finished && report.exists()).then(|| report.clone());
                *report_available = kept.is_some();
                self.inner.state().report = kept;
                done = (*outcome, *exit_code);
            }
            self.emit(e);
        }
        done
    }

    /// Writes run.json (R2), applies retention and sends `history`; the run lasted until `ended_at`. A failure is a
    /// notice: it must not replace the run's outcome.
    fn store(&self, (outcome, exit_code): (RunOutcome, Option<i32>), fingerprint: Option<String>, ended_at: f64) {
        let p = &self.plan;
        let record = RunRecord {
            version: 1,
            started_at: self.started_at,
            duration_millis: ended_at - self.started_at,
            env: p.env.clone(),
            threads: Some(p.threads),
            fingerprint,
            filter: p.filter.clone(),
            selection: p.selection.map(|n| StoredSelection {
                kind: n.kind,
                file: n.file.clone(),
                name: n.name.clone(),
                line: n.line,
                key: n.key.clone(),
            }),
            failed_of: p.failed_of,
            outcome,
            // a kill's code is not Karate's
            exit_code: if outcome == RunOutcome::Cancelled {
                None
            } else {
                exit_code.map(js_exit_code)
            },
            tests: stored_tests(&p.leaves, &self.state.results()),
            debug: false,
        };
        match write_run_record(&self.out_dir, &record) {
            Ok(()) => {
                prune_history(&self.project_dir);
                self.inner.send_history();
            }
            Err(e) => self.notice(format!(
                "The run could not be saved to the history: {}",
                at(&self.out_dir.join("run.json"))(e)
            )),
        }
    }

    /// Reads the events Karate wrote since the last read and sends what they change. A bad event line or malformed
    /// data is fatal: the run's status can no longer be trusted, so Java is killed.
    fn poll(&mut self, tail: &mut JsonlTail, fatal: &mut Option<String>, proc: &KarateProcess) {
        if fatal.is_some() {
            return;
        }
        let read = tail.read().map_err(|e| e.to_string());
        match read.and_then(|events| self.state.apply(&events).map_err(|e| e.to_string())) {
            Ok(events) => events.into_iter().for_each(|e| self.emit(e)),
            Err(message) => {
                *fatal = Some(message);
                proc.kill();
            }
        }
    }
}

impl Inner {
    /// "Run cancelled" when the operation was cancelled.
    fn run_cancelled(&self) -> bool {
        let cancelled = self.cancelled();
        if cancelled {
            self.notice("Run cancelled");
        }
        cancelled
    }

    /// The run's properties, from the saved lists: the effective rows of `env`, secrets decrypted. None, after a
    /// notice, when a secret cannot be read.
    pub(super) fn run_properties(&self, env: Option<&str>) -> Option<Props> {
        let properties = self.state().envs.properties.clone();
        let mut rows = Vec::new();
        let mut secrets = Vec::new();
        // The environment's own variables go last: one that replaced a shared property sits at that property's
        // place, and must still win over a shared variable differing only in case (R2.1).
        let mut late = Vec::new();
        for r in effective_rows(&properties.shared, &properties.by_env, env) {
            // the TypeScript's `Set` of the shared rows, by identity: the same name in the environment is another row
            let shared = properties.shared.iter().any(|s| ptr::eq(s, r));
            let to = if r.env_var && !shared { &mut late } else { &mut rows };
            let env_var = r.env_var.then_some(true);
            let value = match &r.value {
                StoredValue::Plain(value) => value.clone(),
                StoredValue::Secret(cipher) => {
                    let Some(value) = self.read_secret(cipher) else {
                        let scope = if shared {
                            "All environments"
                        } else {
                            env.unwrap_or_default()
                        };
                        self.notice(format!(
                            "Secret {} ({scope}) can't be read. Replace it in Properties.",
                            r.name
                        ));
                        return None;
                    };
                    secrets.push(value.clone());
                    value
                }
            };
            to.push(PlainRow {
                name: r.name.clone(),
                value,
                env_var,
            });
        }
        rows.extend(late);
        Some(Props { rows, secrets })
    }

    /// Re-run and Re-run failed (R5, R6): the record's secrets checked, its environment switched to, its filter shown
    /// and saved, then its selection or tests run with the filter as written.
    pub(super) fn rerun_guarded(&self, id: &str, failed_only: bool) -> io::Result<()> {
        let root = self.project().root;
        let Some(record) = read_run_record(&self.project_dir_of(&root), id) else {
            self.notice("This run is no longer in the history");
            return Ok(());
        };
        if self.run_properties(record.env.as_deref()).is_none() {
            return Ok(()); // before the switch: nothing changes
        }
        let switching = record.env != self.state().envs.env;
        if switching && !self.switch_env(record.env.clone(), record.filter.clone())? {
            return Ok(()); // cancelled
        }
        // As if chosen in the filter bar, without vanished tags (tag filter R5).
        let tree = self.state().tree.clone();
        self.store_filter(match &tree {
            Some(t) => prune_filter(record.filter.clone(), &project_tags(t)),
            None => record.filter.clone(),
        });
        self.send_filter();
        if switching && tree.is_none() {
            return Ok(()); // a parse error: the environment is switched, nothing runs
        }
        // A cancel after the switch's dry run stops run_target before its JVM, once the filter shown is saved.
        let RunRecord {
            selection,
            tests,
            failed_of,
            started_at,
            filter,
            ..
        } = record;
        let target = if failed_only {
            RunTarget::Tests {
                tests: tests
                    .into_iter()
                    .filter(|t| t.status == TestStatusFinal::Failed)
                    .collect(),
                failed_of: Some(started_at),
            }
        } else if let Some(selection) = &selection {
            RunTarget::Node {
                key: selection.key.clone(),
                old: Some(selection.into()),
                gone: "The tests of this run no longer exist",
            }
        } else {
            RunTarget::Tests { tests, failed_of }
        };
        self.run_target(target, filter)
    }

    /// The pre-run checks of a target after the pre-run discovery, then its run. `filter` is the one saved, not
    /// pruned: a vanished included tag shrinks the run, never widens it.
    pub(super) fn run_target(&self, target: RunTarget, filter: FilterState) -> io::Result<()> {
        // a cancel or dispose before the pre-run discovery, e.g. after a re-run's switch: start no JVM
        if self.run_cancelled() {
            return Ok(());
        }
        let env = self.state().envs.env.clone();
        // The rows depend on the environment and the saved lists only: refuse before any JVM starts.
        let Some(props) = self.run_properties(env.as_deref()) else {
            return Ok(());
        };
        let found = self.discover(env.as_deref())?;
        if self.run_cancelled() {
            return Ok(());
        }
        let Discovery::Found(Some(tree)) = found else {
            return Ok(());
        };
        let (root, threads) = {
            let s = self.state();
            (s.project.as_ref().expect("a project is open").root.clone(), s.threads)
        };
        let layout = detect_layout(Path::new(&root));
        let (bases, feature_roots) = (&layout.bases, &layout.feature_roots);
        let plan = |leaves, targets, target_file, selection, failed_of| RunPlan {
            root: root.clone(),
            tree: &tree,
            leaves,
            targets,
            target_file,
            filter: filter.clone(),
            threads,
            env: env.clone(),
            props,
            selection,
            failed_of,
        };

        let node = match target {
            RunTarget::Tests { tests, failed_of } => {
                // file:LINE targets ignore -t: the found tests run whatever their tags are now (R6)
                let resolved = resolve_tests(&tree, &tests);
                let (missing, dynamic) = (resolved.missing, resolved.dynamic);
                let mut said = Vec::new();
                if missing == 1 {
                    said.push("1 failed test no longer exists".to_string());
                }
                if missing > 1 {
                    said.push(format!("{missing} failed tests no longer exist"));
                }
                if dynamic > 0 {
                    let rows = if dynamic == 1 { "row" } else { "rows" };
                    said.push(format!(
                        "{dynamic} failed {rows} of a dynamic Examples table can't be run alone. Run its outline."
                    ));
                }
                if resolved.found.is_empty() {
                    said.push("Nothing to run".to_string());
                }
                // one notice: the window shows only the last
                if !said.is_empty() {
                    self.notice(join_notices(&said));
                }
                if resolved.found.is_empty() {
                    return Ok(());
                }
                let targets = line_targets(resolved.found.iter().copied(), bases);
                return self.run_body(plan(resolved.found, targets, true, None, failed_of));
            }
            RunTarget::Node { key, old, gone } => {
                let node = match &old {
                    Some(old) => resolve_node(&tree, old.as_ref()),
                    None => find_node(&tree, &key),
                };
                let Some(node) = node else {
                    self.notice(gone);
                    return Ok(());
                };
                node
            }
        };
        if node.kind == NodeKind::Row && node.line.is_none() {
            self.notice("Run the outline to run its rows");
            return Ok(());
        }
        if matches!(node.kind, NodeKind::Scenario | NodeKind::Row) && !matches(&node.tags, &filter) {
            self.notice("This test no longer matches the filter");
            return Ok(());
        }
        let leaves = matching_leaves(node, &filter);
        if leaves.is_empty() {
            self.notice("Nothing to run");
            return Ok(());
        }
        let Some(targets) = selection_targets(node, bases, feature_roots, &filter) else {
            // a matching row of a partly matching outline has no line (dynamic Examples)
            self.notice("Clear the filter to run this outline");
            return Ok(());
        };
        self.run_body(plan(leaves, targets, false, Some(node), None))
    }

    /// Runs the plan's leaves with its targets and stores the run in its folder. The argument and targets files are
    /// deleted whatever happens, and the folder of a run that never reached its end events.
    fn run_body(&self, plan: RunPlan<'_>) -> io::Result<()> {
        // A file saved while Java runs could be what made a test flip: the run keeps its fingerprint only if the walk
        // after the run finds what this one does (F15).
        let before = (self.walk)(Path::new(&plan.root));
        if self.cancelled() {
            self.notice("Run cancelled");
            return Ok(());
        }
        self.state().report = None;
        let project_dir = self.project_dir_of(&plan.root);
        let (vars, sys_props): (Vec<PlainRow>, Vec<PlainRow>) =
            plan.props.rows.iter().cloned().partition(|r| r.env_var == Some(true));
        let arg_file = (!sys_props.is_empty()).then(|| project_dir.join("run-args.txt"));
        let target_file = plan.target_file.then(|| project_dir.join("run-targets.txt"));
        let masker = Masker::new(&plan.props.secrets);
        let created = chrono::Local::now();
        // also creates the project folder the argument file goes in
        let RunDir { id, dir: out_dir } = create_run_dir(&project_dir, &created).map_err(at(&project_dir))?;
        let state = RunState::new(plan.tree, plan.leaves.iter().map(|n| n.key.clone()).collect());
        let mut run = Run {
            inner: self,
            plan,
            masker,
            project_dir,
            id,
            out_dir,
            started_at: created.timestamp_millis() as f64,
            ended: false,
            state,
        };
        let result = self.run_java(
            &mut run,
            before,
            arg_file.as_deref(),
            target_file.as_deref(),
            &sys_props,
            &vars,
        );
        // An antivirus or an indexer can hold it a moment. A failure must not replace the run's outcome.
        for file in [&arg_file, &target_file].into_iter().flatten() {
            if let Err(e) = history::remove_file_retrying(file, 5) {
                run.notice(format!(
                    "The run's argument file could not be deleted; the app deletes it when it next starts. {}",
                    at(file)(e)
                ));
            }
        }
        if !run.ended {
            // An error ended the run before its end events: nothing to list. Best effort, as the next start deletes
            // it too (`history::remove_leftover_runs`).
            let _ = history::remove_retrying(&run.out_dir, 5);
        }
        result
    }

    /// `runBody`'s `try`: the argument files written, Java started and followed to its end, the run stored.
    // Rust-native: the run's thread waits on a channel for Java's lines, and reads the events file at each 100 ms
    // deadline in between, where Electron Grace had a readline callback and a `setInterval`; the one thread sends
    // every event of the run, in order, and nothing wakes once Java has ended. Parity: every test of `runs` and
    // `history` (the events and their order), `stores_the_run_after_the_last_poll`.
    fn run_java(
        &self,
        run: &mut Run<'_, '_>,
        before: Option<String>,
        arg_file: Option<&Path>,
        target_file: Option<&Path>,
        sys_props: &[PlainRow],
        vars: &[PlainRow],
    ) -> io::Result<()> {
        if let Some(file) = arg_file {
            write_arg_file(file, sys_props).map_err(at(file))?;
        }
        if let Some(file) = target_file {
            // if it cannot be written, nothing runs
            write_target_file(file, &run.plan.targets).map_err(at(file))?;
        }
        let root = run.plan.root.clone();
        let layout = detect_layout(Path::new(&root));
        let targets = match target_file {
            Some(file) => vec![format!("@{}", text(file))],
            None => run.plan.targets.clone(),
        };
        let (lines_tx, lines) = mpsc::channel::<Vec<String>>();
        let spawn = SpawnPlan {
            root: &root,
            layout: &layout,
            out_dir: &run.out_dir,
            dry_run: false,
            targets: &targets,
            env: run.plan.env.as_deref(),
            arg_file,
            filter: &run.plan.filter,
            threads: run.plan.threads,
            vars,
        };
        let Some(proc) = self.spawn(spawn, move |l| {
            let _ = lines_tx.send(l);
        }) else {
            // Rust-native: a cancel that came after the walk's check, before Java started: none starts, and the
            // folder goes as an ended-early run's does. Parity: `cancel_during_the_walk_starts_no_java`.
            self.notice("Run cancelled");
            return Ok(());
        };
        if let Some(pid) = proc.pid()
            && let Err(e) = fs::write(&self.pid_file, pid.to_string())
        {
            // without the pid file a crash of the app would leave this JVM running: do not run
            proc.kill();
            self.clear_proc();
            let done = run.end(None, false);
            run.store(done, None, now_ms());
            run.notice(at(&self.pid_file)(e).to_string());
            return Ok(());
        }
        run.started_at = now_ms();
        run.emit(AppEvent::RunStarted {
            // stored tests can span the tree
            selection: run.plan.selection.unwrap_or(run.plan.tree).key.clone(),
            run_id: run.id.clone(),
            keys: run.state.keys.clone(),
            total: u32::try_from(run.state.keys.len()).unwrap_or(u32::MAX),
            started_at: run.started_at,
            env: run.plan.env.clone(),
            debug: false,
        });

        let mut tail = JsonlTail::new(run.out_dir.join("karate-json").join("karate-events.jsonl"));
        let mut fatal: Option<String> = None;
        let mut oom = false; // Java printed an OutOfMemoryError
        let mut next = Instant::now() + POLL;
        loop {
            let now = Instant::now();
            if now >= next {
                run.poll(&mut tail, &mut fatal, &proc);
                next = now + POLL;
            }
            match lines.recv_timeout(next.saturating_duration_since(now)) {
                Ok(l) => {
                    oom |= out_of_memory(&l);
                    run.emit(AppEvent::Console { lines: l });
                }
                Err(RecvTimeoutError::Timeout) => {}
                // Java's output has ended, its last line handed over, and it has exited
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        let code = proc.wait();
        let exited_at = now_ms(); // the run's end: the walk below is not part of its duration
        run.poll(&mut tail, &mut fatal, &proc);
        self.clear_proc();
        let _ = fs::remove_file(&self.pid_file);

        // Taken before the walk: a Cancel clicked during it must not turn a finished run into a cancelled one.
        let cancelled = self.cancelled() && fatal.is_none();
        // The walk comes before the end events: the window shows idle at `runFinished`, and a run is refused while we
        // hold busy. A dispose is quitting the app, and a first walk that gave None leaves nothing to compare: no
        // second walk. A walk that panics (a test seam only) reads as None.
        let after = if self.disposing() || before.is_none() {
            None
        } else {
            catch_unwind(AssertUnwindSafe(|| (self.walk)(Path::new(&root)))).unwrap_or(None)
        };
        let done = run.end(if fatal.is_some() { None } else { code }, cancelled);
        run.store(done, if before == after { before } else { None }, exited_at);
        if let Some(fatal) = fatal {
            run.notice(fatal);
        } else if oom {
            run.notice(OUT_OF_MEMORY);
        }
        Ok(())
    }
}
