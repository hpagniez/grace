//! `src/main/karate/libraries.ts`: a Maven or Gradle project's libraries, listed by its own build
//! (docs/specs/2026-11-02-project-libraries.md, ADR 0027), and finding that build's tool
//! (docs/specs/2026-11-03-find-build-tools.md).
//!
//! What differs by platform sits in `find_tool` (the launchers' names, IntelliJ IDEA's Maven on Windows) and in
//! `tool_start`: on Windows the wrappers and `mvn.cmd` are batch files, which only `cmd.exe` runs, so every path is a
//! variable it expands inside quotes; Linux starts the launcher itself with an argument list.
use super::command::{Env, absolute};
use super::js;
use crate::model::Layout;
use crate::model::events::BuildTool;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// R3.2: adds `graceClasspath` to the root project; it writes the test runtime classpath to GRACE_CLASSPATH_FILE.
pub const INIT_SCRIPT: &str = r#"// Grace: the test runtime classpath of the root project (docs/specs/2026-11-02-project-libraries.md R3.2).
rootProject {
    tasks.register('graceClasspath') {
        def files = project.extensions.getByName('sourceSets').getByName('test').runtimeClasspath
        def out = new File(System.getenv('GRACE_CLASSPATH_FILE'))
        doLast { out.setText(files.files.join(File.pathSeparator), 'UTF-8') }
    }
}
"#;

/// The last list the build gave, kept in `<project data folder>/libraries.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptLibraries {
    pub tool: Layout,
    pub key: String,
    pub jars: Vec<String>,
}

/// What Grace asks the build tool to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Goal {
    /// List the test classpath into the classpath file (R3).
    List,
    /// The plain compile of the test code (build from the notice R2.1).
    Build,
}

/// How to start a build tool: the program, its arguments, and whether they are a Windows command line as written.
pub struct ToolStart {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub verbatim_args: bool,
}

/// What `tool_env` adds to the person's environment.
pub struct ToolEnv<'a> {
    pub tool: &'a Path,
    pub file: &'a Path,
    pub init_script: &'a Path,
    /// The PATH Java is looked for on.
    pub search_path: &'a OsStr,
    /// The bundled Java, lent only when there is no other; None for Gradle.
    pub jre_dir: Option<&'a Path>,
}

const KEPT: &str = "libraries.json";

/// `File.pathSeparator`, which Maven and the init script join the list with.
const LIST_SEPARATOR: char = if cfg!(windows) { ';' } else { ':' };
const JAVA: &str = if cfg!(windows) { "java.exe" } else { "java" };
pub(crate) const JAVAC: &str = if cfg!(windows) { "javac.exe" } else { "javac" };

static ERROR_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((\[ERROR\]|\[FATAL\]|ERROR:)\s*)+").expect("a valid pattern"));
static JAVAC_ERROR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\.java:[0-9]+: error: ").expect("a valid pattern"));
/// `build.txt`'s `IU-` of `IU-261.24374.151`.
#[cfg(windows)]
static PRODUCT_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new("^[A-Z]+-").expect("a valid pattern"));

/// The wrapper in a project, and the launchers looked for elsewhere.
fn names(tool: BuildTool) -> (&'static str, &'static [&'static str]) {
    match tool {
        BuildTool::Maven if cfg!(windows) => ("mvnw.cmd", &["mvn.cmd", "mvn.bat"]),
        BuildTool::Maven => ("mvnw", &["mvn"]),
        BuildTool::Gradle if cfg!(windows) => ("gradlew.bat", &["gradle.bat"]),
        BuildTool::Gradle => ("gradlew", &["gradle"]),
    }
}

/// The folders of a PATH; Windows reads a quoted entry as the folder inside its quotes, as `cmd.exe` does. An empty
/// entry is no folder.
fn path_folders(path: &OsStr) -> impl Iterator<Item = PathBuf> {
    std::env::split_paths(path).filter(|dir| !dir.as_os_str().is_empty())
}

/// A folder named by a variable, without the quotes Windows people put around one.
fn unquoted(value: &OsStr) -> PathBuf {
    if cfg!(windows) {
        value.to_string_lossy().replace('"', "").into()
    } else {
        value.into()
    }
}

fn same_variable(key: &OsStr, name: &str) -> bool {
    if cfg!(windows) {
        key.eq_ignore_ascii_case(name)
    } else {
        key == name
    }
}

/// In order, and only as far as needed: PATH, the tool's home variables, then IntelliJ IDEA's Maven.
fn tool_folders(tool: BuildTool, env: &Env) -> impl Iterator<Item = PathBuf> {
    let homes: &[&str] = match tool {
        BuildTool::Maven => &["MAVEN_HOME", "M2_HOME"],
        BuildTool::Gradle => &["GRADLE_HOME"],
    };
    path_folders(var(env, "PATH").unwrap_or_default())
        .chain(
            homes
                .iter()
                .filter_map(move |home| var(env, home))
                .map(|home| unquoted(home).join("bin")),
        )
        // the IDEs are looked for only once nothing before them was found
        .chain(std::iter::once_with(move || idea_mavens(tool, env)).flatten())
}

/// The `bin` folder of each IntelliJ IDEA's own Maven, the highest `build.txt` first: installed for all users in
/// `%ProgramFiles%\JetBrains`, for one user or by the Toolbox App in `%LOCALAPPDATA%\Programs`
/// (find-build-tools R1.3).
#[cfg(windows)]
fn idea_mavens(tool: BuildTool, env: &Env) -> Vec<PathBuf> {
    if tool != BuildTool::Maven {
        return Vec::new();
    }
    // the build less its product code, number by number; none (no build.txt) comes last
    let build = |ide: &Path| -> Vec<u64> {
        let text = fs::read_to_string(ide.join("build.txt")).unwrap_or_default();
        let build = PRODUCT_CODE.replace(js::trim(&text), "");
        build.split('.').map_while(|n| n.parse().ok()).collect()
    };
    let mut found: Vec<(Vec<u64>, PathBuf)> = [("ProgramFiles", "JetBrains"), ("LOCALAPPDATA", "Programs")]
        .iter()
        .filter_map(|(name, folder)| Some(Path::new(var(env, name)?).join(folder)))
        .filter_map(|parent| fs::read_dir(parent).ok())
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let ide = entry.path();
            let bin = ["plugins", "maven", "lib", "maven3", "bin"]
                .iter()
                .fold(ide.clone(), |p, s| p.join(s));
            bin.join("mvn.cmd").is_file().then(|| (build(&ide), bin))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, bin)| bin).collect()
}

