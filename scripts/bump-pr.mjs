// CI side of vendor-updates.yml: commits a bumped pin, opens its draft PR and supersedes older bot-only ones (ADR 0012).
import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { compareVersions, parseJreVersion, parseKarateVersion } from "./bump-vendor.mjs";

export const BOT_EMAIL = "41898282+github-actions[bot]@users.noreply.github.com";

const PARSE = { jre: parseJreVersion, karate: parseKarateVersion };

/** `feature/bump-<dep>-<version>`, with `+` as `_`. */
export function branchName(dep, version) {
  return `feature/bump-${dep}-${version.replace("+", "_")}`;
}

/** The version a bump branch of `dep` names, or null for any other branch. */
export function versionFromBranch(dep, branch) {
  const prefix = `feature/bump-${dep}-`;
  return branch.startsWith(prefix) ? branch.slice(prefix.length).replace("_", "+") : null;
}

export function titleFor(dep, from, to) {
  return `build(deps): bump ${dep === "jre" ? "temurin jre" : "karate"} from ${from} to ${to}`;
}

/**
 * Older open bump PRs of `dep`: bot-only drafts to close, the others to name in the new PR.
 * `commitsOf(number)` is asked only for older drafts: listing every PR's commit authors at once exceeds GitHub's GraphQL node limit.
 */
export function supersedePlan(prs, dep, to, commitsOf) {
  const parse = PARSE[dep];
  const target = parse(to);
  const close = [];
  const keep = [];
  for (const pr of prs) {
    const v = parse(versionFromBranch(dep, pr.headRefName) ?? "");
    if (!v || compareVersions(v, target) >= 0) continue;
    const botOnly =
      pr.isDraft &&
      commitsOf(pr.number).every((c) => c.authors.every((a) => a.email === BOT_EMAIL));
    (botOnly ? close : keep).push(pr.number);
  }
  return { close, keep };
}

/** Whether Karate's RunCommand.java is among the file names of a GitHub compare, which lists at most 300. */
export function runCommandStatus(filenames) {
  if (filenames.some((f) => f.endsWith("/RunCommand.java"))) return "changed";
  return filenames.length >= 300 ? "unknown" : "unchanged";
}

/** @param {{ dep: string, from: string, to: string, keep: number[], runCommand?: string }} o */
export function prBody({ dep, from, to, keep, runCommand }) {
  const parts = [
    "Mark this PR ready for review to run `checks` and `test` (e2e included) on the new version. To skip this version, close this PR.",
  ];
  if (dep === "karate") {
    const status = runCommand === "unknown" ? "unknown, check by hand" : runCommand;
    parts.push(
      [
        "Karate:",
        `- Release notes: https://github.com/karatelabs/karate/releases/tag/v${to}`,
        `- Changes: https://github.com/karatelabs/karate/compare/v${from}...v${to}`,
        `- \`RunCommand.java\`: ${status}. Check the flags in AGENTS.md (Karate integration rules) against it.`,
        `- Fixtures recorded again with Karate ${to} (second commit; time stamps, durations and ports always differ). \`src/main/karate/http.test.ts\` checks the HTTP calls (F11).`,
        "- Check that `RESERVED_KARATE_KEYS` in `src/shared/env.ts` still lists every system property Karate reads.",
      ].join("\n"),
    );
  } else {
    const tag = `jdk-${to}`.replace("+", "%2B");
    parts.push(
      `JRE:\n- Release: https://github.com/adoptium/temurin21-binaries/releases/tag/${tag}`,
    );
  }
  if (keep.length)
    parts.push(
      keep
        .map((n) => `#${n} is still open and has changes by people: carry them over or close it.`)
        .join("\n"),
    );
  return parts.join("\n\n") + "\n";
}

/** Runs `git` or `gh` without a shell and returns stdout; throws on a non-zero exit. */
const execRun = (cmd, args, input) =>
  execFileSync(cmd, args, { encoding: "utf8", input, maxBuffer: 64 << 20 });

/**
 * Opens the draft PR for `dep` moving from `from` to `to`, committing and pushing the branch first unless it exists.
 * Returns true only in check mode, when the branch is new and the Windows job must record the fixtures.
 * @param {{ dep: string, from: string, to: string, check: boolean }} o
 * @param {(cmd: string, args: string[], input?: string) => string} run
 */
export function openBumpPr({ dep, from, to, check }, run = execRun) {
  const gh = (...args) => JSON.parse(run("gh", args));
  const commit = (message) =>
    run("git", [
      "-c",
      "user.name=github-actions[bot]",
      "-c",
      `user.email=${BOT_EMAIL}`,
      "commit",
      "-m",
      message,
    ]);
  const branch = branchName(dep, to);
  const prs = gh("pr", "list", "--head", branch, "--state", "all", "--json", "number");
  if (prs.length) {
    console.log(`${branch}: already has a PR (#${prs[0].number})`);
    return false;
  }
  const exists =
    run("git", ["ls-remote", "--heads", "origin", `refs/heads/${branch}`]).trim() !== "";
  if (check && !exists) {
    console.log(`${branch}: new, the Windows job records the fixtures`);
    return true;
  }
  if (!exists) {
    run("git", ["switch", "-c", branch]);
    run("git", ["add", "scripts/vendor.json"]);
    commit(titleFor(dep, from, to));
    if (dep === "karate") {
      run("git", ["add", "test/fixtures/events"]);
      try {
        run("git", ["diff", "--cached", "--quiet"]);
      } catch {
        commit(`test(fixtures): record events with karate ${to}`);
      }
    }
    run("git", ["push", "origin", branch]);
  }
  const open = gh(
    "pr",
    "list",
    "--state",
    "open",
    "--base",
    "develop",
    "--limit",
    "100",
    "--json",
    "number,headRefName,isDraft",
  );
  const { close, keep } = supersedePlan(
    open,
    dep,
    to,
    (n) => gh("pr", "view", String(n), "--json", "commits").commits,
  );
  // file names only: the full compare is over 1 MB
  const runCommand =
    dep === "karate"
      ? runCommandStatus(
          gh(
            "api",
            `repos/karatelabs/karate/compare/v${from}...v${to}`,
            "--jq",
            "[.files[]?.filename]",
          ),
        )
      : undefined;
  const body = prBody({ dep, from, to, keep, runCommand });
  const url = run(
    "gh",
    [
      "pr",
      "create",
      "--draft",
      "--base",
      "develop",
      "--head",
      branch,
      "--title",
      titleFor(dep, from, to),
      "--body-file",
      "-",
    ],
    body,
  ).trim();
  console.log(`opened ${url}`);
  const number = url.split("/").pop();
  for (const n of close)
    run("gh", [
      "pr",
      "close",
      String(n),
      "--comment",
      `Superseded by #${number}`,
      "--delete-branch",
    ]);
  return false;
}

function main() {
  const [dep, from, to, flag] = process.argv.slice(2);
  if (!PARSE[dep] || !from || !to || (flag && flag !== "--check")) {
    console.error("usage: node scripts/bump-pr.mjs <jre|karate> <from> <to> [--check]");
    process.exit(2);
  }
  const todo = openBumpPr({ dep, from, to, check: flag === "--check" });
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `todo=${todo}\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
