//! The Karate side, ported from `src/main/karate/` (command.ts, tree.ts, process.ts) for the prototype only:
//! the command line of `buildArgs`, the tree of `buildTree`, and a run tailed from `karate-events.jsonl` every
//! 100 ms, as `runner.ts` polls it. No shell, the four option variables removed, `cwd` = the project root.
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

/// Removed from Java's environment: they would override Grace's own values (AGENTS.md, Karate integration rules).
const STRIPPED_ENV: [&str; 4] = ["KARATE_OPTIONS", "JAVA_TOOL_OPTIONS", "JDK_JAVA_OPTIONS", "_JAVA_OPTIONS"];

pub struct Vendor {
    pub java: PathBuf,
    pub jar: PathBuf,
}

impl Vendor {
    /// `GRACE_JAVA` and `GRACE_KARATE_JAR`, else `vendor/jre/bin/java` and `vendor/karate.jar` beside the exe.
    pub fn find() -> Vendor {
        let base = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("vendor")))
            .unwrap_or_else(|| PathBuf::from("vendor"));
        let java = std::env::var_os("GRACE_JAVA")
            .map(PathBuf::from)
            .unwrap_or_else(|| base.join("jre").join("bin").join(if cfg!(windows) { "java.exe" } else { "java" }));
        let jar = std::env::var_os("GRACE_KARATE_JAR")
            .map(PathBuf::from)
            .unwrap_or_else(|| base.join("karate.jar"));
        Vendor { java, jar }
    }
}

/// `buildArgs` of command.ts for a standalone project (classpath = the root).
pub fn args(jar: &Path, root: &Path, out: &Path, dry: bool, threads: u32, targets: &[String]) -> Vec<String> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut a: Vec<String> = vec![
        "-Xms64m".into(),
        "-Xmx2g".into(),
        "-XX:+ExitOnOutOfMemoryError".into(),
        if dry { "-XX:TieredStopAtLevel=1".into() } else { "-XX:CICompilerCount=2".into() },
        "-Dstdout.encoding=UTF-8".into(),
        "-Dstderr.encoding=UTF-8".into(),
        "-cp".into(),
        format!("{}{sep}{}", jar.display(), root.display()),
        "io.karatelabs.Main".into(),
        "run".into(),
        "--no-color".into(),
        "--no-pom".into(),
        "-w".into(),
        root.display().to_string(),
        "-o".into(),
        out.display().to_string(),
        "-B".into(),
        "false".into(),
        "-f".into(),
        "html,karate:jsonl".into(),
    ];
    if dry {
        a.extend(["-D".into(), "-T".into(), "1".into()]);
    } else {
        a.extend(["-T".into(), threads.to_string()]);
    }
    a.extend(targets.iter().cloned());
    a
}

pub fn spawn(v: &Vendor, args: &[String], root: &Path) -> std::io::Result<Child> {
    let mut c = Command::new(&v.java);
    c.args(args).current_dir(root).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for k in STRIPPED_ENV {
        c.env_remove(k);
    }
    c.spawn()
}

pub fn events_file(out: &Path) -> PathBuf {
    out.join("karate-json").join("karate-events.jsonl")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Folder,
    Feature,
    Scenario,
    Outline,
    Row,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    NotRun,
    Queued,
    Running,
    Passed,
    Failed,
    Skipped,
}

pub struct Node {
    pub kind: Kind,
    pub name: String,
    /// The feature's title, shown dim after its file name.
    pub title: String,
    /// The project-relative path: a folder's, or the feature file of anything below it.
    pub file: String,
    pub line: Option<u64>,
    pub tags: Vec<String>,
    pub depth: usize,
    pub children: Vec<usize>,
    pub expanded: bool,
    pub status: Status,
    pub error: Option<String>,
    pub millis: Option<u64>,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    /// Tests (scenarios and outline rows) by feature path + refId: slugs collide, refIds do not.
    pub by_ref: HashMap<(String, String), usize>,
    pub tests: usize,
}

