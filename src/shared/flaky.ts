// Shared by main and renderer: keep this file free of Node imports.
// What the window gets about flaky tests (docs/specs/2026-10-26-flaky-tests.md).

/** One run that proves a test flaky: the test `passed` or `failed` in it. */
export interface FlakyRun {
  id: string;
  startedAt: number;
  status: "passed" | "failed";
  threads: number | null;
  reportAvailable: boolean;
}

/** The proof of one environment (null: none): the runs with the same project files, newest first. */
export interface FlakyEnv {
  env: string | null;
  passed: number;
  failed: number;
  runs: FlakyRun[];
}

/** A test of the tree last sent that is flaky, by its key in that tree. */
export interface FlakyTest {
  key: string;
  envs: FlakyEnv[];
}

export const FLAKY_TITLE = "Flaky: passed and failed with the same project files";

export const flakyHeading = (env: string | null): string =>
  env === null ? "Flaky with no environment" : `Flaky on ${env}`;

const times = (n: number): string => `${n} ${n === 1 ? "time" : "times"}`;

export const flakyCounts = (passed: number, failed: number): string =>
  `Passed ${times(passed)}, failed ${times(failed)} with the same project files`;

export const threadsLabel = (n: number | null): string =>
  n === null ? "—" : n === 1 ? "1 thread" : `${n} threads`;

/** The current environment first, then none, then the rest by name. */
export function orderEnvs(envs: FlakyEnv[], current: string | null): FlakyEnv[] {
  const rank = (e: FlakyEnv) => (e.env === current ? 0 : e.env === null ? 1 : 2);
  return [...envs].sort((a, b) => rank(a) - rank(b) || (a.env ?? "").localeCompare(b.env ?? ""));
}
