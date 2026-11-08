//! A throwaway GPUI prototype of Grace's main screen (#152, #270): open a Karate project, list its tests from a dry run,
//! filter them by text and tag, pick an environment and a thread count, Run all or one node with live progress, cancel
//! after a confirm dialog, the selected test's details, and Console, History and HTTP tabs. Built on GPUI Kit with
//! Grace's look (IBM Plex, Lucide, the two themes). Not part of the build. See ../README.md.
mod a11y;
mod karate;
mod look;
mod properties;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use gpui::{
    App, AppContext, Bounds, Context, Entity, FontWeight, Hsla, KeyBinding, PathPromptOptions, Role, ScrollStrategy,
    SharedString, Subscription, Task, UniformListScrollHandle, Window, WindowBounds, WindowOptions, actions, div,
    prelude::*, px, size, uniform_list,
};
use gpui_base::{SelectableText, TextSelectionHandle};
use gpui_component::{
    Disableable, Icon, IndexPath, Root, Sizable, WindowExt,
    button::{Button, ButtonVariants},
    input::{Input, InputEvent, InputState, NumberInput, NumberInputEvent, StepAction},
    select::{Select, SelectEvent, SelectState},
    tab::{Tab, TabBar},
};
use gpui_kit_assets::IconName;
use karate::{Call, Kind, Msg, Status, Tree, Vendor};
use look::{MONO, Palette, palette};

actions!(grace, [RunAll, Cancel, Open]);

/// `GRACE_PROBE` prints `PROBE <name> <unix ms>` to stderr for the measure script; nothing else reads it.
static PROBING: AtomicBool = AtomicBool::new(false);
/// A wheel event came since the last frame the probe counted. GPUI draws on demand, so without it the probe's gaps
/// include the wait before the first notch and the run's 100 ms redraws after the last one, as if they were slow frames.
static WHEEL_TURNED: AtomicBool = AtomicBool::new(false);

