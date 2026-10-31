import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  branchError,
  commitError,
  limitRaiseError,
  performanceError,
  publicDocsError,
} from "./check-pr.mjs";

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

const limits = (limit: number) =>
  JSON.stringify({
    machine: "test",
    commit: "abc",
    measures: { "idle.memory": { unit: "MB", limit, noise: 5, machine: true } },
  });

describe("limitRaiseError", () => {
  it("names a raised limit that adds no decision record", () => {
    expect(limitRaiseError(limits(95), limits(120), ["README.md"])).toContain("idle.memory");
  });

  it("accepts a raised limit with a new decision record", () => {
    expect(limitRaiseError(limits(95), limits(120), ["docs/decisions/0024-x.md"])).toBeNull();
  });

  it("accepts a lowered limit", () => {
    expect(limitRaiseError(limits(95), limits(80), [])).toBeNull();
  });

  it("accepts a missing base file", () => {
    expect(limitRaiseError(null, limits(120), [])).toBeNull();
  });

  it("names the file when it is not valid JSON", () => {
    expect(limitRaiseError(limits(95), "{", [])).toBe("bench/limits.json is not valid JSON");
  });
});

/** A bench report in the shape bench/judge.mjs documents, with build b's idle memory at `memory` MB. */
const bench = (memory: number, mode = "release") =>
  JSON.stringify({
    mode,
    machine: "test",
    builds: { a: null, b: { label: "b", exe: "b.exe" } },
    rounds: [],
    medians: { a: null, b: { "idle.memory": memory } },
  });

/** A release check record whose Performance section holds `lines`. */
const performance = (...lines: string[]) => `# Check\n\n## Performance\n\n${lines.join("\n")}\n`;

