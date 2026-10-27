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

/** Older open bump PRs of `dep`: bot-only drafts to close, the others to name in the new PR. */
export function supersedePlan(prs, dep, to) {
  const parse = PARSE[dep];
  const target = parse(to);
  const close = [];
  const keep = [];
  for (const pr of prs) {
    const v = parse(versionFromBranch(dep, pr.headRefName) ?? "");
    if (!v || compareVersions(v, target) >= 0) continue;
    const botOnly = pr.commits.every((c) => c.authors.every((a) => a.email === BOT_EMAIL));
    (pr.isDraft && botOnly ? close : keep).push(pr.number);
  }
  return { close, keep };
}

/** Whether Karate's RunCommand.java is among the files of a GitHub compare, which lists at most 300. */
export function runCommandStatus(files) {
  if (files.some((f) => f.filename.endsWith("/RunCommand.java"))) return "changed";
  return files.length >= 300 ? "unknown" : "unchanged";
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

const run = (cmd, args, input) => execFileSync(cmd, args, { encoding: "utf8", input });
const gitCommit = (message) =>
  run("git", [
    "-c",
    "user.name=github-actions[bot]",
    "-c",
    `user.email=${BOT_EMAIL}`,
    "commit",
    "-m",
    message,
  ]);

function output(todo) {
  if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `todo=${todo}\n`);
}

function main() {
  const [dep, from, to, flag] = process.argv.slice(2);
  if (!PARSE[dep] || !from || !to || (flag && flag !== "--check")) {
    console.error("usage: node scripts/bump-pr.mjs <jre|karate> <from> <to> [--check]");
    process.exit(2);
  }
  const branch = branchName(dep, to);
  const prs = JSON.parse(
    run("gh", ["pr", "list", "--head", branch, "--state", "all", "--json", "number"]),
  );
  if (prs.length) {
    console.log(`${branch}: already has a PR (#${prs[0].number})`);
    return output(false);
  }
  const exists =
    run("git", ["ls-remote", "--heads", "origin", `refs/heads/${branch}`]).trim() !== "";
  if (flag === "--check" && !exists) {
    console.log(`${branch}: new, the Windows job records the fixtures`);
    return output(true);
  }
  if (!exists) {
    run("git", ["switch", "-c", branch]);
    run("git", ["add", "scripts/vendor.json"]);
    gitCommit(titleFor(dep, from, to));
    if (dep === "karate") {
      run("git", ["add", "test/fixtures/events"]);
      try {
        run("git", ["diff", "--cached", "--quiet"]);
      } catch {
        gitCommit(`test(fixtures): record events with karate ${to}`);
      }
    }
    run("git", ["push", "origin", branch]);
  }
  const open = JSON.parse(
    run("gh", [
      "pr",
      "list",
      "--state",
      "open",
      "--base",
      "develop",
      "--limit",
      "100",
      "--json",
      "number,headRefName,isDraft,commits",
    ]),
  );
  const { close, keep } = supersedePlan(open, dep, to);
  let runCommand;
  if (dep === "karate") {
    const compare = JSON.parse(
      run("gh", ["api", `repos/karatelabs/karate/compare/v${from}...v${to}`]),
    );
    runCommand = runCommandStatus(compare.files ?? []);
  }
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
  output(false);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
}