fn probe(name: &str) {
    if std::env::var_os("GRACE_PROBE").is_some() {
        let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        eprintln!("PROBE {name} {ms}");
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    NoProject,
    Discovering,
    Ready,
    Running,
}

/// One finished run, for the History tab.
struct Record {
    when: SharedString,
    outcome: &'static str,
    passed: usize,
    failed: usize,
    total: usize,
    secs: f32,
}

type EnvList = Vec<SharedString>;

struct Grace {
    vendor: Vendor,
    root: Option<PathBuf>,
    tree: Option<Tree>,
    /// Indexes of the rows shown, in order: recomputed when a node opens or closes or a filter changes.
    visible: Vec<usize>,
    selected: Option<usize>,
    threads: u32,
    phase: Phase,
    console: Vec<SharedString>,
    done: usize,
    passed: usize,
    failed: usize,
    run_total: usize,
    run_started: Option<Instant>,
    run_secs: f32,
    cancel: Option<Arc<AtomicBool>>,
    scroll: UniformListScrollHandle,
    data: PathBuf,
    task: Option<Task<()>>,
    first_frame: bool,
    tree_probed: bool,
    /// With GRACE_PROBE: when the tree's rows were laid out, once per frame that draws the tree (the frame probe).
    frames: Vec<Instant>,
    /// What a screen reader announces: set when a run starts and ends (a11y.rs).
    announcement: SharedString,

    filter: Entity<InputState>,
    filter_text: String,
    tags: Vec<SharedString>,
    tags_on: Vec<SharedString>,
    envs: Entity<SelectState<EnvList>>,
    env: Option<String>,
    threads_input: Entity<InputState>,
    tab: usize,
    history: Vec<Record>,
    http: Vec<Call>,
    console_scroll: UniformListScrollHandle,
    console_text: TextSelectionHandle,
    _subs: Vec<Subscription>,
}

const CONSOLE_LINES: usize = 400;
const DEFAULT_ENV: &str = "(default)";

impl Grace {
    fn new(root: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let data = std::env::var_os("GRACE_USER_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("grace-gpui"));
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter tests"));
        let envs = cx.new(|cx| SelectState::new(vec![SharedString::from(DEFAULT_ENV)], Some(IndexPath::new(0)), window, cx));
        let threads_input = cx.new(|cx| InputState::new(window, cx).default_value("1"));
        let subs = vec![
            cx.subscribe_in(&filter, window, |g, input, e: &InputEvent, _, cx| {
                if let InputEvent::Change = e {
                    g.filter_text = input.read(cx).value().to_lowercase();
                    g.relist();
                    cx.notify();
                }
            }),
            cx.subscribe_in(&envs, window, |g, _, e: &SelectEvent<EnvList>, _, cx| {
                let SelectEvent::Confirm(v) = e;
                g.env = v.as_ref().filter(|v| v.as_ref() != DEFAULT_ENV).map(|v| v.to_string());
                cx.notify();
            }),
            cx.subscribe_in(&threads_input, window, |g, input, e: &NumberInputEvent, window, cx| {
                let NumberInputEvent::Step(step) = e;
                g.threads = match step {
                    StepAction::Increment => (g.threads + 1).min(16),
                    StepAction::Decrement => g.threads.saturating_sub(1).max(1),
                };
                let v = g.threads.to_string();
                input.update(cx, |i, cx| i.set_value(v, window, cx));
            }),
            cx.subscribe_in(&threads_input, window, |g, input, e: &InputEvent, _, cx| {
                if let InputEvent::Change = e
                    && let Ok(n) = input.read(cx).value().parse::<u32>() {
                        g.threads = n.clamp(1, 16);
                    }
            }),
        ];
        let console_text = TextSelectionHandle::new("", cx);
        let mut g = Grace {
            vendor: Vendor::find(),
            root: None,
            tree: None,
            visible: vec![],
            selected: None,
            threads: 1,
            phase: Phase::NoProject,
            console: vec![],
            done: 0,
            passed: 0,
            failed: 0,
            run_total: 0,
            run_started: None,
            run_secs: 0.0,
            cancel: None,
            scroll: UniformListScrollHandle::new(),
            data,
            task: None,
            first_frame: false,
            tree_probed: false,
            frames: vec![],
            announcement: SharedString::default(),
            filter,
            filter_text: String::new(),
            tags: vec![],
            tags_on: vec![],
            envs,
            env: None,
            threads_input,
            tab: 0,
            history: vec![],
            http: vec![],
            console_scroll: UniformListScrollHandle::new(),
            console_text,
            _subs: subs,
        };
        if let Some(r) = root {
            g.open(r, window, cx);
        }
        g
    }

    fn log(&mut self, line: impl Into<SharedString>) {
        self.console.push(line.into());
        if self.console.len() > CONSOLE_LINES {
            let cut = self.console.len() - CONSOLE_LINES;
            self.console.drain(..cut);
        }
    }

    /// The console follows its end unless the person scrolled up (console R2 in Grace).
    fn follow_console(&self, was_at_end: bool) {
        if was_at_end && !self.console.is_empty() {
            self.console_scroll.scroll_to_item(self.console.len() - 1, ScrollStrategy::Bottom);
        }
    }

    fn console_at_end(&self) -> bool {
        self.console_scroll.is_scrolled_to_end().unwrap_or(true)
    }

    fn out_dir(&self, what: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        self.data.join("runs").join(format!("{what}-{stamp}"))
    }

    /// Discovery: a dry run, then the tree from its events (runner.ts `open`).
    fn open(&mut self, root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.root = Some(root.clone());
        self.tree = None;
        self.visible.clear();
        self.selected = None;
        self.tags.clear();
        self.tags_on.clear();
        self.http.clear();
        self.history.clear();
        self.phase = Phase::Discovering;
        self.console.clear();
        let mut envs: EnvList = vec![DEFAULT_ENV.into()];
        envs.extend(karate::environments(&root).into_iter().map(SharedString::from));
        self.env = None;
        self.envs.update(cx, |s, cx| {
            s.set_items(envs, window, cx);
            s.set_selected_index(Some(IndexPath::new(0)), window, cx);
        });
        self.log(format!("project  {}", root.display()));
        self.log("Finding tests…");
        let out = self.out_dir("dryrun");
        let args = karate::args(&self.vendor.jar, &root, &out, true, 1, None, &[root.display().to_string()]);
        let child = karate::spawn(&self.vendor, &args, &root);
        cx.notify();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        match child {
            Ok(child) => karate::watch(child, out.clone(), Arc::new(AtomicBool::new(false)), tx),
            Err(e) => {
                self.log(format!("Could not start Java: {e}"));
                self.phase = Phase::NoProject;
                return;
            }
        }
        self.task = Some(cx.spawn(async move |this, cx| {
            let mut code = None;
            while let Some(m) = rx.next().await {
                if let Msg::Done { code: c, .. } = m {
                    code = c;
                    break;
                }
            }
            let tree = cx
                .background_executor()
                .spawn(async move { karate::build_tree(&karate::events_file(&out), &name) })
                .await;
            let _ = this.update(cx, |g, cx| {
                match tree {
                    Ok(t) => {
                        g.log(format!("tests    {} scenarios", t.tests));
                        if t.tests == 0 {
                            g.log(format!("No tests found (Karate exited {code:?})."));
                        }
                        let mut tags: Vec<SharedString> =
                            t.nodes.iter().flat_map(|n| n.tags.iter().cloned()).map(SharedString::from).collect();
                        tags.sort();
                        tags.dedup();
                        g.tags = tags;
                        g.tree = Some(t);
                        g.relist();
                        g.log("Ready. Press Run all, or ▶ on a test.");
                        g.phase = Phase::Ready;
                    }
                    Err(e) => {
                        g.log(format!("Could not list the tests (exit {code:?}): {e}"));
                        g.phase = Phase::NoProject;
                    }
                }
                cx.notify();
            });
        }));
    }

    fn filtering(&self) -> bool {
        !self.filter_text.is_empty() || !self.tags_on.is_empty()
    }

    /// A test matches when its name or title, or one of its folders' or feature's, holds the filter text and it has one of the chosen tags (its own or
    /// inherited from its feature, as in Karate).
    fn matches(&self, i: usize, inherited: &[String], named: bool, keep: &mut Vec<bool>) -> bool {
        let t = self.tree.as_ref().unwrap();
        let n = &t.nodes[i];
        let mut tags = inherited.to_vec();
        tags.extend(n.tags.iter().cloned());
        // A folder or feature whose name matches shows every test below it.
        let named = named
            || self.filter_text.is_empty()
            || n.name.to_lowercase().contains(&self.filter_text)
            || n.title.to_lowercase().contains(&self.filter_text);
        let mut any = false;
        for &c in &n.children {
            any |= self.matches(c, &tags, named, keep);
        }
        if n.children.is_empty() {
            let text = named;
            let tag = self.tags_on.is_empty() || tags.iter().any(|t| self.tags_on.iter().any(|on| on.as_ref() == t));
            any = text && tag;
        }
        keep[i] = any;
        any
    }

    fn relist(&mut self) {
        self.visible.clear();
        let Some(t) = &self.tree else { return };
        let filtering = self.filtering();
        let mut keep = vec![true; t.nodes.len()];
        if filtering {
            self.matches(0, &[], false, &mut keep);
        }
        let t = self.tree.as_ref().unwrap();
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            if !keep[i] {
                continue;
            }
            self.visible.push(i);
            // A filter opens every folder that holds a match.
            if t.nodes[i].expanded || filtering {
                stack.extend(t.nodes[i].children.iter().rev());
            }
        }
    }

    fn toggle(&mut self, i: usize, cx: &mut Context<Self>) {
        if let Some(t) = &mut self.tree
            && !t.nodes[i].children.is_empty() {
                t.nodes[i].expanded = !t.nodes[i].expanded;
                self.relist();
                cx.notify();
            }
    }

    fn toggle_tag(&mut self, tag: SharedString, cx: &mut Context<Self>) {
        if let Some(p) = self.tags_on.iter().position(|t| *t == tag) {
            self.tags_on.remove(p);
        } else {
            self.tags_on.push(tag);
        }
        self.relist();
        cx.notify();
    }

    /// The tests under `i`, and the target Karate gets for it (a folder, a feature, or `file:line`).
    fn leaves(&self, i: usize, out: &mut Vec<usize>) {
        let t = self.tree.as_ref().unwrap();
        let n = &t.nodes[i];
        if matches!(n.kind, Kind::Scenario | Kind::Row) {
            out.push(i);
        }
        for &c in &n.children {
            self.leaves(c, out);
        }
    }

    fn target(&self, i: usize) -> Vec<String> {
        let t = self.tree.as_ref().unwrap();
        let root = self.root.as_ref().unwrap();
        let n = &t.nodes[i];
        match n.kind {
            Kind::Folder if i == 0 => vec![root.display().to_string()],
            Kind::Folder | Kind::Feature => vec![root.join(&n.file).display().to_string()],
            Kind::Scenario | Kind::Outline => {
                vec![format!("{}:{}", root.join(&n.file).display(), n.line.unwrap_or(0))]
            }
            Kind::Row => {
                // A row runs its outline's line: Karate has no target for one row.
                let parent = t.nodes.iter().position(|p| p.children.contains(&i)).unwrap();
                self.target(parent)
            }
        }
    }

    fn run(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.phase != Phase::Ready {
            return;
        }
        let root = self.root.clone().unwrap();
        let mut tests = vec![];
        self.leaves(i, &mut tests);
        let mut targets = self.target(i);
        // The tag bar narrows the run the Karate way, with `-t @a,@b` (OR); the text filter only narrows the view.
        if !self.tags_on.is_empty() {
            let mut keep = vec![true; self.tree.as_ref().unwrap().nodes.len()];
            let saved = std::mem::take(&mut self.filter_text);
            self.matches(0, &[], false, &mut keep);
            self.filter_text = saved;
            tests.retain(|&l| keep[l]);
            let tags: Vec<&str> = self.tags_on.iter().map(|t| t.as_ref()).collect();
            targets.splice(0..0, ["-t".to_string(), tags.join(",")]);
        }
        let t = self.tree.as_mut().unwrap();
        for n in t.nodes.iter_mut() {
            n.status = Status::NotRun;
            n.error = None;
            n.millis = None;
        }
        for &l in &tests {
            t.nodes[l].status = Status::Queued;
        }
        self.run_total = tests.len();
        self.done = 0;
        self.passed = 0;
        self.failed = 0;
        self.http.clear();
        self.run_started = Some(Instant::now());
        self.run_secs = 0.0;
        self.phase = Phase::Running;
        self.announcement = format!("Running {} tests", tests.len()).into();
        let at_end = self.console_at_end();
        let env = self.env.clone();
        self.log(format!(
            "Running {} tests on {} thread(s){}…",
            tests.len(),
            self.threads,
            env.as_ref().map(|e| format!(", environment {e}")).unwrap_or_default()
        ));
        self.follow_console(at_end);
        let out = self.out_dir("run");
        let args = karate::args(&self.vendor.jar, &root, &out, false, self.threads, env.as_deref(), &targets);
        let child = match karate::spawn(&self.vendor, &args, &root) {
            Ok(c) => c,
            Err(e) => {
                self.log(format!("Could not start Java: {e}"));
                self.phase = Phase::Ready;
                return;
            }
        };
        probe("run_started");
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(cancel.clone());
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        karate::watch(child, out, cancel, tx);
        cx.notify();
        self.task = Some(cx.spawn(async move |this, cx| {
            // At most one redraw per 100 ms: wait for a message, let the rest of the batch arrive, then apply all.
            while let Some(first) = rx.next().await {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                let mut batch = vec![first];
                while let Ok(Some(m)) = rx.try_recv().map(Some) {
                    batch.push(m);
                }
                let finished = this
                    .update(cx, |g, cx| {
                        let end = g.apply(batch);
                        cx.notify();
                        end
                    })
                    .unwrap_or(true);
                if finished {
                    break;
                }
            }
        }));
    }

    /// Applies a batch of run messages; true once the run is over.
    fn apply(&mut self, batch: Vec<Msg>) -> bool {
        let at_end = self.console_at_end();
        let mut over = false;
        for m in batch {
            match m {
                Msg::Line(l) => self.log(l),
                Msg::Http(c) => self.http.push(c),
                Msg::Started { file, ref_id } => {
                    if let Some(t) = &mut self.tree
                        && let Some(&i) = t.by_ref.get(&(file, ref_id)) {
                            t.nodes[i].status = Status::Running;
                        }
                }
                Msg::Finished { file, ref_id, status, millis, error } => {
                    if let Some(t) = &mut self.tree
                        && let Some(&i) = t.by_ref.get(&(file, ref_id)) {
                            let n = &mut t.nodes[i];
                            n.status = status;
                            n.millis = Some(millis);
                            n.error = error;
                            self.done += 1;
                            match status {
                                Status::Passed => self.passed += 1,
                                Status::Failed => self.failed += 1,
                                _ => {}
                            }
                        }
                }
                Msg::Done { code, cancelled } => {
                    over = true;
                    self.phase = Phase::Ready;
                    self.cancel = None;
                    let outcome = if cancelled {
                        "cancelled"
                    } else if code == Some(0) {
                        "passed"
                    } else if code == Some(1) {
                        "failed"
                    } else {
                        "error"
                    };
                    if let Some(t) = &mut self.tree {
                        for n in t.nodes.iter_mut() {
                            if matches!(n.status, Status::Queued | Status::Running) {
                                n.status = Status::NotRun;
                            }
                        }
                    }
                    self.log(format!("Run {outcome}: {} passed, {} failed (exit {code:?}).", self.passed, self.failed));
                    self.announcement = format!("Run {outcome}: {} passed, {} failed", self.passed, self.failed).into();
                    let secs = self.run_started.map(|s| s.elapsed().as_secs_f32()).unwrap_or(0.0);
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    self.history.insert(
                        0,
                        Record {
                            when: format!("{:02}:{:02}:{:02} UTC", now / 3600 % 24, now / 60 % 60, now % 60).into(),
                            outcome,
                            passed: self.passed,
                            failed: self.failed,
                            total: self.run_total,
                            secs,
                        },
                    );
                    probe("run_finished");
                }
            }
        }
        if let Some(s) = self.run_started {
            self.run_secs = s.elapsed().as_secs_f32();
        }
        self.roll_up();
        self.follow_console(at_end);
        over
    }

    /// A folder, feature or outline shows the worst status below it.
    fn roll_up(&mut self) {
        let Some(t) = &mut self.tree else { return };
        fn up(nodes: &mut Vec<karate::Node>, i: usize) -> Status {
            if nodes[i].children.is_empty() {
                return nodes[i].status;
            }
            let kids = nodes[i].children.clone();
            let all: Vec<Status> = kids.into_iter().map(|c| up(nodes, c)).collect();
            let s = if all.contains(&Status::Running) {
                Status::Running
            } else if all.contains(&Status::Failed) {
                Status::Failed
            } else if all.contains(&Status::Queued) {
                Status::Queued
            } else if all.contains(&Status::Passed) {
                Status::Passed
            } else {
                all.first().copied().unwrap_or(Status::NotRun)
            };
            nodes[i].status = s;
            s
        }
        up(&mut t.nodes, 0);
    }

    fn cancel_run(&mut self, cx: &mut Context<Self>) {
        if let Some(c) = &self.cancel {
            c.store(true, Ordering::Relaxed);
            let at_end = self.console_at_end();
            self.log("Cancelling…");
            self.follow_console(at_end);
            cx.notify();
        }
    }

    /// The confirm dialog: a cancelled run cannot be resumed.
    fn confirm_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.cancel.is_none() {
            return;
        }
        let this = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |alert, _, _| {
            let this = this.clone();
            alert
                .confirm()
                .title("Cancel the run?")
                .description("Karate stops now. Tests that have not finished are marked not run.")
                .ok_text("Cancel run")
                .cancel_text("Keep running")
                .on_ok(move |_, _, cx| {
                    let _ = this.update(cx, |g, cx| g.cancel_run(cx));
                    true
                })
        });
    }

    fn open_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let envs = self.root.as_ref().map(|r| karate::environments(r)).unwrap_or_default();
        let form = cx.new(|cx| properties::PropertiesForm::new(envs, window, cx));
        window.open_dialog(cx, move |dialog, _, _| {
            dialog.title("Properties").width(px(640.)).child(form.clone())
        });
    }

    fn prompt_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.phase == Phase::Running || self.phase == Phase::Discovering {
            return;
        }
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open a Karate project".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(mut p))) = paths.await
                && let Some(dir) = p.pop() {
                    let _ = this.update_in(cx, |g, window, cx| g.open(dir, window, cx));
                }
        })
        .detach();
    }
}

