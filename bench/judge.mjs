// Judges a bench report against the limits file, bench/limits.json (docs/specs/2026-10-31-light-and-fast.md, R3).
import { MEASURES } from "./measures.mjs";

/** @typedef {Record<string, number | null>} Values */

/**
 * @typedef {object} Report What the bench writes: every round, and the medians the table and the judge read.
 * @property {"pr" | "release"} mode
 * @property {string} machine
 * @property {{ a: { label: string, exe: string } | null, b: { label: string, exe: string } }} builds `b` is the build under test, `a` the one it is compared with.
 * @property {{ build: "a" | "b", warmup: boolean, load: number, values: Values }[]} rounds
 * @property {{ a: Values | null, b: Values }} medians
 */

/**
 * @typedef {object} Limit
 * @property {string} unit
 * @property {number} limit The ceiling, or the floor when `better` is `"higher"`.
 * @property {number} noise The move that still counts as no change.
 * @property {number} [aim]
 * @property {"higher"} [better] Absent when lower is better.
 * @property {boolean} machine True when the number depends on the machine, so only the reference machine judges it.
 */

/** @typedef {{ machine: string, commit: string, measures: Record<string, Limit> }} Limits */

/**
 * @typedef {object} Row
 * @property {string} id
 * @property {number | null} a
 * @property {number | null} b
 * @property {number | null} change `b - a`, or null when either is missing.
 * @property {number | null} limit Null when the measure has none, or the machine may not judge it.
 * @property {number | null} aim
 * @property {"pass" | "better" | "worse" | "over limit" | "not measured" | "n/a"} verdict
 */

const AIMS = { paint: 100, frame: 50 };
const MISSES = ["over limit", "worse", "not measured"];

/** Orders `ids` as the catalogue does; ids it does not know come last. */
const inTableOrder = (ids) =>
  [...new Set([...Object.keys(MEASURES), ...ids])].filter((id) => ids.has(id));

/**
 * Rows for every measure the report or the limits name, and `"miss"` when one is over its limit, worse than
 * build a beyond its noise, or has a limit but was not measured (release mode only: a PR measures a subset).
 * @param {Report} report
 * @param {Limits} limits
 * @returns {{ result: "pass" | "miss", rows: Row[] }}
 */
export function judge(report, limits) {
  const { a: medianA, b: medianB } = report.medians;
  const ids = inTableOrder(
    new Set([
      ...Object.keys(limits.measures),
      ...Object.keys(medianA ?? {}),
      ...Object.keys(medianB),
    ]),
  );
  const release = report.mode === "release";
  /** @type {Row[]} */
  const rows = ids.map((id) => {
    const entry = limits.measures[id];
    const higher = entry ? entry.better === "higher" : MEASURES[id]?.unit === "fps";
    const limit = entry && (release || !entry.machine) ? entry.limit : null;
    const a = medianA?.[id] ?? null;
    const b = medianB[id] ?? null;
    const change = a === null || b === null ? null : b - a;
    /** @type {Row["verdict"]} */
    let verdict;
    if (b === null) verdict = entry && release ? "not measured" : "n/a";
    else if (limit !== null && (higher ? b < limit : b > limit)) verdict = "over limit";
    else if (a === null) verdict = "n/a";
    else {
      const noise = entry ? entry.noise : Math.abs(a) / 20;
      const worse = higher ? a - b : b - a;
      verdict = worse > noise ? "worse" : worse < -noise ? "better" : "pass";
    }
    return { id, a, b, change, limit, aim: entry?.aim ?? null, verdict };
  });
  return { result: rows.some((r) => MISSES.includes(r.verdict)) ? "miss" : "pass", rows };
}

/** @param {number[]} values */
function median(values) {
  const sorted = [...values].sort((x, y) => x - y);
  const mid = sorted.length >> 1;
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * Limits from build b's rounds after the warm-up: each limit is the median plus a noise margin, the spread of
 * its rounds, at least 5% of the median and at least a floor (5 ms, else 1), in whole units.
 * @param {Report} report
 * @param {{ machine: string, commit: string }} meta
 * @returns {Limits}
 */
export function makeLimits(report, meta) {
  const rounds = report.rounds.filter((r) => r.build === "b" && !r.warmup);
  /** @type {Record<string, Limit>} */
  const measures = {};
  for (const [id, { unit }] of Object.entries(MEASURES)) {
    const values = rounds.map((r) => r.values[id]).filter((v) => typeof v === "number");
    if (!values.length) continue;
    const mid = median(values);
    const spread = Math.max(...values) - Math.min(...values);
    const noise = Math.ceil(Math.max(spread, mid / 20, unit === "ms" ? 5 : 1));
    const higher = unit === "fps";
    const aim = id.startsWith("smooth.")
      ? AIMS[id.replace("@10k", "").split(".").at(-1)]
      : undefined;
    measures[id] = {
      unit,
      limit: higher ? mid - noise : mid + noise,
      noise,
      ...(aim && { aim }),
      ...(higher && { better: "higher" }),
      machine: !/^(size|page)\./.test(id),
    };
  }
  return { machine: meta.machine, commit: meta.commit, measures };
}

/**
 * The ids whose limit is looser in `head` than in `base`, or missing from `head`. Raising one needs a decision record.
 * @param {Limits | null} base
 * @param {Limits | null} head
 * @returns {string[]}
 */
export function raisedLimits(base, head) {
  if (!base) return [];
  return Object.entries(base.measures)
    .filter(([id, { limit, better }]) => {
      const now = head?.measures[id]?.limit;
      return now === undefined || (better === "higher" ? now < limit : now > limit);
    })
    .map(([id]) => id);
}
