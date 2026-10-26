import type { HttpCall, HttpHeader } from "../../shared/http";

/** What one step log says about a call; the step supplies the rest. */
export type LoggedCall = Omit<HttpCall, "error" | "via" | "startTime">;

const REQUEST_LINE = /^(\d+) > (\S*) ?(.*)$/;
const RESPONSE_TIME = /^response time in milliseconds: (\d+)$/;
const RESPONSE_LINE = /^(\d+) < (\d+)/;
// The two lines Karate adds after a `method` step: `METHOD URL`, then `STATUS (T ms)`.
const SUMMARY_REQUEST = /^\S+ \S+$/;
const SUMMARY_STATUS = /^\d+ \(\d+ ms\)$/;

/** A `<prefix>Name: value` line as a header; null when the line is not one. */
function header(line: string, prefix: string): HttpHeader | null {
  if (!line.startsWith(prefix)) return null;
  const rest = line.slice(prefix.length);
  const colon = rest.indexOf(":");
  if (colon < 1) return null;
  return { name: rest.slice(0, colon), value: rest.slice(colon + 1).replace(/^ /, "") };
}

const bodyOf = (lines: string[]) => (lines.length ? lines.join("\n") : null);

/**
 * The calls one step log holds (spec R1). Karate's format is not a contract: the tests run on recorded logs.
 * A response attaches to the request with the same number `N`, wherever it sits in the log.
 */
export function parseStepLog(log: string): LoggedCall[] {
  const lines = log.split(/\r?\n/);
  if (lines.at(-1) === "") lines.pop(); // the newline that ends the last line
  let end = lines.length;
  if (end >= 2 && SUMMARY_STATUS.test(lines[end - 1]) && SUMMARY_REQUEST.test(lines[end - 2]))
    end -= 2;
  // A marker line inside a body is body text: `request:` starts a call only before a `N > ` line,
  // and `response time in milliseconds` starts a response only before a `N < ` line.
  const startsCall = (i: number) =>
    lines[i] === "request:" && i + 1 < end && /^\d+ > /.test(lines[i + 1]);
  const startsResponse = (i: number) =>
    i + 1 < end && RESPONSE_TIME.test(lines[i]) && RESPONSE_LINE.test(lines[i + 1]);

  const requests: { n: string; call: LoggedCall }[] = [];
  for (let i = 0; i < end;) {
    if (startsCall(i)) {
      const [, n, method, url] = REQUEST_LINE.exec(lines[i + 1])!;
      const call: LoggedCall = {
        method,
        url,
        requestHeaders: [],
        requestBody: null,
        status: null,
        responseHeaders: [],
        responseBody: null,
        millis: null,
        attempt: null,
      };
      i += 2;
      for (let h; i < end && (h = header(lines[i], `${n} > `)); i++) call.requestHeaders.push(h);
      const body: string[] = [];
      while (i < end && !startsCall(i) && !startsResponse(i)) body.push(lines[i++]);
      if (body.at(-1) === "") body.pop();
      call.requestBody = bodyOf(body);
      requests.push({ n, call });
    } else if (startsResponse(i)) {
      const millis = Number(RESPONSE_TIME.exec(lines[i])![1]);
      const [, n, status] = RESPONSE_LINE.exec(lines[i + 1])!;
      const headers: HttpHeader[] = [];
      i += 2;
      for (let h; i < end && (h = header(lines[i], `${n} < `)); i++) headers.push(h);
      const body: string[] = [];
      while (i < end && !startsCall(i) && !startsResponse(i)) body.push(lines[i++]);
      while (body.at(-1) === "") body.pop();
      // A response with no request of its number has no call to join: Karate never writes one.
      const request = requests.find((r) => r.n === n && r.call.status === null);
      if (request)
        Object.assign(request.call, {
          status: Number(status),
          responseHeaders: headers,
          responseBody: bodyOf(body),
          millis,
        });
    } else {
      i++;
    }
  }
  // An unanswered request with another request after it is a followed redirect, not a call.
  const calls = requests
    .filter((r, k) => r.call.status !== null || k === requests.length - 1)
    .map((r) => r.call);
  // Retries resend one request: several calls to different requests are separate calls, not attempts.
  if (
    calls.length > 1 &&
    calls.every((c) => c.method === calls[0].method && c.url === calls[0].url)
  )
    calls.forEach((c, k) => (c.attempt = { n: k + 1, of: calls.length }));
  return calls;
}

/** Appends the calls of these steps, then of the features they called, at any depth. `feature` is null at depth 0. */
function collect(
  stepResults: Record<string, any>[],
  feature: string | null,
  out: HttpCall[],
): void {
  for (const r of stepResults) {
    const via = feature === null ? null : `${feature}:${r.step.line}`;
    const startTime: number = r.result.startTime;
    const error: string | null = r.result.errorMessage ?? null;
    const logged = parseStepLog(r.stepLog ?? "");
    for (const c of logged)
      out.push({ ...c, error: c.status === null ? error : null, via, startTime });
    // Karate logs nothing when the connection fails, so the failed `method` step is the only trace.
    if (logged.length === 0 && r.result.status === "failed" && r.step.keyword === "method") {
      out.push({
        method: r.step.text.trim().toUpperCase(),
        url: null,
        requestHeaders: [],
        requestBody: null,
        status: null,
        responseHeaders: [],
        responseBody: null,
        millis: null,
        error,
        via,
        startTime,
        attempt: null,
      });
    }
    for (const called of r.callResults ?? [])
      for (const sc of called.scenarioResults ?? [])
        collect(sc.stepResults ?? [], called.relativePath, out);
  }
}

/** HTTP calls of every scenario in a depth-0 FEATURE_EXIT, by Karate refId, in start-time order. Calls of called features count for the test whose step made them. */
export function extractCalls(
  featureExitData: Record<string, any>,
): { refId: string; calls: HttpCall[] }[] {
  return (featureExitData.scenarioResults ?? []).map((sc: Record<string, any>) => {
    const calls: HttpCall[] = [];
    collect(sc.stepResults ?? [], null, calls);
    return { refId: sc.refId, calls: calls.sort((a, b) => a.startTime - b.startTime) };
  });
}
