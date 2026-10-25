import path from "node:path";
import { describe, expect, it } from "vitest";
import { readAllEvents } from "./events";
import { extractCalls, parseStepLog } from "./http";

// The spec's example of a successful call (docs/specs/2026-10-25-http-inspector.md, "What Karate writes").
const CALL = [
  "request:",
  "1 > POST http://localhost:60219/users",
  "1 > Content-Type: application/json; charset=UTF-8",
  "1 > Host: localhost:60219",
  "1 > Content-Length: 16",
  "1 > Connection: keep-alive",
  "1 > User-Agent: Apache-HttpClient/5.6.5 (Java/21.0.12.1)",
  "{",
  '  "name": "alice"',
  "}",
  "",
  "response time in milliseconds: 6",
  "1 < 201 POST http://localhost:60219/users",
  "1 < content-length: 23",
  "1 < Content-Type: application/json; charset=UTF-8",
  "{",
  '  "name": "alice",',
  '  "id": 1',
  "}",
  "",
  "POST http://localhost:60219/users",
  "201 (6 ms)",
  "",
].join("\n");

const RESPONSE_BODY = '{\n  "name": "alice",\n  "id": 1\n}';

describe("parseStepLog", () => {
  it("parses one call", () => {
    expect(parseStepLog(CALL)).toEqual([
      {
        method: "POST",
        url: "http://localhost:60219/users",
        requestHeaders: [
          { name: "Content-Type", value: "application/json; charset=UTF-8" },
          { name: "Host", value: "localhost:60219" },
          { name: "Content-Length", value: "16" },
          { name: "Connection", value: "keep-alive" },
          { name: "User-Agent", value: "Apache-HttpClient/5.6.5 (Java/21.0.12.1)" },
        ],
        requestBody: '{\n  "name": "alice"\n}',
        status: 201,
        responseHeaders: [
          { name: "content-length", value: "23" },
          { name: "Content-Type", value: "application/json; charset=UTF-8" },
        ],
        responseBody: RESPONSE_BODY,
        millis: 6,
        attempt: null,
      },
    ]);
  });

  it("ignores text before request: and the summary lines", () => {
    const one = parseStepLog(CALL);
    expect(parseStepLog(`karate-config.js running, env = null\n${CALL}`)).toEqual(one);
  });

  it("keeps blank lines inside a body", () => {
    const log = CALL.replace('"id": 1\n}', '"id": 1\n}\n\nb').replace(
      '{\n  "name": "alice",',
      'a\n\n{\n  "name": "alice",',
    );
    const [call] = parseStepLog(log);
    expect(call.responseBody).toBe('a\n\n{\n  "name": "alice",\n  "id": 1\n}\n\nb');
  });

  it("a request: line inside a body does not start a call", () => {
    const log = CALL.replace("}\n\nPOST", "}\nrequest:\nnot a header\n\nPOST");
    const calls = parseStepLog(log);
    expect(calls).toHaveLength(1);
    expect(calls[0].responseBody).toBe(`${RESPONSE_BODY}\nrequest:\nnot a header`);
  });

  it("reads CRLF lines", () => {
    expect(parseStepLog(CALL.replaceAll("\n", "\r\n"))).toEqual(parseStepLog(CALL));
  });

  it("numbers retry attempts", () => {
    const pair = (n: number) =>
      `request:\n${n} > GET http://h/x\n${n} > Host: h\n\nresponse time in milliseconds: ${n}\n${n} < 404 GET http://h/x\n${n} < Content-Length: 0\n\n`;
    const calls = parseStepLog(pair(1) + pair(2) + pair(3));
    expect(calls.map((c) => c.attempt)).toEqual([
      { n: 1, of: 3 },
      { n: 2, of: 3 },
      { n: 3, of: 3 },
    ]);
    expect(calls.map((c) => c.status)).toEqual([404, 404, 404]);
    expect(calls[0].requestBody).toBeNull();
    expect(calls[0].responseBody).toBeNull();
  });

  it("pairs a response with the request of the same number", () => {
    const log =
      "request:\n1 > GET http://h/a\n1 > Host: h\n\n" +
      "request:\n2 > GET http://h/b\n2 > Host: h\n\n" +
      "response time in milliseconds: 5\n2 < 200 GET http://h/b\n2 < Content-Length: 3\ntwo\n\n" +
      "response time in milliseconds: 7\n1 < 404 GET http://h/a\n1 < Content-Length: 3\none\n";
    const calls = parseStepLog(log);
    expect(calls.map((c) => [c.url, c.status, c.millis, c.responseBody])).toEqual([
      ["http://h/a", 404, 7, "one"],
      ["http://h/b", 200, 5, "two"],
    ]);
    expect(calls.map((c) => c.attempt)).toEqual([
      { n: 1, of: 2 },
      { n: 2, of: 2 },
    ]);
  });

  it("drops a followed redirect's unanswered request", () => {
    const log =
      "request:\n1 > GET http://h/moved\n1 > Host: h\n\n" +
      "request:\n2 > GET http://h/moved\n2 > Host: h\n\nresponse time in milliseconds: 1\n2 < 200 GET http://h/moved\n2 < Content-Length: 2\nok\n\nGET http://h/moved\n200 (1 ms)\n";
    const calls = parseStepLog(log);
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ status: 200, millis: 1, responseBody: "ok", attempt: null });
  });

  it("keeps a last unanswered request as no response", () => {
    const calls = parseStepLog("request:\n1 > GET http://h/slow\n1 > Host: h\n\n");
    expect(calls).toEqual([
      {
        method: "GET",
        url: "http://h/slow",
        requestHeaders: [{ name: "Host", value: "h" }],
        requestBody: null,
        status: null,
        responseHeaders: [],
        responseBody: null,
        millis: null,
        attempt: null,
      },
    ]);
  });

  it("finds nothing in a log without request:", () => {
    expect(parseStepLog("env is null devFlag is false")).toEqual([]);
  });
});

