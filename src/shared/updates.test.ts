import { describe, expect, it } from "vitest";
import {
  autoUpdateTo,
  defaultUpdateSettings,
  downloadPrefix,
  latestOn,
  onChannel,
  parseReleases,
  promptFor,
  releasePage,
  zipName,
  type Release,
} from "./updates";
import { isPrerelease } from "./version";

interface GhOptions {
  prerelease?: boolean;
  draft?: boolean;
  assets?: unknown[];
  digest?: string;
  url?: string;
}

/** A GitHub API release entry; by default one that is installable. */
const gh = (tag: string, o: GhOptions = {}) => {
  const v = tag.slice(1);
  return {
    tag_name: tag,
    draft: o.draft ?? false,
    prerelease: o.prerelease ?? isPrerelease(v),
    published_at: "2026-10-28T10:00:00Z",
    assets: o.assets ?? [
      {
        name: zipName(v),
        size: 218_000_000,
        digest: o.digest ?? `sha256:${"ab".repeat(32)}`,
        browser_download_url: o.url ?? `${downloadPrefix(v)}${zipName(v)}`,
      },
    ],
  };
};

const r = (version: string, prerelease = isPrerelease(version)): Release => ({
  version,
  prerelease,
  publishedAt: "2026-10-28T10:00:00Z",
  url: `${downloadPrefix(version)}${zipName(version)}`,
  size: 218_000_000,
  sha256: "ab".repeat(32),
});

const rs = parseReleases([gh("v1.0.0-rc.5"), gh("v1.0.0-rc.6")]);

describe("names and links", () => {
  it("builds them from a validated version", () => {
    expect(zipName("1.0.0-rc.5")).toBe("Grace-win32-x64-1.0.0-rc.5.zip");
    expect(downloadPrefix("1.0.0-rc.5")).toBe(
      "https://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/",
    );
    expect(releasePage("1.0.0-rc.5")).toBe(
      "https://github.com/hpagniez/grace/releases/tag/v1.0.0-rc.5",
    );
  });
});

describe("parseReleases", () => {
  it("keeps installable releases only (R1.4)", () => {
    const parsed = parseReleases([
      gh("v1.0.0-rc.5"),
      gh("v1.0.0-rc.3"),
      gh("v1.0.0-rc.6", { draft: true }),
      gh("v2.0.0-beta"),
      gh("v1.0.0-rc.7", { assets: [] }),
      gh("v1.0.0-rc.8", { digest: "md5:abc" }),
      gh("v1.0.0-rc.9", { url: "https://evil.example/Grace-win32-x64-1.0.0-rc.9.zip" }),
    ]);
    expect(parsed.map((x) => x.version)).toEqual(["1.0.0-rc.5"]); // rc.3 is below FIRST_UPDATER
  });

  it("reads the fields Grace uses", () => {
    expect(rs[1]).toEqual({
      version: "1.0.0-rc.5",
      prerelease: true,
      publishedAt: "2026-10-28T10:00:00Z",
      url: "https://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/Grace-win32-x64-1.0.0-rc.5.zip",
      size: 218_000_000,
      sha256: "ab".repeat(32),
    });
  });

  it("lists newest first, one per version", () => {
    const parsed = parseReleases([
      gh("v1.0.0-rc.5"),
      gh("v1.0.0-rc.10"),
      gh("v1.0.0-rc.5"),
      gh("v1.0.0-rc.9"),
      gh("v1.0.0", { prerelease: false }),
    ]);
    expect(parsed.map((x) => x.version)).toEqual([
      "1.0.0",
      "1.0.0-rc.10",
      "1.0.0-rc.9",
      "1.0.0-rc.5",
    ]);
  });

  it("rejects an entry that fails any one field", () => {
    const [zip] = gh("v1.0.0-rc.5").assets as Record<string, unknown>[];
    const bad = [
      { ...gh("v1.0.0-rc.5"), tag_name: "1.0.0-rc.5" },
      { ...gh("v1.0.0-rc.5"), draft: undefined },
      { ...gh("v1.0.0-rc.5"), prerelease: undefined },
      { ...gh("v1.0.0-rc.5"), assets: "zip" },
      { ...gh("v1.0.0-rc.5"), assets: [zip, zip] },
      { ...gh("v1.0.0-rc.5"), assets: [{ ...zip, name: "Grace-win32-x64-1.0.0-rc.6.zip" }] },
      { ...gh("v1.0.0-rc.5"), assets: [{ ...zip, size: 0 }] },
      { ...gh("v1.0.0-rc.5"), assets: [{ ...zip, size: "218" }] },
      { ...gh("v1.0.0-rc.5"), assets: [{ ...zip, digest: `sha256:${"AB".repeat(32)}` }] },
      { ...gh("v1.0.0-rc.5"), assets: [{ ...zip, digest: "sha256:abc" }] },
      { ...gh("v1.0.0-rc.5"), assets: [{ ...zip, browser_download_url: undefined }] },
      gh("v1.0.0-rc.5", {
        url: "http://github.com/hpagniez/grace/releases/download/v1.0.0-rc.5/x.zip",
      }),
    ];
    for (const b of bad) expect(parseReleases([b])).toEqual([]);
  });

  it("ignores other assets next to the zip", () => {
    const e = gh("v1.0.0-rc.5");
    e.assets.push({ name: "notes.txt", size: 5, digest: "sha256:x", browser_download_url: "x" });
    expect(parseReleases([e]).map((x) => x.version)).toEqual(["1.0.0-rc.5"]);
  });

  it("ignores anything that is not an array of objects", () => {
    for (const x of [null, {}, "<html>", [1, "a", null]]) expect(parseReleases(x)).toEqual([]);
  });
});

