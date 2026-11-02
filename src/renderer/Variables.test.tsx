import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { createStore, flush } from "solid-js";
import { afterEach, describe, expect, it } from "vitest";
import type { AppEvent, DebugVar } from "../shared/events";
import { initialState, reduce } from "./reduce";
import { Variables } from "./Variables";

afterEach(cleanup);

const VARS: DebugVar[] = [
  { name: "user", kind: "json", value: '{"id":7,"tags":["a"]}' },
  { name: "doc", kind: "xml", value: "<root><item>x</item></root>" },
  { name: "big", kind: "string", value: "x".repeat(10), cut: 36_864 },
  { name: "day", kind: "other", type: "LocalDate", value: "2026-11-02" },
];
const paused: AppEvent = {
  type: "debugPaused",
  paused: {
    key: "s:a#1",
    file: "a.feature",
    line: 2,
    depth: 0,
    prefix: "*",
    keyword: "def",
    text: "x",
    vars: VARS,
  },
};

function setup() {
  const [state, set] = createStore(initialState());
  render(() => <Variables state={state} />);
  const send = (e: AppEvent) => {
    set((draft) => reduce(draft, e));
    flush();
  };
  return { send };
}
const row = (name: string) =>
  screen.getAllByTestId("variable").find((r) => r.dataset.name === name)!;

describe("the Variables tab (step debugger R7.4, R7.5)", () => {
  it("waits for the first pause", () => {
    setup();
    expect(screen.getByTestId("variables").textContent).toContain(
      "Variables show when the run pauses",
    );
  });

  it("lists the variables by name, with their kind or Java type", () => {
    const { send } = setup();
    send(paused);
    expect(screen.getAllByTestId("variable").map((r) => r.dataset.name)).toEqual([
      "big",
      "day",
      "doc",
      "user",
    ]);
    expect(row("day").textContent).toContain("LocalDate");
    expect(row("user").textContent).toContain("json");
    expect(row("big").querySelector("[data-testid=variable-cut]")!.textContent).toBe(
      "36,864 more characters",
    );
    expect(row("big").querySelector("[data-testid=variable-toggle]")).toBeNull();
  });

  it("opens a JSON value indented and an XML value as it is", () => {
    const { send } = setup();
    send(paused);
    fireEvent.click(row("user").querySelector("[data-testid=variable-toggle]")!);
    flush();
    expect(row("user").querySelector("pre")!.textContent).toBe(
      JSON.stringify({ id: 7, tags: ["a"] }, null, 2),
    );
    fireEvent.click(row("doc").querySelector("[data-testid=variable-toggle]")!);
    flush();
    expect(row("doc").querySelector("pre")!.textContent).toBe("<root><item>x</item></root>");
  });

  it("dims the last variables while the run goes on", () => {
    const { send } = setup();
    send(paused);
    expect(screen.getByTestId("variables").className).not.toContain("opacity-60");
    send({ type: "debugResumed" });
    expect(screen.getByTestId("variables").className).toContain("opacity-60");
    expect(screen.getByTestId("variables").textContent).toContain("Running…");
    expect(screen.getAllByTestId("variable")).toHaveLength(4);
  });
});
