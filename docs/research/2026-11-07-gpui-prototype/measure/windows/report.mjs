// The numbers of a Windows run, summed up and written as a Markdown table. No Windows, no Grace checkout needed, so
// it can be tried anywhere: `node report.mjs <windows-label.json>` prints the table of a saved run.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** @typedef {Record<string, number | null>} Values */
/**
 * @typedef {{
 *   label: string, date: string, machine: any, settings: any, rounds: { app: "grace" | "gpui", warmup: boolean, load?: number, values: Values }[],
 *   floors: { app: "electron" | "gpui", memMB: number, cpuMs: number, procs: number }[],
 *   sizes: { grace: Values, gpui: Values }, errors: string[],
 * }} Report
 */

/** [id, what it is, unit]. The ids of bench/measures.mjs where the bench has one; `@10k` is the 10,000-test project. */
export const MEASURES = [
  ["start.paint.reused", "First paint, profile reused", "ms"],
  ["start.tree.reused", "Tree shown (2,000 tests), profile reused", "ms"],
  ["start.paint.fresh", "First paint, fresh profile", "ms"],
  ["start.tree.fresh", "Tree shown (2,000 tests), fresh profile", "ms"],
  ["idle.memory", "Idle memory: private working set, whole tree", "MB"],
  ["idle.procs", "Idle processes", ""],
  ["idle.cpu.shown", "Idle CPU, window shown", "ms"],
  ["idle.cpu.minimised", "Idle CPU, window minimised", "ms"],
  ...["", "@10k"].flatMap((tag) => [
    [`run.memory.app${tag}`, `Run all${tag && " (10,000 tests)"}: peak memory, app`, "MB"],
    [`run.memory.java${tag}`, `Run all${tag && " (10,000 tests)"}: peak memory, Java`, "MB"],
    [`run.cpu.app${tag}`, `Run all${tag && " (10,000 tests)"}: CPU, app`, "ms"],
    [`run.cpu.java${tag}`, `Run all${tag && " (10,000 tests)"}: CPU, Java`, "ms"],
    [`run.wall${tag}`, `Run all${tag && " (10,000 tests)"}: wall time`, "ms"],
    [`run.memory.after${tag}`, `Run all${tag && " (10,000 tests)"}: memory 3 s after the end, app`, "MB"],
  ]),
  ["scroll.idle.frames", "Wheel over the tree, idle: frames drawn", ""],
  ["scroll.idle.maxFrame", "Wheel over the tree, idle: longest frame", "ms"],
  ["scroll.idle.p95Frame", "Wheel over the tree, idle: 95th percentile frame", "ms"],
  ["scroll.run.frames", "Wheel over the tree during Run all: frames drawn", ""],
  ["scroll.run.maxFrame", "Wheel over the tree during Run all: longest frame", "ms"],
  ["scroll.run.p95Frame", "Wheel over the tree during Run all: 95th percentile frame", "ms"],
  ["size.exe", "Size: the app's exe", "MB"],
  ["size.unpacked", "Size: unpacked folder, Java included", "MB"],
  ["size.withoutJava", "Size: unpacked, without Java", "MB"],
  ["size.zip", "Size: zip (GPUI: deflate of the exe, an estimate)", "MB"],
];

/** @param {number[]} xs */
export function median(xs) {
  if (!xs.length) return null;
  const s = [...xs].sort((a, b) => a - b);
  const mid = s.length >> 1;
  return s.length % 2 ? s[mid] : (s[mid - 1] + s[mid]) / 2;
}

/**
 * Median, range and count of each measure of one app over its rounds after the warm-up; the sizes, which come once,
 * count as one round.
 * @param {Report} report
 * @param {"grace" | "gpui"} app
 * @returns {Record<string, { median: number, min: number, max: number, n: number }>}
 */
export function stats(report, app) {
  /** @type {Record<string, number[]>} */
  const by = {};
  for (const r of report.rounds)
    if (r.app === app && !r.warmup)
      for (const [id, v] of Object.entries(r.values)) if (typeof v === "number") (by[id] ??= []).push(v);
  for (const [id, v] of Object.entries(report.sizes?.[app] ?? {})) if (typeof v === "number") by[id] = [v];
  return Object.fromEntries(
    Object.entries(by).map(([id, xs]) => [id, { median: /** @type {number} */ (median(xs)), min: Math.min(...xs), max: Math.max(...xs), n: xs.length }]),
  );
}

