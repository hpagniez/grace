//! Option 1: the thin path of src/main ported to Rust. Open (dry run, tree), run with live progress
//! from karate-events.jsonl, cancel with taskkill. Emits the same AppEvent JSON as src/main, built
//! with `json!`: nothing checks it against src/shared/events.ts.
//! Left out (src/main has them): pre-run discovery, row lines, filters, environments, properties,
//! history, flaky tests, steps and HTTP calls, the pid file and orphan kill.
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Lower-cased: Windows environment keys come in any case.
const STRIPPED: [&str; 6] = [
    "karate_options",
    "karate_env",
    "karate_config_dir",
    "java_tool_options",
    "jdk_java_options",
    "_java_options",
];

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    key: String,
    kind: &'static str,
    name: String,
    file: String,
    line: Option<i64>,
    ref_id: Option<String>,
    tags: Vec<String>,
    children: Vec<Node>,
}

impl Node {
    fn new(kind: &'static str, key: String, name: String, file: String) -> Self {
        Node {
            key,
            kind,
            name,
            file,
            line: None,
            ref_id: None,
            tags: vec![],
            children: vec![],
        }
    }
}

fn tags(d: &Value) -> Vec<String> {
    d["tags"].as_array().map_or(vec![], |a| {
        a.iter()
            .filter_map(|t| t.as_str().map(String::from))
            .collect()
    })
}

/// `[section:line]` or `[section.example:line]`.
fn section(ref_id: &str) -> Option<&str> {
    ref_id.strip_prefix('[')?.split([':', '.']).next()
}

/// Port of src/main/karate/tree.ts buildTree: folder > feature > scenario | outline > row.
pub fn build_tree(events: &[Value], root_name: &str) -> Node {
    let mut features: Vec<Node> = vec![];
    let mut outline_ev: Option<Value> = None;
    let mut outline: Option<usize> = None;
    for ev in events {
        let d = &ev["data"];
        if d["callDepth"].as_i64() != Some(0) {
            continue;
        }
        match ev["type"].as_str() {
            Some("FEATURE_ENTER") => {
                let file = d["path"].as_str().unwrap_or_default().to_string();
                let base = file.rsplit('/').next().unwrap_or_default().to_string();
                let name = d["name"]
                    .as_str()
                    .filter(|n| !n.is_empty())
                    .map_or(base, String::from);
                let mut f = Node::new("feature", format!("f:{file}"), name, file);
                f.tags = tags(d);
                features.push(f);
                (outline_ev, outline) = (None, None);
            }
            Some("OUTLINE_ENTER") => (outline_ev, outline) = (Some(d.clone()), None),
            Some("SCENARIO_ENTER") => {
                let Some(f) = features.last_mut() else {
                    continue;
                };
                let ref_id = d["refId"].as_str().unwrap_or_default();
                let Some(sec) = section(ref_id) else { continue };
                let name = d["name"].as_str().unwrap_or_default().to_string();
                if !d["isOutlineExample"].as_bool().unwrap_or(false) {
                    let mut s = Node::new(
                        "scenario",
                        format!("s:{}#{sec}", f.file),
                        name,
                        f.file.clone(),
                    );
                    (s.line, s.ref_id, s.tags) = (d["line"].as_i64(), Some(ref_id.into()), tags(d));
                    f.children.push(s);
                    (outline_ev, outline) = (None, None);
                    continue;
                }
                let Some(o) = &outline_ev else { continue };
                let i = *outline.get_or_insert_with(|| {
                    let name = o["name"].as_str().unwrap_or_default().to_string();
                    let mut n = Node::new(
                        "outline",
                        format!("o:{}#{sec}", f.file),
                        name,
                        f.file.clone(),
                    );
                    (n.line, n.tags) = (o["line"].as_i64(), tags(o));
                    f.children.push(n);
                    f.children.len() - 1
                });
                let o = &mut f.children[i];
                let mut r = Node::new(
                    "row",
                    format!("s:{}#{sec}.{}", o.file, o.children.len() + 1),
                    name,
                    o.file.clone(),
                );
                (r.ref_id, r.tags) = (Some(ref_id.into()), tags(d));
                o.children.push(r);
            }
            _ => {}
        }
    }
    let mut root = Node::new("folder", "d:".into(), root_name.into(), String::new());
    for f in features {
        let dirs: Vec<String> = f.file.split('/').map(String::from).collect();
        insert(&mut root, &dirs[..dirs.len() - 1], f);
    }
    sort_folders(&mut root);
    root
}

