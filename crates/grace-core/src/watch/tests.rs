// watch.test.ts test for test, then what the TypeScript could not test: each platform's events and the idle cost.
// File events are timing tests: each waits for what it expects with a generous limit, and a test that expects
// nothing writes a feature after what must not count and waits for a batch of that feature alone.
use super::*;
use std::fs::{self, File};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

const QUIET_TEST: Duration = Duration::from_millis(150);
const LIMIT: Duration = Duration::from_secs(10);

/// `<temp>/grace-watch-*/root`, so a test can rename the root's parent.
fn tmp() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::Builder::new().prefix("grace-watch-").tempdir().unwrap();
    let root = dir.path().join("root");
    fs::create_dir(&root).unwrap();
    (dir, root)
}

fn write(file: &Path, text: &str) {
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, text).unwrap();
}

fn feature(file: &Path) {
    write(file, "Feature: x\n");
}

struct Started {
    w: Watcher,
    saved: Receiver<Vec<PathBuf>>,
    errors: Receiver<String>,
}

fn start(roots: &[PathBuf]) -> Started {
    start_quiet(roots, QUIET_TEST)
}

/// For a test whose two writes must fall in one window even on a loaded machine.
const QUIET_LONG: Duration = Duration::from_secs(1);

fn start_quiet(roots: &[PathBuf], quiet: Duration) -> Started {
    let (saved_tx, saved) = mpsc::channel();
    let (errors_tx, errors) = mpsc::channel();
    let w = watch_features(
        roots,
        move |files| saved_tx.send(files).unwrap(),
        move |reason| errors_tx.send(reason).unwrap(),
        quiet,
    )
    .unwrap();
    Started { w, saved, errors }
}

fn sorted(mut files: Vec<PathBuf>) -> Vec<PathBuf> {
    files.sort();
    files
}

fn next(saved: &Receiver<Vec<PathBuf>>) -> Vec<PathBuf> {
    sorted(saved.recv_timeout(LIMIT).expect("a batch"))
}

fn set_modified(file: &Path, at: SystemTime) {
    File::options()
        .write(true)
        .open(file)
        .unwrap()
        .set_modified(at)
        .unwrap();
}

fn set_readonly(file: &Path, readonly: bool) {
    let mut p = fs::metadata(file).unwrap().permissions();
    p.set_readonly(readonly);
    fs::set_permissions(file, p).unwrap();
}

mod counts {
    use super::*;

    #[test]
    fn takes_a_feature_in_any_case_and_folder() {
        for p in ["a.feature", "x\\B.FEATURE", "x/y/c.feature"] {
            assert!(counts(p), "{p}");
        }
    }

    #[test]
    fn leaves_other_files_and_the_skipped_folders() {
        for p in [
            "karate-config.js",
            "a.feature.bak",
            "node_modules\\x\\c.feature",
            "x\\.git\\a.feature",
            ".gradle/b.feature",
            "NODE_MODULES/c.feature",
        ] {
            assert!(!counts(p), "{p}");
        }
    }
}

mod watch_features {
    use super::*;

    #[test]
    fn hands_a_saved_feature_over_once_as_an_absolute_path() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        let a = root.join("sub").join("a.feature");
        feature(&a);
        assert_eq!(next(&s.saved), [a]);
        let b = root.join("b.feature");
        feature(&b);
        assert_eq!(next(&s.saved), [b]);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn makes_two_saves_within_the_quiet_time_one_batch() {
        let (dir, root) = tmp();
        let s = start_quiet(std::slice::from_ref(&root), QUIET_LONG);
        let (a, b) = (root.join("a.feature"), root.join("b.feature"));
        feature(&a);
        thread::sleep(Duration::from_millis(20));
        feature(&b);
        assert_eq!(next(&s.saved), [a, b]);
        drop(s);
        dir.close().unwrap();
    }

