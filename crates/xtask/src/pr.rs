//! `cargo xtask check-pr`: a pull request follows the way of working (WORKFLOW.md, AGENTS.md).
//!
//! Commit headers (ADR 0003), git flow branch targets, a release record's `## Public docs`, the tier the PR names and
//! what follows from it, tests with behaviour, the language of a new script, and no trace of an agent's session.
//! The two checks that judge the bench's JSON stay in `scripts/check-pr.mjs` until the bench is ported.

/// The tier a PR names in its body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tier {
    Small,
    Feature,
    Project,
}

/// One file a PR changes: the lines it adds and removes, and whether the PR adds the file.
pub struct Change {
    pub path: String,
    pub lines: usize,
    pub added: bool,
}

const TYPES: [&str; 10] = [
    "feat", "fix", "docs", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

/// The type and the subject of a Conventional Commits header: `<type>(<scope>)<!>: <subject>`.
fn header_parts(header: &str) -> Option<(&str, &str)> {
    let kind = TYPES.iter().find(|kind| {
        header
            .strip_prefix(**kind)
            .is_some_and(|rest| rest.starts_with(['(', '!', ':']))
    })?;
    let mut rest = &header[kind.len()..];
    if let Some(scoped) = rest.strip_prefix('(') {
        let (scope, after) = scoped.split_once(')')?;
        let lower = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        if scope.is_empty() || !scope.chars().all(lower) {
            return None;
        }
        rest = after;
    }
    let subject = rest.strip_prefix('!').unwrap_or(rest).strip_prefix(": ")?;
    (!subject.starts_with(char::is_whitespace) && !subject.is_empty()).then_some((*kind, subject))
}

/// `None` when `header` (a commit's first line) follows the rules, else the reason.
pub fn commit_error(header: &str) -> Option<String> {
    let Some((_, subject)) = header_parts(header) else {
        return Some("not `<type>(<scope>)<!>: <subject>` with a known type and a lower-case scope".to_string());
    };
    if subject.starts_with(|c: char| c.is_ascii_uppercase()) {
        return Some("subject starts with an upper-case letter".to_string());
    }
    if subject.ends_with('.') {
        return Some("subject ends with a period".to_string());
    }
    // counted as GitHub and the JavaScript check did: UTF-16 units
    let length = header.encode_utf16().count();
    (length > 72).then(|| format!("header is {length} characters, max 72"))
}

/// A release or hotfix branch named after its tag (ADR 0009), a release candidate (ADR 0015): `release/vX.Y.Z-rc.N`.
pub fn versioned(head: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit());
    let Some(version) = head.strip_prefix("release/v").or_else(|| head.strip_prefix("hotfix/v")) else {
        return false;
    };
    let Some((numbers, candidate)) = version.split_once("-rc.") else {
        return false;
    };
    numbers.split('.').count() == 3 && numbers.split('.').all(digits) && digits(candidate)
}

/// `None` when a PR from `head` may target `base` under git flow, else the reason.
pub fn branch_error(head: &str, base: &str) -> Option<String> {
    let kind = head.split('/').next().unwrap_or_default();
    if (kind == "release" || kind == "hotfix") && !versioned(head) {
        return Some(format!("{head} must be named {kind}/vX.Y.Z-rc.N, after its tag"));
    }
    let allowed: &[&str] = match kind {
        "feature" | "dependabot" => &["develop"],
        "release" | "hotfix" => &["main", "develop"],
        _ => &[],
    };
    if head.contains('/') && allowed.contains(&base) {
        return None;
    }
    Some(format!(
        "{head} may not target {base}: feature/* and dependabot/* go to develop, release/vX.Y.Z-rc.N and hotfix/vX.Y.Z-rc.N to main or develop"
    ))
}

/// `None` unless a release or hotfix PR into main lacks its packaged-app check record or the record's
/// `## Public docs` section (docs/process/release.md). `files` are the paths under docs/checks.
pub fn public_docs_error(
    head: &str,
    base: &str,
    files: &[String],
    read: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    if base != "main" || !versioned(head) {
        return None;
    }
    let version = head.split_once("/v")?.1;
    let Some(record) = files
        .iter()
        .find(|file| file.ends_with(&format!("-{version}-packaged.md")))
    else {
        return Some(format!(
            "{head} has no docs/checks/<date>-<release|hotfix>-{version}-packaged.md"
        ));
    };
    let text = read(record).unwrap_or_default();
    if text.lines().any(|line| line.trim_end() == "## Public docs") {
        return None;
    }
    Some(format!(
        "{record} has no \"## Public docs\" section: check README.md and its screenshots against {version} (docs/process/release.md)"
    ))
}

/// The tier a PR's body names, on a line of its own: `Tier: small`, `Tier: feature` or `Tier: project`.
pub fn tier(body: &str) -> Option<Tier> {
    body.lines().find_map(|line| match line.trim() {
        "Tier: small" => Some(Tier::Small),
        "Tier: feature" => Some(Tier::Feature),
        "Tier: project" => Some(Tier::Project),
        _ => None,
    })
}

/// A bot's branch or a release branch: it names no tier, and the checks that follow from one do not apply.
pub fn exempt(head: &str) -> bool {
    ["dependabot/", "feature/bump-", "release/", "hotfix/"]
        .iter()
        .any(|prefix| head.starts_with(prefix))
}

/// `None` when the PR names its tier or needs none, else the reason.
pub fn tier_error(head: &str, body: &str) -> Option<String> {
    (!exempt(head) && tier(body).is_none()).then(|| {
        "the PR body has no \"Tier: small\", \"Tier: feature\" or \"Tier: project\" line (WORKFLOW.md)".to_string()
    })
}

/// What a `label:` line of the body says, when it has one.
fn body_line<'a>(body: &'a str, label: &str) -> Option<&'a str> {
    body.lines()
        .find_map(|line| line.trim().strip_prefix(label))
        .map(str::trim)
}

