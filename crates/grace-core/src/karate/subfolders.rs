//! `src/main/karate/subfolders.ts`: the folders below an opened folder with no test that look like a Karate project
//! (docs/specs/2026-11-04-subfolder-offer.md R2).
//!
//! The walk is a plain blocking loop on a thread of its own, which lives only as long as the walk and sends each folder
//! as it finds it. The caller waits for it on a channel, up to the time bound, and does not poll (ADR 0030): a read
//! that never returns costs the offer only what the walk had not found yet.
use crate::fingerprint::{BUILD_FILES, OUTPUT_DIRS, SKIPPED_DIRS};
use crate::model::js::locale_compare;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Files that make a folder a Karate project on their own.
const MARKERS: &[&str] = &["karate-pom.json", "karate-config.js"];

/// How far the walk looks before the answer is what it has found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Levels below the project's folder.
    pub depth: usize,
    pub found: usize,
    /// Folders read.
    pub folders: usize,
    pub timeout_ms: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            depth: 3,
            found: 10,
            folders: 2_000,
            timeout_ms: 2_000,
        }
    }
}

/// A folder's entry. A link or a junction is neither a file nor a folder, as `Dirent` says of it.
struct Entry {
    name: String,
    file: bool,
    dir: bool,
}

/// A folder's entries, as a seam: a test makes a read hang, or reads a folder as a link.
type Read = dyn Fn(&Path) -> Vec<Entry> + Send + Sync;

/// The entries of `dir`; none when it cannot be read, is gone or is not a folder.
fn read_dir(dir: &Path) -> Vec<Entry> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let kind = entry.file_type().ok()?;
            Some(Entry {
                name: entry.file_name().to_string_lossy().into_owned(),
                file: kind.is_file(),
                dir: kind.is_dir(),
            })
        })
        .collect()
}

/// `karateFolders`: the folders 1 to 3 levels below `root` that hold `karate-pom.json` or `karate-config.js`, or a
/// build file beside a `src/test` with a `.feature` under it; relative with `/`, by level then path, never looked
/// inside. What it found once a limit is reached, after 2 s at most.
pub fn karate_folders(root: &Path) -> Vec<String> {
    walk(root, Limits::default(), Arc::new(read_dir))
}

fn walk(root: &Path, limits: Limits, read: Arc<Read>) -> Vec<String> {
    let deadline = Instant::now() + Duration::from_millis(limits.timeout_ms);
    let (found, offers) = mpsc::channel();
    let root = root.to_path_buf();
    let started = thread::Builder::new().name("grace-subfolders".into()).spawn(move || {
        Walk {
            read,
            limits,
            deadline,
            reads: 0,
        }
        .run(&root, &found);
    });
    if started.is_err() {
        return Vec::new();
    }
    // ends when the walk does, or at the deadline: a walk still waiting on a read stops at its next one
    std::iter::from_fn(|| {
        offers
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .ok()
    })
    .collect()
}

fn lower(e: &Entry) -> String {
    e.name.to_lowercase()
}

fn has_file(entries: &[Entry], names: &[&str]) -> bool {
    entries.iter().any(|e| e.file && names.contains(&lower(e).as_str()))
}

/// The sub-folders to look in, by name.
fn folders(entries: &[Entry]) -> Vec<&str> {
    let mut names: Vec<&str> = entries
        .iter()
        .filter(|e| {
            let name = lower(e);
            e.dir && !SKIPPED_DIRS.contains(&name.as_str()) && !OUTPUT_DIRS.contains(&name.as_str())
        })
        .map(|e| e.name.as_str())
        .collect();
    names.sort_by(|a, b| locale_compare(a, b));
    names
}

/// One walk. Each step answers `None` once a limit is reached, which ends it.
struct Walk {
    read: Arc<Read>,
    limits: Limits,
    deadline: Instant,
    reads: usize,
}

impl Walk {
    fn read(&mut self, dir: &Path) -> Option<Vec<Entry>> {
        if self.reads >= self.limits.folders || Instant::now() >= self.deadline {
            return None;
        }
        self.reads += 1;
        Some((self.read)(dir))
    }