describe("channels", () => {
  it("stable channel hides pre-releases", () => {
    expect(latestOn(rs, "stable")).toBeNull();
    expect(latestOn(rs, "prerelease")?.version).toBe("1.0.0-rc.6");
  });

  it("pre-release channel also holds the stable releases", () => {
    const all = [r("1.0.0"), r("1.0.0-rc.6")];
    expect(onChannel(all, "stable").map((x) => x.version)).toEqual(["1.0.0"]);
    expect(onChannel(all, "prerelease").map((x) => x.version)).toEqual(["1.0.0", "1.0.0-rc.6"]);
    expect(latestOn([r("1.0.0-rc.6"), r("1.0.0")], "prerelease")?.version).toBe("1.0.0");
  });

  it("has no latest in an empty list", () => {
    expect(latestOn([], "prerelease")).toBeNull();
  });
});

describe("defaultUpdateSettings", () => {
  it("defaults the channel from the running version (R1.3)", () => {
    expect(defaultUpdateSettings("1.0.0-rc.5").channel).toBe("prerelease");
    expect(defaultUpdateSettings("1.0.0").channel).toBe("stable");
  });

  it("checks at startup, never updates by itself, remembers nothing (R12)", () => {
    expect(defaultUpdateSettings("1.0.0")).toEqual({
      channel: "stable",
      checkAtStartup: true,
      autoUpdate: false,
      skipped: null,
      failed: null,
      lastChecked: null,
    });
  });
});

describe("promptFor", () => {
  it("prompts once per version above the skipped one (R4.1)", () => {
    const s = { ...defaultUpdateSettings("1.0.0-rc.5"), skipped: "1.0.0-rc.6" };
    expect(promptFor([r("1.0.0-rc.6")], s, "1.0.0-rc.5")).toBeNull();
    expect(promptFor([r("1.0.0-rc.7"), r("1.0.0-rc.6")], s, "1.0.0-rc.5")?.version).toBe(
      "1.0.0-rc.7",
    );
    expect(promptFor([r("1.0.0-rc.7")], { ...s, autoUpdate: true }, "1.0.0-rc.5")).toBeNull();
  });

  it("prompts for latest when nothing is skipped, and not when up to date", () => {
    const s = defaultUpdateSettings("1.0.0-rc.5");
    expect(promptFor([r("1.0.0-rc.6")], s, "1.0.0-rc.5")?.version).toBe("1.0.0-rc.6");
    expect(promptFor([r("1.0.0-rc.5")], s, "1.0.0-rc.5")).toBeNull();
    expect(promptFor([r("1.0.0-rc.4")], s, "1.0.0-rc.5")).toBeNull();
    expect(promptFor([], s, "1.0.0-rc.5")).toBeNull();
  });

  it("only looks at the chosen channel", () => {
    const s = { ...defaultUpdateSettings("1.0.0"), channel: "stable" as const };
    expect(promptFor([r("1.0.1-rc.1")], s, "1.0.0")).toBeNull();
  });
});

describe("autoUpdateTo", () => {
  it("ignores skipped but never installs failed (R5.2)", () => {
    const s = {
      ...defaultUpdateSettings("1.0.0-rc.5"),
      autoUpdate: true,
      skipped: "1.0.0-rc.6",
      failed: "1.0.0-rc.7",
    };
    expect(autoUpdateTo([r("1.0.0-rc.6")], s, "1.0.0-rc.5")?.version).toBe("1.0.0-rc.6");
    expect(autoUpdateTo([r("1.0.0-rc.7"), r("1.0.0-rc.6")], s, "1.0.0-rc.5")).toBeNull();
  });

  it("does nothing when auto-update is off or Grace is up to date", () => {
    const s = defaultUpdateSettings("1.0.0-rc.5");
    expect(autoUpdateTo([r("1.0.0-rc.6")], s, "1.0.0-rc.5")).toBeNull();
    expect(autoUpdateTo([r("1.0.0-rc.5")], { ...s, autoUpdate: true }, "1.0.0-rc.5")).toBeNull();
  });
});
