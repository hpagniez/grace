import { readFile } from "node:fs/promises";
import { isAbsolute } from "node:path";
import { parseReleases, RELEASES_API, type Release } from "../../shared/updates";

/** Where releases come from: GRACE_UPDATE_FEED in an unpackaged build (R13.2), else the GitHub API. */
export function updateFeed(env: NodeJS.ProcessEnv, packaged: boolean): string {
  const f = env.GRACE_UPDATE_FEED;
  const local = f && (isAbsolute(f) || f.startsWith("http://127.0.0.1:"));
  return !packaged && local ? f : RELEASES_API;
}

/** Rejects with the R2.5 reason for an error from fetch or the file system. */
const fail = (e: unknown): never => {
  throw new Error(
    e instanceof Error && e.name === "TimeoutError"
      ? "GitHub did not answer in time"
      : "GitHub could not be reached",
  );
};

async function load(feed: string, timeoutMs: number, fetchFn: typeof fetch): Promise<string> {
  if (!/^https?:\/\//.test(feed)) return readFile(feed, "utf8").catch(fail);
  const res = await fetchFn(feed, {
    headers: { Accept: "application/vnd.github+json", "User-Agent": "Grace" },
    signal: AbortSignal.timeout(timeoutMs),
  }).catch(fail);
  if (!res.ok) throw new Error(`GitHub answered ${res.status}`);
  // the timeout also covers the body
  return res.text().catch(fail);
}

/**
 * The installable releases of a feed (a URL or a file). Rejects with the R2.5 reason.
 * `fetchFn` is Electron's `net.fetch` in the app, which honours the system proxy.
 */
export async function fetchReleases(
  feed: string,
  timeoutMs = 15000,
  fetchFn: typeof fetch = fetch,
): Promise<Release[]> {
  const text = await load(feed, timeoutMs, fetchFn);
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    json = null;
  }
  // an object is an error answer ({"message": …}), not "no releases" (R2.5)
  if (!Array.isArray(json)) throw new Error("GitHub's answer could not be read");
  return parseReleases(json);
}