    /// Whether a `.feature` file is anywhere under `dir`.
    fn has_feature(&mut self, dir: PathBuf) -> Option<bool> {
        // a stack, not recursion: the next one popped is the first by name
        let mut left = vec![dir];
        while let Some(dir) = left.pop() {
            let entries = self.read(&dir)?;
            if entries.iter().any(|e| e.file && lower(e).ends_with(".feature")) {
                return Some(true);
            }
            left.extend(folders(&entries).into_iter().rev().map(|name| dir.join(name)));
        }
        Some(false)
    }

    fn is_project(&mut self, dir: &Path, entries: &[Entry]) -> Option<bool> {
        if has_file(entries, MARKERS) {
            return Some(true);
        }
        let Some(src) = entries.iter().find(|e| e.dir && lower(e) == "src") else {
            return Some(false);
        };
        if !has_file(entries, BUILD_FILES) {
            return Some(false);
        }
        let src = dir.join(&src.name);
        match self.read(&src)?.iter().find(|e| e.dir && lower(e) == "test") {
            Some(test) => self.has_feature(src.join(&test.name)),
            None => Some(false),
        }
    }

    /// Sends each project folder below `root`, level by level.
    fn run(&mut self, root: &Path, found: &mpsc::Sender<String>) -> Option<()> {
        let mut count = 0;
        let mut level = vec![String::new()];
        for depth in 0..=self.limits.depth {
            let mut next = Vec::new();
            for rel in level {
                let dir = root.join(&rel);
                let entries = self.read(&dir)?;
                if depth > 0 && self.is_project(&dir, &entries)? {
                    found.send(rel).ok()?;
                    count += 1;
                    if count >= self.limits.found {
                        return None;
                    }
                } else if depth < self.limits.depth {
                    // a build's own sources are that project's, not another one (#232 review)
                    let build = has_file(&entries, BUILD_FILES);
                    for name in folders(&entries) {
                        if !(build && name.to_lowercase() == "src") {
                            next.push(if rel.is_empty() {
                                name.to_string()
                            } else {
                                format!("{rel}/{name}")
                            });
                        }
                    }
                }
            }
            level = next;
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    /// A fresh folder holding `files` (relative paths); a path ending in `/` is a folder.
    fn write(files: &[&str]) -> TempDir {
        let root = tempfile::Builder::new().prefix("kk-sub-").tempdir().unwrap();
        for f in files {
            let path = root.path().join(f);
            if f.ends_with('/') {
                fs::create_dir_all(path).unwrap();
            } else {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, "").unwrap();
            }
        }
        root
    }

    fn offered(files: &[&str]) -> Vec<String> {
        karate_folders(write(files).path())
    }

    #[test]
    fn offers_each_kind_of_karate_project() {
        let found = offered(&[
            "api/karate-pom.json",
            "cfg/karate-config.js",
            "g/build.gradle",
            "g/src/test/java/a/b.feature",
            "k/build.gradle.kts",
            "k/src/test/x.feature",
            "m/pom.xml",
            "m/src/test/resources/c.feature",
        ]);
        assert_eq!(found, ["api", "cfg", "g", "k", "m"]);
    }

    #[test]
    fn skips_a_build_file_whose_src_test_has_no_feature() {
        let found = offered(&[
            "tool/pom.xml",
            "tool/src/test/java/Helper.java",
            "t2/pom.xml",
            "t2/src/test/target/x.feature",
        ]);
        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn looks_3_levels_down() {
        assert_eq!(offered(&["a/b/c/karate-config.js"]), ["a/b/c"]);
        assert_eq!(offered(&["a/b/c/d/karate-config.js"]), Vec::<String>::new());
    }

    #[test]
    fn skips_tool_and_output_folders_in_any_case() {
        let found = offered(&[
            "node_modules/x/karate-config.js",
            "NODE_MODULES/y/karate-config.js",
            "target/test-classes/karate-config.js",
            "build/z/karate-config.js",
            ".git/w/karate-config.js",
            ".gradle/v/karate-config.js",
        ]);
        assert_eq!(found, Vec::<String>::new());
    }

    #[test]
    fn never_offers_the_project_folder_and_does_not_look_inside_an_offered_one() {
        let found = offered(&[
            "karate-config.js",
            "karate/karate-config.js",
            "karate/nested/karate-pom.json",
        ]);
        assert_eq!(found, ["karate"]);
    }

    #[test]
    fn does_not_offer_the_sources_of_a_folder_with_a_build_file() {
        assert_eq!(
            offered(&["pom.xml", "src/test/java/karate-config.js"]),
            Vec::<String>::new()
        );
        assert_eq!(
            offered(&["app/build.gradle", "app/src/test/resources/karate-config.js"]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn ignores_a_marker_that_is_a_folder() {
        assert_eq!(
            offered(&["x/karate-config.js/", "y/pom.xml", "y/src/test"]),
            Vec::<String>::new()
        );
    }

    /// No test here may make a junction or a link, so this reads one the system already has (`Documents and Settings`
    /// on Windows, `/proc/self` on Linux), then walks a folder read as that one is.
    #[test]
    fn follows_no_junction() {
        let (parent, name) = if cfg!(windows) {
            let drive = std::env::var("SystemDrive").unwrap();
            (format!("{drive}\\"), "Documents and Settings")
        } else {
            ("/proc".to_string(), "self")
        };
        let link = read_dir(Path::new(&parent))
            .into_iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no {name} in {parent}"));
        assert!(!link.file && !link.dir, "a link is neither a file nor a folder");

        let root = write(&["linked/karate-config.js"]);
        let as_a_link: Arc<Read> = Arc::new(|dir: &Path| {
            let mut entries = read_dir(dir);
            for e in entries.iter_mut().filter(|e| e.name == "linked") {
                e.dir = false;
            }
            entries
        });
        assert_eq!(walk(root.path(), Limits::default(), as_a_link), Vec::<String>::new());
        assert_eq!(karate_folders(root.path()), ["linked"], "offered as a folder");
    }

    #[test]
    fn orders_by_level_then_by_path() {
        let found = offered(&["z/karate-config.js", "a/x/karate-config.js", "b/karate-pom.json"]);
        assert_eq!(found, ["b", "z", "a/x"]);
    }

    /// Not in the TypeScript: `localeCompare` ignores case first, where Rust's order puts `B` before `a`.
    #[test]
    fn orders_names_as_locale_compare_does() {
        let found = offered(&["B/karate-config.js", "a/karate-config.js", "C/karate-config.js"]);
        assert_eq!(found, ["a", "B", "C"]);
    }

    #[test]
    fn stops_at_the_limits() {
        let three = write(&["a/karate-config.js", "b/karate-config.js", "c/karate-config.js"]);
        let within = |limits: Limits| walk(three.path(), limits, Arc::new(read_dir));
        let all = Limits::default();
        assert_eq!(within(Limits { found: 2, ..all }), ["a", "b"]);
        assert_eq!(within(Limits { folders: 1, ..all }), Vec::<String>::new());
        assert_eq!(within(Limits { timeout_ms: 0, ..all }), Vec::<String>::new());
        assert_eq!(karate_folders(&three.path().join("missing")), Vec::<String>::new());
    }

    /// The TypeScript's race with its timer: the answer does not wait for a read that hangs.
    #[test]
    fn answers_with_what_it_found_when_a_read_never_returns() {
        let root = write(&["a/karate-config.js", "b/other.txt"]);
        let (release, held) = mpsc::channel::<()>();
        let held = Mutex::new(held);
        let hangs: Arc<Read> = Arc::new(move |dir: &Path| {
            if dir.ends_with("b") {
                let _ = held.lock().unwrap().recv(); // until the test drops its end
            }
            read_dir(dir)
        });
        let limits = Limits {
            timeout_ms: 300,
            ..Limits::default()
        };
        let started = Instant::now();
        assert_eq!(walk(root.path(), limits, hangs), ["a"]);
        assert!(started.elapsed() < Duration::from_secs(10));
        drop(release);
    }
}
