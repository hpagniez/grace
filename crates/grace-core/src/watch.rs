//! Watch mode's watcher (docs/specs/2026-11-03-watch-mode.md, R2): each platform's file events on the feature roots,
//! never a poll. Its threads block in the OS with no timeout until a file changes; the quiet window waits on a
//! channel with a timeout only while a batch is open (ADR 0030). Nothing in the app uses it until the runner does.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::fingerprint::SKIPPED_DIRS;

#[cfg(windows)]
mod sys;

/// How long no counted event must come before a batch of saves is handed over (R2.3).
pub const QUIET: Duration = Duration::from_millis(300);

/// Windows stamps a file from a clock that ticks every 15.6 ms (2 s on FAT): a save just after the start may look
/// older than it. A feature changed in the slack before the start can count once on a read; harmless.
const SLACK: Duration = Duration::from_secs(2);

fn skipped(rel: &str) -> bool {
    rel.split(['\\', '/'])
        .any(|p| SKIPPED_DIRS.iter().any(|s| p.eq_ignore_ascii_case(s)))
}

/// A path as the OS gives it, relative to its root: a `.feature` outside the skipped folders (R2.2).
pub fn counts(rel: &str) -> bool {
    let last = rel.rsplit(['\\', '/']).next().unwrap_or(rel);
    last.to_lowercase().ends_with(".feature") && !skipped(rel)
}

/// Every feature under `dir` outside the skipped folders; links to folders are not followed.
fn features(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return; // a folder removed meanwhile
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        let Ok(kind) = e.file_type() else { continue };
        if kind.is_dir() {
            if !SKIPPED_DIRS.contains(&name.as_str()) {
                features(&e.path(), out);
            }
        } else if kind.is_file() && name.ends_with(".feature") {
            out.push(e.path());
        }
    }
}

/// What a platform's events say, sent to the quiet thread.
enum Msg {
    /// A counted feature, absolute.
    Changed(PathBuf),
    /// A folder made under a folder root after the start, whose files the OS may not have reported (Linux).
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Walk(PathBuf),
    /// The OS lost names: the batch takes every feature of the folder roots.
    Overflow,
    /// A root was deleted, renamed or moved away.
    Gone(PathBuf),
    Failed(String),
    Stop,
}

/// The saves waiting for the quiet window to end, and the modified time last counted per feature.
struct Batch {
    folders: Vec<PathBuf>,
    files: Vec<PathBuf>,
    started: SystemTime,
    seen: HashMap<PathBuf, SystemTime>,
    pending: HashSet<PathBuf>,
    walks: Vec<PathBuf>,
    overflow: bool,
}

impl Batch {
    fn new(roots: &[(PathBuf, bool)]) -> Batch {
        Batch {
            folders: roots.iter().filter(|(_, file)| !file).map(|(r, _)| r.clone()).collect(),
            files: roots.iter().filter(|(_, file)| *file).map(|(r, _)| r.clone()).collect(),
            started: SystemTime::now() - SLACK,
            seen: HashMap::new(),
            pending: HashSet::new(),
            walks: Vec::new(),
            overflow: false,
        }
    }

    fn take(&mut self, msg: Msg) {
        match msg {
            Msg::Changed(p) => drop(self.pending.insert(p)),
            Msg::Walk(dir) => self.walks.push(dir),
            Msg::Overflow => self.overflow = true,
            _ => {}
        }
    }

    /// The batch's features that are still files with a modified time later than the start and than the time last
    /// counted. shortcut: the walk after an overflow runs on the quiet thread, which holds the next batch meanwhile;
    /// move it to a thread of its own if a project's walk ever takes long.
    fn flush(&mut self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        if std::mem::take(&mut self.overflow) {
            for root in &self.folders {
                features(root, &mut found);
            }
            // Linux has one queue for every root, so its overflow may have lost a file root's event too
            found.extend(self.files.iter().cloned());
        }
        for dir in std::mem::take(&mut self.walks) {
            features(&dir, &mut found);
        }
        self.pending.extend(found);
        let mut saved = Vec::new();
        for p in std::mem::take(&mut self.pending) {
            let Ok(meta) = fs::metadata(&p) else {
                continue; // deleted or renamed away
            };
            let Ok(mtime) = meta.modified() else { continue };
            let last = self.seen.get(&p).copied().unwrap_or(self.started).max(self.started);
            if meta.is_file() && mtime > last {
                self.seen.insert(p.clone(), mtime);
                saved.push(p);
            }
        }
        saved
    }
}

/// One platform source of events; closing it is dropping it.
#[cfg(windows)]
type Source = sys::Reader;
#[cfg(target_os = "linux")]
type Source = notify::INotifyWatcher;

