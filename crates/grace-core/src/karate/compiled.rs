//! `src/main/karate/compiled.ts`: says when a Maven or Gradle project's Java code is not built. Grace runs what the
//! build compiled, and builds only when the person clicks Build (docs/specs/2026-11-02-project-libraries.md R8,
//! docs/specs/2026-11-04-build-from-notice.md).
use crate::model::Layout;
use regex::Regex;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BUILT: &str = "built.json";

/// A JUnit runner imports JUnit or Karate's JUnit module (Karate 1 `com.intuit.karate.junit5`, Karate 2
/// `io.karatelabs.junit6`). Grace runs the features itself and never loads one, so it needs no build. A byte order mark
/// is a space to JavaScript's `\s`, not to this one's.
static RUNNER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?m)^[\s\x{FEFF}]*import\s+(?:static\s+)?",
        r"(?:org\.junit|com\.intuit\.karate\.junit|io\.karatelabs\.junit)",
    ))
    .expect("a valid pattern")
});

/// The newest modified time of the files under `dir` named `*<extension>` that `keep` accepts; None without such a file
/// or folder. A linked folder is not entered, and what cannot be read is left out.
fn newest(dir: &Path, extension: &str, keep: fn(&Path) -> bool) -> Option<SystemTime> {
    let mut newest = None;
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                folders.push(path);
            } else if entry.file_name().to_string_lossy().ends_with(extension) && keep(&path) {
                // `None` is older than any time
                newest = newest.max(fs::metadata(&path).and_then(|m| m.modified()).ok());
            }
        }
    }
    newest
}

fn not_runner(file: &Path) -> bool {
    // a source that is not UTF-8 is still a source
    fs::read(file).is_ok_and(|text| !RUNNER.is_match(&String::from_utf8_lossy(&text)))
}

/// R8: when `src/test/java` holds Java code other than JUnit runners, the notice if the build's compiled test folder
/// has no class, or if such a source is newer than its newest class and than `built_at`, the start of the last
/// successful Build; else None. Reads the `.java` files and the `.class` file times, on open and Refresh.
pub fn test_code_notice(root: &Path, layout: Layout, built_at: Option<SystemTime>) -> Option<String> {
    let (compiled, build) = match layout {
        Layout::Maven => ("target/test-classes", "mvn test-compile"),
        Layout::Gradle => ("build/classes/java/test", "gradle testClasses"),
        Layout::Standalone => return None,
    };
    let sources = newest(&root.join("src").join("test").join("java"), ".java", not_runner)?;
    let Some(classes) = newest(&root.join(compiled), ".class", |_| true) else {
        return Some(format!(
            "The project's Java code in src/test/java is not built: build it ({build}) before running."
        ));
    };
    let built = built_at.map_or(classes, |at| at.max(classes));
    (sources > built).then(|| {
        format!(
            "The project's Java code in src/test/java changed since its last build: build it again ({build}) before \
             running."
        )
    })
}

/// Build from the notice R4.3: when Grace's last successful Build started; None if none.
pub fn read_built(dir: &Path) -> Option<SystemTime> {
    let bytes = fs::read(dir.join(BUILT)).ok()?;
    let at = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()?
        .get("at")?
        .as_u64()?;
    UNIX_EPOCH.checked_add(Duration::from_millis(at))
}

/// Keeps `at` in `<dir>/built.json`, in milliseconds.
pub fn write_built(dir: &Path, at: SystemTime) -> io::Result<()> {
    let at = at.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
    fs::create_dir_all(dir)?;
    fs::write(dir.join(BUILT), serde_json::json!({ "at": at }).to_string())
}

#[cfg(test)]
mod tests {
    //! `compiled.test.ts`.
    use super::*;
    use std::fs;
    use std::time::{Duration, UNIX_EPOCH};
    use tempfile::TempDir;