fn status_icon(s: Status, p: &Palette) -> (IconName, Hsla) {
    match s {
        Status::NotRun => (IconName::Circle, p.muted),
        Status::Queued => (IconName::CircleDashed, p.muted),
        Status::Running => (IconName::LoaderCircle, p.running),
        Status::Passed => (IconName::CircleCheck, p.passed),
        Status::Failed => (IconName::CircleX, p.failed),
        Status::Skipped => (IconName::CircleSlash, p.skipped),
    }
}

fn status_word(s: Status) -> &'static str {
    match s {
        Status::NotRun => "Not run",
        Status::Queued => "Queued",
        Status::Running => "Running",
        Status::Passed => "Passed",
        Status::Failed => "Failed",
        Status::Skipped => "Skipped",
    }
}

fn chip(text: impl Into<SharedString>, p: &Palette) -> gpui::Div {
    div()
        .px_1p5()
        .h(px(18.))
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(p.line)
        .text_xs()
        .text_color(p.muted)
        .child(text.into())
}

impl Grace {
    fn toolbar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let ready = self.phase == Phase::Ready;
        let running = self.phase == Phase::Running;
        let tests = self.tree.as_ref().map(|t| t.tests).unwrap_or(0);
        let project = self
            .root
            .as_ref()
            .and_then(|r| r.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "No project".into());
        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .h(px(48.))
            .bg(p.surface)
            .border_b_1()
            .border_color(p.line)
            .child(div().font_weight(FontWeight::SEMIBOLD).text_base().child("Grace"))
            .child(div().text_color(p.muted).child(project))
            .child(div().w(px(220.)).child(Input::new(&self.filter).small().prefix(Icon::new(IconName::Search).small()).cleanable(true).aria_label("Filter tests")))
            .child(div().flex_1())
            .child(div().w(px(150.)).child(Select::new(&self.envs).small().icon(Icon::new(IconName::Globe)).accessibility_label("Environment").disabled(!ready)))
            .child(div().text_sm().text_color(p.muted).child("Threads"))
            .child(div().w(px(96.)).child(NumberInput::new(&self.threads_input).small().disabled(!ready)))
            .child(
                Button::new("properties")
                    .ghost()
                    .small()
                    .icon(Icon::new(IconName::Settings2))
                    .tooltip("Properties")
                    .disabled(self.root.is_none())
                    .on_click(cx.listener(|g, _, window, cx| g.open_properties(window, cx))),
            )
            .child(
                Button::new("open")
                    .small()
                    .icon(Icon::new(IconName::FolderOpen))
                    .label("Open…")
                    .disabled(running)
                    .on_click(cx.listener(|g, _, window, cx| g.prompt_open(window, cx))),
            )
            .child(
                Button::new("cancel")
                    .small()
                    .icon(Icon::new(IconName::Square))
                    .label("Cancel")
                    .disabled(!running)
                    .on_click(cx.listener(|g, _, window, cx| g.confirm_cancel(window, cx))),
            )
            .child(
                div().debug_selector(|| "run-all".into()).child(
                    Button::new("run-all")
                        .primary()
                        .small()
                        .icon(Icon::new(IconName::Play))
                        .label(if self.tags_on.is_empty() { "Run all" } else { "Run tagged" })
                        .disabled(!(ready && tests > 0))
                        .on_click(cx.listener(|g, _, _, cx| g.run(0, cx))),
                ),
            )
    }

    fn progress(&self, p: &Palette) -> impl IntoElement {
        let tests = self.tree.as_ref().map(|t| t.tests).unwrap_or(0);
        let total = self.run_total.max(1) as f32;
        let summary = match self.phase {
            Phase::NoProject => "Open a Karate project folder.".to_string(),
            Phase::Discovering => "Finding tests…".to_string(),
            _ if self.run_total == 0 => format!("{tests} tests · Not run yet"),
            _ => format!("{} of {} · {:.1} s", self.done, self.run_total, self.run_secs),
        };
        div()
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_1p5()
            .bg(p.surface)
            .child(
                // Passed and failed, side by side, out of the run's total.
                div()
                    .id("progress")
                    .role(Role::ProgressIndicator)
                    .aria_label("Run progress")
                    .aria_numeric_value(self.done as f64)
                    .aria_min_numeric_value(0.)
                    .aria_max_numeric_value(self.run_total as f64)
                    .h(px(4.))
                    .w_full()
                    .flex()
                    .rounded_full()
                    .overflow_hidden()
                    .bg(p.line)
                    .child(div().h_full().bg(p.passed).w(gpui::relative(self.passed as f32 / total)))
                    .child(div().h_full().bg(p.failed).w(gpui::relative(self.failed as f32 / total))),
            )
            .child(
                div()
                    .id("summary")
                    .role(Role::Status)
                    .aria_label(SharedString::from(summary.clone()))
                    .flex()
                    .gap_3()
                    .text_sm()
                    .text_color(p.muted)
                    .child(summary)
                    .when(self.run_total > 0, |d| {
                        d.child(div().flex().gap_1().items_center().child(Icon::new(IconName::CircleCheck).small().text_color(p.passed)).child(format!("{} passed", self.passed)))
                            .child(div().flex().gap_1().items_center().child(Icon::new(IconName::CircleX).small().text_color(p.failed)).child(format!("{} failed", self.failed)))
                    }),
            )
    }

    fn tag_bar(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .px_3()
            .pb_2()
            .bg(p.surface)
            .when(!self.tags.is_empty(), |d| d.child(Icon::new(IconName::Tag).small().text_color(p.muted)))
            .children(self.tags.iter().enumerate().map(|(i, tag)| {
                let on = self.tags_on.contains(tag);
                let tag2 = tag.clone();
                Button::new(("tag", i))
                    .xsmall()
                    .label(tag.clone())
                    .when(on, |b| b.primary())
                    .when(!on, |b| b.outline())
                    .on_click(cx.listener(move |g, _, _, cx| g.toggle_tag(tag2.clone(), cx)))
            }))
    }

    fn details(&self, p: &Palette) -> gpui::AnyElement {
        let (Some(i), Some(t)) = (self.selected, &self.tree) else {
            return div().p_4().text_sm().text_color(p.muted).child("Select a test to see its details").into_any_element();
        };
        let n = &t.nodes[i];
        let (icon, c) = status_icon(n.status, p);
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(SelectableText::new(("detail-name", i), n.name.clone()).into_any_element())
            .child(div().text_sm().text_color(p.muted).child(SelectableText::new(
                ("detail-file", i),
                format!("{}{}", n.file, n.line.map(|l| format!(":{l}")).unwrap_or_default()),
            )))
            .child(div().flex().gap_1().children(n.tags.iter().map(|t| chip(t.clone(), p))))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .child(Icon::new(icon).small().text_color(c))
                    .child(status_word(n.status))
                    .children(n.millis.map(|m| format!("· {m} ms"))),
            )
            .children(n.error.clone().map(|e| {
                div()
                    .p_2()
                    .rounded_md()
                    .bg(p.failed_tint)
                    .text_color(p.failed)
                    .font_family(MONO)
                    .text_xs()
                    .child(SelectableText::new(("detail-error", i), e))
            }))
            .into_any_element()
    }

    fn bottom(&self, p: &Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = TabBar::new("bottom-tabs")
            .underline()
            .small()
            .selected_index(self.tab)
            .child(Tab::new().label("Console").prefix(Icon::new(IconName::SquareTerminal).small()))
            .child(Tab::new().label(format!("History ({})", self.history.len())).prefix(Icon::new(IconName::Inbox).small()))
            .child(Tab::new().label(format!("HTTP ({})", self.http.len())).prefix(Icon::new(IconName::Network).small()))
            .on_click(cx.listener(|g, ix: &usize, _, cx| {
                g.tab = *ix;
                cx.notify();
            }));
        let body = match self.tab {
            0 => {
                let handle = self.console_text.clone();
                uniform_list(
                    "console-lines",
                    self.console.len(),
                    cx.processor(move |g: &mut Grace, range: std::ops::Range<usize>, _w, _cx| {
                        range
                            .map(|i| {
                                div().px_3().h(px(18.)).whitespace_nowrap().child(
                                    SelectableText::with_handle(("line", i), handle.clone(), g.console[i].clone())
                                        .document_order(i as u64),
                                )
                            })
                            .collect()
                    }),
                )
                .track_scroll(&self.console_scroll)
                .font_family(MONO)
                .text_xs()
                .flex_1()
                .into_any_element()
            }
            1 => div()
                .id("history")
                .overflow_y_scroll()
                .flex_1()
                .px_3()
                .text_sm()
                .when(self.history.is_empty(), |d| d.text_color(p.muted).child("No runs yet."))
                .children(self.history.iter().map(|r| {
                    let (icon, c) = match r.outcome {
                        "passed" => (IconName::CircleCheck, p.passed),
                        "failed" | "error" => (IconName::CircleX, p.failed),
                        _ => (IconName::CircleSlash, p.skipped),
                    };
                    div()
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .border_b_1()
                        .border_color(p.line)
                        .child(Icon::new(icon).small().text_color(c))
                        .child(div().w(px(110.)).child(r.when.clone()))
                        .child(div().w(px(90.)).child(r.outcome))
                        .child(div().text_color(p.muted).child(format!(
                            "{} of {} passed · {} failed · {:.1} s",
                            r.passed, r.total, r.failed, r.secs
                        )))
                }))
                .into_any_element(),
            _ => uniform_list(
                "http-calls",
                self.http.len(),
                cx.processor(|g: &mut Grace, range: std::ops::Range<usize>, _w, cx| {
                    let p = palette(cx);
                    range
                        .map(|i| {
                            let c = &g.http[i];
                            let ok = c.status.map(|s| s < 400).unwrap_or(false);
                            div()
                                .px_3()
                                .h(px(24.))
                                .w_full()
                                .flex()
                                .items_center()
                                .gap_3()
                                .text_sm()
                                .child(div().w(px(60.)).font_family(MONO).font_weight(FontWeight::MEDIUM).child(c.method.clone()))
                                .child(
                                    div()
                                        .w(px(44.))
                                        .font_family(MONO)
                                        .text_color(if ok { p.passed } else { p.failed })
                                        .child(c.status.map(|s| s.to_string()).unwrap_or_else(|| "—".into())),
                                )
                                .child(div().flex_1().min_w_0().truncate().font_family(MONO).child(c.url.clone()))
                                .child(div().text_color(p.muted).child(c.millis.map(|m| format!("{m} ms")).unwrap_or_default()))
                                .child(div().w(px(180.)).truncate().text_color(p.muted).child(c.feature.clone()))
                        })
                        .collect()
                }),
            )
            .flex_1()
            .when(self.http.is_empty(), |d| d)
            .into_any_element(),
        };
        div()
            .id("bottom")
            .h(px(220.))
            .flex()
            .flex_col()
            .bg(p.surface)
            .border_t_1()
            .border_color(p.line)
            .child(div().px_2().child(tabs))
            .child(div().flex_1().min_h_0().flex().flex_col().pt_1().child(body))
    }
}

