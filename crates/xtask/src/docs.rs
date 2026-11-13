//! `cargo xtask docs`: the index of each docs folder, written from the frontmatter of its docs
//! (docs/specs/2026-11-13-docs-index.md).
//!
//! An agent reads a folder's `README.md` to learn which docs matter and which are live, without opening them. The
//! index is generated, so it cannot drift: `cargo xtask check-rules` fails when a doc lacks a field or an index is
//! not what this writes.

/// What a doc is, by its folder: it decides the fields the doc needs and how its index is laid out.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Spec,
    Plan,
    Check,
    Research,
    Decision,
}

/// The statuses of a spec or a plan, in the order of the index's groups: what is being built comes first.
pub const STATUSES: [&str; 4] = ["approved", "draft", "built", "superseded"];

/// The longest summary, in characters.
const SUMMARY_LIMIT: usize = 160;

/// The lines of a doc, without their line ending or trailing spaces: a file saved with CRLF reads the same.
fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim_end)
}

/// The raw value of a `name:` line of the frontmatter, the lines between a first line `---` and the next one.
pub fn field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let mut lines = lines(text);
    if lines.next() != Some("---") {
        return None;
    }
    lines
        .take_while(|line| *line != "---")
        .find_map(|line| line.strip_prefix(name)?.strip_prefix(':'))
        .map(str::trim)
}

/// The text of the first `# ` line after the frontmatter.
pub fn title(text: &str) -> Option<&str> {
    let mut lines = lines(text).peekable();
    if lines.peek() == Some(&"---") {
        lines.next();
        lines.find(|line| *line == "---");
    }
    lines.find_map(|line| line.strip_prefix("# ")).map(str::trim)
}

/// The `status:` of the frontmatter.
pub fn status(text: &str) -> Option<&str> {
    field(text, "status")
}

/// The `summary:` of the frontmatter without its quotes, or why it is not one. It is always in double quotes, with
/// no `"` and no `\` inside, so it is valid YAML whatever it says: a bare summary with a colon is not.
pub fn summary(text: &str) -> Result<&str, String> {
    let raw = field(text, "summary").ok_or("no summary: line in its frontmatter")?;
    let inner = raw
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or("its summary must be in double quotes")?;
    if inner.contains(['"', '\\']) {
        return Err("its summary must have no \" or \\ inside".to_string());
    }
    if inner.trim().is_empty() {
        return Err("its summary is empty".to_string());
    }
    let length = inner.chars().count();
    if length > SUMMARY_LIMIT {
        return Err(format!("its summary is {length} characters, over {SUMMARY_LIMIT}"));
    }
    Ok(inner)
}

/// The issue numbers of the `issue:` line, `373` or `[64, 231]`; none when the doc has no such line.
pub fn issues(text: &str) -> Result<Vec<u32>, String> {
    let Some(raw) = field(text, "issue") else {
        return Ok(Vec::new());
    };
    if raw.contains('#') {
        // YAML reads `#373` as a comment, so the field would be empty
        return Err(format!("its issue is \"{raw}\": write the number without #"));
    }
    let list = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(raw);
    list.split(',')
        .map(|number| number.trim().parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("its issue is \"{raw}\": write a number, or a list like [64, 231]"))
}

/// One message per field the doc at `path` lacks or holds wrongly, for its kind.
pub fn entry_errors(kind: Kind, path: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if title(text).is_none() {
        errors.push("no \"# \" title".to_string());
    }
    match (kind, status(text)) {
        (Kind::Spec | Kind::Plan | Kind::Decision, None) => {
            errors.push("no status: line in its frontmatter".to_string());
        }
        (Kind::Spec | Kind::Plan, Some(status)) if !STATUSES.contains(&status) => {
            errors.push(format!(
                "status \"{status}\" is not draft, approved, built or superseded"
            ));
        }
        _ => {}
    }
    if kind != Kind::Decision {
        errors.extend(summary(text).err());
    }
    if matches!(kind, Kind::Spec | Kind::Plan) {
        errors.extend(issues(text).err());
    }
    errors.iter().map(|error| format!("{path}: {error}")).collect()
}