    /// 2020-01-01, in seconds since the Unix epoch.
    const BASE: u64 = 1_577_836_800;

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(BASE + seconds)
    }

    /// A temporary project of `files` (relative path, modified time in seconds since 2020), a file named in `texts`
    /// holding that text, the others empty.
    fn project(files: &[(&str, u64)], texts: &[(&str, &str)]) -> TempDir {
        let root = tempfile::Builder::new().prefix("kk-compiled-").tempdir().unwrap();
        for (name, seconds) in files {
            let path = root.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let text = texts.iter().find(|(f, _)| f == name).map_or("", |(_, text)| text);
            fs::write(&path, text).unwrap();
            let file = fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(at(*seconds)).unwrap();
        }
        root
    }

    fn maven(root: &TempDir) -> Option<String> {
        test_code_notice(root.path(), Layout::Maven, None)
    }

    fn not_built(command: &str) -> Option<String> {
        Some(format!(
            "The project's Java code in src/test/java is not built: build it ({command}) before running."
        ))
    }

    fn changed(command: &str) -> Option<String> {
        Some(format!(
            "The project's Java code in src/test/java changed since its last build: build it again ({command}) \
             before running."
        ))
    }

    const HELPER: &str = "src/test/java/a/Helper.java";
    const HELPER_CLASS: &str = "target/test-classes/a/Helper.class";
    const RUNNER_FILE: &str = "src/test/java/a/ATest.java";
    const RUNNER_CLASS: &str = "target/test-classes/a/ATest.class";

    mod test_code_notice {
        use super::*;

        #[test]
        fn says_nothing_without_java_code_in_src_test_java() {
            let root = project(&[("pom.xml", 0), ("src/test/java/a/a.feature", 10)], &[]);
            assert_eq!(maven(&root), None);
        }

        #[test]
        fn says_when_the_code_was_never_built() {
            let root = project(&[("pom.xml", 0), (HELPER, 10)], &[]);
            assert_eq!(maven(&root), not_built("mvn test-compile"));
        }

        #[test]
        fn counts_an_empty_compiled_folder_as_not_built() {
            let root = project(&[(HELPER, 10), ("target/test-classes/a.json", 20)], &[]);
            assert_eq!(maven(&root), not_built("mvn test-compile"));
        }

        #[test]
        fn says_nothing_when_the_newest_class_is_newer_than_every_source() {
            let root = project(
                &[(HELPER, 10), ("src/test/java/b/Other.java", 15), (HELPER_CLASS, 20)],
                &[],
            );
            assert_eq!(maven(&root), None);
        }

        #[test]
        fn says_when_a_source_changed_after_the_last_build() {
            let root = project(&[(HELPER, 30), (HELPER_CLASS, 20)], &[]);
            assert_eq!(maven(&root), changed("mvn test-compile"));
        }

        #[test]
        fn reads_gradles_compiled_folder_and_names_its_task() {
            let gradle = |root: &TempDir| test_code_notice(root.path(), Layout::Gradle, None);
            let root = project(&[(HELPER, 10)], &[]);
            assert_eq!(gradle(&root), not_built("gradle testClasses"));
            let built = project(&[(HELPER, 10), ("build/classes/java/test/a/Helper.class", 20)], &[]);
            assert_eq!(gradle(&built), None);
        }

        /// A runner as Karate's docs show it: no build is needed for it, never built or edited since.
        fn leaves_out(runner: &str) {
            let texts = [(RUNNER_FILE, runner)];
            assert_eq!(maven(&project(&[(RUNNER_FILE, 10)], &texts)), None);
            assert_eq!(maven(&project(&[(RUNNER_FILE, 30), (RUNNER_CLASS, 20)], &texts)), None);
        }

        const KARATE1: &str = "package a;\n\nimport com.intuit.karate.junit5.Karate;\n\nclass ATest {}\n";
        const KARATE2: &str = "package a;\n\nimport io.karatelabs.junit6.Karate;\nimport \
                               org.junit.jupiter.api.DynamicNode;\n\nclass ATest {}\n";
        const PARALLEL: &str = "package a;\n\nimport static org.junit.jupiter.api.Assertions.assertEquals;\n\nclass \
                                ATest {}\n";

        #[test]
        fn leaves_out_a_karate_1_junit_runner() {
            leaves_out(KARATE1);
        }

        #[test]
        fn leaves_out_a_karate_2_junit_runner() {
            leaves_out(KARATE2);
        }

        #[test]
        fn leaves_out_the_parallel_junit_runner() {
            leaves_out(PARALLEL);
        }

        /// Beyond the Vitest file: JavaScript's `\s` takes a byte order mark, and Node reads a file that is not UTF-8.
        #[test]
        fn leaves_out_a_runner_after_a_byte_order_mark_and_keeps_a_helper_that_is_not_utf8() {
            leaves_out("\u{feff}import org.junit.jupiter.api.Test;\r\nclass ATest {}\r\n");
            let root = project(&[(HELPER, 10)], &[]);
            fs::write(root.path().join(HELPER), b"// caf\xe9\nclass Helper {}\n").unwrap();
            assert_eq!(maven(&root), not_built("mvn test-compile"));
        }

        #[test]
        fn still_checks_a_helper_beside_a_runner() {
            let texts = [(RUNNER_FILE, KARATE2)];
            let unbuilt = project(&[(RUNNER_FILE, 10), (HELPER, 10)], &texts);
            assert_eq!(maven(&unbuilt), not_built("mvn test-compile"));
            let edited = project(
                &[(RUNNER_FILE, 10), (HELPER, 30), (RUNNER_CLASS, 20), (HELPER_CLASS, 20)],
                &texts,
            );
            assert_eq!(maven(&edited), changed("mvn test-compile"));
        }

        #[test]
        fn says_nothing_for_a_standalone_project() {
            let root = project(&[(HELPER, 10)], &[]);
            assert_eq!(test_code_notice(root.path(), Layout::Standalone, None), None);
        }
    }

    mod the_last_build {
        use super::*;

        #[test]
        fn counts_sources_older_than_the_last_build_as_built() {
            let root = project(&[(HELPER, 200), (HELPER_CLASS, 100)], &[]);
            let since = |seconds| test_code_notice(root.path(), Layout::Maven, Some(at(seconds)));
            assert_eq!(since(300), None);
            assert_eq!(since(150), changed("mvn test-compile"));
        }

        #[test]
        fn never_counts_a_project_with_no_class_as_built() {
            let root = project(&[(HELPER, 200)], &[]);
            let said = test_code_notice(root.path(), Layout::Maven, Some(at(300)));
            assert_eq!(said, not_built("mvn test-compile"));
        }

        #[test]
        fn keeps_the_time_in_built_json() {
            let root = project(&[], &[]);
            let dir = root.path().join("data");
            assert_eq!(read_built(&dir), None);
            let time = UNIX_EPOCH + Duration::from_millis(1234);
            write_built(&dir, time).unwrap();
            // Electron Grace's file: `{"at": <ms>}`
            assert_eq!(fs::read_to_string(dir.join("built.json")).unwrap(), r#"{"at":1234}"#);
            assert_eq!(read_built(&dir), Some(time));
            for other in [r#"{"at":"x"}"#, "{}", "not json"] {
                fs::write(dir.join("built.json"), other).unwrap();
                assert_eq!(read_built(&dir), None, "{other}");
            }
        }
    }
}
