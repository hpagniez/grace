//! `src/main/fingerprint.ts`: the project fingerprint (F15): a hash of the path, size and modified time of every file
//! under the project root, to tell whether two runs saw the same project files. No file content is read. It is in
//! `run.json`, so the hex must be the one Electron Grace writes for the same files.
//!
//! The TypeScript walks asynchronously, folders in parallel, and abandons a walk stuck in a `stat` that never
//! returns. Here the walk is a plain blocking loop on a thread of its own that lives only as long as the walk; the
//! caller waits for it on a channel, up to the time bound, and does not poll (ADR 0030). The lines are sorted, so
//! the order of the walk does not matter and one folder at a time is enough.
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// Folder names, lower case, whose content is tool state, not the project, wherever they are.
pub const SKIPPED_DIRS: &[&str] = &[".git", "node_modules", ".gradle"];
/// Output folders, lower case. The fingerprint skips them only beside a build file, so a test folder of that name
/// counts; the sub-folder walk skips them everywhere.
pub const OUTPUT_DIRS: &[&str] = &["target", "build"];
const BUILD_FILES: &[&str] = &["pom.xml", "build.gradle", "build.gradle.kts"];
pub const MAX_FILES: usize = 50_000;
pub const WALK_TIMEOUT_MS: u64 = 2_000;

/// How much walking is allowed before the answer is `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_files: usize,
    pub timeout_ms: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_files: MAX_FILES,
            timeout_ms: WALK_TIMEOUT_MS,
        }
    }
}

/// `lstat`, as a seam: a test makes it fail or never return while the rest stays real.
type Lstat = dyn Fn(&Path) -> io::Result<fs::Metadata> + Send + Sync;

/// `Math.trunc(st.mtimeMs)` as Node computes `mtimeMs`: `sec * 1000 + nsec / 1e6` in a double. Near the next
/// millisecond that rounds up (`.9999999` ms is `1.0`), so whole-number arithmetic would give another number.
fn mtime_ms(time: SystemTime) -> i64 {
    let (sec, nsec) = match time.duration_since(UNIX_EPOCH) {
        Ok(since) => (since.as_secs() as i64, i64::from(since.subsec_nanos())),
        // before 1970 the seconds are negative and the nanoseconds still count up, as libuv gives them
        Err(err) => {
            let before = err.duration();
            match before.subsec_nanos() {
                0 => (-(before.as_secs() as i64), 0),
                n => (-(before.as_secs() as i64) - 1, 1_000_000_000 - i64::from(n)),
            }
        }
    };
    (sec as f64 * 1000.0 + nsec as f64 / 1e6).trunc() as i64
}

