import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, T, setup, fixture, byName, record, recordFile, seed } from "../../test/runner";
import type { StoredTest } from "../shared/history";

afterEach(cleanUp);

describe("Runner flaky", () => {
  const FP = "a".repeat(64);
  const hello = (status: StoredTest["status"]): StoredTest => ({
    kind: "scenario",
    file: "tests/basic.feature",
    refId: "[1:7]",
    name: "passing hello",
    line: 7,
    key: "s:tests/basic.feature#1",
    status,
  });
  /** Two runs of the same files: `passing hello` passed in the older and failed in the newer. */
  function seedFlaky(dataDir: string, root: string) {
    const [, newer] = seed(dataDir, root, 2, { fingerprint: FP, tests: [hello("passed")] });
    const file = recordFile(dataDir, root, newer);
    writeFileSync(
      file,
      JSON.stringify({ ...record(dataDir, root, newer), tests: [hello("failed")] }),
    );
  }

  it("sends the flaky tests with the history", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    seedFlaky(dataDir, root);
    await runner.open(root);
    expect(last("flaky").tests).toMatchObject([
      { key: "s:tests/basic.feature#1", envs: [{ env: null, passed: 1, failed: 1 }] },
    ]);
    expect(runner.replay()).toContainEqual(last("flaky"));
  });

  it("follows a new tree", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("clean");
    seedFlaky(dataDir, root);
    await runner.open(root);
    const file = path.join(root, "tests", "basic.feature");
    writeFileSync(
      file,
      readFileSync(file, "utf8").replace(
        /(@smoke\r?\nScenario: passing hello)/,
        "Scenario: added\n  * match 1 == 1\n\n$1",
      ),
    );
    await runner.refresh();
    const key = byName(last("project").tree!, "passing hello").key;
    expect(key).not.toBe("s:tests/basic.feature#1");
    // sent with the new tree, before the history that follows it
    expect(events[events.lastIndexOf(last("project")) + 1]).toMatchObject({
      type: "flaky",
      tests: [{ key }],
    });
    expect(last("flaky").tests.map((t) => t.key)).toEqual([key]);
  });

  it("clear history leaves no flaky tests", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    seedFlaky(dataDir, root);
    await runner.open(root);
    expect(last("flaky").tests).toHaveLength(1);
    runner.clearHistory();
    expect(last("history").runs).toEqual([]);
    expect(last("flaky").tests).toEqual([]);
  });
});
