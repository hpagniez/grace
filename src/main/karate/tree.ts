import type { TestNode } from "../../shared/model";
import type { KarateEvent } from "./events";

const REF_ID = /^\[(\d+)(?:\.(\d+))?:(\d+)\]$/;

export function parseRefId(refId: string): {
  section: number;
  example: number | null;
  line: number;
} {
  const m = REF_ID.exec(refId);
  if (!m) throw new Error(`invalid refId: ${refId}`);
  return {
    section: Number(m[1]),
    example: m[2] === undefined ? null : Number(m[2]),
    line: Number(m[3]),
  };
}

const node = (
  kind: TestNode["kind"],
  key: string,
  name: string,
  file: string,
  extra: Partial<TestNode> = {},
): TestNode => ({
  key,
  kind,
  name,
  file,
  line: null,
  refId: null,
  tags: [],
  children: [],
  ...extra,
});

const byFile = (a: TestNode, b: TestNode) => (a.file < b.file ? -1 : a.file > b.file ? 1 : 0);

function sortFolders(folder: TestNode): void {
  const folders = folder.children.filter((c) => c.kind === "folder").sort(byFile);
  const features = folder.children.filter((c) => c.kind === "feature").sort(byFile);
  folder.children = [...folders, ...features];
  folders.forEach(sortFolders);
}

/**
 * Builds folder > feature > scenario | outline > row from the dry run's depth-0 events.
 * An outline node appears with its first row (the section number only exists on the rows);
 * an outline with no rows is dropped. A row's key number is its 1-based position in the
 * outline across all Examples tables, because refId examples restart per table.
 * Folders come first, then features, each sorted by path; scenarios keep file order.
 */
export function buildTree(events: KarateEvent[], rootName: string): TestNode {
  const root = node("folder", "d:", rootName, "");
  const folders = new Map<string, TestNode>([["", root]]);
  const folderFor = (dir: string): TestNode => {
    let f = folders.get(dir);
    if (!f) {
      const cut = dir.lastIndexOf("/");
      f = node("folder", `d:${dir}`, dir.slice(cut + 1), dir);
      folderFor(cut < 0 ? "" : dir.slice(0, cut)).children.push(f);
      folders.set(dir, f);
    }
    return f;
  };

  let feature: TestNode | null = null;
  let outlineEvent: KarateEvent | null = null;
  let outline: TestNode | null = null;
  for (const ev of events) {
    const d = ev.data;
    if (d.callDepth !== 0) continue;
    if (ev.type === "FEATURE_ENTER") {
      const file: string = d.path;
      const cut = file.lastIndexOf("/");
      feature = node("feature", `f:${file}`, d.name || file.slice(cut + 1), file, {
        tags: d.tags ?? [],
      });
      folderFor(cut < 0 ? "" : file.slice(0, cut)).children.push(feature);
      outlineEvent = outline = null;
    } else if (ev.type === "OUTLINE_ENTER") {
      outlineEvent = ev;
      outline = null;
    } else if (ev.type === "SCENARIO_ENTER") {
      if (!feature) throw new Error(`SCENARIO_ENTER before FEATURE_ENTER: ${d.slug}`);
      const { section } = parseRefId(d.refId);
      const base = { line: d.line, refId: d.refId, tags: d.tags ?? [] };
      if (!d.isOutlineExample) {
        feature.children.push(
          node("scenario", `s:${feature.file}#${section}`, d.name, feature.file, base),
        );
        outlineEvent = outline = null;
        continue;
      }
      if (!outlineEvent) throw new Error(`outline row without OUTLINE_ENTER: ${d.slug}`);
      if (!outline) {
        const o = outlineEvent.data;
        outline = node("outline", `o:${feature.file}#${section}`, o.name, feature.file, {
          line: o.line,
          tags: o.tags ?? [],
        });
        feature.children.push(outline);
      }
      outline.children.push(
        node(
          "row",
          `s:${feature.file}#${section}.${outline.children.length + 1}`,
          d.name,
          feature.file,
          { ...base, line: d.line + 2 + d.exampleIndex }, // d.line is the Examples: line
        ),
      );
    }
  }
  sortFolders(root);
  return root;
}

export function findNode(tree: TestNode, key: string): TestNode | null {
  if (tree.key === key) return tree;
  for (const child of tree.children) {
    const hit = findNode(child, key);
    if (hit) return hit;
  }
  return null;
}

/** Scenario and row keys at or under `n`, in tree order. */
export function leafKeys(n: TestNode): string[] {
  return n.kind === "scenario" || n.kind === "row" ? [n.key] : n.children.flatMap(leafKeys);
}

/** Maps `${file}|${refId}` to the scenario or row node. refIds are the same in a dry run and the run after it. */
export function leafIndex(tree: TestNode): Map<string, TestNode> {
  const index = new Map<string, TestNode>();
  const walk = (n: TestNode) => {
    if (n.refId !== null) index.set(`${n.file}|${n.refId}`, n);
    n.children.forEach(walk);
  };
  walk(tree);
  return index;
}

/** Finds `old` again after a re-discovery: same key and name, else the one same-kind node of that name in the same feature. Refuses to guess. */
export function resolveNode(tree: TestNode, old: TestNode): TestNode | null {
  const same = findNode(tree, old.key);
  if (same && same.name === old.name) return same;
  const matches: TestNode[] = [];
  const walk = (n: TestNode) => {
    if (n.kind === old.kind && n.file === old.file && n.name === old.name) matches.push(n);
    n.children.forEach(walk);
  };
  walk(tree);
  return matches.length === 1 ? matches[0] : null;
}