const exitOf = (file: string, feature: string) =>
  readAllEvents(path.resolve("test/fixtures/events", file)).find(
    (e) => e.type === "FEATURE_EXIT" && e.data.callDepth === 0 && e.data.relativePath === feature,
  )!.data;

/** Calls by scenario name, through the refId as the app's tree keys them. */
const callsByName = (file: string, feature: string) => {
  const data = exitOf(file, feature);
  const calls = extractCalls(data);
  return Object.fromEntries(
    data.scenarioResults.map((sc: Record<string, any>) => [
      sc.name,
      calls.find((c) => c.refId === sc.refId)!.calls,
    ]),
  );
};

describe("extractCalls", () => {
  it("reads every case of the http sample", () => {
    const by = callsByName("run-http.jsonl", "tests/http.feature");

    // `ok`'s first step also carries start-mock.feature (config callSingle), which logs nothing
    expect(by.ok).toHaveLength(1);
    expect(by.ok[0]).toMatchObject({ method: "GET", status: 200, via: null, error: null });

    expect(by.retry.map((c: any) => [c.status, c.attempt])).toEqual([
      [404, { n: 1, of: 3 }],
      [404, { n: 2, of: 3 }],
      [404, { n: 3, of: 3 }],
    ]);

    expect(by["server error"].map((c: any) => c.status)).toEqual([500]);

    expect(by.refused).toHaveLength(1);
    expect(by.refused[0]).toMatchObject({ method: "GET", url: null, status: null });
    expect(by.refused[0].error).toContain("Connection refused");

    expect(by.timeout).toHaveLength(1);
    expect(by.timeout[0].url).toMatch(/\/slow$/);
    expect(by.timeout[0].status).toBeNull();
    expect(by.timeout[0].error).toContain("Read timed out");

    expect(by["redirect followed"].map((c: any) => c.status)).toEqual([200]);

    expect(by["redirect kept"]).toHaveLength(1);
    expect(by["redirect kept"][0].status).toBe(302);
    expect(by["redirect kept"][0].responseHeaders).toContainEqual({
      name: "Location",
      value: "/hello",
    });

    expect(by["via helper"]).toHaveLength(1);
    expect(by["via helper"][0]).toMatchObject({
      method: "POST",
      status: 201,
      via: "tests/common/create-user.feature:8",
    });

    expect(by.xml[0].requestBody).toBe("<user><name>bob</name></user>");
    expect(by.xml[0].status).toBe(200);

    expect(by["secret header"]).toHaveLength(1);
    expect(by["secret header"][0].status).toBe(200);
    expect(by["secret header"][0].requestHeaders).toContainEqual({
      name: "X-Token",
      value: "none",
    });

    expect(Object.values(by).flat()).toHaveLength(12);
  });

  it("puts a callonce call under the first scenario only", () => {
    const by = callsByName("run-http.jsonl", "tests/shared.feature");
    expect(by.first).toHaveLength(1);
    expect(by.first[0].via).toBe("tests/common/create-user.feature:8");
    expect(by.second).toEqual([]);
  });

  it("puts a callSingle call under the first scenario", () => {
    const by = callsByName("run-http-config.jsonl", "tests/shared.feature");
    expect(by.first.map((c: any) => c.via)).toContain("auth.feature:8");
    expect(by.second).toEqual([]);
  });

  it("puts a failed callSingle's call under the first scenario's config step", () => {
    const by = callsByName("run-http-config-fail.jsonl", "tests/shared.feature");
    expect(by.first).toHaveLength(1);
    expect(by.first[0]).toMatchObject({ via: "auth.feature:8", status: 201 });
    expect(by.second).toEqual([]);
  });

  it("walks callResults two deep", () => {
    const step = (line: number, extra: Record<string, any> = {}) => ({
      step: { keyword: "method", text: "get", line },
      result: { status: "passed", startTime: 1 },
      ...extra,
    });
    const log =
      "request:\n1 > GET http://h/x\n1 > Host: h\n\nresponse time in milliseconds: 2\n1 < 200 GET http://h/x\n";
    const feature = (relativePath: string, stepResults: unknown[]) => ({
      relativePath,
      scenarioResults: [{ stepResults }],
    });
    const calls = extractCalls({
      scenarioResults: [
        {
          refId: "[1:1]",
          stepResults: [
            step(3, {
              callResults: [
                feature("a.feature", [
                  step(5, {
                    callResults: [feature("b.feature", [step(9, { stepLog: log })])],
                  }),
                ]),
              ],
            }),
          ],
        },
      ],
    });
    expect(calls).toHaveLength(1);
    expect(calls[0].refId).toBe("[1:1]");
    expect(calls[0].calls).toHaveLength(1);
    expect(calls[0].calls[0]).toMatchObject({ status: 200, via: "b.feature:9" });
  });

  it("orders a test's calls by start time", () => {
    const log = (n: number) =>
      `request:\n1 > GET http://h/${n}\n1 > Host: h\n\nresponse time in milliseconds: 1\n1 < 200 GET http://h/${n}\n`;
    const step = (line: number, startTime: number, extra: Record<string, any> = {}) => ({
      step: { keyword: "method", text: "get", line },
      result: { status: "passed", startTime },
      ...extra,
    });
    const [{ calls }] = extractCalls({
      scenarioResults: [
        {
          refId: "[1:1]",
          stepResults: [
            step(1, 300, { stepLog: log(1) }),
            step(2, 100, { stepLog: log(2) }),
            step(3, 200, { stepLog: log(3) }),
            // equal start times keep the log order
            step(4, 200, { stepLog: log(4) }),
          ],
        },
      ],
    });
    expect(calls.map((c) => c.url)).toEqual([
      "http://h/2",
      "http://h/3",
      "http://h/4",
      "http://h/1",
    ]);
  });
});
