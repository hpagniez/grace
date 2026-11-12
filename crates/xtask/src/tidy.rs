//! `cargo xtask tidy`: what was left behind (WORKFLOW.md, Housekeeping).
//!
//! A leftover is something with no open PR behind it: a branch, a worktree, a stash, untracked files on the default
//! branch, a scratch folder, a link inside the checkout. Work in progress is not one. The command only reports: it
//! changes nothing, prints nothing when all is clean, and never fails, so a session-start hook can run it.

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
            lines.push(format!("branch {name} is merged into {default}: git branch -d {name}"));
        } else if branch.gone && !branch.merged {
            lines.push(format!(
                "branch {name} lost its remote branch (its PR was merged or closed): check it holds nothing, then delete it"
            ));
        } else if facts.remote.is_some() && !branch.merged && !open(name) && branch.age_days >= 1 {
            lines.push(format!(
                "branch {name} has no open PR after {} days: open its PR or delete it",
                branch.age_days
            ));
        }
    }
    if facts.remote.is_some() {
        for (path, branch) in facts.worktrees.iter().filter(|(_, branch)| !open(branch)) {
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
    for folder in facts.scratch.iter().filter(|folder| folder.as_str() != task) {
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
