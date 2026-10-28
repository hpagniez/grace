import { latestOn } from "../shared/updates";
import type { UpdateView } from "../shared/updates";
import { compareVersions } from "../shared/version";
import type { AppState } from "./reduce";

/** The latest version on the channel when it is above the running one (R3.2), or null. */
export function latestAvailable(u: UpdateView | null): string | null {
  const l = u && latestOn(u.releases ?? [], u.settings.channel);
  return l && compareVersions(l.version, u.running) > 0 ? l.version : null;
}

/**
 * Why no version can be installed now (R6.4, R13.1), or undefined. Restarting would kill a run, which
 * comes first; a build that is not packaged never installs.
 */
export const updateBlocked = (s: AppState): string | undefined =>
  s.busy || s.run.active
    ? "Finish or cancel the run first"
    : s.updates?.packaged === false
      ? "Updates install only in the packaged app"
      : undefined;