/// A Feature or Project PR links its issue and names a spec and a plan that exist at its head (`exists`). A PR
/// that changes only docs (`docs_only`) may come before its plan.
pub fn papers_errors(tier: Tier, body: &str, docs_only: bool, exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    let name = match tier {
        Tier::Small => return Vec::new(),
        Tier::Feature => "feature",
        Tier::Project => "project",
    };
    let mut errors = Vec::new();
    let numbered = |line: &str, words: &str| {
        line.split(words)
            .skip(1)
            .any(|after| after.starts_with(|c: char| c.is_ascii_digit()))
    };
    if !body
        .lines()
        .any(|line| numbered(line, "Closes #") || numbered(line, "Part of #"))
    {
        errors.push(format!("a {name} PR links its issue: \"Closes #N\" or \"Part of #N\""));
    }
    for (label, optional) in [("Spec:", false), ("Plan:", docs_only)] {
        match body_line(body, label) {
            Some(path) if !exists(path) => errors.push(format!("{label} {path} is not a file of this PR's head")),
            None if !optional => errors.push(format!("a {name} PR has no \"{label} <path>\" line")),
            _ => {}
        }
    }
    errors
}

/// A project is built slice by slice, each in its own session on the model its plan names (WORKFLOW.md): the plan
/// at `path` must hold a `## Slices` section.
pub fn slices_error(path: &str, plan: &str) -> Option<String> {
    (!plan.lines().any(|line| line.trim_end() == "## Slices")).then(|| {
        format!("Plan: {path} has no \"## Slices\" section: a project's plan names its slices, one PR each, with the model that builds it (WORKFLOW.md)")
    })
}

/// A PR body or a commit message says nothing of the agent but its attribution lines (AGENTS.md): no claude.ai
/// session or project link, no `Claude-Session:` trailer, no "Requested by" line.
pub fn trace_errors(texts: &[String]) -> Vec<String> {
    let trace = |line: &&str| {
        line.contains("claude.ai/") || line.starts_with("Claude-Session:") || line.starts_with("Requested by")
    };
    texts
        .iter()
        .flat_map(|text| text.lines().map(str::trim).filter(trace))
        .map(|line| format!("\"{line}\" says more about the agent than its attribution lines (AGENTS.md)"))
        .collect()
}

/// A new script is Rust unless the body says why not (WORKFLOW.md). App code, tests, the bench and docs are not scripts.
pub fn script_language_error(added: &[String], body: &str) -> Option<String> {
    let script = |path: &&String| {
        let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
        ["py", "mjs", "js", "sh", "ps1"].contains(&extension)
            && !["src/", "e2e/", "test/", "bench/", "docs/"]
                .iter()
                .any(|folder| path.starts_with(folder))
    };
    let path = added.iter().find(script)?;
    body_line(body, "Script-language:").is_none().then(|| {
        format!("{path}: a new script is Rust (cargo xtask) unless the body says why on a \"Script-language:\" line")
    })
}

/// The dependency names of a `package.json` or a `Cargo.toml`.
// shortcut: reads the files as npm and cargo write them (one entry a line, tables at column 0), not as JSON or
// TOML; upgrade to a parser if a manifest is ever formatted otherwise
pub fn dependencies(path: &str, text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut inside = false;
    if path.ends_with("package.json") {
        for line in text.lines().map(str::trim) {
            let key = line
                .strip_prefix('"')
                .and_then(|rest| rest.split_once('"'))
                .map(|(key, _)| key);
            if line.starts_with('}') {
                inside = false;
            } else if line.ends_with('{') {
                let tables = [
                    "dependencies",
                    "devDependencies",
                    "optionalDependencies",
                    "peerDependencies",
                ];
                inside = key.is_some_and(|key| tables.contains(&key));
            } else if inside && let Some(key) = key {
                names.push(key.to_string());
            }
        }
        return names;
    }
    for line in text.lines() {
        if let Some(table) = line
            .strip_prefix('[')
            .and_then(|rest| rest.split_once(']'))
            .map(|(table, _)| table)
        {
            inside = table.ends_with("dependencies");
            if let Some((_, name)) = table.rsplit_once("dependencies.") {
                names.push(name.to_string());
            }
        } else if inside && line.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
            names.push(line.split(['=', '.', ' ']).next().unwrap_or_default().to_string());
        }
    }
    names
}

/// Whether a file's lines count towards a Small PR's size: not docs, lock files or fixtures.
pub fn counted(path: &str) -> bool {
    !(path.starts_with("docs/")
        || path.ends_with(".md")
        || path == "Cargo.lock"
        || path == "package-lock.json"
        || path.starts_with("test/fixtures/"))
}

/// What makes a PR more than Small: too many lines, a new dependency, a new decision record.
pub fn small_errors(changes: &[Change], new_dependencies: &[String]) -> Vec<String> {
    let mut errors = Vec::new();
    let lines: usize = changes
        .iter()
        .filter(|change| counted(&change.path))
        .map(|change| change.lines)
        .sum();
    if lines > 400 {
        errors.push(format!(
            "a Small PR changes {lines} lines outside docs, lock files and fixtures, over 400: step up a tier"
        ));
    }
    if !new_dependencies.is_empty() {
        errors.push(format!(
            "a Small PR adds a dependency ({}): step up a tier",
            new_dependencies.join(", ")
        ));
    }
    if changes
        .iter()
        .any(|change| change.added && change.path.starts_with("docs/decisions/"))
    {
        errors.push("a Small PR adds a decision record: step up a tier".to_string());
    }
    errors
}

/// A file that holds only tests.
pub fn is_test_path(path: &str) -> bool {
    [".test.ts", ".test.tsx", ".spec.ts"]
        .iter()
        .any(|end| path.ends_with(end))
        || path.starts_with("e2e/")
        || (path.starts_with("crates/") && path.contains("/tests/"))
}

/// A `feat` or `fix` that changes app code comes with a test (WORKFLOW.md, the floor).
// shortcut: a Rust test is seen by an added #[test] or assert line, since unit tests share their file; a fix that
// only reorders a test passes unseen, upgrade if that is ever abused
pub fn tests_error(headers: &[String], changes: &[Change], added_rust_lines: &[String]) -> Option<String> {
    let behaviour = headers
        .iter()
        .any(|header| matches!(header_parts(header), Some(("feat" | "fix", _))));
    let code = |change: &Change| {
        (change.path.starts_with("src/") || change.path.starts_with("crates/")) && !is_test_path(&change.path)
    };
    let tested = changes.iter().any(|change| is_test_path(&change.path))
        || added_rust_lines
            .iter()
            .any(|line| line.contains("#[test]") || line.contains("#[gpui::test]") || line.contains("assert"));
    (behaviour && changes.iter().any(code) && !tested).then(|| {
        "a feat or fix commit changes src/ or crates/ and no test: add the test that fails without the change (WORKFLOW.md)"
            .to_string()
    })
}

