import { describe, expect, it } from "vitest";
import {
  BOT_EMAIL,
  branchName,
  openBumpPr,
  prBody,
  runCommandStatus,
  supersedePlan,
  titleFor,
  versionFromBranch,
} from "./bump-pr.mjs";
import { commitError } from "./check-pr.mjs";

const bot = { authors: [{ email: BOT_EMAIL, login: "", name: "github-actions[bot]" }] };
const person = { authors: [{ email: "hugo@example.com", login: "Tusquito", name: "Tusquito" }] };
const pr = (number: number, headRefName: string, isDraft: boolean) => ({
  number,
  headRefName,
  isDraft,
});

describe("branchName", () => {
  it("names the branch after the version, + as _", () => {
    expect(branchName("jre", "21.0.13+11")).toBe("feature/bump-jre-21.0.13_11");
    expect(branchName("karate", "2.1.5")).toBe("feature/bump-karate-2.1.5");
  });

  it("reads the version back only for the same dependency", () => {
    expect(versionFromBranch("jre", "feature/bump-jre-21.0.13_11")).toBe("21.0.13+11");
    expect(versionFromBranch("karate", "feature/bump-jre-21.0.13_11")).toBeNull();
    expect(versionFromBranch("karate", "feature/home-screen")).toBeNull();
  });
});

describe("titleFor", () => {
  it("writes commit headers the PR check accepts", () => {
    expect(titleFor("karate", "2.1.4", "2.1.5")).toBe(
      "build(deps): bump karate from 2.1.4 to 2.1.5",
    );
    expect(commitError(titleFor("karate", "2.1.4", "2.1.5"))).toBeNull();
    expect(titleFor("jre", "21.0.12.1+1", "21.0.13.1+12")).toBe(
      "build(deps): bump temurin jre from 21.0.12.1+1 to 21.0.13.1+12",
    );
    expect(commitError(titleFor("jre", "21.0.12.1+1", "21.0.13.1+12"))).toBeNull();
  });
});

describe("supersedePlan", () => {
  it("closes only older bot-only drafts of the same dependency", () => {
    const prs = [
      pr(1, "feature/bump-karate-2.1.5", true),
      pr(2, "feature/bump-karate-2.1.5", true),
      pr(3, "feature/bump-karate-2.1.4", false),
      pr(4, "feature/bump-karate-2.1.7", true),
      pr(5, "feature/bump-jre-21.0.13_11", true),
      pr(6, "feature/home-screen", true),
      pr(7, "feature/bump-karate-2.1.6", true),
    ];
    const asked: number[] = [];
    const commitsOf = (n: number) => {
      asked.push(n);
      return n === 2 ? [bot, person] : [bot];
    };
    expect(supersedePlan(prs, "karate", "2.1.6", commitsOf)).toEqual({ close: [1], keep: [2, 3] });
    // commits are fetched one PR at a time, and only for older drafts (GitHub's GraphQL node limit)
    expect(asked).toEqual([1, 2]);
  });

  it("compares JRE versions as numbers", () => {
    const prs = [
      pr(1, "feature/bump-jre-21.0.12.1_1", true),
      pr(2, "feature/bump-jre-21.0.9_2", true),
    ];
    expect(supersedePlan(prs, "jre", "21.0.13+11", () => [bot])).toEqual({
      close: [1, 2],
      keep: [],
    });
  });
});

describe("runCommandStatus", () => {
  it("says changed when RunCommand.java is in the list", () => {
    expect(
      runCommandStatus([
        "README.md",
        "karate-core/src/main/java/io/karatelabs/cli/RunCommand.java",
      ]),
    ).toBe("changed");
  });

  it("says unchanged for a short list without it", () => {
    expect(runCommandStatus(["a", "b", "c"])).toBe("unchanged");
  });

  it("says unknown when GitHub's 300-file cap may hide it", () => {
    expect(runCommandStatus(Array.from({ length: 300 }, (_, i) => `f${i}`))).toBe("unknown");
  });
});

/** A fake `git`/`gh` runner: records each call and answers the first matching rule. */
function fakeRun(rules: [RegExp, string | (() => string)][]) {
  const calls: { line: string; input?: string }[] = [];
  const run = (cmd: string, args: string[], input?: string) => {
    const line = [cmd, ...args].join(" ");
    calls.push({ line, input });
    const rule = rules.find(([re]) => re.test(line));
    if (!rule) return "";
    return typeof rule[1] === "function" ? rule[1]() : rule[1];
  };
  return { run, lines: () => calls.map((c) => c.line), calls };
}

