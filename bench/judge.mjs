// Judges a bench report against the limits file, bench/limits.json (docs/specs/2026-10-31-light-and-fast.md, R3).
import { MEASURES } from "./measures.mjs";

/** @typedef {Record<string, number | null>} Values */

/**
 * @typedef {object} Report What the bench writes: every round, and the medians the table and the judge read.
 * @property {"pr" | "release"} mode
 * @property {string} machine
 * @property {string} screen Where the bench placed the app's window.
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

/** The spread (highest less lowest) of `build`'s rounds after the warm-up for `id`, 0 when it has none. */
function range(
  /** @type {Report} */ report,
  /** @type {"a" | "b"} */ build,
  /** @type {string} */ id,
) {
  const values = report.rounds
    .filter((r) => r.build === build && !r.warmup)
    .map((r) => r.values[id])
    .filter((v) => typeof v === "number");
  return values.length ? Math.max(...values) - Math.min(...values) : 0;
}

/**
 * Rows for every measure the report or the limits name, and `"miss"` when one is over its limit, worse than
 * build a beyond its noise, or was not measured: in release mode when it has a limit, in PR mode when build a
 * measured it. The noise is the larger of the limit's (5% of build a's value when there is no limit) and the
 * spread of either build's rounds, so a move within that day's spread is no change. A PR measures a subset, so
 * its table leaves out a measure neither build has.
 * @param {Report} report
 * @param {Limits} limits
 * @returns {{ result: "pass" | "miss", rows: Row[] }}
 */
export function judge(report, limits) {
  const { a: medianA, b: medianB } = report.medians;
  const release = report.mode === "release";
  const ids = inTableOrder(
    new Set([
      ...Object.keys(limits.measures),
      ...Object.keys(medianA ?? {}),
      ...Object.keys(medianB),
    ]),
  ).filter((id) => release || (medianA?.[id] ?? null) !== null || (medianB[id] ?? null) !== null);
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
    // in PR mode, a has it here: the PR broke a probe that its base can take
    if (b === null) verdict = entry || !release ? "not measured" : "n/a";
    else if (limit !== null && (higher ? b < limit : b > limit)) verdict = "over limit";
    else if (a === null) verdict = "n/a";
    else {
      const noise = Math.max(
        entry ? entry.noise : Math.abs(a) / 20,
        range(report, "a", id),
        range(report, "b", id),
      );
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
 * its rounds, at least 5% of the median and at least a floor (5 ms, else 1), in whole units. A size never
 * varies, so its margin is its spread and at least 1 MB, with no 5%. The limit is rounded to the unit: one
 * decimal for MB, a whole number for the rest.
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
    const noise = Math.ceil(
      id.startsWith("size.")
        ? Math.max(spread, 1)
        : Math.max(spread, mid / 20, unit === "ms" ? 5 : 1),
    );
    const higher = unit === "fps";
    const scale = unit === "MB" ? 10 : 1;
    const aim = id.startsWith("smooth.")
      ? AIMS[id.replace("@10k", "").split(".").at(-1)]
      : undefined;
    measures[id] = {
      unit,
      limit: Math.round((higher ? mid - noise : mid + noise) * scale) / scale,
      noise,
      ...(aim && { aim }),
      ...(higher && { better: "higher" }),
      machine: !/^(size|page)\./.test(id),
    };
  }
  return { machine: meta.machine, commit: meta.commit, measures };
}

/**
 * The ids whose limit is looser in `head` than in `base`: a higher limit (lower where higher is better), a wider
 * noise (a worse move counts as no change), a flipped direction, or no entry at all. Raising one needs a
 * decision record.
 * @param {Limits | null} base
 * @param {Limits | null} head
 * @returns {string[]}
 */
export function raisedLimits(base, head) {
  if (!base) return [];
  return Object.entries(base.measures)
    .filter(([id, { limit, noise, better }]) => {
      const now = head?.measures[id];
      if (!now || now.noise > noise || now.better !== better) return true;
      return better === "higher" ? now.limit < limit : now.limit > limit;
    })
    .map(([id]) => id);
}
