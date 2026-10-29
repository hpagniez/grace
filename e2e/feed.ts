// Release feeds in the GitHub releases API's shape, for GRACE_UPDATE_FEED (spec 2026-10-28-app-updates R13.2).
import { createHash } from "node:crypto";
import { writeFileSync } from "node:fs";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { tempDir } from "./helpers";

type FeedRelease = { version: string; prerelease?: boolean };

/** The API's JSON for these releases, each installable (R1.4); the digest is not of a real zip. */
const feedJson = (releases: FeedRelease[]): string =>
  JSON.stringify(
    releases.map(({ version, prerelease = false }) => ({
      tag_name: `v${version}`,
      draft: false,
      prerelease,
      published_at: "2026-10-29T10:00:00Z",
      assets: [
        {
          name: `Grace-win32-x64-${version}.zip`,
          size: 218_557_130,
          digest: `sha256:${createHash("sha256").update(version).digest("hex")}`,
          browser_download_url: `https://github.com/hpagniez/grace/releases/download/v${version}/Grace-win32-x64-${version}.zip`,
        },
      ],
    })),
  );

/** Writes the feed to a file (UTF-8, no byte order mark), deleted after the test, and returns its path. */
export function feedFile(releases: FeedRelease[]): string {
  const file = path.join(tempDir("kk-feed-"), "releases.json");
  writeFileSync(file, feedJson(releases));
  return file;
}

/** Serves the feed on `http://127.0.0.1:<port>/` and counts the requests. Close it at the end of the test. */
export async function feedServer(releases: FeedRelease[]): Promise<{
  url: string;
  requests(): number;
  close(): Promise<void>;
}> {
  const body = feedJson(releases);
  let count = 0;
  const server = createServer((_req, res) => {
    count++;
    res.writeHead(200, { "Content-Type": "application/json" }).end(body);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  server.unref(); // a test that fails before close() must not keep the worker alive
  const { port } = server.address() as AddressInfo;
  return {
    url: `http://127.0.0.1:${port}/`,
    requests: () => count,
    close: () =>
      new Promise<void>((resolve) => {
        server.closeAllConnections(); // the app's keep-alive connection would hold close() open
        server.close(() => resolve());
      }),
  };
}
