import { describe, expect, it } from "vitest";
import type { AppEvent } from "../shared/events";
import { makeMasker, maskEvent, secretForms } from "./mask";

describe("secretForms", () => {
  it("lists the value, its URL-encoded form and its JSON-escaped form", () => {
    expect(secretForms('a b"c')).toEqual(['a b"c', "a%20b%22c", 'a b\\"c']);
  });

  it("drops forms equal to one already listed", () => {
    expect(secretForms("token")).toEqual(["token"]);
  });

  it("has no forms for an empty value", () => {
    expect(secretForms("")).toEqual([]);
  });
});

describe("makeMasker", () => {
  it("masks every occurrence", () => {
    const mask = makeMasker(["s3cret"]);
    expect(mask("a s3cret b s3cret\nx s3cret")).toBe("a **** b ****\nx ****");
  });

  it("masks URL-encoded and JSON-escaped forms", () => {
    const mask = makeMasker(['a b"c']);
    expect(mask('plain a b"c, url a%20b%22c, json a b\\"c')).toBe(
      "plain ****, url ****, json ****",
    );
  });

  it("masks values with regex metacharacters literally", () => {
    const mask = makeMasker(["$&", ".*", "a\\b"]);
    expect(mask("pre $& mid .* post a\\b json a\\\\b end abc")).toBe(
      "pre **** mid **** post **** json **** end abc",
    );
  });

  it("masks the longest form first", () => {
    expect(makeMasker(["abc", "abcdef"])("xabcdefx")).toBe("x****x");
  });

  it("changes nothing without values", () => {
    expect(makeMasker([])("some s3cret text")).toBe("some s3cret text");
  });

  it("ignores empty values", () => {
    const mask = makeMasker(["", "s3cret"]);
    expect(mask("a s3cret b")).toBe("a **** b");
    expect(makeMasker([""])("a b")).toBe("a b");
  });
});

describe("maskEvent", () => {
  const S = "s3cret";
  const mask = makeMasker([S]);

  it("masks the run text of every event", () => {
    const events: [AppEvent, AppEvent][] = [
      [
        { type: "console", lines: [`token ${S}`, "plain"] },
        { type: "console", lines: ["token ****", "plain"] },
      ],
      [
        { type: "scenarios", updates: [{ key: "s:a#1", status: "failed", error: `got ${S}` }] },
        { type: "scenarios", updates: [{ key: "s:a#1", status: "failed", error: "got ****" }] },
      ],
      [
        {
          type: "steps",
          key: "s:a#1",
          steps: [
            {
              keyword: "*",
              text: `def t = '${S}'`,
              line: 3,
              status: "passed",
              millis: 1,
              background: false,
              log: `t = ${S}`,
            },
          ],
        },
        {
          type: "steps",
          key: "s:a#1",
          steps: [
            {
              keyword: "*",
              text: "def t = '****'",
              line: 3,
              status: "passed",
              millis: 1,
              background: false,
              log: "t = ****",
            },
          ],
        },
      ],
      [
        { type: "notice", message: `bad line ${S}` },
        { type: "notice", message: "bad line ****" },
      ],
    ];
    for (const [event, expected] of events) expect(maskEvent(event, mask)).toEqual(expected);
  });

  it("masks a calls event", () => {
    const call = {
      method: "GET",
      url: `http://h/x?t=${S}`,
      requestHeaders: [{ name: "Authorization", value: `Bearer ${S}` }],
      requestBody: null,
      status: 200,
      responseHeaders: [],
      responseBody: null,
      millis: 3,
      error: null,
      via: null,
      startTime: 1,
      attempt: null,
    };
    expect(maskEvent({ type: "calls", key: "s:a#1", calls: [call] }, mask)).toEqual({
      type: "calls",
      key: "s:a#1",
      calls: [
        {
          ...call,
          url: "http://h/x?t=****",
          requestHeaders: [{ name: "Authorization", value: "Bearer ****" }],
        },
      ],
    });
  });

  it("masks a field it does not know, nested too", () => {
    const event = {
      type: "runFinished",
      outcome: "failed",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
      extra: `x ${S} y`,
      nested: { list: [{ text: S }, 2, true, null] },
    } as unknown as AppEvent;
    expect(maskEvent(event, mask)).toEqual({
      type: "runFinished",
      outcome: "failed",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
      extra: "x **** y",
      nested: { list: [{ text: "****" }, 2, true, null] },
    });
  });

  it("leaves the fields the window matches on exact", () => {
    const m = makeMasker(["a", "2026"]);
    const started: AppEvent = {
      type: "runStarted",
      selection: "s:a#1",
      runId: "20261025-143212-007",
      keys: ["s:a#1", "s:a#2"],
      total: 2,
      startedAt: 5,
      env: "qa",
    };
    expect(maskEvent(started, m)).toEqual(started);
    const updates: AppEvent = { type: "scenarios", updates: [{ key: "s:a#1", status: "passed" }] };
    expect(maskEvent(updates, m)).toEqual(updates);
    const finished = {
      type: "runFinished",
      outcome: "cancelled",
      exitCode: 1,
      summary: { scenarioCount: 2, scenariosPassed: 1, scenariosFailed: 1, durationMillis: 9 },
      reportAvailable: false,
    } as const;
    expect(maskEvent(finished, m)).toEqual(finished);
  });
});
