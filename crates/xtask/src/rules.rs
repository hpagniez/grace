//! `cargo xtask check-rules`: the files that instruct agents stay within their limits.
//!
//! `WORKFLOW.md` and `AGENTS.md` load in every session, so their size is capped; `WORKFLOW.md` is copied by other
//! projects, so it names nothing of this one; a `.claude/rules/*.md` file loads only with the files its `paths:`
//! globs match, so a glob that matches nothing is a rule nobody ever reads.

use std::path::Path;
use std::process::Command;

/// The files loaded at the start of every session, and their size limits in bytes of the git blob.
pub const LIMITS: [(&str, usize); 2] = [("WORKFLOW.md", 5000), ("AGENTS.md", 9000)];

/// Words the general file may not hold, whatever their case.
pub const PROJECT_WORDS: [&str; 4] = ["grace", "karate", "gpui", "hpagniez"];

/// Headings accepted decision records link to.
const KEPT_HEADINGS: [&str; 2] = ["## Git workflow", "## No backward compatibility"];

/// The globs of a rules file's `paths:` frontmatter: a YAML list between the first two `---` lines.
pub fn rule_paths(text: &str) -> Vec<String> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Vec::new();
    }
    lines
        .take_while(|line| *line != "---")
        .skip_while(|line| line.trim_end() != "paths:")
        .skip(1)
        .map_while(|line| line.trim_start().strip_prefix("- "))
        .map(|item| item.trim().trim_matches(['"', '\'']).to_string())
        .collect()
}

/// One message per failure. `read` gives a file's text or `None`, `list` the tracked files a glob matches, `rules`
/// the paths of the rules files.
pub fn rules_files_errors(
    read: &dyn Fn(&str) -> Option<String>,
    list: &dyn Fn(&str) -> Vec<String>,
    rules: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();
    for (file, limit) in LIMITS {
        match read(file) {
            None => errors.push(format!("{file} is missing")),
            Some(text) if text.len() > limit => {
                errors.push(format!(
                    "{file} is {} bytes, over {limit}: move text to a rules file",
                    text.len()
                ));
            }
            Some(_) => {}
        }
    }
    if let Some(text) = read("WORKFLOW.md") {
        // ASCII lower case keeps every byte offset, so the word can be quoted as it was written
        let lower = text.to_ascii_lowercase();
        for word in PROJECT_WORDS {
            if let Some(at) = lower.find(word) {
                let found = &text[at..at + word.len()];
                errors.push(format!("WORKFLOW.md must stay general: it names \"{found}\""));
            }
        }
    }
    if let Some(text) = read("AGENTS.md") {
        let first = text.split("\n\n").find(|block| !block.trim_start().starts_with('#'));
        if !first.is_some_and(|block| block.contains("WORKFLOW.md")) {
            errors.push("AGENTS.md must start by pointing to WORKFLOW.md".to_string());
        }
        for heading in KEPT_HEADINGS {
            if !text.lines().any(|line| line == heading) {
                errors.push(format!(
                    "AGENTS.md has no \"{heading}\" heading: decision records link it"
                ));
            }
        }
    }
    for rule in rules {
        let globs = read(rule).map(|text| rule_paths(&text)).unwrap_or_default();
        if globs.is_empty() {
            errors.push(format!("{rule} has no paths: list, so it would load in every session"));
        }
        for glob in globs {
            if glob.contains(['{', '}', '!']) {
                errors.push(format!(
                    "{rule}: \"{glob}\" uses braces or !, which git and Claude Code read differently"
                ));
            } else if list(&glob).is_empty() {
                errors.push(format!(
                    "{rule}: \"{glob}\" matches no tracked file, so its rules would never load"
                ));
            }
        }
    }
    errors
}

/// The rules files one of whose globs lists `file`.
#[cfg(test)]
pub fn matching_rules(
    file: &str,
    read: &dyn Fn(&str) -> Option<String>,
    list: &dyn Fn(&str) -> Vec<String>,
    rules: &[String],
) -> Vec<String> {
    let lists = |rule: &&String| {
        let globs = read(rule).map(|text| rule_paths(&text)).unwrap_or_default();
        globs.iter().any(|glob| list(glob).iter().any(|listed| listed == file))
    };
    rules.iter().filter(lists).cloned().collect()
}

/// The repository this crate is in.
pub(crate) fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// A file's text in the index: what a commit would hold, with git's line endings, not the working tree's.
fn read_index(path: &str) -> Option<String> {
    git(root(), &["show", &format!(":{path}")])
}

/// The tracked files a glob matches.
fn list_index(glob: &str) -> Vec<String> {
    git(root(), &["ls-files", "--", &format!(":(glob){glob}")])
        .unwrap_or_default()
        .lines()
        .map(String::from)
        .collect()
}

/// Checks the repository's index, prints each failure as a CI error, and gives the exit code.
pub fn run() -> u8 {
    let errors = rules_files_errors(&read_index, &list_index, &list_index(".claude/rules/*.md"));
    for error in &errors {
        println!("::error::{error}");
    }
    u8::from(!errors.is_empty())
}

