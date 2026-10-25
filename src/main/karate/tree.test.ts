import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { readAllEvents, type KarateEvent } from "./events";
import type { StoredTest } from "../../shared/history";
import type { TestNode } from "../../shared/model";
import {
  buildTree,
  findNode,
  leafIndex,
  leafKeys,
  parseRefId,
  resolveNode,
  resolveTests,
  rowLines,
} from "./tree";

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
      line: null, // the Runner reads row lines from the feature file
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
    expect(rows.map((r) => r.refId)).toEqual([
      "[2.1:13]",
      "[2.2:13]",
      "[2.3:13]",
      "[2.4:13]",
      "[2.5:13]",
      "[2.1:22]",
      "[2.2:22]",
    ]);
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

describe("rowLines", () => {
  const text = (file: string) => readFileSync(path.resolve("test/fixtures/projects", file), "utf8");

  it("takes the rows after the header", () => {
    expect(rowLines(text("clean/tests/outline.feature"), 7)).toEqual([9, 10, 11]);
  });

  it("skips comment and blank lines inside the table", () => {
    expect(rowLines(text("edge/tests/rows.feature"), 7)).toEqual([10, 12, 13]);
  });

  it("finds no rows in a dynamic Examples table", () => {
    expect(rowLines(text("edge/tests/rows.feature"), 18)).toEqual([]);
  });

  it("reads CRLF files and stops at the next keyword", () => {
    const crlf = ["Examples:", "| a |", "| 1 |", "Scenario: x", "| 2 |", ""].join("\r\n");
    expect(rowLines(crlf, 1)).toEqual([3]);
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

describe("resolveTests", () => {
  const F = "tests/a.feature";
  const leaf = (
    kind: TestNode["kind"],
    key: string,
    name: string,
    line: number | null,
    children: TestNode[] = [],
  ): TestNode => ({ key, kind, name, file: F, line, refId: null, tags: [], children });
  const tree = leaf("folder", "d:", "x", null, [
    leaf("feature", `f:${F}`, "a", null, [
      leaf("scenario", `s:${F}#1`, "other", 3),
      leaf("scenario", `s:${F}#2`, "moved", 8), // was #1 before "other" was added above it
      leaf("scenario", `s:${F}#3`, "stays", 14),
      leaf("outline", `o:${F}#4`, "dyn <n>", 20, [leaf("row", `s:${F}#4.1`, "dyn 1", null)]),
      leaf("scenario", `s:${F}#5`, "twin", 30),
      leaf("scenario", `s:${F}#6`, "twin", 36),
    ]),
  ]);
  const stored = (kind: StoredTest["kind"], key: string, name: string, line: number | null) =>
    ({ kind, file: F, refId: "[1:1]", name, line, key, status: "failed" }) satisfies StoredTest;

  it("finds moved and same-key tests, counts missing and dynamic", () => {
    const tests = [
      stored("scenario", `s:${F}#3`, "stays", 14), // same key
      stored("scenario", `s:${F}#9`, "gone", 50), // deleted
      stored("row", `s:${F}#4.1`, "dyn 1", null), // row of a dynamic Examples table
      // Its key matches neither twin: a same-key hit would pass before the ambiguity check.
      stored("scenario", `s:${F}#7`, "twin", 40),
      stored("scenario", `s:${F}#1`, "moved", 3), // key shifted, name kept
      stored("scenario", `s:${F}#3`, "stays", 14), // again: found once
    ];
    const { found, missing, dynamic } = resolveTests(tree, tests);
    expect(found.map((n) => n.key)).toEqual([`s:${F}#2`, `s:${F}#3`]); // tree order
    expect(missing).toBe(2);
    expect(dynamic).toBe(1);
  });
});