/// What a run knows of the PR. `body` is `None` outside a `pull_request` event, where there is none to read.
pub struct Env {
    pub head_ref: String,
    pub base_ref: String,
    pub base_sha: String,
    pub head_sha: String,
    pub body: Option<String>,
}

/// Every reason the PR does not follow the rules. `git` gives git's output for its arguments, `None` when it fails.
pub fn errors(env: &Env, git: &dyn Fn(&[&str]) -> Option<String>) -> Vec<String> {
    let lines = |args: &[&str]| -> Vec<String> { git(args).unwrap_or_default().lines().map(String::from).collect() };
    let show = |sha: &str, path: &str| git(&["show", &format!("{sha}:{path}")]);
    let (head, base) = (env.head_sha.as_str(), env.base_sha.as_str());
    let mut errors = Vec::new();

    errors.extend(branch_error(&env.head_ref, &env.base_ref));
    let records = lines(&["ls-tree", "--name-only", head, "docs/checks/"]);
    errors.extend(public_docs_error(&env.head_ref, &env.base_ref, &records, &|path| {
        show(head, path)
    }));

    // commits already on develop were checked in their own PR; a release PR into main carries them all
    let Some(log) = git(&[
        "log",
        "--no-merges",
        "--format=%B%x00",
        head,
        "--not",
        base,
        "origin/develop",
    ]) else {
        errors.push("could not list the PR's commits: is origin/develop fetched?".to_string());
        return errors;
    };
    let messages: Vec<String> = log.split('\0').map(|message| message.trim().to_string()).collect();
    let messages: Vec<String> = messages.into_iter().filter(|message| !message.is_empty()).collect();
    let headers: Vec<String> = messages
        .iter()
        .map(|message| message.lines().next().unwrap_or_default().to_string())
        .collect();
    // Dependabot writes its own commit headers, often over 72 characters
    if !env.head_ref.starts_with("dependabot/") {
        for header in &headers {
            errors.extend(commit_error(header).map(|error| format!("commit \"{header}\": {error}")));
        }
    }
    errors.extend(trace_errors(&messages));

    // a PR changes only what it added to where it left the base, not to the base's tip
    let fork = git(&["merge-base", base, head]).unwrap_or_default().trim().to_string();
    let added = lines(&[
        "diff",
        "--no-color",
        "--name-only",
        "--no-renames",
        "--diff-filter=A",
        &fork,
        head,
    ]);
    let changes: Vec<Change> = lines(&["diff", "--no-color", "--numstat", "--no-renames", &fork, head])
        .iter()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            // a binary file counts no line: git prints "-" for it
            let count = |field: Option<&str>| field.and_then(|field| field.parse::<usize>().ok()).unwrap_or(0);
            let lines = count(fields.next()) + count(fields.next());
            let path = fields.next()?.to_string();
            Some(Change {
                added: added.contains(&path),
                path,
                lines,
            })
        })
        .collect();

    if let Some(body) = &env.body {
        errors.extend(trace_errors(std::slice::from_ref(body)));
    }
    if exempt(&env.head_ref) {
        return errors;
    }
    let rust: Vec<String> = lines(&["diff", "--no-color", "-U0", "--no-renames", &fork, head, "--", "*.rs"])
        .into_iter()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .collect();
    errors.extend(tests_error(&headers, &changes, &rust));
    let Some(body) = &env.body else {
        return errors;
    };
    errors.extend(tier_error(&env.head_ref, body));
    errors.extend(script_language_error(&added, body));
    match tier(body) {
        Some(Tier::Small) => {
            let manifest = |change: &&Change| change.path == "package.json" || change.path.ends_with("Cargo.toml");
            let mut new = Vec::new();
            for change in changes.iter().filter(manifest) {
                let before = dependencies(&change.path, &show(&fork, &change.path).unwrap_or_default());
                let after = dependencies(&change.path, &show(head, &change.path).unwrap_or_default());
                new.extend(after.into_iter().filter(|name| !before.contains(name)));
            }
            errors.extend(small_errors(&changes, &new));
        }
        Some(tier) => {
            let docs_only = changes
                .iter()
                .all(|change| change.path.starts_with("docs/") || change.path.ends_with(".md"));
            let exists = |path: &str| {
                git(&["cat-file", "-t", &format!("{head}:{path}")]).is_some_and(|kind| kind.trim() == "blob")
            };
            errors.extend(papers_errors(tier, body, docs_only, &exists));
            if let (Tier::Project, Some(path)) = (tier, body_line(body, "Plan:"))
                && exists(path)
            {
                errors.extend(slices_error(path, &show(head, path).unwrap_or_default()));
            }
        }
        None => {}
    }
    errors
}

/// The PR to check: what CI's variables say, and without them the branch in hand against `origin/develop`, with no
/// body, so the check also runs on a machine before a push.
pub fn env_from(var: &dyn Fn(&str) -> Option<String>, current: &str) -> Env {
    let or = |name: &str, default: &str| {
        var(name)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| default.to_string())
    };
    Env {
        head_ref: or("HEAD_REF", current),
        base_ref: or("BASE_REF", "develop"),
        base_sha: or("BASE_SHA", "origin/develop"),
        head_sha: or("HEAD_SHA", "HEAD"),
        body: (var("EVENT_NAME").as_deref() == Some("pull_request")).then(|| var("PR_BODY").unwrap_or_default()),
    }
}

