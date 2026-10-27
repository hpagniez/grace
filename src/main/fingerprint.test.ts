import { createHash } from "node:crypto";
import {
  mkdirSync,
  mkdtempSync,
  renameSync,
  rmSync,
  statSync,
  symlinkSync,
  utimesSync,
  writeFileSync,
  appendFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it, vi } from "vitest";
import { projectFingerprint } from "./fingerprint";

// Lets a test make the stat of a file fail while the rest of the module stays real.
const stat = vi.hoisted(() => ({ fail: false }));
vi.mock("node:fs/promises", async (importOriginal) => {
  const real = await importOriginal<typeof import("node:fs/promises")>();
  return {
    ...real,
    lstat: ((...args: Parameters<typeof real.lstat>) =>
      stat.fail ? Promise.reject(new Error("EPERM")) : real.lstat(...args)) as typeof real.lstat,
  };
});

const temps: string[] = [];
const tmp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-fp-"));
  temps.push(d);
  return d;
};
afterEach(() => {
  stat.fail = false;
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
});

/** Writes `files` (relative path to content) under `root`, folders included. */
function put(root: string, files: Record<string, string>): void {
  for (const [rel, content] of Object.entries(files)) {
    const file = path.join(root, rel);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, content);
  }
}

/** A root holding `a.txt` and `sub/b.txt`. */
function project(): string {
  const root = tmp();
  put(root, { "a.txt": "abc", "sub/b.txt": "hello" });
  return root;
}

describe("projectFingerprint", () => {
  it("matches the spec's line format", async () => {
    const root = project();
    const line = (rel: string) => {
      const st = statSync(path.join(root, rel));
      return `${rel}\t${st.size}\t${Math.trunc(st.mtimeMs)}\n`;
    };
    const expected = createHash("sha256")
      .update(line("a.txt") + line("sub/b.txt"))
      .digest("hex");
    const value = await projectFingerprint(root);
    expect(value).toMatch(/^[0-9a-f]{64}$/);
    expect(value).toBe(expected);
    expect(await projectFingerprint(root)).toBe(value);
  });

  it.each<[string, (root: string) => void]>([
    ["a byte is appended", (root) => appendFileSync(path.join(root, "a.txt"), "d")],
    [
      "a file's modified time moves 5 s",
      (root) => {
        const file = path.join(root, "a.txt");
        const t = statSync(file).mtimeMs / 1000 + 5;
        utimesSync(file, t, t);
      },
    ],
    ["a file is added", (root) => put(root, { "c.txt": "new" })],
    ["a file is removed", (root) => rmSync(path.join(root, "sub", "b.txt"))],
    ["a file is renamed", (root) => renameSync(path.join(root, "a.txt"), path.join(root, "z.txt"))],
  ])("each change gives a new value: %s", async (_name, change) => {
    const root = project();
    const before = await projectFingerprint(root);
    expect(before).not.toBeNull();
    change(root);
    const after = await projectFingerprint(root);
    expect(after).not.toBeNull();
    expect(after).not.toBe(before);
  });

  it("skips build and tool folders at any depth and in any case", async () => {
    const root = project();
    const before = await projectFingerprint(root);
    put(root, {
      ".git/x": "1",
      "node_modules/x": "1",
      "Target/x": "1",
      "sub/build/x": "1",
      ".GRADLE/x": "1",
    });
    expect(await projectFingerprint(root)).toBe(before);
    put(root, { "sub/targets/x": "1" });
    expect(await projectFingerprint(root)).not.toBe(before);
  });

  it("does not follow or list a junction", async () => {
    const root = project();
    const other = tmp();
    put(other, { "f.txt": "elsewhere", "deep/g.txt": "elsewhere" });
    const before = await projectFingerprint(root);
    symlinkSync(other, path.join(root, "link"), "junction");
    expect(await projectFingerprint(root)).toBe(before);
  });

  it("null past the file cap", async () => {
    const root = tmp();
    put(root, { "1": "", "2": "", "3": "", "4": "" });
    expect(await projectFingerprint(root, { maxFiles: 3, timeoutMs: 2000 })).toBeNull();
    expect(await projectFingerprint(root, { maxFiles: 4, timeoutMs: 2000 })).toMatch(
      /^[0-9a-f]{64}$/,
    );
  });

  it("null past the time bound", async () => {
    const root = project();
    expect(await projectFingerprint(root, { maxFiles: 50_000, timeoutMs: 0 })).toBeNull();
  });

  it("null for a missing root", async () => {
    await expect(projectFingerprint(path.join(tmp(), "missing"))).resolves.toBeNull();
  });

  it("null when a stat fails", async () => {
    const root = project();
    stat.fail = true;
    await expect(projectFingerprint(root)).resolves.toBeNull();
  });
});
