import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { karateFolders, SUBFOLDER_LIMITS } from "./subfolders";

const dirs: string[] = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A fresh folder holding `files` (relative path → content); a path ending in `/` is a folder. */
function write(files: string[]): string {
  const root = mkdtempSync(path.join(tmpdir(), "kk-sub-"));
  dirs.push(root);
  for (const f of files) {
    const p = path.join(root, f);
    if (f.endsWith("/")) mkdirSync(p, { recursive: true });
    else {
      mkdirSync(path.dirname(p), { recursive: true });
      writeFileSync(p, "");
    }
  }
  return root;
}

describe("karateFolders", () => {
  it("offers each kind of Karate project", async () => {
    const root = write([
      "api/karate-pom.json",
      "cfg/karate-config.js",
      "g/build.gradle",
      "g/src/test/java/a/b.feature",
      "k/build.gradle.kts",
      "k/src/test/x.feature",
      "m/pom.xml",
      "m/src/test/resources/c.feature",
    ]);
    expect(await karateFolders(root)).toEqual(["api", "cfg", "g", "k", "m"]);
  });

  it("skips a build file whose src/test has no feature", async () => {
    const root = write([
      "tool/pom.xml",
      "tool/src/test/java/Helper.java",
      "t2/pom.xml",
      "t2/src/test/target/x.feature",
    ]);
    expect(await karateFolders(root)).toEqual([]);
  });

  it("looks 3 levels down", async () => {
    expect(await karateFolders(write(["a/b/c/karate-config.js"]))).toEqual(["a/b/c"]);
    expect(await karateFolders(write(["a/b/c/d/karate-config.js"]))).toEqual([]);
  });

  it("skips tool and output folders in any case", async () => {
    const root = write([
      "node_modules/x/karate-config.js",
      "NODE_MODULES/y/karate-config.js",
      "target/test-classes/karate-config.js",
      "build/z/karate-config.js",
      ".git/w/karate-config.js",
      ".gradle/v/karate-config.js",
    ]);
    expect(await karateFolders(root)).toEqual([]);
  });

  it("never offers the project folder and does not look inside an offered one", async () => {
    const root = write([
      "karate-config.js",
      "karate/karate-config.js",
      "karate/nested/karate-pom.json",
    ]);
    expect(await karateFolders(root)).toEqual(["karate"]);
  });

  it("does not offer the sources of a folder with a build file", async () => {
    expect(await karateFolders(write(["pom.xml", "src/test/java/karate-config.js"]))).toEqual([]);
    const root = write(["app/build.gradle", "app/src/test/resources/karate-config.js"]);
    expect(await karateFolders(root)).toEqual([]);
  });

  it("ignores a marker that is a folder", async () => {
    const root = write(["x/karate-config.js/", "y/pom.xml", "y/src/test"]);
    expect(await karateFolders(root)).toEqual([]);
  });

  it("follows no junction", async () => {
    const real = write(["karate-config.js"]);
    const root = write([]);
    symlinkSync(real, path.join(root, "link"), "junction");
    expect(await karateFolders(root)).toEqual([]);
  });

  it("orders by level, then by path", async () => {
    const root = write(["z/karate-config.js", "a/x/karate-config.js", "b/karate-pom.json"]);
    expect(await karateFolders(root)).toEqual(["b", "z", "a/x"]);
  });

  it("stops at the limits", async () => {
    const three = write(["a/karate-config.js", "b/karate-config.js", "c/karate-config.js"]);
    expect(await karateFolders(three, { ...SUBFOLDER_LIMITS, found: 2 })).toEqual(["a", "b"]);
    expect(await karateFolders(three, { ...SUBFOLDER_LIMITS, folders: 1 })).toEqual([]);
    expect(await karateFolders(three, { ...SUBFOLDER_LIMITS, timeoutMs: 0 })).toEqual([]);
    expect(await karateFolders(path.join(three, "missing"))).toEqual([]);
  });
});
