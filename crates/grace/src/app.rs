//! The `Grace` entity: the runner and the window's state, in one process (migration guide, Architecture).
//!
//! Electron Grace's renderer called main over IPC (`src/shared/ipc.ts`) and main answered with `AppEvent`s. Here the
//! views call `Grace`'s methods; each runs the runner's blocking call on GPUI's background executor, and the runner's
//! events come back on a channel that one foreground task waits on. That task applies every event queued behind the
//! first as one batch (`batch.ts`'s tick, #123) and notifies the views once. While no event comes it waits on the
//! channel, so nothing wakes while Grace is idle (ADR 0030).
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{App, AppContext as _, Context, Entity, Task};
use grace_core::model::events::AppEvent;
use grace_core::model::tags::FilterState;
use grace_core::runner::Runner;
use grace_core::state::AppState;

use crate::seams::Seams;

/// What the window asks of the runner: `grace_core::runner::Runner`'s calls that the views make. A trait, so view
/// tests run on a fake (guide R6.2). Every method blocks; `Grace` calls them off the UI thread.
pub trait Backend: Send + Sync + 'static {
    /// Opens a folder the person picked; true if it is now the open project.
    fn open(&self, root: &str) -> std::io::Result<bool>;
    fn open_recent(&self, root: &str) -> std::io::Result<()>;
    fn remove_recent(&self, root: &str);
    /// Checks the saved recent folders and sends them.
    fn send_recent(&self);
    fn close(&self);
    /// Cancels a run and waits for Java to end: before Grace quits (WS-09).
    fn dispose(&self);
    /// What is open or opening now, as events.
    fn replay(&self) -> Vec<AppEvent>;
    /// Lists the open project's tests again.
    fn refresh(&self) -> std::io::Result<()>;
    /// Runs the tests under a node: the tree's root for Run all.
    fn run(&self, key: &str) -> std::io::Result<()>;
    /// Runs again the tests that failed in a listed run.
    fn rerun_failed(&self, id: &str) -> std::io::Result<()>;
    fn cancel(&self);
    /// Sets the open project's tag filter; the runner sends it back as an event.
    fn set_filter(&self, filter: FilterState);
    /// Sets the open project's environment, `None` for none; the runner sends the one in force back.
    fn set_env(&self, name: Option<&str>) -> std::io::Result<()>;
    /// Sets the open project's thread count as the box holds it; the runner checks it and sends the count in force.
    fn set_threads(&self, n: &serde_json::Value);
    /// The last run's HTML report, if it is there.
    fn report_path(&self) -> Option<PathBuf>;
}

impl Backend for Runner {
    fn open(&self, root: &str) -> std::io::Result<bool> {
        Runner::open(self, root)
    }
    fn open_recent(&self, root: &str) -> std::io::Result<()> {
        Runner::open_recent(self, root)
    }
    fn remove_recent(&self, root: &str) {
        Runner::remove_recent(self, root)
    }
    fn send_recent(&self) {
        Runner::send_recent(self)
    }
    fn close(&self) {
        Runner::close(self)
    }
    fn dispose(&self) {
        Runner::dispose(self)
    }
    fn replay(&self) -> Vec<AppEvent> {
        Runner::replay(self)
    }
    fn refresh(&self) -> std::io::Result<()> {
        Runner::refresh(self)
    }
    fn run(&self, key: &str) -> std::io::Result<()> {
        Runner::run(self, key)
    }
    fn rerun_failed(&self, id: &str) -> std::io::Result<()> {
        Runner::rerun_failed(self, id)
    }
    fn cancel(&self) {
        Runner::cancel(self)
    }
    fn set_filter(&self, filter: FilterState) {
        Runner::set_filter(self, filter)
    }
    fn set_env(&self, name: Option<&str>) -> std::io::Result<()> {
        Runner::set_env(self, name)
    }
    fn set_threads(&self, n: &serde_json::Value) {
        Runner::set_threads(self, n)
    }
    fn report_path(&self) -> Option<PathBuf> {
        Runner::report_path(self)
    }
}