describe("openBumpPr", () => {
  const karate = { dep: "karate", from: "2.1.4", to: "2.1.6" };

  it("stops when the version's branch already has a PR", () => {
    const f = fakeRun([[/^gh pr list --head/, '[{"number":5}]']]);
    expect(openBumpPr({ ...karate, check: false }, f.run)).toBe(false);
    expect(f.lines()).toEqual([
      "gh pr list --head feature/bump-karate-2.1.6 --state all --json number",
    ]);
  });

  it("leaves a new Karate branch to the Windows job in check mode", () => {
    const f = fakeRun([[/^gh pr list --head/, "[]"]]);
    expect(openBumpPr({ ...karate, check: true }, f.run)).toBe(true);
    expect(f.lines()).toEqual([
      "gh pr list --head feature/bump-karate-2.1.6 --state all --json number",
      "git ls-remote --heads origin refs/heads/feature/bump-karate-2.1.6",
    ]);
  });

  it("commits, pushes, opens the draft and supersedes the older bot-only draft", () => {
    const f = fakeRun([
      [/^gh pr list --head/, "[]"],
      [
        /^gh pr list --state open/,
        JSON.stringify([
          pr(1, "feature/bump-karate-2.1.5", true),
          pr(9, "feature/home-screen", true),
        ]),
      ],
      [/^gh pr view 1 --json commits/, JSON.stringify({ commits: [bot] })],
      [
        /^git diff --cached --quiet/,
        () => {
          throw new Error("exit 1: staged changes");
        },
      ],
      [/^gh api repos\/karatelabs\/karate\/compare\//, '["README.md"]'],
      [/^gh pr create/, "https://github.com/Tusquito/karate-kid/pull/42\n"],
    ]);
    expect(openBumpPr({ ...karate, check: false }, f.run)).toBe(false);
    const commit = `git -c user.name=github-actions[bot] -c user.email=${BOT_EMAIL} commit -m`;
    expect(f.lines()).toEqual([
      "gh pr list --head feature/bump-karate-2.1.6 --state all --json number",
      "git ls-remote --heads origin refs/heads/feature/bump-karate-2.1.6",
      "git switch -c feature/bump-karate-2.1.6",
      "git add scripts/vendor.json",
      `${commit} build(deps): bump karate from 2.1.4 to 2.1.6`,
      "git add test/fixtures/events",
      "git diff --cached --quiet",
      `${commit} test(fixtures): record events with karate 2.1.6`,
      "git push origin feature/bump-karate-2.1.6",
      "gh pr list --state open --base develop --limit 100 --json number,headRefName,isDraft",
      "gh pr view 1 --json commits",
      "gh api repos/karatelabs/karate/compare/v2.1.4...v2.1.6 --jq [.files[]?.filename]",
      "gh pr create --draft --base develop --head feature/bump-karate-2.1.6 --title build(deps): bump karate from 2.1.4 to 2.1.6 --body-file -",
      "gh pr close 1 --comment Superseded by #42 --delete-branch",
    ]);
    expect(f.calls.find((c) => c.line.startsWith("gh pr create"))?.input).toContain(
      "`RunCommand.java`: unchanged.",
    );
  });

  it("opens the missing PR of a branch already pushed, without committing", () => {
    const f = fakeRun([
      [/^gh pr list --head/, "[]"],
      [/^git ls-remote/, "abc123\trefs/heads/feature/bump-jre-21.0.13_11\n"],
      [/^gh pr list --state open/, "[]"],
      [/^gh pr create/, "https://github.com/Tusquito/karate-kid/pull/43\n"],
    ]);
    expect(
      openBumpPr({ dep: "jre", from: "21.0.12.1+1", to: "21.0.13+11", check: true }, f.run),
    ).toBe(false);
    expect(f.lines().some((l) => /^git (switch|commit|push)|commit -m/.test(l))).toBe(false);
    expect(f.lines().at(-1)).toMatch(
      /^gh pr create --draft .* --head feature\/bump-jre-21.0.13_11 /,
    );
  });
});

describe("prBody", () => {
  it("lists what a Karate reviewer must check", () => {
    const body = prBody({
      dep: "karate",
      from: "2.1.4",
      to: "2.1.5",
      keep: [],
      runCommand: "unchanged",
    });
    expect(body.startsWith("Mark this PR ready for review to run `checks` and `test`")).toBe(true);
    expect(body).toContain("To skip this version, close this PR.");
    expect(body).toContain("`gh workflow run test.yml --ref feature/bump-karate-2.1.5`");
    expect(body).toContain("https://github.com/karatelabs/karate/releases/tag/v2.1.5");
    expect(body).toContain("https://github.com/karatelabs/karate/compare/v2.1.4...v2.1.5");
    expect(body).toContain("`RunCommand.java`: unchanged.");
    expect(body).toContain("Fixtures recorded again with Karate 2.1.5");
    expect(body).toContain("`RESERVED_KARATE_KEYS` in `src/shared/env.ts`");
    expect(body).not.toContain("JRE:");
    expect(body).not.toContain("still open");
  });

  it("says unknown, check by hand, and names kept PRs", () => {
    const body = prBody({
      dep: "karate",
      from: "2.1.4",
      to: "2.1.6",
      keep: [7],
      runCommand: "unknown",
    });
    expect(body).toContain("`RunCommand.java`: unknown, check by hand.");
    expect(
      body
        .trimEnd()
        .endsWith("#7 is still open and has changes by people: carry them over or close it."),
    ).toBe(true);
  });

  it("links the Adoptium release for a JRE bump", () => {
    const body = prBody({ dep: "jre", from: "21.0.12.1+1", to: "21.0.13+11", keep: [] });
    expect(body).toContain(
      "https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-21.0.13%2B11",
    );
    expect(body).not.toContain("Karate:");
  });
});
