//! A throwaway GPUI prototype of Grace's main screen (#152): open a Karate project, list its tests from a dry run,
//! Run all or one node with live progress, cancel, the selected test's details, and Karate's console output.
//! Not part of the build. See ../README.md.
mod karate;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures::StreamExt;
use gpui::{
    App, Bounds, Context, FontWeight, KeyBinding, PathPromptOptions, Role, SharedString, Task,
    UniformListScrollHandle, Window, WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
    uniform_list,
};
use karate::{Kind, Msg, Status, Tree, Vendor};

actions!(grace, [RunAll, Cancel, Open]);

/// `GRACE_PROBE` prints `PROBE <name> <unix ms>` to stderr for the measure script; nothing else reads it.
static PROBING: AtomicBool = AtomicBool::new(false);

fn probe(name: &str) {
    if std::env::var_os("GRACE_PROBE").is_some() {
        let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        eprintln!("PROBE {name} {ms}");
    }
}

/// Grace's light theme, from the semantic tokens of src/index.css (approximated).
mod color {
    pub const BG: u32 = 0xffffff;
    pub const SURFACE: u32 = 0xf6f7f6;
    pub const BORDER: u32 = 0xe3e5e3;
    pub const TEXT: u32 = 0x1c1f1d;
    pub const MUTED: u32 = 0x6b716d;
    pub const ACCENT: u32 = 0x1f7a3d;
    pub const PASSED: u32 = 0x1f7a3d;
    pub const FAILED: u32 = 0xc4312b;
    pub const RUNNING: u32 = 0x2563c9;
    pub const SELECTED: u32 = 0xe6f0ea;
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    NoProject,
    Discovering,
    Ready,
    Running,
}

struct Grace {
    vendor: Vendor,
    root: Option<PathBuf>,
    tree: Option<Tree>,
    /// Indexes of the rows shown, in order: recomputed when a node opens or closes.
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
}

const CONSOLE_LINES: usize = 400;

impl Grace {
    fn new(root: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let data = std::env::var_os("GRACE_USER_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("grace-gpui"));
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
        };
        if let Some(r) = root {
            g.open(r, cx);
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

    fn out_dir(&self, what: &str) -> PathBuf {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        self.data.join("runs").join(format!("{what}-{stamp}"))
    }

    /// Discovery: a dry run, then the tree from its events (runner.ts `open`).
    fn open(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.root = Some(root.clone());
        self.tree = None;
        self.visible.clear();
        self.selected = None;
        self.phase = Phase::Discovering;
        self.console.clear();
        self.log(format!("project  {}", root.display()));
        self.log("Finding tests…");
        let out = self.out_dir("dryrun");
        let args = karate::args(&self.vendor.jar, &root, &out, true, 1, &[root.display().to_string()]);
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

    fn relist(&mut self) {
        self.visible.clear();
        let Some(t) = &self.tree else { return };
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            self.visible.push(i);
            if t.nodes[i].expanded {
                stack.extend(t.nodes[i].children.iter().rev());
            }
        }
    }

    fn toggle(&mut self, i: usize, cx: &mut Context<Self>) {
        if let Some(t) = &mut self.tree {
            if !t.nodes[i].children.is_empty() {
                t.nodes[i].expanded = !t.nodes[i].expanded;
                self.relist();
                cx.notify();
            }
        }
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
        let targets = self.target(i);
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
        self.run_started = Some(Instant::now());
        self.run_secs = 0.0;
        self.phase = Phase::Running;
        self.log(format!("Running {} tests on {} thread(s)…", tests.len(), self.threads));
        let out = self.out_dir("run");
        let args = karate::args(&self.vendor.jar, &root, &out, false, self.threads, &targets);
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
        let mut over = false;
        for m in batch {
            match m {
                Msg::Line(l) => self.log(l),
                Msg::Started { file, ref_id } => {
                    if let Some(t) = &mut self.tree {
                        if let Some(&i) = t.by_ref.get(&(file, ref_id)) {
                            t.nodes[i].status = Status::Running;
                        }
                    }
                }
                Msg::Finished { file, ref_id, status, millis, error } => {
                    if let Some(t) = &mut self.tree {
                        if let Some(&i) = t.by_ref.get(&(file, ref_id)) {
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
                    probe("run_finished");
                }
            }
        }
        if let Some(s) = self.run_started {
            self.run_secs = s.elapsed().as_secs_f32();
        }
        self.roll_up();
        over
    }

    /// A folder, feature or outline shows the worst status below it.
    fn roll_up(&mut self) {
        let Some(t) = &mut self.tree else { return };
        fn up(nodes: &mut Vec<Node>, i: usize) -> Status {
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
            } else if all.iter().all(|s| *s == Status::Passed) {
                Status::Passed
            } else if all.iter().any(|s| *s == Status::Passed) {
                Status::Passed
            } else {
                all.first().copied().unwrap_or(Status::NotRun)
            };
            nodes[i].status = s;
            s
        }
        use karate::Node;
        up(&mut t.nodes, 0);
    }

    fn cancel_run(&mut self, cx: &mut Context<Self>) {
        if let Some(c) = &self.cancel {
            c.store(true, Ordering::Relaxed);
            self.log("Cancelling…");
            cx.notify();
        }
    }

    fn prompt_open(&mut self, cx: &mut Context<Self>) {
        if self.phase == Phase::Running || self.phase == Phase::Discovering {
            return;
        }
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open a Karate project".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(mut p))) = paths.await {
                if let Some(dir) = p.pop() {
                    let _ = this.update(cx, |g, cx| g.open(dir, cx));
                }
            }
        })
        .detach();
    }
}

