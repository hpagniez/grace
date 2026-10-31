import { describe, expect, it } from "vitest";
import { judge, makeLimits, raisedLimits } from "./judge.mjs";
import { MEASURES, RUN_PHASE } from "./measures.mjs";

type Values = Record<string, number | null>;
type Entry = {
  unit: string;
  limit: number;
  noise: number;
  aim?: number;
  better?: "higher";
  machine: boolean;
};

type Round = { build: "a" | "b"; warmup: boolean; load: number; values: Values };

/** A report with build b's medians (and build a's, when given), its rounds when given, and nothing else the judge reads. */
function report(mode: "pr" | "release", b: Values, a: Values | null = null, rounds: Round[] = []) {
  return {
    mode,
    machine: "test",
    builds: { a: a && { label: "a", exe: "a.exe" }, b: { label: "b", exe: "b.exe" } },
    rounds,
    medians: { a, b },
  };
}

/** Rounds of `build` after the warm-up, one per value of `id`. */
const roundsOf = (id: string, build: "a" | "b", values: number[]): Round[] =>
  values.map((v) => ({ build, warmup: false, load: 10, values: { [id]: v } }));

function limits(measures: Record<string, Entry>) {
  return { machine: "test", commit: "abc", measures };
}

const memory: Entry = { unit: "MB", limit: 95, noise: 5, machine: true };

describe("MEASURES", () => {
  it("lists the ids in table order with their units, release-only ones flagged", () => {
    const ids = Object.keys(MEASURES);
    expect(ids.slice(0, 13)).toEqual([
      "size.unpacked",
      "size.zip",
      "start.paint.fresh",
      "start.tree.fresh",
      "start.paint.reused",
      "start.tree.reused",
      "idle.memory",
      "idle.cpu.shown",
      "idle.cpu.minimised",
      "run.memory.app",
      "run.memory.java",
      "run.cpu.app",
      "run.cpu.java",
    ]);
    expect(ids.indexOf("smooth.select.idle.paint")).toBe(ids.indexOf("run.finishedLag") + 1);
    expect(ids.slice(ids.indexOf("smooth.run.fps") - 1, ids.indexOf("page.nodes.open"))).toEqual([
      "smooth.scroll.run.frame",
      "smooth.run.fps",
      "smooth.run.frame",
    ]);
    expect(MEASURES["size.zip"]).toEqual({ unit: "MB" });
    expect(MEASURES["start.paint.fresh"]).toEqual({ unit: "ms", release: true });
    expect(MEASURES["idle.cpu.shown"]).toEqual({ unit: "ms" });
    expect(MEASURES["smooth.run.fps"]).toEqual({ unit: "fps" });
    expect(MEASURES["page.nodes.open"]).toEqual({ unit: "count" });
    expect(MEASURES["page.heap.run"]).toEqual({ unit: "MB" });
  });

  it("repeats every run, smooth and page id with @10k, for a release only", () => {
    const base = Object.keys(MEASURES).filter(
      (id) => /^(run|smooth|page)\./.test(id) && !id.endsWith("@10k"),
    );
    expect(base).toHaveLength(5 + 16 + 4);
    for (const id of base) {
      expect(MEASURES[`${id}@10k`]).toEqual({ ...MEASURES[id], release: true });
    }
    expect(Object.keys(MEASURES).filter((id) => id.endsWith("@10k"))).toHaveLength(base.length);
    expect(MEASURES["idle.memory@10k"]).toBeUndefined();
  });

  it("names the ids taken during Run all, untagged", () => {
    expect(RUN_PHASE).toEqual([
      "run.memory.app",
      "run.memory.java",
      "run.cpu.app",
      "run.cpu.java",
      "run.finishedLag",
      "smooth.select.run.paint",
      "smooth.expand.run.paint",
      "smooth.http.run.paint",
      "smooth.select.run.frame",
      "smooth.expand.run.frame",
      "smooth.http.run.frame",
      "smooth.scroll.run.frame",
      "smooth.run.fps",
      "smooth.run.frame",
      "page.nodes.run",
      "page.heap.run",
    ]);
  });
});