describe("performanceError", () => {
  const release = "release/v1.0.0-rc.7";
  const record = "docs/checks/2026-10-30-release-1.0.0-rc.7-packaged.md";
  const json = "docs/checks/2026-10-30-release-1.0.0-rc.7-packaged.bench.json";
  const check = (md: string, report: string | null = bench(80), head = release, base = "main") =>
    performanceError(
      head,
      base,
      report === null ? [record] : [record, json],
      (f: string) => (f === record ? md : (report ?? "")),
      limits(95),
    );
  const missed = ["Result: miss", "", "### Analysis", "", "idle.memory grew."];

  it("skips a feature PR into develop", () => {
    expect(check("# Check\n", null, "feature/x", "develop")).toBeNull();
  });

  it("accepts Result: pass over a JSON that passes", () => {
    expect(check(performance("Result: pass"))).toBeNull();
  });

  it("accepts Result: miss with an analysis and Decision: go", () => {
    expect(check(performance(...missed, "", "Decision: go"), bench(120))).toBeNull();
  });

  it("accepts Result: miss over a JSON that passes: the conservative side", () => {
    expect(check(performance("Result: miss", "", "Decision: go"))).toBeNull();
  });

  it("accepts Windows line endings", () => {
    expect(check(performance("Result: pass").replaceAll("\n", "\r\n"))).toBeNull();
  });

  it("rejects a record with no Performance section", () => {
    expect(check("# Check\n\nResult: pass\n")).toContain('"## Performance"');
  });

  it("rejects a record with no bench JSON beside it", () => {
    expect(check(performance("Result: pass"), null)).toContain(json);
  });

  it("rejects a record with no Result line", () => {
    expect(check(performance("It went fine."))).toContain("Result:");
  });

  it("rejects Result: pass over a JSON that misses, and names the measure", () => {
    expect(check(performance("Result: pass"), bench(120))).toContain("idle.memory");
  });

  it("rejects Result: miss with no Decision: go", () => {
    expect(check(performance(...missed), bench(120))).toContain("Decision: go");
  });

  it("rejects Decision: no-go", () => {
    expect(check(performance(...missed, "", "Decision: no-go"), bench(120))).toContain("no-go");
  });

  it("rejects a PR-mode report, which judges no machine limit", () => {
    expect(check(performance("Result: pass"), bench(999, "pr"))).toBe(
      `${json} is a pr-mode report: run the bench with --release`,
    );
  });

  it("names the bench JSON when it is not valid JSON", () => {
    expect(check(performance("Result: pass"), "{")).toBe(`${json} is not valid JSON`);
  });

  it("names the limits file when it is not valid JSON", () => {
    const md = performance("Result: pass");
    const read = (f: string) => (f === record ? md : bench(80));
    expect(performanceError(release, "main", [record, json], read, "{")).toBe(
      "bench/limits.json is not valid JSON",
    );
  });

  it("judges against no limits when the head has no limits file", () => {
    const md = performance("Result: pass");
    const read = (f: string) => (f === record ? md : bench(999));
    expect(performanceError(release, "main", [record, json], read, null)).toBeNull();
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

  const run = (baseSha = base) =>
    spawnSync(process.execPath, [join(__dirname, "check-pr.mjs")], {
      cwd: repo,
      encoding: "utf8",
      env: {
        ...process.env,
        HEAD_REF: "release/v1.0.0-rc.1",
        BASE_REF: "main",
        BASE_SHA: baseSha,
        HEAD_SHA: git("rev-parse", "HEAD"),
      },
    });

  it("fails a release into main whose check record has no Public docs section", () => {
    const r = run();
    expect(r.stdout).toContain(`${record} has no "## Public docs" section`);
    expect(r.status).toBe(1);
  });

  it("fails a release into main whose check record has no Performance section", () => {
    writeFileSync(join(repo, record), "# Check\n\n## Public docs\n\nREADME.md: unchanged.\n");
    git("commit", "-q", "-am", "docs: record the public docs check");
    const r = run();
    expect(r.stdout).toContain(`${record} has no "## Performance" section`);
    expect(r.stdout).not.toContain("Public docs");
    expect(r.status).toBe(1);
  });

  it("skips commits already on develop", () => {
    writeFileSync(
      join(repo, record),
      `${readFileSync(join(repo, record), "utf8")}\n## Performance\n\nResult: pass\n`,
    );
    writeFileSync(join(repo, record.replace(/\.md$/, ".bench.json")), bench(80));
    git("add", ".");
    git("commit", "-q", "-m", "docs: record the performance check");
    const r = run();
    expect(r.stdout).toBe("");
    expect(r.status).toBe(0);
  });

  it("ignores a limit that develop lowered after the branch was cut", () => {
    mkdirSync(join(repo, "bench"));
    writeFileSync(join(repo, "bench", "limits.json"), limits(120));
    git("add", ".");
    git("commit", "-q", "-m", "build(bench): limit idle memory");
    git("switch", "-q", "-c", "develop-lowered");
    writeFileSync(join(repo, "bench", "limits.json"), limits(80));
    git("commit", "-q", "-am", "build(bench): lower the idle memory limit");
    const lowered = git("rev-parse", "HEAD");
    git("switch", "-q", "release/v1.0.0-rc.1");
    git("commit", "-q", "--allow-empty", "-m", "docs: note the limit");
    const r = run(lowered);
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

  it("fails a PR that raises a limit without a decision record", () => {
    writeFileSync(join(repo, "bench", "limits.json"), limits(95));
    git("commit", "-q", "-am", "build(bench): tighten the idle memory limit");
    const before = git("rev-parse", "HEAD");
    writeFileSync(join(repo, "bench", "limits.json"), limits(120));
    git("commit", "-q", "-am", "build(bench): raise the idle memory limit");
    expect(run(before).stdout).toContain("bench/limits.json raises idle.memory");
    mkdirSync(join(repo, "docs", "decisions"));
    writeFileSync(join(repo, "docs", "decisions", "0024-x.md"), "# 0024\n");
    git("add", ".");
    git("commit", "-q", "-m", "docs(decisions): allow more idle memory");
    expect(run(before).stdout).not.toContain("raises");
  });
});
