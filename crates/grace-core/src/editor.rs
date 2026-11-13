//! `src/main/editor.ts`: which file Open in VS Code may open, and the link that opens it (docs/specs/2026-11-03-open-in-editor.md).
//!
//! `validRequest` and `validPoint` (R3.1) are not ported: they checked what arrived over IPC, and here a path is a
//! `Path` and a line a `u32` (plan P3, Redesigns, `app`).
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::karate::command::{absolute, inside};

/// `file` under `root` as the project was opened, if it is an existing file whose real path, links followed, is inside
/// `root`'s real folder (R3.3); `None` otherwise. A mapped drive keeps its letter: its real path is the share's.
pub fn project_file(root: &Path, file: &Path) -> Option<PathBuf> {
    project_file_with(root, file, &real_path)
}

/// `project_file` with the way to read a real path given, so a test can count what is read.
fn project_file_with(root: &Path, file: &Path, real: &dyn Fn(&Path) -> io::Result<PathBuf>) -> Option<PathBuf> {
    let real_root = real(root).ok()?;
    let abs = absolute(file);
    // Checked by name before any read: reading a share or a device path would reach it.
    // Karate may write the project's path as given or as its real path (8.3 names expanded).
    if !inside(root, &abs) && !inside(&real_root, &abs) {
        return None;
    }
    let real = real(&abs).ok()?;
    if !inside(&real_root, &real) || !fs::metadata(&real).ok()?.is_file() {
        return None;
    }
    Some(
        root.join(
            real.components()
                .skip(real_root.components().count())
                .collect::<PathBuf>(),
        ),
    )
}

/// Where Karate says a test failed: the first line of its error, an absolute Windows path, a colon and a line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub file: String,
    pub line: u32,
}

/// The place on a failed test's error's first line (open-in-editor R2.1); `None` when it names none: `failedAt` of
/// `nodes.ts`. A line of 0 is no place: VS Code has no line 0, and Electron Grace's request check refused it.
pub fn failed_at(error: &str) -> Option<Place> {
    let first = error.split('\n').next()?;
    let b = first.as_bytes();
    if b.len() < 3 || !b[0].is_ascii_alphabetic() || b[1] != b':' || !matches!(b[2], b'\\' | b'/') {
        return None;
    }
    let (file, line) = first.rsplit_once(':')?;
    // `file` keeps the drive's own colon, so a line-less first line leaves the drive alone: `C` and no `\`
    if file.len() < 3 || line.is_empty() || !line.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let line: u32 = line.parse().ok().filter(|l| *l >= 1)?;
    Some(Place { file: file.into(), line })
}

/// Node's `realpathSync.native`: the real path, links followed, without the `\\?\` prefix Rust's `canonicalize` adds.
fn real_path(path: &Path) -> io::Result<PathBuf> {
    path.canonicalize()
        .map(|p| PathBuf::from(strip_verbatim(&p.to_string_lossy())))
}

/// `\\?\C:\p` as `C:\p` and `\\?\UNC\server\share` as `\\server\share`.
fn strip_verbatim(path: &str) -> String {
    match path.strip_prefix(r"\\?\") {
        Some(rest) => match rest.strip_prefix(r"UNC\") {
            Some(share) => format!(r"\\{share}"),
            None => rest.to_string(),
        },
        None => path.to_string(),
    }
}

