//! `src/main/karate/argfile.ts`: the two argument files a run hands to Java and to picocli. Their bytes are part of
//! the Karate command line (migration guide R1.5) and stay what Electron Grace writes: quoting, CRLF, encoding.
use std::fs;
use std::io;
use std::path::Path;

use crate::model::env::PlainRow;

/// Inside the quotes of a Java argument file, `\` and `"` are escapes.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// One `"-D<name>=<value>"` line per row, each ended by CRLF.
pub fn arg_file_text(rows: &[PlainRow]) -> String {
    rows.iter()
        .map(|r| format!("\"{}\"\r\n", quote(&format!("-D{}={}", r.name, r.value))))
        .collect()
}

/// Java reads the file in the Windows code page; the values are ASCII, so latin1 is exact. Like Node's `"latin1"`
/// encoding, each UTF-16 unit is written as its low byte (`é` is `E9`; a character above U+00FF is cut, not an error).
pub fn write_arg_file(file: &Path, rows: &[PlainRow]) -> io::Result<()> {
    let bytes: Vec<u8> = arg_file_text(rows).encode_utf16().map(|unit| unit as u8).collect();
    fs::write(file, bytes)
}

/// picocli reads its `@file` as UTF-8 (unlike the Java launcher): one `"target"` line per target, CRLF.
pub fn write_target_file<S: AsRef<str>>(file: &Path, targets: &[S]) -> io::Result<()> {
    let text: String = targets
        .iter()
        .map(|t| format!("\"{}\"\r\n", quote(t.as_ref())))
        .collect();
    fs::write(file, text)
}

