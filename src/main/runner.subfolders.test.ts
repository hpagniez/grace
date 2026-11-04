import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { calls, cleanUp, fixture, setup, stubTool, T, temps } from "../../test/runner";
import { leafKeys } from "./karate/tree";
import { loadRecentProjects } from "./settings";

afterEach(cleanUp);

/** A fresh folder holding `files` (relative path → content). */
function write(files: Record<string, string>, root = mkdtempSync(path.join(tmpdir(), "kk-sub-"))) {
  if (!temps.includes(root)) temps.push(root);
  for (const [f, text] of Object.entries(files)) {
    mkdirSync(path.dirname(path.join(root, f)), { recursive: true });
    writeFileSync(path.join(root, f), text);
  }
  return root;
}

/** Another tool's pom.xml at the root, a stubbed Maven wrapper, and the Karate project in karate/. */
function repo() {
  const root = write({
    "pom.xml": "<project/>",
    "karate/karate-config.js": "function fn() { return {}; }",
    "karate/tests/basic.feature":
      "Feature: basic\nScenario: one\n  * print 1\nScenario: two\n  * print 2\n",
  });
  return { root, log: stubTool(root, "mvnw.cmd", "") };
}

describe("Runner sub-folder offer", () => {
  it(
    "offers the Karate project below a folder with no test, without listing libraries",
    T,
    async () => {
      const { runner, last, all } = setup();
      const { root, log } = repo();
      await runner.open(root);
      const p = last("project");
      expect(p.subfolders).toEqual(["karate"]);
      expect(leafKeys(p.tree!)).toEqual([]);
      expect(calls(log)).toEqual([]);
      expect(all("notice")).toEqual([]);
    },
  );

  it("skips the listing on Refresh too", T, async () => {
    const { runner } = setup();
    const { root, log } = repo();
    await runner.open(root);
    await runner.refresh();
    expect(calls(log)).toEqual([]);
  });

  it("lists libraries when karate-pom.json paths give feature roots", T, async () => {
    const { runner } = setup();
    const root = write({
      "pom.xml": "<project/>",
      "karate-pom.json": JSON.stringify({ paths: ["tests"] }),
      "tests/t.feature": "Feature: t\nScenario: s\n  * print 1\n",
    });
    const log = stubTool(root, "mvnw.cmd", "");
    await runner.open(root);
    expect(calls(log)).toHaveLength(1);
  });

  it("offers nothing when the tree has tests", T, async () => {
    const { runner, last } = setup();
    const root = write({
      "t.feature": "Feature: t\nScenario: s\n  * print 1\n",
      "sub/karate-config.js": "function fn() { return {}; }",
    });
    await runner.open(root);
    expect(last("project")).not.toHaveProperty("subfolders");
  });

  it("offers nothing after a failed dry run", T, async () => {
    const { runner, last } = setup();
    const root = write({ "sub/karate-pom.json": "{}" }, fixture("broken"));
    await runner.open(root);
    expect(last("project").error).toContain("syntax.feature");
    expect(last("project")).not.toHaveProperty("subfolders");
  });

  it("keeps the offer in the replay", T, async () => {
    const { runner } = setup();
    await runner.open(repo().root);
    expect(runner.replay()[0]).toMatchObject({ type: "project", subfolders: ["karate"] });
  });

  it("opens an offered folder as any project", T, async () => {
    const { runner, last, dataDir } = setup();
    const { root } = repo();
    await runner.open(root);
    await runner.openSubfolder("karate");
    const p = last("project");
    expect(p.project!.root).toBe(path.join(root, "karate"));
    expect(leafKeys(p.tree!)).toHaveLength(2);
    expect(loadRecentProjects(dataDir)[0]).toBe(path.join(root, "karate"));
  });

  it("refuses a folder it did not offer", T, async () => {
    const { runner, last } = setup();
    const { root } = repo();
    await runner.open(root);
    for (const folder of ["../elsewhere", "karate/"]) {
      await runner.openSubfolder(folder);
      expect(last("notice").message).toBe("Invalid request");
      expect(last("project").project!.root).toBe(root);
    }
    await runner.openSubfolder("karate");
    await runner.openSubfolder("karate"); // karate's own tree offers nothing
    expect(last("notice").message).toBe("Invalid request");
    expect(last("project").project!.root).toBe(path.join(root, "karate"));
  });
});
