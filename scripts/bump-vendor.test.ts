import { createHash } from "node:crypto";
import { copyFileSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  PINS_PATH,
  bump,
  compareVersions,
  parseJreVersion,
  parseKarateVersion,
  pickJre,
  pickKarate,
} from "./bump-vendor.mjs";

const HEX = "a".repeat(64);
const ADOPTIUM =
  "https://api.adoptium.net/v3/assets/latest/21/hotspot?os=windows&architecture=x64&image_type=jre";
const RELEASES = "https://api.github.com/repos/karatelabs/karate/releases?per_page=100";

const jreAsset = (version: string, checksum: string | null = HEX) => [
  {
    release_name: `jdk-${version}`,
    binary: {
      package: {
        name: "OpenJDK21U-jre_x64_windows_hotspot.zip",
        link: `https://github.com/adoptium/temurin21-binaries/releases/download/jdk-${version.replace("+", "%2B")}/OpenJDK21U-jre_x64_windows_hotspot.zip`,
        checksum,
      },
    },
  },
];

const release = (tag: string, extra: Record<string, unknown> = {}) => {
  const ver = tag.slice(1);
  return {
    tag_name: tag,
    draft: false,
    prerelease: false,
    assets: [
      {
        name: `karate-${ver}.jar`,
        digest: `sha256:${HEX}`,
        browser_download_url: `https://github.com/karatelabs/karate/releases/download/${tag}/karate-${ver}.jar`,
      },
    ],
    ...extra,
  };
};

/** A fetch that answers each URL from `routes`, and 404 for any other. */
const fakeFetch = (routes: Record<string, Response>) =>
  (async (url: string) => routes[url] ?? new Response("", { status: 404 })) as typeof fetch;

const json = (x: unknown) => new Response(JSON.stringify(x));

function tempPins() {
  const file = join(mkdtempSync(join(tmpdir(), "kk-bump-")), "vendor.json");
  copyFileSync(PINS_PATH, file);
  return file;
}

describe("parseJreVersion", () => {
  it("reads four- and three-part Temurin versions", () => {
    expect(parseJreVersion("21.0.12.1+1")).toEqual([21, 0, 12, 1, 1]);
    expect(parseJreVersion("21.0.13+11")).toEqual([21, 0, 13, 0, 11]);
  });

  it("rejects anything else", () => {
    expect(parseJreVersion("21.0.13+11-ea")).toBeNull();
    expect(parseJreVersion("2.1.4")).toBeNull();
  });
});

describe("compareVersions", () => {
  const jre = (v: string) => parseJreVersion(v)!;
  const karate = (v: string) => parseKarateVersion(v)!;

  it("compares part by part as numbers", () => {
    expect(compareVersions(jre("21.0.13+11"), jre("21.0.12.1+1"))).toBeGreaterThan(0);
    expect(compareVersions(jre("21.0.13.1+2"), jre("21.0.13+11"))).toBeGreaterThan(0);
    expect(compareVersions(karate("2.1.10"), karate("2.1.9"))).toBeGreaterThan(0);
    expect(compareVersions(karate("2.1.4"), karate("2.1.4"))).toBe(0);
    expect(compareVersions(karate("2.1.3"), karate("2.1.4"))).toBeLessThan(0);
  });
});

describe("pickKarate", () => {
  it.each([
    ["v2.1.4 is the newest", [release("v2.1.4")]],
    ["v2.1.3 is the newest", [release("v2.1.3")]],
    ["only a new major", [release("v3.0.0"), release("v2.1.4")]],
    ["a pre-release", [release("v2.2.0", { prerelease: true }), release("v2.1.4")]],
    ["a draft", [release("v2.2.0", { draft: true }), release("v2.1.4")]],
    ["a release candidate tag", [release("v2.2.0.RC1"), release("v2.1.4")]],
  ])("keeps 2.1.4 when %s", (_, releases) => {
    expect(pickKarate(releases, "2.1.4")).toBeNull();
  });

  it("picks the highest by number, not by text", () => {
    expect(pickKarate([release("v2.1.9"), release("v2.1.10")], "2.1.9")).toEqual({
      version: "2.1.10",
      url: "https://github.com/karatelabs/karate/releases/download/v2.1.10/karate-2.1.10.jar",
      sha256: HEX,
      file: "karate-2.1.10.jar",
    });
  });

  it("fails when the jar asset is missing", () => {
    expect(() => pickKarate([release("v2.1.5", { assets: [] })], "2.1.4")).toThrow(
      /has no karate-2.1.5.jar asset/,
    );
  });

  it("fails when the jar has no sha256 digest", () => {
    const r = release("v2.1.5");
    r.assets[0].digest = null as unknown as string;
    expect(() => pickKarate([r], "2.1.4")).toThrow(/karate-2.1.5.jar has no sha256 digest/);
  });

  it("fails when the pinned version does not parse", () => {
    expect(() => pickKarate([release("v2.1.5")], "2.1")).toThrow(
      /vendor.json karate version "2.1" does not parse/,
    );
  });
});