/// The folders that have an index: the folder, the kind of its docs, and the index's heading.
pub const FOLDERS: [(&str, Kind, &str); 5] = [
    ("docs/specs", Kind::Spec, "Specs"),
    ("docs/plans", Kind::Plan, "Plans"),
    ("docs/checks", Kind::Check, "Checks"),
    ("docs/research", Kind::Research, "Research"),
    ("docs/decisions", Kind::Decision, "Decisions"),
];

/// The first line of every index.
const GENERATED: &str =
    "<!-- Generated by `cargo xtask docs` from the frontmatter of the docs in this folder. Do not edit. -->";

/// The path and text of each doc of a folder, newest first: its `*.md` files but the index itself, and for research
/// the `README.md` of each folder directly under it. `list` gives the files a glob matches, `read` a file's text;
/// a listed file that cannot be read (deleted, not yet staged) is left out.
pub fn entries(
    folder: &str,
    kind: Kind,
    read: &dyn Fn(&str) -> Option<String>,
    list: &dyn Fn(&str) -> Vec<String>,
) -> Vec<(String, String)> {
    let mut paths = list(&format!("{folder}/*.md"));
    if kind == Kind::Research {
        paths.extend(list(&format!("{folder}/*/README.md")));
    }
    paths.retain(|path| *path != format!("{folder}/README.md"));
    // every name starts with its date, a decision's with its number
    paths.sort_by(|a, b| b.cmp(a));
    paths
        .into_iter()
        .filter_map(|path| read(&path).map(|text| (path, text)))
        .collect()
}

/// The text of a folder's index: one line per entry, in the order given, under a heading per status for the kinds
/// that have one.
pub fn index(folder: &str, kind: Kind, heading: &str, entries: &[(String, String)]) -> String {
    let line = |(path, text): &(String, String)| {
        let relative = path
            .strip_prefix(folder)
            .map_or(path.as_str(), |rest| rest.trim_start_matches('/'));
        let stamp = relative
            .get(..if kind == Kind::Decision { 4 } else { 10 })
            .unwrap_or(relative);
        let mut line = format!("- {stamp} [{}]({relative})", title(text).unwrap_or(relative));
        for issue in issues(text).unwrap_or_default() {
            line.push_str(&format!(" #{issue}"));
        }
        if let (true, Ok(summary)) = (kind != Kind::Decision, summary(text)) {
            line.push_str(&format!(": {summary}"));
        }
        line
    };
    let statuses: Vec<&str> = match kind {
        Kind::Spec | Kind::Plan => STATUSES.to_vec(),
        Kind::Decision => {
            let mut found: Vec<&str> = entries.iter().filter_map(|(_, text)| status(text)).collect();
            found.sort_by_key(|status| (*status != "accepted", *status));
            found.dedup();
            found
        }
        Kind::Check | Kind::Research => Vec::new(),
    };
    let mut blocks = vec![GENERATED.to_string(), format!("# {heading}")];
    if matches!(kind, Kind::Check | Kind::Research) && !entries.is_empty() {
        blocks.push(entries.iter().map(line).collect::<Vec<_>>().join("\n"));
    }
    for group in statuses {
        let lines: Vec<String> = entries
            .iter()
            .filter(|(_, text)| status(text) == Some(group))
            .map(line)
            .collect();
        if !lines.is_empty() {
            let mut letters = group.chars();
            let first = letters.next().map(|letter| letter.to_uppercase().to_string());
            blocks.push(format!("## {}{}", first.unwrap_or_default(), letters.as_str()));
            blocks.push(lines.join("\n"));
        }
    }
    blocks.join("\n\n") + "\n"
}

