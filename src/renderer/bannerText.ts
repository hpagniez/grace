import type { TestNode } from "../shared/model";
import { leaves } from "./nodes";
import type { AppState } from "./reduce";

/** What the console's state line says, while it has no output: F24 spec R2.1. */
export type BannerState = "none" | "finding" | "starting" | "ready";

const features = (n: TestNode): number =>
  (n.kind === "feature" ? 1 : 0) + n.children.reduce((sum, c) => sum + features(c), 0);

const count = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

/** The banner's tests line, null for no line. Counts the whole tree, not the tag filter. */
export function testsText(s: AppState): string | null {
  if (!s.project) return null;
  if (s.error) return "could not be listed: see the message in the tree";
  if (!s.tree) return s.busy ? "finding tests…" : null;
  const n = leaves(s.tree).length;
  if (n === 0) return "No feature files found";
  return `${count(n, "scenario")} in ${count(features(s.tree), "feature")}`;
}

/** Null once the console has output: the lines then follow the banner. */
export function bannerState(s: AppState): BannerState | null {
  if (s.console.length > 0) return null;
  if (s.run.active) return "starting";
  if (s.busy) return "finding";
  return s.project ? "ready" : "none";
}