describe("judge", () => {
  it("misses a release measure over its limit", () => {
    const out = judge(report("release", { "idle.memory": 96 }), limits({ "idle.memory": memory }));
    expect(out.result).toBe("miss");
    expect(out.rows).toEqual([
      {
        id: "idle.memory",
        a: null,
        b: 96,
        change: null,
        limit: 95,
        aim: null,
        verdict: "over limit",
      },
    ]);
  });

  it("does not judge a machine measure against its limit in PR mode", () => {
    const out = judge(
      report("pr", { "idle.memory": 96 }, { "idle.memory": 95 }),
      limits({ "idle.memory": memory }),
    );
    expect(out.rows[0]).toMatchObject({ a: 95, b: 96, change: 1, limit: null, verdict: "pass" });
    expect(out.result).toBe("pass");
  });

  it("judges a measure that does not depend on the machine against its limit in PR mode", () => {
    const nodes: Entry = { unit: "count", limit: 95, noise: 5, machine: false };
    const out = judge(
      report("pr", { "page.nodes.open": 96 }, { "page.nodes.open": 95 }),
      limits({ "page.nodes.open": nodes }),
    );
    expect(out.rows[0]).toMatchObject({ limit: 95, verdict: "over limit" });
    expect(out.result).toBe("miss");
  });

  it("calls a move beyond the noise worse or better, and one within it a pass", () => {
    const l = limits({ "idle.memory": { ...memory, limit: 200 } });
    const verdict = (b: number) =>
      judge(report("release", { "idle.memory": b }, { "idle.memory": 80 }), l);
    expect(verdict(90).rows[0]).toMatchObject({ change: 10, verdict: "worse" });
    expect(verdict(90).result).toBe("miss");
    expect(verdict(85).rows[0].verdict).toBe("pass");
    expect(verdict(84).rows[0].verdict).toBe("pass");
    expect(verdict(70).rows[0]).toMatchObject({ change: -10, verdict: "better" });
    expect(verdict(70).result).toBe("pass");
  });

  it("reads a higher-is-better measure the other way round", () => {
    const fps: Entry = { unit: "fps", limit: 60, noise: 2, better: "higher", machine: true };
    const l = limits({ "smooth.run.fps": fps });
    const run = (b: number, a = 70) =>
      judge(report("release", { "smooth.run.fps": b }, { "smooth.run.fps": a }), l);
    expect(run(55).rows[0].verdict).toBe("over limit");
    expect(run(65).rows[0].verdict).toBe("worse");
    expect(run(69).rows[0].verdict).toBe("pass");
    expect(run(75).rows[0].verdict).toBe("better");
  });

  it("judges on the limit alone when build a has no value", () => {
    const l = limits({ "idle.memory": { ...memory, limit: 95 } });
    const none = judge(report("release", { "idle.memory": 90 }, { "idle.memory": null }), l);
    expect(none.rows[0]).toEqual({
      id: "idle.memory",
      a: null,
      b: 90,
      change: null,
      limit: 95,
      aim: null,
      verdict: "n/a",
    });
    expect(none.result).toBe("pass");
    const single = judge(report("release", { "idle.memory": 96 }), l);
    expect(single.rows[0].verdict).toBe("over limit");
  });

  it("misses a limited measure that was not measured in release mode", () => {
    const l = limits({ "idle.memory": memory });
    const release = judge(report("release", { "idle.memory": null }, { "idle.memory": 90 }), l);
    expect(release.rows[0]).toMatchObject({
      a: 90,
      b: null,
      change: null,
      limit: 95,
      verdict: "not measured",
    });
    expect(release.result).toBe("miss");
    const missing = judge(report("release", {}), l);
    expect(missing.rows[0]).toMatchObject({ id: "idle.memory", b: null, verdict: "not measured" });
    expect(missing.result).toBe("miss");
  });

  it("calls a measure build a has and build b lacks not measured in PR mode, limit or not", () => {
    const out = judge(
      report("pr", { "idle.memory": null }, { "idle.memory": 90, "run.cpu.app": 5 }),
      limits({ "idle.memory": memory }),
    );
    expect(out.rows.map((r) => [r.id, r.verdict])).toEqual([
      ["idle.memory", "not measured"],
      ["run.cpu.app", "not measured"],
    ]);
    expect(out.result).toBe("miss");
  });

  it("leaves out of a PR table a measure neither build has, but keeps it in release mode", () => {
    const l = limits({ "idle.memory": memory, "start.paint.fresh": { ...memory, unit: "ms" } });
    const medians = { "idle.memory": 90, "size.zip": null };
    const pr = judge(report("pr", medians, { ...medians }), l);
    expect(pr.rows.map((r) => r.id)).toEqual(["idle.memory"]);
    expect(pr.result).toBe("pass");
    const release = judge(report("release", medians, { ...medians }), l);
    expect(release.rows.map((r) => [r.id, r.verdict])).toEqual([
      ["size.zip", "n/a"],
      ["start.paint.fresh", "not measured"],
      ["idle.memory", "pass"],
    ]);
  });

  it("keeps a measure only build b has in a PR table, as n/a", () => {
    const out = judge(report("pr", { "idle.memory": 90 }, { "idle.memory": null }), limits({}));
    expect(out.rows).toEqual([
      { id: "idle.memory", a: null, b: 90, change: null, limit: null, aim: null, verdict: "n/a" },
    ]);
  });

  it.each(["pr", "release"] as const)(
    "widens the noise to the spread of either build's rounds, in %s mode",
    (mode) => {
      const id = "smooth.select.idle.paint";
      const l = limits({ [id]: { unit: "ms", limit: 999, noise: 5, aim: 100, machine: true } });
      const a = roundsOf(id, "a", [10, 12, 16]); // median 12, spread 6
      const judged = (b: number[]) => {
        const r = report(mode, { [id]: b[1] }, { [id]: 12 }, [...a, ...roundsOf(id, "b", b)]);
        return judge(r, l).rows[0];
      };
      // 18 against 12 is 6 ms: beyond the limit's noise of 5, within a's spread of 6
      expect(judged([12, 18, 18])).toMatchObject({ change: 6, verdict: "pass" });
      // b's own spread counts too: 7 ms is within b's 9
      expect(judged([10, 19, 19])).toMatchObject({ change: 7, verdict: "pass" });
      // beyond both spreads
      expect(judged([19, 20, 22])).toMatchObject({ change: 8, verdict: "worse" });
      expect(judged([3, 4, 5])).toMatchObject({ change: -8, verdict: "better" });
    },
  );

  it("leaves warm-up rounds and missing values out of the spread", () => {
    const id = "idle.memory";
    const rounds: Round[] = [
      { build: "a", warmup: true, load: 10, values: { [id]: 300 } },
      { build: "b", warmup: false, load: 10, values: { [id]: null } },
      ...roundsOf(id, "a", [80, 81]),
      ...roundsOf(id, "b", [90]),
    ];
    const out = judge(
      report("release", { [id]: 90 }, { [id]: 80 }, rounds),
      limits({ [id]: { ...memory, limit: 200 } }),
    );
    expect(out.rows[0].verdict).toBe("worse");
  });

  it("compares a measure with no limit using a 5% noise", () => {
    const run = (b: number) =>
      judge(report("release", { "idle.memory": b }, { "idle.memory": 100 }), limits({}));
    expect(run(104).rows[0]).toEqual({
      id: "idle.memory",
      a: 100,
      b: 104,
      change: 4,
      limit: null,
      aim: null,
      verdict: "pass",
    });
    expect(run(106).rows[0].verdict).toBe("worse");
    expect(run(94).rows[0].verdict).toBe("better");
    expect(run(106).result).toBe("miss");
  });

  it("lists rows in table order and carries the aim", () => {
    const aimed: Entry = { unit: "ms", limit: 150, noise: 5, aim: 100, machine: true };
    const out = judge(
      report("release", { "smooth.select.idle.paint": 90, "idle.memory": 90, "custom.thing": 1 }),
      limits({ "smooth.select.idle.paint": aimed, "idle.memory": memory }),
    );
    expect(out.rows.map((r) => r.id)).toEqual([
      "idle.memory",
      "smooth.select.idle.paint",
      "custom.thing",
    ]);
    expect(out.rows[1].aim).toBe(100);
  });
});

