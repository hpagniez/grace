//! What Electron Grace's `src/main/karate/` holds: the Karate CLI's side of a run. The command line (`command`), the
//! argument files (`argfile`), the Java process (`process`), the environments a project names (`environments`), what
//! Grace reads of `karate-pom.json` (`pom`), and the run's events: reading `karate-events.jsonl` (`events`), the dry
//! run's tree (`tree`), the run's state (`run_state`), and the steps and HTTP calls of a finished feature (`steps`,
//! `http`). `debug` holds only the listener's class name until the step debugger ports (phase P4).
pub mod argfile;
pub mod command;
pub mod debug;
pub mod environments;
pub mod events;
pub mod http;
mod js;
pub mod pom;
pub mod process;
pub mod run_state;
pub mod steps;
pub mod tree;

/// What the tests that start the real Java share (guide R6.1): the `java` and the pinned `vendor/karate.jar`, and
/// `test/fixtures/copy.ts`'s `copyFixture`.
#[cfg(test)]
pub(crate) mod testing {
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    /// `vendor.ts`'s `vendorPaths` for the tests: the bundled JRE on Windows; on Linux the `java` on `PATH`, and as
    /// its JRE folder the one that `java` runs from (`/usr/lib/jvm/<jdk>`, symbolic links resolved).
    pub struct Vendor {
        pub java: PathBuf,
        pub jar: PathBuf,
        pub jre_dir: PathBuf,
    }

    fn repo() -> PathBuf {
        // not canonicalized: on Windows that gives a `\\?\` path, which Java's classpath does not take
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
    }

    pub fn vendor() -> Vendor {
        let vendor = repo().join("vendor");
        let jar = vendor.join("karate.jar");
        assert!(jar.is_file(), "{} is missing: run `npm run vendor`", jar.display());
        if cfg!(windows) {
            let jre_dir = vendor.join("jre");
            let java = jre_dir.join("bin").join("java.exe");
            return Vendor { java, jar, jre_dir };
        }
        let on_path = std::env::var_os("PATH")
            .iter()
            .flat_map(std::env::split_paths)
            .map(|dir| dir.join("java"))
            .find(|java| java.is_file())
            .expect("no java on PATH");
        let real = fs::canonicalize(&on_path).unwrap();
        let jre_dir = real.parent().and_then(Path::parent).unwrap().to_path_buf();
        Vendor {
            java: PathBuf::from("java"),
            jar,
            jre_dir,
        }
    }

    /// `test/fixtures/projects/<name>`, read in place (never copied into the crate).
    pub fn fixture(name: &str) -> PathBuf {
        repo().join("test").join("fixtures").join("projects").join(name)
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

    /// `copyFixture(name)` for the fixtures that are one folder: a fresh copy in its own temporary folder, removed
    /// when the `TempDir` is dropped.
    pub fn copy_fixture(name: &str) -> TempDir {
        let dir = tempfile::Builder::new()
            .prefix(&format!("kk-{name}-"))
            .tempdir()
            .unwrap();
        copy_dir(&fixture(name), dir.path());
        dir
    }
}
