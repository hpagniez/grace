//! `src/main/karate/environments.ts`: the environments a project names (docs/specs/2026-10-25-environment-switcher.md):
//! its `karate-config-<env>.js` overlays and the `@env=` and `@envnot=` tags of its feature files.
use super::js;
use crate::model::env::{env_tag_values, sorted_unique, valid_env_name};
use std::fs;
use std::path::{Path, PathBuf};

const SKIPPED_DIRS: [&str; 4] = ["node_modules", "target", "build", ".git"];

/// `<env>` of a `karate-config-<env>.js` file name, as written: `/^karate-config-(.+)\.js$/i`. The `i` of a JavaScript
/// regular expression without `u` folds only ASCII letters here, and `.` matches anything but a line end.
fn overlay_env(file: &str) -> Option<&str> {
    const PREFIX: &str = "karate-config-";
    const SUFFIX: &str = ".js";
    let len = file.len();
    if len <= PREFIX.len() + SUFFIX.len()
        || !file.is_char_boundary(PREFIX.len())
        || !file.is_char_boundary(len - SUFFIX.len())
        || !file[..PREFIX.len()].eq_ignore_ascii_case(PREFIX)
        || !file[len - SUFFIX.len()..].eq_ignore_ascii_case(SUFFIX)
    {
        return None;
    }
    let env = &file[PREFIX.len()..len - SUFFIX.len()];
    (!env.contains(['\n', '\r', '\u{2028}', '\u{2029}'])).then_some(env)
}

/// `<env>` of every `karate-config-<env>.js` directly inside each folder, as written. Case-insensitive: on Windows
/// Karate loads `Karate-Config-QA.js` for `-e QA`.
pub fn overlay_envs<P: AsRef<Path>>(dirs: &[P]) -> Vec<String> {
    let lists: Vec<Vec<String>> = dirs
        .iter()
        .map(|dir| {
            // folder missing or unreadable: no suggestions from it
            let Ok(entries) = fs::read_dir(dir) else {
                return vec![];
            };
            entries
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name();
                    let env = overlay_env(name.to_str()?)?;
                    valid_env_name(env).then(|| env.to_string())
                })
                .collect()
        })
        .collect();
    sorted_unique(&lists)
}

/// The feature files at or under `dir`, without following links to folders, skipping `SKIPPED_DIRS` at any depth.
fn feature_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        // a karate-pom.json path may be a file
        if dir.as_os_str().as_encoded_bytes().ends_with(b".feature") {
            out.push(dir.to_path_buf());
        }
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            if !SKIPPED_DIRS.iter().any(|s| name == *s) {
                feature_files(&e.path(), out);
            }
        } else if name.as_encoded_bytes().ends_with(b".feature") {
            out.push(e.path());
        }
    }
}

/// Tag lines are lines whose first token starts with `@`; `# @env=x` comments are not tag lines.
fn file_tag_envs(text: &str) -> Vec<String> {
    let tags: Vec<String> = text
        .split('\n')
        .flat_map(|line| {
            // `line.trim().split(/\s+/)`: JavaScript's white space
            let tokens: Vec<&str> = js::trim(line).split(js::is_space).filter(|t| !t.is_empty()).collect();
            let tag_line = tokens.first().is_some_and(|t| t.starts_with('@'));
            tokens
                .into_iter()
                .filter(move |t| tag_line && t.starts_with('@'))
                .map(|t| t[1..].to_string())
        })
        .collect();
    env_tag_values(&tags).into_iter().map(String::from).collect()
}

/// `feature_tag_envs` with the file reader given, so a test can watch the reads.
fn tag_envs_with<P: AsRef<Path>>(feature_roots: &[P], read: &dyn Fn(&Path) -> Option<String>) -> Vec<String> {
    let mut files = Vec::new();
    for root in feature_roots {
        feature_files(root.as_ref(), &mut files);
    }
    let envs: Vec<Vec<String>> = files
        .iter()
        .map(|f| read(f).map(|text| file_tag_envs(&text)).unwrap_or_default())
        .collect();
    sorted_unique(&envs)
}