describe("makeLimits", () => {
  const rounds = (id: string, build: "a" | "b", values: number[], warmup = false) =>
    values.map((v) => ({ build, warmup, load: 10, values: { [id]: v } as Values }));
  const meta = { machine: "ref", commit: "e3cfe3b7" };

  it("sets the limit at the median plus a noise from the spread", () => {
    const r = { ...report("release", {}), rounds: rounds("idle.memory", "b", [100, 104, 98]) };
    const out = makeLimits(r, meta);
    expect(out.machine).toBe("ref");
    expect(out.commit).toBe("e3cfe3b7");
    expect(out.measures["idle.memory"]).toEqual({
      unit: "MB",
      limit: 106,
      noise: 6,
      machine: true,
    });
  });

  it("gives a size its spread as noise, at least 1 MB, with no 5% of the median", () => {
    const r = {
      ...report("release", {}),
      rounds: [
        ...rounds("size.unpacked", "b", [538.7, 538.7, 538.7]),
        ...rounds("size.zip", "b", [218.7, 220.2, 218.7]),
      ],
    };
    const { measures } = makeLimits(r, meta);
    expect(measures["size.unpacked"]).toEqual({
      unit: "MB",
      limit: 539.7,
      noise: 1,
      machine: false,
    });
    expect(measures["size.zip"]).toMatchObject({ noise: 2, limit: 220.7 });
  });

  it("takes 5% of the median, or the floor for the unit, when the spread is smaller", () => {
    const r = {
      ...report("release", {}),
      rounds: [
        ...rounds("idle.cpu.shown", "b", [1000, 1000, 1000]),
        ...rounds("start.tree.reused", "b", [10, 10, 10]),
        ...rounds("page.nodes.open", "b", [3, 3, 3]),
      ],
    };
    const { measures } = makeLimits(r, meta);
    expect(measures["idle.cpu.shown"]).toMatchObject({ noise: 50, limit: 1050 });
    expect(measures["start.tree.reused"]).toMatchObject({ noise: 5, limit: 15 });
    expect(measures["page.nodes.open"]).toMatchObject({ noise: 1, limit: 4 });
  });

  it("gives the Smooth measures their aims", () => {
    const r = {
      ...report("release", {}),
      rounds: [
        ...rounds("smooth.select.idle.paint", "b", [80, 80, 80]),
        ...rounds("smooth.scroll.run.frame", "b", [90, 90, 90]),
        ...rounds("smooth.run.frame@10k", "b", [300, 300, 300]),
        ...rounds("idle.memory", "b", [90, 90, 90]),
      ],
    };
    const { measures } = makeLimits(r, meta);
    expect(measures["smooth.select.idle.paint"].aim).toBe(100);
    expect(measures["smooth.scroll.run.frame"].aim).toBe(50);
    expect(measures["smooth.run.frame@10k"].aim).toBe(50);
    expect(measures["idle.memory"]).not.toHaveProperty("aim");
  });

  it("marks sizes and page measures as independent of the machine", () => {
    const ids = [
      "size.unpacked",
      "page.heap.open",
      "page.nodes.run@10k",
      "run.cpu.java",
      "start.paint.reused",
    ];
    const r = {
      ...report("release", {}),
      rounds: ids.flatMap((id) => rounds(id, "b", [100, 100, 100])),
    };
    const { measures } = makeLimits(r, meta);
    expect(ids.map((id) => measures[id].machine)).toEqual([false, false, false, true, true]);
  });

  it("sets a higher-is-better limit at the median minus the noise", () => {
    const r = { ...report("release", {}), rounds: rounds("smooth.run.fps", "b", [60, 58, 59]) };
    expect(makeLimits(r, meta).measures["smooth.run.fps"]).toEqual({
      unit: "fps",
      limit: 56,
      noise: 3,
      better: "higher",
      machine: true,
    });
  });

  it("rounds a limit to the unit: one decimal for MB, whole numbers for the rest", () => {
    const r = {
      ...report("release", {}),
      rounds: [
        ...rounds("idle.memory", "b", [96.43, 96.43, 96.43]),
        ...rounds("start.tree.reused", "b", [612.4, 612.4, 612.4]),
        ...rounds("smooth.run.fps", "b", [59.6, 59.6, 59.6]),
        ...rounds("page.nodes.open", "b", [100.5, 100.5, 100.5]),
      ],
    };
    const { measures } = makeLimits(r, meta);
    expect(measures["idle.memory"]).toMatchObject({ noise: 5, limit: 101.4 });
    expect(measures["start.tree.reused"]).toMatchObject({ noise: 31, limit: 643 });
    expect(measures["smooth.run.fps"]).toMatchObject({ noise: 3, limit: 57 });
    expect(measures["page.nodes.open"]).toMatchObject({ noise: 6, limit: 107 });
  });

  it("ignores warm-up rounds, build a and values it did not get", () => {
    const r = {
      ...report("release", {}),
      rounds: [
        ...rounds("idle.memory", "b", [500], true),
        ...rounds("idle.memory", "a", [900, 900, 900]),
        ...rounds("idle.memory", "b", [100, 104, 98]),
        {
          build: "b" as const,
          warmup: false,
          load: 10,
          values: { "idle.memory": null, "size.zip": null },
        },
      ],
    };
    const { measures } = makeLimits(r, meta);
    expect(measures["idle.memory"]).toMatchObject({ limit: 106, noise: 6 });
    expect(measures["size.zip"]).toBeUndefined();
  });
});

