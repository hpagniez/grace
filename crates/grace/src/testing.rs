//! What the views' headless tests share: a `Grace` on the fake runner, a window that draws one view of it with the
//! window's tooltips, and the states the Electron tests built (`test/fixtures`, `Toolbar.test.tsx`'s `state()`).
use std::path::PathBuf;
use std::sync::Arc;

use gpui::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, SharedString, Styled as _,
    TestAppContext, VisualTestContext, Window, div, px, size,
};
use grace_core::model::events::{Project, RunOutcome};
use grace_core::model::history::HistoryEntry;
use grace_core::model::node::{Layout, NodeKind, TestNode};
use grace_core::model::tags::no_filter;
use grace_core::state::AppState;

use crate::app::fake::FakeBackend;
use crate::app::{Backend, Grace};
use crate::seams::fake::FakeSeams;
use crate::tooltip::Tips;

/// Draws a part of the window on `grace`.
pub type Draw = Box<dyn Fn(&Entity<Grace>, &mut Window, &mut gpui::App) -> AnyElement>;

pub struct View {
    pub grace: Entity<Grace>,
    pub tips: Option<Entity<Tips>>,
    draw: Draw,
}

impl Render for View {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tips = self
            .tips
            .get_or_insert_with(|| crate::tooltip::install(window, cx))
            .clone();
        div()
            .size_full()
            .flex()
            .flex_col()
            .child((self.draw)(&self.grace, window, cx))
            .child(tips)
    }
}

pub struct Harness {
    pub grace: Entity<Grace>,
    pub backend: Arc<FakeBackend>,
    pub seams: Arc<FakeSeams>,
    pub view: Entity<View>,
}

impl Harness {
    /// Puts `state` in place of the runner's, as the Electron tests rendered a view on a state.
    pub fn set(&self, cx: &mut VisualTestContext, state: AppState) {
        self.grace.update(cx, |g, cx| {
            *g.state_mut() = state;
            cx.notify();
        });
        self.view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
    }

    pub fn update(&self, cx: &mut VisualTestContext, change: impl FnOnce(&mut AppState)) {
        self.grace.update(cx, |g, cx| {
            change(g.state_mut());
            cx.notify();
        });
        self.view.update(cx, |_, cx| cx.notify());
        cx.run_until_parked();
    }

    /// The tooltip showing now.
    pub fn tip(&self, cx: &mut VisualTestContext) -> Option<SharedString> {
        cx.update(|_, cx| self.view.read(cx).tips.as_ref()?.read(cx).text())
    }
}

/// A window `width` wide drawing `draw` on a `Grace` holding `state`.
pub fn open(
    cx: &mut TestAppContext,
    width: f32,
    state: AppState,
    draw: impl Fn(&Entity<Grace>, &mut Window, &mut gpui::App) -> AnyElement + 'static,
) -> (Harness, VisualTestContext) {
    cx.update(crate::init);
    let backend = Arc::new(FakeBackend::default());
    let seams = Arc::new(FakeSeams::answering(Some("/picked")));
    let (tx, rx) = crate::app::events();
    std::mem::forget(tx); // the channel stays open, so the events task waits
    let b: Arc<dyn Backend> = backend.clone();
    let s: Arc<dyn crate::seams::Seams> = seams.clone();
    let grace = cx.update(|cx| cx.new(|cx| Grace::new(b, s, PathBuf::new(), rx, cx)));
    grace.update(cx, |g, _| *g.state_mut() = state);
    let g = grace.clone();
    let window = cx.add_window(move |_, _| View {
        grace: g,
        tips: None,
        draw: Box::new(draw),
    });
    let view = window.root(cx).unwrap();
    let vcx = VisualTestContext::from_window(window.into(), cx);
    vcx.simulate_resize(size(px(width), px(700.)));
    vcx.run_until_parked();
    (
        Harness {
            grace,
            backend,
            seams,
            view,
        },
        vcx,
    )
}

fn node(key: &str, kind: NodeKind, name: &str, tags: &[&str], children: Vec<TestNode>) -> TestNode {
    TestNode {
        key: key.into(),
        kind,
        name: name.into(),
        file: String::new(),
        line: None,
        ref_id: None,
        tags: tags.iter().map(|t| t.to_string()).collect(),
        example: None,
        children,
    }
}

/// A tree of one feature with `n` scenarios, the first tagged `smoke`.
pub fn tree(n: usize) -> TestNode {
    let scenarios = (0..n)
        .map(|i| {
            let tags: &[&str] = if i == 0 { &["smoke"] } else { &[] };
            node(
                &format!("s:a.feature#{i}"),
                NodeKind::Scenario,
                &format!("s{i}"),
                tags,
                vec![],
            )
        })
        .collect();
    node(
        "d:",
        NodeKind::Folder,
        "",
        &[],
        vec![node("f:a.feature", NodeKind::Feature, "a", &[], scenarios)],
    )
}

/// A project open at `root`, with `tests` tests and no run yet.
pub fn project(root: &str, layout: Layout, tests: usize) -> AppState {
    let mut s = AppState::new();
    s.loaded = true;
    s.project = Some(Project {
        root: root.into(),
        layout,
    });
    s.tree = Some(tree(tests));
    s.filter = no_filter();
    s
}

/// A listed run with `failed` failures.
pub fn run(id: &str, passed: u32, failed: u32) -> HistoryEntry {
    HistoryEntry {
        id: id.into(),
        started_at: 0.,
        duration_millis: 1000.,
        env: None,
        threads: None,
        filter: no_filter(),
        selection: None,
        failed_of: None,
        outcome: RunOutcome::Finished,
        exit_code: Some(i64::from(failed > 0)),
        passed,
        failed,
        total: passed + failed,
        report_available: true,
        debug: false,
    }
}
