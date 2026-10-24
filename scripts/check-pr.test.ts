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
