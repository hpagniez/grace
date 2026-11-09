//! `src/main/karate/command.ts`: the Karate command line (migration guide R1.5). A project's layout and what Grace
//! takes from its `karate-pom.json`, the run targets of a selection, the `-t` arguments of the tag filter, Java's
//! arguments, the environment Java gets, and Java's out-of-memory line. The strings are Electron Grace's, byte for
//! byte; the tests are `command.test.ts`'s, on the same expected strings.
use super::debug::DEBUG_LISTENER;
use super::pom::read_pom;
use crate::model::env::{PlainRow, STRIPPED_ENV_VARS};
use crate::model::tags::{FilterState, filter_on, matching_leaves, no_filter};
use crate::model::{Layout, NodeKind, TestNode};
use indexmap::IndexMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, MAIN_SEPARATOR_STR, Path, PathBuf};

/// Where a project's files are, as Grace runs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectLayout {
    pub layout: Layout,
    /// The source folders, then the build's compiled output folders that exist.
    pub classpath: Vec<PathBuf>,
    /// The karate-pom.json `paths` kept, or the source folders.
    pub feature_roots: Vec<PathBuf>,
    /// Karate's `-g`, from karate-pom.json, as written.
    pub config_dir: Option<String>,
    /// Where `karate-config-<env>.js` files are read from; never a compiled folder, whose copies may be stale.
    pub config_dirs: Vec<PathBuf>,
    /// What the open notice says about karate-pom.json; None when there is nothing to say.
    pub pom_notice: Option<String>,
    /// karate-pom.json's `output.junitXml`: runs also ask Karate for JUnit XML (docs/specs/2026-11-03-export-reports.md
    /// R1).
    pub junit_xml: bool,
    /// Where Karate's names of the tree's files resolve, for `locate`: Karate's root, then the project.
    pub bases: Vec<PathBuf>,
}

const SOURCE: [&str; 2] = ["src/test/java", "src/test/resources"];

fn compiled(layout: Layout) -> &'static [&'static str] {
    match layout {
        Layout::Maven => &["target/test-classes", "target/classes"],
        Layout::Gradle => &[
            "build/classes/java/test",
            "build/resources/test",
            "build/classes/java/main",
            "build/resources/main",
        ],
        Layout::Standalone => &[],
    }
}

/// Node's `path.normalize` for the paths Grace builds: `.` dropped, `..` taking back the name before it (never past
/// the root), one separator between names, the platform's. Lexical: links are not followed.
pub(crate) fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    // the names a `..` may take back: not a `..` kept at the start of a relative path
    let mut names = 0usize;
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir if names > 0 => {
                out.pop();
                names -= 1;
            }
            Component::ParentDir if out.has_root() => {}
            Component::ParentDir => out.push(".."),
            Component::Normal(name) => {
                out.push(name);
                names += 1;
            }
            Component::Prefix(_) | Component::RootDir => out.push(c.as_os_str()),
        }
    }
    out
}

/// Node's `path.join(base, rel)`: `rel` appended whatever it starts with (`/conf` is not an absolute path here), its
/// `/` a separator on every platform, then normalized.
pub(crate) fn join(base: &Path, rel: &str) -> PathBuf {
    let mut joined = base.as_os_str().to_os_string();
    joined.push(MAIN_SEPARATOR_STR);
    joined.push(rel);
    normalize(Path::new(&joined))
}

/// The path made absolute against the working directory, as Node's `path.resolve` does, and normalized.
pub(crate) fn absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return normalize(path);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    normalize(&cwd.join(path))
}

/// Node's `path.resolve(root, p)`: `p` when it is absolute, else `p` under `root`.
pub(crate) fn resolve(root: &Path, p: &str) -> PathBuf {
    absolute(&root.join(p))
}

