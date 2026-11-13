---
status: draft
issue: 373
summary: "Five tasks: the frontmatter reader and index writer in crates/xtask, fields on 146 docs, the five indexes and rule lines, then the two checks"
---

# Docs index Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every spec, plan, check and research note carries frontmatter, each docs folder has a generated `README.md` index, and two checks keep both true.

**Architecture:** One new module, `crates/xtask/src/docs.rs`: pure functions from a `read` and a `list` closure to the five index texts and to a list of errors, as `rules.rs` does. `cargo xtask docs` runs them on the working tree and writes the indexes; `cargo xtask check-rules` runs them on the git index and fails on an error. `pr.rs` gets one pure function for the `built` rule. The checks are wired in last, after the docs comply, so every commit passes.

**Tech Stack:** Rust (the version of `rust-toolchain.toml`), standard library only; `git` started with an argument list.

**Spec:** [docs/specs/2026-11-13-docs-index.md](../specs/2026-11-13-docs-index.md)

## Global Constraints

- No dependency in `crates/xtask/Cargo.toml`.
- `status` of a spec or plan: `draft`, `approved`, `built` or `superseded`.
- `summary`: one line, in double quotes, at most 160 characters, no `"` and no `\` inside.
- `issue`: `373` or `[64, 231]`.
- No file under `docs/decisions/` changes but its new `README.md`. `AGENTS.md` and `WORKFLOW.md` do not change.
- The indexes are `docs/specs/README.md`, `docs/plans/README.md`, `docs/checks/README.md`, `docs/research/README.md`, `docs/decisions/README.md`.
- Subagents run on `sonnet`, never with the model unset.
- Before each commit that touches `crates/xtask`: `cargo fmt --all --check`, `cargo clippy -p xtask --all-targets -- -D warnings`, `cargo test -p xtask`.
- Commit headers: `build:` for `crates/xtask`, `docs:` for docs and rules files. Each message ends with `Part of #373` and the attribution lines of `.claude/settings.json`.

## Review Focus

1. A doc saved with CRLF line endings: `cargo xtask docs` reads the working tree and `check-rules` the git index, so both must give the same index. Task 1 and Task 2 test it.
2. `issue: #64`: YAML reads `#64` as a comment, so the field is empty. The error must say to write the number without `#`. Task 1 tests it.
3. A summary of 160 characters with accents or arrows: the limit counts characters, not bytes. Task 1 tests it.
4. A tracked doc deleted in the working tree but not staged: `list` names it and `read` gives `None`. It is skipped, with no panic. Task 2 tests it.
5. A PR body whose `Spec:` names a file with no frontmatter while it says `Closes #N`: the error names the file and the missing status. Task 5 tests it.

---

### Task 1: Read a doc's frontmatter and title

**Files:**
- Create: `crates/xtask/src/docs.rs`
- Modify: `crates/xtask/src/main.rs` (add `mod docs;`)