/// Git's output for `args` in `dir`, or `None` when it fails.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(dir).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const AGENTS: &str =
        "# AGENTS.md\n\nFollow [WORKFLOW.md](WORKFLOW.md).\n\n## Git workflow\n\n## No backward compatibility\n";
    const RULE: &str = "---\npaths:\n  - \"crates/**\"\n---\n# Rules\n";

    fn ok() -> HashMap<&'static str, String> {
        HashMap::from([
            ("WORKFLOW.md", "a".repeat(100)),
            ("AGENTS.md", AGENTS.to_string()),
            ("r.md", RULE.to_string()),
        ])
    }

    /// The errors for `files`, with every glob matching a file but `nope/**`.
    fn errors(files: &HashMap<&'static str, String>) -> Vec<String> {
        let read = |path: &str| files.get(path).cloned();
        let list = |glob: &str| {
            if glob == "nope/**" {
                vec![]
            } else {
                vec!["x".to_string()]
            }
        };
        rules_files_errors(&read, &list, &["r.md".to_string()])
    }

    /// The one error `files` gives.
    fn one_error(files: &HashMap<&'static str, String>) -> String {
        let all = errors(files);
        assert_eq!(all.len(), 1, "{all:?}");
        all[0].clone()
    }

    fn with(name: &'static str, text: impl Into<String>) -> HashMap<&'static str, String> {
        let mut files = ok();
        files.insert(name, text.into());
        files
    }

    #[test]
    fn accepts_files_within_their_limits() {
        assert_eq!(errors(&ok()), Vec::<String>::new());
    }

    #[test]
    fn accepts_files_exactly_at_their_limits() {
        let mut files = with("WORKFLOW.md", "a".repeat(5000));
        files.insert("AGENTS.md", format!("{AGENTS}{}", "a".repeat(9000 - AGENTS.len())));
        assert_eq!(errors(&files), Vec::<String>::new());
    }

    #[test]
    fn workflow_md_is_missing() {
        let mut files = ok();
        files.remove("WORKFLOW.md");
        assert!(one_error(&files).contains("WORKFLOW.md is missing"));
    }

    #[test]
    fn workflow_md_is_over_its_limit() {
        let error = one_error(&with("WORKFLOW.md", "a".repeat(5001)));
        assert!(error.contains("WORKFLOW.md is 5001 bytes, over 5000"), "{error}");
    }

    #[test]
    fn agents_md_is_over_its_limit() {
        let text = format!("{AGENTS}{}", "a".repeat(9001 - AGENTS.len()));
        let error = one_error(&with("AGENTS.md", text));
        assert!(error.contains("AGENTS.md is 9001 bytes, over 9000"), "{error}");
    }

    #[test]
    fn counts_bytes_not_characters() {
        let error = one_error(&with("WORKFLOW.md", "é".repeat(2501)));
        assert!(error.contains("5002 bytes"), "{error}");
    }

    #[test]
    fn workflow_md_names_the_project() {
        for word in ["Grace", "KARATE", "gpui", "hpagniez"] {
            let error = one_error(&with("WORKFLOW.md", format!("Ask {word} first.")));
            assert!(
                error.contains(&format!("WORKFLOW.md must stay general: it names \"{word}\"")),
                "{error}"
            );
        }
    }

    #[test]
    fn agents_md_points_to_workflow_md() {
        let text = AGENTS.replace(
            "Follow [WORKFLOW.md](WORKFLOW.md).",
            "Grace runs Karate.\n\nSee WORKFLOW.md later.",
        );
        let error = one_error(&with("AGENTS.md", text));
        assert!(
            error.contains("AGENTS.md must start by pointing to WORKFLOW.md"),
            "{error}"
        );
    }

    #[test]
    fn agents_md_keeps_the_linked_headings() {
        for heading in ["## Git workflow", "## No backward compatibility"] {
            let error = one_error(&with("AGENTS.md", AGENTS.replace(heading, "## Other")));
            assert!(
                error.contains(&format!("AGENTS.md has no \"{heading}\" heading")),
                "{error}"
            );
        }
    }

    #[test]
    fn a_rules_file_without_paths() {
        let error = one_error(&with("r.md", "# Rules\n"));
        assert!(error.contains("r.md has no paths: list"), "{error}");
    }

    #[test]
    fn a_glob_that_matches_no_tracked_file() {
        let error = one_error(&with("r.md", RULE.replace("crates/**", "nope/**")));
        assert!(error.contains("r.md: \"nope/**\" matches no tracked file"), "{error}");
    }

    #[test]
    fn a_glob_with_braces_or_a_bang() {
        for glob in ["src/**/*.{ts,tsx}", "!src/x"] {
            let error = one_error(&with("r.md", RULE.replace("crates/**", glob)));
            assert!(error.contains(&format!("r.md: \"{glob}\" uses braces or !")), "{error}");
        }
    }

    #[test]
    fn rule_paths_reads_quoted_and_bare_items() {
        assert_eq!(
            rule_paths("---\npaths:\n  - \"a/**\"\n  - b/*\n---\ntext\n"),
            ["a/**", "b/*"]
        );
    }

    #[test]
    fn rule_paths_ignores_paths_after_the_frontmatter() {
        assert_eq!(
            rule_paths("---\nname: x\n---\npaths:\n  - a/**\n"),
            Vec::<String>::new()
        );
        assert_eq!(rule_paths("paths:\n  - a/**\n"), Vec::<String>::new());
    }

    #[test]
    fn matching_rules_names_the_files_whose_globs_list_it() {
        let files = HashMap::from([
            ("a.md", "---\npaths:\n  - \"src/**\"\n  - \"e2e/**\"\n---\n"),
            ("b.md", "---\npaths:\n  - \"crates/**\"\n---\n"),
        ]);
        let read = |path: &str| files.get(path).map(|text| text.to_string());
        let list = |glob: &str| match glob {
            "src/**" => vec!["src/a.ts".to_string(), "src/b.ts".to_string()],
            "crates/**" => vec!["crates/x.rs".to_string()],
            _ => vec![],
        };
        let rules = ["a.md".to_string(), "b.md".to_string()];
        assert_eq!(matching_rules("src/b.ts", &read, &list, &rules), ["a.md"]);
        assert_eq!(matching_rules("docs/x.md", &read, &list, &rules), Vec::<String>::new());
    }

    #[test]
    fn the_repository_passes() {
        let rules = list_index(".claude/rules/*.md");
        assert_eq!(rules.len(), 6, "{rules:?}");
        assert_eq!(
            rules_files_errors(&read_index, &list_index, &rules),
            Vec::<String>::new()
        );
    }

    /// Each area's rules load with its files and with no other's, and a rule that binds both apps loads on both sides.
    #[test]
    fn rules_load_by_area() {
        let rules = list_index(".claude/rules/*.md");
        // one git call per glob, not one per glob and sample file
        let listed: HashMap<String, Vec<String>> = rules
            .iter()
            .flat_map(|rule| rule_paths(&read_index(rule).unwrap()))
            .map(|glob| (glob.clone(), list_index(&glob)))
            .collect();
        let list_index = |glob: &str| listed[glob].clone();
        for (file, expected) in [
            ("crates/grace/src/app.rs", vec!["gpui.md"]),
            ("crates/grace-core/src/karate/events.rs", vec!["gpui.md", "karate.md"]),
            ("crates/grace/src/theme.rs", vec!["both-apps.md", "gpui.md"]),
            ("crates/xtask/src/rules.rs", vec!["ci.md"]),
            ("src/renderer/App.tsx", vec!["both-apps.md", "electron.md"]),
            ("src/index.css", vec!["both-apps.md", "electron.md"]),
            ("src/main/karate/http.ts", vec!["electron.md", "karate.md"]),
            (".github/workflows/checks.yml", vec!["ci.md"]),
            ("docs/features.md", vec!["docs.md"]),
            // a Karate bump starts at the pin: the fixtures rule must load there
            ("scripts/vendor.json", vec!["ci.md", "karate.md"]),
            ("README.md", vec![]),
        ] {
            let found = matching_rules(file, &read_index, &list_index, &rules);
            let names: Vec<&str> = found
                .iter()
                .map(|rule| rule.trim_start_matches(".claude/rules/"))
                .collect();
            assert_eq!(names, expected, "{file}");
        }
    }

    /// A rule reaches the agent who needs it: some rules file loaded with `file` holds `text`.
    fn loads_with(file: &str, text: &str) -> bool {
        let rules = list_index(".claude/rules/*.md");
        matching_rules(file, &read_index, &list_index, &rules)
            .iter()
            .any(|rule| read_index(rule).unwrap().contains(text))
    }

    #[test]
    fn rules_reach_the_files_they_bind() {
        for (file, text) in [
            // the script crate is Rust too, and a `crates/` PR
            (
                "crates/xtask/src/rules.rs",
                "- Before pushing Rust: `cargo fmt --all --check`",
            ),
            (
                "crates/xtask/src/rules.rs",
                "- A PR that changes `crates/` gets a Windows run by hand",
            ),
            (
                "crates/grace/src/app.rs",
                "- Before pushing Rust: `cargo fmt --all --check`",
            ),
            // a dependency is added in a manifest, not in a doc
            ("package.json", "A PR that adds a dependency states its size"),
            ("Cargo.toml", "A PR that adds a dependency states its size"),
            // a screenshot run on the owner's machine needs that file's rules
            ("src/renderer/App.tsx", "(../../docs/process/owner-machine.md)"),
            ("crates/grace/src/app.rs", "(../../docs/process/owner-machine.md)"),
        ] {
            assert!(loads_with(file, text), "{file} does not load: {text}");
        }
    }

    /// What binds every code PR is named in a file loaded at the start, whatever the PR touches.
    #[test]
    fn agents_md_points_to_the_pr_rules() {
        let agents = read_index("AGENTS.md").unwrap();
        assert!(
            agents.contains("Read it before marking a code PR ready"),
            "AGENTS.md does not point to ci.md"
        );
    }
}