/// The sorted `<relative path>\t<size>\t<mtime ms>\n` lines of the files under `root`, or `None` past `max_files`
/// files, past the deadline or the stop flag, or on any read or stat error.
fn walk(
    root: &Path,
    max_files: usize,
    deadline: Option<Instant>,
    stop: &AtomicBool,
    lstat: &Lstat,
) -> Option<Vec<String>> {
    let within = |files: usize| {
        !(stop.load(Ordering::Relaxed) || deadline.is_some_and(|d| Instant::now() >= d) || files > max_files)
    };
    let mut lines = vec![];
    let mut files = 0;
    let mut folders = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = folders.pop() {
        if !within(files) {
            return None;
        }
        let mut entries = vec![];
        for entry in fs::read_dir(&dir).ok()? {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            entries.push((name, entry.path(), entry.file_type().ok()?));
        }
        files += entries
            .iter()
            .filter(|(_, _, kind)| !kind.is_dir() && !kind.is_symlink())
            .count();
        if !within(files) {
            return None;
        }
        let has_build_file = entries
            .iter()
            .any(|(name, _, kind)| kind.is_file() && BUILD_FILES.contains(&name.to_lowercase().as_str()));
        for (name, path, kind) in entries {
            // links and junctions are neither followed nor listed
            if kind.is_symlink() {
                continue;
            }
            let entry_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if kind.is_dir() {
                let lower = name.to_lowercase();
                let lower = lower.as_str();
                if !SKIPPED_DIRS.contains(&lower) && !(has_build_file && OUTPUT_DIRS.contains(&lower)) {
                    folders.push((path, entry_rel));
                }
                continue;
            }
            if !within(files) {
                return None;
            }
            let meta = lstat(&path).ok()?;
            let mtime = mtime_ms(meta.modified().ok()?);
            lines.push(format!("{entry_rel}\t{}\t{mtime}\n", meta.len()));
        }
    }
    Some(lines)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn fingerprint_with(root: &Path, limits: Limits, lstat: Arc<Lstat>) -> Option<String> {
    let timeout = Duration::from_millis(limits.timeout_ms);
    let deadline = Instant::now().checked_add(timeout);
    // Set once the result is known (`None` or done). A walk still running then stops before its next folder or stat;
    // one waiting on a stat that never returns is only abandoned, and ends with that stat.
    let stop = Arc::new(AtomicBool::new(false));
    let (send, receive) = mpsc::channel();
    let spawned = thread::Builder::new().name("fingerprint".to_string()).spawn({
        let (root, stop) = (root.to_path_buf(), Arc::clone(&stop));
        move || {
            let _ = send.send(walk(&root, limits.max_files, deadline, &stop, &*lstat));
        }
    });
    let walked = spawned.ok().and_then(|_| receive.recv_timeout(timeout).ok());
    stop.store(true, Ordering::Relaxed);
    let mut lines = walked??;
    // `Array.prototype.sort()` compares UTF-16 units, not code points: a name with a character above U+FFFF sorts
    // below one with a character from U+E000 to U+FFFF, which UTF-8 order puts the other way round.
    lines.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    let mut hash = Sha256::new();
    for line in &lines {
        hash.update(line.as_bytes());
    }
    Some(hex(&hash.finalize()))
}

/// The SHA-256, lower-case hex, of one `<relative path>\t<size>\t<mtime ms>\n` line per file, sorted. Symbolic
/// links and junctions are neither followed nor listed. `None` past `limits.max_files` files, past
/// `limits.timeout_ms` (even if a read or a stat never returns), or on any read or stat error. Never panics. Blocks
/// for at most the time bound: call it off the UI thread.
pub fn project_fingerprint_within(root: &Path, limits: Limits) -> Option<String> {
    fingerprint_with(root, limits, Arc::new(|path: &Path| fs::symlink_metadata(path)))
}

/// [`project_fingerprint_within`] with the default limits (50,000 files, 2 s).
pub fn project_fingerprint(root: &Path) -> Option<String> {
    project_fingerprint_within(root, Limits::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::Builder::new().prefix("kk-fp-").tempdir().unwrap()
    }

    /// Writes `files` (relative path to content) under `root`, folders included.
    fn put(root: &Path, files: &[(&str, &str)]) {
        for (rel, content) in files {
            let file = root.join(rel);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, content).unwrap();
        }
    }

    /// A root holding `a.txt` and `sub/b.txt`.
    fn project() -> TempDir {
        let root = tmp();
        put(root.path(), &[("a.txt", "abc"), ("sub/b.txt", "hello")]);
        root
    }

    fn set_mtime(file: &Path, time: SystemTime) {
        fs::OpenOptions::new()
            .write(true)
            .open(file)
            .unwrap()
            .set_modified(time)
            .unwrap();
    }

    fn is_hex64(value: &str) -> bool {
        value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }

    fn within(max_files: usize, timeout_ms: u64) -> Limits {
        Limits { max_files, timeout_ms }
    }

    mod project_fingerprint {
        use super::*;

        #[test]
        fn matches_the_specs_line_format() {
            let root = project();
            // whole milliseconds, so that counting them in whole numbers and in Node's double agree
            let base = UNIX_EPOCH + Duration::from_millis(1_700_000_000_123);
            set_mtime(&root.path().join("a.txt"), base);
            set_mtime(&root.path().join("sub/b.txt"), base + Duration::from_millis(7));
            let line = |rel: &str| {
                let meta = fs::metadata(root.path().join(rel)).unwrap();
                let ms = meta.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_millis();
                format!("{rel}\t{}\t{ms}\n", meta.len())
            };
            let expected = hex(&Sha256::digest((line("a.txt") + &line("sub/b.txt")).as_bytes()));
            let value = project_fingerprint(root.path()).unwrap();
            assert!(is_hex64(&value), "{value}");
            assert_eq!(value, expected);
            assert_eq!(project_fingerprint(root.path()).as_deref(), Some(value.as_str()));
        }

        // Not in the TypeScript tests: the hex Electron Grace's `projectFingerprint` gives for this very tree
        // (made with exact mtimes, run through `src/main/fingerprint.ts` under Node 22). It pins what the other
        // tests cannot: the double Node counts the millisecond in (`…999.9999 ms` is `…000`), the order of names
        // that UTF-16 and UTF-8 sort differently (`z～` U+FF5E, `z😀` U+1F600), non-ASCII names, tool folders and
        // `target` beside a `pom.xml` left out, and a test folder named `build` counted.
        #[test]
        fn gives_the_hex_of_electron_grace_for_the_same_files() {
            let root = tmp();
            let files: [(&str, &[u8], u64, u32); 9] = [
                ("a.txt", b"abc", 1_700_000_000, 123_456_700),
                ("sub/b.txt", b"hello", 1_700_000_000, 999_999_900),
                ("sub/z\u{ff5e}.txt", b"1", 1_700_000_001, 0),
                ("sub/z\u{1f600}.txt", b"22", 1_700_000_002, 500_000_000),
                ("\u{e9}.txt", "\u{e9}".as_bytes(), 1_700_000_003, 100),
                ("pom.xml", b"<project/>", 1_700_000_004, 0),
                ("target/x", b"ignored", 1_700_000_004, 0),
                (".git/config", b"ignored", 1_700_000_004, 0),
                ("tests/build/y.feature", b"2", 1_700_000_005, 750_000_000),
            ];
            for (rel, content, secs, nanos) in files {
                let file = root.path().join(rel);
                fs::create_dir_all(file.parent().unwrap()).unwrap();
                fs::write(&file, content).unwrap();
                set_mtime(&file, UNIX_EPOCH + Duration::new(secs, nanos));
            }
            assert_eq!(
                project_fingerprint(root.path()).as_deref(),
                Some("4ec406a4e15d0f10669db6901520697d86b04e354145f47f1ced9dd901683044")
            );
        }
    }

    /// One test per row of the TypeScript's `it.each` ("each change gives a new value: %s").
    mod each_change_gives_a_new_value {
        use super::*;

        fn changes(change: impl Fn(&Path)) {
            let root = project();
            let before = project_fingerprint(root.path());
            assert!(before.is_some());
            change(root.path());
            let after = project_fingerprint(root.path());
            assert!(after.is_some());
            assert_ne!(after, before);
        }

        #[test]
        fn a_byte_is_appended() {
            changes(|root| {
                use std::io::Write;
                let mut file = fs::OpenOptions::new().append(true).open(root.join("a.txt")).unwrap();
                file.write_all(b"d").unwrap();
            });
        }

        #[test]
        fn a_files_modified_time_moves_5_s() {
            changes(|root| {
                let file = root.join("a.txt");
                let time = fs::metadata(&file).unwrap().modified().unwrap();
                set_mtime(&file, time + Duration::from_secs(5));
            });
        }

        #[test]
        fn a_file_is_added() {
            changes(|root| put(root, &[("c.txt", "new")]));
        }

        #[test]
        fn a_file_is_removed() {
            changes(|root| fs::remove_file(root.join("sub").join("b.txt")).unwrap());
        }

        #[test]
        fn a_file_is_renamed() {
            changes(|root| fs::rename(root.join("a.txt"), root.join("z.txt")).unwrap());
        }
    }

    mod skipped_folders {
        use super::*;

        #[test]
        fn skips_tool_folders_at_any_depth_and_in_any_case() {
            let root = project();
            let before = project_fingerprint(root.path());
            put(
                root.path(),
                &[
                    (".git/x", "1"),
                    ("node_modules/x", "1"),
                    ("sub/Node_Modules/x", "1"),
                    (".GRADLE/x", "1"),
                ],
            );
            assert_eq!(project_fingerprint(root.path()), before);
            put(root.path(), &[("sub/targets/x", "1")]);
            assert_ne!(project_fingerprint(root.path()), before);
        }

        #[test]
        fn skips_target_and_build_next_to_a_maven_or_gradle_build_file_at_any_depth_and_in_any_case() {
            let root = project();
            put(
                root.path(),
                &[
                    ("pom.xml", "<project/>"),
                    ("sub/build.gradle", ""),
                    ("sub/m/POM.XML", ""),
                    ("k/Build.Gradle.kts", ""),
                ],
            );
            let before = project_fingerprint(root.path());
            put(
                root.path(),
                &[
                    ("Target/x", "1"),
                    ("sub/build/x", "1"),
                    ("sub/m/target/x", "1"),
                    ("k/BUILD/x", "1"),
                ],
            );
            assert_eq!(project_fingerprint(root.path()), before);
        }

        #[test]
        fn counts_a_test_folder_named_build_or_target() {
            let root = project();
            let before = project_fingerprint(root.path());
            put(root.path(), &[("tests/build/x.feature", "1")]);
            let with_build = project_fingerprint(root.path());
            assert_ne!(with_build, before);
            put(root.path(), &[("tests/target/x.feature", "1")]);
            let with_both = project_fingerprint(root.path());
            assert_ne!(with_both, with_build);
            // the build file is in the parent folder, not beside the folder
            put(root.path(), &[("pom.xml", "<project/>")]);
            let with_pom = project_fingerprint(root.path());
            put(root.path(), &[("tests/build/y.feature", "2")]);
            assert_ne!(project_fingerprint(root.path()), with_pom);
        }

        // A symbolic link on Linux, a junction on Windows (`mklink /J` needs no privilege, a symbolic link does).
        #[test]
        fn does_not_follow_or_list_a_junction() {
            let root = project();
            let other = tmp();
            put(other.path(), &[("f.txt", "elsewhere"), ("deep/g.txt", "elsewhere")]);
            let before = project_fingerprint(root.path());
            let link = root.path().join("link");
            #[cfg(unix)]
            std::os::unix::fs::symlink(other.path(), &link).unwrap();
            #[cfg(windows)]
            {
                let made = std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/J"])
                    .arg(&link)
                    .arg(other.path())
                    .output()
                    .unwrap();
                assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stdout));
            }
            assert!(link.exists());
            assert_eq!(project_fingerprint(root.path()), before);
        }
    }

    mod limits {
        use super::*;

        #[test]
        fn null_past_the_file_cap() {
            let root = tmp();
            put(root.path(), &[("1", ""), ("2", ""), ("3", ""), ("4", "")]);
            assert_eq!(project_fingerprint_within(root.path(), within(3, 2000)), None);
            let four = project_fingerprint_within(root.path(), within(4, 2000)).unwrap();
            assert!(is_hex64(&four), "{four}");
        }

        #[test]
        fn null_past_the_time_bound() {
            let root = project();
            assert_eq!(project_fingerprint_within(root.path(), within(50_000, 0)), None);
        }

        // The TypeScript's `lstat` mock never settles; this one blocks until the test lets it go.
        #[test]
        fn null_within_the_time_bound_when_a_stat_never_returns() {
            let root = project();
            let (release, blocked) = mpsc::channel::<()>();
            let blocked = Mutex::new(blocked);
            let lstat: Arc<Lstat> = Arc::new(move |_: &Path| {
                let _ = blocked.lock().unwrap().recv();
                Err(io::Error::other("released"))
            });
            let start = Instant::now();
            let value = fingerprint_with(root.path(), within(50_000, 50), lstat);
            assert_eq!(value, None);
            assert!(start.elapsed() < Duration::from_secs(1));
            drop(release);
        }

        #[test]
        fn null_for_a_missing_root() {
            let root = tmp();
            assert_eq!(project_fingerprint(&root.path().join("missing")), None);
        }

        #[test]
        fn null_when_a_stat_fails() {
            let root = project();
            let lstat: Arc<Lstat> = Arc::new(|_: &Path| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
            assert_eq!(fingerprint_with(root.path(), Limits::default(), lstat), None);
        }
    }

    // Not in the TypeScript tests: the double Node counts a time in.
    mod mtime_ms {
        use super::*;

        fn at(secs: u64, nanos: u32) -> SystemTime {
            UNIX_EPOCH + Duration::new(secs, nanos)
        }

        #[test]
        fn truncates_to_the_millisecond() {
            assert_eq!(mtime_ms(at(1_700_000_000, 123_456_789)), 1_700_000_000_123);
            assert_eq!(mtime_ms(at(1_700_000_000, 0)), 1_700_000_000_000);
            assert_eq!(mtime_ms(UNIX_EPOCH), 0);
        }

        #[test]
        fn rounds_up_where_node_does() {
            // 1700000000.9999999 s is not a double of its own: Node counts 1700000001000 ms
            assert_eq!(mtime_ms(at(1_700_000_000, 999_999_900)), 1_700_000_001_000);
            // a hundred microseconds short of the next millisecond is still in this one
            assert_eq!(mtime_ms(at(1_700_000_000, 999_900_000)), 1_700_000_000_999);
        }

        #[test]
        fn truncates_towards_zero_before_1970() {
            assert_eq!(mtime_ms(UNIX_EPOCH - Duration::new(1, 500_000_000)), -1500);
            assert_eq!(mtime_ms(UNIX_EPOCH - Duration::new(2, 0)), -2000);
            assert_eq!(mtime_ms(UNIX_EPOCH - Duration::from_micros(500)), 0);
        }
    }

    // Not in the TypeScript tests.
    #[test]
    fn sorts_the_lines_by_utf16_units() {
        let mut names = ["z\u{1f600}", "z\u{ff5e}", "z\u{e9}", "za"];
        names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
        assert_eq!(names, ["za", "z\u{e9}", "z\u{1f600}", "z\u{ff5e}"]);
        let mut by_code_point = names;
        by_code_point.sort();
        assert_eq!(by_code_point, ["za", "z\u{e9}", "z\u{ff5e}", "z\u{1f600}"]);
    }
}
