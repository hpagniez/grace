//! `cargo xtask tidy`: what was left behind (WORKFLOW.md, Housekeeping).
//!
//! A leftover is something with no open PR behind it: a branch, a worktree, a stash, untracked files on the default
//! branch, a scratch folder, a link inside the checkout. Work in progress is not one. The command only reports: it
//! changes nothing, prints nothing when all is clean, and never fails, so a session-start hook can run it.

use std::path::Path;

/// A local branch, other than the default one and `main`.
pub struct Branch {
    pub name: String,
    /// Every commit of it is on the default branch.
    pub merged: bool,
    /// It tracked a remote branch that no longer exists: its PR was merged or closed.
    pub gone: bool,
    /// Whole days since its last commit.
    pub age_days: u64,
}

/// An open pull request.
pub struct Pr {
    pub number: u64,
    pub branch: String,
    /// Whole days since anything happened on it.
    pub idle_days: u64,
    pub bot: bool,
}

/// What GitHub holds: every branch, and the open pull requests.
pub struct Remote {
    pub branches: Vec<String>,
    pub prs: Vec<Pr>,
}

/// The state of a checkout.
pub struct Facts {
    /// The default branch, such as `develop`.
    pub default: String,
    /// The branch checked out, empty on a detached HEAD.
    pub current: String,
    pub branches: Vec<Branch>,
    /// The path and branch of every worktree but the main checkout.
    pub worktrees: Vec<(String, String)>,
    pub stashes: usize,
    pub untracked: Vec<String>,
    /// Junctions and symbolic links found in untracked or ignored folders.
    pub links: Vec<String>,
    /// The task folders under `<temp>/agent-work/<repository>/` that are not empty.
    pub scratch: Vec<String>,
    /// `None` when GitHub cannot be read: no `gh`, no network.
    pub remote: Option<Remote>,
}

/// Every leftover, one line each, with what to do about it.
pub fn leftovers(facts: &Facts) -> Vec<String> {
    let default = &facts.default;
    let open = |branch: &str| {
        facts
            .remote
            .as_ref()
            .is_some_and(|remote| remote.prs.iter().any(|pr| pr.branch == branch))
    };
    let mut lines = Vec::new();

    for branch in &facts.branches {
        let name = &branch.name;
        // a branch just cut has no commit of its own yet, so git calls it merged: that is work in progress
        if branch.merged && (*name != facts.current || branch.gone) {
            let first = if *name == facts.current {
                format!("switch to {default}, then ")
            } else {
                String::new()
            };
            lines.push(format!(
                "branch {name} is merged into {default}: {first}git branch -d {name}"
            ));
        } else if branch.gone && !branch.merged {
            lines.push(format!(
                "branch {name} lost its remote branch (its PR was merged or closed): check it holds nothing, then delete it"
            ));
        } else if facts.remote.is_some()
            && !branch.merged
            && !open(name)
            && branch.age_days >= 1
            && *name != facts.current
        {
            lines.push(format!(
                "branch {name} has no open PR after {} days: open its PR or delete it",
                branch.age_days
            ));
        }
    }
    if facts.remote.is_some() {
        let age = |name: &str| {
            facts
                .branches
                .iter()
                .find(|branch| branch.name == name)
                .map_or(u64::MAX, |branch| branch.age_days)
        };
        let left = |(_, branch): &&(String, String)| !open(branch) && *branch != facts.current && age(branch) >= 1;
        for (path, branch) in facts.worktrees.iter().filter(left) {
            lines.push(format!(
                "worktree {path} ({branch}) has no open PR: finish it or git worktree remove it"
            ));
        }
    }
    match facts.stashes {
        0 => {}
        1 => lines.push("1 stash: git stash list".to_string()),
        count => lines.push(format!("{count} stashes: git stash list")),
    }
    if !facts.untracked.is_empty() && facts.current == *default {
        let shown: Vec<&str> = facts.untracked.iter().take(5).map(String::as_str).collect();
        let more = facts.untracked.len().saturating_sub(shown.len());
        let rest = if more > 0 {
            format!(" (+{more} more)")
        } else {
            String::new()
        };
        lines.push(format!("untracked files on {default}: {}{rest}", shown.join(", ")));
    }
    for link in &facts.links {
        lines.push(format!(
            "link inside the checkout: {link}: remove the link alone (rmdir or unlink), never a recursive delete"
        ));
    }
    // the folder of the task in hand is work in progress
    let task = facts.current.rsplit('/').next().unwrap_or_default();
    let reviewed = |folder: &str| {
        facts
            .remote
            .as_ref()
            .is_some_and(|remote| remote.prs.iter().any(|pr| pr.branch.rsplit('/').next() == Some(folder)))
    };
    for folder in facts
        .scratch
        .iter()
        .filter(|folder| folder.as_str() != task && !reviewed(folder))
    {
        lines.push(format!(
            "scratch folder {folder} is not empty: its task is over, remove what it left"
        ));
    }
    if let Some(remote) = &facts.remote {
        let kept = [default.as_str(), "main", "feature/pr-screenshots"];
        for branch in remote
            .branches
            .iter()
            .filter(|branch| !kept.contains(&branch.as_str()) && !open(branch))
        {
            lines.push(format!(
                "remote branch {branch} has no open PR: open one, or delete it on the remote"
            ));
        }
        // a bot's PR and a release wait for the owner, however long
        let waits = |pr: &&Pr| pr.bot || pr.branch.starts_with("release/") || pr.branch.starts_with("hotfix/");
        for pr in remote.prs.iter().filter(|pr| pr.idle_days >= 7 && !waits(pr)) {
            lines.push(format!(
                "PR #{} ({}) has not moved for {} days: finish it or close it with a note",
                pr.number, pr.branch, pr.idle_days
            ));
        }
    }
    lines
}

