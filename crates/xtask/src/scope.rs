//! `cargo xtask scope [<base>]`: the areas of the repository a diff touches, which say the CI jobs it needs
//! (docs/specs/2026-11-12-ci-scope.md). `test.yml` and `perf.yml` read its lines from `$GITHUB_OUTPUT`.

/// Electron Grace: `unit`, `e2e` and `perf`.
pub const ELECTRON: u8 = 1;
/// GPUI Grace: `rust`.
pub const GPUI: u8 = 2;
/// The Node scripts, whose tests are in `npm test`: `unit`.
pub const SCRIPTS: u8 = 4;
pub const ALL: u8 = ELECTRON | GPUI | SCRIPTS;

/// The rule, first match wins: a pattern ending in `/` is a folder, one starting with `*` a suffix, one ending in `*`
/// the start of a top-level file's name, any other a path. A path no row matches touches every area.
const RULES: &[(&str, u8)] = &[
    // `checks` covers these, or nothing runs them on a PR
    ("*.md", 0),
    ("docs/", 0),
    (".claude/", 0),
    (".github/", 0),
    ("crates/xtask/", 0),
    ("LICENSE", 0),
    (".gitignore", 0),
    (".prettierignore", 0),
    (".oxlintrc.json", 0),
    ("rustfmt.toml", 0),
    (".signpath/", 0),
    // GPUI Grace's tests read them: the icons, the theme, the fixtures, the versions, the JRE and jar
    ("src/renderer/", ELECTRON | GPUI),
    ("src/index.css", ELECTRON | GPUI),
    ("index.html", ELECTRON | GPUI),
    ("assets/", ELECTRON | GPUI),
    ("test/", ELECTRON | GPUI),
    ("listener/", ELECTRON | GPUI),
    ("package.json", ELECTRON | GPUI),
    ("scripts/vendor.json", ELECTRON | GPUI),
    ("scripts/fetch-vendor.mjs", ELECTRON | GPUI),
    ("src/", ELECTRON),
    ("e2e/", ELECTRON),
    ("bench/", ELECTRON),
    ("package-lock.json", ELECTRON),
    ("tsconfig.json", ELECTRON),
    ("forge.config.mts", ELECTRON),
    ("vite.*", ELECTRON),
    ("vitest.config.mts", ELECTRON),
    ("playwright.*", ELECTRON),
    ("crates/", GPUI),
    // scripts/bump-cargo.test.ts reads the pins and the patches
    ("patches/", GPUI | SCRIPTS),
    ("Cargo.toml", GPUI | SCRIPTS),
    ("rust-toolchain.toml", GPUI | SCRIPTS),
    ("Cargo.lock", GPUI),
    (".cargo/", GPUI),
    ("scripts/e2e-gpui.sh", GPUI),
    ("scripts/", SCRIPTS),
];

const NAMES: [(&str, u8); 3] = [("electron", ELECTRON), ("gpui", GPUI), ("scripts", SCRIPTS)];

fn matches(pattern: &str, path: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        path.ends_with(suffix)
    } else if let Some(start) = pattern.strip_suffix('*') {
        !path.contains('/') && path.starts_with(start)
    } else if pattern.ends_with('/') {
        path.starts_with(pattern)
    } else {
        path == pattern
    }
}

/// The areas a path touches.
pub fn areas(path: &str) -> u8 {
    RULES
        .iter()
        .find(|(pattern, _)| matches(pattern, path))
        .map_or(ALL, |&(_, areas)| areas)
}

/// The `name=true|false` lines of `$GITHUB_OUTPUT`.
pub fn outputs(areas: u8) -> String {
    NAMES
        .iter()
        .map(|(name, area)| format!("{name}={}\n", areas & area != 0))
        .collect()
}

/// The areas of `git diff <base>...HEAD`; without a base (a run by hand), every area.
pub fn scope(base: Option<&str>, git: &dyn Fn(&[&str]) -> Option<String>) -> Result<u8, String> {
    let Some(base) = base.filter(|base| !base.is_empty()) else {
        return Ok(ALL);
    };
    let range = format!("{base}...HEAD");
    let diff = git(&["diff", "--name-only", "--no-renames", &range])
        .ok_or_else(|| format!("git diff {range} failed: is {base} fetched?"))?;
    Ok(diff.lines().fold(0, |all, path| all | areas(path)))
}

