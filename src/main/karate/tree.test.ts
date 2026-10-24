import path from "node:path";
import { describe, expect, it } from "vitest";
import { readAllEvents, type KarateEvent } from "./events";
import type { TestNode } from "../../shared/model";
import { buildTree, findNode, leafIndex, leafKeys, parseRefId, resolveNode } from "./tree";

const events = (name: string) => readAllEvents(path.resolve("test/fixtures/events", name));
const clean = () => events("dryrun-clean.jsonl");

/** Deep copy of the events with `fn` applied to the data of every event of `file`. */
function mutate(evs: KarateEvent[], file: string, fn: (data: Record<string, any>) => void) {
  const copy = structuredClone(evs);
  for (const ev of copy) if (ev.data.feature === file) fn(ev.data);
  return copy;
}

/** Shift section numbers and lines of basic.feature, as if scenarios were added above. */
const shiftBasic = (evs: KarateEvent[], sections: number, lines: number) =>
  mutate(evs, "tests/basic.feature", (d) => {
    const { section, line } = parseRefId(d.refId);
    d.refId = `[${section + sections}:${line + lines}]`;
    d.line += lines;
  });

describe("parseRefId", () => {
  it("parses rows and scenarios", () => {
    expect(parseRefId("[1.3:7]")).toEqual({ section: 1, example: 3, line: 7 });
    expect(parseRefId("[3:22]")).toEqual({ section: 3, example: null, line: 22 });
  });
});

describe("buildTree", () => {
  it("builds the clean project tree", () => {
    const root = buildTree(clean(), "clean");
    expect(root).toMatchObject({ kind: "folder", key: "d:", file: "", name: "clean" });
    expect(root.children.map((c) => c.key)).toEqual(["d:tests"]);
    expect(root.children[0].children.map((c) => c.key)).toEqual([
      "f:tests/basic.feature",
      "f:tests/outline.feature",
      "f:tests/slow.feature",
    ]);
    expect(leafKeys(root)).toHaveLength(9);
    const files: string[] = [];
    const walk = (n: TestNode) => {
      files.push(n.file);
      n.children.forEach(walk);
    };
    walk(root);
    expect(files.some((f) => f.startsWith("tests/common/"))).toBe(false);
  });

  it("groups outline rows", () => {
    const outline = findNode(buildTree(clean(), "clean"), "o:tests/outline.feature#1")!;
    expect(outline).toMatchObject({
      kind: "outline",
      name: "add <a> and <b>",
      line: 3,
      refId: null,
    });
    expect(outline.children).toHaveLength(3);
    const row = outline.children.find((r) => r.name === "add 2 and 3")!;
    expect(row).toMatchObject({
      key: "s:tests/outline.feature#1.2",
      kind: "row",
      line: 10,
      refId: "[1.2:7]",
    });
  });

  it("handles two Examples tables", () => {
    const tree = buildTree(events("dryrun-outline-sizes.jsonl"), "extra");
    const feature = findNode(tree, "f:extra/outline-sizes.feature")!;
    expect(feature.children.map((c) => c.key)).toEqual([
      "o:extra/outline-sizes.feature#1",
      "o:extra/outline-sizes.feature#2",
    ]);
    const rows = feature.children[1].children;
    expect(rows).toHaveLength(7);
    // first table: Examples at 13 -> 15..19; second table: Examples at 22 -> 24, 25
    expect(rows.map((r) => r.line)).toEqual([15, 16, 17, 18, 19, 24, 25]);
    expect(rows[5].key).toBe("s:extra/outline-sizes.feature#2.6");
    expect(rows[6].key).toBe("s:extra/outline-sizes.feature#2.7");
    const keys = leafKeys(tree);
    expect(keys).toHaveLength(8);
    expect(new Set(keys).size).toBe(8);
  });

  it("keeps scenario tags", () => {
    const node = findNode(buildTree(clean(), "clean"), "s:tests/basic.feature#2")!;
    expect(node).toMatchObject({ name: "failing match", tags: ["wip"], line: 15, refId: "[2:15]" });
  });
});

describe("leafIndex", () => {
  it("maps file|refId to the node", () => {
    const index = leafIndex(buildTree(clean(), "clean"));
    expect(index.size).toBe(9);
    expect(index.get("tests/outline.feature|[1.2:7]")!.name).toBe("add 2 and 3");
  });
});

describe("resolveNode", () => {
  const oldNode = () => findNode(buildTree(clean(), "clean"), "s:tests/basic.feature#2")!;

  it("finds a scenario that moved", () => {
    const moved = buildTree(shiftBasic(clean(), 1, 5), "clean");
    expect(findNode(moved, oldNode().key)!.name).not.toBe("failing match");
    expect(resolveNode(moved, oldNode())).toMatchObject({
      name: "failing match",
      key: "s:tests/basic.feature#3",
      line: 20,
    });
  });

  it("returns null when the name is ambiguous", () => {
    const dup = mutate(shiftBasic(clean(), 2, 5), "tests/basic.feature", (d) => {
      if (d.name === "passing hello") d.name = "failing match";
    });
    expect(resolveNode(buildTree(dup, "clean"), oldNode())).toBeNull();
  });

  it("returns null when the scenario is gone", () => {
    const gone = clean().filter((e) => e.data.name !== "failing match");
    expect(resolveNode(buildTree(gone, "clean"), oldNode())).toBeNull();
  });
});