/// The sending end the runner's `emit` holds.
pub type Events = async_channel::Sender<AppEvent>;

/// A channel for the runner's events: give the sender to the runner's `emit`, the receiver to [`Grace::new`].
pub fn events() -> (Events, async_channel::Receiver<AppEvent>) {
    async_channel::unbounded()
}

/// The runner, the window's state and the platform seams.
pub struct Grace {
    /// Shared with the views as they draw, so a redraw copies no state; an event copies it only while a view still
    /// holds the old one, which none does between frames.
    pub state: Rc<AppState>,
    backend: Arc<dyn Backend>,
    pub seams: Arc<dyn Seams>,
    data_dir: PathBuf,
    /// Closed once the last call sent to the runner has done what the next must wait for (`Grace::background`).
    order: async_channel::Receiver<()>,
    _events: Task<()>,
}

/// What a call to the runner holds back until it is done, for the calls sent after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Order {
    /// A quick call, such as a setting: the calls after it wait for its end, so a run after a new thread count uses it.
    End,
    /// A call that lasts, such as a run or a discovery: the calls after it wait only for its start, so they are not
    /// held for the whole run (the runner refuses what a run forbids).
    Start,
}

impl Grace {
    /// Starts applying the runner's events as they come.
    pub fn new(
        backend: Arc<dyn Backend>,
        seams: Arc<dyn Seams>,
        data_dir: PathBuf,
        events: async_channel::Receiver<AppEvent>,
        cx: &mut Context<Self>,
    ) -> Self {
        let task = cx.spawn(async move |this, cx| {
            // Waits for an event, then takes every one queued behind it: one update and one redraw per batch.
            while let Ok(first) = events.recv().await {
                let mut batch = vec![first];
                while let Ok(e) = events.try_recv() {
                    batch.push(e);
                }
                let applied = this.update(cx, |grace, cx| {
                    for e in &batch {
                        grace.state_mut().reduce(e);
                    }
                    cx.notify();
                });
                if applied.is_err() {
                    break; // the window is gone
                }
            }
        });
        let (_, order) = async_channel::bounded(1);
        Self {
            state: Rc::new(AppState::new()),
            backend,
            seams,
            data_dir,
            order,
            _events: task,
        }
    }