/// IntelliJ IDEA's Maven is looked for on Windows only.
#[cfg(unix)]
fn idea_mavens(_tool: BuildTool, _env: &Env) -> Vec<PathBuf> {
    Vec::new()
}

/// R4.2: the files whose change can change the list.
fn build_files(tool: BuildTool) -> &'static [&'static str] {
    match tool {
        BuildTool::Maven => &[
            "pom.xml",
            ".mvn/maven.config",
            ".mvn/extensions.xml",
            ".mvn/wrapper/maven-wrapper.properties",
        ],
        BuildTool::Gradle => &[
            "build.gradle",
            "build.gradle.kts",
            "settings.gradle",
            "settings.gradle.kts",
            "gradle.properties",
            "gradle/libs.versions.toml",
            "gradle/wrapper/gradle-wrapper.properties",
        ],
    }
}

/// The last `> ` cause under Gradle's `* What went wrong:`, else its first line; the first names only the task.
fn gradle_cause<'a>(said: &[&'a str]) -> Option<&'a str> {
    let start = said.iter().position(|l| *l == "* What went wrong:")?;
    let block = said[start + 1..].split(|l| l.is_empty()).next()?;
    match block.iter().rev().find_map(|l| l.strip_prefix('>')) {
        Some(cause) => Some(js::trim(cause)),
        None => block.first().copied(),
    }
}

/// R2 and find-build-tools R1: the project's wrapper, else the first launcher `env` leads to: on PATH, then under the
/// tool's home variables, then (Windows, Maven) IntelliJ IDEA's own.
pub fn find_tool(root: &Path, tool: BuildTool, env: &Env) -> Option<PathBuf> {
    let (wrapper, launchers) = names(tool);
    let wrapper = root.join(wrapper);
    if wrapper.is_file() {
        return Some(wrapper);
    }
    tool_folders(tool, env)
        .flat_map(|dir| launchers.iter().map(move |name| dir.join(name)))
        .find(|launcher| launcher.is_file())
}

/// find-build-tools R2: where `find_tool` looked.
pub fn no_tool_reason(tool: BuildTool) -> &'static str {
    match tool {
        BuildTool::Maven if cfg!(windows) => {
            "the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA"
        }
        BuildTool::Maven => "the project has no mvnw, and mvn is not on PATH, in MAVEN_HOME or in M2_HOME",
        BuildTool::Gradle if cfg!(windows) => {
            "the project has no gradlew.bat, and gradle is not on PATH or in GRADLE_HOME"
        }
        BuildTool::Gradle => "the project has no gradlew, and gradle is not on PATH or in GRADLE_HOME",
    }
}

/// R4.2: the SHA-256 of the name and content of each file whose change can change the list.
pub fn library_key(root: &Path, tool: BuildTool) -> String {
    let mut hash = Sha256::new();
    for name in build_files(tool) {
        // "-" stands for an absent file
        let content = fs::read(root.join(name)).unwrap_or_else(|_| b"-".to_vec());
        for part in [name.as_bytes(), b"\0", &content, b"\0"] {
            hash.update(part);
        }
    }
    hash.finalize().iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The kept list, or None when missing or not of that shape.
pub fn read_kept(dir: &Path) -> Option<KeptLibraries> {
    let bytes = fs::read(dir.join(KEPT)).ok()?;
    let kept: KeptLibraries = serde_json::from_slice(&bytes).ok()?;
    (kept.tool != Layout::Standalone).then_some(kept)
}

pub fn write_kept(dir: &Path, kept: &KeptLibraries) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join(KEPT), serde_json::to_string(kept)?)
}

/// R4.3: the tool must run again.
pub fn is_stale(kept: Option<&KeptLibraries>, tool: BuildTool, key: &str) -> bool {
    !kept.is_some_and(|k| k.tool == tool.layout() && k.key == key && k.jars.iter().all(|j| Path::new(j).exists()))
}

/// R6.2: what a run puts on the classpath, as kept, even if stale.
pub fn kept_jars(dir: &Path, layout: Layout) -> Vec<PathBuf> {
    match read_kept(dir) {
        Some(kept) if kept.tool == layout => kept.jars.into_iter().map(PathBuf::from).collect(),
        _ => Vec::new(),
    }
}

/// R3: run by `cmd.exe /d /s /c "<command>"` on Windows. Every path is a variable that cmd.exe expands inside quotes,
/// so no path is ever parsed as part of the command (`&` or `%` in a folder name).
pub fn windows_command(tool: BuildTool, goal: Goal) -> &'static str {
    match (tool, goal) {
        (BuildTool::Maven, Goal::List) => {
            r#""%GRACE_TOOL%" -q -B dependency:build-classpath "-Dmdep.outputFile=%GRACE_CLASSPATH_FILE%""#
        }
        (BuildTool::Gradle, Goal::List) => {
            r#""%GRACE_TOOL%" -q --no-daemon --console=plain --no-configuration-cache --init-script "%GRACE_INIT_SCRIPT%" graceClasspath"#
        }
        (BuildTool::Maven, Goal::Build) => r#""%GRACE_TOOL%" -B -q test-compile"#,
        (BuildTool::Gradle, Goal::Build) => r#""%GRACE_TOOL%" -q --no-daemon --console=plain testClasses"#,
    }
}

