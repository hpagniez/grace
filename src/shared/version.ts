// Shared by main and renderer: keep this file free of Node imports.
// Version rules for in-app updates (docs/specs/2026-10-28-app-updates.md R1).

const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-rc\.(0|[1-9]\d*))?$/;

/** Parses `X.Y.Z` or `X.Y.Z-rc.N`, the only forms ADR 0015 allows. Anything else is null. */
export function parseVersion(
  s: string,
): { major: number; minor: number; patch: number; rc: number | null } | null {
  const m = typeof s === "string" ? VERSION.exec(s) : null;
  if (!m) return null;
  return { major: +m[1], minor: +m[2], patch: +m[3], rc: m[4] === undefined ? null : +m[4] };
}

/** SemVer precedence: negative, 0 or positive. Throws when either side is not a valid version. */
export function compareVersions(a: string, b: string): number {
  const x = parseVersion(a);
  const y = parseVersion(b);
  if (!x || !y) throw new Error(`Invalid version: ${x ? b : a}`);
  // a release (no rc) ranks above any of its release candidates; Infinity - Infinity is NaN, so `|| 0`
  const rc = (v: { rc: number | null }) => v.rc ?? Infinity;
  return x.major - y.major || x.minor - y.minor || x.patch - y.patch || rc(x) - rc(y) || 0;
}

/** True for a valid `X.Y.Z-rc.N`. */
export const isPrerelease = (v: string): boolean => typeof parseVersion(v)?.rc === "number";
