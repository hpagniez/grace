// The bench's catalogue: every measure id, in table order, with its unit.
// `release: true` marks a measure only a release-mode bench takes (docs/specs/2026-10-31-light-and-fast.md, R2).

/** @typedef {{ unit: "MB" | "ms" | "count" | "fps", release?: true }} Measure */

/** @type {Record<string, Measure>} */
const BASE = {
  "size.unpacked": { unit: "MB" },
  "size.zip": { unit: "MB" },
  "start.paint.fresh": { unit: "ms", release: true },
  "start.tree.fresh": { unit: "ms", release: true },
  "start.paint.reused": { unit: "ms" },
  "start.tree.reused": { unit: "ms" },
  "idle.memory": { unit: "MB" },
  "idle.cpu.shown": { unit: "ms" },
  "idle.cpu.minimised": { unit: "ms", release: true },
};

// Measured on the 2,000-test project, and again on the 10,000-test one (`@10k`) in release mode.
/** @type {Record<string, Measure>} */
const BIG = {
  "run.memory.app": { unit: "MB" },
  "run.memory.java": { unit: "MB" },
  "run.cpu.app": { unit: "ms" },
  "run.cpu.java": { unit: "ms" },
  "run.finishedLag": { unit: "ms" },
};
for (const action of ["select", "expand", "http"])
  for (const state of ["idle", "run"]) BIG[`smooth.${action}.${state}.paint`] = { unit: "ms" };
for (const action of ["select", "expand", "http", "scroll"])
  for (const state of ["idle", "run"]) BIG[`smooth.${action}.${state}.frame`] = { unit: "ms" };
BIG["smooth.run.fps"] = { unit: "fps" };
BIG["smooth.run.frame"] = { unit: "ms" };
Object.assign(BIG, {
  "page.nodes.open": { unit: "count" },
  "page.nodes.run": { unit: "count" },
  "page.heap.open": { unit: "MB" },
  "page.heap.run": { unit: "MB" },
});

/** @type {Record<string, Measure>} */
export const MEASURES = { ...BASE, ...BIG };
for (const [id, measure] of Object.entries(BIG))
  MEASURES[`${id}@10k`] = { ...measure, release: true };
