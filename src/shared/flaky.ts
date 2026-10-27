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