fn insert(folder: &mut Node, dirs: &[String], feature: Node) {
    let Some(first) = dirs.first() else {
        return folder.children.push(feature);
    };
    let dir = if folder.file.is_empty() {
        first.clone()
    } else {
        format!("{}/{first}", folder.file)
    };
    let i = match folder
        .children
        .iter()
        .position(|c| c.kind == "folder" && c.file == dir)
    {
        Some(i) => i,
        None => {
            folder
                .children
                .push(Node::new("folder", format!("d:{dir}"), first.clone(), dir));
            folder.children.len() - 1
        }
    };
    insert(&mut folder.children[i], &dirs[1..], feature);
}

fn sort_folders(folder: &mut Node) {
    let (mut folders, mut features): (Vec<Node>, Vec<Node>) =
        folder.children.drain(..).partition(|c| c.kind == "folder");
    folders.sort_by(|a, b| a.file.cmp(&b.file));
    features.sort_by(|a, b| a.file.cmp(&b.file));
    folders.iter_mut().for_each(sort_folders);
    folder.children = folders.into_iter().chain(features).collect();
}

fn find<'a>(n: &'a Node, key: &str) -> Option<&'a Node> {
    if n.key == key {
        return Some(n);
    }
    n.children.iter().find_map(|c| find(c, key))
}

fn leaves<'a>(n: &'a Node, out: &mut Vec<&'a Node>) {
    if n.kind == "scenario" || n.kind == "row" {
        out.push(n);
    }
    n.children.iter().for_each(|c| leaves(c, out));
}

/// Reads new complete lines of a growing JSONL file; a partial last line waits for its newline.
struct Tail {
    file: PathBuf,
    offset: u64,
}

