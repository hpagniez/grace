import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { JsonlTail, readAllEvents } from "./events";

const fixture = (name: string) => path.resolve("test/fixtures/events", name);
const line = (type: string) => JSON.stringify({ type, timeStamp: 1, threadId: null, data: {} });

describe("JsonlTail", () => {
  let dir: string;
  let file: string;
  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "jsonl-tail-"));
    file = path.join(dir, "events.jsonl");
  });
  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("returns [] before the file exists", () => {
    expect(new JsonlTail(file).read()).toEqual([]);
    expect(readAllEvents(file)).toEqual([]);
  });

  it("returns only complete lines and keeps a partial last line", () => {
    const tail = new JsonlTail(file);
    const half = line("SCENARIO_ENTER");
    fs.writeFileSync(
      file,
      `${line("SUITE_ENTER")}\n${line("FEATURE_ENTER")}\n${half.slice(0, 20)}`,
    );
    expect(tail.read().map((e) => e.type)).toEqual(["SUITE_ENTER", "FEATURE_ENTER"]);
    fs.appendFileSync(file, `${half.slice(20)}\n`);
    expect(tail.read().map((e) => e.type)).toEqual(["SCENARIO_ENTER"]);
    expect(tail.read()).toEqual([]);
  });

  it("does not decode a split multi-byte character early", () => {
    const tail = new JsonlTail(file);
    const bytes = Buffer.from(
      `${JSON.stringify({ type: "X", timeStamp: 1, threadId: null, data: { name: "équipe" } })}\n`,
    );
    const cut = bytes.indexOf(0xc3) + 1; // between the two bytes of "é"
    fs.writeFileSync(file, bytes.subarray(0, cut));
    expect(tail.read()).toEqual([]);
    fs.appendFileSync(file, bytes.subarray(cut));
    expect(tail.read()[0].data.name).toBe("équipe");
  });

  it("skips blank lines and names file and line on bad JSON", () => {
    fs.writeFileSync(file, `${line("A")}\n\n{oops\n`);
    expect(() => readAllEvents(file)).toThrow(`${file}:3`);
    fs.writeFileSync(file, `${line("A")}\n\r\n${line("B")}\r\n`);
    expect(readAllEvents(file).map((e) => e.type)).toEqual(["A", "B"]);
  });

  it("does not quote the bad line: it can hold part of a secret", () => {
    fs.writeFileSync(file, `{"data": s3cret-token-123}\n`); // V8 says `"{"data": s3cret-tok"...`
    expect(() => readAllEvents(file)).toThrow(new Error(`${file}:1: invalid JSON event`));
  });
});

describe("readAllEvents", () => {
  it("parses a real event stream", () => {
    const events = readAllEvents(fixture("run-clean.jsonl"));
    expect(events.filter((e) => e.type === "SUITE_ENTER")).toHaveLength(1);
    expect(events.filter((e) => e.type === "SUITE_EXIT")).toHaveLength(1);
    expect(events.filter((e) => e.type === "SCENARIO_EXIT" && e.data.callDepth === 0)).toHaveLength(
      9,
    );
  });

  it("handles a file that ends without EXIT events", () => {
    const events = readAllEvents(fixture("4-after-taskkill-complete-file.jsonl"));
    expect(events.at(-1)?.type).toBe("SCENARIO_ENTER");
  });
});
