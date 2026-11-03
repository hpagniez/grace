import { mkdirSync, mkdtempSync, rmSync, utimesSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { testCodeNotice } from "./compiled";

const dirs: string[] = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A temp project holding `files` (relative path → modified time in seconds since 2020). */
function project(files: Record<string, number>): string {
  const root = mkdtempSync(path.join(tmpdir(), "kk-compiled-"));
  dirs.push(root);
  const base = Date.UTC(2020, 0, 1) / 1000;
  for (const [f, at] of Object.entries(files)) {
    const file = path.join(root, f);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, "");
    utimesSync(file, base + at, base + at);
  }
  return root;
}

const NOT_BUILT = (cmd: string) =>
  `The project's Java code in src/test/java is not built: build it (${cmd}) before running.`;
const CHANGED = (cmd: string) =>
  `The project's Java code in src/test/java changed since its last build: build it again (${cmd}) before running.`;

describe("testCodeNotice", () => {
  it("says nothing without Java code in src/test/java", async () => {
    const root = project({ "pom.xml": 0, "src/test/java/a/a.feature": 10 });
    expect(await testCodeNotice(root, "maven")).toBeNull();
  });
  it("says when the code was never built", async () => {
    const root = project({ "pom.xml": 0, "src/test/java/a/Helper.java": 10 });
    expect(await testCodeNotice(root, "maven")).toBe(NOT_BUILT("mvn test-compile"));
  });
  it("counts an empty compiled folder as not built", async () => {
    const root = project({ "src/test/java/a/Helper.java": 10, "target/test-classes/a.json": 20 });
    expect(await testCodeNotice(root, "maven")).toBe(NOT_BUILT("mvn test-compile"));
  });
  it("says nothing when the newest class is newer than every source", async () => {
    const root = project({
      "src/test/java/a/Helper.java": 10,
      "src/test/java/b/Other.java": 15,
      "target/test-classes/a/Helper.class": 20,
    });
    expect(await testCodeNotice(root, "maven")).toBeNull();
  });
  it("says when a source changed after the last build", async () => {
    const root = project({
      "src/test/java/a/Helper.java": 30,
      "target/test-classes/a/Helper.class": 20,
    });
    expect(await testCodeNotice(root, "maven")).toBe(CHANGED("mvn test-compile"));
  });
  it("reads Gradle's compiled folder and names its task", async () => {
    const root = project({ "src/test/java/a/Helper.java": 10 });
    expect(await testCodeNotice(root, "gradle")).toBe(NOT_BUILT("gradle testClasses"));
    const built = project({
      "src/test/java/a/Helper.java": 10,
      "build/classes/java/test/a/Helper.class": 20,
    });
    expect(await testCodeNotice(built, "gradle")).toBeNull();
  });
  it("says nothing for a standalone project", async () => {
    const root = project({ "src/test/java/a/Helper.java": 10 });
    expect(await testCodeNotice(root, "standalone")).toBeNull();
  });
});
