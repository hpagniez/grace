import { describe, expect, it } from "vitest";
import { judge } from "./judge.mjs";
import { MEASURES } from "./measures.mjs";
import { markdown } from "./report.mjs";

type Values = Record<string, number | null>;

/** Every measure at `value`. */
const all = (value: number | null): Values =>
  Object.fromEntries(Object.keys(MEASURES).map((id) => [id, value]));

function report(mode: "pr" | "release", b: Values, a: Values | null = null) {
  return {
    mode,
    machine: "Windows 11 Pro, Ryzen 7 5800X, 32 GB",
    screen: "DISPLAY1, not the main screen",
    builds: {
      a: a && { label: "1.0.0-rc.6", exe: "a.exe" },
      b: { label: "this branch", exe: "b.exe" },
    },
    rounds: [
      { build: "b" as const, warmup: true, load: 30, values: {} },
      { build: "b" as const, warmup: false, load: 12.4, values: {} },
      { build: "b" as const, warmup: false, load: 20, values: {} },
    ],
    medians: { a, b },
  };
}

const limits = (measures = {}) => ({ machine: "ref", commit: "abc", measures });
const memory = { unit: "MB", limit: 95, noise: 5, machine: true };

/** The cells of the table row for `id`: measure, A, B, change, limit, result. */
function row(md: string, id: string) {
  const line = md.split("\n").find((l) => l.startsWith(`| \`${id}\``));
  if (!line) throw new Error(`no row for ${id}`);
  return line
    .split("|")
    .slice(1, -1)
    .map((cell) => cell.trim());
}

describe("markdown", () => {
  it("names both builds, the machine and the screen under a level-3 heading", () => {
    const r = report("pr", all(1), all(1));
    const md = markdown(r, judge(r, limits()));
    const header = md.slice(0, md.indexOf("| Measure"));
    expect(md.split("\n")[0]).toBe("### Bench: this branch against 1.0.0-rc.6");
    expect(header).toContain("Windows 11 Pro, Ryzen 7 5800X, 32 GB");
    expect(header).toContain("Screen: DISPLAY1, not the main screen");
  });

  it("has one row per measure, in the catalogue's order", () => {
    const r = report("release", all(1), all(1));
    const md = markdown(r, judge(r, limits()));
    const ids = [...md.matchAll(/^\| `([^`]+)`/gm)].map((m) => m[1]);
    expect(ids).toEqual(Object.keys(MEASURES));
  });

  it("leaves out of a PR table the measures neither build took", () => {
    const pr: Values = { ...all(1), "size.zip": null, "start.paint.fresh": null };
    const r = report("pr", pr, { ...pr });
    const md = markdown(r, judge(r, limits({ "start.paint.fresh": memory })));
    const ids = [...md.matchAll(/^\| `([^`]+)`/gm)].map((m) => m[1]);
    expect(ids).toEqual(Object.keys(MEASURES).filter((id) => pr[id] !== null));
  });

  it("shows in bold a PR measure the base took and this build did not", () => {
    const r = report("pr", { ...all(1), "run.cpu.app": null }, all(1));
    const [, a, b, change, , result] = row(markdown(r, judge(r, limits())), "run.cpu.app");
    expect([a, b, change, result]).toEqual(["1", "n/a", "n/a", "**not measured**"]);
  });

  it("writes n/a for a measure with no number", () => {
    const r = report("pr", { ...all(1), "idle.memory": 90 }, { ...all(1), "idle.memory": null });
    const [, a, b, change, , result] = row(markdown(r, judge(r, limits())), "idle.memory");
    expect([a, b, change, result]).toEqual(["n/a", "90", "n/a", "n/a"]);
  });

  it("shows the change from A to B with its sign", () => {
    const r = report("pr", { ...all(1), "idle.memory": 100 }, { ...all(1), "idle.memory": 96.5 });
    expect(row(markdown(r, judge(r, limits())), "idle.memory").slice(1, 4)).toEqual([
      "96.5",
      "100",
      "+3.5",
    ]);
  });

  it("shows – in the limit column when there is no limit, else the limit", () => {
    const r = report("release", { ...all(1), "idle.memory": 90 });
    const md = markdown(r, judge(r, limits({ "idle.memory": memory })));
    expect(row(md, "idle.memory")[4]).toBe("95");
    expect(row(md, "size.zip")[4]).toBe("–");
  });

  it("ends with Result: miss in release mode when a measure misses", () => {
    const r = report("release", { ...all(1), "idle.memory": 120 });
    const verdict = judge(r, limits({ "idle.memory": memory }));
    expect(verdict.result).toBe("miss");
    expect(markdown(r, verdict).trimEnd().split("\n").at(-1)).toBe("Result: miss");
  });

  it("ends with Result: pass in release mode when nothing misses", () => {
    const r = report("release", all(1));
    expect(markdown(r, judge(r, limits())).trimEnd().split("\n").at(-1)).toBe("Result: pass");
  });

  it("has no Result: line in PR mode, miss or not", () => {
    const r = report("pr", { ...all(1), "idle.memory": 120 });
    const md = markdown(r, judge(r, limits({ "idle.memory": memory })));
    expect(md).not.toMatch(/^Result:/m);
  });
});