describe("raisedLimits", () => {
  const lim = (n: number, better?: "higher") =>
    limits({
      "idle.memory": { unit: "MB", limit: n, noise: 5, machine: true, ...(better && { better }) },
    });

  it("lists a lower-is-better limit that went up, not one that went down", () => {
    expect(raisedLimits(lim(95), lim(96))).toEqual(["idle.memory"]);
    expect(raisedLimits(lim(95), lim(95))).toEqual([]);
    expect(raisedLimits(lim(95), lim(90))).toEqual([]);
  });

  it("lists a higher-is-better limit that went down", () => {
    expect(raisedLimits(lim(60, "higher"), lim(55, "higher"))).toEqual(["idle.memory"]);
    expect(raisedLimits(lim(60, "higher"), lim(65, "higher"))).toEqual([]);
  });

  it("lists a wider noise, not a narrower one", () => {
    const noisy = (noise: number) =>
      limits({ "idle.memory": { unit: "MB", limit: 95, noise, machine: true } });
    expect(raisedLimits(noisy(5), noisy(6))).toEqual(["idle.memory"]);
    expect(raisedLimits(noisy(5), noisy(4))).toEqual([]);
  });

  it("lists a flipped direction, whatever the limit", () => {
    expect(raisedLimits(lim(60, "higher"), lim(60))).toEqual(["idle.memory"]);
    expect(raisedLimits(lim(95), lim(10, "higher"))).toEqual(["idle.memory"]);
  });

  it("lists an id removed from head, or every id when head is gone", () => {
    expect(raisedLimits(lim(95), limits({}))).toEqual(["idle.memory"]);
    expect(raisedLimits(lim(95), null)).toEqual(["idle.memory"]);
  });

  it("lists nothing without a base, or for an id only head has", () => {
    expect(raisedLimits(null, lim(95))).toEqual([]);
    expect(raisedLimits(limits({}), lim(95))).toEqual([]);
  });
});
