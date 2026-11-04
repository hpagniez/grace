import { createServer, type IncomingMessage, type Server, type ServerResponse } from "node:http";
import type { AddressInfo } from "node:net";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { urlError } from "../../shared/openapi";
import { displayUrl, fetchText } from "./fetch";

const BAD_URL = "Enter an http or https URL";
const CREDENTIALS = "Remove the user name and password from the URL";

describe("urlError", () => {
  it("accepts an absolute http or https URL", () => {
    expect(urlError("https://a.test/x.yaml")).toBeNull();
    expect(urlError("http://127.0.0.1:8080/x?token=1")).toBeNull();
  });

  it("refuses anything that is not one", () => {
    for (const url of ["file:///C:/x.yaml", "ftp://a", "a.test/x", ""]) {
      expect(urlError(url), url).toBe(BAD_URL);
    }
  });

  it("accepts 2,000 characters and refuses 2,001", () => {
    const url = (n: number) => `https://a.test/${"x".repeat(n - 15)}`;
    expect(urlError(url(2000))).toBeNull();
    expect(urlError(url(2001))).toBe(BAD_URL);
  });

  it("refuses a user name or password", () => {
    expect(urlError("https://u:p@a.test/x")).toBe(CREDENTIALS);
    expect(urlError("https://u@a.test/x")).toBe(CREDENTIALS);
  });
});

describe("displayUrl", () => {
  it("drops user info, query and fragment", () => {
    expect(displayUrl("https://u:p@a.test/x.yaml?token=1#f")).toBe("https://a.test/x.yaml");
  });
});

type Handler = (req: IncomingMessage, res: ServerResponse) => void;

describe("fetchText", () => {
  const routes: Record<string, Handler> = {};
  let server: Server;
  let base: string;
  let hits: string[];
  const ok =
    (body: string): Handler =>
    (_, res) =>
      res.end(body);
  const redirect =
    (status: number, location: string): Handler =>
    (_, res) => {
      res.writeHead(status, { location });
      res.end();
    };
  const run = (path: string, limits?: Parameters<typeof fetchText>[3]) =>
    fetchText(base + path, fetch, new AbortController().signal, limits);

  beforeEach(async () => {
    hits = [];
    for (const key of Object.keys(routes)) delete routes[key];
    server = createServer((req, res) => {
      hits.push(req.url!);
      (
        routes[req.url!] ??
        ((_, r) => {
          r.statusCode = 404;
          r.end();
        })
      )(req, res);
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  });

  afterEach(async () => {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  });

  it("returns the body and the final URL", async () => {
    routes["/x"] = ok("openapi: 3.0.0");
    expect(await run("/x")).toEqual({ text: "openapi: 3.0.0", url: `${base}/x` });
  });

  it("follows redirects by hand and reports the last URL", async () => {
    routes["/a"] = redirect(302, "/b");
    routes["/b"] = redirect(301, `${base}/c`);
    routes["/c"] = ok("done");
    expect(await run("/a")).toEqual({ text: "done", url: `${base}/c` });
    expect(hits).toEqual(["/a", "/b", "/c"]);
  });

  it("follows 5 redirects and refuses the 6th", async () => {
    for (let i = 1; i <= 5; i++) routes[`/s${i}`] = redirect(302, `/s${i + 1}`);
    routes["/s6"] = ok("done");
    expect((await run("/s1")).url).toBe(`${base}/s6`);
    for (let i = 1; i <= 6; i++) routes[`/r${i}`] = redirect(302, `/r${i + 1}`);
    routes["/r7"] = ok("done");
    await expect(run("/r1")).rejects.toThrow(`${base}/r1 redirects more than 5 times`);
    expect(hits).not.toContain("/r7");
  });

  it("refuses a redirect to a URL that is not http or https", async () => {
    routes["/start"] = redirect(302, "file:///C:/x");
    await expect(run("/start")).rejects.toThrow(
      `${base}/start redirects to a URL that is not http or https`,
    );
    expect(hits).toEqual(["/start"]);
  });

  it("reports a status other than 2xx", async () => {
    await expect(run("/x?token=1")).rejects.toThrow(`${base}/x answered 404`);
  });

  it("refuses a body past 10 MB, streamed", async () => {
    routes["/big"] = (_, res) => {
      const chunk = Buffer.alloc(64 * 1024, "a");
      let left = 10 * 1024 * 1024 + 1;
      res.on("error", () => {});
      const write = () => {
        while (left > 0) {
          const part = left >= chunk.length ? chunk : chunk.subarray(0, left);
          left -= part.length;
          if (!res.write(part)) return void res.once("drain", write);
        }
        res.end();
      };
      write();
    };
    await expect(run("/big")).rejects.toThrow(`${base}/big is larger than 10 MB`);
  });

  it("accepts a body of exactly the limit and refuses one byte more", async () => {
    routes["/exact"] = ok("12345");
    routes["/over"] = ok("123456");
    const limits = { timeoutMs: 5000, maxBytes: 5, maxRedirects: 5 };
    expect((await run("/exact", limits)).text).toBe("12345");
    await expect(run("/over", limits)).rejects.toThrow("is larger than 10 MB");
  });

  it("gives up when the server never answers", async () => {
    routes["/hang"] = () => {};
    await expect(run("/hang", { timeoutMs: 200, maxBytes: 1024, maxRedirects: 5 })).rejects.toThrow(
      `${base}/hang did not answer within 30 seconds`,
    );
  });

  it("rejects with an AbortError when the caller aborts", async () => {
    routes["/hang"] = () => {};
    const controller = new AbortController();
    const result = fetchText(`${base}/hang`, fetch, controller.signal);
    const check = expect(result).rejects.toMatchObject({ name: "AbortError" });
    await vi.waitFor(() => expect(hits).toEqual(["/hang"]));
    controller.abort();
    await check;
  });
});
