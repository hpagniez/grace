import { createHash } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { cleanUp, T, type Of, setup, fixture, printed, byName, until } from "../../test/runner";
import type { AppEvent } from "../shared/events";
import type { TestNode } from "../shared/model";
import { matchingLeaves, NO_FILTER } from "../shared/tags";
import { fakeSecretBox } from "./secrets";
import { loadProjectEnv, loadProjectFilter, saveProjectEnv } from "./settings";

afterEach(cleanUp);

describe("Runner environment", () => {
  const names = (tree: TestNode | null) =>
    tree ? matchingLeaves(tree, NO_FILTER).map((n) => n.name) : [];
  /** Event types after `from`, without the busy, opening and recent events. */
  const typesAfter = (events: AppEvent[], from: number) =>
    events
      .slice(from)
      .map((e) => e.type)
      .filter((t) => t !== "busy" && t !== "opening" && t !== "recent");

  it("suggestions from overlays and tags", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("env"));
    expect(last("environment").view).toMatchObject({
      env: null,
      suggestions: ["prod", "qa"],
      overlays: ["qa"],
    });
    expect(typesAfter(events, 0)).toEqual([
      "project",
      "flaky",
      "filter",
      "threads",
      "environment",
      "history",
      "flaky",
    ]);
  });

  it("setEnv rediscovers", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    expect(names(last("project").tree)).not.toContain("qa only");
    expect(names(last("project").tree)).toContain("not prod");
    const from = events.length;
    await runner.setEnv("qa");
    expect(typesAfter(events, from)).toEqual([
      "project",
      "flaky",
      "filter",
      "threads",
      "environment",
      "history",
      "flaky",
    ]);
    expect(names(last("project").tree)).toContain("qa only");
    expect(last("environment").view.env).toBe("qa");
    // the tag of a scenario prod lacks is pruned, as after a Refresh
    runner.setFilter({ include: ["env=qa"], exclude: [], hideNonMatching: false });
    await runner.setEnv("prod");
    const tree = last("project").tree;
    expect(names(tree)).not.toContain("qa only");
    expect(names(tree)).not.toContain("not prod");
    expect(last("filter").filter).toEqual(NO_FILTER);
    expect(loadProjectFilter(dataDir, root)).toEqual(NO_FILTER);
  });

  it("a run uses -e", T, async () => {
    const { runner, events, last } = setup();
    await runner.open(fixture("env"));
    await runner.setEnv("qa");
    await runner.run(byName(last("project").tree!, "always").key);
    expect(printed(events).some((l) => l.includes("karate.env = qa"))).toBe(true);
    expect(last("runStarted").env).toBe("qa");
    expect(runner.reportPath()).not.toBeNull();
    // a result from qa is not the state of prod
    await runner.setEnv("prod");
    expect(runner.reportPath()).toBeNull();
  });

  it("setEnv saves and open restores", T, async () => {
    const first = setup();
    const root = fixture("env");
    const properties = {
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: { qa: [{ name: "api.token", secret: fakeSecretBox().encrypt("tok-123") }] },
    };
    saveProjectEnv(first.dataDir, root, { env: null, properties });
    await first.runner.open(root);
    await first.runner.setEnv("qa");
    expect(loadProjectEnv(first.dataDir, root)).toEqual({ env: "qa", properties });
    await first.runner.dispose();
    const second = setup(first.dataDir);
    await second.runner.open(root);
    expect(second.last("environment").view.env).toBe("qa");
    expect(names(second.last("project").tree)).toContain("qa only");
    expect(second.runner.replay()).toEqual([
      second.runner.current(),
      { type: "filter", filter: NO_FILTER },
      { type: "threads", count: 1 },
      second.last("environment"),
      second.last("history"),
      second.last("flaky"),
      { type: "opening", root: null },
      { type: "busy", busy: false },
    ]);
  });

  it("the view shows secret rows without their values", T, async () => {
    const root = fixture("env");
    const dataDir = mkdtempSync(path.join(tmpdir(), "kk-data-"));
    const token = fakeSecretBox().encrypt("tok-123");
    saveProjectEnv(dataDir, root, {
      env: null,
      properties: {
        shared: [{ name: "timeout", value: "5000" }],
        byEnv: {
          qa: [
            { name: "api.token", secret: token },
            { name: "broken", secret: "Zm9v" },
          ],
          toString: [],
        },
      },
    });
    const { runner, last } = setup(dataDir);
    await runner.open(root);
    const { view } = last("environment");
    expect(view.suggestions).toEqual(["prod", "qa", "toString"]);
    expect(view.secretsAvailable).toBe(true);
    expect(view.properties).toEqual({
      shared: [{ name: "timeout", value: "5000" }],
      byEnv: {
        qa: [
          { name: "api.token", secret: true, state: "set" },
          { name: "broken", secret: true, state: "unreadable" },
        ],
        toString: [],
      },
    });
    expect(JSON.stringify(view)).not.toMatch(/tok-123|Zm9v/);
    expect(JSON.stringify(view)).not.toContain(token);

    const off = setup(dataDir, fakeSecretBox(false));
    await off.runner.open(root);
    const unavailable = off.last("environment").view;
    expect(unavailable.secretsAvailable).toBe(false);
    expect(unavailable.properties.byEnv.qa[0]).toEqual({
      name: "api.token",
      secret: true,
      state: "unreadable",
    });
  });

  it("cancel during setEnv restores the previous environment", T, async () => {
    const { runner, events, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    await runner.setEnv("qa");
    const before = runner.current();
    const view = last("environment");
    const from = events.length;
    const switching = runner.setEnv("prod");
    await runner.cancel();
    await switching;
    expect(typesAfter(events, from)).toEqual([
      "project",
      "filter",
      "threads",
      "environment",
      "history",
      "flaky",
      "notice",
    ]);
    expect(all("project").at(-1)).toEqual(before);
    expect(last("environment")).toEqual(view);
    expect(last("notice").message).toBe("Cancelled");
    expect(loadProjectEnv(dataDir, root).env).toBe("qa");
    // the restored environment is the one discovery uses
    await runner.refresh();
    expect(names(last("project").tree)).toContain("qa only");
  });

  it("a parse error still applies and saves the environment", T, async () => {
    const { runner, last, dataDir } = setup();
    const root = fixture("broken");
    await runner.open(root);
    await runner.setEnv("qa");
    expect(last("project").error).toContain("syntax.feature");
    expect(last("environment").view.env).toBe("qa");
    expect(loadProjectEnv(dataDir, root).env).toBe("qa");
  });

  it("invalid env name is refused", T, async () => {
    const { runner, events, all, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    for (const name of ["bad name", "-Dx=1", "", "a".repeat(65)]) {
      const from = events.length;
      await runner.setEnv(name);
      expect(last("notice").message).toBe("Invalid environment name");
      // the view in force is re-sent, so a window that shows the typed name reverts
      expect(typesAfter(events, from)).toEqual(["notice", "environment"]);
    }
    await runner.setEnv(null); // the same environment: nothing to do
    expect(all("project")).toHaveLength(1);
    expect(last("environment").view.env).toBeNull();
    expect(loadProjectEnv(dataDir, root).env).toBeNull();
  });

  it("setEnv while busy gets the busy notice", T, async () => {
    const { runner, events, all, last } = setup();
    await runner.open(fixture("env"));
    const run = runner.run(byName(last("project").tree!, "slow").key);
    await until(() => all("runStarted").length > 0);
    const from = events.length;
    await runner.setEnv("qa");
    expect(last("notice").message).toBe("A run is already in progress");
    expect(typesAfter(events, from).filter((t) => t === "notice" || t === "environment")).toEqual([
      "notice",
      "environment",
    ]);
    await runner.cancel();
    await run;
    expect(all("environment")).toHaveLength(2);
    expect(last("environment").view.env).toBeNull();
  });

  it("a change refused during a discovery re-sends the environment in force", T, async () => {
    const { runner, events, all, last } = setup();
    await runner.open(fixture("env"));
    const switching = runner.setEnv("qa");
    await until(() => all("busy").at(-1)?.busy === true);
    const from = events.length;
    await runner.setEnv("prod");
    // the window must not show the environment still being discovered
    expect(typesAfter(events, from)).toEqual(["notice", "environment"]);
    expect(last("notice").message).toBe("A run is already in progress");
    expect(last("environment").view.env).toBeNull();
    const replayed = runner.replay().find((e): e is Of<"environment"> => e.type === "environment");
    expect(replayed?.view.env).toBeNull();
    await switching;
    expect(last("environment").view.env).toBe("qa");
  });

  it("a discovery that throws keeps the environment before", T, async () => {
    const { runner, events, last, dataDir } = setup();
    const root = fixture("env");
    await runner.open(root);
    const before = runner.replay();
    // a file where the run folders go: emptying the dry run folder throws
    const runs = path.join(
      dataDir,
      "projects",
      createHash("sha1").update(root).digest("hex").slice(0, 12),
    );
    rmSync(runs, { recursive: true, force: true, maxRetries: 5 });
    writeFileSync(runs, "");
    const from = events.length;
    await expect(runner.setEnv("qa")).rejects.toThrow();
    // the window still shows the name it sent until main re-sends the environment in force
    expect(typesAfter(events, from)).toEqual(["environment"]);
    expect(last("environment").view.env).toBeNull();
    expect(runner.replay()).toEqual(before);
    expect(loadProjectEnv(dataDir, root).env).toBeNull();
    rmSync(runs);
    await runner.refresh(); // discovers with the environment before
    expect(names(last("project").tree)).not.toContain("qa only");
  });

  it("KARATE_ENV in the parent environment is ignored", T, async () => {
    const saved = process.env.KARATE_ENV;
    process.env.KARATE_ENV = "prod";
    try {
      const { runner, events, last } = setup();
      await runner.open(fixture("env"));
      expect(names(last("project").tree)).toContain("not prod");
      await runner.run(byName(last("project").tree!, "always").key);
      expect(printed(events).some((l) => l.includes("karate.env = null"))).toBe(true);
      expect(last("runStarted").env).toBeNull();
    } finally {
      if (saved === undefined) delete process.env.KARATE_ENV;
      else process.env.KARATE_ENV = saved;
    }
  });
});
