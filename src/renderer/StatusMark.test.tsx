import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { MARK, StatusMark, type MarkStatus } from "./StatusMark";
import { STATUS_LABEL } from "./nodes";

afterEach(cleanup);

const KEYS = Object.keys(MARK) as MarkStatus[];
const drawn = (status: MarkStatus) => {
  const { container } = render(() => <StatusMark status={status} />);
  const mark = container.querySelector<HTMLElement>("[role=img]")!;
  cleanup();
  return mark;
};

describe("StatusMark", () => {
  it("covers every status plus skipped and cancelled", () => {
    expect(KEYS.sort()).toEqual(
      [...Object.keys(STATUS_LABEL), "skipped", "cancelled"].sort() as MarkStatus[],
    );
  });

  it.each(KEYS)("%s is an image with its word as the name", (status) => {
    render(() => <StatusMark status={status} />);
    const mark = screen.getByRole("img");
    expect(mark.getAttribute("aria-label")).toBe(MARK[status].label);
    expect(mark.getAttribute("data-status")).toBe(status);
    expect(mark.querySelector("svg")!.getAttribute("aria-hidden")).toBe("true");
  });

  it("names the words", () => {
    expect(KEYS.map((k) => MARK[k].label).sort()).toEqual(
      [...Object.values(STATUS_LABEL), "Skipped", "Cancelled"].sort(),
    );
    expect(MARK.passed.label).toBe("Passed");
    expect(MARK.notRun.label).toBe("Not run");
    expect(MARK.interrupted.label).toBe("Interrupted");
  });

  it("draws a shape of its own, except interrupted and cancelled which share one", () => {
    const shape = (k: MarkStatus) => drawn(k).querySelector("svg")!.innerHTML;
    for (const a of KEYS)
      for (const b of KEYS) {
        if (a >= b) continue;
        const shared = [a, b].sort().join() === "cancelled,interrupted";
        expect(shape(a) === shape(b), `${a} vs ${b}`).toBe(shared);
      }
  });

  it("colours by status", () => {
    const text = (k: MarkStatus) => drawn(k).className;
    expect(text("passed")).toContain("text-passed");
    expect(text("failed")).toContain("text-failed");
    expect(text("running")).toContain("text-running");
    expect(text("interrupted")).toContain("text-cancelled");
    expect(text("cancelled")).toContain("text-cancelled");
    for (const k of ["notRun", "pending", "skipped"] as const)
      expect(text(k)).toContain("text-skipped");
  });

  it("turns only while running", () => {
    for (const k of KEYS)
      expect(drawn(k).querySelector(".animate-spin") !== null, k).toBe(k === "running");
  });

  it("is 14 px unless asked for 16", () => {
    const { container } = render(() => <StatusMark status="passed" size={16} />);
    expect(container.querySelector("svg")!.getAttribute("width")).toBe("16");
    expect(drawn("passed").querySelector("svg")!.getAttribute("width")).toBe("14");
  });
});