fn node(kind: Kind, name: &str, file: &str, depth: usize) -> Node {
    Node {
        kind,
        name: name.to_string(),
        title: String::new(),
        file: file.to_string(),
        line: None,
        tags: vec![],
        depth,
        children: vec![],
        expanded: kind == Kind::Folder,
        status: Status::NotRun,
        error: None,
        millis: None,
    }
}

fn tags_of(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|t| t.as_str().map(|s| s.trim_start_matches('@').to_string())).collect())
        .unwrap_or_default()
}

/// `buildTree` of tree.ts: folder > feature > scenario | outline > row, from the dry run's depth-0 events.
pub fn build_tree(events: &Path, root_name: &str) -> std::io::Result<Tree> {
    let mut nodes = vec![node(Kind::Folder, root_name, "", 0)];
    let mut folders: HashMap<String, usize> = HashMap::from([(String::new(), 0)]);
    let mut by_ref = HashMap::new();
    let mut feature: Option<usize> = None;
    let mut outline_ev: Option<Value> = None;
    let mut outline: Option<usize> = None;
    let mut tests = 0;

    fn folder_for(dir: &str, nodes: &mut Vec<Node>, folders: &mut HashMap<String, usize>) -> usize {
        if let Some(&i) = folders.get(dir) {
            return i;
        }
        let (parent_dir, name) = dir.rsplit_once('/').unwrap_or(("", dir));
        let parent = folder_for(parent_dir, nodes, folders);
        let i = nodes.len();
        let depth = nodes[parent].depth + 1;
        nodes.push(node(Kind::Folder, name, dir, depth));
        nodes[parent].children.push(i);
        folders.insert(dir.to_string(), i);
        i
    }

    for line in BufReader::new(File::open(events)?).lines() {
        let Ok(ev) = serde_json::from_str::<Value>(&line?) else { continue };
        let d = &ev["data"];
        if d["callDepth"].as_u64() != Some(0) {
            continue;
        }
        match ev["type"].as_str() {
            Some("FEATURE_ENTER") => {
                let file = d["path"].as_str().unwrap_or_default().to_string();
                let (dir, base) = file.rsplit_once('/').unwrap_or(("", &file));
                let parent = folder_for(dir, &mut nodes, &mut folders);
                let i = nodes.len();
                let mut n = node(Kind::Feature, base, &file, nodes[parent].depth + 1);
                n.title = d["name"].as_str().unwrap_or_default().to_string();
                n.tags = tags_of(&d["tags"]);
                n.line = d["line"].as_u64();
                nodes.push(n);
                nodes[parent].children.push(i);
                feature = Some(i);
                outline_ev = None;
                outline = None;
            }
            Some("OUTLINE_ENTER") => {
                outline_ev = Some(d.clone());
                outline = None;
            }
            Some("SCENARIO_ENTER") => {
                let Some(f) = feature else { continue };
                let file = nodes[f].file.clone();
                let ref_id = d["refId"].as_str().unwrap_or_default().to_string();
                let depth = nodes[f].depth + 1;
                let mut n = node(Kind::Scenario, d["name"].as_str().unwrap_or_default(), &file, depth);
                n.line = d["line"].as_u64();
                n.tags = tags_of(&d["tags"]);
                if d["isOutlineExample"].as_bool() == Some(true) {
                    let o = match outline {
                        Some(o) => o,
                        None => {
                            let oe = outline_ev.clone().unwrap_or(Value::Null);
                            let mut on = node(Kind::Outline, oe["name"].as_str().unwrap_or_default(), &file, depth);
                            on.line = oe["line"].as_u64();
                            on.tags = tags_of(&oe["tags"]);
                            let oi = nodes.len();
                            nodes.push(on);
                            nodes[f].children.push(oi);
                            outline = Some(oi);
                            oi
                        }
                    };
                    n.kind = Kind::Row;
                    n.depth = depth + 1;
                    let i = nodes.len();
                    nodes.push(n);
                    nodes[o].children.push(i);
                    by_ref.insert((file, ref_id), i);
                } else {
                    let i = nodes.len();
                    nodes.push(n);
                    nodes[f].children.push(i);
                    by_ref.insert((file, ref_id), i);
                    outline_ev = None;
                    outline = None;
                }
                tests += 1;
            }
            _ => {}
        }
    }
    // Folders first, then features, each by path; scenarios keep file order.
    let order: Vec<(usize, bool, String)> =
        nodes.iter().enumerate().map(|(i, n)| (i, n.kind == Kind::Folder, n.file.clone())).collect();
    for n in nodes.iter_mut() {
        if n.kind == Kind::Folder {
            n.children.sort_by(|a, b| {
                let (_, fa, pa) = &order[*a];
                let (_, fb, pb) = &order[*b];
                fb.cmp(fa).then(pa.cmp(pb))
            });
        }
    }
    Ok(Tree { nodes, by_ref, tests })
}

