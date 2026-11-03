import { mkdirSync, mkdtempSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { projectFile, vscodeUrl } from "./editor";

let outer: string;
let root: string;
/** Writes an empty file at `rel` under `base`, with its folders, and returns its path. */
const touch = (base: string, rel: string) => {
  const f = path.join(base, rel);
  mkdirSync(path.dirname(f), { recursive: true });
  writeFileSync(f, "");
  return f;
};

beforeEach(() => {
  outer = mkdtempSync(path.join(tmpdir(), "kk-editor-"));
  root = path.join(outer, "p");
  touch(root, "tests/a.feature");
});
afterEach(() => rmSync(outer, { recursive: true, force: true }));

describe("projectFile (open-in-editor R3.3)", () => {
  it("keeps a file of the project, by its real path, whatever its letter case", () => {
    const file = path.join(root, "tests", "a.feature");
    const real = realpathSync.native(file);
    expect(projectFile(root, file)).toBe(real);
    expect(projectFile(root, file.toUpperCase())).toBe(real);
  });

  it("keeps a file under a folder whose name starts with ..", () => {
    const file = touch(root, "..data/b.feature");
    expect(projectFile(root, file)).toBe(realpathSync.native(file));
  });

  it("refuses a file outside the project, a missing file and the project folder", () => {
    touch(outer, "outside.feature");
    const other = mkdtempSync(path.join(tmpdir(), "kk-editor-other-"));
    try {
      expect(projectFile(root, path.join(root, "..", "outside.feature"))).toBeNull();
      expect(projectFile(root, touch(other, "x.feature"))).toBeNull();
    } finally {
      rmSync(other, { recursive: true, force: true });
    }
    expect(projectFile(root, path.join(root, "tests", "gone.feature"))).toBeNull();
    expect(projectFile(root, root)).toBeNull();
  });

  it("refuses a file reached through a link to a folder outside the project", () => {
    touch(outer, "other/out.feature");
    symlinkSync(path.join(outer, "other"), path.join(root, "link"), "junction");
    expect(projectFile(root, path.join(root, "link", "out.feature"))).toBeNull();
  });
});

describe("vscodeUrl (open-in-editor R4.2)", () => {
  it("uses / separators and encodes what a URL needs", () => {
    expect(vscodeUrl("C:\\My Project\\a#b.feature", 7)).toBe(
      "vscode://file/C:/My%20Project/a%23b.feature:7",
    );
    expect(vscodeUrl("C:\\p\\100%\\é?.feature", 2)).toBe(
      "vscode://file/C:/p/100%25/%C3%A9%3F.feature:2",
    );
  });
});