    /// Added: one quiet window for all roots (R2.3), so a "Save all" across roots is one batch.
    #[test]
    fn makes_saves_in_two_roots_one_batch() {
        let (dir, root) = tmp();
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        let s = start_quiet(&[root.clone(), other.clone()], QUIET_LONG);
        let (a, b) = (root.join("a.feature"), other.join("b.feature"));
        feature(&a);
        feature(&b);
        assert_eq!(next(&s.saved), sorted(vec![a, b]));
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn does_not_count_other_files() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        write(&root.join("karate-config.js"), "x");
        write(&root.join("data.json"), "x");
        feature(&root.join("node_modules").join("x").join("c.feature"));
        let sentinel = root.join("sentinel.feature");
        feature(&sentinel);
        assert_eq!(next(&s.saved), [sentinel]);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn drops_a_feature_deleted_before_its_batch() {
        let (dir, root) = tmp();
        let s = start_quiet(std::slice::from_ref(&root), QUIET_LONG);
        let a = root.join("a.feature");
        feature(&a);
        fs::remove_file(&a).unwrap();
        let sentinel = root.join("sentinel.feature");
        feature(&sentinel);
        assert_eq!(next(&s.saved), [sentinel]);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn does_not_count_an_event_with_no_new_modified_time() {
        let (dir, root) = tmp();
        let old = root.join("old.feature");
        feature(&old);
        set_modified(&old, SystemTime::now() - Duration::from_secs(3600)); // saved an hour before the start
        let s = start(std::slice::from_ref(&root));
        set_readonly(&old, true); // an attribute event, the modified time unchanged
        let a = root.join("a.feature");
        feature(&a);
        assert_eq!(next(&s.saved), std::slice::from_ref(&a));
        set_readonly(&a, true);
        let sentinel = root.join("sentinel.feature");
        feature(&sentinel);
        assert_eq!(next(&s.saved), [sentinel]);
        drop(s);
        set_readonly(&old, false); // so the folder can be deleted
        set_readonly(&a, false);
        dir.close().unwrap();
    }

    #[test]
    fn counts_a_save_stamped_a_moment_before_the_start_windows_stamps_files_from_a_coarse_clock() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        let a = root.join("a.feature");
        feature(&a);
        set_modified(&a, SystemTime::now() - Duration::from_millis(500));
        assert_eq!(next(&s.saved), [a]);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn watches_a_root_that_is_a_file() {
        let (dir, root) = tmp();
        let file = root.join("one.feature");
        feature(&file);
        thread::sleep(Duration::from_millis(50));
        let s = start(std::slice::from_ref(&file));
        write(&file, "Feature: changed\n");
        assert_eq!(next(&s.saved), [file]);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn throws_for_a_root_it_cannot_watch() {
        let (dir, root) = tmp();
        let missing = root.join("missing");
        assert!(watch_features(&[root, missing], |_| {}, |_| {}, QUIET).is_err());
        dir.close().unwrap();
    }

    /// The error, then nothing: the quiet thread has ended, so the batch's channel is closed.
    fn one_error_then_stopped(s: &Started) {
        let reason = s.errors.recv_timeout(LIMIT).expect("an error");
        assert!(reason.ends_with("was removed or renamed"), "{reason}");
        assert_eq!(s.saved.recv_timeout(LIMIT), Err(RecvTimeoutError::Disconnected));
        assert!(s.errors.recv_timeout(LIMIT).is_err());
    }

    #[test]
    fn reports_one_error_and_stops_when_a_watched_root_is_deleted() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        fs::remove_dir_all(&root).unwrap();
        one_error_then_stopped(&s);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn reports_one_error_when_a_watched_root_is_renamed() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        fs::rename(&root, root.with_file_name("root-renamed")).unwrap();
        one_error_then_stopped(&s);
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn reports_one_error_when_a_watched_root_is_moved_away() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        let gone = PathBuf::from(format!("{}-gone", dir.path().display()));
        fs::rename(&root, &gone).unwrap();
        one_error_then_stopped(&s);
        drop(s);
        fs::remove_dir_all(gone).unwrap();
        dir.close().unwrap();
    }

    #[test]
    fn hands_over_every_feature_of_a_burst_the_os_cannot_report_whole() {
        let (dir, root) = tmp();
        let s = start(std::slice::from_ref(&root));
        let files: Vec<PathBuf> = (0..300).map(|i| root.join("m").join(format!("f{i}.feature"))).collect();
        for f in &files {
            feature(f); // at once: Windows' change buffer overflows
        }
        let mut got = Vec::new();
        let end = Instant::now() + LIMIT;
        while got.len() < files.len() {
            let left = end.saturating_duration_since(Instant::now());
            got.extend(s.saved.recv_timeout(left).expect("the rest of the burst"));
        }
        assert_eq!(sorted(got), sorted(files));
        drop(s);
        dir.close().unwrap();
    }

    #[test]
    fn stops_on_close_and_frees_the_roots_parent_folder() {
        let (dir, root) = tmp();
        let mut s = start(std::slice::from_ref(&root));
        s.w.close();
        s.w.close();
        feature(&root.join("a.feature"));
        assert_eq!(s.saved.recv_timeout(LIMIT), Err(RecvTimeoutError::Disconnected));
        let parent = dir.path().to_path_buf();
        let moved = PathBuf::from(format!("{}-moved", parent.display()));
        fs::rename(&parent, &moved).unwrap();
        fs::rename(&moved, &parent).unwrap();
        drop(s);
        dir.close().unwrap();
    }
}

/// The batch alone, with the messages a platform would send: what no file system can be made to do on demand.
mod batch {
    use super::*;

    #[test]
    fn an_overflow_hands_over_every_feature_of_the_roots_whose_time_moved() {
        let (dir, root) = tmp();
        let (file_root, old_file_root) = (dir.path().join("one.feature"), dir.path().join("two.feature"));
        feature(&file_root);
        feature(&old_file_root);
        let old = root.join("old.feature");
        feature(&old);
        let hour_ago = SystemTime::now() - Duration::from_secs(3600);
        set_modified(&old, hour_ago);
        set_modified(&old_file_root, hour_ago);
        let roots = [(root.clone(), false), (file_root.clone(), true), (old_file_root, true)];
        let mut batch = Batch::new(&roots);
        let (a, b) = (root.join("a.feature"), root.join("x").join("b.feature"));
        feature(&a);
        feature(&b);
        feature(&root.join("node_modules").join("c.feature"));
        batch.take(Msg::Overflow);
        // a file root too: Linux has one event queue for all roots
        assert_eq!(sorted(batch.flush()), sorted(vec![a, b, file_root]));
        dir.close().unwrap();
    }

    #[test]
    fn walks_a_folder_made_after_the_start() {
        let (dir, root) = tmp();
        let mut batch = Batch::new(&[(root.clone(), false)]);
        let a = root.join("new").join("deep").join("a.feature");
        feature(&a);
        batch.take(Msg::Walk(root.join("new")));
        assert_eq!(batch.flush(), [a]);
        dir.close().unwrap();
    }
}

/// With no file changing for 2 s, the watcher's threads run not once (ADR 0030): Windows counts each thread's CPU
/// cycles, Linux its context switches, which a thread blocked in the OS makes none of.
#[test]
fn its_threads_use_no_cpu_while_no_file_changes() {
    let (dir, root) = tmp();
    let file = dir.path().join("one.feature");
    feature(&file);
    let s = start(&[root.clone(), file]);
    // one event, so every thread has run once (Linux learns notify's thread from it), then a quiet batch
    feature(&root.join("a.feature"));
    assert_eq!(next(&s.saved), [root.join("a.feature")]);
    // settled once two readings 100 ms apart agree; a thread that never settles is the failure
    let mut before = s.w.cpu();
    let end = Instant::now() + LIMIT;
    loop {
        thread::sleep(Duration::from_millis(100));
        let now = s.w.cpu();
        if now == before {
            break;
        }
        assert!(
            Instant::now() < end,
            "the threads never settled: {before:?} then {now:?}"
        );
        before = now;
    }
    assert!(before.len() >= if cfg!(windows) { 4 } else { 2 }, "{before:?}");
    thread::sleep(Duration::from_secs(2));
    assert_eq!(s.w.cpu(), before);
    drop(s);
    dir.close().unwrap();
}

#[cfg(windows)]
mod windows {
    use super::*;

    /// One `FILE_NOTIFY_INFORMATION`: the next entry's offset, the action, the name's length in bytes, the name.
    fn entry(next: u32, name: &str, len: Option<u32>) -> Vec<u8> {
        let units: Vec<u16> = name.encode_utf16().collect();
        let mut b = Vec::new();
        b.extend(next.to_le_bytes());
        b.extend(1u32.to_le_bytes());
        b.extend(len.unwrap_or(units.len() as u32 * 2).to_le_bytes());
        for u in units {
            b.extend(u.to_le_bytes());
        }
        while b.len() % 4 != 0 {
            b.push(0);
        }
        b
    }

    #[test]
    fn walks_a_buffer_made_by_hand() {
        let first = entry(0, "a.feature", None);
        let next = first.len() as u32;
        let mut buf = entry(next, "a.feature", None);
        let second = entry(0, "dossier\\été.feature", None);
        buf.extend(&second);
        assert_eq!(
            entries(&buf),
            (vec!["a.feature".into(), "dossier\\été.feature".into()], true)
        );

        // a second entry that says its name runs past the end: the first is kept, and the walk is not whole
        let mut cut = entry(next, "a.feature", None);
        cut.extend(entry(0, "b.feature", Some(400)));
        assert_eq!(entries(&cut), (vec!["a.feature".into()], false));
        // an entry cut inside its header, an odd name length, and an offset past the end
        assert_eq!(entries(&buf[..next as usize + 6]), (vec!["a.feature".into()], false));
        assert_eq!(entries(&entry(0, "a", Some(1))), (vec![], false));
        assert_eq!(
            entries(&entry(4000, "a.feature", None)),
            (vec!["a.feature".into()], false)
        );
        assert_eq!(entries(&[]), (vec![], false));
    }

    #[test]
    fn reads_a_completion_of_0_bytes_and_error_notify_enum_dir_as_an_overflow() {
        assert!(matches!(completion(Ok(&[])), Read::Overflow));
        assert!(matches!(completion(Err(1022)), Read::Overflow)); // ERROR_NOTIFY_ENUM_DIR
        let mut cut = entry(40, "a.feature", None);
        cut.extend([0; 6]);
        assert!(matches!(completion(Ok(&cut)), Read::Overflow));
        assert!(matches!(completion(Ok(&entry(0, "a.feature", None))), Read::Names(n) if n == ["a.feature"]));
        assert!(matches!(completion(Err(5)), Read::Failed(e) if e.raw_os_error() == Some(5)));
    }

    /// What Windows really says when its buffer fills while the reader is busy: the burst test's reader keeps up.
    #[test]
    fn a_burst_while_the_reader_is_busy_reads_as_an_overflow() {
        let (dir, root) = tmp();
        let (kinds_tx, kinds) = mpsc::channel();
        let (hold_tx, hold) = mpsc::channel::<()>();
        let reader = sys::Reader::start(&root, true, FOLDER, move |read| {
            let first = kinds_tx.send(matches!(read, Read::Overflow)).is_ok();
            let _ = hold.recv(); // busy until the test lets it go
            first
        })
        .unwrap();
        // declared after `reader`, so a failed assert drops it first and the reader's thread is let go
        let hold_tx = hold_tx;
        feature(&root.join("first.feature"));
        assert!(!kinds.recv_timeout(LIMIT).unwrap());
        for i in 0..300 {
            feature(&root.join(format!("f{i}.feature")));
        }
        drop(hold_tx); // every later `hold.recv()` returns at once
        assert!(
            (0..50)
                .map_while(|_| kinds.recv_timeout(LIMIT).ok())
                .any(|overflow| overflow),
            "no overflow"
        );
        drop(reader);
        dir.close().unwrap();
    }

    /// A reader of `dir` whose thread waits for `hold` before it issues its first read.
    fn held(dir: &Path) -> (sys::Reader, mpsc::Sender<()>, Receiver<()>) {
        let (hold_tx, hold) = mpsc::channel::<()>();
        let (reader, issued) = sys::Reader::spawn(
            dir,
            true,
            FOLDER,
            |_| true,
            move || {
                let _ = hold.recv();
            },
        )
        .unwrap();
        (reader, hold_tx, issued)
    }

    #[test]
    fn close_before_the_thread_has_issued_its_read_returns_at_once() {
        let (dir, root) = tmp();
        let (reader, hold, issued) = held(&root);
        reader.signal(); // the close lands first; the read is issued after it
        hold.send(()).unwrap();
        issued.recv_timeout(LIMIT).unwrap();
        let t = Instant::now();
        drop(reader);
        assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
        dir.close().unwrap();
    }

    #[test]
    fn close_while_it_waits_returns_at_once() {
        let (dir, root) = tmp();
        let (reader, hold, issued) = held(&root);
        hold.send(()).unwrap();
        issued.recv_timeout(LIMIT).unwrap(); // the thread now waits on its read and the stop event
        let t = Instant::now();
        drop(reader);
        assert!(t.elapsed() < Duration::from_secs(1), "{:?}", t.elapsed());
        dir.close().unwrap();
    }
}
