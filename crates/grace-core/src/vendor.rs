//! `src/main/vendor.ts`: where the Java and the Karate jar that Grace runs are (ADR 0006: bundled, never the
//! machine's). Windows ships the JRE in `vendor/jre`. Linux builds and runs for development and CI only (migration
//! guide R5.2) and has no bundled JRE: there Grace runs `GRACE_JAVA`, else the first `java` on `PATH`, with the
//! pinned `vendor/karate.jar` all the same.
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendorPaths {
    pub java: PathBuf,
    pub jar: PathBuf,
    /// The step debugger's Karate listener (ADR 0025); only a debug run puts it on the classpath.
    pub listener: PathBuf,
    /// The JRE that `java` belongs to: a Java left behind by a crash is killed at the next start only if it runs from
    /// here (`karate::process::kill_orphan`).
    pub jre_dir: PathBuf,
}

/// `base` is `<repo>/vendor` in development and tests, the folder of `Grace.exe`'s resources when packaged.
pub fn vendor_paths(base: &Path) -> VendorPaths {
    vendor_paths_from(base, |name| env::var_os(name))
}

/// `vendor_paths` with the environment `var` gives.
// Rust-native: Linux, which Electron Grace never ran on, takes the machine's Java (guide R5.2); Windows is
// `vendorPaths` as it was. Parity: `the_bundled_jre_on_windows`, and the Linux tests below.
pub fn vendor_paths_from(base: &Path, var: impl Fn(&str) -> Option<OsString>) -> VendorPaths {
    let jar = base.join("karate.jar");
    let listener = base.join("listener.jar");
    if cfg!(windows) {
        let jre_dir = base.join("jre");
        return VendorPaths {
            java: jre_dir.join("bin").join("java.exe"),
            jar,
            listener,
            jre_dir,
        };
    }
    let java = var("GRACE_JAVA")
        .filter(|j| !j.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            let path = var("PATH")?;
            env::split_paths(&path)
                .map(|dir| dir.join("java"))
                .find(|j| j.is_file())
        })
        // not found: the spawn says so (`spawn java ENOENT`)
        .unwrap_or_else(|| PathBuf::from("java"));
    // `/usr/lib/jvm/<jdk>/bin/java`, links resolved (`/usr/bin/java` is a link to it)
    let jre_dir = fs::canonicalize(&java)
        .ok()
        .and_then(|real| Some(real.parent()?.parent()?.to_path_buf()))
        .unwrap_or_else(|| base.join("jre"));
    VendorPaths {
        java,
        jar,
        listener,
        jre_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<OsString> {
        None
    }

    #[cfg(windows)]
    #[test]
    fn the_bundled_jre_on_windows() {
        let base = Path::new(r"C:\Grace\resources");
        assert_eq!(
            vendor_paths_from(base, |_| Some("C:\\java".into())),
            VendorPaths {
                java: base.join("jre").join("bin").join("java.exe"),
                jar: base.join("karate.jar"),
                listener: base.join("listener.jar"),
                jre_dir: base.join("jre"),
            }
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn grace_java_wins_over_path() {
        let dir = tempfile::tempdir().unwrap();
        let jdk = dir.path().join("jdk");
        fs::create_dir_all(jdk.join("bin")).unwrap();
        fs::write(jdk.join("bin").join("java"), "").unwrap();
        let on_path = dir.path().join("other");
        fs::create_dir_all(&on_path).unwrap();
        fs::write(on_path.join("java"), "").unwrap();
        let java = jdk.join("bin").join("java");
        let vars = |name: &str| match name {
            "GRACE_JAVA" => Some(java.clone().into_os_string()),
            "PATH" => Some(on_path.clone().into_os_string()),
            _ => None,
        };
        let v = vendor_paths_from(Path::new("/repo/vendor"), vars);
        assert_eq!(v.java, java);
        assert_eq!(v.jre_dir, fs::canonicalize(&jdk).unwrap());
        assert_eq!(v.jar, Path::new("/repo/vendor/karate.jar"));
        assert_eq!(v.listener, Path::new("/repo/vendor/listener.jar"));
    }

    #[cfg(not(windows))]
    #[test]
    fn takes_the_first_java_on_path_and_its_jre() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        let jdk = dir.path().join("jdk");
        fs::create_dir_all(&empty).unwrap();
        fs::create_dir_all(jdk.join("bin")).unwrap();
        fs::write(jdk.join("bin").join("java"), "").unwrap();
        // `/usr/bin/java` is a link into the JDK
        let links = dir.path().join("usr-bin");
        fs::create_dir_all(&links).unwrap();
        std::os::unix::fs::symlink(jdk.join("bin").join("java"), links.join("java")).unwrap();
        let path = env::join_paths([&empty, &links]).unwrap();
        let v = vendor_paths_from(Path::new("/repo/vendor"), |name| (name == "PATH").then(|| path.clone()));
        assert_eq!(v.java, links.join("java"));
        assert_eq!(v.jre_dir, fs::canonicalize(&jdk).unwrap());
    }

    #[cfg(not(windows))]
    #[test]
    fn no_java_found_is_java_for_the_spawn_to_report() {
        let v = vendor_paths_from(Path::new("/repo/vendor"), none);
        assert_eq!(v.java, Path::new("java"));
        assert_eq!(v.jre_dir, Path::new("/repo/vendor/jre"));
    }

    #[test]
    fn the_jar_and_the_listener_are_in_the_vendor_folder() {
        let v = vendor_paths_from(Path::new("vendor"), none);
        assert_eq!(v.jar, Path::new("vendor").join("karate.jar"));
        assert_eq!(v.listener, Path::new("vendor").join("listener.jar"));
    }
}