/// Checks the PR the environment describes, prints each failure as a CI error, and gives the exit code.
pub fn run() -> u8 {
    let root = crate::rules::root();
    let current = crate::rules::git(root, &["branch", "--show-current"]).unwrap_or_default();
    let env = env_from(&|name| std::env::var(name).ok(), current.trim());
    let errors = errors(&env, &|args| crate::rules::git(root, args));
    for error in &errors {
        println!("::error::{error}");
    }
    u8::from(!errors.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    fn change(path: &str, lines: usize) -> Change {
        Change {
            path: path.to_string(),
            lines,
            added: false,
        }
    }

    #[test]
    fn without_ci_the_branch_in_hand_is_checked() {
        let env = env_from(&|_| None, "feature/x");
        let refs = (
            env.head_ref.as_str(),
            env.base_ref.as_str(),
            env.base_sha.as_str(),
            env.head_sha.as_str(),
        );
        assert_eq!(refs, ("feature/x", "develop", "origin/develop", "HEAD"));
        assert_eq!(env.body, None);
    }

    #[test]
    fn ci_says_which_pr_to_check() {
        let var = |name: &str| {
            let value = match name {
                "HEAD_REF" => "feature/y",
                "BASE_REF" => "main",
                "BASE_SHA" => "abc",
                "HEAD_SHA" => "def",
                "EVENT_NAME" => "pull_request",
                "PR_BODY" => "Tier: small",
                _ => return None,
            };
            Some(value.to_string())
        };
        let env = env_from(&var, "feature/x");
        let refs = (
            env.head_ref.as_str(),
            env.base_ref.as_str(),
            env.base_sha.as_str(),
            env.head_sha.as_str(),
        );
        assert_eq!(refs, ("feature/y", "main", "abc", "def"));
        assert_eq!(env.body.as_deref(), Some("Tier: small"));
        // a run by hand in CI has the refs and no body
        let dispatch = |name: &str| {
            if name == "EVENT_NAME" {
                Some("workflow_dispatch".to_string())
            } else {
                var(name)
            }
        };
        assert_eq!(env_from(&dispatch, "feature/x").body, None);
        // an empty variable is an unset one
        assert_eq!(env_from(&|_| Some(String::new()), "feature/x").head_sha, "HEAD");
    }

    // commit headers, ported from scripts/check-pr.test.ts

    #[test]
    fn commit_error_accepts_good_headers() {
        for header in [
            "docs(agents): add subagent model selection rule (#5)",
            "feat(runner)!: drop java 17",
            "ci: add checks workflow",
        ] {
            assert_eq!(commit_error(header), None, "{header}");
        }
    }

    #[test]
    fn commit_error_rejects_bad_headers() {
        let long = format!("fix: {}", "a".repeat(68));
        for header in [
            "Add thing",
            "feature: x",
            "fix(Runner): x",
            "fix: Add x",
            "fix: add x.",
            "fix:add x",
            long.as_str(),
        ] {
            assert!(commit_error(header).is_some(), "{header}");
        }
    }

    #[test]
    fn commit_error_says_why() {
        assert_eq!(
            commit_error("fix: Add x").unwrap(),
            "subject starts with an upper-case letter"
        );
        assert_eq!(commit_error("fix: add x.").unwrap(), "subject ends with a period");
        assert_eq!(
            commit_error(&format!("fix: {}", "a".repeat(68))).unwrap(),
            "header is 73 characters, max 72"
        );
        assert!(
            commit_error("Add thing")
                .unwrap()
                .starts_with("not `<type>(<scope>)<!>: <subject>`")
        );
    }

    /// The titles scripts/bump-pr.mjs and scripts/bump-cargo.mjs write (their tests pin the same strings).
    #[test]
    fn the_bump_scripts_titles_pass() {
        for header in [
            "build(deps): bump karate from 2.1.4 to 2.1.5",
            "build(deps): bump temurin jre from 21.0.12.1+1 to 21.0.13.1+12",
            "build(deps): bump gpui-pre 0.3.12, gpui kit 0.9.1, rust 1.99.1",
            "build(deps): bump rust 1.100.0",
            "build(deps): update cargo.lock",
            "build(deps): bump gpui-pre 0.3.1200, gpui kit 0.9.1000, rust 1.1000.10",
        ] {
            assert_eq!(commit_error(header), None, "{header}");
        }
    }

    // branch targets, ported

    #[test]
    fn branch_error_accepts_git_flow() {
        for (head, base) in [
            ("feature/ci-cd", "develop"),
            ("dependabot/npm_and_yarn/x", "develop"),
            ("release/v1.0.0-rc.1", "main"),
            ("release/v1.0.0-rc.1", "develop"),
            ("hotfix/v1.0.1-rc.1", "main"),
            ("hotfix/v1.0.1-rc.1", "develop"),
            ("release/v10.20.30-rc.12", "main"),
        ] {
            assert_eq!(branch_error(head, base), None, "{head} into {base}");
        }
    }

    #[test]
    fn branch_error_rejects_the_rest() {
        for (head, base) in [
            ("feature/x", "main"),
            ("develop", "main"),
            ("fix-thing", "develop"),
            ("release/1.0.0-rc.1", "main"),
            ("hotfix/1.0.1-rc.1", "main"),
            ("release/v1.0.0", "main"),
            ("hotfix/v1.0.1", "develop"),
            ("release/v1.0-rc.1", "main"),
            ("release/v1.0.0-rc", "main"),
            ("release/v1.0.0-rc1", "main"),
            ("release/v1.0.0-beta.1", "main"),
            ("hotfix/latest", "develop"),
        ] {
            assert!(branch_error(head, base).is_some(), "{head} into {base}");
        }
    }

    // a release record's Public docs section, ported

    const RECORD: &str = "docs/checks/2026-10-30-release-1.0.0-rc.7-packaged.md";
    const DOCS: &str = "# Check\n\n## Public docs\n\nREADME.md: install steps updated.\n";

    fn public_docs(head: &str, base: &str, files: &[&str], text: &str) -> Option<String> {
        public_docs_error(head, base, &strings(files), &|_| Some(text.to_string()))
    }

    #[test]
    fn public_docs_accepts_a_release_record_with_the_section() {
        assert_eq!(public_docs("release/v1.0.0-rc.7", "main", &[RECORD], DOCS), None);
    }

    #[test]
    fn public_docs_accepts_a_hotfix_record_with_windows_line_endings() {
        let hotfix = "docs/checks/2026-10-31-hotfix-1.0.0-rc.8-packaged.md";
        assert_eq!(
            public_docs("hotfix/v1.0.0-rc.8", "main", &[hotfix], &DOCS.replace('\n', "\r\n")),
            None
        );
    }

    #[test]
    fn public_docs_skips_other_prs() {
        for (head, base) in [
            ("feature/x", "develop"),
            ("release/v1.0.0-rc.7", "develop"),
            ("hotfix/v1.0.0-rc.7", "develop"),
        ] {
            assert_eq!(public_docs(head, base, &[], ""), None, "{head} into {base}");
        }
    }

    #[test]
    fn public_docs_rejects_a_release_without_its_record() {
        let older = "docs/checks/2026-10-29-release-1.0.0-rc.6-packaged.md";
        let error = public_docs("release/v1.0.0-rc.7", "main", &[older], DOCS).unwrap();
        assert!(error.contains("-1.0.0-rc.7-packaged.md"), "{error}");
    }

    #[test]
    fn public_docs_rejects_a_record_without_the_section() {
        let text = "# Check\n\nPublic docs were fine.\n### Public docs\n";
        let error = public_docs("release/v1.0.0-rc.7", "main", &[RECORD], text).unwrap();
        assert!(error.contains("\"## Public docs\""), "{error}");
    }

    // the tier

    #[test]
    fn tier_reads_its_line() {
        assert_eq!(tier("x\nTier: feature\ny"), Some(Tier::Feature));
        assert_eq!(tier("Tier: small\r\n"), Some(Tier::Small));
        assert_eq!(tier("  Tier: project  "), Some(Tier::Project));
        assert_eq!(tier("Tier: Small"), None);
        assert_eq!(tier("Tier: big"), None);
        assert_eq!(tier("The Tier: small one"), None);
    }

    #[test]
    fn tier_error_names_the_line() {
        let error = tier_error("feature/x", "## Summary\n").unwrap();
        assert!(
            error.contains("the PR body has no \"Tier: small\", \"Tier: feature\" or \"Tier: project\" line"),
            "{error}"
        );
        assert_eq!(tier_error("feature/x", "Tier: small"), None);
    }

    #[test]
    fn bot_and_release_branches_need_no_tier() {
        for head in [
            "dependabot/npm_and_yarn/x",
            "feature/bump-cargo-2026-11-17",
            "feature/bump-karate-2.1.5",
            "release/v1.0.0-rc.10",
            "hotfix/v1.0.0-rc.11",
        ] {
            assert_eq!(tier_error(head, ""), None, "{head}");
        }
    }

    #[test]
    fn feature_bump_is_not_every_feature() {
        assert!(tier_error("feature/bumper", "").is_some());
    }

    // the papers of a feature or project PR

    const PAPERS: &str = "Part of #356\nSpec: docs/specs/a.md\nPlan: docs/plans/a.md\n";

    #[test]
    fn a_small_pr_needs_no_papers() {
        assert_eq!(papers_errors(Tier::Small, "", false, &|_| false), Vec::<String>::new());
    }

    #[test]
    fn a_feature_needs_its_issue() {
        let errors = papers_errors(Tier::Feature, "Spec: a.md\nPlan: b.md\n", false, &|_| true);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("a feature PR links its issue: \"Closes #N\" or \"Part of #N\""),
            "{errors:?}"
        );
    }

    #[test]
    fn closes_and_part_of_both_count() {
        assert_eq!(
            papers_errors(Tier::Project, PAPERS, false, &|_| true),
            Vec::<String>::new()
        );
        let closes = PAPERS.replace("Part of #356", "Closes #12");
        assert_eq!(
            papers_errors(Tier::Feature, &closes, false, &|_| true),
            Vec::<String>::new()
        );
        assert_eq!(
            papers_errors(Tier::Feature, &PAPERS.replace("#356", "#N"), false, &|_| true).len(),
            1
        );
    }

    #[test]
    fn a_feature_needs_its_spec_and_plan() {
        let errors = papers_errors(Tier::Feature, "Closes #12\n", false, &|_| true);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("no \"Spec: <path>\" line"), "{errors:?}");
        assert!(errors[1].contains("no \"Plan: <path>\" line"), "{errors:?}");
    }

    #[test]
    fn the_named_files_must_exist() {
        let body = "Closes #12\nSpec: docs/specs/nope.md\nPlan: docs/plans/a.md\n";
        let errors = papers_errors(Tier::Feature, body, false, &|path| path != "docs/specs/nope.md");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("Spec: docs/specs/nope.md is not a file of this PR's head"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_spec_only_pr_needs_no_plan() {
        let body = "Part of #356\nSpec: docs/specs/a.md\n";
        assert_eq!(
            papers_errors(Tier::Project, body, true, &|_| true),
            Vec::<String>::new()
        );
        assert_eq!(papers_errors(Tier::Project, body, false, &|_| true).len(), 1);
    }

    // a project's plan names its slices

    #[test]
    fn a_plan_with_its_slices() {
        let plan = "# Plan\n\n## Slices\n\n| Slice | What | Model |\n|---|---|---|\n| 1 | HTTP | sonnet |\n";
        assert_eq!(slices_error("docs/plans/a.md", plan), None);
        assert_eq!(slices_error("docs/plans/a.md", &plan.replace('\n', "\r\n")), None);
    }

    #[test]
    fn a_plan_without_its_slices() {
        for plan in [
            "# Plan\n\n### Task 1\n",
            "# Plan\n\nThe ## Slices come later.\n",
            "# Plan\n\n### Slices\n",
        ] {
            let error = slices_error("docs/plans/a.md", plan).unwrap();
            assert!(
                error.contains("Plan: docs/plans/a.md has no \"## Slices\" section"),
                "{error}"
            );
        }
    }

    // agent traces

    #[test]
    fn traces_are_refused() {
        for text in [
            "See https://claude.ai/code/session_01",
            "fix: x\n\nClaude-Session: abc",
            "Requested by: someone",
        ] {
            let errors = trace_errors(&strings(&[text]));
            assert_eq!(errors.len(), 1, "{text}");
            assert!(
                errors[0].contains("says more about the agent than its attribution lines"),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn the_attribution_lines_pass() {
        let text = "docs: x\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)\n\nCo-Authored-By: Claude <noreply@anthropic.com>";
        assert_eq!(trace_errors(&strings(&[text])), Vec::<String>::new());
    }

    // script language

    #[test]
    fn a_new_script_needs_its_reason() {
        let error = script_language_error(&strings(&["scripts/x.mjs"]), "Tier: small").unwrap();
        assert!(
            error.contains("scripts/x.mjs: a new script is Rust (cargo xtask) unless the body says why on a \"Script-language:\" line"),
            "{error}"
        );
    }

    #[test]
    fn a_reason_lets_it_in() {
        let body = "Tier: small\nScript-language: runs inside Playwright\n";
        assert_eq!(script_language_error(&strings(&["scripts/x.mjs"]), body), None);
    }

    #[test]
    fn app_code_and_docs_are_not_scripts() {
        let added = strings(&[
            "src/main/a.js",
            "e2e/b.mjs",
            "bench/c.mjs",
            "test/d.js",
            "docs/research/x/e.py",
            "crates/x/f.rs",
            "scripts/g.ts",
        ]);
        assert_eq!(script_language_error(&added, ""), None);
    }

    #[test]
    fn every_script_extension_counts() {
        for path in ["a.py", "tools/b.js", "c.sh", "d.ps1", "scripts/e.mjs"] {
            assert!(script_language_error(&strings(&[path]), "").is_some(), "{path}");
        }
    }

    // dependencies

    #[test]
    fn package_json_dependencies() {
        let text = r#"{
  "name": "grace",
  "scripts": {
    "start": "electron-forge start"
  },
  "dependencies": {
    "solid-js": "2.0.0-rc.19",
    "lucide": "1.64.0"
  },
  "devDependencies": {
    "vitest": "5.0.5"
  },
  "optionalDependencies": {
    "fsevents": "2.3.3"
  },
  "peerDependencies": {
    "react": "19.0.0"
  }
}"#;
        assert_eq!(
            dependencies("package.json", text),
            ["solid-js", "lucide", "vitest", "fsevents", "react"]
        );
    }

    #[test]
    fn cargo_dependencies() {
        let text = r#"[package]
name = "x"
version.workspace = true

[dependencies]
serde_json = "1.0.154"
grace-core.workspace = true
# a comment
atspi = { version = "=0.29.0", features = [
  "proxies",
] }

[dev-dependencies]
tempfile = "3"

[workspace.dependencies]
gpui = { package = "gpui-pre", version = "=0.3.12" }

[target.'cfg(windows)'.dependencies]
windows = "=0.62.2"

[dependencies.foo]
version = "1"

[features]
screens = []
"#;
        assert_eq!(
            dependencies("crates/x/Cargo.toml", text),
            [
                "serde_json",
                "grace-core",
                "atspi",
                "tempfile",
                "gpui",
                "windows",
                "foo"
            ]
        );
    }

    // Small stays small

    #[test]
    fn a_small_pr_within_its_limits() {
        let changes = [change("src/a.ts", 300), change("crates/x/src/b.rs", 100)];
        assert_eq!(small_errors(&changes, &[]), Vec::<String>::new());
    }

    #[test]
    fn a_small_pr_over_400_lines() {
        let errors = small_errors(&[change("src/a.ts", 401)], &[]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains(
                "a Small PR changes 401 lines outside docs, lock files and fixtures, over 400: step up a tier"
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn docs_lock_files_and_fixtures_are_not_counted() {
        let changes = [
            change("docs/x.md", 5000),
            change("Cargo.lock", 5000),
            change("package-lock.json", 5000),
            change("test/fixtures/a.jsonl", 5000),
            change("README.md", 5000),
        ];
        assert_eq!(small_errors(&changes, &[]), Vec::<String>::new());
        assert!(counted("src/a.ts") && counted("crates/x/Cargo.toml") && !counted("docs/checks/a.bench.json"));
    }

    #[test]
    fn a_small_pr_adds_a_dependency() {
        let errors = small_errors(&[], &strings(&["serde_json", "toml"]));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("a Small PR adds a dependency (serde_json, toml): step up a tier"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_small_pr_adds_a_decision_record() {
        let added = Change {
            path: "docs/decisions/0034-x.md".to_string(),
            lines: 20,
            added: true,
        };
        let errors = small_errors(&[added], &[]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("a Small PR adds a decision record: step up a tier"),
            "{errors:?}"
        );
        // editing a record's typo is not adding one
        assert_eq!(
            small_errors(&[change("docs/decisions/0001-x.md", 2)], &[]),
            Vec::<String>::new()
        );
    }

    // tests come with behaviour

    #[test]
    fn a_fix_without_a_test() {
        let error = tests_error(&strings(&["fix(runner): x"]), &[change("src/main/a.ts", 3)], &[]).unwrap();
        assert!(
            error.contains("a feat or fix commit changes src/ or crates/ and no test"),
            "{error}"
        );
        assert!(tests_error(&strings(&["feat!: y"]), &[change("crates/grace/src/a.rs", 3)], &[]).is_some());
    }

    #[test]
    fn a_fix_with_its_test_file() {
        for test in [
            "src/main/a.test.ts",
            "src/renderer/A.test.tsx",
            "e2e/run.spec.ts",
            "crates/grace-e2e/tests/run.rs",
        ] {
            let changes = [change("src/main/a.ts", 3), change(test, 9)];
            assert_eq!(
                tests_error(&strings(&["fix(runner): x"]), &changes, &[]),
                None,
                "{test}"
            );
        }
    }

    #[test]
    fn a_rust_fix_with_a_test_in_the_same_file() {
        for line in ["    #[test]", "        assert_eq!(a, b);", "    #[gpui::test]"] {
            let changes = [change("crates/grace/src/a.rs", 12)];
            assert_eq!(
                tests_error(&strings(&["fix: x"]), &changes, &strings(&["let a = 1;", line])),
                None,
                "{line}"
            );
        }
    }

    #[test]
    fn a_rust_fix_with_neither() {
        let changes = [change("crates/grace/src/a.rs", 12)];
        assert!(tests_error(&strings(&["fix: x"]), &changes, &strings(&["let a = 1;"])).is_some());
    }

    #[test]
    fn docs_and_chores_need_no_test() {
        for header in ["docs: x", "refactor(ui): x", "build: fix the script", "fixture: x"] {
            assert_eq!(
                tests_error(&strings(&[header]), &[change("src/main/a.ts", 3)], &[]),
                None,
                "{header}"
            );
        }
    }

    #[test]
    fn a_feat_that_touches_no_code() {
        assert_eq!(
            tests_error(&strings(&["feat: x"]), &[change("docs/specs/a.md", 30)], &[]),
            None
        );
    }
}

/// `errors` on real repositories, each made in a temporary folder and removed after its test.
#[cfg(test)]
mod repository_tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Repo {
        dir: PathBuf,
        base: String,
    }

    impl Repo {
        /// A repository with one commit on `main`, which is also `origin/develop`, and a branch `head` cut from it.
        fn new(head: &str) -> Repo {
            static COUNT: AtomicUsize = AtomicUsize::new(0);
            let name = format!(
                "grace-xtask-{}-{}",
                std::process::id(),
                COUNT.fetch_add(1, Ordering::Relaxed)
            );
            let dir = std::env::temp_dir().join(name);
            std::fs::create_dir_all(&dir).unwrap();
            let mut repo = Repo {
                dir,
                base: String::new(),
            };
            repo.git(&["init", "-q", "-b", "main"]);
            repo.write(
                "package.json",
                "{\n  \"dependencies\": {\n    \"a\": \"1.0.0\"\n  }\n}\n",
            );
            repo.commit("chore: init");
            repo.commit("Merge pull request #4 from x/y");
            repo.git(&["update-ref", "refs/remotes/origin/develop", "HEAD"]);
            repo.base = repo.git(&["rev-parse", "HEAD"]);
            repo.git(&["switch", "-q", "-c", head]);
            repo
        }

        /// Git in this repository, with none of the machine's settings: no global or system configuration (identity,
        /// commit signing, hooks), and no repository a caller such as a git hook points to.
        fn command(&self) -> std::process::Command {
            let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
            let mut git = std::process::Command::new("git");
            git.env("GIT_CONFIG_GLOBAL", null).env("GIT_CONFIG_NOSYSTEM", "1");
            for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
                git.env_remove(name);
            }
            git.arg("-C")
                .arg(&self.dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"]);
            git
        }

        fn git(&self, args: &[&str]) -> String {
            let out = self.command().args(args).output().unwrap();
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }

        /// Git as `errors` calls it, on a machine that colours everything: its output must still be read right.
        fn hostile_git(&self, args: &[&str]) -> Option<String> {
            let out = self
                .command()
                .args(["-c", "color.ui=always"])
                .args(args)
                .output()
                .ok()?;
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
        }

        fn write(&self, path: &str, text: &str) {
            let file = self.dir.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }

        fn commit(&self, message: &str) {
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "--allow-empty", "-m", message]);
        }

        /// The errors of a PR from this repository's branch into `base`, with `body`.
        fn check(&self, base: &str, body: Option<&str>) -> Vec<String> {
            let env = Env {
                head_ref: self.git(&["branch", "--show-current"]),
                base_ref: base.to_string(),
                base_sha: self.base.clone(),
                head_sha: self.git(&["rev-parse", "HEAD"]),
                body: body.map(String::from),
            };
            errors(&env, &|args| self.hostile_git(args))
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn a_small_docs_pr_passes() {
        let repo = Repo::new("feature/x");
        repo.write("docs/a.md", "# A\n");
        repo.commit("docs: note a");
        assert_eq!(
            repo.check("develop", Some("## Summary\n\nTier: small\n")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_pr_without_a_tier_fails() {
        let repo = Repo::new("feature/x");
        repo.write("docs/a.md", "# A\n");
        repo.commit("docs: note a");
        let errors = repo.check("develop", Some("## Summary\n"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("has no \"Tier: small\""), "{errors:?}");
    }

    #[test]
    fn a_dispatch_run_skips_the_body() {
        let repo = Repo::new("feature/x");
        repo.write("docs/a.md", "# A\n");
        repo.commit("docs: note a");
        assert_eq!(repo.check("develop", None), Vec::<String>::new());
    }

    #[test]
    fn a_bad_header_is_reported_and_a_merge_commit_is_not() {
        let repo = Repo::new("feature/x");
        repo.commit("Bad header");
        // a real merge inside the PR: its "Merge branch" header is no Conventional Commit, and is not checked
        repo.git(&["switch", "-q", "-c", "side"]);
        repo.commit("docs: a side note");
        repo.git(&["switch", "-q", "feature/x"]);
        repo.git(&[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            "Merge branch 'side' into feature/x",
            "side",
        ]);
        let errors = repo.check("develop", Some("Tier: small"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].starts_with("commit \"Bad header\": "), "{errors:?}");
    }

    #[test]
    fn what_the_base_gained_after_the_branch_was_cut_is_not_the_prs() {
        let repo = Repo::new("feature/x");
        repo.write("docs/a.md", "# A\n");
        repo.commit("docs: note a");
        // develop moves on: a dependency and 500 lines that this PR never touched
        repo.git(&["switch", "-q", "main"]);
        repo.write(
            "package.json",
            "{\n  \"dependencies\": {\n    \"a\": \"1.0.0\",\n    \"b\": \"2.0.0\"\n  }\n}\n",
        );
        repo.write("src/big.ts", &"export const x = 1;\n".repeat(500));
        repo.commit("feat: add b");
        let moved = repo.git(&["rev-parse", "HEAD"]);
        repo.git(&["update-ref", "refs/remotes/origin/develop", "HEAD"]);
        repo.git(&["switch", "-q", "feature/x"]);
        let moved = Repo {
            dir: repo.dir.clone(),
            base: moved,
        };
        assert_eq!(moved.check("develop", Some("Tier: small")), Vec::<String>::new());
        std::mem::forget(moved); // `repo` removes the folder
    }

    #[test]
    fn a_release_into_main_is_not_asked_about_scripts_already_on_develop() {
        let repo = Repo::new("feature/x");
        // a script lands on develop through its own PR, where it was checked
        repo.write("scripts/x.mjs", "console.log(1);\n");
        repo.commit("build: add x");
        repo.git(&["update-ref", "refs/remotes/origin/develop", "HEAD"]);
        repo.git(&["switch", "-q", "-c", "release/v1.0.0-rc.1"]);
        repo.write(
            "docs/checks/2026-10-30-release-1.0.0-rc.1-packaged.md",
            "# Check\n\n## Public docs\n\nFine.\n",
        );
        repo.commit("docs: record the 1.0.0-rc.1 packaged-app check");
        assert_eq!(repo.check("main", Some("Release 1.0.0-rc.1")), Vec::<String>::new());
    }

    #[test]
    fn a_spec_line_left_as_the_template_has_it_fails() {
        let repo = Repo::new("feature/x");
        repo.write("docs/specs/a.md", "# Spec\n");
        repo.write("docs/plans/a.md", "# Plan\n");
        repo.commit("docs: add the spec and the plan");
        let body = "Tier: feature\nCloses #1\nSpec: docs/specs/\nPlan: docs/plans/a.md\n";
        let errors = repo.check("develop", Some(body));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("Spec: docs/specs/ is not a file of this PR's head"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_branch_that_may_not_target_main_fails() {
        let repo = Repo::new("feature/x");
        repo.commit("docs: note");
        let errors = repo.check("main", Some("Tier: small"));
        assert!(
            errors
                .iter()
                .any(|error| error.contains("feature/x may not target main")),
            "{errors:?}"
        );
    }

    #[test]
    fn a_release_into_main_without_its_record_fails() {
        let repo = Repo::new("release/v1.0.0-rc.1");
        repo.commit("build: bump version to 1.0.0-rc.1");
        let errors = repo.check("main", Some(""));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("-1.0.0-rc.1-packaged.md"), "{errors:?}");
        repo.write(
            "docs/checks/2026-10-30-release-1.0.0-rc.1-packaged.md",
            "# Check\n\n## Public docs\n\nFine.\n",
        );
        repo.commit("docs: record the 1.0.0-rc.1 packaged-app check");
        assert_eq!(repo.check("main", Some("")), Vec::<String>::new());
    }

    #[test]
    fn a_dependabot_pr_passes() {
        let repo = Repo::new("dependabot/npm_and_yarn/develop/minor-and-patch-450526cfbb");
        repo.write(
            "package.json",
            "{\n  \"dependencies\": {\n    \"a\": \"1.0.1\"\n  }\n}\n",
        );
        repo.commit("Bump the minor-and-patch group across 1 directory with 12 updates, far beyond 72 characters");
        assert_eq!(
            repo.check("develop", Some("Bumps the minor-and-patch group.")),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_small_pr_that_adds_a_dependency_fails() {
        let repo = Repo::new("feature/x");
        repo.write(
            "package.json",
            "{\n  \"dependencies\": {\n    \"a\": \"1.0.0\",\n    \"b\": \"2.0.0\"\n  }\n}\n",
        );
        repo.write(
            "crates/x/Cargo.toml",
            "[package]\nname = \"x\"\n\n[dependencies]\nserde_json = \"1\"\n",
        );
        repo.commit("build(deps): add b and serde_json");
        let errors = repo.check("develop", Some("Tier: small"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("a Small PR adds a dependency (serde_json, b)"),
            "{errors:?}"
        );
        // the same change is fine in a feature PR with its papers
        repo.write("docs/specs/a.md", "# Spec\n");
        repo.write("docs/plans/a.md", "# Plan\n");
        repo.commit("docs: add the spec and the plan");
        let body = "Tier: feature\nCloses #1\nSpec: docs/specs/a.md\nPlan: docs/plans/a.md\n";
        assert_eq!(repo.check("develop", Some(body)), Vec::<String>::new());
    }

    #[test]
    fn a_feature_names_files_that_exist_at_its_head() {
        let repo = Repo::new("feature/x");
        repo.write("src/a.ts", "export const a = 1;\n");
        repo.write("src/a.test.ts", "it('a', () => {});\n");
        repo.write("docs/specs/a.md", "# Spec\n");
        repo.commit("feat: add a");
        let body = "Tier: feature\nCloses #1\nSpec: docs/specs/a.md\nPlan: docs/plans/a.md\n";
        let errors = repo.check("develop", Some(body));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("Plan: docs/plans/a.md is not a file of this PR's head"),
            "{errors:?}"
        );
    }

    #[test]
    fn a_rust_fix_needs_a_test_line() {
        let repo = Repo::new("feature/x");
        repo.write("crates/x/src/a.rs", "pub fn a() -> u8 {\n    2\n}\n");
        repo.commit("fix: give a its value");
        let errors = repo.check("develop", Some("Tier: small"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("a feat or fix commit changes src/ or crates/ and no test"),
            "{errors:?}"
        );
        repo.write(
            "crates/x/src/a.rs",
            "pub fn a() -> u8 {\n    2\n}\n\n#[test]\nfn a_is_two() {\n    assert_eq!(a(), 2);\n}\n",
        );
        repo.commit("test: pin a");
        assert_eq!(repo.check("develop", Some("Tier: small")), Vec::<String>::new());
    }

    #[test]
    fn a_session_link_in_a_commit_is_refused() {
        let repo = Repo::new("feature/x");
        repo.git(&[
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "docs: note",
            "-m",
            "https://claude.ai/code/session_01",
        ]);
        let errors = repo.check("develop", Some("Tier: small"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("says more about the agent"), "{errors:?}");
    }

    #[test]
    fn a_project_pr_names_a_plan_with_slices() {
        let repo = Repo::new("feature/x");
        repo.write("docs/specs/a.md", "# Spec\n");
        repo.write("docs/plans/a.md", "# Plan\n\n### Task 1\n");
        repo.commit("docs: add the spec and the plan");
        let body = "Tier: project\nPart of #1\nSpec: docs/specs/a.md\nPlan: docs/plans/a.md\n";
        let errors = repo.check("develop", Some(body));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains("Plan: docs/plans/a.md has no \"## Slices\" section"),
            "{errors:?}"
        );
        // a feature's plan needs none
        assert_eq!(
            repo.check("develop", Some(&body.replace("project", "feature"))),
            Vec::<String>::new()
        );
        repo.write(
            "docs/plans/a.md",
            "# Plan\n\n## Slices\n\n| Slice | Model |\n|---|---|\n| 1 | sonnet |\n",
        );
        repo.commit("docs: name the slices");
        assert_eq!(repo.check("develop", Some(body)), Vec::<String>::new());
    }
}
