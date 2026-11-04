// Shared by main and renderer: keep this file free of Node imports.
// What the window gets for the trend strip (docs/specs/2026-11-04-trends.md).

/** One stored run as the trend strip shows it, for the selection asked about. */
export interface TrendPoint {
  /** The run folder's name. */
  id: string;
  startedAt: number;
  /** Counted tests (stored `passed` or `failed`, under the selection) that passed. */
  passed: number;
  failed: number;
  /** The tests under the selection in the current tree. */
  total: number;
  /** The mean `durationMillis` of the counted tests that have one; null if none has. */
  meanMillis: number | null;
}
