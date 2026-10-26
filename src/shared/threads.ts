// Shared by main and renderer: keep this file free of Node imports.
// Rules for the thread count (docs/specs/2026-10-26-thread-count.md).

export const MAX_THREADS = 32;

/** True only for a whole number from 1 to MAX_THREADS. */
export const validThreads = (n: unknown): n is number =>
  typeof n === "number" && Number.isInteger(n) && n >= 1 && n <= MAX_THREADS;

export const THREADS_ERROR = `Threads must be a whole number from 1 to ${MAX_THREADS}`;