impl Render for Grace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.first_frame {
            self.first_frame = true;
            window.on_next_frame(|_, _| probe("first_frame"));
        }
        if self.tree.is_some() && !self.tree_probed {
            self.tree_probed = true;
            window.on_next_frame(|_, _| probe("tree_shown"));
        }
        let p = palette(cx);

        let rows = self.visible.len();
        let tree = uniform_list(
            "tree",
            rows,
            cx.processor(|g: &mut Grace, range: std::ops::Range<usize>, _w, cx| {
                if PROBING.load(Ordering::Relaxed) && WHEEL_TURNED.swap(false, Ordering::Relaxed) {
                    g.frames.push(Instant::now());
                }
                let p = palette(cx);
                let t = g.tree.as_ref().unwrap();
                let filtering = g.filtering();
                range
                    .map(|vi| {
                        let i = g.visible[vi];
                        let n = &t.nodes[i];
                        let (icon, icon_color) = status_icon(n.status, &p);
                        let has_kids = !n.children.is_empty();
                        let open = n.expanded || filtering;
                        let selected = g.selected == Some(i);
                        div()
                            .id(("node", i))
                            .role(Role::TreeItem)
                            .aria_label(SharedString::from(n.name.clone()))
                            // What AT-SPI showed missing (#269): the status, the level, open or closed, selected.
                            .aria_description(status_word(n.status))
                            .aria_level(n.depth + 1)
                            .when(has_kids, |d| d.aria_expanded(open))
                            .aria_selected(selected)
                            .h(px(26.))
                            .w_full()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .pl(px(8. + 16. * n.depth as f32))
                            .pr_2()
                            .text_sm()
                            .when(selected, |d| d.bg(p.selected))
                            .when(!selected, |d| d.hover(|s| s.bg(p.hover)))
                            .on_click(cx.listener(move |g, _, _, cx| {
                                g.selected = Some(i);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .id(("chevron", i))
                                    .w(px(14.))
                                    .text_color(p.muted)
                                    .when(has_kids, |d| {
                                        // AccessKit's AT-SPI adapter has no expand action, so the chevron is a button (#269).
                                        d.role(Role::Button).aria_label(if open { "Close" } else { "Open" }).child(Icon::new(if open { IconName::ChevronDown } else { IconName::ChevronRight }).small())
                                    })
                                    .on_click(cx.listener(move |g, _, _, cx| {
                                        cx.stop_propagation();
                                        g.toggle(i, cx);
                                    })),
                            )
                            .child(Icon::new(icon).small().text_color(icon_color))
                            .child(div().flex_shrink_0().text_color(p.ink).child(n.name.clone()))
                            .when(!n.title.is_empty(), |d| d.child(div().min_w_0().truncate().text_color(p.muted).child(n.title.clone())))
                            .child(div().flex_1())
                            .children(n.tags.iter().take(2).map(|t| chip(t.clone(), &p)))
                            .children(n.millis.map(|m| div().text_xs().text_color(p.muted).child(format!("{m} ms"))))
                            .child(
                                Button::new(("run-node", i))
                                    .ghost()
                                    .xsmall()
                                    .icon(Icon::new(IconName::Play))
                                    .tooltip("Run")
                                    .disabled(g.phase != Phase::Ready)
                                    .on_click(cx.listener(move |g, _, _, cx| {
                                        cx.stop_propagation();
                                        g.run(i, cx);
                                    })),
                            )
                    })
                    .collect()
            }),
        )
        .track_scroll(&self.scroll)
        .size_full();

        let empty_filter = self.filtering() && self.visible.is_empty() && self.tree.is_some();
        div()
            .id("grace")
            .key_context("Grace")
            .on_action(cx.listener(|g, _: &RunAll, _, cx| g.run(0, cx)))
            .on_action(cx.listener(|g, _: &Cancel, _, cx| g.cancel_run(cx)))
            .on_action(cx.listener(|g, _: &Open, window, cx| g.prompt_open(window, cx)))
            .role(Role::Application)
            .aria_label("Grace")
            .size_full()
            .flex()
            .flex_col()
            .bg(p.ground)
            .text_color(p.ink)
            .font_family(look::SANS)
            .text_sm()
            .child(self.toolbar(&p, cx))
            .child(self.progress(&p))
            .child(a11y::live(
                div()
                    .id("announcement")
                    .role(Role::Status)
                    // No label until the first run: AccessKit announces a live node's name as soon as it appears.
                    .when(!self.announcement.is_empty(), |d| d.aria_label(self.announcement.clone()))
                    .size_0(),
                gpui::accesskit::Live::Polite,
            ))
            .child(self.tag_bar(&p, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap_px()
                    .bg(p.line)
                    .border_t_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .w(gpui::relative(0.55))
                            .h_full()
                            .bg(p.surface)
                            .when(empty_filter, |d| d.p_4().text_color(p.muted).child("No test matches the filter."))
                            .when(!empty_filter, |d| d.child(tree)),
                    )
                    .child(div().flex_1().h_full().bg(p.surface).child(self.details(&p))),
            )
            .child(self.bottom(&p, cx))
            .child(
                div()
                    .px_3()
                    .h(px(24.))
                    .flex()
                    .items_center()
                    .bg(p.sunken)
                    .border_t_1()
                    .border_color(p.line)
                    .text_xs()
                    .text_color(p.muted)
                    .child(self.root.as_ref().map(|r| r.display().to_string()).unwrap_or_default()),
            )
            .child(
                gpui::canvas(
                    |_, _, _| (),
                    |_, _, window, _| {
                        window.on_mouse_event(|_: &gpui::ScrollEvent, phase, _, _| {
                            if phase == gpui::DispatchPhase::Capture && PROBING.load(Ordering::Relaxed) {
                                WHEEL_TURNED.store(true, Ordering::Relaxed);
                            }
                        });
                    },
                )
                .size_0(),
            )
    }
}

