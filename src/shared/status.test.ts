import { describe, expect, it } from "vitest";
import type { Status, TestNode } from "./model";
import { deriveStatus } from "./status";

const leaf = (key: string): TestNode => ({
  key,
  kind: "scenario",
  name: key,
  file: "a.feature",
  line: 1,
  refId: null,
  tags: [],
  children: [],
});
const feature = (...children: TestNode[]): TestNode => ({
  key: "f:a.feature",
  kind: "feature",
  name: "a",
  file: "a.feature",
  line: null,
  refId: null,
  tags: [],
  children,
});
const derive = (...statuses: Status[]) => {
  const kids = statuses.map((_, i) => leaf(`s${i}`));
  return deriveStatus(feature(...kids), Object.fromEntries(statuses.map((s, i) => [`s${i}`, s])));
};

describe("deriveStatus", () => {
  it("failed wins over passed and running", () => {
    expect(derive("passed", "failed")).toBe("failed");
    expect(derive("running", "failed")).toBe("failed");
  });
  it("running wins over passed", () => expect(derive("passed", "running")).toBe("running"));
  it("all passed is passed", () => expect(derive("passed", "passed")).toBe("passed"));
  it("notRun when any child is not run and none failed or running", () =>
    expect(derive("passed", "notRun")).toBe("notRun"));
  it("interrupted when no child failed", () =>
    expect(derive("passed", "interrupted")).toBe("interrupted"));
  it("pending when a child waits", () => expect(derive("passed", "pending")).toBe("pending"));
  it("leaf returns its own status, default notRun", () => {
    expect(deriveStatus(leaf("x"), { x: "passed" })).toBe("passed");
    expect(deriveStatus(leaf("x"), {})).toBe("notRun");
  });
  it("empty folder is notRun", () => expect(deriveStatus(feature(), {})).toBe("notRun"));
  it("recurses through nested nodes", () => {
    const tree = feature({ ...feature(leaf("a"), leaf("b")), key: "o:x" });
    expect(deriveStatus(tree, { a: "passed", b: "failed" })).toBe("failed");
  });
});