/// Watches each root, a folder recursively or a single file, and hands over the absolute paths of the features saved
/// once `quiet` passed with no counted event (`on_saved`, on the watcher's own thread). A burst too big for the OS
/// loses its names: the batch then takes every feature of the folder roots that the modified-time check keeps. Each
/// folder root's parent is watched too, without its subfolders, as a renamed or moved root may report nothing itself.
/// Fails, having closed what it opened, if a root cannot be watched; a read Windows refuses later is an `on_error`.
/// After `on_error`, called once, nothing more is called; nor after `close`. The callbacks must only send or post,
/// never wait on what `close`'s caller may hold: `close` waits for a callback that is running.
pub fn watch_features(
    roots: &[PathBuf],
    mut on_saved: impl FnMut(Vec<PathBuf>) + Send + 'static,
    on_error: impl FnOnce(String) + Send + 'static,
    quiet: Duration,
) -> io::Result<Watcher> {
    let mut kinds = Vec::new();
    for root in roots {
        kinds.push((root.clone(), fs::metadata(root)?.is_file()));
    }
    let mut batch = Batch::new(&kinds);
    let (tx, rx) = mpsc::channel();
    #[cfg(test)]
    let threads = Arc::new(Mutex::new(Vec::new()));
    let sources = Arc::new(Mutex::new(platform::sources(
        &kinds,
        &tx,
        #[cfg(test)]
        &threads,
    )?));
    let held = Arc::clone(&sources);
    let closed = Arc::new(AtomicBool::new(false));
    let seen_closed = Arc::clone(&closed);
    #[cfg(all(test, target_os = "linux"))]
    let quiet_threads = Arc::clone(&threads);
    let quiet_thread = thread::Builder::new().name("grace-watch".into()).spawn(move || {
        #[cfg(all(test, target_os = "linux"))]
        quiet_threads.lock().unwrap().push(platform::tid());
        let mut open = false;
        let failure = loop {
            let msg = if open {
                match rx.recv_timeout(quiet) {
                    Ok(msg) => msg,
                    Err(RecvTimeoutError::Timeout) => {
                        open = false;
                        let saved = batch.flush();
                        if !saved.is_empty() && !seen_closed.load(Ordering::SeqCst) {
                            on_saved(saved);
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => return,
                }
            };
            match msg {
                Msg::Stop => return,
                Msg::Gone(root) => break format!("{} was removed or renamed", root.display()),
                Msg::Failed(reason) => break reason,
                msg => {
                    batch.take(msg);
                    open = true;
                }
            }
        };
        drop(std::mem::take(&mut *held.lock().unwrap()));
        // `close` from inside `on_saved` may land before a `Gone` already queued
        if !seen_closed.load(Ordering::SeqCst) {
            on_error(failure);
        }
    })?;
    Ok(Watcher {
        closed,
        tx,
        quiet: Some(quiet_thread),
        sources,
        #[cfg(test)]
        threads,
    })
}

/// A running watcher; `close` or a drop stops it.
pub struct Watcher {
    closed: Arc<AtomicBool>,
    tx: mpsc::Sender<Msg>,
    quiet: Option<thread::JoinHandle<()>>,
    sources: Arc<Mutex<Vec<Source>>>,
    /// Linux: the watcher's threads' ids, for the idle test.
    #[cfg(test)]
    #[cfg_attr(windows, allow(dead_code))]
    threads: Arc<Mutex<Vec<u32>>>,
}

impl Watcher {
    /// Stops every source and the quiet thread, and returns once nothing more can be called. From `on_saved` or
    /// `on_error` it returns without waiting for the thread it runs on.
    pub fn close(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        let _ = self.tx.send(Msg::Stop);
        drop(std::mem::take(&mut *self.sources.lock().unwrap()));
        if let Some(quiet) = self.quiet.take()
            && quiet.thread().id() != thread::current().id()
        {
            let _ = quiet.join();
        }
    }

    /// Each of the watcher's threads' CPU so far: cycles on Windows, context switches on Linux.
    #[cfg(test)]
    fn cpu(&self) -> Vec<u64> {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            let quiet = self.quiet.as_ref().map(|q| sys::cycles(q.as_raw_handle()));
            let sources = self.sources.lock().unwrap();
            quiet.into_iter().chain(sources.iter().map(|s| s.cycles())).collect()
        }
        #[cfg(target_os = "linux")]
        {
            let tids = self.threads.lock().unwrap();
            tids.iter().map(|tid| platform::switches(*tid)).collect()
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(windows)]
use sys::FOLDER;

/// The `FILE_NOTIFY_INFORMATION` entries of a completed read, walked as bytes: the names, and whether the walk
/// reached the last entry with every offset and length inside the buffer.
#[cfg(windows)]
fn entries(buf: &[u8]) -> (Vec<String>, bool) {
    let field = |at: usize| -> Option<u32> {
        let bytes = buf.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    };
    let mut names = Vec::new();
    let mut at = 0usize;
    loop {
        let (Some(next), Some(len)) = (field(at), at.checked_add(8).and_then(field)) else {
            return (names, false);
        };
        let start = at + 12; // `field(at + 8)` read up to here
        let len = len as usize;
        let Some(name) = start.checked_add(len).and_then(|end| buf.get(start..end)) else {
            return (names, false);
        };
        if !len.is_multiple_of(2) {
            return (names, false);
        }
        let units: Vec<u16> = name.as_chunks::<2>().0.iter().map(|&u| u16::from_le_bytes(u)).collect();
        names.push(String::from_utf16_lossy(&units));
        if next == 0 {
            return (names, true);
        }
        match at.checked_add(next as usize) {
            Some(n) => at = n,
            None => return (names, false),
        }
    }
}

/// What one completed read says.
#[cfg(windows)]
enum Read {
    Names(Vec<String>),
    Overflow,
    Failed(io::Error),
}

/// A read that completed with these bytes or this Windows error. Windows says an overflow by 0 bytes or by
/// `ERROR_NOTIFY_ENUM_DIR`; a buffer that cannot be walked whole is one too, as the batch then walks the roots.
#[cfg(windows)]
fn completion(result: Result<&[u8], u32>) -> Read {
    const ERROR_NOTIFY_ENUM_DIR: u32 = 1022;
    match result {
        Ok([]) | Err(ERROR_NOTIFY_ENUM_DIR) => Read::Overflow,
        Ok(bytes) => match entries(bytes) {
            (names, true) => Read::Names(names),
            (_, false) => Read::Overflow,
        },
        Err(code) => Read::Failed(io::Error::from_raw_os_error(code as i32)),
    }
}

#[cfg(windows)]
mod platform {
    use super::*;

    /// A root gone, or another error once it has been looked for.
    fn failed(root: &Path, err: io::Error) -> Msg {
        if root.exists() {
            Msg::Failed(format!("{}: {err}", root.display()))
        } else {
            Msg::Gone(root.to_path_buf())
        }
    }

    /// One reader per folder root (its tree), per folder root's parent (its own entries) and per file root (its
    /// folder's own entries, kept to its name).
    pub(super) fn sources(
        roots: &[(PathBuf, bool)],
        tx: &mpsc::Sender<Msg>,
        #[cfg(test)] _threads: &Arc<Mutex<Vec<u32>>>,
    ) -> io::Result<Vec<Source>> {
        let mut out = Vec::new();
        for (root, file) in roots {
            let (root, tx) = (root.clone(), tx.clone());
            if *file {
                let (Some(dir), Some(name)) = (root.parent(), root.file_name()) else {
                    return Err(io::Error::other(format!("{} has no folder", root.display())));
                };
                let name = name.to_string_lossy().to_lowercase();
                let dir = dir.to_path_buf();
                out.push(sys::Reader::start(&dir, false, sys::FILE, move |read| {
                    send(&tx, file_root(&root, &name, read))
                })?);
                continue;
            }
            let parent = root.parent().map(Path::to_path_buf);
            let tree = (root.clone(), tx.clone());
            out.push(sys::Reader::start(&root, true, FOLDER, move |read| {
                send(&tree.1, folder_root(&tree.0, read))
            })?);
            // a drive's root has no parent
            if let Some(parent) = parent {
                let base = root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                out.push(sys::Reader::start(&parent, false, sys::PARENT, move |read| {
                    send(&tx, parent_of(&root, &base, read))
                })?);
            }
        }
        Ok(out)
    }

    /// Sends what a read says; false, which ends the reader, once the read failed.
    fn send(tx: &mpsc::Sender<Msg>, msgs: (Vec<Msg>, bool)) -> bool {
        for m in msgs.0 {
            let _ = tx.send(m);
        }
        msgs.1
    }

    fn folder_root(root: &Path, read: Read) -> (Vec<Msg>, bool) {
        match read {
            Read::Names(names) => (
                names
                    .iter()
                    .filter(|n| counts(n))
                    .map(|n| Msg::Changed(root.join(n)))
                    .collect(),
                true,
            ),
            Read::Overflow => (vec![Msg::Overflow], true),
            Read::Failed(err) => (vec![failed(root, err)], false),
        }
    }

    fn file_root(root: &Path, name: &str, read: Read) -> (Vec<Msg>, bool) {
        match read {
            Read::Names(names) if names.iter().any(|n| n.to_lowercase() == name) => {
                (vec![Msg::Changed(root.to_path_buf())], true)
            }
            Read::Names(_) => (vec![], true),
            Read::Overflow => (vec![Msg::Changed(root.to_path_buf())], true),
            Read::Failed(err) => (vec![failed(root, err)], false),
        }
    }

    fn parent_of(root: &Path, base: &str, read: Read) -> (Vec<Msg>, bool) {
        match read {
            Read::Names(names) if names.iter().any(|n| n.to_lowercase() == base) && !root.exists() => {
                (vec![Msg::Gone(root.to_path_buf())], true)
            }
            Read::Overflow if !root.exists() => (vec![Msg::Gone(root.to_path_buf())], true),
            Read::Names(_) | Read::Overflow => (vec![], true),
            Read::Failed(err) => (vec![failed(root, err)], false),
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use notify::event::{CreateKind, EventKind};
    use notify::{Config, Event, RecursiveMode, Watcher as _};

    /// One inotify watcher, whose one thread notify starts: each folder root with its subfolders and its parent
    /// without them, and each file root's folder without its subfolders, kept to its name.
    pub(super) fn sources(
        roots: &[(PathBuf, bool)],
        tx: &mpsc::Sender<Msg>,
        #[cfg(test)] threads: &Arc<Mutex<Vec<u32>>>,
    ) -> io::Result<Vec<Source>> {
        let (owned, tx) = (roots.to_vec(), tx.clone());
        #[cfg(test)]
        let threads = Arc::clone(threads);
        let handler = move |res: notify::Result<Event>| {
            #[cfg(test)]
            {
                let mut t = threads.lock().unwrap();
                let tid = tid();
                if !t.contains(&tid) {
                    t.push(tid);
                }
            }
            match res {
                Err(e) => drop(tx.send(Msg::Failed(e.to_string()))),
                Ok(ev) => {
                    if ev.need_rescan() {
                        let _ = tx.send(Msg::Overflow);
                    }
                    for msg in ev.paths.iter().filter_map(|p| classify(&owned, &ev.kind, p)) {
                        let _ = tx.send(msg);
                    }
                }
            }
        };
        // links not followed, as `features` follows none
        let config = Config::default().with_follow_symlinks(false);
        let mut w = notify::INotifyWatcher::new(handler, config).map_err(io::Error::other)?;
        let folders: Vec<&PathBuf> = roots.iter().filter(|(_, file)| !file).map(|(r, _)| r).collect();
        for root in &folders {
            w.watch(root, RecursiveMode::Recursive).map_err(io::Error::other)?;
        }
        // A parent or a file root's folder inside a folder root is watched already: watching it again without its
        // subfolders would make notify drop the subfolders of the root's watch there.
        for parent in roots.iter().filter_map(|(r, _)| r.parent()) {
            if !folders.iter().any(|f| parent.starts_with(f)) {
                w.watch(parent, RecursiveMode::NonRecursive).map_err(io::Error::other)?;
            }
        }
        Ok(vec![w])
    }

    /// What an event on `p` says of the roots. Reads, which Karate makes in every run, say nothing.
    fn classify(roots: &[(PathBuf, bool)], kind: &EventKind, p: &Path) -> Option<Msg> {
        if matches!(kind, EventKind::Access(_)) {
            return None;
        }
        for (root, file) in roots {
            if p == root {
                // a file root's batch drops it if it is gone: an editor may save by deleting and writing again
                if *file {
                    return Some(Msg::Changed(root.clone()));
                }
                if !root.exists() {
                    return Some(Msg::Gone(root.clone()));
                }
                continue;
            }
            if *file {
                continue;
            }
            let Ok(rel) = p.strip_prefix(root) else { continue };
            let rel = rel.to_string_lossy();
            if matches!(kind, EventKind::Create(CreateKind::Folder)) && !skipped(&rel) {
                return Some(Msg::Walk(p.to_path_buf()));
            }
            if counts(&rel) {
                return Some(Msg::Changed(p.to_path_buf()));
            }
        }
        None
    }

    /// The calling thread's id, read without `unsafe` from `/proc/thread-self`.
    #[cfg(test)]
    pub(super) fn tid() -> u32 {
        let link = fs::read_link("/proc/thread-self").unwrap();
        link.file_name().unwrap().to_string_lossy().parse().unwrap()
    }

    /// A thread's voluntary and involuntary context switches so far.
    #[cfg(test)]
    pub(super) fn switches(tid: u32) -> u64 {
        let status = fs::read_to_string(format!("/proc/self/task/{tid}/status")).unwrap_or_default();
        status
            .lines()
            .filter(|l| l.starts_with("voluntary_ctxt_switches") || l.starts_with("nonvoluntary_ctxt_switches"))
            .filter_map(|l| l.split_whitespace().last()?.parse::<u64>().ok())
            .sum()
    }
}

#[cfg(test)]
mod tests;
