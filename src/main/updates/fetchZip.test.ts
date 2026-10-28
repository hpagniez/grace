import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { download, TAR, unpack, verify } from "./fetchZip";

let root: string;
let server: Server;
let base: string;
let zip: string;
let size: number;
let sha: string;
let otherZip: string;
let evilZip: string;

/** A zip of `src`'s contents, made with the same tar the app uses. */
const makeZip = (out: string, src: string) =>
  execFileSync(TAR, ["-a", "-cf", out, "-C", src, "."], { windowsHide: true });

beforeAll(async () => {
  root = mkdtempSync(path.join(tmpdir(), "grace-fetchzip-"));

  // non-ASCII letter and a space, as in a real user folder
  const app = path.join(root, "Zoë src");
  mkdirSync(path.join(app, "resources"), { recursive: true });
  writeFileSync(path.join(app, "Grace.exe"), "exe");
  writeFileSync(path.join(app, "resources", "app.asar"), "asar");
  zip = path.join(root, "Zoë app.zip");
  makeZip(zip, app);
  const bytes = readFileSync(zip);
  size = bytes.length;
  sha = createHash("sha256").update(bytes).digest("hex");

  const other = path.join(root, "other");
  mkdirSync(other);
  writeFileSync(path.join(other, "other.txt"), "x");
  otherZip = path.join(root, "other.zip");
  makeZip(otherZip, other);

  // an entry named ../f.txt, which would land beside the target folder (-P keeps the ".."; this
  // tar has no -s)
  const evil = path.join(root, "evil");
  mkdirSync(path.join(evil, "sub"), { recursive: true });
  writeFileSync(path.join(evil, "f.txt"), "x");
  evilZip = path.join(root, "evil.zip");
  execFileSync(TAR, ["-a", "-cf", evilZip, "-P", "../f.txt"], {
    cwd: path.join(evil, "sub"),
    windowsHide: true,
  });

  server = createServer((req, res) => {
    if (req.url === "/file") {
      res.writeHead(200, { "Content-Length": size });
      res.end(bytes);
    } else if (req.url === "/redirect") {
      res.writeHead(302, { Location: "/file" });
      res.end();
    } else if (req.url === "/stall") {
      // part of a body, then silence
      res.writeHead(200);
      res.write(Buffer.alloc(1000));
    } else {
      res.writeHead(404);
      res.end();
    }
  });
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});

afterAll(async () => {
  server.closeAllConnections();
  await new Promise((r) => server.close(r));
  rmSync(root, { recursive: true, force: true });
});

describe("download", () => {
  it("downloads with progress and follows a redirect", async () => {
    const dest = path.join(root, "new folder", "dl.zip");
    const seen: number[] = [];
    await download(`${base}/redirect`, dest, (n) => seen.push(n), new AbortController().signal);
    expect(readFileSync(dest)).toEqual(readFileSync(zip));
    expect(seen.at(-1)).toBe(size);
  });

  it("deletes the partial file on abort", async () => {
    const dest = path.join(root, "aborted.zip");
    const ac = new AbortController();
    // abort once the first bytes arrive: the file exists and is incomplete
    await expect(download(`${base}/stall`, dest, () => ac.abort(), ac.signal)).rejects.toThrow();
    expect(existsSync(dest)).toBe(false);
  });

  it("rejects on a non-2xx answer and leaves no file", async () => {
    const dest = path.join(root, "missing.zip");
    await expect(
      download(`${base}/nope`, dest, () => {}, new AbortController().signal),
    ).rejects.toThrow(/404/);
    expect(existsSync(dest)).toBe(false);
  });

  it("uses the fetch it is given", async () => {
    const dest = path.join(root, "custom.zip");
    const custom = vi.fn(fetch);
    await download(`${base}/file`, dest, () => {}, new AbortController().signal, custom);
    expect(custom).toHaveBeenCalledOnce();
    expect(existsSync(dest)).toBe(true);
  });
});

describe("verify", () => {
  it("verifies size and sha256", async () => {
    expect(await verify(zip, size, sha)).toBe(true);
    expect(await verify(zip, size, "0".repeat(64))).toBe(false);
    expect(await verify(zip, size + 1, sha)).toBe(false);
  });
});

describe("unpack", () => {
  it("unpacks into a path with spaces and non-ASCII letters", async () => {
    const dir = path.join(root, "Zoë dir", "out");
    await unpack(zip, dir);
    expect(existsSync(path.join(dir, "Grace.exe"))).toBe(true);
    expect(existsSync(path.join(dir, "resources", "app.asar"))).toBe(true);
  });

  it("rejects a zip without Grace.exe", async () => {
    await expect(unpack(otherZip, path.join(root, "other out"))).rejects.toThrow();
  });

  it("rejects a file that is not a zip", async () => {
    const bad = path.join(root, "bad.zip");
    writeFileSync(bad, "not a zip");
    await expect(unpack(bad, path.join(root, "bad out"))).rejects.toThrow();
  });

  it("never writes outside the target folder", async () => {
    // prove the archive really holds the escaping entry
    expect(execFileSync(TAR, ["-tf", evilZip], { encoding: "utf8", windowsHide: true })).toContain(
      "../f.txt",
    );
    const dir = path.join(root, "evil target", "inner");
    await unpack(evilZip, dir).catch(() => {});
    expect(existsSync(path.join(dir, "..", "f.txt"))).toBe(false);
  });
});