/// Every entry's errors, folder by folder.
fn entries_errors(read: &dyn Fn(&str) -> Option<String>, list: &dyn Fn(&str) -> Vec<String>) -> Vec<String> {
    let mut errors = Vec::new();
    for (folder, kind, _) in FOLDERS {
        for (path, text) in entries(folder, kind, read, list) {
            errors.extend(entry_errors(kind, &path, &text));
        }
    }
    errors
}

/// The path and text of each folder's index.
fn indexes(read: &dyn Fn(&str) -> Option<String>, list: &dyn Fn(&str) -> Vec<String>) -> Vec<(String, String)> {
    FOLDERS
        .iter()
        .map(|(folder, kind, heading)| {
            let entries = entries(folder, *kind, read, list);
            (format!("{folder}/README.md"), index(folder, *kind, heading, &entries))
        })
        .collect()
}

/// Writes the indexes from the working tree's docs, new ones included, and gives the exit code. A doc that lacks a
/// field stops it before it writes anything: an index must never be built on a guess.
pub fn run() -> u8 {
    let root = crate::rules::root();
    let read = |path: &str| std::fs::read_to_string(root.join(path)).ok();
    let list = |glob: &str| -> Vec<String> {
        let tracked_and_new = ["ls-files", "--cached", "--others", "--exclude-standard", "--"];
        let glob = format!(":(glob){glob}");
        crate::rules::git(root, &[&tracked_and_new[..], &[glob.as_str()]].concat())
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    };
    let errors = entries_errors(&read, &list);
    for error in &errors {
        println!("::error::{error}");
    }
    if !errors.is_empty() {
        return 1;
    }
    for (path, text) in indexes(&read, &list) {
        if let Err(error) = std::fs::write(root.join(&path), text) {
            println!("::error::could not write {path}: {error}");
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const SPEC: &str =
        "---\nstatus: approved\nissue: 373\nsummary: \"Finds the doc in one read\"\n---\n\n# Docs index\n\nText.\n";

    #[test]
    fn reads_the_fields_and_the_title() {
        assert_eq!(status(SPEC), Some("approved"));
        assert_eq!(summary(SPEC), Ok("Finds the doc in one read"));
        assert_eq!(issues(SPEC), Ok(vec![373]));
        assert_eq!(title(SPEC), Some("Docs index"));
        assert_eq!(entry_errors(Kind::Spec, "docs/specs/a.md", SPEC), Vec::<String>::new());
    }

    #[test]
    fn reads_a_list_of_issues() {
        assert_eq!(issues(&SPEC.replace("373", "[64, 231]")), Ok(vec![64, 231]));
    }

    #[test]
    fn reads_a_doc_saved_with_crlf() {
        let text = SPEC.replace('\n', "\r\n");
        assert_eq!(status(&text), Some("approved"));
        assert_eq!(title(&text), Some("Docs index"));
    }

    #[test]
    fn a_doc_without_frontmatter_has_no_field() {
        assert_eq!(status("# Title\n\nstatus: built\n"), None);
    }

    #[test]
    fn a_title_inside_the_frontmatter_is_not_the_title() {
        assert_eq!(title("---\n# comment\nstatus: draft\n---\n# Real\n"), Some("Real"));
    }

    #[test]
    fn a_spec_needs_a_known_status() {
        let errors = entry_errors(Kind::Spec, "docs/specs/a.md", &SPEC.replace("approved", "done"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("docs/specs/a.md: status \"done\" is not draft, approved, built or superseded"));
        let errors = entry_errors(Kind::Plan, "docs/plans/a.md", &SPEC.replace("status: approved\n", ""));
        assert!(
            errors[0].contains("docs/plans/a.md: no status: line in its frontmatter"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_check_needs_a_summary_and_no_status() {
        let text = "---\nsummary: \"VL-29 passes\"\n---\n# Check\n";
        assert_eq!(
            entry_errors(Kind::Check, "docs/checks/a.md", text),
            Vec::<String>::new()
        );
        let errors = entry_errors(Kind::Research, "docs/research/a.md", "# Note\n");
        assert!(
            errors[0].contains("docs/research/a.md: no summary: line in its frontmatter"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_summary_is_quoted_short_and_plain() {
        for (value, reason) in [
            ("Finds: the doc", "in double quotes"),
            ("\"Says \"hello\"\"", "no \" or \\ inside"),
            ("\"a\\b\"", "no \" or \\ inside"),
        ] {
            let text = format!("---\nsummary: {value}\n---\n# T\n");
            let error = summary(&text).unwrap_err();
            assert!(error.contains(reason), "{error}");
        }
        let long = format!("---\nsummary: \"{}\"\n---\n# T\n", "é".repeat(161));
        assert!(summary(&long).unwrap_err().contains("161 characters, over 160"));
        let exact = format!("---\nsummary: \"{}\"\n---\n# T\n", "é".repeat(160));
        assert!(summary(&exact).is_ok());
    }

    #[test]
    fn an_issue_is_a_number_without_a_hash() {
        let error = issues(&SPEC.replace("373", "#373")).unwrap_err();
        assert!(error.contains("write the number without #"), "{error}");
        assert!(issues(&SPEC.replace("373", "soon")).is_err());
    }

    #[test]
    fn a_decision_needs_only_its_status_and_title() {
        let text = "---\nstatus: accepted\ndate: 2026-10-24\n---\n\n# Keep plain Markdown\n";
        assert_eq!(
            entry_errors(Kind::Decision, "docs/decisions/0002-x.md", text),
            Vec::<String>::new()
        );
        let errors = entry_errors(
            Kind::Decision,
            "docs/decisions/0002-x.md",
            "---\nstatus: accepted\n---\n",
        );
        assert!(
            errors[0].contains("docs/decisions/0002-x.md: no \"# \" title"),
            "{errors:?}"
        );
    }

    const COMMENT: &str =
        "<!-- Generated by `cargo xtask docs` from the frontmatter of the docs in this folder. Do not edit. -->";

    fn files() -> HashMap<&'static str, String> {
        let built = SPEC
            .replace("approved", "built")
            .replace("373", "[64, 231]")
            .replace("Docs index", "Setup");
        HashMap::from([
            ("docs/specs/2026-11-13-b.md", SPEC.to_string()),
            ("docs/specs/2026-11-02-a.md", built),
            ("docs/specs/README.md", String::new()),
            (
                "docs/research/2026-11-07-p/README.md",
                "---\nsummary: \"Go\"\n---\n# Prototype\n".to_string(),
            ),
            ("docs/research/2026-11-07-p/notes/README.md", "# Deeper\n".to_string()),
            (
                "docs/decisions/0002-plain.md",
                "---\nstatus: accepted\n---\n# Keep plain Markdown\n".to_string(),
            ),
        ])
    }

    /// The paths of `files` a glob with one `*` matches, as git lists them: sorted, and `*` stops at a `/`.
    fn list_in(files: &HashMap<&'static str, String>, glob: &str) -> Vec<String> {
        let (before, after) = glob.split_once('*').unwrap();
        let matches = |path: &&&str| {
            path.strip_prefix(before)
                .and_then(|rest| rest.strip_suffix(after))
                .is_some_and(|middle| !middle.contains('/'))
        };
        let mut paths: Vec<String> = files.keys().filter(matches).map(|path| path.to_string()).collect();
        paths.sort();
        paths
    }

    /// The entries of a folder of `files`.
    fn entries_in(files: &HashMap<&'static str, String>, folder: &str, kind: Kind) -> Vec<(String, String)> {
        entries(folder, kind, &|path| files.get(path).cloned(), &|glob| {
            list_in(files, glob)
        })
    }

    fn paths(entries: &[(String, String)]) -> Vec<&str> {
        entries.iter().map(|(path, _)| path.as_str()).collect()
    }

    #[test]
    fn lists_entries_newest_first_without_the_index() {
        assert_eq!(
            paths(&entries_in(&files(), "docs/specs", Kind::Spec)),
            ["docs/specs/2026-11-13-b.md", "docs/specs/2026-11-02-a.md"]
        );
    }

    #[test]
    fn a_research_folder_is_listed_by_its_readme() {
        assert_eq!(
            paths(&entries_in(&files(), "docs/research", Kind::Research)),
            ["docs/research/2026-11-07-p/README.md"]
        );
    }

    #[test]
    fn skips_a_listed_file_that_cannot_be_read() {
        let files = files();
        let list = |glob: &str| {
            let mut paths = list_in(&files, glob);
            paths.push("docs/specs/2026-11-14-gone.md".to_string());
            paths
        };
        let entries = entries("docs/specs", Kind::Spec, &|path| files.get(path).cloned(), &list);
        assert_eq!(
            paths(&entries),
            ["docs/specs/2026-11-13-b.md", "docs/specs/2026-11-02-a.md"]
        );
    }

    #[test]
    fn writes_the_specs_index() {
        let entries = entries_in(&files(), "docs/specs", Kind::Spec);
        let expected = format!(
            "{COMMENT}\n\n# Specs\n\n## Approved\n\n\
             - 2026-11-13 [Docs index](2026-11-13-b.md) #373: Finds the doc in one read\n\n\
             ## Built\n\n\
             - 2026-11-02 [Setup](2026-11-02-a.md) #64 #231: Finds the doc in one read\n"
        );
        assert_eq!(index("docs/specs", Kind::Spec, "Specs", &entries), expected);
    }

    #[test]
    fn writes_the_research_and_decisions_indexes() {
        let files = files();
        let research = entries_in(&files, "docs/research", Kind::Research);
        assert_eq!(
            index("docs/research", Kind::Research, "Research", &research),
            format!("{COMMENT}\n\n# Research\n\n- 2026-11-07 [Prototype](2026-11-07-p/README.md): Go\n")
        );
        let decisions = entries_in(&files, "docs/decisions", Kind::Decision);
        assert_eq!(
            index("docs/decisions", Kind::Decision, "Decisions", &decisions),
            format!("{COMMENT}\n\n# Decisions\n\n## Accepted\n\n- 0002 [Keep plain Markdown](0002-plain.md)\n")
        );
    }

    #[test]
    fn accepted_decisions_come_first() {
        let record = |status: &str| format!("---\nstatus: {status}\n---\n# T\n");
        let entries = [
            ("docs/decisions/0003-c.md".to_string(), record("accepted")),
            ("docs/decisions/0002-b.md".to_string(), record("superseded by ADR 0003")),
            ("docs/decisions/0001-a.md".to_string(), record("accepted")),
        ];
        let text = index("docs/decisions", Kind::Decision, "Decisions", &entries);
        let accepted = text
            .find("## Accepted\n\n- 0003 [T](0003-c.md)\n- 0001 [T](0001-a.md)\n")
            .unwrap();
        let superseded = text
            .find("## Superseded by ADR 0003\n\n- 0002 [T](0002-b.md)\n")
            .unwrap();
        assert!(accepted < superseded, "{text}");
    }

    #[test]
    fn the_same_docs_with_crlf_give_the_same_index() {
        let entries = entries_in(&files(), "docs/specs", Kind::Spec);
        let crlf: Vec<(String, String)> = entries
            .iter()
            .map(|(path, text)| (path.clone(), text.replace('\n', "\r\n")))
            .collect();
        assert_eq!(
            index("docs/specs", Kind::Spec, "Specs", &crlf),
            index("docs/specs", Kind::Spec, "Specs", &entries)
        );
    }
}
