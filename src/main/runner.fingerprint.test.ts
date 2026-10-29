import { writeFileSync } from "node:fs";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, T, setup, fixture, byName, until, folders, record } from "../../test/runner";
import { projectFingerprint } from "./fingerprint";
import { Runner } from "./runner";

afterEach(cleanUp);

describe("Runner fingerprint", () => {
  it("a run of an unchanged project stores its fingerprint", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    const [id] = folders(dataDir, root);
    const { fingerprint } = record(dataDir, root, id);
    expect(fingerprint).not.toBeNull();
    expect(fingerprint).toBe(await projectFingerprint(root));
  });

  it("a file saved during a run stores no fingerprint", T, async () => {
    const { runner, all, dataDir } = setup();
    const root = fixture("clean");
    await runner.open(root);
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    writeFileSync(path.join(root, "note.txt"), "saved during the run");
    await run;
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id).fingerprint).toBeNull();
  });

  it("different walks store no fingerprint", T, async () => {
    const walks = ["a".repeat(64), "b".repeat(64)];
    const fake = async () => walks.shift()!;
    const { runner, last, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    expect(walks).toEqual([]);
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id).fingerprint).toBeNull();
  });

  it("a cancel during the first walk starts no Java", T, async () => {
    let runner: Runner | undefined;
    const fake = async () => {
      await runner!.cancel();
      return "a".repeat(64);
    };
    const s = setup(undefined, undefined, undefined, fake);
    runner = s.runner;
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(s.last("project").tree!, "passing hello").key);
    expect(s.all("runStarted")).toHaveLength(0);
    expect(s.last("notice").message).toBe("Run cancelled");
    expect(folders(s.dataDir, root)).toEqual([]);
  });

  it("dispose skips the second walk", T, async () => {
    let walks = 0;
    const fake = async () => {
      walks++;
      return "a".repeat(64);
    };
    const { runner, all, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    const run = runner.run("f:tests/slow.feature");
    await until(() => all("runStarted").length > 0);
    await runner.dispose();
    expect(walks).toBe(1);
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id)).toMatchObject({ outcome: "cancelled", fingerprint: null });
    await run;
  });

  it("a first walk that gave null skips the second", T, async () => {
    let walks = 0;
    const fake = async () => {
      walks++;
      return null;
    };
    const { runner, last, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    expect(walks).toBe(1);
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id)).toMatchObject({ outcome: "finished", fingerprint: null });
  });

  it("the stored duration ends when Java exits, not after the second walk", T, async () => {
    let walks = 0;
    let secondWalkAt = 0;
    const fake = async () => {
      if (++walks === 2) {
        secondWalkAt = Date.now();
        await new Promise((resolve) => setTimeout(resolve, 300));
      }
      return "a".repeat(64);
    };
    const { runner, last, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    const [id] = folders(dataDir, root);
    const { startedAt, durationMillis } = record(dataDir, root, id);
    // the walk starts right after the exit, so a run that ended there is over by then
    expect(startedAt + durationMillis).toBeLessThanOrEqual(secondWalkAt);
  });

  it("the second walk comes before runFinished", T, async () => {
    let walks = 0;
    let finishedAtSecondWalk: boolean | undefined;
    const fake = async () => {
      if (++walks === 2) finishedAtSecondWalk = s.all("runFinished").length > 0;
      return "a".repeat(64);
    };
    const s = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await s.runner.open(root);
    await s.runner.run(byName(s.last("project").tree!, "passing hello").key);
    expect(finishedAtSecondWalk).toBe(false);
    const types = s.events.map((e) => e.type);
    expect(types.lastIndexOf("runFinished")).toBeLessThan(types.lastIndexOf("history"));
  });

  it("a cancel during the second walk keeps the run's outcome", T, async () => {
    let runner: Runner | undefined;
    let walks = 0;
    const fake = async () => {
      if (++walks === 2) await runner!.cancel();
      return "a".repeat(64);
    };
    const s = setup(undefined, undefined, undefined, fake);
    runner = s.runner;
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(s.last("project").tree!, "passing hello").key);
    expect(s.last("runFinished").outcome).toBe("finished");
    const [id] = folders(s.dataDir, root);
    expect(record(s.dataDir, root, id)).toMatchObject({
      outcome: "finished",
      fingerprint: "a".repeat(64),
    });
  });

  it("a failing second walk still ends the run", T, async () => {
    let walks = 0;
    const fake = async () => {
      if (++walks === 2) throw new Error("walk failed");
      return "a".repeat(64);
    };
    const { runner, all, last, dataDir } = setup(undefined, undefined, undefined, fake);
    const root = fixture("clean");
    await runner.open(root);
    await runner.run(byName(last("project").tree!, "passing hello").key);
    expect(all("runFinished")).toHaveLength(1);
    const [id] = folders(dataDir, root);
    expect(record(dataDir, root, id)).toMatchObject({ outcome: "finished", fingerprint: null });
  });
});