/// The start of `launcher` for `goal`, in this platform's way. The environment is `tool_env`'s.
pub fn tool_start(tool: BuildTool, goal: Goal, launcher: &Path, file: &Path, init_script: &Path) -> ToolStart {
    #[cfg(windows)]
    {
        let _ = (launcher, file, init_script); // cmd.exe expands them from `tool_env`'s variables
        let command = format!("\"{}\"", windows_command(tool, goal));
        ToolStart {
            program: super::process::cmd(),
            args: ["/d", "/s", "/c", &command].map(OsString::from).to_vec(),
            verbatim_args: true,
        }
    }
    #[cfg(unix)]
    {
        let args: &[&str] = match (tool, goal) {
            (BuildTool::Maven, Goal::List) => &["-q", "-B", "dependency:build-classpath"],
            (BuildTool::Gradle, Goal::List) => &[
                "-q",
                "--no-daemon",
                "--console=plain",
                "--no-configuration-cache",
                "--init-script",
            ],
            (BuildTool::Maven, Goal::Build) => &["-B", "-q", "test-compile"],
            (BuildTool::Gradle, Goal::Build) => &["-q", "--no-daemon", "--console=plain", "testClasses"],
        };
        let mut args: Vec<OsString> = args.iter().map(OsString::from).collect();
        match (tool, goal) {
            (BuildTool::Maven, Goal::List) => {
                let mut output = OsString::from("-Dmdep.outputFile=");
                output.push(file);
                args.push(output);
            }
            (BuildTool::Gradle, Goal::List) => args.extend([init_script.into(), "graceClasspath".into()]),
            (_, Goal::Build) => {}
        }
        ToolStart {
            program: launcher.to_path_buf(),
            args,
            verbatim_args: false,
        }
    }
}

/// Build from the notice R5.2: `javac` in `JAVA_HOME`'s `bin` or in a PATH folder.
pub fn has_jdk(env: &Env) -> bool {
    var(env, "JAVA_HOME")
        .map(|home| unquoted(home).join("bin"))
        .into_iter()
        .chain(path_folders(var(env, "PATH").unwrap_or_default()))
        .any(|dir| dir.join(JAVAC).is_file())
}

/// Build from the notice R5.1–R5.2: javac's first error line (Gradle prints it as javac wrote it), else R6.3's reason;
/// a full stop unless it ends with a question, and the JDK sentence when there is no JDK.
pub fn build_failure(tool: BuildTool, lines: &[String], jdk: bool) -> String {
    let reason = lines
        .iter()
        .find(|l| JAVAC_ERROR.is_match(l))
        .map_or_else(|| failure_reason(lines), |l| js::trim(l).to_string());
    let stop = if reason.ends_with('?') { "" } else { "." };
    let need = if jdk {
        ""
    } else {
        " Building needs a JDK: set JAVA_HOME to one."
    };
    format!("Grace could not build the project's Java code with {tool:?}: {reason}{stop}{need}")
}

/// R2.4: the person's environment as is, Grace's variables, and `jre_dir` as `JAVA_HOME` only when there is no other
/// Java. Gradle gets none: it needs a JDK to resolve a Java project's classpath, which the bundled Java is not.
pub fn tool_env(base: Env, o: &ToolEnv) -> Env {
    let lend = o.jre_dir.filter(|_| {
        var(&base, "JAVA_HOME").is_none() && !path_folders(o.search_path).any(|dir| dir.join(JAVA).is_file())
    });
    let grace = [
        lend.map(|jre| ("JAVA_HOME", jre)),
        Some(("GRACE_TOOL", o.tool)),
        Some(("GRACE_CLASSPATH_FILE", o.file)),
        Some(("GRACE_INIT_SCRIPT", o.init_script)),
    ];
    let mut env = base;
    for (name, value) in grace.into_iter().flatten() {
        env.retain(|(k, _)| !same_variable(k, name));
        env.push((name.into(), value.into()));
    }
    env
}

/// R3.4: the listed entries that exist, in order, once, less those already on the classpath.
pub fn parse_libraries(text: &str, classpath: &[PathBuf]) -> Vec<String> {
    let key = |p: &Path| {
        let p = absolute(p);
        // Windows names one file in any letter case; Linux does not
        if cfg!(windows) {
            PathBuf::from(p.to_string_lossy().to_lowercase())
        } else {
            p
        }
    };
    let mut seen: HashSet<PathBuf> = classpath.iter().map(|p| key(p)).collect();
    text.split([LIST_SEPARATOR, '\r', '\n'])
        .map(js::trim)
        .filter(|e| !e.is_empty() && Path::new(e).exists() && seen.insert(key(Path::new(e))))
        .map(String::from)
        .collect()
}

