import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream, createWriteStream } from "node:fs";
import { access, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import type { ReadableStream } from "node:stream/web";
import { promisify } from "node:util";

const run = promisify(execFile);

/** bsdtar reads zip. By absolute path: Git's GNU tar on PATH cannot. */
export const TAR = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");

/**
 * R8.2: streams `url` (redirects followed) to `dest`, and stops past `maxBytes` so a wrong answer
 * cannot fill the disk. The exact size and the hash are `verify`'s job.
 * `fetchFn` is Electron's `net.fetch` in the app, which honours the system proxy.
 */
export async function download(
  url: string,
  dest: string,
  onProgress: (received: number) => void,
  signal: AbortSignal,
  fetchFn: typeof fetch = fetch,
  maxBytes = Infinity,
): Promise<void> {
  await mkdir(path.dirname(dest), { recursive: true });
  try {
    const res = await fetchFn(url, { signal, headers: { "User-Agent": "Grace" } });
    if (!res.ok || !res.body) throw new Error(`The download answered ${res.status}`);
    let received = 0;
    await pipeline(
      Readable.fromWeb(res.body as ReadableStream<Uint8Array>),
      async function* (chunks: AsyncIterable<Buffer>) {
        for await (const chunk of chunks) {
          received += chunk.length;
          if (received > maxBytes) throw new Error("The download is bigger than the release says");
          onProgress(received);
          yield chunk;
        }
      },
      createWriteStream(dest),
      { signal },
    );
  } catch (e) {
    await rm(dest, { force: true });
    throw e;
  }
}

/** R8.3: the file's size and SHA-256 (lower-case hex) match. Streamed: the zip is hundreds of MB. */
export async function verify(file: string, size: number, sha256: string): Promise<boolean> {
  const hash = createHash("sha256");
  let bytes = 0;
  for await (const chunk of createReadStream(file) as AsyncIterable<Buffer>) {
    hash.update(chunk);
    bytes += chunk.length;
  }
  return bytes === size && hash.digest("hex") === sha256.toLowerCase();
}

/**
 * R8.4: unpacks `zip` into `dir`. Rejects unless the app's two key files are there.
 * Without `-P`, bsdtar refuses `..` entries and strips the drive or leading slash of an absolute
 * one, which then lands inside `dir`: nothing lands outside it.
 */
export async function unpack(zip: string, dir: string): Promise<void> {
  await mkdir(dir, { recursive: true });
  try {
    await run(TAR, ["-xf", zip, "-C", dir], { windowsHide: true });
  } catch (e) {
    const { stderr } = e as { stderr?: string };
    throw new Error(`Unpacking failed: ${stderr?.trim() || (e as Error).message}`);
  }
  for (const file of ["Grace.exe", path.join("resources", "app.asar")]) {
    await access(path.join(dir, file)).catch(() => {
      throw new Error(`The unpacked update has no ${file}`);
    });
  }
}
