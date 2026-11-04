// Fetches one OpenAPI document as text, with the limits of spec R2.3.

/** Electron's `net.fetch` in the app, Node's `fetch` in tests. */
export type Fetch = (url: string, init: RequestInit) => Promise<Response>;

export interface FetchLimits {
  timeoutMs: number;
  maxBytes: number;
  maxRedirects: number;
}

const LIMITS: FetchLimits = { timeoutMs: 30_000, maxBytes: 10 * 1024 * 1024, maxRedirects: 5 };

const isHttp = (url: string) => URL.canParse(url) && /^https?:$/.test(new URL(url).protocol);

/** A URL as shown to the person: no user info, no query, no fragment. */
export function displayUrl(url: string): string {
  if (!URL.canParse(url)) return url;
  const u = new URL(url);
  u.username = u.password = u.search = u.hash = "";
  return u.href;
}

/**
 * Reads `url` as text, following redirects itself. `url` in the result is the final one.
 * The messages keep saying 30 seconds and 10 MB, whatever shorter `limits` a test passes.
 */
export async function fetchText(
  url: string,
  fetch: Fetch,
  signal: AbortSignal,
  limits: FetchLimits = LIMITS,
): Promise<{ text: string; url: string }> {
  const timeout = AbortSignal.timeout(limits.timeoutMs);
  const both = AbortSignal.any([signal, timeout]);
  let current = url;
  try {
    for (let redirects = 0; ; redirects++) {
      // Every URL we request, the first included: Electron's net.fetch would also open file: URLs.
      if (!isHttp(current)) throw new Error(`${displayUrl(current)} is not an http or https URL`);
      const res = await fetch(current, { redirect: "manual", signal: both });
      const location = res.status >= 300 && res.status < 400 ? res.headers.get("location") : null;
      if (location !== null) {
        await res.body?.cancel();
        if (redirects === limits.maxRedirects)
          throw new Error(`${displayUrl(url)} redirects more than 5 times`);
        const next = URL.canParse(location, current) ? new URL(location, current) : null;
        if (!next || !isHttp(next.href)) {
          throw new Error(`${displayUrl(current)} redirects to a URL that is not http or https`);
        }
        if (next.username || next.password) {
          throw new Error(`${displayUrl(current)} redirects to a URL with a user name or password`);
        }
        current = next.href;
        continue;
      }
      if (!res.ok) {
        await res.body?.cancel();
        throw new Error(`${displayUrl(current)} answered ${res.status}`);
      }
      return { text: await readText(res, limits.maxBytes, current), url: current };
    }
  } catch (e) {
    // The timeout and the caller's abort both reject with an abort error; only the timeout's is ours to word.
    const abort = e instanceof Error && (e.name === "TimeoutError" || e.name === "AbortError");
    if (abort && timeout.aborted && !signal.aborted) {
      throw new Error(`${displayUrl(current)} did not answer within 30 seconds`);
    }
    throw e;
  }
}

async function readText(res: Response, maxBytes: number, url: string): Promise<string> {
  const reader = res.body?.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  while (reader) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > maxBytes) {
      await reader.cancel();
      throw new Error(`${displayUrl(url)} is larger than 10 MB`);
    }
    chunks.push(value);
  }
  return Buffer.concat(chunks).toString("utf8");
}