/// R6.3: Maven's first `[ERROR]`, else a launcher's `ERROR:` (no Java), else the deepest cause Gradle says went wrong,
/// else the last line; without its full stop, which the notice adds, and 300 characters at most.
pub fn failure_reason(lines: &[String]) -> String {
    let said: Vec<&str> = lines.iter().map(|l| js::trim(l)).collect();
    // Maven may stack its prefixes: `[ERROR] [ERROR] Some problems were encountered while processing the POMs:`
    let errors: Vec<&str> = said
        .iter()
        .filter_map(|l| ERROR_PREFIX.find(l).map(|m| &l[m.end()..]))
        .collect();
    // a line ending with ":" announces the cause on the next one
    let caused = |i: usize| errors.get(i + 1).is_some_and(|next| !next.is_empty());
    let error = errors
        .iter()
        .enumerate()
        .find(|(i, l)| !(l.ends_with(':') && caused(*i)))
        .map(|(_, l)| *l)
        .or_else(|| errors.first().copied());
    let reason = error
        .or_else(|| gradle_cause(&said))
        .or_else(|| said.iter().rev().find(|l| !l.is_empty()).copied())
        .unwrap_or("it stopped with no message");
    reason.trim_end_matches('.').chars().take(300).collect()
}

/// `env`'s variable `name`, None when unset or empty. Windows names them in any case (`Path`).
pub fn var<'a>(env: &'a Env, name: &str) -> Option<&'a OsStr> {
    env.iter()
        .find(|(k, _)| same_variable(k, name))
        .map(|(_, v)| v.as_os_str())
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use BuildTool::{Gradle, Maven};
    use std::fs;
    use tempfile::TempDir;

    const MVNW: &str = if cfg!(windows) { "mvnw.cmd" } else { "mvnw" };
    const MVN: &str = if cfg!(windows) { "mvn.cmd" } else { "mvn" };
    const GRADLEW: &str = if cfg!(windows) { "gradlew.bat" } else { "gradlew" };
    const GRADLE: &str = if cfg!(windows) { "gradle.bat" } else { "gradle" };
    const JAVA: &str = if cfg!(windows) { "java.exe" } else { "java" };
    const JAVAC: &str = if cfg!(windows) { "javac.exe" } else { "javac" };

    /// A temporary folder holding `files` (relative path, content).
    fn folder(files: &[(&str, &str)]) -> TempDir {
        let d = tempfile::Builder::new().prefix("kk-lib-").tempdir().unwrap();
        for (name, text) in files {
            let path = d.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        d
    }

    /// `base` joined with each part of a `/`-separated path, as `path.join` writes it.
    fn at(base: &Path, rel: &str) -> PathBuf {
        rel.split('/').fold(base.to_path_buf(), |p, s| p.join(s))
    }

    fn env(vars: &[(&str, &OsStr)]) -> Env {
        vars.iter().map(|(k, v)| (k.into(), v.into())).collect()
    }

    /// `PATH` of these entries, as the platform writes it.
    fn search_path(entries: &[&Path]) -> OsString {
        std::env::join_paths(entries).unwrap()
    }

    fn lines(l: &[&str]) -> Vec<String> {
        l.iter().map(|s| s.to_string()).collect()
    }

    mod find_tool {
        use super::*;

        fn found(root: &TempDir, tool: BuildTool, env: &Env) -> Option<PathBuf> {
            super::super::find_tool(root.path(), tool, env)
        }

        #[test]
        fn takes_the_projects_wrapper_before_path() {
            let root = folder(&[(MVNW, "")]);
            let bin = folder(&[(MVN, "")]);
            let env = env(&[("PATH", bin.path().as_os_str())]);
            assert_eq!(found(&root, Maven, &env), Some(root.path().join(MVNW)));
        }

        #[cfg(windows)]
        #[test]
        fn searches_path_in_order_mvn_cmd_before_mvn_bat() {
            let a = folder(&[("mvn.bat", "")]);
            let b = folder(&[("mvn.cmd", "")]);
            // an empty entry is no folder
            let path = format!(";{};{}", a.path().display(), b.path().display());
            let first = env(&[("Path", OsStr::new(&path))]);
            assert_eq!(found(&folder(&[]), Maven, &first), Some(a.path().join("mvn.bat")));
            let c = folder(&[("mvn.bat", ""), ("mvn.cmd", "")]);
            let both = env(&[("PATH", c.path().as_os_str())]);
            assert_eq!(found(&folder(&[]), Maven, &both), Some(c.path().join("mvn.cmd")));
        }

        #[test]
        fn finds_the_gradle_wrapper_else_gradle() {
            let root = folder(&[(GRADLEW, "")]);
            let bin = folder(&[(GRADLE, "")]);
            let env = env(&[("PATH", bin.path().as_os_str())]);
            assert_eq!(found(&root, Gradle, &env), Some(root.path().join(GRADLEW)));
            assert_eq!(found(&folder(&[]), Gradle, &env), Some(bin.path().join(GRADLE)));
        }

        #[cfg(windows)]
        #[test]
        fn reads_a_quoted_path_entry_as_cmd_exe_does() {
            let bin = folder(&[("mvn.cmd", ""), ("java.exe", "")]);
            let quoted = OsString::from(format!("\"{}\"", bin.path().display()));
            let env = env(&[("PATH", &quoted)]);
            assert_eq!(found(&folder(&[]), Maven, &env), Some(bin.path().join("mvn.cmd")));
            let lent = tool_env(
                Vec::new(),
                &ToolEnv {
                    tool: Path::new("t"),
                    file: Path::new("f"),
                    init_script: Path::new("i"),
                    search_path: &quoted,
                    jre_dir: Some(Path::new("jre")),
                },
            );
            assert_eq!(var(&lent, "JAVA_HOME"), None);
        }

        #[test]
        fn then_looks_in_maven_home_then_m2_home_and_in_gradle_home_for_gradle() {
            let mvn = format!("bin/{MVN}");
            let m = folder(&[(&mvn, "")]);
            let m2 = folder(&[(&mvn, "")]);
            let g = folder(&[(&format!("bin/{GRADLE}"), "")]);
            // cmd.exe reads a quoted value as the folder inside the quotes; Linux knows no such quotes
            let gradle_home = if cfg!(windows) {
                OsString::from(format!("\"{}\"", g.path().display()))
            } else {
                g.path().into()
            };
            let none = folder(&[]);
            let homes = |maven: &Path| {
                env(&[
                    ("MAVEN_HOME", maven.as_os_str()),
                    ("M2_HOME", m2.path().as_os_str()),
                    ("GRADLE_HOME", &gradle_home),
                ])
            };
            assert_eq!(found(&folder(&[]), Maven, &homes(m.path())), Some(at(m.path(), &mvn)));
            assert_eq!(
                found(&folder(&[]), Maven, &homes(none.path())),
                Some(at(m2.path(), &mvn))
            );
            assert_eq!(
                found(&folder(&[]), Gradle, &homes(m.path())),
                Some(g.path().join("bin").join(GRADLE))
            );
            let gradle_only = env(&[("GRADLE_HOME", g.path().as_os_str())]);
            assert_eq!(found(&folder(&[]), Maven, &gradle_only), None);
            let bin = folder(&[(MVN, "")]);
            let mut on_path = homes(m.path());
            on_path.push(("PATH".into(), bin.path().into()));
            assert_eq!(found(&folder(&[]), Maven, &on_path), Some(bin.path().join(MVN)));
        }

        #[cfg(windows)]
        #[test]
        fn then_takes_intellij_ideas_maven_the_newest_build_first() {
            let mvn = "plugins/maven/lib/maven3/bin/mvn.cmd";
            let program_files = folder(&[
                (&format!("JetBrains/IntelliJ IDEA 2024.3/{mvn}"), ""),
                ("JetBrains/IntelliJ IDEA 2024.3/build.txt", "IU-243.21565.193"),
                ("JetBrains/Rider/build.txt", "RD-261.1.1"),
            ]);
            // a higher build, whatever the product code before it
            let local_app_data = folder(&[
                (&format!("Programs/IntelliJ IDEA/{mvn}"), ""),
                ("Programs/IntelliJ IDEA/build.txt", "IC-243.21565.1000\n"),
                ("Programs/Microsoft VS Code/Code.exe", ""),
            ]);
            let mut ide = env(&[
                ("ProgramFiles", program_files.path().as_os_str()),
                ("LOCALAPPDATA", local_app_data.path().as_os_str()),
            ]);
            let newest = at(local_app_data.path(), &format!("Programs/IntelliJ IDEA/{mvn}"));
            assert_eq!(found(&folder(&[]), Maven, &ide), Some(newest));
            fs::remove_file(at(local_app_data.path(), "Programs/IntelliJ IDEA/build.txt")).unwrap();
            let older = at(program_files.path(), &format!("JetBrains/IntelliJ IDEA 2024.3/{mvn}"));
            assert_eq!(found(&folder(&[]), Maven, &ide), Some(older));
            assert_eq!(found(&folder(&[]), Gradle, &ide), None);
            let m2 = folder(&[("bin/mvn.cmd", "")]);
            ide.push(("M2_HOME".into(), m2.path().into()));
            assert_eq!(found(&folder(&[]), Maven, &ide), Some(at(m2.path(), "bin/mvn.cmd")));
        }

        #[test]
        fn finds_nothing() {
            assert_eq!(found(&folder(&[]), Maven, &Vec::new()), None);
            assert_eq!(found(&folder(&[(MVNW, "")]), Gradle, &Vec::new()), None);
        }

        #[test]
        fn names_where_it_looked() {
            if cfg!(windows) {
                assert_eq!(
                    no_tool_reason(Maven),
                    "the project has no mvnw.cmd, and mvn is not on PATH, in MAVEN_HOME, in M2_HOME or in IntelliJ IDEA"
                );
                assert_eq!(
                    no_tool_reason(Gradle),
                    "the project has no gradlew.bat, and gradle is not on PATH or in GRADLE_HOME"
                );
            } else {
                assert_eq!(
                    no_tool_reason(Maven),
                    "the project has no mvnw, and mvn is not on PATH, in MAVEN_HOME or in M2_HOME"
                );
                assert_eq!(
                    no_tool_reason(Gradle),
                    "the project has no gradlew, and gradle is not on PATH or in GRADLE_HOME"
                );
            }
        }
    }

    mod library_key {
        use super::*;

        fn key(root: &TempDir, tool: BuildTool) -> String {
            super::super::library_key(root.path(), tool)
        }

        #[test]
        fn changes_with_the_layouts_build_files_only() {
            let root = folder(&[("pom.xml", "<project/>"), ("README.md", "a")]);
            let first = key(&root, Maven);
            // Electron Grace's key of the same files: the two apps share libraries.json
            assert_eq!(
                first,
                "48ed6cccab26ee7c0c33b78308bacabe17f0bc5033b678e02bea18c5860db728"
            );
            assert_eq!(key(&root, Maven), first);
            fs::write(root.path().join("README.md"), "b").unwrap();
            fs::write(root.path().join("build.gradle"), "").unwrap();
            assert_eq!(key(&root, Maven), first);
            fs::write(root.path().join("pom.xml"), "<project></project>").unwrap();
            let edited = key(&root, Maven);
            assert_ne!(edited, first);
            fs::create_dir(root.path().join(".mvn")).unwrap();
            fs::write(at(root.path(), ".mvn/maven.config"), "-Pqa").unwrap();
            assert_ne!(key(&root, Maven), edited);
        }

        #[test]
        fn reads_gradles_version_catalog() {
            let root = folder(&[("build.gradle", ""), ("gradle/libs.versions.toml", "a")]);
            let first = key(&root, Gradle);
            fs::write(at(root.path(), "gradle/libs.versions.toml"), "b").unwrap();
            assert_ne!(key(&root, Gradle), first);
        }
    }

    mod the_kept_list {
        use super::*;

        fn maven(jars: Vec<String>) -> KeptLibraries {
            KeptLibraries {
                tool: Layout::Maven,
                key: "k".into(),
                jars,
            }
        }

        #[test]
        fn round_trips() {
            let base = folder(&[]);
            let dir = at(base.path(), "projects/abc");
            write_kept(&dir, &maven(vec!["a.jar".into()])).unwrap();
            assert_eq!(read_kept(&dir), Some(maven(vec!["a.jar".into()])));
            // Electron Grace's text
            assert_eq!(
                fs::read_to_string(dir.join("libraries.json")).unwrap(),
                r#"{"tool":"maven","key":"k","jars":["a.jar"]}"#
            );
        }

        #[test]
        fn reads_anything_else_as_none() {
            let dir = folder(&[]);
            assert_eq!(read_kept(dir.path()), None);
            for text in [
                "nope",
                "{}",
                r#"{"tool":"ant","key":"k","jars":[]}"#,
                r#"{"tool":"standalone","key":"k","jars":[]}"#,
                r#"{"tool":"maven","key":"k","jars":[1]}"#,
            ] {
                fs::write(dir.path().join("libraries.json"), text).unwrap();
                assert_eq!(read_kept(dir.path()), None, "{text}");
            }
        }

        #[test]
        fn is_stale_when_missing_of_another_tool_or_key_or_a_jar_is_gone() {
            let d = folder(&[("a.jar", "")]);
            let jar = d.path().join("a.jar").to_string_lossy().into_owned();
            let kept = maven(vec![jar.clone()]);
            assert!(is_stale(None, Maven, "k"));
            assert!(is_stale(Some(&kept), Gradle, "k"));
            assert!(is_stale(Some(&kept), Maven, "other"));
            assert!(is_stale(
                Some(&maven(vec![jar.clone(), format!("{jar}.gone")])),
                Maven,
                "k"
            ));
            assert!(!is_stale(Some(&kept), Maven, "k"));
        }

        #[test]
        fn gives_the_jars_only_to_the_same_tool() {
            let dir = folder(&[]);
            write_kept(dir.path(), &maven(vec!["a.jar".into()])).unwrap();
            assert_eq!(kept_jars(dir.path(), Layout::Maven), [PathBuf::from("a.jar")]);
            assert!(kept_jars(dir.path(), Layout::Gradle).is_empty());
            assert!(kept_jars(dir.path(), Layout::Standalone).is_empty());
            assert!(kept_jars(folder(&[]).path(), Layout::Maven).is_empty());
        }
    }

    mod the_command {
        use super::*;

        fn lent(base: &[(&str, &str)], search_path: &OsStr, jre_dir: Option<&str>) -> Env {
            let base = base.iter().map(|(k, v)| (k.into(), v.into())).collect();
            tool_env(
                base,
                &ToolEnv {
                    tool: Path::new("t.cmd"),
                    file: Path::new("f.txt"),
                    init_script: Path::new("i.gradle"),
                    search_path,
                    jre_dir: jre_dir.map(Path::new),
                },
            )
        }

        #[test]
        fn passes_every_path_through_a_quoted_variable() {
            assert_eq!(
                windows_command(Maven, Goal::List),
                r#""%GRACE_TOOL%" -q -B dependency:build-classpath "-Dmdep.outputFile=%GRACE_CLASSPATH_FILE%""#
            );
            assert_eq!(
                windows_command(Gradle, Goal::List),
                r#""%GRACE_TOOL%" -q --no-daemon --console=plain --no-configuration-cache --init-script "%GRACE_INIT_SCRIPT%" graceClasspath"#
            );
        }

        #[test]
        fn keeps_the_persons_environment_and_adds_graces_variables() {
            let env = lent(
                &[("MAVEN_OPTS", "-Xmx1g"), ("JAVA_HOME", "jdk")],
                OsStr::new(""),
                Some("jre"),
            );
            let expected: Env = [
                ("MAVEN_OPTS", "-Xmx1g"),
                ("JAVA_HOME", "jdk"),
                ("GRACE_TOOL", "t.cmd"),
                ("GRACE_CLASSPATH_FILE", "f.txt"),
                ("GRACE_INIT_SCRIPT", "i.gradle"),
            ]
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
            assert_eq!(env, expected);
        }

        #[test]
        fn lends_the_bundled_java_only_when_there_is_no_other() {
            let jre = Some(OsStr::new("jre"));
            assert_eq!(var(&lent(&[], OsStr::new(""), Some("jre")), "JAVA_HOME"), jre);
            let empty = lent(&[("JAVA_HOME", "")], OsStr::new(""), Some("jre"));
            assert_eq!(var(&empty, "JAVA_HOME"), jre);
            assert_eq!(empty.iter().filter(|(k, _)| k == "JAVA_HOME").count(), 1);
            let bin = folder(&[(JAVA, "")]);
            let with_java = lent(&[], &search_path(&[bin.path()]), Some("jre"));
            assert_eq!(var(&with_java, "JAVA_HOME"), None);
        }

        #[test]
        fn lends_nothing_without_a_java_to_lend() {
            assert!(lent(&[], OsStr::new(""), None).iter().all(|(k, _)| k != "JAVA_HOME"));
        }
    }

    mod parse_libraries {
        use super::*;

        #[test]
        fn keeps_existing_entries_in_order_once_less_the_classpath() {
            // the separator of the platform's `File.pathSeparator`, which Maven and the init script write
            let s = if cfg!(windows) { ';' } else { ':' };
            let d = folder(&[("a.jar", ""), ("b.jar", "")]);
            let text = |p: PathBuf| p.to_string_lossy().into_owned();
            let (a, b) = (text(d.path().join("a.jar")), text(d.path().join("b.jar")));
            let parse = super::super::parse_libraries;
            assert_eq!(parse(&format!("{a}{s}{b}\r\n"), &[]), [a.clone(), b.clone()]);
            assert_eq!(
                parse(&format!("{b}{s}{a}.gone{s}{a}{s}{b}"), &[]),
                [b.clone(), a.clone()]
            );
            let classpath = [d.path().to_path_buf()];
            let upper = text(d.path().to_path_buf()).to_uppercase();
            assert_eq!(parse(&format!("{a}{s}{upper}"), &classpath), std::slice::from_ref(&a));
            // on Linux the upper-case folder does not exist: only the folder as written shows it is left out there
            let same = text(d.path().to_path_buf());
            assert_eq!(parse(&format!("{same}{s}{a}"), &classpath), std::slice::from_ref(&a));
            assert!(parse("", &[]).is_empty());
        }
    }

    mod failure_reason {
        use super::*;

        fn reason(l: &[&str]) -> String {
            super::super::failure_reason(&lines(l))
        }

        #[test]
        fn takes_mavens_first_error() {
            assert_eq!(
                reason(&["[INFO] x", "[ERROR] Could not resolve dependencies", "[ERROR] other"]),
                "Could not resolve dependencies"
            );
        }

        // recorded from Maven 3.9.16 on three broken poms, paths shortened
        #[test]
        fn takes_the_cause_under_mavens_pom_problems_line() {
            let unreadable = reason(&[
                "[ERROR] [ERROR] Some problems were encountered while processing the POMs:",
                "[FATAL] Non-readable POM C:\\p\\pom.xml: no more data available - expected end tags </modelVersion></project> to close start tag <modelVersion> from line 1 and start tag <project> from line 1, parser stopped on TEXT seen ...<modelVersion>4.0.0</modelVersion\\r\\n... @2:1 @ ",
                " @ ",
                "[ERROR] The build could not read 1 project -> [Help 1]",
            ]);
            assert!(
                unreadable.starts_with("Non-readable POM C:\\p\\pom.xml: no more data available"),
                "{unreadable}"
            );
            assert_eq!(
                reason(&[
                    "[ERROR] [ERROR] Some problems were encountered while processing the POMs:",
                    "[ERROR] 'dependencies.dependency.version' for org.xerial:sqlite-jdbc:jar is missing. @ line 1, column 136",
                    " @ ",
                    "[ERROR] The build could not read 1 project -> [Help 1]",
                ]),
                "'dependencies.dependency.version' for org.xerial:sqlite-jdbc:jar is missing. @ line 1, column 136"
            );
            assert_eq!(
                reason(&[
                    "[ERROR] Failed to execute goal on project y: Could not resolve dependencies for project x:y:jar:1",
                    "[ERROR] dependency: org.acme:nope:jar:9.9 (compile)",
                    "[ERROR] -> [Help 1]",
                ]),
                "Failed to execute goal on project y: Could not resolve dependencies for project x:y:jar:1"
            );
        }

        #[test]
        fn takes_what_gradle_says_went_wrong() {
            assert_eq!(
                reason(&[
                    "FAILURE: Build failed with an exception.",
                    "",
                    "* What went wrong:",
                    "Task 'graceClasspath' not found in root project 'x'.",
                ]),
                "Task 'graceClasspath' not found in root project 'x'"
            );
        }

        #[test]
        fn takes_the_deepest_cause_gradle_gives() {
            assert_eq!(
                reason(&[
                    "* What went wrong:",
                    "Execution failed for task ':graceClasspath' (registered in initialization script 'x').",
                    "> Could not resolve all dependencies for configuration ':testRuntimeClasspath'.",
                    "   > Could not find org.acme:client:1.0.",
                    "",
                    "* Try:",
                    "> Run with --stacktrace option to get the stack trace.",
                ]),
                "Could not find org.acme:client:1.0"
            );
        }

        #[test]
        fn takes_a_launchers_error_line() {
            assert_eq!(
                reason(&[
                    "",
                    "ERROR: JAVA_HOME is not set and no 'java' command could be found in your PATH.",
                    "",
                    "Please set the JAVA_HOME variable in your environment to match the",
                    "location of your Java installation.",
                ]),
                "JAVA_HOME is not set and no 'java' command could be found in your PATH"
            );
        }

        #[test]
        fn else_the_last_line_cut_to_300_characters() {
            assert_eq!(reason(&["a", "b", ""]), "b");
            // characters, not bytes: a cut inside one would panic
            assert_eq!(reason(&[&"é".repeat(400)]).chars().count(), 300);
            assert_eq!(reason(&[]), "it stopped with no message");
        }
    }

    mod building {
        use super::*;

        #[test]
        fn runs_the_plain_compile_of_the_test_code() {
            assert_eq!(
                windows_command(Maven, Goal::Build),
                r#""%GRACE_TOOL%" -B -q test-compile"#
            );
            assert_eq!(
                windows_command(Gradle, Goal::Build),
                r#""%GRACE_TOOL%" -q --no-daemon --console=plain testClasses"#
            );
        }

        #[test]
        fn finds_a_jdk_by_javac_in_java_home_or_on_path() {
            let jdk = folder(&[(&format!("bin/{JAVAC}"), ""), (&format!("bin/{JAVA}"), "")]);
            let java_home = |v: &OsStr| env(&[("JAVA_HOME", v)]);
            assert!(!has_jdk(&Vec::new()));
            assert!(has_jdk(&java_home(jdk.path().as_os_str())));
            if cfg!(windows) {
                let quoted = OsString::from(format!("\"{}\"", jdk.path().display()));
                assert!(has_jdk(&java_home(&quoted)));
            }
            let path = search_path(&[Path::new("x"), &jdk.path().join("bin")]);
            assert!(has_jdk(&env(&[("PATH", &path)])));
            let jre = folder(&[(&format!("bin/{JAVA}"), "")]);
            assert!(!has_jdk(&java_home(jre.path().as_os_str())));
        }

        #[test]
        fn names_javacs_first_error_before_gradles_cause() {
            assert_eq!(
                build_failure(
                    Gradle,
                    &lines(&[
                        "C:\\p\\Helper.java:3: error: cannot find symbol",
                        "* What went wrong:",
                        "Execution failed for task ':compileTestJava'.",
                        "> Compilation failed; see the compiler error output for details.",
                    ]),
                    true
                ),
                "Grace could not build the project's Java code with Gradle: C:\\p\\Helper.java:3: error: cannot find symbol."
            );
        }

        #[test]
        fn adds_no_stop_after_a_question_and_says_when_a_jdk_is_missing() {
            assert_eq!(
                build_failure(
                    Maven,
                    &lines(&[
                        "[ERROR] COMPILATION ERROR : ",
                        "[ERROR] No compiler is provided in this environment. Perhaps you are running on a JRE rather than a JDK?",
                    ]),
                    false
                ),
                "Grace could not build the project's Java code with Maven: No compiler is provided in this environment. Perhaps you are running on a JRE rather than a JDK? Building needs a JDK: set JAVA_HOME to one."
            );
        }
    }

    // Linux's own search and command line (the P4 plan's Redesigns, `libraries` (Linux)): the Windows tests above that
    // name `.cmd` files, quotes or IntelliJ IDEA have these beside them.
    #[cfg(unix)]
    mod linux {
        use super::*;

        #[test]
        fn searches_path_at_colons_in_order() {
            let a = folder(&[("mvn", "")]);
            let b = folder(&[("mvn", "")]);
            let path = format!(":{}:{}", a.path().display(), b.path().display());
            let env = env(&[("PATH", OsStr::new(&path))]);
            assert_eq!(find_tool(folder(&[]).path(), Maven, &env), Some(a.path().join("mvn")));
        }

        #[test]
        fn takes_no_windows_launcher_and_looks_for_no_intellij() {
            let d = folder(&[("mvn.cmd", ""), ("JetBrains/idea/plugins/maven/lib/maven3/bin/mvn", "")]);
            let env = env(&[
                ("PATH", d.path().as_os_str()),
                ("ProgramFiles", d.path().as_os_str()),
                ("LOCALAPPDATA", d.path().as_os_str()),
            ]);
            assert_eq!(find_tool(d.path(), Maven, &env), None);
        }

        #[test]
        fn starts_the_launcher_itself_with_an_argument_list() {
            let start = |tool, goal| {
                let s = tool_start(
                    tool,
                    goal,
                    Path::new("/a b&c/mvnw"),
                    Path::new("/d $e/libraries.txt"),
                    Path::new("/d $e/libraries.gradle"),
                );
                assert_eq!(s.program, Path::new("/a b&c/mvnw"));
                assert!(!s.verbatim_args);
                s.args
            };
            assert_eq!(
                start(Maven, Goal::List),
                [
                    "-q",
                    "-B",
                    "dependency:build-classpath",
                    "-Dmdep.outputFile=/d $e/libraries.txt"
                ]
            );
            assert_eq!(
                start(Gradle, Goal::List),
                [
                    "-q",
                    "--no-daemon",
                    "--console=plain",
                    "--no-configuration-cache",
                    "--init-script",
                    "/d $e/libraries.gradle",
                    "graceClasspath"
                ]
            );
            assert_eq!(start(Maven, Goal::Build), ["-B", "-q", "test-compile"]);
            assert_eq!(
                start(Gradle, Goal::Build),
                ["-q", "--no-daemon", "--console=plain", "testClasses"]
            );
        }

        #[test]
        fn keeps_two_jars_whose_paths_differ_in_case() {
            let d = folder(&[("a.jar", ""), ("A.jar", "")]);
            let (a, upper) = (d.path().join("a.jar"), d.path().join("A.jar"));
            let text = format!("{}:{}", a.display(), upper.display());
            assert_eq!(parse_libraries(&text, &[]).len(), 2);
        }
    }

    #[cfg(windows)]
    mod windows {
        use super::*;

        #[test]
        fn runs_the_command_through_cmd_exe_as_written() {
            let s = tool_start(
                Maven,
                Goal::List,
                Path::new(r"C:\a b&c\mvnw.cmd"),
                Path::new("f"),
                Path::new("i"),
            );
            assert_eq!(s.program, crate::karate::process::cmd());
            assert!(s.verbatim_args);
            let command = format!("\"{}\"", windows_command(Maven, Goal::List));
            assert_eq!(s.args, ["/d", "/s", "/c", &command]);
        }

        #[test]
        fn replaces_graces_variables_in_any_letter_case() {
            let env = tool_env(
                vec![("java_home".into(), "".into()), ("Grace_Tool".into(), "old".into())],
                &ToolEnv {
                    tool: Path::new("new"),
                    file: Path::new("f"),
                    init_script: Path::new("i"),
                    search_path: OsStr::new(""),
                    jre_dir: Some(Path::new("jre")),
                },
            );
            let names: Vec<String> = env.iter().map(|(k, _)| k.to_string_lossy().to_uppercase()).collect();
            assert_eq!(
                names,
                ["JAVA_HOME", "GRACE_TOOL", "GRACE_CLASSPATH_FILE", "GRACE_INIT_SCRIPT"]
            );
            assert_eq!(var(&env, "GRACE_TOOL"), Some(OsStr::new("new")));
        }
    }
}
