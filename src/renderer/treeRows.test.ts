import { describe, expect, it } from "vitest";
import type { TestNode } from "../shared/model";
import { NO_FILTER } from "../shared/tags";
import { rowName, treeRows } from "./treeRows";

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

const row = (n: number, name: string) =>
  node("row", `s:tests/outline.feature#1.${n}`, name, { line: 5 + n });
const tree = (): TestNode =>
  node("folder", "d:", "clean", {
    children: [
      node("folder", "d:tests", "tests", {
        children: [
          node("feature", "f:tests/basic.feature", "basic API checks", {
            file: "tests/basic.feature",
            children: [
              node("scenario", "s:tests/basic.feature#1", "passing hello", {
                line: 6,
                tags: ["smoke"],
              }),
              node("scenario", "s:tests/basic.feature#2", "failing match", { line: 14 }),
            ],
          }),
          node("feature", "f:tests/outline.feature", "outline with examples", {
            file: "tests/outline.feature",
            children: [
              node("outline", "o:tests/outline.feature#1", "add <a> and <b>", {
                children: [row(1, "add 1 and 2"), row(2, "add 2 and 3")],
              }),
            ],
          }),
        ],
      }),
    ],
  });

const keys = (rows: ReturnType<typeof treeRows>) => rows.map((r) => r.node.key);

describe("treeRows", () => {
  it("lists every node but the root in tree order, with its level and place among its siblings", () => {
    const rows = treeRows(tree(), new Set(), NO_FILTER);
    expect(keys(rows)).toEqual([
      "d:tests",
      "f:tests/basic.feature",
      "s:tests/basic.feature#1",
      "s:tests/basic.feature#2",
      "f:tests/outline.feature",
      "o:tests/outline.feature#1",
      "s:tests/outline.feature#1.1",
      "s:tests/outline.feature#1.2",
    ]);
    expect(rows.map((r) => r.level)).toEqual([1, 2, 3, 3, 2, 3, 4, 4]);
    expect(rows[4]).toMatchObject({ posInSet: 2, setSize: 2, parentKey: "d:tests" });
    expect(rows[0]).toMatchObject({ posInSet: 1, setSize: 1, parentKey: null });
  });

  it("leaves out the children of a closed node but keeps the node", () => {
    const rows = treeRows(tree(), new Set(["f:tests/basic.feature"]), NO_FILTER);
    expect(keys(rows)).toEqual([
      "d:tests",
      "f:tests/basic.feature",
      "f:tests/outline.feature",
      "o:tests/outline.feature#1",
      "s:tests/outline.feature#1.1",
      "s:tests/outline.feature#1.2",
    ]);
  });

  it("leaves out non-matching rows under Hide non-matching, and counts only the shown siblings", () => {
    const rows = treeRows(tree(), new Set(), {
      include: ["smoke"],
      exclude: [],
      hideNonMatching: true,
    });
    expect(keys(rows)).toEqual(["d:tests", "f:tests/basic.feature", "s:tests/basic.feature#1"]);
    expect(rows[1]).toMatchObject({ posInSet: 1, setSize: 1 });
    expect(rows[2]).toMatchObject({ level: 3, posInSet: 1, setSize: 1 });
  });

  it("keeps every row when the filter only dims", () => {
    const rows = treeRows(tree(), new Set(), {
      include: ["smoke"],
      exclude: [],
      hideNonMatching: false,
    });
    expect(rows).toHaveLength(8);
  });
});

describe("rowName", () => {
  it("joins the label, the status word in lower case, the flaky words and the tags", () => {
    expect(rowName("login.feature", "failed", "2 flaky", ["smoke"])).toBe(
      "login.feature, failed, 2 flaky, smoke",
    );
    expect(rowName("login works", "failed", "flaky", [])).toBe("login works, failed, flaky");
    expect(rowName("x", "notRun", null, [])).toBe("x, not run");
  });
});