describe("pickJre", () => {
  it("keeps the pin when Adoptium's latest is the pinned build", () => {
    expect(pickJre(jreAsset("21.0.12.1+1"), "21.0.12.1+1")).toBeNull();
  });

  it("picks a newer build with Adoptium's link and checksum", () => {
    const [a] = jreAsset("21.0.13+11");
    expect(pickJre([a], "21.0.12.1+1")).toEqual({
      version: "21.0.13+11",
      url: a.binary.package.link,
      sha256: HEX,
    });
  });

  it("fails without a checksum", () => {
    expect(() => pickJre(jreAsset("21.0.13+11", null), "21.0.12.1+1")).toThrow(/checksum/);
  });

  it("fails on a release name that is not a Java version", () => {
    expect(() => pickJre(jreAsset("21.0.13+11-ea"), "21.0.12.1+1")).toThrow(
      /is not a Java version/,
    );
  });
});

describe("bump", () => {
  it("bumps Karate and its LICENSE, and nothing else", async () => {
    const pinsPath = tempPins();
    const before = JSON.parse(readFileSync(pinsPath, "utf8"));
    const fetch = fakeFetch({
      [RELEASES]: json([release("v2.1.5"), release("v2.1.4")]),
      "https://raw.githubusercontent.com/karatelabs/karate/v2.1.5/LICENSE": new Response("MIT"),
    });
    await expect(bump("karate", { pinsPath, fetch })).resolves.toEqual({
      from: "2.1.4",
      to: "2.1.5",
    });
    const text = readFileSync(pinsPath, "utf8");
    const after = JSON.parse(text);
    expect(after.karate).toEqual({
      version: "2.1.5",
      url: "https://github.com/karatelabs/karate/releases/download/v2.1.5/karate-2.1.5.jar",
      sha256: HEX,
      file: "karate-2.1.5.jar",
    });
    expect(after.license).toEqual({
      url: "https://raw.githubusercontent.com/karatelabs/karate/v2.1.5/LICENSE",
      sha256: createHash("sha256").update("MIT").digest("hex"),
      file: "karate-LICENSE.txt",
    });
    expect(after.jre).toEqual(before.jre);
    expect(text.endsWith("}\n")).toBe(true);
    expect(text).toBe(JSON.stringify(after, null, 2) + "\n");
  });

  it.each([
    ["the releases API answers 500", { [RELEASES]: new Response("", { status: 500 }) }],
    ["the LICENSE answers 404", { [RELEASES]: json([release("v2.1.5")]) }],
  ])("fails and changes nothing when %s", async (_, routes) => {
    const pinsPath = tempPins();
    const before = readFileSync(pinsPath, "utf8");
    await expect(bump("karate", { pinsPath, fetch: fakeFetch(routes) })).rejects.toThrow(
      /GET https:\/\/\S+ failed: (500|404)/,
    );
    expect(readFileSync(pinsPath, "utf8")).toBe(before);
  });

  it("leaves an up-to-date JRE alone", async () => {
    const pinsPath = tempPins();
    const before = readFileSync(pinsPath, "utf8");
    const pinned = JSON.parse(before).jre.version;
    const fetch = fakeFetch({ [ADOPTIUM]: json(jreAsset(pinned)) });
    await expect(bump("jre", { pinsPath, fetch })).resolves.toBeNull();
    expect(readFileSync(pinsPath, "utf8")).toBe(before);
  });
});