/// Folders never searched for links: too large, and never where an agent works.
const SKIPPED: [&str; 5] = ["node_modules", "vendor", "out", "target", ".git"];

/// The jq filter that turns `gh pr list` into one line a PR: number, branch, whole days idle, whether a bot opened it.
const PRS: &str = ".[] | [.number, .headRefName, ([0, ((now - (.updatedAt | fromdate)) / 86400 | floor)] | max), (.author.is_bot // false)] | @tsv";

/// Whether `path` is a symbolic link or, on Windows, a junction.
fn is_link(path: &Path) -> bool {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return false;
    };
    meta.file_type().is_symlink()
}

/// The links in `dir`, down to `depth` folders below it, as paths from `root`. A link is named, never followed.
fn links_in(root: &Path, dir: &Path, depth: usize, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|entry| entry.path()) {
        let skipped = path
            .file_name()
            .is_some_and(|name| SKIPPED.iter().any(|skip| name == *skip));
        if is_link(&path) {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            found.push(relative.to_string_lossy().replace('\\', "/"));
        } else if depth > 0 && !skipped && path.is_dir() {
            links_in(root, &path, depth - 1, found);
        }
    }
}

/// What the checkout at `root` holds, without GitHub. `scratch` is this repository's folder of task folders, `now`
/// the time in seconds since 1970, and `git` gives git's output for its arguments, `None` when it fails.
pub fn local_facts(root: &Path, scratch: &Path, now: u64, git: &dyn Fn(&[&str]) -> Option<String>) -> Facts {
    let lines = |args: &[&str]| -> Vec<String> { git(args).unwrap_or_default().lines().map(String::from).collect() };
    let head = git(&["symbolic-ref", "--short", "refs/remotes/origin/HEAD"]).unwrap_or_default();
    let default = head.trim().strip_prefix("origin/").unwrap_or("develop").to_string();
    let current = git(&["branch", "--show-current"])
        .unwrap_or_default()
        .trim()
        .to_string();

    // against the remote's default branch when there is one: the local one may be behind
    let format = "--format=%(refname:short)";
    let merged = git(&["branch", "--merged", &format!("origin/{default}"), format])
        .or_else(|| git(&["branch", "--merged", &default, format]))
        .unwrap_or_default();
    let merged: Vec<&str> = merged.lines().map(str::trim).collect();
    let refs = "--format=%(refname:short)%09%(upstream:track)%09%(committerdate:unix)";
    let branches = lines(&["for-each-ref", "refs/heads", refs])
        .iter()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.to_string();
            let gone = fields.next()?.contains("gone");
            let committed = fields.next()?.trim().parse::<u64>().ok()?;
            let age_days = now.saturating_sub(committed) / 86_400;
            (name != default && name != "main").then(|| Branch {
                merged: merged.contains(&name.as_str()),
                name,
                gone,
                age_days,
            })
        })
        .collect();

    // git lists the main checkout first, each worktree as a block of lines
    let worktrees = git(&["worktree", "list", "--porcelain"])
        .unwrap_or_default()
        .replace("\r\n", "\n")
        .split("\n\n")
        .skip(1)
        .filter_map(|block| {
            let field = |name: &str| block.lines().find_map(|line| line.strip_prefix(name));
            let branch = field("branch refs/heads/").unwrap_or("detached");
            Some((field("worktree ")?.to_string(), branch.to_string()))
        })
        .collect();

    let status = git(&[
        "--no-optional-locks",
        "status",
        "--porcelain",
        "-z",
        "--untracked-files=normal",
        "--ignored",
    ])
    .unwrap_or_default();
    let status: Vec<String> = status.split('\0').map(String::from).collect();
    let untracked: Vec<String> = status
        .iter()
        .filter_map(|line| line.strip_prefix("?? "))
        .map(String::from)
        .collect();
    let mut links = Vec::new();
    // an agent's stray folder is untracked or ignored: tracked files are what the project chose to hold
    for folder in status
        .iter()
        .filter_map(|line| line.strip_prefix("?? ").or_else(|| line.strip_prefix("!! ")))
    {
        let Some(folder) = folder.strip_suffix('/') else {
            continue;
        };
        let path = root.join(folder);
        if is_link(&path) {
            links.push(folder.to_string());
        } else if !SKIPPED.contains(&folder.rsplit('/').next().unwrap_or(folder)) {
            links_in(root, &path, 3, &mut links);
        }
    }

    // git lists no folder that holds no file, so a link to an empty folder hides from it: look at the top level too
    let tracked = lines(&["ls-tree", "--name-only", "HEAD"]);
    for path in std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
    {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if tracked.contains(&name) || SKIPPED.contains(&name.as_str()) {
            continue;
        }
        if is_link(&path) {
            links.push(name);
        } else if path.is_dir() {
            links_in(root, &path, 3, &mut links);
        }
    }
    links.sort();
    links.dedup();

    let filled = |dir: &Path| std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some());
    let mut scratch: Vec<String> = std::fs::read_dir(scratch)
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect::<Vec<_>>())
        .unwrap_or_default()
        .iter()
        .filter(|path| path.is_dir() && filled(path))
        .filter_map(|path| Some(path.file_name()?.to_string_lossy().into_owned()))
        .collect();
    scratch.sort();

    Facts {
        default,
        current,
        branches,
        worktrees,
        stashes: lines(&["stash", "list"]).len(),
        untracked,
        links,
        scratch,
        remote: None,
    }
}

