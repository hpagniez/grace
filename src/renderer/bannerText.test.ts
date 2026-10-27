import { describe, expect, it } from "vitest";
import type { NodeKind, TestNode } from "../shared/model";
import { bannerState, testsText } from "./bannerText";
import { initialState, type AppState } from "./reduce";

const node = (kind: NodeKind, key: string, children: TestNode[] = []): TestNode => ({
  key,
  kind,
  name: key,
  file: key,
  line: kind === "folder" || kind === "feature" ? null : 1,
  refId: null,
  tags: [],
  children,
});
const scenario = (key: string) => node("scenario", key);
const feature = (key: string, children: TestNode[]) => node("feature", key, children);
const root = (...features: TestNode[]) => node("folder", "d:", features);

const opened = (tree: TestNode | null, more: Partial<AppState> = {}): AppState => ({
  ...initialState(),
  project: { root: "D:\\work\\orders-api", layout: "standalone" },
  tree,
  ...more,
});

const threeInTwo = root(
  feature("f:a", [scenario("s:a1"), scenario("s:a2")]),
  feature("f:b", [scenario("s:b1")]),
);

describe("testsText", () => {
  it("has no tests line with no project", () => {
    expect(testsText(initialState())).toBeNull();
  });

  it("counts scenarios and features", () => {
    expect(testsText(opened(threeInTwo))).toBe("3 scenarios in 2 features");
  });

  it("uses the singular for one of each", () => {
    expect(testsText(opened(root(feature("f:a", [scenario("s:a1")]))))).toBe(
      "1 scenario in 1 feature",
    );
  });

  it("counts outline rows, not outlines", () => {
    const outline = node("outline", "o:a", [node("row", "s:a.1"), node("row", "s:a.2")]);
    expect(testsText(opened(root(feature("f:a", [scenario("s:a0"), outline]))))).toBe(
      "3 scenarios in 1 feature",
    );
  });

  it("counts ignore the tag filter", () => {
    const filter = { include: ["smoke"], exclude: [], hideNonMatching: true };
    expect(testsText(opened(threeInTwo, { filter }))).toBe("3 scenarios in 2 features");
  });

  it("says when there are no tests", () => {
    expect(testsText(opened(root(feature("f:a", []))))).toBe("No feature files found");
  });

  it("says it is finding tests while busy with no tree", () => {
    expect(testsText(opened(null, { busy: true }))).toBe("finding tests…");
    expect(testsText(opened(null))).toBeNull();
  });

  it("points at the tree after a discovery error", () => {
    expect(testsText(opened(null, { error: "boom" }))).toBe(
      "could not be listed: see the message in the tree",
    );
  });
});

describe("bannerState", () => {
  const idle = initialState();

  it("asks for a project with none open", () => {
    expect(bannerState(idle)).toBe("none");
  });

  it("is ready with a project and nothing running", () => {
    expect(bannerState(opened(threeInTwo))).toBe("ready");
  });

  it("is finding while busy and no run is active", () => {
    expect(bannerState({ ...idle, busy: true })).toBe("finding");
    expect(bannerState(opened(threeInTwo, { busy: true }))).toBe("finding");
  });

  it("is starting once a run is active and Karate has not printed", () => {
    const run = { ...idle.run, active: true };
    expect(bannerState(opened(threeInTwo, { busy: true, run }))).toBe("starting");
  });

  it("goes once the console has output, in every state", () => {
    const run = { ...idle.run, active: true };
    for (const s of [
      idle,
      opened(threeInTwo),
      opened(threeInTwo, { busy: true }),
      opened(threeInTwo, { busy: true, run }),
    ])
      expect(bannerState({ ...s, console: ["x"] })).toBeNull();
  });

  it("is ready again after a run that printed nothing", () => {
    const run = { ...idle.run, active: false, outcome: "cancelled" as const };
    expect(bannerState(opened(threeInTwo, { run }))).toBe("ready");
  });
});