fn status_mark(s: Status) -> (&'static str, u32) {
    match s {
        Status::NotRun => ("○", color::MUTED),
        Status::Queued => ("◌", color::MUTED),
        Status::Running => ("◐", color::RUNNING),
        Status::Passed => ("●", color::PASSED),
        Status::Failed => ("✕", color::FAILED),
        Status::Skipped => ("–", color::MUTED),
    }
}

fn button(id: &'static str, label: &'static str, enabled: bool, primary: bool) -> gpui::Stateful<gpui::Div> {
    let (bg, fg, border) = if primary { (color::ACCENT, color::BG, color::ACCENT) } else { (color::BG, color::TEXT, color::BORDER) };
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .px_3()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(border))
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .text_sm()
        .when(!enabled, |d| d.opacity(0.45))
        .when(enabled, |d| d.cursor_pointer().hover(|s| s.opacity(0.85)))
        .child(label)
}

fn chip(text: String) -> gpui::Div {
    div()
        .px_1p5()
        .rounded_sm()
        .border_1()
        .border_color(rgb(color::BORDER))
        .text_xs()
        .text_color(rgb(color::MUTED))
        .child(text)
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
        let ready = self.phase == Phase::Ready;
        let running = self.phase == Phase::Running;
        let project = self
            .root
            .as_ref()
            .and_then(|r| r.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "No project".into());
        let tests = self.tree.as_ref().map(|t| t.tests).unwrap_or(0);

        let toolbar = div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(rgb(color::BORDER))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Grace"))
            .child(div().text_color(rgb(color::MUTED)).child(project))
            .child(div().flex_1())
            .child(div().text_sm().text_color(rgb(color::MUTED)).child("Threads"))
            .child(
                button("threads-down", "−", ready && self.threads > 1, false)
                    .on_click(cx.listener(|g, _, _, cx| {
                        g.threads = g.threads.saturating_sub(1).max(1);
                        cx.notify();
                    })),
            )
            .child(div().text_sm().child(self.threads.to_string()))
            .child(button("threads-up", "+", ready, false).on_click(cx.listener(|g, _, _, cx| {
                g.threads = (g.threads + 1).min(16);
                cx.notify();
            })))
            .child(button("open", "Open…", !running, false).on_click(cx.listener(|g, _, _, cx| g.prompt_open(cx))))
            .child(button("cancel", "Cancel", running, false).on_click(cx.listener(|g, _, _, cx| g.cancel_run(cx))))
            .child(button("run-all", "Run all", ready && tests > 0, true).debug_selector(|| "run-all".into()).on_click(cx.listener(|g, _, _, cx| g.run(0, cx))));

        let pct = if self.run_total > 0 { self.done as f32 / self.run_total as f32 } else { 0.0 };
        let summary = match self.phase {
            Phase::NoProject => "Open a Karate project folder.".to_string(),
            Phase::Discovering => "Finding tests…".to_string(),
            _ if self.run_total == 0 => format!("{tests} tests · Not run yet"),
            _ => format!(
                "{} of {} · {} passed · {} failed · {:.1} s",
                self.done, self.run_total, self.passed, self.failed, self.run_secs
            ),
        };
        let progress = div()
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div().h(px(4.)).w_full().rounded_full().bg(rgb(color::BORDER)).child(
                    div()
                        .h_full()
                        .rounded_full()
                        .bg(rgb(if self.failed > 0 { color::FAILED } else { color::PASSED }))
                        .w(gpui::relative(pct)),
                ),
            )
            .child(div().text_sm().text_color(rgb(color::MUTED)).child(summary));

        let rows = self.visible.len();
        let tree = uniform_list(
            "tree",
            rows,
            cx.processor(|g: &mut Grace, range: std::ops::Range<usize>, _w, cx| {
                if PROBING.load(Ordering::Relaxed) {
                    g.frames.push(Instant::now());
                }
                let t = g.tree.as_ref().unwrap();
                range
                    .map(|vi| {
                        let i = g.visible[vi];
                        let n = &t.nodes[i];
                        let (mark, mark_color) = status_mark(n.status);
                        let has_kids = !n.children.is_empty();
                        let chevron = if !has_kids { " " } else if n.expanded { "▾" } else { "▸" };
                        let selected = g.selected == Some(i);
                        div()
                            .id(("node", i))
                            .role(Role::TreeItem)
                            .aria_label(SharedString::from(n.name.clone()))
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .pl(px(8. + 16. * n.depth as f32))
                            .pr_2()
                            .text_sm()
                            .when(selected, |d| d.bg(rgb(color::SELECTED)))
                            .hover(|s| s.bg(rgb(color::SURFACE)))
                            .on_click(cx.listener(move |g, _, _, cx| {
                                g.selected = Some(i);
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .id(("chevron", i))
                                    .w(px(12.))
                                    .text_color(rgb(color::MUTED))
                                    .child(chevron)
                                    .on_click(cx.listener(move |g, _, _, cx| {
                                        cx.stop_propagation();
                                        g.toggle(i, cx);
                                    })),
                            )
                            .child(div().w(px(12.)).text_color(rgb(mark_color)).child(mark))
                            .child(div().flex_shrink_0().text_color(rgb(color::TEXT)).child(n.name.clone()))
                            .when(!n.title.is_empty(), |d| {
                                d.child(div().min_w_0().truncate().text_color(rgb(color::MUTED)).child(n.title.clone()))
                            })
                            .child(div().flex_1())
                            .children(n.tags.iter().take(2).map(|t| chip(t.clone())))
                            .child(
                                div()
                                    .id(("run-node", i))
                                    .role(Role::Button)
                                    .aria_label("Run")
                                    .px_1()
                                    .text_color(rgb(color::MUTED))
                                    .when(g.phase == Phase::Ready, |d| d.cursor_pointer().hover(|s| s.text_color(rgb(color::ACCENT))))
                                    .child("▶")
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

        let details = match (self.selected, &self.tree) {
            (Some(i), Some(t)) => {
                let n = &t.nodes[i];
                let (mark, c) = status_mark(n.status);
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_4()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(n.name.clone()))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(color::MUTED))
                            .child(format!("{}{}", n.file, n.line.map(|l| format!(":{l}")).unwrap_or_default())),
                    )
                    .child(div().flex().gap_1().children(n.tags.iter().map(|t| chip(t.clone()))))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .text_sm()
                            .child(div().text_color(rgb(c)).child(mark))
                            .child(format!("{:?}", n.status))
                            .children(n.millis.map(|m| format!("· {m} ms"))),
                    )
                    .children(n.error.clone().map(|e| {
                        div()
                            .p_2()
                            .rounded_md()
                            .bg(rgb(0xfdf0ef))
                            .text_color(rgb(color::FAILED))
                            .font_family("monospace")
                            .text_xs()
                            .child(e)
                    }))
            }
            _ => div()
                .p_4()
                .text_sm()
                .text_color(rgb(color::MUTED))
                .child("Select a test to see its details"),
        };

        let console_len = self.console.len();
        let console = div()
            .id("console")
            .h(px(190.))
            .border_t_1()
            .border_color(rgb(color::BORDER))
            .flex()
            .flex_col()
            .child(div().px_3().py_1().text_sm().font_weight(FontWeight::SEMIBOLD).child("Console"))
            .child(
                uniform_list(
                    "console-lines",
                    console_len,
                    cx.processor(|g: &mut Grace, range: std::ops::Range<usize>, _w, _cx| {
                        range
                            .map(|i| {
                                div().px_3().h(px(16.)).text_xs().font_family("monospace").whitespace_nowrap().child(g.console[i].clone())
                            })
                            .collect()
                    }),
                )
                .flex_1(),
            );

        div()
            .id("grace")
            .key_context("Grace")
            .on_action(cx.listener(|g, _: &RunAll, _, cx| g.run(0, cx)))
            .on_action(cx.listener(|g, _: &Cancel, _, cx| g.cancel_run(cx)))
            .on_action(cx.listener(|g, _: &Open, _, cx| g.prompt_open(cx)))
            .role(Role::Application)
            .aria_label("Grace")
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(color::BG))
            .text_color(rgb(color::TEXT))
            .child(toolbar)
            .child(progress)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .border_t_1()
                    .border_color(rgb(color::BORDER))
                    .child(div().w(gpui::relative(0.5)).h_full().border_r_1().border_color(rgb(color::BORDER)).child(tree))
                    .child(div().flex_1().h_full().child(details)),
            )
            .child(console)
            .child(
                div()
                    .px_3()
                    .py_1()
                    .border_t_1()
                    .border_color(rgb(color::BORDER))
                    .text_xs()
                    .text_color(rgb(color::MUTED))
                    .child(self.root.as_ref().map(|r| r.display().to_string()).unwrap_or_default()),
            )
    }
}

