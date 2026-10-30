// Checks a PR against AGENTS.md: Conventional Commits headers (ADR 0003), git flow branch targets,
// and a release's check of the public docs.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

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

function main() {
  const { HEAD_REF, BASE_REF, BASE_SHA, HEAD_SHA } = process.env;
  const errors = [];
  const branch = branchError(HEAD_REF, BASE_REF);
  if (branch) errors.push(branch);
  const git = (...args) => execFileSync("git", args, { encoding: "utf8" });
  const docs = publicDocsError(
    HEAD_REF,
    BASE_REF,
    git("ls-tree", "--name-only", HEAD_SHA, "docs/checks/").split("\n").filter(Boolean),
    (f) => git("show", `${HEAD_SHA}:${f}`),
  );
  if (docs) errors.push(docs);
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
