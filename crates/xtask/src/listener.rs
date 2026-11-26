//! `cargo xtask listener`: compiles the step debugger's listener, `listener/grace/DebugListener.java`, into
//! `vendor/listener.jar` with the JDK of `JAVA_HOME`, against `vendor/karate.jar`. For Linux, where the `rust` job's
//! tests run the system's Java 21 and `scripts/fetch-vendor.mjs --jar` fetches the Karate jar alone: that script
//! compiles the listener with ECJ on the bundled Windows JRE (ADR 0025). The source is the same; the jar is not shipped.
use crate::rules::root;
use std::env::consts::EXE_SUFFIX;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A program of the JDK and its arguments.
type Step = (PathBuf, Vec<OsString>);

/// `javac`, then `jar`: the classes of `root/listener` into `classes`, then zipped into `vendor/listener.jar`.
fn steps(java_home: &Path, root: &Path, classes: &Path) -> [Step; 2] {
    let bin = |name: &str| java_home.join("bin").join(format!("{name}{EXE_SUFFIX}"));
    let vendor = root.join("vendor");
    let javac = vec![
        "--release".into(),
        "21".into(),
        "-nowarn".into(),
        "-cp".into(),
        vendor.join("karate.jar").into(),
        "-d".into(),
        classes.into(),
        root.join("listener").join("grace").join("DebugListener.java").into(),
    ];
    let jar = vec![
        "--create".into(),
        "--file".into(),
        vendor.join("listener.jar").into(),
        "-C".into(),
        classes.into(),
        "grace".into(),
    ];
    [(bin("javac"), javac), (bin("jar"), jar)]
}

pub fn run() -> u8 {
    let Some(java_home) = std::env::var_os("JAVA_HOME") else {
        eprintln!("cargo xtask listener: JAVA_HOME is not set; it names the JDK that compiles the listener");
        return 1;
    };
    let root = root();
    let classes = root.join("vendor").join(".download").join("listener-classes");
    let _ = fs::remove_dir_all(&classes);
    for (program, args) in steps(Path::new(&java_home), root, &classes) {
        match Command::new(&program).args(&args).status() {
            Ok(s) if s.success() => {}
            Ok(s) => {
                eprintln!("cargo xtask listener: {} failed ({s})", program.display());
                return 1;
            }
            Err(e) => {
                eprintln!("cargo xtask listener: {} could not start: {e}", program.display());
                return 1;
            }
        }
    }
    println!("vendor/listener.jar ready");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(args: &[OsString]) -> Vec<String> {
        args.iter().map(|a| a.to_string_lossy().replace('\\', "/")).collect()
    }

    #[test]
    fn compiles_the_listener_against_karate_then_zips_its_classes() {
        let [(javac, compile), (jar, zip)] = steps(Path::new("/jdk"), Path::new("/repo"), Path::new("/tmp/classes"));
        assert_eq!(javac, Path::new("/jdk/bin").join(format!("javac{EXE_SUFFIX}")));
        assert_eq!(
            strings(&compile),
            [
                "--release",
                "21",
                "-nowarn",
                "-cp",
                "/repo/vendor/karate.jar",
                "-d",
                "/tmp/classes",
                "/repo/listener/grace/DebugListener.java"
            ]
        );
        assert_eq!(jar, Path::new("/jdk/bin").join(format!("jar{EXE_SUFFIX}")));
        assert_eq!(
            strings(&zip),
            [
                "--create",
                "--file",
                "/repo/vendor/listener.jar",
                "-C",
                "/tmp/classes",
                "grace"
            ]
        );
    }
}
