import path from "node:path";
import { describe, expect, it } from "vitest";
import { readAllEvents } from "./events";
import { extractSteps } from "./steps";

const basic = () => {
  const ev = readAllEvents(path.resolve("test/fixtures/events/run-clean.jsonl")).find(
    (e) =>
      e.type === "FEATURE_EXIT" &&
      e.data.callDepth === 0 &&
      e.data.relativePath === "tests/basic.feature",
  )!;
  return extractSteps(ev.data);
};

describe("extractSteps", () => {
  it("returns one entry per scenario, by refId", () => {
    expect(basic().map((s) => s.refId)).toEqual(["[1:7]", "[2:15]", "[3:22]", "[4:28]"]);
  });

  it("maps a failed step", () => {
    const step = basic()[1].steps.find((s) => s.status === "failed")!;
    expect(step.keyword).toBe("And");
    expect(step.text).toMatch(/^match response\.message/);
    expect(step.line).toBe(20);
  });

  it("keeps the log of a step and flags background steps", () => {
    const steps = basic()[0].steps;
    const get = steps.find((s) => s.text === "method get")!;
    expect(get.log).toContain("GET");
    expect(get.log).toContain("200");
    expect(steps[0].background).toBe(true);
    expect(steps[1].background).toBe(false);
    expect(steps[1].log).toBeNull();
  });
});