fn main() {
    probe("main");
    let root = std::env::args().nth(1).map(PathBuf::from);
    let probe_mode = std::env::var_os("GRACE_PROBE").is_some();
    gpui_platform::application().run(move |cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("ctrl-enter", RunAll, None),
            KeyBinding::new("escape", Cancel, None),
            KeyBinding::new("ctrl-o", Open, None),
        ]);
        let bounds = if probe_mode {
            Bounds::new(gpui::point(px(0.), px(0.)), size(px(1400.), px(900.)))
        } else {
            Bounds::centered(None, size(px(1400.), px(900.)), cx)
        };
        let window = cx
            .open_window(
                WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), ..Default::default() },
                |_, cx| cx.new(|cx| Grace::new(root.clone(), cx)),
            )
            .unwrap();
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
                    let _ = window.update(cx, |g, _, cx| match cmd.as_str() {
                        "run" => g.run(0, cx),
                        "quit" => cx.quit(),
                        // The frame probe: "frames-start", scroll, then "frames-stop" prints each frame's gap.
                        "frames-start" => {
                            g.frames.clear();
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

    fn window(cx: &mut TestAppContext, root: Option<PathBuf>) -> (gpui::Entity<Grace>, VisualTestContext) {
        let w = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|cx| Grace::new(root.clone(), cx))).unwrap()
        });
        let mut vcx = VisualTestContext::from_window(w.into(), cx);
        let g = w.root(&mut vcx).unwrap();
        (g, vcx)
    }

    /// A component test with a recorded tree and fed events: no Java.
    #[gpui::test]
    fn statuses_roll_up_from_live_events(cx: &mut TestAppContext) {
        let (g, mut cx) = window(cx, None);
        g.update(&mut cx, |g, cx| {
            let t = karate::build_tree(&fixtures().join("events/dryrun-clean.jsonl"), "clean").unwrap();
            g.root = Some(PathBuf::from("/clean"));
            g.tree = Some(t);
            g.relist();
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
        g.update(&mut cx, |g, cx| {
            g.tree = Some(karate::build_tree(&fixtures().join("events/dryrun-clean.jsonl"), "clean").unwrap());
            g.relist();
            assert_eq!(g.visible.len(), 5, "root, tests, and its 3 features");
            g.toggle(g.visible[1], cx);
            assert_eq!(g.visible.len(), 2);
        });
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
        });
        std::fs::remove_dir_all(data).ok();
    }
}