/// What every window of the app needs: GPUI Kit, IBM Plex, the theme and the keys.
fn init(cx: &mut App) {
    gpui_component::init(cx);
    // Quitting during a run or a listing stops its Java, as Grace does, instead of leaving it running.
    cx.on_app_quit(|_| {
        karate::kill_all();
        async {}
    })
    .detach();
    look::load_fonts(cx);
    cx.bind_keys([
        KeyBinding::new("ctrl-enter", RunAll, None),
        KeyBinding::new("escape", Cancel, Some("Grace")),
        KeyBinding::new("ctrl-o", Open, None),
    ]);
}

/// The window's root: GPUI Kit's `Root` (dialogs, tooltips, text selection) around Grace.
fn open_window(cx: &mut App, options: WindowOptions, root: Option<PathBuf>) -> gpui::WindowHandle<Root> {
    cx.open_window(options, |window, cx| {
        look::apply(look::wants_dark(window), Some(window), cx);
        let grace = cx.new(|cx| Grace::new(root.clone(), window, cx));
        cx.new(|cx| Root::new(grace, window, cx))
    })
    .unwrap()
}

fn main() {
    probe("main");
    let root = std::env::args().nth(1).map(PathBuf::from);
    let probe_mode = std::env::var_os("GRACE_PROBE").is_some();
    gpui_platform::application().with_assets(look::Assets).run(move |cx: &mut App| {
        init(cx);
        let bounds = if probe_mode {
            Bounds::new(gpui::point(px(0.), px(0.)), size(px(1400.), px(900.)))
        } else {
            Bounds::centered(None, size(px(1400.), px(900.)), cx)
        };
        let window = open_window(
            cx,
            WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
            root.clone(),
        );
        // The theme follows the system's, as Grace's does, unless GRACE_THEME pins it.
        let _ = window.update(cx, |_, window, _| {
            window
                .observe_window_appearance(|window, cx| look::apply(look::wants_dark(window), Some(window), cx))
                .detach();
        });
        let grace = window.update(cx, |root, _, _| root.view().clone().downcast::<Grace>().unwrap()).unwrap();
        // The measure script drives the run through stdin ("run", "quit"), as the bench clicks Run all over CDP.
        if probe_mode {
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<String>();
            std::thread::spawn(move || {
                let mut line = String::new();
                while std::io::stdin().read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
                    let _ = tx.unbounded_send(line.trim().to_string());
                    line.clear();
                }
            });
            cx.spawn(async move |cx| {
                while let Some(cmd) = rx.next().await {
                    grace.update(cx, |g, cx| match cmd.as_str() {
                        "run" => g.run(0, cx),
                        "quit" => cx.quit(),
                        // The Windows idle patch's check: a second window opened while the first is minimised.
                        c if c.starts_with("open-window ") => {
                            let xy: Vec<f32> = c[12..].split(' ').filter_map(|s| s.parse().ok()).collect();
                            let bounds = Bounds::new(gpui::point(px(xy[0]), px(xy[1])), size(px(900.), px(600.)));
                            open_window(
                                cx,
                                WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
                                None,
                            );
                        }
                        // For screenshots: select the first failed test, or show a bottom tab.
                        "select-failed" => {
                            g.selected = g.tree.as_ref().and_then(|t| {
                                t.nodes.iter().position(|n| n.children.is_empty() && n.status == Status::Failed)
                            });
                            if let Some(t) = &mut g.tree {
                                for n in t.nodes.iter_mut() {
                                    n.expanded = true;
                                }
                            }
                            g.relist();
                            cx.notify();
                        }
                        c if c.starts_with("tab ") => {
                            g.tab = c[4..].parse().unwrap_or(0);
                            cx.notify();
                        }
                        // The frame probe: "frames-start", scroll, then "frames-stop" prints each frame's gap.
                        "frames-start" => {
                            g.frames.clear();
                            WHEEL_TURNED.store(false, Ordering::Relaxed);
                            PROBING.store(true, Ordering::Relaxed);
                        }
                        "frames-stop" => {
                            PROBING.store(false, Ordering::Relaxed);
                            let gaps: Vec<String> = g
                                .frames
                                .windows(2)
                                .map(|w| format!("{:.1}", (w[1] - w[0]).as_secs_f64() * 1000.0))
                                .collect();
                            eprintln!("PROBE frames {} {}", g.frames.len(), gaps.join(","));
                        }
                        _ => {}
                    });
                }
            })
            .detach();
        }
    });
}

