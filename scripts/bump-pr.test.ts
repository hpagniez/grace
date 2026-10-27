import { describe, expect, it } from "vitest";
import {
  BOT_EMAIL,
  branchName,
  prBody,
  runCommandStatus,
  supersedePlan,
  titleFor,
  versionFromBranch,
} from "./bump-pr.mjs";
import { commitError } from "./check-pr.mjs";

const bot = { authors: [{ email: BOT_EMAIL, login: "", name: "github-actions[bot]" }] };
const person = { authors: [{ email: "hugo@example.com", login: "Tusquito", name: "Tusquito" }] };
const pr = (number: number, headRefName: string, isDraft: boolean, commits = [bot]) => ({
  number,
  headRefName,
  isDraft,
  commits,
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
      pr(2, "feature/bump-karate-2.1.5", true, [bot, person]),
      pr(3, "feature/bump-karate-2.1.4", false),
      pr(4, "feature/bump-karate-2.1.7", true),
      pr(5, "feature/bump-jre-21.0.13_11", true),
      pr(6, "feature/home-screen", true),
      pr(7, "feature/bump-karate-2.1.6", true),
    ];
    expect(supersedePlan(prs, "karate", "2.1.6")).toEqual({ close: [1], keep: [2, 3] });
  });

  it("compares JRE versions as numbers", () => {
    const prs = [
      pr(1, "feature/bump-jre-21.0.12.1_1", true),
      pr(2, "feature/bump-jre-21.0.9_2", true),
    ];
    expect(supersedePlan(prs, "jre", "21.0.13+11")).toEqual({ close: [1, 2], keep: [] });
  });
});

describe("runCommandStatus", () => {
  const file = (filename: string) => ({ filename });

  it("says changed when RunCommand.java is in the list", () => {
    expect(
      runCommandStatus([
        file("README.md"),
        file("karate-core/src/main/java/io/karatelabs/cli/RunCommand.java"),
      ]),
    ).toBe("changed");
  });

  it("says unchanged for a short list without it", () => {
    expect(runCommandStatus([file("a"), file("b"), file("c")])).toBe("unchanged");
  });

  it("says unknown when GitHub's 300-file cap may hide it", () => {
    expect(runCommandStatus(Array.from({ length: 300 }, (_, i) => file(`f${i}`)))).toBe("unknown");
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
