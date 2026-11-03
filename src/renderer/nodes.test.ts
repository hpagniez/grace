import { describe, expect, it } from "vitest";
import { featureOf } from "./nodes";

describe("featureOf (#224)", () => {
  it("reads the feature of a feature, scenario, example row or outline key", () => {
    expect(featureOf("f:tests/a.feature")).toBe("f:tests/a.feature");
    expect(featureOf("s:tests/a.feature#3")).toBe("f:tests/a.feature");
    expect(featureOf("s:tests/a.feature#3.2")).toBe("f:tests/a.feature");
    expect(featureOf("o:tests/a.feature#3")).toBe("f:tests/a.feature");
  });

  it("keeps a # in the path", () => {
    expect(featureOf("s:tests/a#b/x.feature#2")).toBe("f:tests/a#b/x.feature");
  });

  it("gives null for a folder, the root or no key", () => {
    expect(featureOf("d:tests")).toBeNull();
    expect(featureOf("d:")).toBeNull();
    expect(featureOf(null)).toBeNull();
  });
});
