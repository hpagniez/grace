import { describe, expect, it } from "vitest";
import type { TestNode } from "./model";
import {
  NO_FILTER,
  chipTags,
  cycleTag,
  filterable,
  filterOn,
  matches,
  matchingLeaves,
  parseFilterState,
  projectTags,
  pruneFilter,
} from "./tags";

const node = (
  kind: TestNode["kind"],
  key: string,
  name: string,
  extra: Partial<TestNode> = {},
): TestNode => ({
  key,
  kind,
  name,
  file: "tests/x.feature",
  line: null,
  refId: null,
  tags: [],
  children: [],
  ...extra,
});

const scenario = (n: number, name: string, tags: string[]) =>
  node("scenario", `s:tests/x.feature#${n}`, name, { tags });
const row = (n: number) => node("row", `s:tests/outline.feature#1.${n}`, `add ${n}`);

// The clean sample project: 9 tests, tagged as in the tag filter spec.
const tree = (): TestNode =>
  node("folder", "d:", "clean", {
    children: [
      node("folder", "d:tests", "tests", {
        children: [
          node("feature", "f:tests/basic.feature", "basic", {
            children: [
              scenario(1, "passing hello", ["smoke"]),
              scenario(2, "failing match", ["wip"]),
              scenario(3, "calls helper feature", []),
              scenario(4, "smoke but wip", ["smoke", "wip"]),
            ],
          }),
          node("feature", "f:tests/outline.feature", "outline", {
            children: [
              node("outline", "o:tests/outline.feature#1", "add", {
                children: [row(1), row(2), row(3)],
              }),
            ],
          }),
          node("feature", "f:tests/slow.feature", "slow", {
            children: [
              scenario(1, "slow loop", ["slow"]),
              scenario(2, "smoke and slow", ["smoke", "slow"]),
            ],
          }),
        ],
      }),
    ],
  });

const names = (f: { include?: string[]; exclude?: string[] }) =>
  matchingLeaves(tree(), { include: [], exclude: [], ...f }).map((n) => n.name);

describe("matching", () => {
  it("empty filter matches everything", () => {
    expect(matchingLeaves(tree(), NO_FILTER).length).toBe(9);
    expect(filterOn(NO_FILTER)).toBe(false);
  });

  it("include smoke exclude wip", () => {
    expect(names({ include: ["smoke"], exclude: ["wip"] })).toEqual([
      "passing hello",
      "smoke and slow",
    ]);
  });

  it("exclude only keeps untagged tests", () => {
    expect(names({ exclude: ["wip"] }).length).toBe(7);
  });

  it("included tags combine with OR", () => {
    expect(names({ include: ["wip", "slow"] })).toEqual([
      "failing match",
      "smoke but wip",
      "slow loop",
      "smoke and slow",
    ]);
  });

  it("matching is case-sensitive", () => {
    expect(matches(["Smoke"], { include: ["smoke"], exclude: [] })).toBe(false);
  });

  it("filterOn is true with an include or an exclude", () => {
    expect(filterOn({ include: ["a"], exclude: [] })).toBe(true);
    expect(filterOn({ include: [], exclude: ["a"] })).toBe(true);
  });

  it("matchingLeaves from a leaf returns that leaf only when it matches", () => {
    const leaf = scenario(1, "x", ["smoke"]);
    expect(matchingLeaves(leaf, { include: ["smoke"], exclude: [] })).toEqual([leaf]);
    expect(matchingLeaves(leaf, { include: ["wip"], exclude: [] })).toEqual([]);
  });
});

describe("chips", () => {
  it("projectTags is sorted and unique", () => {
    expect(projectTags(tree())).toEqual(["slow", "smoke", "wip"]);
  });

  it("filterable accepts only tags -t can select", () => {
    for (const t of ["smoke", "owner=bob", 'd"x', "r)x", "a@b", "a~b"])
      expect(filterable(t), t).toBe(true);
    // Karate reads `\` as a JS escape: `b\` fails to parse, `a\nb` never matches itself.
    for (const t of ["", "a b", "a\tb", "owner=bob,alice", "q'x", "p(x", "@x", "~x", "b\\x", "b\\"])
      expect(filterable(t), t).toBe(false);
  });

  it("projectTags skips tags -t cannot select", () => {
    const t = node("folder", "d:", "root", {
      children: [scenario(1, "odd", ["smoke", "owner=bob,alice", "q'x", "p(x", "owner=bob"])],
    });
    expect(projectTags(t)).toEqual(["owner=bob", "smoke"]);
  });

  it("chipTags keeps a filtered tag the tree lost", () => {
    expect(chipTags(tree(), { include: ["gone"], exclude: [] })).toContain("gone");
    expect(chipTags(null, { include: ["b"], exclude: ["a"] })).toEqual(["a", "b"]);
  });
});

describe("filter state", () => {
  it("cycleTag goes off, include, exclude, off", () => {
    const a = cycleTag(NO_FILTER, "smoke");
    expect(a).toEqual({ include: ["smoke"], exclude: [], hideNonMatching: false });
    const b = cycleTag(a, "smoke");
    expect(b).toEqual({ include: [], exclude: ["smoke"], hideNonMatching: false });
    const c = cycleTag(b, "smoke");
    expect(c).toEqual(NO_FILTER);
    expect(NO_FILTER).toEqual({ include: [], exclude: [], hideNonMatching: false });
  });

  it("pruneFilter drops unknown tags and keeps hideNonMatching", () => {
    const f = { include: ["a", "gone"], exclude: ["b", "lost"], hideNonMatching: true };
    expect(pruneFilter(f, ["a", "b"])).toEqual({
      include: ["a"],
      exclude: ["b"],
      hideNonMatching: true,
    });
  });

  it("parseFilterState rejects bad shapes", () => {
    const ok = { include: ["a"], exclude: ["b"], hideNonMatching: true };
    expect(parseFilterState(ok)).toEqual(ok);
    const bad: unknown[] = [
      undefined,
      null,
      {},
      { ...ok, include: "a" },
      { ...ok, include: [1] },
      { ...ok, include: [""] },
      { ...ok, include: ["x".repeat(201)] },
      { ...ok, include: Array.from({ length: 501 }, (_, i) => `t${i}`) },
      { ...ok, include: ["a"], exclude: ["a"] },
      { ...ok, hideNonMatching: "yes" },
      { ...ok, include: ["a,b"] },
      { ...ok, exclude: ["q'x"] },
      { ...ok, exclude: ["p(x"] },
      { ...ok, include: ["@a"] },
      { ...ok, include: ["a b"] },
    ];
    for (const x of bad) expect(parseFilterState(x), JSON.stringify(x)).toBeNull();
  });
});