impl Tail {
    fn read(&mut self) -> Vec<Value> {
        let Ok(mut f) = File::open(&self.file) else {
            return vec![];
        };
        let mut buf = vec![];
        if f.seek(SeekFrom::Start(self.offset))
            .and_then(|_| f.read_to_end(&mut buf))
            .is_err()
        {
            return vec![];
        }
        let Some(end) = buf.iter().rposition(|&b| b == b'\n') else {
            return vec![];
        };
        self.offset += end as u64 + 1;
        String::from_utf8_lossy(&buf[..end])
            .lines()
            .filter_map(|l| serde_json::from_str(l.trim()).ok())
            .collect()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Full path: a taskkill.exe earlier on PATH must not be run instead.
pub fn taskkill(pid: u32) {
    let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let _ = Command::new(
        Path::new(&system_root)
            .join("System32")
            .join("taskkill.exe"),
    )
    .args(["/PID", &pid.to_string(), "/T", "/F"])
    .creation_flags(CREATE_NO_WINDOW)
    .status();
}

pub struct Vendor {
    pub java: PathBuf,
    pub jar: PathBuf,
}

#[derive(Default)]
struct State {
    project: Option<Value>,
    tree: Option<Node>,
    error: Option<String>,
    feature_roots: Vec<PathBuf>,
    /// The folder `open` is discovering, as Runner.opening.
    opening: Option<PathBuf>,
    busy: bool,
    pid: Option<u32>,
    cancelled: bool,
}

pub struct Runner {
    vendor: Vendor,
    data: PathBuf,
    emit: Box<dyn Fn(Value) + Send + Sync>,
    state: Mutex<State>,
}

impl Runner {
    pub fn new(
        vendor: Vendor,
        data: PathBuf,
        emit: impl Fn(Value) + Send + Sync + 'static,
    ) -> Self {
        Runner {
            vendor,
            data,
            emit: Box::new(emit),
            state: Mutex::default(),
        }
    }

    fn emit(&self, e: Value) {
        (self.emit)(e)
    }

    fn notice(&self, message: &str) {
        self.emit(json!({ "type": "notice", "message": message }));
    }

    fn project_event(s: &State) -> Value {
        json!({ "type": "project", "project": s.project, "tree": s.tree, "error": s.error })
    }

    /// What a page that just loaded needs, like Runner.replay. The page shows nothing until `opening`.
    pub fn replay(&self) {
        let (project, opening, busy) = {
            let s = self.state.lock().unwrap();
            (
                s.project.is_some().then(|| Self::project_event(&s)),
                s.opening.clone(),
                s.busy,
            )
        };
        if let Some(p) = project {
            self.emit(p);
            self.emit(json!({ "type": "filter", "filter": { "include": [], "exclude": [], "hideNonMatching": false } }));
            self.emit(json!({ "type": "threads", "count": 1 }));
        }
        self.emit(json!({ "type": "opening", "root": opening }));
        self.emit(json!({ "type": "busy", "busy": busy }));
        self.emit(json!({ "type": "recent", "entries": [] }));
    }

    /// False if a discovery or run is already going.
    fn start(&self) -> bool {
        let mut s = self.state.lock().unwrap();
        if s.busy {
            drop(s);
            self.notice("Wait for the current run to end");
            return false;
        }
        (s.busy, s.cancelled) = (true, false);
        drop(s);
        self.emit(json!({ "type": "busy", "busy": true }));
        true
    }

    fn end(&self) {
        let mut s = self.state.lock().unwrap();
        (s.busy, s.pid) = (false, None);
        drop(s);
        self.emit(json!({ "type": "busy", "busy": false }));
    }

    fn args(
        &self,
        root: &Path,
        classpath: &[PathBuf],
        out: &Path,
        dry: bool,
        targets: &[String],
    ) -> Vec<String> {
        let cp: Vec<String> = std::iter::once(&self.vendor.jar)
            .chain(classpath)
            .map(|p| p.display().to_string())
            .collect();
        let mut a: Vec<String> = [
            "-Dstdout.encoding=UTF-8",
            "-Dstderr.encoding=UTF-8",
            "-cp",
            &cp.join(";"),
            "io.karatelabs.Main",
            "run",
            "--no-color",
            "-w",
            &root.display().to_string(),
            "-o",
            &out.display().to_string(),
            "-B",
            "false",
            "-f",
            "html,karate:jsonl",
        ]
        .map(String::from)
        .into();
        a.extend(
            if dry {
                ["-D", "-T", "1"].as_slice()
            } else {
                ["-T", "1"].as_slice()
            }
            .iter()
            .map(|s| s.to_string()),
        );
        a.extend_from_slice(targets);
        a
    }

    /// Never through a shell; no console window; the four Java and Karate option variables removed.
    fn spawn(&self, root: &Path, args: &[String]) -> std::io::Result<Child> {
        let mut c = Command::new(&self.vendor.java);
        c.args(args)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW);
        for (k, _) in std::env::vars() {
            if STRIPPED.contains(&k.to_lowercase().as_str()) {
                c.env_remove(k);
            }
        }
        let child = c.spawn()?;
        self.state.lock().unwrap().pid = Some(child.id());
        Ok(child)
    }

    /// Collects both pipes' lines; the caller takes them every 100 ms.
    fn pump(child: &mut Child) -> (Arc<Mutex<Vec<String>>>, Vec<thread::JoinHandle<()>>) {
        let lines = Arc::new(Mutex::new(vec![]));
        let mut joins = vec![];
        let streams: [Box<dyn Read + Send>; 2] = [
            Box::new(child.stdout.take().unwrap()),
            Box::new(child.stderr.take().unwrap()),
        ];
        for s in streams {
            let lines = lines.clone();
            joins.push(thread::spawn(move || {
                for l in BufReader::new(s).lines().map_while(Result::ok) {
                    lines.lock().unwrap().push(l);
                }
            }));
        }
        (lines, joins)
    }

    pub fn open(&self, root: PathBuf) {
        if !self.start() {
            return;
        }
        self.state.lock().unwrap().opening = Some(root.clone());
        self.emit(json!({ "type": "opening", "root": root }));
        let has = |f: &str| root.join(f).exists();
        let layout = if has("pom.xml") {
            "maven"
        } else if has("build.gradle") || has("build.gradle.kts") {
            "gradle"
        } else {
            "standalone"
        };
        let feature_roots: Vec<PathBuf> = if layout == "standalone" {
            vec![root.clone()]
        } else {
            [r"src\test\java", r"src\test\resources"]
                .iter()
                .map(|d| root.join(d))
                .filter(|d| d.exists())
                .collect()
        };
        let out = self.data.join("dry-run");
        let _ = fs::remove_dir_all(&out);
        let targets: Vec<String> = feature_roots
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        let args = self.args(&root, &feature_roots, &out, true, &targets);
        let (tree, error) = match self.spawn(&root, &args) {
            Err(e) => (None, Some(e.to_string())),
            Ok(mut child) => {
                let (lines, joins) = Self::pump(&mut child);
                let code = child.wait().ok().and_then(|s| s.code());
                joins.into_iter().for_each(|j| drop(j.join()));
                if code == Some(0) {
                    let events = Tail {
                        file: out.join("karate-json").join("karate-events.jsonl"),
                        offset: 0,
                    }
                    .read();
                    let name = root
                        .file_name()
                        .map_or(String::new(), |n| n.to_string_lossy().into());
                    (Some(build_tree(&events, &name)), None)
                } else {
                    let lines = lines.lock().unwrap();
                    (
                        None,
                        Some(
                            lines
                                .iter()
                                .rev()
                                .take(5)
                                .rev()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ),
                    )
                }
            }
        };
        let cancelled = {
            let mut s = self.state.lock().unwrap();
            s.opening = None;
            if !s.cancelled {
                s.project = Some(json!({ "root": root, "layout": layout }));
                (s.tree, s.error, s.feature_roots) = (tree, error, feature_roots);
            }
            s.cancelled
        };
        if !cancelled {
            self.replay(); // ends with `opening` null
        } else {
            self.emit(json!({ "type": "opening", "root": null }));
        }
        self.end();
    }

    pub fn run(&self, key: &str) {
        let (root, tree, feature_roots) = {
            let s = self.state.lock().unwrap();
            let Some(root) = s
                .project
                .as_ref()
                .and_then(|p| p["root"].as_str())
                .map(PathBuf::from)
            else {
                return;
            };
            let Some(tree) = s.tree.clone() else { return };
            (root, tree, s.feature_roots.clone())
        };
        let Some(node) = find(&tree, key) else {
            return self.notice("This test changed; the tree was refreshed");
        };
        let abs = root.join(node.file.replace('/', "\\"));
        let targets: Vec<String> = match (node.kind, node.line) {
            ("folder", _) if node.file.is_empty() => feature_roots
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            ("folder" | "feature", _) => vec![abs.display().to_string()],
            (_, Some(line)) => vec![format!("{}:{line}", abs.display())],
            _ => return self.notice("Run the outline to run its rows"),
        };
        let mut picked = vec![];
        leaves(node, &mut picked);
        let keys: Vec<String> = picked.iter().map(|n| n.key.clone()).collect();
        let index: HashMap<String, String> = picked
            .iter()
            .filter_map(|n| Some((format!("{}|{}", n.file, n.ref_id.as_ref()?), n.key.clone())))
            .collect();
        if !self.start() {
            return;
        }
        let started = now_ms();
        let id = chrono_like_id(started);
        let out = self.data.join("runs").join(&id);
        let args = self.args(&root, &feature_roots, &out, false, &targets);
        let mut child = match self.spawn(&root, &args) {
            Ok(c) => c,
            Err(e) => {
                self.notice(&e.to_string());
                return self.end();
            }
        };
        self.emit(
            json!({ "type": "runStarted", "selection": key, "runId": id, "keys": keys,
            "total": keys.len(), "startedAt": started, "env": null }),
        );
        let (lines, joins) = Self::pump(&mut child);
        let mut tail = Tail {
            file: out.join("karate-json").join("karate-events.jsonl"),
            offset: 0,
        };
        let mut status: HashMap<String, &'static str> = HashMap::new();
        let mut summary = Value::Null;
        let mut suite_exit = false;
        let mut step = |status: &mut HashMap<String, &'static str>| {
            let mut updates: Vec<Value> = vec![];
            for ev in tail.read() {
                let d = &ev["data"];
                let t = ev["type"].as_str().unwrap_or_default();
                if t == "SUITE_EXIT" {
                    suite_exit = true;
                    let s = &d["summary"];
                    summary = json!({ "scenarioCount": s["scenarioCount"], "scenariosPassed": s["scenariosPassed"],
                        "scenariosFailed": s["scenariosFailed"], "durationMillis": s["durationMillis"] });
                    continue;
                }
                if d["callDepth"].as_i64() != Some(0)
                    || !(t == "SCENARIO_ENTER" || t == "SCENARIO_EXIT")
                {
                    continue;
                }
                let k = format!(
                    "{}|{}",
                    d["feature"].as_str().unwrap_or_default(),
                    d["refId"].as_str().unwrap_or_default()
                );
                let Some(key) = index.get(&k) else { continue };
                let mut u = json!({ "key": key, "status": "running" });
                if t == "SCENARIO_EXIT" {
                    // skipped before passed: a dry run sets both
                    let s = if d["skipped"] == true {
                        "notRun"
                    } else if d["passed"] == true {
                        "passed"
                    } else {
                        "failed"
                    };
                    u["status"] = s.into();
                    u["durationMillis"] = d["durationMillis"].clone();
                    if s == "failed" && d["error"].is_string() {
                        u["error"] = d["error"].clone();
                    }
                }
                status.insert(
                    key.clone(),
                    if u["status"] == "running" {
                        "running"
                    } else {
                        "ended"
                    },
                );
                updates.push(u);
            }
            if !updates.is_empty() {
                self.emit(json!({ "type": "scenarios", "updates": updates }));
            }
            let batch: Vec<String> = std::mem::take(&mut *lines.lock().unwrap());
            if !batch.is_empty() {
                self.emit(json!({ "type": "console", "lines": batch }));
            }
        };
        let code = loop {
            let exited = child.try_wait();
            step(&mut status);
            match exited {
                Ok(Some(s)) => break s.code(),
                Ok(None) => thread::sleep(Duration::from_millis(100)),
                Err(_) => break None,
            }
        };
        joins.into_iter().for_each(|j| drop(j.join()));
        step(&mut status);
        let cancelled = self.state.lock().unwrap().cancelled;
        let outcome = if cancelled {
            "cancelled"
        } else if suite_exit && matches!(code, Some(0 | 1)) {
            "finished"
        } else {
            "failed"
        };
        // A finished suite ran all it would: a test with no SCENARIO_EXIT did not run.
        let (unended, to): (Vec<&String>, &str) = if outcome == "finished" {
            (
                keys.iter()
                    .filter(|k| status.get(*k).is_none_or(|s| *s == "running"))
                    .collect(),
                "notRun",
            )
        } else {
            (
                keys.iter()
                    .filter(|k| status.get(*k) == Some(&"running"))
                    .collect(),
                "interrupted",
            )
        };
        if !unended.is_empty() {
            let updates: Vec<Value> = unended
                .iter()
                .map(|k| json!({ "key": k, "status": to }))
                .collect();
            self.emit(json!({ "type": "scenarios", "updates": updates }));
        }
        let report = outcome == "finished" && out.join("karate-summary.html").exists();
        self.emit(
            json!({ "type": "runFinished", "outcome": outcome, "exitCode": code,
            "summary": summary, "reportAvailable": report }),
        );
        self.end();
    }

