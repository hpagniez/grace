// The bench's Markdown table: the PR comment, the job summary and a release's `## Performance` section
// (docs/specs/2026-10-31-light-and-fast.md, R2.6).
import { MEASURES } from "./measures.mjs";

/** @typedef {import("./judge.mjs").Report} Report */
/** @typedef {import("./judge.mjs").judge} Judge */

const MISSES = ["over limit", "worse", "not measured"];

/** A number to one decimal, or `n/a`. */
const fmt = (/** @type {number | null} */ n) =>
  n === null ? "n/a" : String(Math.round(n * 10) / 10);

/**
 * The table of `verdict`'s rows, under a header naming both builds and the machine. A release-mode table ends with
 * the `Result:` line that `scripts/check-pr.mjs` reads; a PR-mode one has none, because a PR measures only a subset
 * and nothing there fails on a number.
 * @param {Report} report
 * @param {ReturnType<Judge>} verdict
 * @returns {string}
 */
export function markdown(report, verdict) {
  const { a, b } = report.builds;
  const loads = report.rounds.map((r) => r.load);
  const rounds = report.rounds.filter((r) => r.build === "b" && !r.warmup).length;
  const lines = [
    `## Bench: ${b.label}${a ? ` against ${a.label}` : ""}`,
    "",
    `Machine: ${report.machine}`,
    `Mode: ${report.mode}, median of ${rounds} round${rounds === 1 ? "" : "s"} after a warm-up`,
    `Machine load before the rounds: ${fmt(Math.min(...loads))}% to ${fmt(Math.max(...loads))}%`,
    "",
    `| Measure | A (${a?.label ?? "none"}) | B (${b.label}) | Change | Limit | Result |`,
    "|---|---|---|---|---|---|",
  ];
  for (const row of verdict.rows) {
    const unit = MEASURES[row.id]?.unit;
    const note = [unit, row.aim === null ? null : `aim ${row.aim}`].filter(Boolean).join(", ");
    const change = row.change === null ? "n/a" : (row.change > 0 ? "+" : "") + fmt(row.change);
    const result = MISSES.includes(row.verdict) ? `**${row.verdict}**` : row.verdict;
    const limit = row.limit === null ? "–" : fmt(row.limit);
    lines.push(
      `| \`${row.id}\`${note && ` (${note})`} | ${fmt(row.a)} | ${fmt(row.b)} | ${change} | ${limit} | ${result} |`,
    );
  }
  if (report.mode === "release") lines.push("", `Result: ${verdict.result}`);
  return lines.join("\n") + "\n";
}
