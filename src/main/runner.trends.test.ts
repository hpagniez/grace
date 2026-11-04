import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, T, setup, fixture, leaves, seed } from "../../test/runner";
import type { StoredTest } from "../shared/history";

afterEach(cleanUp);

describe("Runner trend", () => {
  const hello: StoredTest = {
    kind: "scenario",
    file: "tests/basic.feature",
    refId: "[1:7]",
    name: "passing hello",
    line: 7,
    key: "s:tests/basic.feature#1",
    status: "passed",
    durationMillis: 400,
  };

  it("gives the trend of the open project", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    const ids = seed(dataDir, root, 2, { tests: [hello] });
    await runner.open(root);
    const all = runner.trend(null);
    expect(all.map((p) => p.id)).toEqual(ids); // oldest first
    expect(all[0]).toMatchObject({ passed: 1, failed: 0, total: leaves(last("project").tree) });
    expect(runner.trend("s:tests/basic.feature#1")).toMatchObject([{ total: 1 }, { total: 1 }]);
  });

  it("gives nothing with no project", () => {
    expect(setup().runner.trend(null)).toEqual([]);
  });
});