/// Deletes every `run-args.txt` and `run-targets.txt` under `<data_dir>/projects/<hash>/` that a crash left behind.
/// Never fails: one stuck file must not keep the others.
pub fn remove_leftover_arg_files(data_dir: &Path) {
    let Ok(projects) = fs::read_dir(data_dir.join("projects")) else {
        return;
    };
    for project in projects.flatten() {
        for name in ["run-args.txt", "run-targets.txt"] {
            let _ = fs::remove_file(project.path().join(name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::Builder::new().prefix("kk-arg-").tempdir().unwrap()
    }

    fn row(name: &str, value: &str) -> PlainRow {
        PlainRow {
            name: name.to_string(),
            value: value.to_string(),
            env_var: None,
        }
    }

    const TRICKY: &str = r#"x y "q" C:\Users\me %P% $x 'z'"#;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    mod arg_file_text {
        use super::*;

        #[test]
        fn escapes_backslashes_and_quotes() {
            let a = r#"x y "q" C:\Users\me %P%"#;
            assert_eq!(
                arg_file_text(&[row("a", a)]),
                r#""-Da=x y \"q\" C:\\Users\\me %P%""#.to_string() + "\r\n"
            );
        }

        #[test]
        fn writes_one_crlf_ended_line_per_row_and_nothing_for_no_rows() {
            assert_eq!(
                arg_file_text(&[row("a", "1"), row("b.c", "")]),
                "\"-Da=1\"\r\n\"-Db.c=\"\r\n"
            );
            assert_eq!(arg_file_text(&[]), "");
        }
    }

    mod write_arg_file {
        use super::*;

        #[test]
        fn writes_the_text_as_latin1() {
            let dir = tmp();
            let file = dir.path().join("run-args.txt");
            write_arg_file(&file, &[row("a", TRICKY)]).unwrap();
            let bytes = fs::read(&file).unwrap();
            let latin1: String = bytes.iter().map(|&b| b as char).collect();
            assert_eq!(latin1, arg_file_text(&[row("a", TRICKY)]));
        }

        // Not in the TypeScript tests: the bytes Electron Grace's `writeArgFile` writes for the same rows (run
        // through `src/main/karate/argfile.ts` under Node 22). What is not ASCII is Node's "latin1": the low byte of
        // each UTF-16 unit. `é` is E9; the euro sign U+20AC is cut to AC; U+1F600 is the units D83D DE00, so 3D 00.
        #[test]
        fn writes_the_bytes_electron_grace_writes() {
            let dir = tmp();
            let file = dir.path().join("run-args.txt");
            let rows = [
                row("a", TRICKY),
                row("b.c", ""),
                row("é", "é€😀\\\""),
                row("tab", "a\tb\r\nc"),
            ];
            write_arg_file(&file, &rows).unwrap();
            let expected = "222d44613d782079205c22715c2220433a5c5c55736572735c5c6d652025502520247820277a27220d0a\
                222d44622e633d220d0a222d44e93de9ac3d005c5c5c22220d0a222d447461623d6109620d0a63220d0a";
            assert_eq!(hex(&fs::read(&file).unwrap()), expected);
        }
    }

    mod write_target_file {
        use super::*;

        #[test]
        fn quotes_doubles_backslashes_and_writes_utf8() {
            let dir = tmp();
            let file = dir.path().join("run-targets.txt");
            let targets = [r#"C:\Users\hé llo\a.feature:3:15"#, r#"D:\p "q"\b.feature:7"#];
            write_target_file(&file, &targets).unwrap();
            let text = r#""C:\\Users\\hé llo\\a.feature:3:15""#.to_string()
                + "\r\n"
                + r#""D:\\p \"q\"\\b.feature:7""#
                + "\r\n";
            assert_eq!(fs::read(&file).unwrap(), text.into_bytes());
        }

        #[test]
        fn writes_an_empty_file_for_no_targets() {
            let dir = tmp();
            let file = dir.path().join("run-targets.txt");
            write_target_file(&file, &[] as &[&str]).unwrap();
            assert_eq!(fs::read(&file).unwrap().len(), 0);
        }

        // Not in the TypeScript tests: the bytes Electron Grace's `writeTargetFile` writes for these targets, one of
        // them with a character above U+FFFF (run through `src/main/karate/argfile.ts` under Node 22).
        #[test]
        fn writes_the_bytes_electron_grace_writes() {
            let dir = tmp();
            let file = dir.path().join("run-targets.txt");
            write_target_file(
                &file,
                &[r#"C:\Users\hé llo\a.feature:3:15"#, r#"D:\p "q"\b😀€.feature:7"#],
            )
            .unwrap();
            let expected = "22433a5c5c55736572735c5c68c3a9206c6c6f5c5c612e666561747572653a333a3135220d0a\
                22443a5c5c70205c22715c225c5c62f09f9880e282ac2e666561747572653a37220d0a";
            assert_eq!(hex(&fs::read(&file).unwrap()), expected);
        }
    }

    mod remove_leftover_arg_files {
        use super::*;

        fn put(path: PathBuf, content: &str) {
            fs::write(path, content).unwrap();
        }

        #[test]
        fn also_deletes_run_targets_txt() {
            let data_dir = tmp();
            let project = data_dir.path().join("projects").join("h1");
            fs::create_dir_all(&project).unwrap();
            for f in ["run-args.txt", "run-targets.txt", "env.json"] {
                put(project.join(f), "x");
            }
            remove_leftover_arg_files(data_dir.path());
            assert!(!project.join("run-args.txt").exists());
            assert!(!project.join("run-targets.txt").exists());
            assert!(project.join("env.json").exists());
        }

        #[test]
        fn deletes_every_run_args_txt_and_nothing_else() {
            let data_dir = tmp();
            let projects = data_dir.path().join("projects");
            for p in ["h1", "h2", "h3"] {
                fs::create_dir_all(projects.join(p)).unwrap();
            }
            for p in ["h1", "h2"] {
                put(projects.join(p).join("run-args.txt"), "x");
            }
            put(projects.join("h2").join("env.json"), "{}");
            put(data_dir.path().join("settings.json"), "{}");
            remove_leftover_arg_files(data_dir.path());
            assert!(!projects.join("h1").join("run-args.txt").exists());
            assert!(!projects.join("h2").join("run-args.txt").exists());
            assert!(projects.join("h2").join("env.json").exists());
            assert!(data_dir.path().join("settings.json").exists());
        }

        // The TypeScript asserts `not.toThrow()`; Rust's equivalent is that the call returns and nothing panics.
        #[test]
        fn never_throws_whatever_is_there() {
            let data_dir = tmp();
            remove_leftover_arg_files(&data_dir.path().join("missing"));
            // a folder named like the file
            fs::create_dir_all(data_dir.path().join("projects/h1/run-args.txt")).unwrap();
            // a file where a folder is expected
            put(data_dir.path().join("projects").join("file"), "x");
            remove_leftover_arg_files(data_dir.path());
            // the folder is still there: `remove_file` does not take folders
            assert!(data_dir.path().join("projects/h1/run-args.txt").is_dir());
        }
    }

    /// What `vendor.ts`, `command.ts` (`buildArgs`), `process.ts` (`spawnKarate`) and `events.ts` (`readAllEvents`)
    /// give the two Java tests. Their ports are other modules of this phase (`karate::{command, process, events}`);
    /// until they land the tests build the same command line by hand and run it without a shell. Swap this module for
    /// them then. Real Java and Karate (guide R6.1): the `java` on PATH on Linux, the bundled JRE on Windows, and the
    /// pinned `vendor/karate.jar` (`npm run vendor`).
    mod java {
        use super::*;
        use std::process::Command;

        /// `STRIPPED_ENV_VARS` of `src/shared/env.ts`: they override the values the run passes.
        const STRIPPED: [&str; 6] = [
            "KARATE_OPTIONS",
            "KARATE_ENV",
            "KARATE_CONFIG_DIR",
            "JAVA_TOOL_OPTIONS",
            "JDK_JAVA_OPTIONS",
            "_JAVA_OPTIONS",
        ];

        pub struct Vendor {
            pub java: PathBuf,
            pub jar: PathBuf,
        }

        pub fn vendor() -> Vendor {
            // not canonicalized: on Windows that gives a `\\?\` path, which Java's classpath does not take
            let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
            let java = if cfg!(windows) {
                repo.join("vendor").join("jre").join("bin").join("java.exe")
            } else {
                PathBuf::from("java")
            };
            let jar = repo.join("vendor").join("karate.jar");
            assert!(jar.is_file(), "{} is missing: run `npm run vendor`", jar.display());
            Vendor { java, jar }
        }

        /// `buildArgs` for a run without tags, environment, config folder, JUnit XML or debugger, on one thread.
        pub fn build_args(
            jar: &Path,
            classpath: &[&Path],
            root: &Path,
            out_dir: &Path,
            arg_file: Option<&Path>,
            targets: &[String],
        ) -> Vec<String> {
            let sep = if cfg!(windows) { ";" } else { ":" };
            let cp: Vec<String> = std::iter::once(jar)
                .chain(classpath.iter().copied())
                .map(|p| p.display().to_string())
                .collect();
            let mut args: Vec<String> = vec![];
            // The launcher expands `@file` only before the main class, so it goes first.
            args.extend(arg_file.map(|f| format!("@{}", f.display())));
            args.extend(
                [
                    "-Xms64m",
                    "-Xmx2g",
                    "-XX:+ExitOnOutOfMemoryError",
                    "-XX:CICompilerCount=2",
                    "-Dstdout.encoding=UTF-8",
                    "-Dstderr.encoding=UTF-8",
                    "-cp",
                ]
                .map(String::from),
            );
            args.push(cp.join(sep));
            args.extend(["io.karatelabs.Main", "run", "--no-color", "--no-pom", "-w"].map(String::from));
            args.push(root.display().to_string());
            args.push("-o".to_string());
            args.push(out_dir.display().to_string());
            args.extend(["-B", "false", "-f", "html,karate:jsonl", "-T", "1"].map(String::from));
            args.extend(targets.iter().cloned());
            args
        }

        /// The exit code and every line Java printed (stdout, then stderr).
        pub fn run(java: &Path, args: &[String], cwd: &Path) -> (Option<i32>, Vec<String>) {
            let mut cmd = Command::new(java);
            cmd.args(args).current_dir(cwd);
            for name in STRIPPED {
                cmd.env_remove(name);
            }
            // Windows hands Java its command line in UTF-16. On Linux Java decodes it in the locale's charset, and
            // under `LANG=C` (a CI container) a path with `é` is "malformed input": run it in a UTF-8 locale.
            // `karate::process` must do the same for a project folder with a non-ASCII name.
            #[cfg(not(windows))]
            cmd.env("LC_ALL", "C.UTF-8");
            let out = cmd.output().expect("java could not be started");
            let mut lines = vec![];
            for bytes in [&out.stdout, &out.stderr] {
                lines.extend(String::from_utf8_lossy(bytes).lines().map(String::from));
            }
            (out.status.code(), lines)
        }

        /// The `<feature> <refId>` of every depth-0 `SCENARIO_EXIT` of a run's events file, sorted.
        pub fn ran(events_file: &Path) -> Vec<String> {
            let text = fs::read_to_string(events_file).unwrap();
            let mut ran: Vec<String> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
                .filter(|e| e["type"] == "SCENARIO_EXIT" && e["data"]["callDepth"] == 0)
                .map(|e| {
                    format!(
                        "{} {}",
                        e["data"]["feature"].as_str().unwrap(),
                        e["data"]["refId"].as_str().unwrap()
                    )
                })
                .collect();
            ran.sort();
            ran
        }

        /// `cpSync(from, to, { recursive: true })`.
        pub fn copy_dir(from: &Path, to: &Path) {
            fs::create_dir_all(to).unwrap();
            for entry in fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                let target = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy_dir(&entry.path(), &target);
                } else {
                    fs::copy(entry.path(), target).unwrap();
                }
            }
        }
    }

    mod argument_file_through_java {
        use super::*;

        #[test]
        fn hands_every_value_to_karate_properties_unchanged() {
            let vendor = java::vendor();
            let root = tmp();
            fs::write(
                root.path().join("p.feature"),
                [
                    "Feature: argument file",
                    "Scenario: print the run properties",
                    "* print 'A<' + karate.properties['a'] + '>'",
                    "* print 'B<' + karate.properties['b'] + '>'",
                    "",
                ]
                .join("\n"),
            )
            .unwrap();
            let arg_dir = tmp();
            let arg_file = arg_dir.path().join("run-args.txt");
            write_arg_file(&arg_file, &[row("a", TRICKY), row("b", "plain")]).unwrap();
            let out_dir = tmp();
            let args = java::build_args(
                &vendor.jar,
                &[root.path()],
                root.path(),
                out_dir.path(),
                Some(&arg_file),
                &[root.path().display().to_string()],
            );
            let (exit, lines) = java::run(&vendor.java, &args, root.path());
            assert_eq!(exit, Some(0), "{}", lines.join("\n"));
            let printed = |key: &str| {
                let re = regex::Regex::new(&format!("{key}<(.*)>$")).unwrap();
                lines.iter().find_map(|l| re.captures(l).map(|c| c[1].to_string()))
            };
            assert_eq!(printed("A").as_deref(), Some(TRICKY));
            assert_eq!(printed("B").as_deref(), Some("plain"));
        }
    }

    mod target_file_through_java {
        use super::*;

        #[test]
        fn runs_exactly_the_targets_of_the_file_in_paths_with_a_space_and_e_acute() {
            let vendor = java::vendor();
            let root = tempfile::Builder::new().prefix("kk hist é-").tempdir().unwrap();
            let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures/projects/edge");
            java::copy_dir(&fixture, root.path());
            fs::write(
                root.path().join("tests").join("second.feature"),
                ["Feature: second", "", "Scenario: only one", "* match 1 == 1", ""].join("\n"),
            )
            .unwrap();
            // the file itself under a path with a space, as the app data folder can be
            let args_dir = tempfile::Builder::new().prefix("kk run targets-").tempdir().unwrap();
            let target_file = args_dir.path().join("run-targets.txt");
            // rows `| 1 |` and `| 3 |` of the outline `add <a>` (lines 10 and 13), then the scenario at line 3
            write_target_file(
                &target_file,
                &[
                    format!("{}:10:13", root.path().join("tests").join("rows.feature").display()),
                    format!("{}:3", root.path().join("tests").join("second.feature").display()),
                ],
            )
            .unwrap();
            let out_dir = tmp();
            let args = java::build_args(
                &vendor.jar,
                &[root.path()],
                root.path(),
                out_dir.path(),
                None,
                &[format!("@{}", target_file.display())],
            );
            let (exit, lines) = java::run(&vendor.java, &args, root.path());
            assert_eq!(exit, Some(0), "{}", lines.join("\n"));
            let ran = java::ran(&out_dir.path().join("karate-json").join("karate-events.jsonl"));
            // rows 1 and 3, not 2 (the refIds of rows carry the Examples line)
            assert_eq!(
                ran,
                [
                    "tests/rows.feature [1.1:7]",
                    "tests/rows.feature [1.3:7]",
                    "tests/second.feature [1:3]",
                ]
            );
        }
    }
}
