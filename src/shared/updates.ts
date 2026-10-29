// Shared by main and renderer: keep this file free of Node imports.
// Releases, channels and update settings (docs/specs/2026-10-28-app-updates.md R1, R4, R5, R12).
import { compareVersions, isPrerelease, parseVersion } from "./version";

export type Channel = "stable" | "prerelease";

/** The first release that ships the updater; older ones cannot be installed from the app. */
export const FIRST_UPDATER = "1.0.0-rc.4";

export const RELEASES_API = "https://api.github.com/repos/hpagniez/grace/releases?per_page=100";

export const releasePage = (v: string) => `https://github.com/hpagniez/grace/releases/tag/v${v}`;
export const zipName = (v: string) => `Grace-win32-x64-${v}.zip`;
export const downloadPrefix = (v: string) =>
  `https://github.com/hpagniez/grace/releases/download/v${v}/`;

/** An installable release (R1.4), reduced to the fields Grace uses. */
export interface Release {
  version: string;
  prerelease: boolean;
  publishedAt: string;
  url: string;
  size: number;
  sha256: string;
}

const isObject = (x: unknown): x is Record<string, unknown> => typeof x === "object" && x !== null;

/** One GitHub release entry, checked field by field (R1.4); null when it is not installable. */
function toRelease(e: unknown): Release | null {
  if (!isObject(e) || e.draft !== false || typeof e.prerelease !== "boolean") return null;
  const tag = e.tag_name;
  if (typeof tag !== "string" || !tag.startsWith("v") || !Array.isArray(e.assets)) return null;
  const version = tag.slice(1);
  if (!parseVersion(version) || compareVersions(version, FIRST_UPDATER) < 0) return null;
  const zips = e.assets.filter((a) => isObject(a) && a.name === zipName(version));
  if (zips.length !== 1) return null;
  const { size, digest, browser_download_url: url } = zips[0] as Record<string, unknown>;
  const sha256 =
    typeof digest === "string" ? /^sha256:([0-9a-f]{64})$/.exec(digest)?.[1] : undefined;
  if (typeof size !== "number" || !Number.isInteger(size) || size <= 0 || !sha256) return null;
  // exactly: a prefix would let `/../` take the download elsewhere
  if (url !== downloadPrefix(version) + zipName(version)) return null;
  const date = e.published_at;
  const publishedAt = typeof date === "string" && !Number.isNaN(Date.parse(date)) ? date : "";
  return { version, prerelease: e.prerelease, publishedAt, url, size, sha256 };
}

/** The installable releases in a releases API response, newest first, one per version (R1.4). */
export function parseReleases(json: unknown): Release[] {
  if (!Array.isArray(json)) return [];
  const rs = json.flatMap((e) => toRelease(e) ?? []);
  rs.sort((a, b) => compareVersions(b.version, a.version));
  return rs.filter((r, i) => i === 0 || r.version !== rs[i - 1].version);
}

/** The releases a channel offers: stable hides pre-releases. */
export const onChannel = (rs: Release[], c: Channel): Release[] =>
  c === "stable" ? rs.filter((r) => !r.prerelease) : rs;

/** The highest version on the channel (R1.5), or null. */
export const latestOn = (rs: Release[], c: Channel): Release | null =>
  onChannel(rs, c).reduce<Release | null>(
    (best, r) => (!best || compareVersions(r.version, best.version) > 0 ? r : best),
    null,
  );

export interface UpdateSettings {
  /** Null until the person picks one, and never saved till then: see `channelOf` (R1.3). */
  channel: Channel | null;
  checkAtStartup: boolean;
  autoUpdate: boolean;
  skipped: string | null;
  failed: string | null;
  lastChecked: string | null;
}

/** R1.3: the channel picked, else pre-release while a pre-release runs and stable otherwise. */
export const channelOf = (s: UpdateSettings, running: string): Channel =>
  s.channel ?? (isPrerelease(running) ? "prerelease" : "stable");

/** Settings before the person changes any (R12). */
export const defaultUpdateSettings = (): UpdateSettings => ({
  channel: null,
  checkAtStartup: true,
  autoUpdate: false,
  skipped: null,
  failed: null,
  lastChecked: null,
});

/** The release the startup modal offers: out of date, auto-update off, above the skipped one (R4.1). */
export function promptFor(rs: Release[], s: UpdateSettings, running: string): Release | null {
  const latest = latestOn(rs, channelOf(s, running));
  if (!latest || s.autoUpdate || compareVersions(latest.version, running) <= 0) return null;
  return s.skipped !== null && compareVersions(latest.version, s.skipped) <= 0 ? null : latest;
}

/** The release auto-update installs at startup: skipped counts for nothing, failed is never retried (R5.2). */
export function autoUpdateTo(rs: Release[], s: UpdateSettings, running: string): Release | null {
  const latest = latestOn(rs, channelOf(s, running));
  if (!latest || !s.autoUpdate || compareVersions(latest.version, running) <= 0) return null;
  return latest.version === s.failed ? null : latest;
}

/** A saved copy of the data of one version (R9). */
export interface Backup {
  version: string;
  savedAt: string;
}

/** What the renderer shows about updates. */
export interface UpdateView {
  running: string;
  packaged: boolean;
  settings: UpdateSettings;
  releases: Release[] | null;
  checking: boolean;
  error: string | null;
  backups: Backup[];
}

/** Install progress pushed from main to the renderer (R8). */
export type UpdateProgress =
  | {
      version: string;
      rollback: boolean;
      /** `restart`: the hand-over to the switch script, which cannot be cancelled (R8.5). */
      step: "download" | "check" | "unpack" | "restart";
      received: number;
      total: number;
    }
  | { version: string; rollback: boolean; error: string };
