import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { branchError, commitError } from "./check-pr.mjs";

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
    ["release/1.0.0", "main"],
    ["release/1.0.0", "develop"],
    ["hotfix/1.0.1", "main"],
    ["hotfix/1.0.1", "develop"],
  ])("accepts %s into %s", (head, base) => {
    expect(branchError(head, base)).toBeNull();
  });

  it.each([
    ["feature/x", "main"],
    ["develop", "main"],
    ["fix-thing", "develop"],
  ])("rejects %s into %s", (head, base) => {
    expect(branchError(head, base)).toEqual(expect.any(String));
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
  git("switch", "-q", "-c", "release/1.0.0");
  git("commit", "-q", "--allow-empty", "-m", "build: bump version to 1.0.0");

  const run = () =>
    spawnSync(process.execPath, [join(__dirname, "check-pr.mjs")], {
      cwd: repo,
      encoding: "utf8",
      env: {
        ...process.env,
        HEAD_REF: "release/1.0.0",
        BASE_REF: "main",
        BASE_SHA: base,
        HEAD_SHA: git("rev-parse", "HEAD"),
      },
    });

  it("skips commits already on develop", () => {
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