/// Environment names written in `@env=`/`@envnot=` tags of the feature files under the roots. Never fails: a root or a
/// file that cannot be read gives nothing.
///
/// It blocks while it scans: call it off the window's thread. The files are read one at a time, as UTF-8 with a bad
/// byte replaced (Node's `"utf8"`).
// Rust-native: a plain function the caller runs on a background thread, reading one file at a time, in place of an
// async scan with 64 readers that kept Node's event loop free. Parity: the same names from the same files (the
// TypeScript's tests); `reads_at_most_64_files_at_once` now sees one read at a time, and
// `does_not_block_the_event_loop_while_it_scans` runs the scan on its own thread.
pub fn feature_tag_envs<P: AsRef<Path>>(feature_roots: &[P]) -> Vec<String> {
    tag_envs_with(feature_roots, &|file| {
        fs::read(file)
            .ok()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::karate::testing::copy_fixture;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::Builder::new().prefix("kk-env-").tempdir().unwrap()
    }

    /// Writes files (relative path -> text) under `root`, creating folders.
    fn put(root: &Path, files: &[(&str, &str)]) {
        for (rel, text) in files {
            let file = rel.split('/').fold(root.to_path_buf(), |p, s| p.join(s));
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
    }

    mod overlay_envs {
        use super::*;

        #[test]
        fn finds_overlay_files_in_each_layout() {
            let standalone = tmp();
            put(
                standalone.path(),
                &[("karate-config-qa.js", ""), ("karate-config-dev.js", "")],
            );
            assert_eq!(overlay_envs(&[standalone.path()]), ["dev", "qa"]);

            let maven = tmp();
            put(
                maven.path(),
                &[
                    ("karate-config-local.js", ""),
                    ("src/test/java/karate-config-qa.js", ""),
                    ("src/test/resources/karate-config-qa.js", ""),
                    ("src/test/resources/karate-config-prod.js", ""),
                ],
            );
            let java = maven.path().join("src").join("test").join("java");
            let resources = maven.path().join("src").join("test").join("resources");
            assert_eq!(
                overlay_envs(&[maven.path().to_path_buf(), java, resources]),
                ["local", "prod", "qa"]
            );
        }

        #[test]
        fn ignores_karate_config_js_bad_names_other_files_and_nested_folders() {
            let root = tmp();
            put(
                root.path(),
                &[
                    ("karate-config.js", ""),
                    ("karate-config-.js", ""),
                    ("karate-config--x.js", ""),
                    ("karate-config-a b.js", ""),
                    ("karate-config-qa.json", ""),
                    ("karate-config-qa.js.bak", ""),
                    ("my-karate-config-qa.js", ""),
                    ("karate-config-ok.js", ""),
                    ("sub/karate-config-nested.js", ""),
                ],
            );
            assert_eq!(overlay_envs(&[root.path()]), ["ok"]);
        }

        #[test]
        fn matches_the_file_name_case_insensitively_and_keeps_the_environment_as_written() {
            let root = tmp();
            put(
                root.path(),
                &[("Karate-Config-QA.js", ""), ("KARATE-CONFIG-dev.JS", "")],
            );
            assert_eq!(overlay_envs(&[root.path()]), ["QA", "dev"]);
        }

        #[test]
        fn skips_folders_that_do_not_exist() {
            let root = tmp();
            put(root.path(), &[("karate-config-qa.js", "")]);
            assert_eq!(
                overlay_envs(&[root.path().join("nope"), root.path().to_path_buf()]),
                ["qa"]
            );
            assert_eq!(overlay_envs::<PathBuf>(&[]), [] as [String; 0]);
        }
    }

    mod feature_tag_envs {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[test]
        fn reads_tag_values_from_the_env_fixture() {
            let env = copy_fixture("env");
            assert_eq!(feature_tag_envs(&[env.path()]), ["prod", "qa"]);
        }

        #[test]
        fn handles_several_tags_per_line_feature_tags_lists_and_comments() {
            let root = tmp();
            let a = [
                "@smoke @env=dev,qa",
                "Feature: a",
                "",
                "  # @env=commented",
                "  @envnot=prod @slow",
                "  Scenario: one",
                "    * print '@env=instep'",
                "",
                "@env=bad/name,ok-1,",
                "Scenario: two",
            ]
            .join("\n");
            put(
                root.path(),
                &[
                    ("a.feature", &a),
                    ("deep/er/b.feature", "@env=staging\r\nFeature: b\r\n"),
                    ("deep/er/c.txt", "@env=notafeature"),
                ],
            );
            assert_eq!(
                feature_tag_envs(&[root.path()]),
                ["dev", "ok-1", "prod", "qa", "staging"]
            );
        }

        #[test]
        fn merges_several_roots_and_survives_roots_that_do_not_exist() {
            let a = tmp();
            let b = tmp();
            put(a.path(), &[("x.feature", "@env=a\nFeature: x")]);
            put(b.path(), &[("y.feature", "@env=b\nFeature: y")]);
            let roots = [a.path().to_path_buf(), a.path().join("missing"), b.path().to_path_buf()];
            assert_eq!(feature_tag_envs(&roots), ["a", "b"]);
            assert_eq!(feature_tag_envs::<PathBuf>(&[]), [] as [String; 0]);
            assert_eq!(feature_tag_envs(&[a.path().join("x.feature")]), ["a"]);
        }

        #[test]
        fn skips_target_build_node_modules_and_git() {
            let root = tmp();
            put(
                root.path(),
                &[
                    ("keep/x.feature", "@env=kept\nFeature: x"),
                    ("target/x.feature", "@env=in-target\nFeature: x"),
                    ("build/x.feature", "@env=in-build\nFeature: x"),
                    ("node_modules/pkg/x.feature", "@env=in-modules\nFeature: x"),
                    (".git/x.feature", "@env=in-git\nFeature: x"),
                    ("keep/target/x.feature", "@env=nested-target\nFeature: x"),
                ],
            );
            assert_eq!(feature_tag_envs(&[root.path()]), ["kept"]);
        }

        // The TypeScript read with 64 readers at once and checked that 64 was the most in flight (EMFILE). The scan
        // here reads one file at a time on its caller's thread, so the most in flight is 1, within the bound.
        #[test]
        fn reads_at_most_64_files_at_once() {
            let root = tmp();
            let envs: Vec<String> = (0..200).map(|i| format!("e{i:03}")).collect();
            let files: Vec<(String, String)> = envs
                .iter()
                .enumerate()
                .map(|(i, e)| (format!("d{}/{e}.feature", i % 7), format!("@env={e}\nFeature: x")))
                .collect();
            let files: Vec<(&str, &str)> = files.iter().map(|(f, t)| (f.as_str(), t.as_str())).collect();
            put(root.path(), &files);
            let now = AtomicUsize::new(0);
            let max = AtomicUsize::new(0);
            let read = |file: &Path| {
                max.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                let text = fs::read_to_string(file).ok();
                now.fetch_sub(1, Ordering::SeqCst);
                text
            };
            assert_eq!(tag_envs_with(&[root.path()], &read), envs);
            assert!(max.load(Ordering::SeqCst) <= 64);
            assert_eq!(max.load(Ordering::SeqCst), 1);
        }

        // The TypeScript checked that its async scan let a `setImmediate` run. Here the scan blocks its own thread, so
        // the caller runs it on another and goes on meanwhile: each read waits for a tick of the caller's thread, so
        // the scan ends only because that thread ran while it scanned.
        #[test]
        fn does_not_block_the_event_loop_while_it_scans() {
            let root = tmp();
            put(
                root.path(),
                &[
                    ("a.feature", "@env=a\nFeature: a"),
                    ("x/b.feature", "@env=b\nFeature: b"),
                    ("x/y/c.feature", "@env=c\nFeature: c"),
                ],
            );
            let dir = root.path().to_path_buf();
            let (tick, ticks) = std::sync::mpsc::channel::<()>();
            let scan = std::thread::spawn(move || {
                let read = move |file: &Path| {
                    ticks.recv().ok();
                    fs::read_to_string(file).ok()
                };
                tag_envs_with(&[dir], &read)
            });
            for _ in 0..3 {
                tick.send(()).unwrap();
            }
            assert_eq!(scan.join().unwrap(), ["a", "b", "c"]);
        }
    }

    // Not in the TypeScript tests: the regular expression of `overlayEnvs`, written as a check of the name.
    #[test]
    fn overlay_env_is_the_typescripts_regular_expression() {
        assert_eq!(overlay_env("karate-config-qa.js"), Some("qa"));
        assert_eq!(overlay_env("KARATE-CONFIG-qa.JS"), Some("qa"));
        assert_eq!(overlay_env("karate-config-.js"), None);
        assert_eq!(overlay_env("karate-config-a.js.js"), Some("a.js"));
        assert_eq!(overlay_env("karate-config-é.js"), Some("é"));
        assert_eq!(overlay_env("karate-config-a\nb.js"), None);
        assert_eq!(overlay_env("karate-config.js"), None);
        // the long s folds to S in JavaScript only with the `u` flag
        assert_eq!(overlay_env("karate-config-x.j\u{17f}"), None);
    }
}
