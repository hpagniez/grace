// The two PR checks that judge the bench's JSON: a release's performance record (docs/specs/2026-10-31-light-and-fast.md,
// R5.6) and any raised bench limit (R3.4). Every other PR check is Rust, `cargo xtask check-pr` (crates/xtask/src/pr.rs).
// These two call bench/judge.mjs, so they move to Rust with the bench (docs/specs/2026-11-12-way-of-working.md, section 5).
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { judge, raisedLimits } from "../bench/judge.mjs";

/** Release and hotfix branches are named after their tag (ADR 0009), a release candidate until a stable release is decided (ADR 0015). */
const VERSIONED = /^(release|hotfix)\/v\d+\.\d+\.\d+-rc\.\d+$/;

/**
 * Null unless a PR raises a limit in bench/limits.json (`base` and `head` are its texts, null when the file is
 * missing) without adding a record under docs/decisions/ (`added` lists the files the PR adds).
 */
export function limitRaiseError(base, head, added) {
  let ids;
  try {
    ids = raisedLimits(base && JSON.parse(base), head && JSON.parse(head));
  } catch {
    return "bench/limits.json is not valid JSON";
  }
  if (!ids.length || added.some((f) => f.startsWith("docs/decisions/"))) return null;
  return `bench/limits.json raises ${ids.join(", ")} without a new docs/decisions/ record (.claude/rules/ci.md)`;
}

/**
 * Null unless a release or hotfix PR into main lacks the `## Performance` section of its check record, the
 * bench JSON beside it (`<record name>.bench.json`: release mode, against a previous release, 5 rounds), or a
 * `Result:` line that agrees with that JSON judged against `limits` (the text of bench/limits.json, null when
 * missing). A miss also needs `Decision: go`.
 * `files` are the paths under docs/checks; `read` gives one's text.
 */
export function performanceError(head, base, files, read, limits) {
  if (base !== "main" || !VERSIONED.test(head)) return null;
  const version = head.slice(head.indexOf("/v") + 2);
  const record = files.find((f) => f.endsWith(`-${version}-packaged.md`));
  if (!record) return `${head} has no docs/checks/<date>-<release|hotfix>-${version}-packaged.md`;
  const text = read(record);
  if (!/^## Performance\s*$/m.test(text))
    return `${record} has no "## Performance" section: run npm run bench in release mode and add its table (docs/process/release.md)`;
  const json = record.replace(/\.md$/, ".bench.json");
  if (!files.includes(json)) return `${record} has no ${json}: commit the bench's JSON beside it`;
  const said = /^Result: (pass|miss)\s*$/m.exec(text)?.[1];
  if (!said)
    return `${record} has no "Result: pass" or "Result: miss" line in its Performance section`;
  let report;
  try {
    report = JSON.parse(read(json));
  } catch {
    return `${json} is not valid JSON`;
  }
  // a PR-mode report judges no machine limit, so it would pass nearly whatever it measured
  if (report?.mode !== "release")
    return `${json} is a ${report?.mode}-mode report: run the bench with --release`;
  // with no build a, nothing can read "worse": only the limits would judge the release
  if (!report.builds?.a || !report.medians?.a)
    return `${json} compares with no previous release: run the release bench with --a <previous release exe>`;
  const rounds = (report.rounds ?? []).filter((r) => r.build === "b" && !r.warmup).length;
  if (rounds < 5)
    return `${json} has ${rounds} rounds of the release build after the warm-up, not 5: run the release bench without --rounds`;
  let limitsFile;
  try {
    limitsFile = JSON.parse(limits ?? '{"machine":"","commit":"","measures":{}}');
  } catch {
    return "bench/limits.json is not valid JSON";
  }
  const { result, rows } = judge(report, limitsFile);
  if (said === "pass" && result === "miss") {
    const missed = rows.filter((r) => !["pass", "better", "n/a"].includes(r.verdict));
    return `${record} says "Result: pass" but ${json} misses: ${missed.map((r) => `${r.id} (${r.verdict})`).join(", ")}`;
  }
  if (/^Decision: no-go\s*$/m.test(text))
    return `${record} says "Decision: no-go": the release stops`;
  if (said === "miss" && !/^Decision: go\s*$/m.test(text))
    return `${record} says "Result: miss" without "Decision: go": the owner decides after the ### Analysis (docs/process/release.md)`;
  return null;
}

function main() {
  const { HEAD_REF, BASE_REF, BASE_SHA, HEAD_SHA } = process.env;
  const errors = [];
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
  // a PR raises a limit only through its own changes: compare with where it left the base, not the base's tip
  const fork = git("merge-base", BASE_SHA, HEAD_SHA).trim();
  const added = git("diff", "--name-only", "--diff-filter=A", fork, HEAD_SHA).split("\n");
  for (const err of [
    performanceError(HEAD_REF, BASE_REF, checks, read, limits),
    limitRaiseError(show(fork, "bench/limits.json"), limits, added),
  ])
    if (err) errors.push(err);
  for (const e of errors) console.log(`::error::${e}`);
  process.exit(errors.length ? 1 : 0);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
