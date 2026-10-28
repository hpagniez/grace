import { describe, expect, it } from "vitest";
import { compareVersions, isPrerelease, parseVersion } from "./version";

describe("parseVersion", () => {
  it("parses only X.Y.Z and X.Y.Z-rc.N", () => {
    expect(parseVersion("1.0.0-rc.4")).toEqual({ major: 1, minor: 0, patch: 0, rc: 4 });
    expect(parseVersion("1.2.3")).toEqual({ major: 1, minor: 2, patch: 3, rc: null });
    for (const bad of [
      "v1.0.0",
      "1.0",
      "1.0.0-rc.04",
      "01.0.0",
      "1.0.0-rc.4.1",
      "1.0.0-beta.1",
      "1.0.0-rc",
      "",
      "1.0.0\n",
    ])
      expect(parseVersion(bad)).toBeNull();
  });

  it("refuses anything that is not a string", () => {
    for (const bad of [["1.0.0"], 1, null, undefined])
      expect(parseVersion(bad as unknown as string)).toBeNull();
  });
});

describe("compareVersions", () => {
  it("orders by SemVer precedence", () => {
    const sorted = ["1.0.0", "1.0.0-rc.10", "0.9.9", "1.0.0-rc.9", "1.0.1-rc.1"].sort(
      compareVersions,
    );
    expect(sorted).toEqual(["0.9.9", "1.0.0-rc.9", "1.0.0-rc.10", "1.0.0", "1.0.1-rc.1"]);
  });

  it("is 0 for equal versions, release candidates or not", () => {
    expect(compareVersions("1.0.0", "1.0.0")).toBe(0);
    expect(compareVersions("1.0.0-rc.4", "1.0.0-rc.4")).toBe(0);
  });

  it("throws when either side is invalid", () => {
    expect(() => compareVersions("v1.0.0", "1.0.0")).toThrow();
    expect(() => compareVersions("1.0.0", "x")).toThrow();
  });
});

describe("isPrerelease", () => {
  it("is true only for a valid release candidate", () => {
    expect(isPrerelease("1.0.0-rc.0")).toBe(true);
    expect(isPrerelease("1.0.0")).toBe(false);
    expect(isPrerelease("1.0.0-beta.1")).toBe(false);
  });
});