/// What GitHub holds, `None` when `git` cannot list the remote's branches or `gh` its pull requests.
pub fn remote(git: &dyn Fn(&[&str]) -> Option<String>, gh: &dyn Fn(&[&str]) -> Option<String>) -> Option<Remote> {
    let heads = git(&["ls-remote", "--heads", "origin"])?;
    let branches = heads
        .lines()
        .filter_map(|line| line.split_once("refs/heads/"))
        .map(|(_, name)| name.trim().to_string());
    let fields = ["number,headRefName,updatedAt,author", "--jq", PRS];
    let listed = gh(&[
        "pr", "list", "--state", "open", "--limit", "200", "--json", fields[0], fields[1], fields[2],
    ])?;
    let prs = listed.lines().filter_map(|line| {
        let mut fields = line.split('\t');
        Some(Pr {
            number: fields.next()?.parse().ok()?,
            branch: fields.next()?.to_string(),
            idle_days: fields
                .next()?
                .trim()
                .parse::<i64>()
                .map_or(0, |days| days.max(0) as u64),
            bot: fields.next()?.trim() == "true",
        })
    });
    Some(Remote {
        branches: branches.collect(),
        prs: prs.collect(),
    })
}

/// Prints the leftovers of this repository, if any. Always 0: a report never fails a session.
pub fn run() -> u8 {
    let root = crate::rules::root();
    let git = |args: &[&str]| crate::rules::git(root, args);
    // GitHub is read without a prompt and with a short wait: the start of a session never hangs on a passphrase,
    // a host key or a login
    // the user's own ssh command, when git is told to use one: another ssh may not reach their key agent
    let ssh = git(&["config", "--get", "core.sshCommand"]).unwrap_or_default();
    let ssh = format!(
        "{} -o BatchMode=yes -o ConnectTimeout=5",
        if ssh.trim().is_empty() { "ssh" } else { ssh.trim() }
    );
    let quiet = |program: &str, args: &[&str]| {
        let out = std::process::Command::new(program)
            .current_dir(root)
            .args(args)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", &ssh)
            .env("GH_PROMPT_DISABLED", "1")
            .stdin(std::process::Stdio::null())
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    // <temp>/agent-work/<repository>/<task>/ (WORKFLOW.md)
    let name = root
        .canonicalize()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_os_string()));
    let scratch = std::env::temp_dir().join("agent-work").join(name.unwrap_or_default());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());

    let mut facts = local_facts(root, &scratch, now, &git);
    facts.remote = remote(&|args| quiet("git", args), &|args| quiet("gh", args));
    let found = leftovers(&facts);
    if !found.is_empty() {
        println!(
            "Leftovers in this repository (cargo xtask tidy). Delete on your own only what cannot lose work; show the rest to the owner (WORKFLOW.md, Housekeeping):"
        );
        for line in found {
            println!("- {line}");
        }
        if facts.remote.is_none() {
            println!("- GitHub could not be read (gh or the network): its branches and pull requests were not checked");
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean() -> Facts {
        Facts {
            default: "develop".to_string(),
            current: "develop".to_string(),
            branches: Vec::new(),
            worktrees: Vec::new(),
            stashes: 0,
            untracked: Vec::new(),
            links: Vec::new(),
            scratch: Vec::new(),
            remote: None,
        }
    }

    fn branch(name: &str, merged: bool, gone: bool, age_days: u64) -> Branch {
        Branch {
            name: name.to_string(),
            merged,
            gone,
            age_days,
        }
    }

    fn pr(number: u64, branch: &str, idle_days: u64) -> Pr {
        Pr {
            number,
            branch: branch.to_string(),
            idle_days,
            bot: false,
        }
    }

    fn remote(branches: &[&str], prs: Vec<Pr>) -> Option<Remote> {
        Some(Remote {
            branches: branches.iter().map(|name| name.to_string()).collect(),
            prs,
        })
    }

    /// The one leftover of `facts`.
    fn one(facts: &Facts) -> String {
        let all = leftovers(facts);
        assert_eq!(all.len(), 1, "{all:?}");
        all[0].clone()
    }

    fn none(facts: &Facts) {
        assert_eq!(leftovers(facts), Vec::<String>::new());
    }

    #[test]
    fn a_clean_checkout_has_none() {
        none(&clean());
        none(&Facts {
            remote: remote(&["main", "develop", "feature/pr-screenshots"], Vec::new()),
            ..clean()
        });
    }

    #[test]
    fn a_merged_branch() {
        let facts = Facts {
            branches: vec![branch("feature/x", true, false, 3)],
            ..clean()
        };
        assert!(one(&facts).contains("branch feature/x is merged into develop: git branch -d feature/x"));
    }

    #[test]
    fn a_branch_whose_remote_is_gone() {
        let facts = Facts {
            branches: vec![branch("feature/x", false, true, 3)],
            ..clean()
        };
        assert!(one(&facts).contains("branch feature/x lost its remote branch"));
    }

    #[test]
    fn a_branch_just_cut_is_work_in_progress() {
        let facts = Facts {
            current: "feature/x".to_string(),
            branches: vec![branch("feature/x", true, false, 0)],
            ..clean()
        };
        none(&facts);
    }

    #[test]
    fn a_stale_current_branch() {
        let facts = Facts {
            current: "feature/x".to_string(),
            branches: vec![branch("feature/x", true, true, 9)],
            ..clean()
        };
        assert!(one(&facts).contains("branch feature/x is merged into develop"));
    }

    #[test]
    fn a_branch_without_a_pr() {
        let facts = Facts {
            branches: vec![branch("feature/x", false, false, 2)],
            remote: remote(&["main", "develop"], Vec::new()),
            ..clean()
        };
        assert!(one(&facts).contains("branch feature/x has no open PR after 2 days"));
    }

    #[test]
    fn a_young_branch_or_one_with_a_pr() {
        let young = Facts {
            branches: vec![branch("feature/x", false, false, 0)],
            remote: remote(&["main", "develop"], Vec::new()),
            ..clean()
        };
        none(&young);
        let reviewed = Facts {
            branches: vec![branch("feature/x", false, false, 5)],
            remote: remote(&["main", "develop", "feature/x"], vec![pr(7, "feature/x", 1)]),
            ..clean()
        };
        none(&reviewed);
    }

    #[test]
    fn without_github_a_branch_is_not_judged() {
        none(&Facts {
            branches: vec![branch("feature/x", false, false, 9)],
            ..clean()
        });
    }

    #[test]
    fn a_worktree() {
        let worktrees = vec![("/tmp/w".to_string(), "feature/x".to_string())];
        let orphan = Facts {
            worktrees: worktrees.clone(),
            remote: remote(&["develop"], Vec::new()),
            ..clean()
        };
        assert!(one(&orphan).contains("worktree /tmp/w (feature/x) has no open PR"));
        let reviewed = Facts {
            worktrees: worktrees.clone(),
            remote: remote(&["develop", "feature/x"], vec![pr(7, "feature/x", 0)]),
            ..clean()
        };
        none(&reviewed);
        none(&Facts { worktrees, ..clean() });
    }

    #[test]
    fn the_session_in_hand_is_work_in_progress() {
        // the current branch has no PR yet and sits in its own worktree: neither is a leftover
        let facts = Facts {
            current: "feature/x".to_string(),
            branches: vec![branch("feature/x", false, false, 3)],
            worktrees: vec![("/tmp/w".to_string(), "feature/x".to_string())],
            remote: remote(&["develop"], Vec::new()),
            ..clean()
        };
        none(&facts);
    }

    #[test]
    fn a_worktree_cut_today_is_work_in_progress() {
        let facts = Facts {
            branches: vec![branch("feature/y", false, false, 0)],
            worktrees: vec![("/tmp/w".to_string(), "feature/y".to_string())],
            remote: remote(&["develop"], Vec::new()),
            ..clean()
        };
        none(&facts);
    }

    #[test]
    fn a_scratch_folder_of_an_open_pr_is_left_alone() {
        let facts = Facts {
            scratch: vec!["other-task".to_string()],
            remote: remote(&["develop", "feature/other-task"], vec![pr(9, "feature/other-task", 0)]),
            ..clean()
        };
        none(&facts);
    }

    #[test]
    fn a_stale_current_branch_is_told_to_switch_first() {
        let facts = Facts {
            current: "feature/x".to_string(),
            branches: vec![branch("feature/x", true, true, 9)],
            ..clean()
        };
        assert!(one(&facts).contains("switch to develop, then git branch -d feature/x"));
    }

    #[test]
    fn a_stash() {
        assert!(one(&Facts { stashes: 2, ..clean() }).contains("2 stashes: git stash list"));
        assert!(one(&Facts { stashes: 1, ..clean() }).contains("1 stash: git stash list"));
    }

    #[test]
    fn untracked_files_on_the_default_branch() {
        let files = |count: usize| (0..count).map(|i| format!("f{i}.txt")).collect::<Vec<_>>();
        assert!(
            one(&Facts {
                untracked: files(2),
                ..clean()
            })
            .contains("untracked files on develop: f0.txt, f1.txt")
        );
        none(&Facts {
            untracked: files(2),
            current: "feature/x".to_string(),
            ..clean()
        });
        let many = one(&Facts {
            untracked: files(7),
            ..clean()
        });
        assert!(many.contains("f4.txt (+2 more)") && !many.contains("f5.txt"), "{many}");
    }

    #[test]
    fn a_link_inside_the_checkout() {
        let line = one(&Facts {
            links: vec!["before/before/vendor".to_string()],
            ..clean()
        });
        assert!(
            line.contains("link inside the checkout: before/before/vendor"),
            "{line}"
        );
        assert!(line.contains("never a recursive delete"), "{line}");
    }

    #[test]
    fn a_scratch_folder() {
        let scratch = |name: &str| Facts {
            current: "feature/new-task".to_string(),
            scratch: vec![name.to_string()],
            ..clean()
        };
        assert!(one(&scratch("old-task")).contains("scratch folder old-task is not empty"));
        none(&scratch("new-task"));
    }

    #[test]
    fn a_remote_branch_without_a_pr() {
        let parked = Facts {
            remote: remote(&["main", "develop", "feature/parked"], Vec::new()),
            ..clean()
        };
        assert!(one(&parked).contains("remote branch feature/parked has no open PR"));
        let bot = Pr {
            bot: true,
            ..pr(3, "dependabot/npm/x", 20)
        };
        none(&Facts {
            remote: remote(&["develop", "dependabot/npm/x"], vec![bot]),
            ..clean()
        });
    }

    #[test]
    fn an_idle_pr() {
        let with = |pr: Pr| Facts {
            remote: remote(&["develop", &pr.branch.clone()], vec![pr]),
            ..clean()
        };
        assert!(one(&with(pr(12, "feature/x", 7))).contains("PR #12 (feature/x) has not moved for 7 days"));
        none(&with(pr(12, "feature/x", 6)));
        none(&with(Pr {
            bot: true,
            ..pr(13, "feature/bump-cargo-2026-11-17", 30)
        }));
        none(&with(pr(14, "release/v1.0.0-rc.10", 30)));
        none(&with(pr(15, "hotfix/v1.0.0-rc.11", 30)));
    }
}

/// The facts, read from real repositories made in temporary folders and removed after their test.
#[cfg(test)]
mod repository_tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A repository on `develop` with one commit, a bare `origin` beside it, and a scratch folder.
    struct Repo {
        dir: PathBuf,
        origin: PathBuf,
        scratch: PathBuf,
        target: PathBuf,
        worktree: PathBuf,
    }

    impl Repo {
        fn new() -> Repo {
            static COUNT: AtomicUsize = AtomicUsize::new(0);
            let name = format!(
                "grace-xtask-tidy-{}-{}",
                std::process::id(),
                COUNT.fetch_add(1, Ordering::Relaxed)
            );
            let base = std::env::temp_dir().join(name);
            let repo = Repo {
                dir: base.join("repo"),
                origin: base.join("origin.git"),
                scratch: base.join("scratch"),
                target: base.join("target"),
                worktree: base.join("worktree"),
            };
            for dir in [&repo.dir, &repo.scratch, &repo.target] {
                std::fs::create_dir_all(dir).unwrap();
            }
            repo.git(&["init", "-q", "-b", "develop"]);
            repo.write("a.txt", "a\n");
            repo.commit("chore: init");
            repo
        }

        /// Git in this repository, with none of the machine's settings (as in `pr.rs`).
        fn try_git(&self, args: &[&str]) -> Option<String> {
            let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
            let mut git = std::process::Command::new("git");
            git.env("GIT_CONFIG_GLOBAL", null).env("GIT_CONFIG_NOSYSTEM", "1");
            for name in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
                git.env_remove(name);
            }
            let out = git
                .arg("-C")
                .arg(&self.dir)
                .args(["-c", "user.name=t", "-c", "user.email=t@t"])
                .args(args)
                .output()
                .ok()?;
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
        }

        fn git(&self, args: &[&str]) -> String {
            self.try_git(args)
                .unwrap_or_else(|| panic!("git {args:?} failed"))
                .trim()
                .to_string()
        }

        fn write(&self, path: &str, text: &str) {
            let file = self.dir.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }

        fn commit(&self, message: &str) {
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "-m", message]);
        }

        /// Pushes `develop` to a bare `origin` whose default branch it is.
        fn with_origin(&self) {
            let origin = self.origin.to_string_lossy().into_owned();
            self.git(&["init", "-q", "--bare", "-b", "develop", &origin]);
            self.git(&["remote", "add", "origin", &origin]);
            self.git(&["push", "-q", "-u", "origin", "develop"]);
            self.git(&["remote", "set-head", "origin", "develop"]);
        }

        fn facts(&self) -> Facts {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            local_facts(&self.dir, &self.scratch, now, &|args| self.try_git(args))
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            // the link first, alone: nothing here may be deleted through it
            let link = self.dir.join("stray dir").join("inner").join("link");
            let _ = std::fs::remove_dir(&link).or_else(|_| std::fs::remove_file(&link));
            let _ = std::fs::remove_dir_all(self.dir.parent().unwrap());
        }
    }

    #[test]
    fn a_fresh_repository_is_clean() {
        let repo = Repo::new();
        let facts = repo.facts();
        assert_eq!((facts.default.as_str(), facts.current.as_str()), ("develop", "develop"));
        assert_eq!(leftovers(&facts), Vec::<String>::new());
        repo.with_origin();
        assert_eq!(leftovers(&repo.facts()), Vec::<String>::new());
    }

    #[test]
    fn it_finds_what_was_left() {
        let repo = Repo::new();
        repo.git(&["branch", "feature/merged"]);
        repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "feature/w",
            &repo.worktree.to_string_lossy(),
        ]);
        repo.write("a.txt", "changed\n");
        repo.git(&["stash", "-q"]);
        repo.write("loose.txt", "x\n");
        repo.write("loose file.txt", "x\n");
        std::fs::create_dir_all(repo.scratch.join("old-task")).unwrap();
        std::fs::write(repo.scratch.join("old-task").join("log.txt"), "x").unwrap();
        std::fs::create_dir_all(repo.scratch.join("empty-task")).unwrap();
        // a link two levels deep in an untracked folder, as the one found on 2026-11-12
        let inner = repo.dir.join("stray dir").join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        #[cfg(windows)]
        {
            let made = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(inner.join("link"))
                .arg(&repo.target)
                .output()
                .unwrap();
            assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&repo.target, inner.join("link")).unwrap();

        let mut facts = repo.facts();
        let found = leftovers(&facts).join("\n");
        for expected in [
            "branch feature/merged is merged into develop",
            "1 stash: git stash list",
            "untracked files on develop: ",
            "loose.txt",
            "loose file.txt",
            "link inside the checkout: stray dir/inner/link",
            "scratch folder old-task is not empty",
        ] {
            assert!(found.contains(expected), "no \"{expected}\" in:\n{found}");
        }
        assert!(!found.contains("empty-task") && !found.contains("worktree"), "{found}");
        assert_eq!(facts.worktrees.len(), 1, "{:?}", facts.worktrees);
        assert_eq!(facts.worktrees[0].1, "feature/w");
        // with GitHub read and no PR for its branch, the worktree is a leftover too, once it is a day old
        facts.remote = Some(Remote {
            branches: vec!["develop".to_string()],
            prs: Vec::new(),
        });
        assert!(
            !leftovers(&facts).join("\n").contains("worktree"),
            "a worktree cut today is work in progress"
        );
        facts.branches.iter_mut().for_each(|branch| branch.age_days = 2);
        let found = leftovers(&facts).join("\n");
        assert!(found.contains("(feature/w) has no open PR"), "{found}");
        repo.git(&["worktree", "remove", "--force", &repo.worktree.to_string_lossy()]);
    }

    #[test]
    fn a_gone_upstream_is_seen() {
        let repo = Repo::new();
        repo.with_origin();
        repo.git(&["switch", "-q", "-c", "feature/x"]);
        repo.write("b.txt", "b\n");
        repo.commit("docs: add b");
        repo.git(&["push", "-q", "-u", "origin", "feature/x"]);
        repo.git(&["switch", "-q", "develop"]);
        assert_eq!(leftovers(&repo.facts()), Vec::<String>::new());
        repo.git(&["push", "-q", "origin", "--delete", "feature/x"]);
        repo.git(&["fetch", "-q", "--prune"]);
        let found = leftovers(&repo.facts());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("branch feature/x lost its remote branch"),
            "{found:?}"
        );
    }

    #[test]
    fn remote_reads_branches_and_prs() {
        let git = |args: &[&str]| {
            assert_eq!(args, ["ls-remote", "--heads", "origin"]);
            Some("abc\trefs/heads/develop\ndef\trefs/heads/feature/x\n".to_string())
        };
        let gh = |args: &[&str]| {
            assert_eq!(&args[..3], ["pr", "list", "--state"]);
            Some("12\tfeature/x\t8\tfalse\n13\tdependabot/npm/y\t30\ttrue\n14\tfeature/z\t-1\tfalse\n".to_string())
        };
        let remote = remote(&git, &gh).unwrap();
        assert_eq!(remote.branches, ["develop", "feature/x"]);
        assert_eq!(remote.prs.len(), 3);
        assert_eq!((remote.prs[2].number, remote.prs[2].idle_days), (14, 0));
        let first = &remote.prs[0];
        assert_eq!(
            (first.number, first.branch.as_str(), first.idle_days, first.bot),
            (12, "feature/x", 8, false)
        );
        assert!(remote.prs[1].bot);
    }

    #[test]
    fn remote_is_none_when_github_cannot_be_read() {
        let branches = |_: &[&str]| Some("abc\trefs/heads/develop\n".to_string());
        assert!(remote(&branches, &|_| None).is_none());
        assert!(remote(&|_| None, &|_| Some(String::new())).is_none());
    }
}