/// The text of a path on the command line. Grace keeps project folders as JSON strings, so a path that is not Unicode
/// never gets here; it would be written lossily.
fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// Two names of a path, compared as the platform's file system does: without regard to case on Windows, as Node's
/// `path.win32.relative` compares (it lower-cases both).
fn same_name(a: Component, b: Component) -> bool {
    if cfg!(windows) {
        a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// True for the project folder and anything under it, by the names alone; a folder named `..data` is under it.
// Rust-native: the TypeScript asks `path.relative` and looks for a leading `..`; this compares the names of the two
// normalized paths, which is the same question without building the relative path. Parity: `keeps_a_folder_whose_name_
// starts_with_dots` and `keeps_the_paths_inside_the_project_and_names_the_others`.
pub fn inside(root: &Path, abs: &Path) -> bool {
    let root = absolute(root);
    let abs = absolute(abs);
    let mut names = abs.components();
    root.components().all(|r| names.next().is_some_and(|a| same_name(r, a)))
}

/// The folder Karate names files relative to, as its `Suite.computeRoot` finds it: a configDir that is not on the
/// classpath, else the folder of the first `karate-config.js` (or configDir file) on the classpath, else the project.
/// shortcut: a library folder later on the classpath is not checked, as no tree file is under one; check them if one
/// can be.
fn karate_root(root: &Path, classpath: &[PathBuf], config_dir: Option<&str>) -> PathBuf {
    if let Some(dir) = config_dir.filter(|d| !d.starts_with("classpath:")) {
        let resolved = resolve(root, dir.strip_prefix("file:").unwrap_or(dir));
        return match resolved.parent() {
            Some(parent) if dir.ends_with(".js") => parent.to_path_buf(),
            _ => resolved,
        };
    }
    let rel = config_dir.map_or("", |d| &d["classpath:".len()..]);
    // `path.join(rel, "karate-config.js")`; joined to a folder below, a leading `/` of either is only a separator
    let config = if rel.ends_with(".js") {
        rel.to_string()
    } else {
        format!("{rel}/karate-config.js")
    };
    classpath
        .iter()
        .map(|d| join(d, &config))
        .find(|f| f.exists())
        .and_then(|f| f.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| root.to_path_buf())
}

/// The paths without repeats, in their order: `[...new Set(paths)]`.
fn unique(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for p in paths {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

/// All paths absolute. Maven/Gradle: whichever of src/test/java and src/test/resources exist. The project's
/// karate-pom.json gives the feature roots and the config folder (docs/specs/2026-11-02-project-setup.md).
pub fn detect_layout(root: &Path) -> ProjectLayout {
    let has = |f: &str| join(root, f).exists();
    let layout = if has("pom.xml") {
        Layout::Maven
    } else if has("build.gradle") || has("build.gradle.kts") {
        Layout::Gradle
    } else {
        Layout::Standalone
    };
    let existing =
        |dirs: &[&str]| -> Vec<PathBuf> { dirs.iter().map(|d| join(root, d)).filter(|p| p.exists()).collect() };
    let source = if layout == Layout::Standalone {
        vec![root.to_path_buf()]
    } else {
        existing(&SOURCE)
    };
    let pom = read_pom(root);
    let paths: &[String] = pom.paths.as_deref().unwrap_or_default();
    let kept: Vec<&String> = paths
        .iter()
        .filter(|p| {
            let abs = resolve(root, p);
            !p.starts_with("classpath:") && abs.exists() && inside(root, &abs)
        })
        .collect();
    let left_out: Vec<&str> = paths.iter().filter(|p| !kept.contains(p)).map(String::as_str).collect();
    let notice: Vec<String> = match &pom.error {
        Some(error) => vec![format!(
            "karate-pom.json could not be read: {error}. Grace uses the default folders."
        )],
        None => {
            let mut notice = Vec::new();
            if !pom.unused.is_empty() {
                notice.push(format!(
                    "Grace does not use these karate-pom.json settings: {}.",
                    pom.unused.join(", ")
                ));
            }
            if !left_out.is_empty() {
                notice.push(format!(
                    "These karate-pom.json paths are not folders or files in the project: {}.",
                    left_out.join(", ")
                ));
            }
            notice
        }
    };
    let mut classpath = source.clone();
    classpath.extend(existing(compiled(layout)));
    let config_dir = pom.config_dir.clone();
    let config_dirs = match config_dir.as_deref() {
        None => unique(std::iter::once(root.to_path_buf()).chain(source.iter().cloned())),
        Some(dir) => {
            let mut dirs: Vec<PathBuf> = match dir.strip_prefix("classpath:") {
                Some(rel) => source.iter().map(|s| join(s, rel)).collect(),
                None => vec![resolve(root, dir)],
            };
            dirs.push(root.to_path_buf());
            dirs
        }
    };
    let bases = unique([karate_root(root, &classpath, config_dir.as_deref()), root.to_path_buf()]);
    ProjectLayout {
        layout,
        feature_roots: match &pom.paths {
            Some(_) => kept.iter().map(|p| resolve(root, p)).collect(),
            None => source,
        },
        classpath,
        config_dir,
        config_dirs,
        pom_notice: (!notice.is_empty()).then(|| notice.join(" ")),
        junit_xml: pom.junit_xml,
        bases,
    }
}

/// The Karate CLI's extension folders that exist, home first, each as a `<folder>/*` classpath entry: its launcher
/// puts every jar of `~/.karate/ext` and `.karate/ext` on the classpath (docs/specs/2026-11-02-project-libraries.md
/// R1).
pub fn extension_jars(home: &Path, root: &Path) -> Vec<PathBuf> {
    [home, root]
        .into_iter()
        .map(|d| join(d, ".karate/ext"))
        .filter(|d| fs::metadata(d).is_ok_and(|m| m.is_dir()))
        .map(|d| join(&d, "*"))
        .collect()
}

/// `-t` arguments: included tags in one OR group, each excluded tag its own (AND). Attached as `-t=…`: picocli reads a
/// separate argument starting with `@` as an argument file from the cwd. Takes Electron Grace's `TagFilter` as a
/// `FilterState` and ignores `hide_non_matching` (`model::tags`).
pub fn tag_args(f: &FilterState) -> Vec<String> {
    let mut args = Vec::new();
    if !f.include.is_empty() {
        let tags: Vec<String> = f.include.iter().map(|t| format!("@{t}")).collect();
        args.push(format!("-t={}", tags.join(",")));
    }
    args.extend(f.exclude.iter().map(|t| format!("-t=~@{t}")));
    args
}

/// The absolute path of a tree file or folder: the first of `bases` (Karate's root, then the project folder) that holds
/// it. Karate gives it relative to its root when the file is under it, else to the working directory (#208, #223).
/// `bases` is never empty (`detect_layout` gives the project at least); if it were, the file's own path is given.
pub fn locate(bases: &[PathBuf], file: &str) -> PathBuf {
    let mut first = None;
    for base in bases {
        let path = join(base, file);
        if path.exists() {
            return path;
        }
        first.get_or_insert(path);
    }
    first.unwrap_or_else(|| PathBuf::from(file))
}

/// A line as a template literal writes it: `null` when there is none (a node the dry run gave no line).
fn js_line(line: Option<u32>) -> String {
    line.map_or_else(|| "null".to_string(), |l| l.to_string())
}

/// Run targets for a node. A `file:LINE` target bypasses `-t` and `@ignore`, so an outline under a filter lists its
/// matching rows' lines, even when every row matches: the outline's own line would also run rows the dry run does not
/// list. Only when a row has no line (dynamic Examples) and every row matches does it fall back to the outline's line.
/// None when a matching row has no line and not every row matches, or when no row matches.
pub fn selection_targets(
    node: &TestNode,
    bases: &[PathBuf],
    feature_roots: &[PathBuf],
    f: &FilterState,
) -> Option<Vec<String>> {
    if node.kind == NodeKind::Folder && node.file.is_empty() {
        return Some(feature_roots.iter().map(|p| text(p)).collect());
    }
    let abs = text(&locate(bases, &node.file));
    if node.kind == NodeKind::Outline && filter_on(f) {
        let hits = matching_leaves(node, f);
        if hits.is_empty() {
            return None;
        }
        if let Some(lines) = hits.iter().map(|r| r.line).collect::<Option<Vec<u32>>>() {
            let lines: Vec<String> = lines.iter().map(u32::to_string).collect();
            return Some(vec![format!("{abs}:{}", lines.join(":"))]);
        }
        let every = hits.len() == matching_leaves(node, &no_filter()).len();
        return every.then(|| vec![format!("{abs}:{}", js_line(node.line))]);
    }
    Some(match node.kind {
        NodeKind::Folder | NodeKind::Feature => vec![abs],
        _ => vec![format!("{abs}:{}", js_line(node.line))],
    })
}

/// One `<abs file>:L1:L2…` target per feature file for nodes that have a line: lines ascending, files in first-seen
/// order.
pub fn line_targets<'a>(nodes: impl IntoIterator<Item = &'a TestNode>, bases: &[PathBuf]) -> Vec<String> {
    let mut by_file: IndexMap<String, Vec<u32>> = IndexMap::new();
    for n in nodes {
        let Some(line) = n.line else { continue };
        by_file.entry(text(&locate(bases, &n.file))).or_default().push(line);
    }
    by_file
        .into_iter()
        .map(|(abs, mut lines)| {
            lines.sort_unstable();
            let lines: Vec<String> = lines.iter().map(u32::to_string).collect();
            format!("{abs}:{}", lines.join(":"))
        })
        .collect()
}

/// A heap that starts small and stops at 2 GB, and an exit at the first OutOfMemoryError
/// (docs/specs/2026-11-01-java-memory.md).
const HEAP: [&str; 3] = ["-Xms64m", "-Xmx2g", "-XX:+ExitOnOutOfMemoryError"];

/// A dry run runs no step, so C1 alone lists the tests for a quarter of the CPU (docs/specs/2026-11-02-java-jit.md).
const JIT_DRY: &str = "-XX:TieredStopAtLevel=1";
/// A run keeps C2, which a suite of heavy JavaScript needs, with one compiler thread of each kind instead of 12 on 16
/// cores.
const JIT_RUN: &str = "-XX:CICompilerCount=2";

/// What separates the entries of Java's `-cp`: `;` on Windows, `:` on Linux (migration guide R5.3).
#[cfg(windows)]
pub const CLASSPATH_SEPARATOR: &str = ";";
/// What separates the entries of Java's `-cp`: `;` on Windows, `:` on Linux (migration guide R5.3).
#[cfg(not(windows))]
pub const CLASSPATH_SEPARATOR: &str = ":";

pub const OUT_OF_MEMORY: &str =
    "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once.";

/// The JVM's own line for a full heap, not a test's output (a logged response body can name the error too), nor
/// another OutOfMemoryError (Metaspace, native threads) that the 2 GB ceiling did not cause.
pub fn out_of_memory<S: AsRef<str>>(lines: &[S]) -> bool {
    lines.iter().any(|l| {
        l.as_ref()
            .starts_with("Terminating due to java.lang.OutOfMemoryError: Java heap space")
    })
}

/// A debug run's listener jar and socket (docs/specs/2026-11-02-step-debugger.md, R2.1).
#[derive(Clone, Copy, Debug)]
pub struct DebugArgs<'a> {
    pub jar: &'a Path,
    pub port: u16,
    pub token: &'a str,
}

/// What `build_args` makes Java's command line from: `buildArgs`'s options.
#[derive(Clone, Copy, Debug)]
pub struct BuildArgs<'a> {
    pub jar: &'a Path,
    pub classpath: &'a [PathBuf],
    pub root: &'a Path,
    pub out_dir: &'a Path,
    pub dry_run: bool,
    pub tags: &'a [String],
    pub targets: &'a [String],
    pub env: Option<&'a str>,
    pub arg_file: Option<&'a Path>,
    /// Karate's `-T`. A dry run always gets 1: more threads would interleave its events.
    pub threads: u32,
    /// Karate's `-g`, as written in karate-pom.json.
    pub config_dir: Option<&'a str>,
    pub debug: Option<DebugArgs<'a>>,
    /// Adds Karate's JUnit XML to the formats.
    pub junit_xml: bool,
}

/// Java's arguments, after the program. An empty `env`, `arg_file` or `config_dir` is left out, as the TypeScript's
/// falsy empty string is.
pub fn build_args(o: &BuildArgs) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    // The launcher expands `@file` only before the main class, so it goes first.
    if let Some(file) = o.arg_file.filter(|f| !f.as_os_str().is_empty()) {
        args.push(format!("@{}", text(file)));
    }
    args.extend(HEAP.map(String::from));
    args.push(if o.dry_run { JIT_DRY } else { JIT_RUN }.to_string());
    args.push("-Dstdout.encoding=UTF-8".into());
    args.push("-Dstderr.encoding=UTF-8".into());
    if let Some(debug) = &o.debug {
        args.push(format!("-Dgrace.debug.port={}", debug.port));
        args.push(format!("-Dgrace.debug.token={}", debug.token));
    }
    args.push("-cp".into());
    let classpath: Vec<String> = std::iter::once(o.jar)
        .chain(o.debug.map(|d| d.jar))
        .chain(o.classpath.iter().map(PathBuf::as_path))
        .map(text)
        .collect();
    args.push(classpath.join(CLASSPATH_SEPARATOR));
    args.extend(["io.karatelabs.Main", "run", "--no-color"].map(String::from));
    // Grace reads karate-pom.json itself: Karate would apply its env, tags or dryRun unseen
    args.push("--no-pom".into());
    args.extend(["-w".into(), text(o.root), "-o".into(), text(o.out_dir)]);
    args.extend(["-B", "false", "-f"].map(String::from));
    args.push(
        if o.junit_xml {
            "html,karate:jsonl,junit:xml"
        } else {
            "html,karate:jsonl"
        }
        .into(),
    );
    if o.debug.is_some() {
        args.extend(["--listener".into(), DEBUG_LISTENER.into()]);
    }
    if let Some(env) = o.env.filter(|e| !e.is_empty()) {
        args.extend(["-e".into(), env.into()]);
    }
    if let Some(dir) = o.config_dir.filter(|d| !d.is_empty()) {
        args.extend(["-g".into(), dir.into()]);
    }
    if o.dry_run {
        args.extend(["-D", "-T", "1"].map(String::from));
    } else {
        args.extend(["-T".into(), o.threads.to_string()]);
    }
    args.extend(o.tags.iter().cloned());
    args.extend(o.targets.iter().cloned());
    args
}