const fmt = (/** @type {number} */ n) => n.toLocaleString("en-US", { maximumFractionDigits: 1 });
const cell = (/** @type {{ median: number, min: number, max: number, n: number } | undefined} */ s) =>
  !s ? "n/a" : s.n > 1 ? `${fmt(s.median)} [${fmt(s.min)}–${fmt(s.max)}]` : fmt(s.median);

/**
 * @param {Report} report
 * @param {{ measures?: Record<string, { limit: number }> } | null} [limits] bench/limits.json: the limit of the bench's measures is shown beside Grace's
 */
export function markdown(report, limits = null) {
  const g = stats(report, "grace");
  const p = stats(report, "gpui");
  const m = report.machine ?? {};
  const rounds = (/** @type {string} */ app) => report.rounds.filter((r) => r.app === app && !r.warmup).length;
  const out = [
    `### Windows: GPUI prototype against Grace, ${report.label}`,
    "",
    `Date ${report.date}. ${m.os ?? "?"}, ${m.cpu ?? "?"}, ${m.ramGB ?? "?"} GB. Rounds after the warm-up: Grace ${rounds("grace")}, GPUI ${rounds("gpui")}; ` +
      `settle ${report.settings?.settleS} s, idle ${report.settings?.idleS} s. Medians, the range of the rounds in brackets.`,
    "",
    `Grace commit ${m.graceCommit ?? "?"}. Display adapters: ${(m.displays?.adapters ?? []).map((a) => `${a.name} (${a.hz} Hz, driver ${a.driver})`).join("; ") || "unknown"}. ` +
      `GPUI window: ${report.settings?.gpuiWindow ?? "?"}.`,
    "",
    "| Measure | Unit | Grace (Electron) | GPUI prototype | GPUI / Grace | Grace's limit |",
    "|---|---|---|---|---|---|",
  ];
  for (const [id, what, unit] of MEASURES) {
    if (!g[id] && !p[id]) continue;
    const ratio = g[id] && p[id] && g[id].median > 0 ? `${(p[id].median / g[id].median).toFixed(2)}x` : "n/a";
    const limit = limits?.measures?.[id]?.limit;
    out.push(`| ${what} | ${unit} | ${cell(g[id])} | ${cell(p[id])} | ${ratio} | ${limit === undefined ? "" : fmt(limit)} |`);
  }
  if (report.floors?.length) {
    const floor = (/** @type {string} */ app, /** @type {"memMB" | "cpuMs" | "procs"} */ k) => {
      const xs = report.floors.filter((f) => f.app === app).map((f) => f[k]);
      if (!xs.length) return "n/a";
      const mid = fmt(/** @type {number} */ (median(xs)));
      return xs.length > 1 ? `${mid} [${fmt(Math.min(...xs))}–${fmt(Math.max(...xs))}]` : mid;
    };
    out.push(
      "",
      "Each stack's empty 1400 x 900 window, idle:",
      "",
      "| Empty window | Electron | GPUI |",
      "|---|---|---|",
      `| Idle memory (private working set), MB | ${floor("electron", "memMB")} | ${floor("gpui", "memMB")} |`,
      `| Idle CPU, ms | ${floor("electron", "cpuMs")} | ${floor("gpui", "cpuMs")} |`,
      `| Processes | ${floor("electron", "procs")} | ${floor("gpui", "procs")} |`,
    );
  }
  if (report.errors?.length) out.push("", "Phases that failed (their measures are missing):", "", ...report.errors.map((e) => `- ${e}`));
  return out.join("\n") + "\n";
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const file = process.argv[2];
  if (!file) {
    console.error("usage: node report.mjs <windows-<label>.json> [bench/limits.json]");
    process.exit(2);
  }
  const limits = process.argv[3] ? JSON.parse(readFileSync(process.argv[3], "utf8")) : null;
  process.stdout.write(markdown(JSON.parse(readFileSync(file, "utf8")), limits));
}
