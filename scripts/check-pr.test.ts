import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { branchError, commitError, publicDocsError } from "./check-pr.mjs";

describe("commitError", () => {
  it.each([
    "docs(agents): add subagent model selection rule (#5)",
    "feat(runner)!: drop java 17",
    "ci: add checks workflow",
  ])("accepts %s", (header) => {
    expect(commitError(header)).toBeNull();
  });

  it.each([
    ["no type", "Add thing"],
    ["unknown type", "feature: x"],
    ["upper-case scope", "fix(Runner): x"],
    ["upper-case subject", "fix: Add x"],
    ["trailing period", "fix: add x."],
    ["no space after colon", "fix:add x"],
    ["73 characters", `fix: ${"a".repeat(68)}`],
  ])("rejects %s", (_, header) => {
    expect(commitError(header)).toEqual(expect.any(String));
  });
});

describe("branchError", () => {
  it.each([
    ["feature/ci-cd", "develop"],
    ["dependabot/npm_and_yarn/x", "develop"],
    ["release/v1.0.0-rc.1", "main"],
    ["release/v1.0.0-rc.1", "develop"],
    ["hotfix/v1.0.1-rc.1", "main"],
    ["hotfix/v1.0.1-rc.1", "develop"],
    ["release/v10.20.30-rc.12", "main"],
  ])("accepts %s into %s", (head, base) => {
    expect(branchError(head, base)).toBeNull();
  });

  it.each([
    ["feature/x", "main"],
    ["develop", "main"],
    ["fix-thing", "develop"],
    ["release/1.0.0-rc.1", "main"],
    ["hotfix/1.0.1-rc.1", "main"],
    ["release/v1.0.0", "main"],
    ["hotfix/v1.0.1", "develop"],
    ["release/v1.0-rc.1", "main"],
    ["release/v1.0.0-rc", "main"],
    ["release/v1.0.0-rc1", "main"],
    ["release/v1.0.0-beta.1", "main"],
    ["hotfix/latest", "develop"],
  ])("rejects %s into %s", (head, base) => {
    expect(branchError(head, base)).toEqual(expect.any(String));
  });
});

describe("publicDocsError", () => {
  const record = "docs/checks/2026-10-30-release-1.0.0-rc.7-packaged.md";
  const docs = "# Check\n\n## Public docs\n\nREADME.md: install steps updated.\n";
  const read = (text: string) => () => text;

  it("accepts a release record with a Public docs section", () => {
    expect(publicDocsError("release/v1.0.0-rc.7", "main", [record], read(docs))).toBeNull();
  });

  it("accepts a hotfix record, with Windows line endings", () => {
    const hotfix = "docs/checks/2026-10-31-hotfix-1.0.0-rc.8-packaged.md";
    const crlf = docs.replaceAll("\n", "\r\n");
    expect(publicDocsError("hotfix/v1.0.0-rc.8", "main", [hotfix], read(crlf))).toBeNull();
  });

  it.each([
    ["feature/x", "develop"],
    ["release/v1.0.0-rc.7", "develop"],
    ["hotfix/v1.0.0-rc.7", "develop"],
  ])("skips %s into %s", (head, base) => {
    expect(publicDocsError(head, base, [], read(""))).toBeNull();
  });

  it("rejects a release without its record", () => {
    const older = "docs/checks/2026-10-29-release-1.0.0-rc.6-packaged.md";
    expect(publicDocsError("release/v1.0.0-rc.7", "main", [older], read(docs))).toContain(
      "-1.0.0-rc.7-packaged.md",
    );
  });

  it("rejects a record without the section", () => {
    const text = "# Check\n\nPublic docs were fine.\n### Public docs\n";
    expect(publicDocsError("release/v1.0.0-rc.7", "main", [record], read(text))).toContain(
      '"## Public docs"',
    );
  });
});

describe("check-pr CLI", () => {
  // main: init; develop: a bad header that already landed; release branch: its own commits
  const repo = mkdtempSync(join(tmpdir(), "kk-check-pr-"));
  const git = (...args: string[]) =>
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], {
      cwd: repo,
      encoding: "utf8",
    }).trim();
  git("init", "-q", "-b", "main");
  git("commit", "-q", "--allow-empty", "-m", "chore: init");
  const base = git("rev-parse", "HEAD");
  git("commit", "-q", "--allow-empty", "-m", "Merge pull request #4 from x/y");
  git("update-ref", "refs/remotes/origin/develop", "HEAD");
  git("switch", "-q", "-c", "release/v1.0.0-rc.1");
  git("commit", "-q", "--allow-empty", "-m", "build: bump version to 1.0.0-rc.1");
  mkdirSync(join(repo, "docs", "checks"), { recursive: true });
  const record = "docs/checks/2026-10-30-release-1.0.0-rc.1-packaged.md";
  writeFileSync(join(repo, record), "# Check\n");
  git("add", ".");
  git("commit", "-q", "-m", "docs: record the 1.0.0-rc.1 packaged-app check");

  const run = () =>
    spawnSync(process.execPath, [join(__dirname, "check-pr.mjs")], {
      cwd: repo,
      encoding: "utf8",
      env: {
        ...process.env,
        HEAD_REF: "release/v1.0.0-rc.1",
        BASE_REF: "main",
        BASE_SHA: base,
        HEAD_SHA: git("rev-parse", "HEAD"),
      },
    });

  it("fails a release into main whose check record has no Public docs section", () => {
    const r = run();
    expect(r.stdout).toContain(`${record} has no "## Public docs" section`);
    expect(r.status).toBe(1);
  });

  it("skips commits already on develop", () => {
    writeFileSync(join(repo, record), "# Check\n\n## Public docs\n\nREADME.md: unchanged.\n");
    git("commit", "-q", "-am", "docs: record the public docs check");
    const r = run();
    expect(r.stdout).toBe("");
    expect(r.status).toBe(0);
  });

  it("still checks commits only on the PR branch", () => {
    git("commit", "-q", "--allow-empty", "-m", "Bad header");
    const r = run();
    expect(r.stdout).toContain('commit "Bad header"');
    expect(r.stdout).not.toContain("Merge pull request");
    expect(r.status).toBe(1);
  });
});