**Interfaces:**
- Produces:
  - `pub enum Kind { Spec, Plan, Check, Research, Decision }` (`Clone, Copy, PartialEq, Debug`)
  - `pub const STATUSES: [&str; 4] = ["approved", "draft", "built", "superseded"];` (the index's group order)
  - `pub fn field<'a>(text: &'a str, name: &str) -> Option<&'a str>`: the raw value of `name:` in the frontmatter, trimmed, quotes kept; `None` when the doc has no frontmatter or no such line.
  - `pub fn title(text: &str) -> Option<&str>`: the text of the first `# ` line after the frontmatter.
  - `pub fn status(text: &str) -> Option<&str>`: `field(text, "status")`.
  - `pub fn summary(text: &str) -> Result<&str, String>`: the text between the double quotes, or the reason it is not valid.
  - `pub fn issues(text: &str) -> Result<Vec<u32>, String>`: empty when there is no `issue:` line.
  - `pub fn entry_errors(kind: Kind, path: &str, text: &str) -> Vec<String>`: one message per failure, each starting with `{path}: `.

- [ ] **Step 1: Write the failing tests** in `docs.rs`, `mod tests`:

```rust
const SPEC: &str = "---\nstatus: approved\nissue: 373\nsummary: \"Finds the doc in one read\"\n---\n\n# Docs index\n\nText.\n";

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
    assert!(errors[0].contains("docs/plans/a.md: no status: line in its frontmatter"), "{errors:?}");
}

#[test]
fn a_check_needs_a_summary_and_no_status() {
    let text = "---\nsummary: \"VL-29 passes\"\n---\n# Check\n";
    assert_eq!(entry_errors(Kind::Check, "docs/checks/a.md", text), Vec::<String>::new());
    let errors = entry_errors(Kind::Research, "docs/research/a.md", "# Note\n");
    assert!(errors[0].contains("docs/research/a.md: no summary: line in its frontmatter"), "{errors:?}");
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
    assert_eq!(entry_errors(Kind::Decision, "docs/decisions/0002-x.md", text), Vec::<String>::new());
    let errors = entry_errors(Kind::Decision, "docs/decisions/0002-x.md", "---\nstatus: accepted\n---\n");
    assert!(errors[0].contains("docs/decisions/0002-x.md: no \"# \" title"), "{errors:?}");
}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p xtask docs::`
Expected: does not compile, `field`, `title` and the others are not defined.

- [ ] **Step 3: Implement the functions of the Interfaces block**

The frontmatter is the lines between a first line `---` and the next `---`, each compared after `trim_end` so CRLF reads the same. A field is a line that starts with `name:` at column 0. `entry_errors` checks: a title for every kind; `status` in `STATUSES` for `Spec` and `Plan`, present for `Decision`; `summary` for every kind but `Decision`; `issues` when the line is there, for `Spec` and `Plan`.

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test -p xtask docs::`
Expected: all pass.

- [ ] **Step 5: Commit** as `build: read a doc's frontmatter and title`.

---

### Task 2: Write the indexes, and the `docs` command

**Files:**
- Modify: `crates/xtask/src/docs.rs`
- Modify: `crates/xtask/src/main.rs` (the `docs` arm and its `USAGE` line)
- Modify: `crates/xtask/src/rules.rs` (`read_index` and `list_index` become `pub(crate)`)

**Interfaces:**
- Consumes: Task 1's functions.
- Produces:
  - `pub const FOLDERS: [(&str, Kind, &str); 5]`: `("docs/specs", Kind::Spec, "Specs")`, `("docs/plans", Kind::Plan, "Plans")`, `("docs/checks", Kind::Check, "Checks")`, `("docs/research", Kind::Research, "Research")`, `("docs/decisions", Kind::Decision, "Decisions")`.
  - `pub fn entries(folder: &str, kind: Kind, read: &dyn Fn(&str) -> Option<String>, list: &dyn Fn(&str) -> Vec<String>) -> Vec<(String, String)>`: `(path, text)` of each entry, newest first. It lists `{folder}/*.md`, and for `Kind::Research` also `{folder}/*/README.md`; drops `{folder}/README.md` and any path `read` gives `None` for; sorts by path, descending.
  - `pub fn index(folder: &str, kind: Kind, heading: &str, entries: &[(String, String)]) -> String`
  - `pub fn docs_errors(read: &dyn Fn(&str) -> Option<String>, list: &dyn Fn(&str) -> Vec<String>) -> Vec<String>`: every `entry_errors`; when there is none, one error per index that `read` does not give byte for byte: `{folder}/README.md is not what cargo xtask docs writes: run it and commit the result`.
  - `pub fn run() -> u8`: on the working tree. Prints the entry errors and returns 1 when there are any, writing nothing; otherwise writes the five files and returns 0.

**The index text, exactly:**

```markdown
<!-- Generated by `cargo xtask docs` from the frontmatter of the docs in this folder. Do not edit. -->

# Specs

## Approved

- 2026-11-13 [Docs index: find the right doc in one read](2026-11-13-docs-index.md) #373: Finds the doc in one read

## Built

- 2026-11-02 [Project setup](2026-11-02-project-setup.md) #64 #231: One line
```

- A line is `- {stamp} [{title}]({path relative to the folder})`, then ` #N` per issue, then `: {summary}` when the kind has one. The stamp is the first 10 characters of the relative path, 4 for a decision.
- Specs and plans: one `## ` group per status of `STATUSES`, in that order, with the first letter in upper case. Decisions: one group per status, `accepted` first, then the others in alphabetical order. A group with no entry is left out. Checks and research: no group, the lines follow the `# ` heading.
- Every line ends with `\n`, there is one empty line between blocks, and the file ends with one `\n`.

- [ ] **Step 1: Write the failing tests**

```rust
fn files() -> HashMap<&'static str, String> {
    HashMap::from([
        ("docs/specs/2026-11-13-b.md", SPEC.to_string()),
        ("docs/specs/2026-11-02-a.md", SPEC.replace("approved", "built").replace("373", "[64, 231]").replace("Docs index", "Setup")),
        ("docs/specs/README.md", String::new()),
        ("docs/research/2026-11-07-p/README.md", "---\nsummary: \"Go\"\n---\n# Prototype\n".to_string()),
        ("docs/research/2026-11-07-p/notes/README.md", "# Deeper\n".to_string()),
        ("docs/decisions/0002-plain.md", "---\nstatus: accepted\n---\n# Keep plain Markdown\n".to_string()),
    ])
}
```

with a `list` that gives, sorted ascending, the keys a glob matches (`*` does not cross `/`), and a `read` on the map. Tests:

- `lists_entries_newest_first_without_the_index`: `entries("docs/specs", Kind::Spec, ..)` gives the paths `2026-11-13-b.md` then `2026-11-02-a.md`.
- `a_research_folder_is_listed_by_its_readme`: `entries("docs/research", Kind::Research, ..)` gives only `docs/research/2026-11-07-p/README.md`.
- `skips_a_listed_file_that_cannot_be_read`: a `list` that also names `docs/specs/2026-11-14-gone.md` gives the same two entries.
- `writes_the_specs_index`: `index("docs/specs", Kind::Spec, "Specs", &entries)` equals, as one string, the comment line, `# Specs`, `## Approved` with `- 2026-11-13 [Docs index](2026-11-13-b.md) #373: Finds the doc in one read`, then `## Built` with `- 2026-11-02 [Setup](2026-11-02-a.md) #64 #231: Finds the doc in one read`, and no `## Draft`.
- `writes_the_research_and_decisions_indexes`: the research line is `- 2026-11-07 [Prototype](2026-11-07-p/README.md): Go`; the decisions index has `## Accepted` and `- 0002 [Keep plain Markdown](0002-plain.md)`.
- `the_same_docs_with_crlf_give_the_same_index`: `index` on entries whose text had `\n` replaced by `\r\n` equals `index` on the originals.
- `an_index_out_of_date_is_an_error`: with every folder's `README.md` set to its `index` text, `docs_errors` is empty; with `docs/specs/README.md` changed, it holds one error that contains `docs/specs/README.md is not what cargo xtask docs writes`.
- `entry_errors_come_before_the_index`: with the status removed from one spec, `docs_errors` holds that file's error and no index error.

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p xtask docs::`
Expected: does not compile.

- [ ] **Step 3: Implement `entries`, `index`, `docs_errors` and `run`**

`run` lists with `git ls-files --cached --others --exclude-standard -- ":(glob){glob}"` and reads with `std::fs::read_to_string` under `rules::root()`, replacing `\r\n` by `\n`. It writes with `std::fs::write`. Add to `USAGE`:

```
  docs         writes the README.md index of each docs folder from the docs' frontmatter; fails, and
               writes nothing, when a doc lacks a field
```

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test -p xtask`
Expected: all pass.

- [ ] **Step 5: Commit** as `build: write an index of each docs folder`.

---

### Task 3: Give the existing docs their fields

**Files:**
- Modify: the 47 `docs/specs/*.md`, the 50 `docs/plans/*.md`, the 32 `docs/checks/*.md`, the 6 `docs/research/*.md` and the 11 `docs/research/*/README.md`.

- [ ] **Step 1: Decide each spec's and plan's status**

List every issue a spec names with its state: `gh issue list --state all --limit 400 --json number,state,title`. A spec's issue comes from its `Covers`, `Part of` or `Issue:` text; an `F` id from `docs/features.md`. A plan's issue is its spec's. Then:

| The issue | The spec says | Status |
|---|---|---|
| closed | anything | `built` |
| open | approved | `approved` |
| open | draft | `draft` |
| none | | read the doc and the merged PRs that name it (`gh pr list --state merged --search "<file name>"`) |

A plan of a phase that is merged while its issue stays open is `built`: the tracking issue says which phases are merged. Write the table (file, issue, status) to `<temp>/agent-work/grace/docs-index/status.tsv`, and list for the owner any row that stays unsure.

- [ ] **Step 2: Add the frontmatter, one subagent per folder, in parallel**

Four `sonnet` subagents: specs, plans, checks, research. Each brief holds: the folder, the rows of `status.tsv` for it, R1.1 to R1.6 of the spec, the summary rule of Global Constraints, and these rules:

- Add the frontmatter as the first lines of the file, then one empty line.
- The summary says what the doc decides or found: a check's result with its number, a research note's verdict, a spec's behaviour, a plan's outcome. Read the doc's first 80 lines and its result or verdict section.
- In a spec, remove the word `Status:` and its value from the prose line; keep the rest of the line (the date, `Covers`, the links). Text that tells the state of long work stays, as a plain sentence.
- Change nothing else. Run no git command.

- [ ] **Step 3: Check the result**

Run: `cargo xtask docs`
Expected: exit 0 and five `README.md` files written. On an error, fix the named file and run again. Then read `git diff --stat -- docs/decisions` (expected: empty) and sample ten summaries against their docs.

- [ ] **Step 4: Commit** the 146 docs, without the indexes, as `docs: give specs, plans, checks and research notes frontmatter`. Remove `<temp>/agent-work/grace/docs-index/`.

---

### Task 4: The indexes, the rules and Obsidian

**Files:**
- Create: the five `README.md` indexes (written by `cargo xtask docs`)
- Modify: `.claude/rules/docs.md`, `.claude/rules/electron.md`, `.claude/rules/gpui.md`, `.claude/rules/karate.md`, `.claude/rules/both-apps.md`, `.claude/rules/ci.md`, `.gitignore`

- [ ] **Step 1: Write the rules**

In `docs.md`, a new section `## Frontmatter and indexes` that holds R1.1, R1.3, R1.4, R1.5, R4.2, R5.1 and R5.3 of the spec in the file's own short style, and the line: "After adding a doc or changing its frontmatter, run `cargo xtask docs` and commit the indexes." In the docs map, the rows of the five folders say their `README.md` is the index.

In each of the four area rules files, one line near the top: "Before changing behaviour, read `docs/specs/README.md` and `docs/decisions/README.md`, and open only the docs whose line matches."

In `ci.md`, the `checks` row: after "the size and globs of the agent rules files" add "and the docs' frontmatter and indexes".

In `.gitignore`: `.obsidian/`.

- [ ] **Step 2: Write the indexes**

Run: `cargo xtask docs`, then again.
Expected: the second run changes no file (`git status --short` is the same).

- [ ] **Step 3: Check the rules files**

Run: `cargo xtask check-rules`
Expected: exit 0 (the docs check is not wired in yet; this proves the rules files' globs and sizes).

- [ ] **Step 4: Commit** as `docs: index each docs folder and tell agents to read it`.

---

### Task 5: The two checks

**Files:**
- Modify: `crates/xtask/src/rules.rs` (`run`, the module comment)
- Modify: `crates/xtask/src/pr.rs` (`built_errors`, its call in `errors`)
- Modify: `crates/xtask/src/main.rs` (`USAGE` of `check-rules` and `check-pr`)

**Interfaces:**
- Consumes: `docs::docs_errors`, `docs::status`.
- Produces: `pub fn built_errors(body: &str, read: &dyn Fn(&str) -> Option<String>) -> Vec<String>` in `pr.rs`.

- [ ] **Step 1: Write the failing tests** in `pr.rs`, `mod tests`:

```rust
#[test]
fn a_pr_that_closes_its_issue_names_built_papers() {
    let built = "---\nstatus: built\n---\n# A\n";
    let read = |path: &str| match path {
        "docs/specs/a.md" => Some(built.to_string()),
        "docs/plans/a.md" => Some(built.replace("built", "approved")),
        "docs/specs/old.md" => Some("# No frontmatter\n".to_string()),
        _ => None,
    };
    let body = "Closes #12\nSpec: docs/specs/a.md\nPlan: docs/plans/a.md\n";
    let errors = built_errors(body, &read);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("Plan: docs/plans/a.md has status \"approved\": a PR that closes its issue sets it to built"));
    assert_eq!(built_errors(&body.replace("Closes", "Part of"), &read), Vec::<String>::new());
    let errors = built_errors("Closes #12\nSpec: docs/specs/old.md\n", &read);
    assert!(errors[0].contains("Spec: docs/specs/old.md has no status"), "{errors:?}");
    assert_eq!(built_errors("Closes #12\nSpec: docs/specs/nope.md\n", &read), Vec::<String>::new());
}
```

A path that is not a file gives no error here: `papers_errors` already reports it. And in `rules.rs`, `mod repository_tests` (make it if the file has none, as `pr.rs` has):

```rust
#[test]
fn the_repository_docs_have_their_fields_and_indexes() {
    assert_eq!(crate::docs::docs_errors(&read_index, &list_index), Vec::<String>::new());
}
```

- [ ] **Step 2: Run them**

Run: `cargo test -p xtask`
Expected: `built_errors` does not compile; after a stub, its test fails and the repository test passes (Tasks 3 and 4 made it true).

- [ ] **Step 3: Implement**

`built_errors`: when a line of `body` has `Closes #` followed by a digit, for `Spec:` and `Plan:` read the named path and compare `docs::status` with `built`. Move the `numbered` closure of `papers_errors` to a private function both use. In `errors`, call it in the `Some(tier)` arm with `show(head, path)`. In `rules::run`, extend the errors with `docs::docs_errors(&read_index, &list_index)`.

- [ ] **Step 4: Run the tests, then the acceptance checks of the spec**

Run: `cargo test -p xtask`, `cargo xtask check-rules` (exit 0), then by hand, undoing each change after:

- remove `summary` from one spec, `git add` it: `cargo xtask check-rules` exits 1 and names the file and `summary`;
- add a line to `docs/specs/README.md`, `git add` it: `cargo xtask check-rules` exits 1 and names `cargo xtask docs`;
- `git diff --stat origin/develop -- docs/decisions AGENTS.md WORKFLOW.md` lists only `docs/decisions/README.md`.

- [ ] **Step 5: Commit** as `build: check the docs' fields, indexes and built status`.

- [ ] **Step 6: Close the work**

Set this plan and the spec to `status: built`, run `cargo xtask docs`, commit as `docs: mark the docs index spec and plan built`. Add `Plan: docs/plans/2026-11-13-docs-index.md` to the PR body. Run `cargo xtask check-pr`. Ask one fresh `opus` reviewer to read the whole branch against the spec and this plan's Review Focus; fix what it finds. Say in the PR body that an open Feature PR must merge `develop` and set its spec and plan to `built` before it closes its issue. Mark the PR ready when `checks` passes.