    /// The state, to change it.
    pub fn state_mut(&mut self) -> &mut AppState {
        Rc::make_mut(&mut self.state)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Runs a blocking call of the runner off the UI thread, in the order the calls were made, as Electron's main
    /// process took them one by one: a setting is in force before a run sent after it starts. An error becomes a
    /// notice, as `main.ts`'s `handle` made it.
    fn background(
        &mut self,
        cx: &mut Context<Self>,
        order: Order,
        call: impl FnOnce(&dyn Backend) -> std::io::Result<()> + Send + 'static,
    ) {
        let backend = Arc::clone(&self.backend);
        let (done, next) = async_channel::bounded::<()>(1);
        let before = std::mem::replace(&mut self.order, next);
        let task = cx.background_spawn(async move {
            let _ = before.recv().await; // fails once the call before has closed it
            if order == Order::Start {
                drop(done);
                call(backend.as_ref())
            } else {
                let result = call(backend.as_ref());
                drop(done);
                result
            }
        });
        Self::report(task, cx);
    }

    /// Runs a call at once, ahead of any waiting: Cancel must reach a run that holds the calls behind it.
    fn at_once(&self, cx: &mut Context<Self>, call: impl FnOnce(&dyn Backend) -> std::io::Result<()> + Send + 'static) {
        let backend = Arc::clone(&self.backend);
        let task = cx.background_spawn(async move { call(backend.as_ref()) });
        Self::report(task, cx);
    }

    fn report(task: Task<std::io::Result<()>>, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            if let Err(e) = task.await {
                let _ = this.update(cx, |grace, cx| {
                    grace.state_mut().reduce(&AppEvent::notice(e.to_string()));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// At start (`main.ts`): reopens the last project, or says nothing is open, then sends the recent folders. The
    /// window shows the ground alone until one of them arrives, so the home screen never flashes before a project
    /// (HS-05).
    pub fn start(&mut self, cx: &mut Context<Self>) {
        let backend = Arc::clone(&self.backend);
        let dir = self.data_dir.clone();
        let task = cx.background_spawn(async move {
            let opened = match grace_core::settings::load_last_project(&dir) {
                Some(last) => backend.open(&last).map(|_| Vec::new()),
                None => Ok(backend.replay()),
            };
            backend.send_recent();
            opened
        });
        cx.spawn(async move |this, cx| {
            let events = match task.await {
                Ok(events) => events,
                Err(e) => vec![AppEvent::notice(e.to_string())],
            };
            if !events.is_empty() {
                let _ = this.update(cx, |grace, cx| {
                    for e in &events {
                        grace.state_mut().reduce(e);
                    }
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Open…: the folder dialog, starting next to the newest recent folder that still exists, then the folder.
    pub fn open_project(&mut self, cx: &mut Context<Self>) {
        let seams = Arc::clone(&self.seams);
        let data_dir = self.data_dir.clone();
        let backend = Arc::clone(&self.backend);
        let start = cx.background_spawn(async move { start_folder(&data_dir) });
        cx.spawn(async move |this, cx| {
            let start = start.await;
            let Some(folder) = seams.pick_folder(start, cx).await else {
                return;
            };
            let opened = cx
                .background_spawn(async move { backend.open(&folder.to_string_lossy()).map(|_| ()) })
                .await;
            if let Err(e) = opened {
                let _ = this.update(cx, |grace, cx| {
                    grace.state_mut().reduce(&AppEvent::notice(e.to_string()));
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn open_recent(&mut self, root: String, cx: &mut Context<Self>) {
        self.background(cx, Order::Start, move |b| b.open_recent(&root));
    }

    pub fn remove_recent(&mut self, root: String, cx: &mut Context<Self>) {
        self.background(cx, Order::End, move |b| {
            b.remove_recent(&root);
            Ok(())
        });
    }

    pub fn send_recent(&mut self, cx: &mut Context<Self>) {
        self.background(cx, Order::End, |b| {
            b.send_recent();
            Ok(())
        });
    }

    /// Home: closes the project and goes back to the home screen.
    pub fn close_project(&mut self, cx: &mut Context<Self>) {
        self.background(cx, Order::Start, |b| {
            b.close();
            Ok(())
        });
    }

    /// Refresh: lists the tests again.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.background(cx, Order::Start, |b| b.refresh());
    }

    /// Run all: every test of the tree, whatever the filter hides (the runner applies the filter).
    pub fn run_all(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.state.tree.as_ref().map(|t| t.key.clone()) else {
            return;
        };
        self.background(cx, Order::Start, move |b| b.run(&key));
    }

    /// Re-run failed: the failed tests of the newest listed run.
    pub fn rerun_failed(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.state.history.first().map(|h| h.id.clone()) else {
            return;
        };
        self.background(cx, Order::Start, move |b| b.rerun_failed(&id));
    }

    /// Runs the tests under a row of the tree.
    pub fn run(&mut self, key: String, cx: &mut Context<Self>) {
        self.background(cx, Order::Start, move |b| b.run(&key));
    }

    /// The tree's selection, which the details pane shows.
    pub fn select(&mut self, key: &str, cx: &mut Context<Self>) {
        self.state_mut().select(key);
        cx.notify();
    }

    /// The tag filter. The window never edits the filter itself: the runner sends it back.
    pub fn set_filter(&mut self, filter: FilterState, cx: &mut Context<Self>) {
        self.background(cx, Order::End, move |b| {
            b.set_filter(filter);
            Ok(())
        });
    }

    /// The environment box's commit. The window never edits the environment itself: the runner sends it back.
    pub fn set_env(&mut self, name: Option<String>, cx: &mut Context<Self>) {
        self.background(cx, Order::Start, move |b| b.set_env(name.as_deref()));
    }

    /// The threads box's commit, as the box holds it: the runner refuses anything but 1 to 32.
    pub fn set_threads(&mut self, n: serde_json::Value, cx: &mut Context<Self>) {
        self.background(cx, Order::End, move |b| {
            b.set_threads(&n);
            Ok(())
        });
    }

    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.at_once(cx, |b| {
            b.cancel();
            Ok(())
        });
    }

    /// Report: the last run's HTML report in the default browser, as `shell.openPath` opened it.
    pub fn open_report(&mut self, cx: &mut Context<Self>) {
        let backend = Arc::clone(&self.backend);
        let seams = Arc::clone(&self.seams);
        let path = cx.background_spawn(async move { backend.report_path() });
        cx.spawn(async move |_, cx| {
            if let Some(path) = path.await {
                cx.update(|cx| seams.open_path(&path, cx));
            }
        })
        .detach();
    }

    /// Cancels a run and waits for Java to end, off the UI thread.
    pub fn dispose(&self, cx: &App) -> Task<()> {
        let backend = Arc::clone(&self.backend);
        cx.background_spawn(async move { backend.dispose() })
    }
}

/// Where Open… starts: the parent of the newest recent folder that still exists (#82). `checkRecent` gives up on a
/// folder that does not answer within its time bound, so this never hangs on a dead share.
pub fn start_folder(data_dir: &Path) -> Option<PathBuf> {
    let recent = grace_core::settings::load_recent_projects(data_dir);
    let checked = grace_core::recent::RecentChecker::new().check(&recent);
    let newest = checked.into_iter().find(|r| r.exists)?;
    Path::new(&newest.path).parent().map(Path::to_path_buf)
}

/// A `Grace` with the app's runner, its events flowing into it.
pub fn new_app(
    data_dir: PathBuf,
    vendor: grace_core::vendor::VendorPaths,
    seams: Arc<dyn Seams>,
    cx: &mut App,
) -> Entity<Grace> {
    let (tx, rx) = events();
    let runner = Runner::new(
        data_dir.clone(),
        vendor,
        Arc::new(grace_core::secrets::os_secret_box()),
        move |e| {
            let _ = tx.try_send(e); // unbounded: fails only once the window is gone
        },
    );
    cx.new(|cx| Grace::new(Arc::new(runner), seams, data_dir, rx, cx))
}

#[cfg(test)]
pub mod fake {
    //! A runner for view tests: records each call and sends the events a test gives it.
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct FakeBackend {
        pub calls: Mutex<Vec<String>>,
    }

    impl FakeBackend {
        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
        fn record(&self, call: String) {
            self.calls.lock().unwrap().push(call);
        }
    }

    impl Backend for FakeBackend {
        fn open(&self, root: &str) -> std::io::Result<bool> {
            self.record(format!("open {root}"));
            Ok(true)
        }
        fn open_recent(&self, root: &str) -> std::io::Result<()> {
            self.record(format!("open_recent {root}"));
            Ok(())
        }
        fn remove_recent(&self, root: &str) {
            self.record(format!("remove_recent {root}"));
        }
        fn send_recent(&self) {
            self.record("send_recent".into());
        }
        fn close(&self) {
            self.record("close".into());
        }
        fn dispose(&self) {
            self.record("dispose".into());
        }
        fn replay(&self) -> Vec<AppEvent> {
            self.record("replay".into());
            vec![AppEvent::Opening { root: None }, AppEvent::Busy { busy: false }]
        }
        fn refresh(&self) -> std::io::Result<()> {
            self.record("refresh".into());
            Ok(())
        }
        fn run(&self, key: &str) -> std::io::Result<()> {
            self.record(format!("run {key}"));
            Ok(())
        }
        fn rerun_failed(&self, id: &str) -> std::io::Result<()> {
            self.record(format!("rerun_failed {id}"));
            Ok(())
        }
        fn cancel(&self) {
            self.record("cancel".into());
        }
        fn set_filter(&self, filter: FilterState) {
            self.record(format!("set_filter {}", serde_json::to_string(&filter).unwrap()));
        }
        fn set_env(&self, name: Option<&str>) -> std::io::Result<()> {
            self.record(format!("set_env {}", name.unwrap_or("null")));
            Ok(())
        }
        fn set_threads(&self, n: &serde_json::Value) {
            self.record(format!("set_threads {n}"));
        }
        fn report_path(&self) -> Option<PathBuf> {
            self.record("report_path".into());
            Some(PathBuf::from("/out/karate-summary.html"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeBackend;
    use super::*;
    use crate::seams::fake::FakeSeams;
    use gpui::TestAppContext;
    use grace_core::model::events::RecentEntry;

    fn grace(cx: &mut TestAppContext) -> (Entity<Grace>, Arc<FakeBackend>, Events) {
        let backend = Arc::new(FakeBackend::default());
        let (tx, rx) = events();
        let b: Arc<dyn Backend> = backend.clone();
        let g = cx.update(|cx| cx.new(|cx| Grace::new(b, Arc::new(FakeSeams::default()), PathBuf::new(), rx, cx)));
        (g, backend, tx)
    }

    #[gpui::test]
    fn applies_the_runners_events_as_they_come(cx: &mut TestAppContext) {
        let (g, _, tx) = grace(cx);
        tx.try_send(AppEvent::notice("hi")).unwrap();
        tx.try_send(AppEvent::Recent {
            entries: vec![RecentEntry {
                path: "D:/p".into(),
                exists: true,
            }],
        })
        .unwrap();
        cx.run_until_parked();
        g.read_with(cx, |g, _| {
            assert_eq!(g.state.notice.as_ref().unwrap().text, "hi");
            assert_eq!(g.state.recent.as_ref().unwrap().len(), 1);
        });
    }

    #[gpui::test]
    fn calls_the_runner_off_the_ui_thread(cx: &mut TestAppContext) {
        let (g, backend, _tx) = grace(cx);
        g.update(cx, |g, cx| {
            g.open_recent("D:/p".into(), cx);
            g.remove_recent("D:/q".into(), cx);
        });
        cx.run_until_parked();
        let mut calls = backend.calls();
        calls.sort();
        assert_eq!(calls, ["open_recent D:/p", "remove_recent D:/q"]);
    }

    #[gpui::test(iterations = 20)]
    fn a_setting_is_in_force_before_a_run_sent_after_it(cx: &mut TestAppContext) {
        let (g, backend, _tx) = grace(cx);
        g.update(cx, |g, cx| {
            g.state_mut().tree = Some(crate::testing::tree(1));
            g.set_threads(serde_json::json!(2), cx);
            g.run_all(cx);
            g.set_filter(grace_core::model::tags::no_filter(), cx);
            g.refresh(cx);
        });
        cx.run_until_parked();
        let calls: Vec<String> = backend
            .calls()
            .into_iter()
            .map(|c| c[..c.find(' ').unwrap_or(c.len())].to_string())
            .collect();
        assert_eq!(calls, ["set_threads", "run", "set_filter", "refresh"]);
    }

    #[gpui::test]
    fn a_first_start_says_nothing_is_open_and_sends_the_recent_folders(cx: &mut TestAppContext) {
        let (g, backend, _tx) = grace(cx);
        g.update(cx, |g, cx| g.start(cx));
        cx.run_until_parked();
        assert_eq!(backend.calls(), ["replay", "send_recent"]);
        g.read_with(cx, |g, _| assert!(g.state.loaded && g.state.home()));
    }

    #[gpui::test]
    fn open_asks_for_a_folder_and_opens_it(cx: &mut TestAppContext) {
        let backend = Arc::new(FakeBackend::default());
        let seams = Arc::new(FakeSeams::answering(Some("/work/api")));
        let (_tx, rx) = events();
        let b: Arc<dyn Backend> = backend.clone();
        let s: Arc<dyn Seams> = seams.clone();
        let dir = std::env::temp_dir().join("grace-open-test-none");
        let g = cx.update(|cx| cx.new(|cx| Grace::new(b, s, dir, rx, cx)));
        g.update(cx, |g, cx| g.open_project(cx));
        cx.run_until_parked();
        assert_eq!(backend.calls(), ["open /work/api"]);
        assert_eq!(seams.asked(), vec![None]); // no recent folder: the dialog's own start
    }
}