/// What a run sends to the window. The window drains them in batches, at most every 100 ms.
pub enum Msg {
    Started { file: String, ref_id: String },
    Finished { file: String, ref_id: String, status: Status, millis: u64, error: Option<String> },
    Line(String),
    Done { code: Option<i32>, cancelled: bool },
}

/// Tails `karate-events.jsonl` every 100 ms while the run lives, as runner.ts does, and reads the process output.
/// Nothing of this outlives the run: no thread or timer is left once `Done` is sent.
pub fn watch(
    mut child: Child,
    out: PathBuf,
    cancel: Arc<AtomicBool>,
    tx: futures::channel::mpsc::UnboundedSender<Msg>,
) {
    let lines_tx = tx.clone();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    for pipe in [stdout.map(|s| Box::new(s) as Box<dyn Read + Send>), stderr.map(|s| Box::new(s) as Box<dyn Read + Send>)]
        .into_iter()
        .flatten()
    {
        let tx = lines_tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = tx.unbounded_send(Msg::Line(line));
            }
        });
    }
    std::thread::spawn(move || {
        let path = events_file(&out);
        let mut offset = 0u64;
        let mut partial = String::new();
        let mut cancelled = false;
        loop {
            let exited = child.try_wait().ok().flatten();
            if cancel.load(Ordering::Relaxed) && exited.is_none() && !cancelled {
                cancelled = true;
                kill_tree(&mut child);
            }
            read_new(&path, &mut offset, &mut partial, &tx);
            if let Some(status) = exited {
                read_new(&path, &mut offset, &mut partial, &tx);
                let _ = tx.unbounded_send(Msg::Done { code: status.code(), cancelled });
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}

#[cfg(windows)]
fn kill_tree(child: &mut Child) {
    let _ = Command::new("taskkill").args(["/PID", &child.id().to_string(), "/T", "/F"]).status();
}

#[cfg(not(windows))]
fn kill_tree(child: &mut Child) {
    let _ = child.kill();
}

fn read_new(path: &Path, offset: &mut u64, partial: &mut String, tx: &futures::channel::mpsc::UnboundedSender<Msg>) {
    let Ok(mut f) = File::open(path) else { return };
    if f.seek(SeekFrom::Start(*offset)).is_err() {
        return;
    }
    let mut buf = String::new();
    let Ok(n) = f.read_to_string(&mut buf) else { return };
    *offset += n as u64;
    partial.push_str(&buf);
    let Some(cut) = partial.rfind('\n') else { return };
    let rest = partial.split_off(cut + 1);
    for line in partial.lines() {
        let Ok(ev) = serde_json::from_str::<Value>(line) else { continue };
        let d = &ev["data"];
        if d["callDepth"].as_u64() != Some(0) {
            continue;
        }
        let file = d["feature"].as_str().unwrap_or_default().to_string();
        let ref_id = d["refId"].as_str().unwrap_or_default().to_string();
        match ev["type"].as_str() {
            Some("SCENARIO_ENTER") => {
                let _ = tx.unbounded_send(Msg::Started { file, ref_id });
            }
            Some("SCENARIO_EXIT") => {
                // `skipped` before `passed`: a dry run sets both.
                let status = if d["skipped"].as_bool() == Some(true) {
                    Status::Skipped
                } else if d["passed"].as_bool() == Some(true) {
                    Status::Passed
                } else {
                    Status::Failed
                };
                let error = d["errorReason"].as_str().or(d["error"].as_str()).map(str::to_string);
                let millis = d["durationMillis"].as_u64().unwrap_or(0);
                let _ = tx.unbounded_send(Msg::Finished { file, ref_id, status, millis, error });
            }
            _ => {}
        }
    }
    *partial = rest;
}

/// Unit tests: the Vitest `*.test.ts` layer, against the same recorded fixtures as `tree.test.ts` and `events.test.ts`.
#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../test/fixtures/events").join(name)
    }

    #[test]
    fn builds_folder_feature_scenario_outline_row() {
        let t = build_tree(&fixture("dryrun-clean.jsonl"), "clean").unwrap();
        assert_eq!(t.tests, 9);
        let root = &t.nodes[0];
        assert_eq!(root.children.len(), 1);
        let tests = &t.nodes[root.children[0]];
        assert_eq!((tests.kind, tests.name.as_str()), (Kind::Folder, "tests"));
        let names: Vec<&str> = tests.children.iter().map(|&i| t.nodes[i].name.as_str()).collect();
        assert_eq!(names, ["basic.feature", "outline.feature", "slow.feature"]);
        let outline_feature = &t.nodes[tests.children[1]];
        let outline = &t.nodes[outline_feature.children[0]];
        assert_eq!((outline.kind, outline.name.as_str(), outline.children.len()), (Kind::Outline, "add <a> and <b>", 3));
        assert_eq!(t.nodes[outline.children[2]].kind, Kind::Row);
    }

    #[test]
    fn keys_tests_by_feature_and_ref_id() {
        let t = build_tree(&fixture("dryrun-clean.jsonl"), "clean").unwrap();
        let i = t.by_ref[&("tests/outline.feature".to_string(), "[1.2:7]".to_string())];
        assert_eq!(t.nodes[i].name, "add 2 and 3");
    }

    #[test]
    fn reads_live_results_in_batches_and_keeps_a_partial_line() {
        let src = std::fs::read_to_string(fixture("1-normal-run-failure.jsonl")).unwrap();
        let dir = std::env::temp_dir().join(format!("grace-gpui-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        // The file grows as Karate writes it: first half a line, then the rest.
        let cut = src.len() / 2;
        std::fs::write(&path, &src[..cut]).unwrap();
        let (tx, mut rx) = futures::channel::mpsc::unbounded();
        let (mut offset, mut partial) = (0u64, String::new());
        read_new(&path, &mut offset, &mut partial, &tx);
        std::fs::write(&path, &src).unwrap();
        read_new(&path, &mut offset, &mut partial, &tx);
        drop(tx);
        let mut failed = 0;
        let mut finished = 0;
        while let Ok(m) = rx.try_recv() {
            if let Msg::Finished { status, error, .. } = m {
                finished += 1;
                if status == Status::Failed {
                    failed += 1;
                    assert!(error.unwrap().contains("match failed"));
                }
            }
        }
        assert_eq!((finished, failed), (src.matches("\"SCENARIO_EXIT\"").count() - nested(&src), 1));
        std::fs::remove_dir_all(dir).ok();
    }

    /// SCENARIO_EXIT events of called features (callDepth > 0), which the tree ignores.
    fn nested(src: &str) -> usize {
        src.lines()
            .filter(|l| l.contains("\"SCENARIO_EXIT\"") && !l.contains("\"callDepth\":0"))
            .count()
    }

    #[test]
    fn dry_run_has_one_thread_and_c1_only() {
        let a = args(Path::new("k.jar"), Path::new("/p"), Path::new("/o"), true, 4, &["/p".into()]);
        assert!(a.contains(&"-XX:TieredStopAtLevel=1".to_string()));
        let t = a.iter().position(|x| x == "-T").unwrap();
        assert_eq!(a[t + 1], "1");
        assert_eq!(a.last().unwrap(), "/p");
    }
}
