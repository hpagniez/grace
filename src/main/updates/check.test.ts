import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { downloadPrefix, RELEASES_API, zipName } from "../../shared/updates";
import { fetchReleases, updateFeed } from "./check";

/** A GitHub API release entry that is installable. */
const gh = (v: string) => ({
  tag_name: `v${v}`,
  draft: false,
  prerelease: true,
  published_at: "2026-10-28T10:00:00Z",
  assets: [
    {
      name: zipName(v),
      size: 218_000_000,
      digest: `sha256:${"ab".repeat(32)}`,
      browser_download_url: `${downloadPrefix(v)}${zipName(v)}`,
    },
  ],
});
const feedJson = JSON.stringify([gh("1.0.0-rc.5"), gh("1.0.0-rc.4"), { tag_name: "junk" }]);

let server: Server;
let base: string;
let dir: string;

beforeAll(async () => {
  dir = await mkdtemp(join(tmpdir(), "grace-feed-"));
  server = createServer((req, res) => {
    if (req.url === "/ok") res.end(feedJson);
    else if (req.url === "/object") res.end('{"message":"Not Found"}');
    else if (req.url === "/403") res.writeHead(403).end("rate limited");
    else if (req.url === "/html") res.end("<html>sign in</html>");
    // /hang: never answers
  });
  await new Promise<void>((ok) => server.listen(0, "127.0.0.1", ok));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});

afterAll(async () => {
  server.closeAllConnections();
  await new Promise((ok) => server.close(ok));
  await rm(dir, { recursive: true, force: true });
});

describe("fetchReleases", () => {
  it("reads releases from a URL and from a file", async () => {
    const file = join(dir, "feed.json");
    await writeFile(file, feedJson);
    const fromUrl = await fetchReleases(`${base}/ok`);
    expect(fromUrl.map((r) => r.version)).toEqual(["1.0.0-rc.5", "1.0.0-rc.4"]);
    expect(await fetchReleases(file)).toEqual(fromUrl);
  });

  it("fails on JSON that is not a releases list", async () => {
    await expect(fetchReleases(`${base}/object`)).rejects.toThrow(
      "GitHub's answer could not be read",
    );
  });

  it("gives no releases, and no error, for an empty list", async () => {
    const f = vi.fn<typeof fetch>(async () => new Response("[]"));
    expect(await fetchReleases("https://example.test/releases", 1000, f)).toEqual([]);
  });

  it("fails with the status on HTTP errors", async () => {
    await expect(fetchReleases(`${base}/403`)).rejects.toThrow("GitHub answered 403");
  });

  it("fails on an unreadable answer", async () => {
    await expect(fetchReleases(`${base}/html`)).rejects.toThrow(
      "GitHub's answer could not be read",
    );
    const file = join(dir, "bad.json");
    await writeFile(file, "not json");
    await expect(fetchReleases(file)).rejects.toThrow("GitHub's answer could not be read");
  });

  it("times out", async () => {
    await expect(fetchReleases(`${base}/hang`, 200)).rejects.toThrow(
      "GitHub did not answer in time",
    );
  });

  it("fails offline", async () => {
    // a port that was just free: nothing listens on it
    const s = createServer();
    await new Promise<void>((ok) => s.listen(0, "127.0.0.1", ok));
    const { port } = s.address() as AddressInfo;
    await new Promise((ok) => s.close(ok));
    await expect(fetchReleases(`http://127.0.0.1:${port}/`)).rejects.toThrow(
      "GitHub could not be reached",
    );
  });

  it("fetches with the function it is given: the app passes Electron's net.fetch", async () => {
    const f = vi.fn<typeof fetch>(async () => new Response(feedJson));
    const rs = await fetchReleases("https://example.test/releases", 1000, f);
    expect(f).toHaveBeenCalledOnce();
    expect(rs.map((r) => r.version)).toEqual(["1.0.0-rc.5", "1.0.0-rc.4"]);
  });

  it("fails when the feed file is missing", async () => {
    await expect(fetchReleases(join(dir, "missing.json"))).rejects.toThrow(
      "GitHub could not be reached",
    );
  });
});

describe("updateFeed", () => {
  it("honours GRACE_UPDATE_FEED only when not packaged", () => {
    const url = "http://127.0.0.1:5000/r";
    expect(updateFeed({ GRACE_UPDATE_FEED: url }, false)).toBe(url);
    expect(updateFeed({ GRACE_UPDATE_FEED: url }, true)).toBe(RELEASES_API);
    expect(updateFeed({ GRACE_UPDATE_FEED: "https://evil.example/r" }, false)).toBe(RELEASES_API);
  });

  it("accepts an absolute file path and nothing else local", () => {
    const abs = join(tmpdir(), "feed.json");
    expect(updateFeed({ GRACE_UPDATE_FEED: abs }, false)).toBe(abs);
    expect(updateFeed({ GRACE_UPDATE_FEED: "feed.json" }, false)).toBe(RELEASES_API);
    expect(updateFeed({ GRACE_UPDATE_FEED: "http://localhost:5000/r" }, false)).toBe(RELEASES_API);
    expect(updateFeed({}, false)).toBe(RELEASES_API);
  });
});
