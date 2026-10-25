import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { copyFixture } from "../../../test/fixtures/copy";
import { featureTagEnvs, overlayEnvs } from "./environments";

const temps: string[] = [];
const tmp = () => {
  const d = mkdtempSync(path.join(tmpdir(), "kk-env-"));
  temps.push(d);
  return d;
};
afterEach(() => {
  for (const d of temps.splice(0)) rmSync(d, { recursive: true, force: true, maxRetries: 5 });
});

/** Writes files (relative path -> text) under `root`, creating folders. */
function put(root: string, files: Record<string, string>) {
  for (const [rel, text] of Object.entries(files)) {
    const file = path.join(root, ...rel.split("/"));
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, text);
  }
}

describe("overlayEnvs", () => {
  it("finds overlay files in each layout", () => {
    const standalone = tmp();
    put(standalone, { "karate-config-qa.js": "", "karate-config-dev.js": "" });
    expect(overlayEnvs([standalone])).toEqual(["dev", "qa"]);

    const maven = tmp();
    put(maven, {
      "karate-config-local.js": "",
      "src/test/java/karate-config-qa.js": "",
      "src/test/resources/karate-config-qa.js": "",
      "src/test/resources/karate-config-prod.js": "",
    });
    const java = path.join(maven, "src", "test", "java");
    const resources = path.join(maven, "src", "test", "resources");
    expect(overlayEnvs([maven, java, resources])).toEqual(["local", "prod", "qa"]);
  });

  it("ignores karate-config.js, bad names, other files and nested folders", () => {
    const root = tmp();
    put(root, {
      "karate-config.js": "",
      "karate-config-.js": "",
      "karate-config--x.js": "",
      "karate-config-a b.js": "",
      "karate-config-qa.json": "",
      "karate-config-qa.js.bak": "",
      "my-karate-config-qa.js": "",
      "karate-config-ok.js": "",
      "sub/karate-config-nested.js": "",
    });
    expect(overlayEnvs([root])).toEqual(["ok"]);
  });

  it("matches the file name case-insensitively and keeps the environment as written", () => {
    const root = tmp();
    put(root, { "Karate-Config-QA.js": "", "KARATE-CONFIG-dev.JS": "" });
    expect(overlayEnvs([root])).toEqual(["QA", "dev"]);
  });

  it("skips folders that do not exist", () => {
    const root = tmp();
    put(root, { "karate-config-qa.js": "" });
    expect(overlayEnvs([path.join(root, "nope"), root])).toEqual(["qa"]);
    expect(overlayEnvs([])).toEqual([]);
  });
});

describe("featureTagEnvs", () => {
  it("reads tag values from the env fixture", async () => {
    expect(await featureTagEnvs([copyFixture("env")])).toEqual(["prod", "qa"]);
  });

  it("handles several tags per line, Feature tags, lists, and comments", async () => {
    const root = tmp();
    put(root, {
      "a.feature": [
        "@smoke @env=dev,qa",
        "Feature: a",
        "",
        "  # @env=commented",
        "  @envnot=prod @slow",
        "  Scenario: one",
        "    * print '@env=instep'",
        "",
        "@env=bad/name,ok-1,",
        "Scenario: two",
      ].join("\n"),
      "deep/er/b.feature": "@env=staging\r\nFeature: b\r\n",
      "deep/er/c.txt": "@env=notafeature",
    });
    expect(await featureTagEnvs([root])).toEqual(["dev", "ok-1", "prod", "qa", "staging"]);
  });

  it("merges several roots and survives roots that do not exist", async () => {
    const a = tmp();
    const b = tmp();
    put(a, { "x.feature": "@env=a\nFeature: x" });
    put(b, { "y.feature": "@env=b\nFeature: y" });
    expect(await featureTagEnvs([a, path.join(a, "missing"), b])).toEqual(["a", "b"]);
    expect(await featureTagEnvs([])).toEqual([]);
  });

  it("skips target, build, node_modules and .git", async () => {
    const root = tmp();
    put(root, {
      "keep/x.feature": "@env=kept\nFeature: x",
      "target/x.feature": "@env=in-target\nFeature: x",
      "build/x.feature": "@env=in-build\nFeature: x",
      "node_modules/pkg/x.feature": "@env=in-modules\nFeature: x",
      ".git/x.feature": "@env=in-git\nFeature: x",
      "keep/target/x.feature": "@env=nested-target\nFeature: x",
    });
    expect(await featureTagEnvs([root])).toEqual(["kept"]);
  });

  it("does not block the event loop while it scans", async () => {
    const root = tmp();
    put(root, {
      "a.feature": "@env=a\nFeature: a",
      "x/b.feature": "@env=b\nFeature: b",
      "x/y/c.feature": "@env=c\nFeature: c",
    });
    let ticked = false;
    setImmediate(() => (ticked = true));
    expect(await featureTagEnvs([root])).toEqual(["a", "b", "c"]);
    expect(ticked).toBe(true);
  });
});
