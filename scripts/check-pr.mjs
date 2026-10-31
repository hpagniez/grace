// Checks a PR against AGENTS.md: Conventional Commits headers (ADR 0003), git flow branch targets,
// a release's check of the public docs, its performance record (docs/specs/2026-10-31-light-and-fast.md, R5.6)
// and any raised bench limit (R3.4).
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { judge, raisedLimits } from "../bench/judge.mjs";

const HEADER =
  /^(feat|fix|docs|refactor|perf|test|build|ci|chore|revert)(\([a-z0-9-]+\))?!?: (\S.*)$/;

/** Null when `header` (a commit's first line) follows the rules, else the reason. */
export function commitError(header) {
  const m = HEADER.exec(header);
  if (!m) return "not `<type>(<scope>)<!>: <subject>` with a known type and a lower-case scope";
  const subject = m[3];
  if (/^[A-Z]/.test(subject)) return "subject starts with an upper-case letter";
  if (subject.endsWith(".")) return "subject ends with a period";
  if (header.length > 72) return `header is ${header.length} characters, max 72`;
  return null;
}

const TARGETS = {
  feature: ["develop"],
  dependabot: ["develop"],
  release: ["main", "develop"],
  hotfix: ["main", "develop"],
};

/** Release and hotfix branches are named after their tag (ADR 0009), a release candidate until a stable release is decided (ADR 0015). */
const VERSIONED = /^(release|hotfix)\/v\d+\.\d+\.\d+-rc\.\d+$/;

/** Null when a PR from `head` may target `base` under git flow, else the reason. */
export function branchError(head, base) {
  const kind = head.split("/")[0];
  if ((kind === "release" || kind === "hotfix") && !VERSIONED.test(head))
    return `${head} must be named ${kind}/vX.Y.Z-rc.N, after its tag`;
  const allowed = TARGETS[kind];
  if (head.includes("/") && allowed?.includes(base)) return null;
  return `${head} may not target ${base}: feature/* and dependabot/* go to develop, release/vX.Y.Z-rc.N and hotfix/vX.Y.Z-rc.N to main or develop`;
}

/**
 * Null unless a release or hotfix PR into main lacks its packaged-app check record or the record's
 * `## Public docs` section (AGENTS.md). `files` are the paths under docs/checks; `read` gives one's text.
 */
export function publicDocsError(head, base, files, read) {
  if (base !== "main" || !VERSIONED.test(head)) return null;
  const version = head.slice(head.indexOf("/v") + 2);
  const record = files.find((f) => f.endsWith(`-${version}-packaged.md`));
  if (!record) return `${head} has no docs/checks/<date>-<release|hotfix>-${version}-packaged.md`;
  if (!/^## Public docs\s*$/m.test(read(record)))
    return `${record} has no "## Public docs" section: check README.md and its screenshots against ${version} (AGENTS.md)`;
  return null;
}

/**
 * Null unless a PR raises a limit in bench/limits.json (`base` and `head` are its texts, null when the file is
 * missing) without adding a record under docs/decisions/ (`added` lists the files the PR adds).
 */
export function limitRaiseError(base, head, added) {
  const ids = raisedLimits(base && JSON.parse(base), head && JSON.parse(head));
  if (!ids.length || added.some((f) => f.startsWith("docs/decisions/"))) return null;
  return `bench/limits.json raises ${ids.join(", ")} without a new docs/decisions/ record (AGENTS.md)`;
}

/**
 * Null unless a release or hotfix PR into main lacks the `## Performance` section of its check record, the
 * bench JSON beside it (`<record name>.bench.json`), or a `Result:` line that agrees with that JSON judged
 * against `limits` (the text of bench/limits.json, null when missing). A miss also needs `Decision: go`.
 * `files` are the paths under docs/checks; `read` gives one's text.
 */
export function performanceError(head, base, files, read, limits) {
  if (base !== "main" || !VERSIONED.test(head)) return null;
  const version = head.slice(head.indexOf("/v") + 2);
  const record = files.find((f) => f.endsWith(`-${version}-packaged.md`));
  if (!record) return `${head} has no docs/checks/<date>-<release|hotfix>-${version}-packaged.md`;
  const text = read(record);
  if (!/^## Performance\s*$/m.test(text))
    return `${record} has no "## Performance" section: run npm run bench in release mode and add its table (AGENTS.md)`;
  const json = record.replace(/\.md$/, ".bench.json");
  if (!files.includes(json)) return `${record} has no ${json}: commit the bench's JSON beside it`;
  const said = /^Result: (pass|miss)\s*$/m.exec(text)?.[1];
  if (!said)
    return `${record} has no "Result: pass" or "Result: miss" line in its Performance section`;
  const { result, rows } = judge(
    JSON.parse(read(json)),
    JSON.parse(limits ?? '{"machine":"","commit":"","measures":{}}'),
  );
  if (said === "pass" && result === "miss") {
    const missed = rows.filter((r) => !["pass", "better", "n/a"].includes(r.verdict));
    return `${record} says "Result: pass" but ${json} misses: ${missed.map((r) => `${r.id} (${r.verdict})`).join(", ")}`;
  }
  if (/^Decision: no-go\s*$/m.test(text))
    return `${record} says "Decision: no-go": the release stops`;
  if (said === "miss" && !/^Decision: go\s*$/m.test(text))
    return `${record} says "Result: miss" without "Decision: go": the owner decides after the ### Analysis (AGENTS.md)`;
  return null;
}

function main() {
  const { HEAD_REF, BASE_REF, BASE_SHA, HEAD_SHA } = process.env;
  const errors = [];
  const branch = branchError(HEAD_REF, BASE_REF);
  if (branch) errors.push(branch);
  const git = (...args) => execFileSync("git", args, { encoding: "utf8" });
  const show = (sha, f) => {
    try {
      return execFileSync("git", ["show", `${sha}:${f}`], { encoding: "utf8", stdio: "pipe" });
    } catch {
      return null;
    }
  };
  const checks = git("ls-tree", "--name-only", HEAD_SHA, "docs/checks/")
    .split("\n")
    .filter(Boolean);
  const read = (f) => git("show", `${HEAD_SHA}:${f}`);
  const limits = show(HEAD_SHA, "bench/limits.json");
  const added = git("diff", "--name-only", "--diff-filter=A", BASE_SHA, HEAD_SHA).split("\n");
  for (const err of [
    publicDocsError(HEAD_REF, BASE_REF, checks, read),
    performanceError(HEAD_REF, BASE_REF, checks, read, limits),
    limitRaiseError(show(BASE_SHA, "bench/limits.json"), limits, added),
  ])
    if (err) errors.push(err);
  // Dependabot writes its own commit headers, often over 72 characters
  if (!HEAD_REF.startsWith("dependabot/")) {
    // commits already on develop were checked in their own PR; a release PR into main carries them all
    const log = execFileSync(
      "git",
      ["log", "--no-merges", "--format=%s", HEAD_SHA, "--not", BASE_SHA, "origin/develop"],
      { encoding: "utf8" },
    );
    for (const header of log.split("\n").filter(Boolean)) {
      const err = commitError(header);
      if (err) errors.push(`commit "${header}": ${err}`);
    }
  }
  for (const e of errors) console.log(`::error::${e}`);
  process.exit(errors.length ? 1 : 0);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