/// Prints the areas of the diff from `base`, or the reason it cannot, and gives the exit code.
pub fn run(base: Option<String>) -> u8 {
    match scope(base.as_deref(), &|args| crate::rules::git(crate::rules::root(), args)) {
        Ok(areas) => {
            print!("{}", outputs(areas));
            0
        }
        Err(error) => {
            // stderr: stdout goes to $GITHUB_OUTPUT
            eprintln!("::error::{error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn each(paths: &[&str], expected: u8) {
        for path in paths {
            assert_eq!(areas(path), expected, "{path}");
        }
    }

    #[test]
    fn docs_and_xtask_touch_nothing() {
        each(
            &[
                "docs/specs/a.md",
                "docs/research/x/src/main.rs",
                ".github/workflows/test.yml",
                ".claude/rules/ci.md",
                "crates/xtask/src/main.rs",
                "AGENTS.md",
                "LICENSE",
                "rustfmt.toml",
            ],
            0,
        );
    }

    #[test]
    fn markdown_anywhere_touches_nothing() {
        each(&["src/README.md", "patches/README.md"], 0);
    }

    #[test]
    fn renderer_touches_both_apps() {
        each(
            &[
                "src/renderer/App.tsx",
                "src/index.css",
                "index.html",
                "test/fixtures/events/a.jsonl",
                "assets/icon.svg",
                "listener/grace/DebugListener.java",
                "package.json",
                "scripts/vendor.json",
                "scripts/fetch-vendor.mjs",
            ],
            ELECTRON | GPUI,
        );
    }

    #[test]
    fn main_process_touches_electron() {
        each(
            &[
                "src/main/runner.ts",
                "e2e/run.spec.ts",
                "bench/bench.mjs",
                "package-lock.json",
                "tsconfig.json",
                "vite.main.config.mts",
                "playwright.config.ts",
                "forge.config.mts",
            ],
            ELECTRON,
        );
    }

    #[test]
    fn crates_touch_gpui() {
        each(
            &[
                "crates/grace/src/lib.rs",
                "crates/grace-core/src/lib.rs",
                "Cargo.lock",
                ".cargo/config.toml",
                "scripts/e2e-gpui.sh",
            ],
            GPUI,
        );
    }

    #[test]
    fn the_cargo_bump_tests_read_the_pins_and_patches() {
        // scripts/bump-cargo.test.ts, in `npm test`, reads them
        each(
            &[
                "Cargo.toml",
                "rust-toolchain.toml",
                "patches/series",
                "patches/check.sh",
                "patches/gpui-pre-windows/Cargo.toml",
            ],
            GPUI | SCRIPTS,
        );
    }

    #[test]
    fn node_scripts_touch_scripts() {
        each(&["scripts/bump-pr.mjs", "scripts/bump-pr.test.ts"], SCRIPTS);
    }

    #[test]
    fn an_unknown_path_touches_every_area() {
        each(&[".gitattributes", "new-tool.toml"], ALL);
    }

    #[test]
    fn a_folder_row_matches_only_that_folder() {
        each(&["src-old/x.ts", "docs-old/a.txt", "vite/x.ts"], ALL);
    }

    #[test]
    fn outputs_names_every_area() {
        assert_eq!(outputs(0), "electron=false\ngpui=false\nscripts=false\n");
        assert_eq!(outputs(ELECTRON | SCRIPTS), "electron=true\ngpui=false\nscripts=true\n");
        assert_eq!(outputs(ALL), "electron=true\ngpui=true\nscripts=true\n");
    }

    #[test]
    fn no_base_touches_every_area() {
        let git = |_: &[&str]| -> Option<String> { panic!("no git without a base") };
        assert_eq!(scope(None, &git), Ok(ALL));
        assert_eq!(scope(Some(""), &git), Ok(ALL));
    }

    #[test]
    fn scope_ors_the_diff() {
        let called = RefCell::new(Vec::new());
        let git = |args: &[&str]| {
            called.replace(args.iter().map(|arg| arg.to_string()).collect());
            Some("src/main/a.ts\ncrates/grace/b.rs\n".to_string())
        };
        assert_eq!(scope(Some("origin/develop"), &git), Ok(ELECTRON | GPUI));
        assert_eq!(
            *called.borrow(),
            ["diff", "--name-only", "--no-renames", "origin/develop...HEAD"]
        );
    }

    #[test]
    fn an_empty_diff_touches_nothing() {
        assert_eq!(scope(Some("HEAD^1"), &|_| Some(String::new())), Ok(0));
    }

    #[test]
    fn run_fails_when_git_fails() {
        assert!(scope(Some("no-such-ref"), &|_| None).is_err());
    }
}