    pub fn cancel(&self) {
        let pid = {
            let mut s = self.state.lock().unwrap();
            if !s.busy {
                return;
            }
            s.cancelled = true;
            s.pid
        };
        if let Some(pid) = pid {
            taskkill(pid);
        }
    }
}

/// `YYYYMMDD-HHMMSS-mmm` in UTC, the shape of src/shared/history.ts RUN_ID (Grace uses local time).
fn chrono_like_id(ms: u64) -> String {
    let (secs, millis) = (ms / 1000, ms % 1000);
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // civil-from-days, Howard Hinnant's algorithm
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}-{millis:03}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(file: &str) -> Vec<Value> {
        fs::read_to_string(file)
            .unwrap()
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// Same tree as src/main/karate/tree.test.ts builds from the recorded clean-project dry run.
    #[test]
    fn builds_the_clean_project_tree() {
        let tree = build_tree(
            &events("../../../../test/fixtures/events/dryrun-clean.jsonl"),
            "clean",
        );
        let mut all = vec![];
        leaves(&tree, &mut all);
        assert_eq!(tree.key, "d:");
        assert!(tree.children.iter().any(|c| c.kind == "folder"));
        assert!(!all.is_empty());
        assert!(all.iter().all(|n| n.ref_id.is_some()));
    }

    #[test]
    fn run_ids_match_grace_shape() {
        assert_eq!(chrono_like_id(1_761_726_955_783), "20251029-083555-783");
    }
}