/// VS Code's link to `file` at `line` (R4.2): `/` separators, and what `encodeURI` encodes plus `#` and `?`, which would
/// end the path. A share's `//server` follows `file` directly: VS Code reads `file///server` as `\server`.
pub fn vscode_url(file: &Path, line: u32) -> String {
    let mut path = String::new();
    for b in file.to_string_lossy().replace('\\', "/").bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b';'
            | b','
            | b'/'
            | b':'
            | b'@'
            | b'&'
            | b'='
            | b'+'
            | b'$'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => path.push(b as char),
            _ => path.push_str(&format!("%{b:02X}")),
        }
    }
    let slash = if path.starts_with('/') { "" } else { "/" };
    format!("vscode://file{slash}{path}:{line}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use tempfile::TempDir;

    /// An empty file at `rel` under `base`, with its folders; its path.
    fn touch(base: &Path, rel: &str) -> PathBuf {
        let f = base.join(rel);
        fs::create_dir_all(f.parent().unwrap()).unwrap();
        fs::write(&f, "").unwrap();
        f
    }

    /// `outer/p` holds `tests/a.feature`; `outer` is removed with the test.
    fn project() -> (TempDir, PathBuf) {
        let outer = tempfile::Builder::new().prefix("kk-editor-").tempdir().unwrap();
        // the real folder, so a machine whose temp folder is a link still opens the project by its real path
        let base = real_path(outer.path()).unwrap();
        let root = base.join("p");
        touch(&root, "tests/a.feature");
        (outer, root)
    }

    /// A link to a folder: a symbolic link on Linux, a junction on Windows (`mklink /J` needs no privilege).
    fn link_dir(target: &Path, link: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(target, link).unwrap();
        #[cfg(windows)]
        assert!(
            std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(link)
                .arg(target)
                .output()
                .unwrap()
                .status
                .success()
        );
    }

    mod project_file {
        use super::*;

        #[test]
        fn keeps_a_file_of_the_project_by_its_path_in_the_project_whatever_its_letter_case() {
            let (_outer, root) = project();
            let file = root.join("tests").join("a.feature");
            assert_eq!(project_file(&root, &file), Some(file.clone()));
            // only a case-insensitive file system answers to another case
            if cfg!(windows) {
                let upper = PathBuf::from(file.to_string_lossy().to_uppercase());
                assert_eq!(project_file(&root, &upper), Some(file));
            }
        }

        #[test]
        fn names_the_file_under_the_project_folder_as_it_was_opened_a_mapped_drive_keeps_its_letter() {
            // A link stands for a mapped drive: its real path is elsewhere, as `Z:\` is `\\server\share`.
            let (outer, root) = project();
            let opened = real_path(outer.path()).unwrap().join("z");
            link_dir(&root, &opened);
            let file = opened.join("tests").join("a.feature");
            assert_eq!(project_file(&opened, &file), Some(file));
        }

        #[test]
        fn keeps_a_file_under_a_folder_whose_name_starts_with_two_dots() {
            let (_outer, root) = project();
            let file = touch(&root, "..data/b.feature");
            assert_eq!(project_file(&root, &file), Some(file));
        }

        #[test]
        fn refuses_a_file_outside_the_project_a_missing_file_and_the_project_folder() {
            let (outer, root) = project();
            touch(&real_path(outer.path()).unwrap(), "outside.feature");
            let other = tempfile::Builder::new().prefix("kk-editor-other-").tempdir().unwrap();
            assert_eq!(project_file(&root, &root.join("..").join("outside.feature")), None);
            assert_eq!(project_file(&root, &touch(other.path(), "x.feature")), None);
            assert_eq!(project_file(&root, &root.join("tests").join("gone.feature")), None);
            assert_eq!(project_file(&root, &root), None);
        }

        #[test]
        fn refuses_a_path_outside_the_project_without_reading_it_a_share_or_a_device_is_never_reached() {
            let (_outer, root) = project();
            let outside = [
                r"\\attacker\s\x.feature",
                "//attacker/s/x.feature",
                r"\\.\pipe\x",
                r"\\?\C:\Windows\win.ini",
                r"C:\Windows\win.ini",
            ];
            let read = RefCell::new(Vec::new());
            let real = |p: &Path| {
                read.borrow_mut().push(p.to_path_buf());
                real_path(p)
            };
            for file in outside {
                assert_eq!(project_file_with(&root, Path::new(file), &real), None, "{file}");
            }
            // a file of the project is read, so a refusal that reads nothing says something
            let inside_file = root.join("tests").join("a.feature");
            assert!(project_file_with(&root, &inside_file, &real).is_some());
            let read = read.borrow();
            for file in outside {
                assert!(!read.iter().any(|p| p == Path::new(file)), "{file} was read");
            }
        }

        #[test]
        fn refuses_a_file_reached_through_a_link_to_a_folder_outside_the_project() {
            let (outer, root) = project();
            touch(&real_path(outer.path()).unwrap(), "other/out.feature");
            link_dir(&real_path(outer.path()).unwrap().join("other"), &root.join("link"));
            assert_eq!(project_file(&root, &root.join("link").join("out.feature")), None);
        }

        #[test]
        fn drops_the_verbatim_prefix_of_a_real_path() {
            assert_eq!(strip_verbatim(r"\\?\C:\p\a.feature"), r"C:\p\a.feature");
            assert_eq!(
                strip_verbatim(r"\\?\UNC\server\share\a.feature"),
                r"\\server\share\a.feature"
            );
            assert_eq!(strip_verbatim(r"C:\p\a.feature"), r"C:\p\a.feature");
        }
    }

    mod failed_at {
        use super::*;

        fn place(file: &str, line: u32) -> Option<Place> {
            Some(Place { file: file.into(), line })
        }

        #[test]
        fn takes_the_place_karate_names_on_the_first_line() {
            let error = "C:\\p\\tests\\auth.feature:9\nThen status 200\nexpected status: 200";
            assert_eq!(failed_at(error), place("C:\\p\\tests\\auth.feature", 9));
        }

        #[test]
        fn takes_a_lowercase_drive_and_slash_separators() {
            assert_eq!(failed_at("c:/p/a.feature:3\nboom"), place("c:/p/a.feature", 3));
        }

        #[test]
        fn takes_the_last_colon_as_the_line() {
            assert_eq!(failed_at("C:/p/a:b.feature:12"), place("C:/p/a:b.feature", 12));
        }

        #[test]
        fn finds_none_in_an_error_without_a_place() {
            for error in [
                "Config evaluation failed: karate-config.js - js failed:\nLine: 4",
                "boom at x.feature:7",
                "",
                "C:\\p\\a.feature",
                "C:\\p\\a.feature:",
                "C:\\p\\a.feature:x1",
                "C:\\p\\a.feature:0",
                "\nC:\\p\\a.feature:3",
                "tests/a.feature:3",
            ] {
                assert_eq!(failed_at(error), None, "{error:?}");
            }
        }
    }

    mod vscode_url {
        use super::*;

        #[test]
        fn uses_slash_separators_and_encodes_what_a_url_needs() {
            assert_eq!(
                vscode_url(Path::new(r"C:\My Project\a#b.feature"), 7),
                "vscode://file/C:/My%20Project/a%23b.feature:7"
            );
            assert_eq!(
                vscode_url(Path::new(r"C:\p\100%\é?.feature"), 2),
                "vscode://file/C:/p/100%25/%C3%A9%3F.feature:2"
            );
        }

        #[test]
        fn keeps_a_share_paths_two_leading_slashes_as_vs_code_reads_them() {
            assert_eq!(
                vscode_url(Path::new(r"\\server\share\p\x.feature"), 3),
                "vscode://file//server/share/p/x.feature:3"
            );
        }
    }
}