/// UI tests: the `*.test.tsx` (Testing Library) and e2e layers. GPUI's test platform draws no pixels and opens no real
/// window, but lays out and hit-tests the real element tree, so a test clicks what a person would click.
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Modifiers, TestAppContext, VisualTestContext};
    use std::path::Path;

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../test/fixtures")
    }

    fn window(cx: &mut TestAppContext, root: Option<PathBuf>) -> (Entity<Grace>, VisualTestContext) {
        let w = cx.update(|cx| {
            init(cx);
            open_window(cx, Default::default(), root.clone())
        });
        let mut vcx = VisualTestContext::from_window(w.into(), cx);
        let r = w.root(&mut vcx).unwrap();
        let g = r.read_with(&vcx, |r, _| r.view().clone().downcast::<Grace>().unwrap());
        (g, vcx)
    }

    fn with_tree(g: &Entity<Grace>, cx: &mut VisualTestContext, events: &str, name: &str) {
        g.update(cx, |g, cx| {
            let t = karate::build_tree(&fixtures().join("events").join(events), name).unwrap();
            let mut tags: Vec<SharedString> = t.nodes.iter().flat_map(|n| n.tags.iter().cloned()).map(Into::into).collect();
            tags.sort();
            tags.dedup();
            g.tags = tags;
            g.root = Some(PathBuf::from("/").join(name));
            g.tree = Some(t);
            g.relist();
            g.phase = Phase::Ready;
            cx.notify();
        });
    }

    /// A component test with a recorded tree and fed events: no Java.
    #[gpui::test]
    fn statuses_roll_up_from_live_events(cx: &mut TestAppContext) {
        let (g, mut cx) = window(cx, None);
        with_tree(&g, &mut cx, "dryrun-clean.jsonl", "clean");
        g.update(&mut cx, |g, cx| {
            g.phase = Phase::Running;
            g.run_total = 9;
            g.apply(vec![
                Msg::Started { file: "tests/basic.feature".into(), ref_id: "[1:7]".into() },
                Msg::Finished {
                    file: "tests/basic.feature".into(),
                    ref_id: "[2:15]".into(),
                    status: Status::Failed,
                    millis: 8,
                    error: Some("match failed".into()),
                },
            ]);
            cx.notify();
        });
        cx.run_until_parked();
        g.read_with(&cx, |g, _| {
            let t = g.tree.as_ref().unwrap();
            let basic = t.nodes.iter().position(|n| n.name == "basic.feature").unwrap();
            assert_eq!(t.nodes[basic].status, Status::Running, "a running test wins over a failed one");
            assert_eq!((g.done, g.failed), (1, 1));
        });
    }

    #[gpui::test]
    fn closing_a_folder_hides_its_rows(cx: &mut TestAppContext) {
        let (g, mut cx) = window(cx, None);
        with_tree(&g, &mut cx, "dryrun-clean.jsonl", "clean");
        g.update(&mut cx, |g, cx| {
            assert_eq!(g.visible.len(), 5, "root, tests, and its 3 features");
            g.toggle(g.visible[1], cx);
            assert_eq!(g.visible.len(), 2);
        });
    }

    /// Typing in the filter box narrows the tree to the matching tests and the folders that hold them.
    #[gpui::test]
    fn typing_in_the_filter_narrows_the_tree(cx: &mut TestAppContext) {
        let (g, mut cx) = window(cx, None);
        with_tree(&g, &mut cx, "dryrun-clean.jsonl", "clean");
        let filter = g.read_with(&cx, |g, _| g.filter.clone());
        cx.update(|window, cx| filter.update(cx, |f, cx| f.focus(window, cx)));
        cx.simulate_input("basic");
        cx.run_until_parked();
        g.read_with(&cx, |g, _| {
            let t = g.tree.as_ref().unwrap();
            let names: Vec<&str> = g.visible.iter().map(|&i| t.nodes[i].name.as_str()).collect();
            assert!(names.contains(&"basic.feature"), "{names:?}");
            assert!(!names.iter().any(|n| n.ends_with(".feature") && *n != "basic.feature"), "{names:?}");
        });
    }

    /// The Cancel button asks first; only the dialog's own button cancels.
    #[gpui::test]
    fn cancel_asks_before_it_stops_the_run(cx: &mut TestAppContext) {
        let (g, mut cx) = window(cx, None);
        with_tree(&g, &mut cx, "dryrun-clean.jsonl", "clean");
        let flag = Arc::new(AtomicBool::new(false));
        g.update(&mut cx, |g, cx| {
            g.phase = Phase::Running;
            g.cancel = Some(flag.clone());
            cx.notify();
        });
        cx.update(|window, cx| g.update(cx, |g, cx| g.confirm_cancel(window, cx)));
        cx.run_until_parked();
        assert!(cx.update(|window, cx| window.has_active_dialog(cx)), "a dialog opened");
        assert!(!flag.load(Ordering::Relaxed), "nothing cancelled yet");
    }

    /// The port of e2e/run.spec.ts "Run all with live progress": the real Karate on the `clean` fixture project,
    /// driven by a click on the Run all button, headless.
    #[gpui::test]
    async fn run_all_with_live_progress(cx: &mut TestAppContext) {
        let java = std::env::var("GRACE_JAVA");
        if java.is_err() {
            eprintln!("skipped: set GRACE_JAVA and GRACE_KARATE_JAR to run Karate");
            return;
        }
        cx.executor().allow_parking();
        let data = std::env::temp_dir().join(format!("grace-gpui-e2e-{}", std::process::id()));
        // SAFETY: tests in this binary that read GRACE_USER_DATA run after this point only.
        unsafe { std::env::set_var("GRACE_USER_DATA", &data) };
        let (g, mut cx) = window(cx, Some(fixtures().join("projects/clean")));
        let wait = |cx: &mut VisualTestContext, until: &dyn Fn(&Grace) -> bool| {
            for _ in 0..600 {
                cx.executor().advance_clock(Duration::from_millis(100));
                cx.run_until_parked();
                if g.read_with(cx, |g, _| until(g)) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            panic!("timed out");
        };
        wait(&mut cx, &|g| g.phase == Phase::Ready);
        g.read_with(&cx, |g, _| assert_eq!(g.tree.as_ref().unwrap().tests, 9));

        let button = cx.debug_bounds("run-all").expect("a debug build records selectors");
        cx.simulate_click(button.center(), Modifiers::default());
        g.read_with(&cx, |g, _| assert!(g.phase == Phase::Running));
        wait(&mut cx, &|g| g.phase == Phase::Ready);

        g.read_with(&cx, |g, _| {
            assert_eq!((g.done, g.passed, g.failed), (9, 8, 1));
            let t = g.tree.as_ref().unwrap();
            let basic = t.nodes.iter().position(|n| n.name == "basic.feature").unwrap();
            assert_eq!(t.nodes[basic].status, Status::Failed);
            assert_eq!(g.history.len(), 1);
            assert_eq!(g.history[0].outcome, "failed");
        });
        std::fs::remove_dir_all(data).ok();
    }
}
