import { mkdirSync, mkdtempSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { projectFile, validPoint, validRequest, vscodeUrl } from "./editor";

// The real realpathSync, its native form counted: a path it reads is a path it touched.
vi.mock("node:fs", async (importOriginal) => {
  const fs = await importOriginal<typeof import("node:fs")>();
  const realpathSync = (p: string) => fs.realpathSync(p);
  return {
    ...fs,
    realpathSync: Object.assign(realpathSync, { native: vi.fn(fs.realpathSync.native) }),
  };
});

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
  it("keeps a file of the project, by its path in the project, whatever its letter case", () => {
    const file = path.join(root, "tests", "a.feature");
    expect(projectFile(root, file)).toBe(file);
    expect(projectFile(root, file.toUpperCase())).toBe(file);
  });

  it("names the file under the project folder as it was opened: a mapped drive keeps its letter", () => {
    // A junction stands for a mapped drive: its real path is elsewhere, as Z:\ is \\server\share.
    const opened = path.join(outer, "z");
    symlinkSync(root, opened, "junction");
    const file = path.join(opened, "tests", "a.feature");
    expect(projectFile(opened, file)).toBe(file);
  });

  it("keeps a file under a folder whose name starts with ..", () => {
    const file = touch(root, "..data/b.feature");
    expect(projectFile(root, file)).toBe(file);
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

  it("refuses a path outside the project without reading it: a share or a device is never reached", () => {
    const outside = [
      "\\\\attacker\\s\\x.feature",
      "//attacker/s/x.feature",
      "\\\\.\\pipe\\x",
      "\\\\?\\C:\\Windows\\win.ini",
      "C:\\Windows\\win.ini",
    ];
    for (const file of outside) expect(projectFile(root, file)).toBeNull();
    const read = vi.mocked(realpathSync.native).mock.calls.map(([p]) => p);
    for (const file of outside) expect(read).not.toContain(file);
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

  it("keeps a share path's two leading slashes, as VS Code reads them", () => {
    expect(vscodeUrl("\\\\server\\share\\p\\x.feature", 3)).toBe(
      "vscode://file//server/share/p/x.feature:3",
    );
  });
});

describe("validRequest and validPoint (open-in-editor-from-tree R3.1)", () => {
  it("takes a path of at most 1000 characters and a line of 1 or more", () => {
    expect(validRequest("tests/a.feature", 1)).toBe(true);
    expect(validRequest("a".repeat(1000), 12)).toBe(true);
    for (const [file, line] of [
      ["tests/a.feature", 0],
      ["tests/a.feature", 1.5],
      ["tests/a.feature", "12"],
      ["a".repeat(1001), 1],
      [null, 1],
    ])
      expect(validRequest(file, line)).toBe(false);
  });

  it("takes a point of two finite numbers", () => {
    expect(validPoint(0, 0)).toBe(true);
    expect(validPoint(30.5, 40)).toBe(true);
    for (const [x, y] of [
      ["12", 4],
      [1, Number.NaN],
      [Infinity, 1],
      [1, undefined],
    ])
      expect(validPoint(x, y)).toBe(false);
  });
});
