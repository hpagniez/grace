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
 * Requests and responses are read in order, one after the other, as Karate writes them; the number `N`
 * only tells a call's header lines from its body.
 */
export function parseStepLog(log: string): LoggedCall[] {
  const lines = log.split(/\r?\n/);
  if (lines.at(-1) === "") lines.pop(); // the newline that ends the last line
  let end = lines.length;
  if (end >= 2 && SUMMARY_STATUS.test(lines[end - 1]) && SUMMARY_REQUEST.test(lines[end - 2]))
    end -= 2;
  // A `request:` line inside a body is body text: it starts a call only before a `N > ` line.
  const startsCall = (i: number) =>
    lines[i] === "request:" && i + 1 < end && /^\d+ > /.test(lines[i + 1]);

  const calls: LoggedCall[] = [];
  for (let i = 0; i < end;) {
    if (!startsCall(i)) {
      i++;
      continue;
    }
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
    const request: string[] = [];
    while (i < end && !startsCall(i) && !RESPONSE_TIME.test(lines[i])) request.push(lines[i++]);
    if (request.at(-1) === "") request.pop();
    call.requestBody = bodyOf(request);

    const time = i < end ? RESPONSE_TIME.exec(lines[i]) : null;
    const status = time && i + 1 < end ? RESPONSE_LINE.exec(lines[i + 1]) : null;
    if (status) {
      call.millis = Number(time![1]);
      call.status = Number(status[2]);
      i += 2;
      for (let h; i < end && (h = header(lines[i], `${status[1]} < `)); i++)
        call.responseHeaders.push(h);
      const response: string[] = [];
      while (i < end && !startsCall(i)) response.push(lines[i++]);
      while (response.at(-1) === "") response.pop();
      call.responseBody = bodyOf(response);
    } else if (i < end && startsCall(i)) {
      continue; // no response, then another request: a followed redirect, not a call
    }
    calls.push(call);
  }
  if (calls.length > 1) calls.forEach((c, k) => (c.attempt = { n: k + 1, of: calls.length }));
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