/// An environment for a child process: its variables in order.
pub type Env = Vec<(OsString, OsString)>;

/// `env` without the variables Grace strips, then with each of `vars` set, replacing one of the same name in any
/// letter case: Windows sees one variable. A later one wins. Windows env keys come in any case, so compare upper-cased
/// (JavaScript's `toUpperCase`, which is Rust's `to_uppercase`). Takes the variables by value, so the caller's are never
/// changed. On Linux, `process::spawn_karate` adds the locale Java needs (`LC_ALL`); it is not part of this list.
pub fn child_env<K, V>(env: impl IntoIterator<Item = (K, V)>, vars: &[PlainRow]) -> Env
where
    K: Into<OsString>,
    V: Into<OsString>,
{
    let upper = |k: &OsString| k.to_string_lossy().to_uppercase();
    let mut out: Env = env
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .filter(|(k, _)| !STRIPPED_ENV_VARS.contains(&upper(k).as_str()))
        .collect();
    for var in vars {
        let name = var.name.to_uppercase();
        out.retain(|(k, _)| upper(k) != name);
        out.push((var.name.clone().into(), var.value.clone().into()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// `project(files, folders)`: a temporary folder with empty files and folders; removed when the `TempDir` drops,
    /// as the TypeScript's `afterEach` does.
    fn project(files: &[&str], folders: &[&str]) -> TempDir {
        let d = tempfile::Builder::new().prefix("kk-cmd-").tempdir().unwrap();
        for f in files {
            fs::write(d.path().join(f), "").unwrap();
        }
        for f in folders {
            fs::create_dir_all(d.path().join(f)).unwrap();
        }
        d
    }

    fn pom(root: &Path, json: &str) {
        fs::write(root.join("karate-pom.json"), json).unwrap();
    }

    /// `path.join(root, d)` for a `/`-separated `d`.
    fn at(root: &Path, d: &str) -> PathBuf {
        d.split('/').fold(root.to_path_buf(), |p, s| p.join(s))
    }

    fn all_at(root: &Path, dirs: &[&str]) -> Vec<PathBuf> {
        dirs.iter().map(|d| at(root, d)).collect()
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    mod extension_jars {
        use super::*;

        fn ext(root: &Path) -> PathBuf {
            at(root, ".karate/ext")
        }

        #[test]
        fn lists_the_home_folder_then_the_projects_as_wildcards() {
            let home = project(&[], &[".karate/ext"]);
            let root = project(&[], &[".karate/ext"]);
            assert_eq!(
                extension_jars(home.path(), root.path()),
                [ext(home.path()).join("*"), ext(root.path()).join("*")]
            );
        }

        #[test]
        fn skips_a_folder_that_does_not_exist() {
            let home = project(&[], &[]);
            let root = project(&[], &[".karate/ext"]);
            assert_eq!(extension_jars(home.path(), root.path()), [ext(root.path()).join("*")]);
            let other = project(&[], &[]);
            assert_eq!(extension_jars(home.path(), other.path()), [] as [PathBuf; 0]);
        }
    }

    mod detect_layout {
        use super::*;

        #[test]
        fn maven_with_both_test_folders() {
            let dir = project(&["pom.xml"], &["src/test/java", "src/test/resources"]);
            let root = dir.path();
            let both = all_at(root, &["src/test/java", "src/test/resources"]);
            let mut config_dirs = vec![root.to_path_buf()];
            config_dirs.extend(both.clone());
            assert_eq!(
                detect_layout(root),
                ProjectLayout {
                    layout: Layout::Maven,
                    classpath: both.clone(),
                    feature_roots: both,
                    config_dir: None,
                    config_dirs,
                    pom_notice: None,
                    junit_xml: false,
                    bases: vec![root.to_path_buf()],
                }
            );
        }

        #[test]
        fn gradle() {
            let dir = project(&["build.gradle.kts"], &["src/test/resources"]);
            assert_eq!(detect_layout(dir.path()).layout, Layout::Gradle);
        }

        #[test]
        fn only_existing_folders() {
            let dir = project(&["pom.xml"], &["src/test/java"]);
            let only = all_at(dir.path(), &["src/test/java"]);
            let layout = detect_layout(dir.path());
            assert_eq!(layout.classpath, only);
            assert_eq!(layout.feature_roots, only);
        }

        #[test]
        fn standalone() {
            let dir = project(&[], &[]);
            let root = dir.path().to_path_buf();
            assert_eq!(
                detect_layout(&root),
                ProjectLayout {
                    layout: Layout::Standalone,
                    classpath: vec![root.clone()],
                    feature_roots: vec![root.clone()],
                    config_dir: None,
                    config_dirs: vec![root.clone()],
                    pom_notice: None,
                    junit_xml: false,
                    bases: vec![root],
                }
            );
        }

        // Karate's names of the files are relative to its root when they are under it (#208, #223)
        mod bases {
            use super::*;

            #[test]
            fn karates_root_first_the_classpath_folder_of_the_first_karate_config_js() {
                let dir = project(
                    &["pom.xml"],
                    &["src/test/java", "src/test/resources", "target/test-classes"],
                );
                let root = dir.path();
                for d in ["src/test/resources", "target/test-classes"] {
                    fs::write(at(root, d).join("karate-config.js"), "").unwrap();
                }
                assert_eq!(
                    detect_layout(root).bases,
                    [at(root, "src/test/resources"), root.to_path_buf()]
                );
            }

            #[test]
            fn a_config_dir_off_the_classpath_as_a_folder_a_file_path_or_a_js_file() {
                let dir = project(&[], &[]);
                let root = dir.path();
                for config_dir in ["conf", "file:conf", "conf/my-config.js"] {
                    pom(root, &serde_json::json!({ "configDir": config_dir }).to_string());
                    assert_eq!(detect_layout(root).bases, [at(root, "conf"), root.to_path_buf()]);
                }
            }

            #[test]
            fn the_classpath_folder_of_a_classpath_config_dir() {
                let dir = project(&["pom.xml"], &["src/test/java/conf", "src/test/resources/conf"]);
                let root = dir.path();
                fs::write(at(root, "src/test/resources/conf/karate-config.js"), "").unwrap();
                pom(root, r#"{"configDir":"classpath:conf"}"#);
                assert_eq!(
                    detect_layout(root).bases,
                    [at(root, "src/test/resources/conf"), root.to_path_buf()]
                );
                fs::write(at(root, "src/test/java/conf/my.js"), "").unwrap();
                pom(root, r#"{"configDir":"classpath:/conf/my.js"}"#);
                assert_eq!(
                    detect_layout(root).bases,
                    [at(root, "src/test/java/conf"), root.to_path_buf()]
                );
            }

            #[test]
            fn the_project_when_no_classpath_folder_holds_the_config() {
                let dir = project(&["pom.xml", "karate-config.js"], &["src/test/java"]);
                let root = dir.path();
                pom(root, r#"{"configDir":"classpath:conf"}"#);
                assert_eq!(detect_layout(root).bases, [root.to_path_buf()]);
            }
        }

        mod compiled_output {
            use super::*;

            #[test]
            fn maven_after_the_source_folders_never_a_feature_root_or_a_config_folder() {
                let dir = project(
                    &["pom.xml"],
                    &[
                        "src/test/java",
                        "src/test/resources",
                        "target/test-classes",
                        "target/classes",
                    ],
                );
                let root = dir.path();
                let source = all_at(root, &["src/test/java", "src/test/resources"]);
                let layout = detect_layout(root);
                let mut classpath = source.clone();
                classpath.extend(all_at(root, &["target/test-classes", "target/classes"]));
                assert_eq!(layout.classpath, classpath);
                assert_eq!(layout.feature_roots, source);
                let mut config_dirs = vec![root.to_path_buf()];
                config_dirs.extend(source);
                assert_eq!(layout.config_dirs, config_dirs);
            }

            #[test]
            fn gradle_in_its_order() {
                let dir = project(
                    &["build.gradle"],
                    &[
                        "src/test/java",
                        "build/resources/main",
                        "build/resources/test",
                        "build/classes/java/test",
                    ],
                );
                let root = dir.path();
                assert_eq!(
                    detect_layout(root).classpath,
                    all_at(
                        root,
                        &[
                            "src/test/java",
                            "build/classes/java/test",
                            "build/resources/test",
                            "build/resources/main",
                        ]
                    )
                );
            }

            #[test]
            fn standalone_unchanged() {
                let dir = project(&[], &["target/test-classes"]);
                assert_eq!(detect_layout(dir.path()).classpath, [dir.path().to_path_buf()]);
            }
        }

        mod karate_pom_json {
            use super::*;

            #[test]
            fn takes_output_junit_xml() {
                let dir = project(&[], &[]);
                pom(dir.path(), r#"{"output":{"junitXml":true}}"#);
                let layout = detect_layout(dir.path());
                assert!(layout.junit_xml);
                assert_eq!(layout.pom_notice, None);
            }

            #[test]
            fn keeps_the_paths_inside_the_project_and_names_the_others() {
                let dir = project(&[], &["tests/a"]);
                let root = dir.path();
                pom(root, r#"{"paths":["tests/a","missing","..","classpath:x","."]}"#);
                let layout = detect_layout(root);
                assert_eq!(layout.feature_roots, [at(root, "tests/a"), root.to_path_buf()]);
                assert_eq!(
                    layout.pom_notice.as_deref(),
                    Some(
                        "These karate-pom.json paths are not folders or files in the project: missing, .., classpath:x."
                    )
                );
            }

            #[test]
            fn keeps_a_folder_whose_name_starts_with_dots() {
                let dir = project(&[], &["..data"]);
                let root = dir.path();
                pom(root, r#"{"paths":["..data"]}"#);
                let layout = detect_layout(root);
                assert_eq!(layout.feature_roots, [at(root, "..data")]);
                assert_eq!(layout.pom_notice, None);
            }

            #[test]
            fn keeps_the_default_feature_roots_without_paths() {
                let dir = project(&["pom.xml"], &["src/test/java"]);
                let root = dir.path();
                pom(root, r#"{"paths":[]}"#);
                let layout = detect_layout(root);
                assert_eq!(layout.feature_roots, all_at(root, &["src/test/java"]));
                assert_eq!(layout.pom_notice, None);
            }

            #[test]
            fn passes_config_dir_as_written_and_reads_environments_there_and_in_the_project() {
                let dir = project(&[], &[]);
                let root = dir.path();
                pom(root, r#"{"configDir":"conf","env":"qa"}"#);
                let layout = detect_layout(root);
                assert_eq!(layout.config_dir.as_deref(), Some("conf"));
                assert_eq!(layout.config_dirs, [at(root, "conf"), root.to_path_buf()]);
                assert_eq!(layout.pom_notice, None);
            }

            #[test]
            fn resolves_a_classpath_config_dir_under_each_source_folder() {
                let dir = project(&["pom.xml"], &["src/test/java", "src/test/resources", "target/classes"]);
                let root = dir.path();
                pom(root, r#"{"configDir":"classpath:conf"}"#);
                let mut expected = all_at(root, &["src/test/java/conf", "src/test/resources/conf"]);
                expected.push(root.to_path_buf());
                assert_eq!(detect_layout(root).config_dirs, expected);
            }

            #[test]
            fn says_both_settings_first() {
                let dir = project(&[], &[]);
                pom(dir.path(), r#"{"threads":64,"paths":["gone"],"dryRun":true}"#);
                assert_eq!(
                    detect_layout(dir.path()).pom_notice.as_deref(),
                    Some(
                        "Grace does not use these karate-pom.json settings: threads 64, dryRun. \
                         These karate-pom.json paths are not folders or files in the project: gone."
                    )
                );
            }

            #[test]
            fn uses_nothing_from_a_file_it_cannot_read() {
                let dir = project(&["pom.xml"], &["src/test/java"]);
                let root = dir.path();
                pom(root, r#"{"paths":"x","configDir":"conf"}"#);
                let layout = detect_layout(root);
                assert_eq!(layout.feature_roots, all_at(root, &["src/test/java"]));
                assert_eq!(layout.config_dir, None);
                let mut config_dirs = vec![root.to_path_buf()];
                config_dirs.extend(all_at(root, &["src/test/java"]));
                assert_eq!(layout.config_dirs, config_dirs);
                assert_eq!(
                    layout.pom_notice.as_deref(),
                    Some(
                        "karate-pom.json could not be read: paths is not a list of strings. Grace uses the default folders."
                    )
                );
            }
        }
    }

    fn node(kind: NodeKind, file: &str, line: Option<u32>) -> TestNode {
        TestNode {
            key: "x".into(),
            kind,
            name: "n".into(),
            file: file.into(),
            line,
            ref_id: None,
            tags: vec![],
            example: None,
            children: vec![],
        }
    }

    mod locate {
        use super::*;

        #[test]
        fn takes_the_first_base_that_holds_the_file_else_the_first_base() {
            let dir = project(&["pom.xml"], &["src/test/java/examples", "tests"]);
            let root = dir.path().to_path_buf();
            let java = at(&root, "src/test/java");
            let bases = [root.clone(), java.clone()];
            assert_eq!(locate(&bases, "examples"), java.join("examples"));
            assert_eq!(locate(&bases, "tests"), root.join("tests"));
            assert_eq!(locate(&bases, "gone/a.feature"), root.join("gone").join("a.feature"));
        }
    }

    /// `path.join("D:", "Mes tests", "équipe")`: a folder that does not exist, with a space and an accent.
    fn mes_tests() -> PathBuf {
        Path::new("D:").join("Mes tests").join("équipe")
    }

    fn display(p: &Path) -> String {
        p.to_string_lossy().into_owned()
    }

    mod selection_targets {
        use super::*;

        fn roots() -> Vec<PathBuf> {
            vec![at(&mes_tests(), "src/test/java")]
        }

        fn abs() -> String {
            display(&at(&mes_tests(), "tests/a.feature"))
        }

        #[test]
        fn maps_each_node_kind() {
            let root = [mes_tests()];
            let nf = no_filter();
            let roots_text: Vec<String> = roots().iter().map(|p| display(p)).collect();
            assert_eq!(
                selection_targets(&node(NodeKind::Folder, "", None), &root, &roots(), &nf),
                Some(roots_text)
            );
            assert_eq!(
                selection_targets(&node(NodeKind::Folder, "tests", None), &root, &roots(), &nf),
                Some(vec![display(&at(&mes_tests(), "tests"))])
            );
            assert_eq!(
                selection_targets(&node(NodeKind::Feature, "tests/a.feature", None), &root, &roots(), &nf),
                Some(vec![abs()])
            );
            for kind in [NodeKind::Scenario, NodeKind::Outline, NodeKind::Row] {
                assert_eq!(
                    selection_targets(&node(kind, "tests/a.feature", Some(12)), &root, &roots(), &nf),
                    Some(vec![format!("{}:12", abs())])
                );
            }
        }

        mod outline_targets_under_a_filter {
            use super::*;

            fn row(line: Option<u32>, tag: &str) -> TestNode {
                TestNode {
                    tags: vec![tag.into()],
                    ..node(NodeKind::Row, "tests/a.feature", line)
                }
            }

            fn outline(r9_line: Option<u32>) -> TestNode {
                TestNode {
                    children: vec![row(r9_line, "fast"), row(Some(10), "fast"), row(Some(11), "slow")],
                    ..node(NodeKind::Outline, "tests/a.feature", Some(5))
                }
            }

            fn f(include: &[&str]) -> FilterState {
                FilterState {
                    include: strings(include),
                    ..no_filter()
                }
            }

            fn targets(outline: &TestNode, filter: &FilterState) -> Option<Vec<String>> {
                selection_targets(outline, &[mes_tests()], &roots(), filter)
            }

            #[test]
            fn lists_the_matching_rows_lines_in_one_target() {
                assert_eq!(
                    targets(&outline(Some(9)), &f(&["fast"])),
                    Some(vec![format!("{}:9:10", abs())])
                );
            }

            #[test]
            fn lists_every_rows_line_when_every_row_matches_the_outline_line_would_run_ignore_rows() {
                assert_eq!(
                    targets(&outline(Some(9)), &f(&["fast", "slow"])),
                    Some(vec![format!("{}:9:10:11", abs())])
                );
            }

            #[test]
            fn falls_back_to_the_outline_line_when_every_row_matches_and_one_has_no_line() {
                assert_eq!(
                    targets(&outline(None), &f(&["fast", "slow"])),
                    Some(vec![format!("{}:5", abs())])
                );
            }

            #[test]
            fn is_null_when_a_matching_row_has_no_line() {
                assert_eq!(targets(&outline(None), &f(&["fast"])), None);
            }

            #[test]
            fn is_null_when_no_row_matches() {
                assert_eq!(targets(&outline(Some(9)), &f(&["other"])), None);
            }
        }
    }

    mod line_targets {
        use super::*;

        #[test]
        fn groups_by_file_with_ascending_lines_files_in_first_seen_order() {
            let root = mes_tests();
            let at_line = |file: &str, line: u32| node(NodeKind::Scenario, file, Some(line));
            let nodes = [
                at_line("tests/b.feature", 20),
                at_line("tests/a.feature", 12),
                at_line("tests/b.feature", 5),
            ];
            assert_eq!(
                line_targets(&nodes, std::slice::from_ref(&root)),
                [
                    format!("{}:5:20", display(&at(&root, "tests/b.feature"))),
                    format!("{}:12", display(&at(&root, "tests/a.feature"))),
                ]
            );
        }
    }

    mod tag_args {
        use super::*;

        fn filter(include: &[&str], exclude: &[&str]) -> FilterState {
            FilterState {
                include: strings(include),
                exclude: strings(exclude),
                hide_non_matching: false,
            }
        }

        #[test]
        fn is_empty_when_the_filter_is_off() {
            assert_eq!(tag_args(&filter(&[], &[])), [] as [String; 0]);
        }

        #[test]
        fn joins_included_tags_in_one_t_and_gives_each_excluded_tag_its_own() {
            assert_eq!(
                tag_args(&filter(&["smoke", "api"], &["wip", "slow"])),
                ["-t=@smoke,@api", "-t=~@wip", "-t=~@slow"]
            );
        }

        #[test]
        fn omits_the_include_group_when_nothing_is_included() {
            assert_eq!(tag_args(&filter(&[], &["wip"])), ["-t=~@wip"]);
        }

        #[test]
        fn attaches_every_value_so_no_argument_starts_with_at_picocli_argument_files() {
            assert_eq!(tag_args(&filter(&["fast"], &[])), ["-t=@fast"]);
        }
    }

    mod build_args {
        use super::*;

        // The one platform difference of the command line (migration guide R5.3): the classpath separator.
        #[cfg(windows)]
        const CP: &str = "k.jar;c1;c2";
        #[cfg(not(windows))]
        const CP: &str = "k.jar:c1:c2";
        #[cfg(windows)]
        const DEBUG_CP: &str = "k.jar;listener.jar;c1;c2";
        #[cfg(not(windows))]
        const DEBUG_CP: &str = "k.jar:listener.jar:c1:c2";

        struct Paths {
            jar: PathBuf,
            classpath: Vec<PathBuf>,
            root: PathBuf,
            out_dir: PathBuf,
            targets: Vec<String>,
        }

        fn paths() -> Paths {
            Paths {
                jar: "k.jar".into(),
                classpath: vec!["c1".into(), "c2".into()],
                root: "R".into(),
                out_dir: "O".into(),
                targets: strings(&["t1"]),
            }
        }

        /// The TypeScript's `o`: no tags, environment, argument file, config folder, JUnit XML or debugger, one thread.
        fn o(p: &Paths, dry_run: bool) -> BuildArgs<'_> {
            BuildArgs {
                jar: &p.jar,
                classpath: &p.classpath,
                root: &p.root,
                out_dir: &p.out_dir,
                dry_run,
                tags: &[],
                targets: &p.targets,
                env: None,
                arg_file: None,
                threads: 1,
                config_dir: None,
                debug: None,
                junit_xml: false,
            }
        }

        fn base() -> Vec<String> {
            strings(&[
                "-Xms64m",
                "-Xmx2g",
                "-XX:+ExitOnOutOfMemoryError",
                "-XX:CICompilerCount=2",
                "-Dstdout.encoding=UTF-8",
                "-Dstderr.encoding=UTF-8",
                "-cp",
                CP,
                "io.karatelabs.Main",
                "run",
                "--no-color",
                "--no-pom",
                "-w",
                "R",
                "-o",
                "O",
                "-B",
                "false",
                "-f",
                "html,karate:jsonl",
            ])
        }

        /// `base.with(i, value)`.
        fn with(mut v: Vec<String>, i: usize, value: &str) -> Vec<String> {
            v[i] = value.into();
            v
        }

        // A dry run compiles with C1 only; a run keeps C2 on fewer threads.
        fn dry_base() -> Vec<String> {
            with(base(), 3, "-XX:TieredStopAtLevel=1")
        }

        fn cat(parts: &[&[String]]) -> Vec<String> {
            parts.concat()
        }

        #[test]
        fn builds_the_exact_command_line() {
            let p = paths();
            assert_eq!(build_args(&o(&p, false)), cat(&[&base(), &strings(&["-T", "1", "t1"])]));
            assert_eq!(
                build_args(&o(&p, true)),
                cat(&[&dry_base(), &strings(&["-D", "-T", "1", "t1"])])
            );
        }

        #[test]
        fn asks_for_junit_xml_when_told_to() {
            let p = paths();
            let args = build_args(&BuildArgs {
                junit_xml: true,
                ..o(&p, false)
            });
            assert_eq!(
                args,
                cat(&[
                    &with(base(), 19, "html,karate:jsonl,junit:xml"),
                    &strings(&["-T", "1", "t1"])
                ])
            );
        }

        #[test]
        fn adds_the_debug_listener_and_its_socket_to_a_debug_run_only() {
            let p = paths();
            let listener = PathBuf::from("listener.jar");
            let debug = DebugArgs {
                jar: &listener,
                port: 5005,
                token: "t",
            };
            let args = build_args(&BuildArgs {
                debug: Some(debug),
                ..o(&p, false)
            });
            assert_eq!(
                args,
                cat(&[
                    &base()[..6],
                    &strings(&["-Dgrace.debug.port=5005", "-Dgrace.debug.token=t", "-cp", DEBUG_CP]),
                    &base()[8..],
                    &strings(&["--listener", "grace.DebugListener", "-T", "1", "t1"]),
                ])
            );
            // the TypeScript's `debug: null`
            let none = build_args(&BuildArgs {
                debug: None,
                ..o(&p, false)
            });
            assert_eq!(none, cat(&[&base(), &strings(&["-T", "1", "t1"])]));
        }

        #[test]
        fn passes_the_count_on_runs_and_t_1_on_dry_runs() {
            let p = paths();
            let run = build_args(&BuildArgs {
                threads: 8,
                ..o(&p, false)
            });
            assert_eq!(run[run.len() - 3..], strings(&["-T", "8", "t1"]));
            let dry = build_args(&BuildArgs {
                threads: 8,
                ..o(&p, true)
            });
            assert_eq!(dry[dry.len() - 4..], strings(&["-D", "-T", "1", "t1"]));
        }

        #[test]
        fn puts_the_tag_arguments_just_before_the_targets() {
            let p = paths();
            let tags = strings(&["-t=@smoke"]);
            assert_eq!(
                build_args(&BuildArgs {
                    tags: &tags,
                    ..o(&p, true)
                }),
                cat(&[&dry_base(), &strings(&["-D", "-T", "1"]), &tags, &strings(&["t1"])])
            );
            assert_eq!(
                build_args(&BuildArgs {
                    tags: &tags,
                    ..o(&p, false)
                }),
                cat(&[&base(), &strings(&["-T", "1"]), &tags, &strings(&["t1"])])
            );
        }

        #[test]
        fn puts_the_argument_file_first_and_e_after_f() {
            let p = paths();
            let arg_file = PathBuf::from("A.txt");
            let args = build_args(&BuildArgs {
                env: Some("qa"),
                arg_file: Some(&arg_file),
                ..o(&p, false)
            });
            assert_eq!(
                args,
                cat(&[&strings(&["@A.txt"]), &base(), &strings(&["-e", "qa", "-T", "1", "t1"])])
            );
        }

        #[test]
        fn puts_g_after_f_and_e() {
            let p = paths();
            let args = build_args(&BuildArgs {
                env: Some("qa"),
                config_dir: Some("conf"),
                ..o(&p, false)
            });
            assert_eq!(
                args,
                cat(&[&base(), &strings(&["-e", "qa", "-g", "conf", "-T", "1", "t1"])])
            );
        }

        #[test]
        fn without_env_or_arg_file_is_unchanged() {
            let p = paths();
            assert_eq!(build_args(&o(&p, false)), cat(&[&base(), &strings(&["-T", "1", "t1"])]));
        }

        // Not in the TypeScript tests (Electron Grace runs on Windows only): the platform seam of the command line,
        // one test on each platform (migration guide R5.3).
        #[cfg(windows)]
        #[test]
        fn the_classpath_separator_is_a_semicolon_on_windows() {
            assert_eq!(CLASSPATH_SEPARATOR, ";");
        }

        #[cfg(not(windows))]
        #[test]
        fn the_classpath_separator_is_a_colon_on_linux() {
            assert_eq!(CLASSPATH_SEPARATOR, ":");
        }
    }

    mod out_of_memory {
        use super::*;

        #[test]
        fn finds_javas_out_of_memory_line_among_the_others() {
            assert!(out_of_memory(&[
                "a",
                "Terminating due to java.lang.OutOfMemoryError: Java heap space"
            ]));
            // other out-of-memory errors are not the heap's ceiling: Java's own line is enough there
            assert!(!out_of_memory(&[
                "Terminating due to java.lang.OutOfMemoryError: Metaspace"
            ]));
            assert!(!out_of_memory(&[
                "Terminating due to java.lang.OutOfMemoryError: unable to create native thread: possibly out of memory"
            ]));
            // a response body that names the error is the test's, not Java's
            assert!(!out_of_memory(&[
                r#"  "error": "java.lang.OutOfMemoryError: Java heap space""#
            ]));
            assert!(!out_of_memory(&["scenario failed", "OutOfMemory in a test name"]));
            assert!(!out_of_memory::<&str>(&[]));
        }

        #[test]
        fn says_what_happened_and_what_to_do() {
            assert_eq!(
                OUT_OF_MEMORY,
                "Java ran out of memory: Grace lets it use 2 GB. Run fewer tests or fewer threads at once."
            );
        }
    }

    mod child_env {
        use super::*;

        fn env(pairs: &[(&str, &str)]) -> Env {
            pairs.iter().map(|(k, v)| (k.into(), v.into())).collect()
        }

        fn var(name: &str, value: &str) -> PlainRow {
            PlainRow {
                name: name.into(),
                value: value.into(),
                env_var: Some(true),
            }
        }

        #[test]
        fn drops_the_option_variables_in_any_case_keeps_the_rest_does_not_mutate() {
            let given = env(&[
                ("PATH", "p"),
                ("Karate_Options", "a"),
                ("JAVA_TOOL_OPTIONS", "b"),
                ("jdk_java_options", "c"),
                ("_JAVA_OPTIONS", "d"),
            ]);
            assert_eq!(child_env(given.iter().cloned(), &[]), env(&[("PATH", "p")]));
            assert_eq!(given.len(), 5);
        }

        #[test]
        fn sets_each_variable_replacing_one_of_the_same_name_in_any_case_a_later_one_wins() {
            assert_eq!(
                child_env(env(&[("PATH", "p"), ("app_x", "m")]), &[var("APP_X", "g")]),
                env(&[("PATH", "p"), ("APP_X", "g")])
            );
            assert_eq!(
                child_env(Env::new(), &[var("APP_X", "a"), var("app_x", "b")]),
                env(&[("app_x", "b")])
            );
        }

        #[test]
        fn strips_karate_env_and_karate_config_dir() {
            assert_eq!(
                child_env([("PATH", "p"), ("KARATE_ENV", "prod"), ("karate_config_dir", "x")], &[]),
                env(&[("PATH", "p")])
            );
        }
    }

    // Not in the TypeScript tests, which check it through the runner (`runner.runs.test.ts`): the memory flags on a
    // real Java, and the line `out_of_memory` finds (JM-01, JM-02). The feature asks for one array of about 2 GB, which
    // fails at once without using the memory (docs/specs/2026-11-01-java-memory.md, Left to the builder).
    mod build_args_through_java {
        use super::*;
        use crate::karate::process::{SpawnOptions, spawn_karate};
        use crate::karate::testing::vendor;
        use std::sync::{Arc, Mutex};

        #[test]
        fn java_stops_at_the_first_out_of_memory_error_with_the_line_out_of_memory_finds() {
            let vendor = vendor();
            let dir = project(&[], &[]);
            let root = dir.path();
            fs::write(
                root.join("big.feature"),
                [
                    "Feature: big",
                    "",
                    "Scenario: asks for more than the limit",
                    "  * def bytes = Java.type('java.lang.reflect.Array').newInstance(Java.type('java.lang.Byte').TYPE, 2147483000)",
                    "",
                    "Scenario: after it",
                    "  * print 'after it ran'",
                    "",
                ]
                .join("\n"),
            )
            .unwrap();
            let out = tempfile::Builder::new().prefix("kk-out-").tempdir().unwrap();
            let layout = detect_layout(root);
            let targets = vec![display(&root.join("big.feature"))];
            let args = build_args(&BuildArgs {
                jar: &vendor.jar,
                classpath: &layout.classpath,
                root,
                out_dir: out.path(),
                dry_run: false,
                tags: &[],
                targets: &targets,
                env: None,
                arg_file: None,
                threads: 1,
                config_dir: None,
                debug: None,
                junit_xml: false,
            });
            let lines = Arc::new(Mutex::new(Vec::<String>::new()));
            let sink = Arc::clone(&lines);
            let p = spawn_karate(
                &vendor.java,
                &args,
                root,
                move |l| sink.lock().unwrap().extend(l),
                SpawnOptions::default(),
            );
            let exit = p.wait();
            let lines = lines.lock().unwrap();
            assert_eq!(exit, Some(3), "{}", lines.join("\n"));
            assert!(out_of_memory(&lines), "{}", lines.join("\n"));
            assert!(!lines.iter().any(|l| l.contains("after it ran")));
        }
    }

    // The path rules `detect_layout` and `locate` lean on, as Node's `path` has them. Not in the TypeScript tests.
    mod paths {
        use super::*;

        #[test]
        fn join_appends_a_leading_slash_and_normalizes_as_node_does() {
            // A base already in the platform's own form: Node's `path.join` also turns `/` into `\` on Windows
            let p = std::env::temp_dir().join("p");
            let base = p.join("src");
            assert_eq!(join(&base, "/conf/my.js"), at(&base, "conf/my.js"));
            assert_eq!(join(&base, "a/./b/../c"), at(&base, "a/c"));
            assert_eq!(join(&base, "../x"), at(&p, "x"));
            assert_eq!(join(&base, "").as_os_str(), base.as_os_str());
        }

        #[test]
        fn inside_compares_names_not_text() {
            let root = std::env::temp_dir().join("kk-root");
            assert!(inside(&root, &root));
            assert!(inside(&root, &root.join("..data")));
            assert!(inside(&root, &root.join("a").join("..").join("b")));
            assert!(!inside(&root, &root.join("..")));
            assert!(!inside(&root, &std::env::temp_dir().join("kk-root-2")));
        }

        #[cfg(windows)]
        #[test]
        fn inside_ignores_letter_case_on_windows() {
            assert!(inside(
                Path::new(r"C:\Users\Me\Proj"),
                Path::new(r"c:\users\me\proj\tests")
            ));
        }

        #[cfg(not(windows))]
        #[test]
        fn inside_minds_letter_case_on_linux() {
            assert!(!inside(Path::new("/home/me/Proj"), Path::new("/home/me/proj/tests")));
        }
    }
}
