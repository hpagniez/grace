import { render, screen, cleanup } from "@solidjs/testing-library";
import { createSignal, flush } from "solid-js";
import { afterEach, describe, expect, it } from "vitest";
import { Console } from "./Console";

afterEach(cleanup);

function setup() {
  const [lines, setLines] = createSignal<string[]>([]);
  render(() => <Console lines={lines()} />);
  const el = screen.getByTestId("console");
  // jsdom has no layout: fake a 10px-per-line scroller 100px tall
  Object.defineProperty(el, "scrollHeight", { get: () => lines().length * 10 });
  Object.defineProperty(el, "clientHeight", { value: 100 });
  const add = (...more: string[]) => {
    setLines((l) => [...l, ...more]);
    flush();
  };
  return { el, add };
}

describe("Console", () => {
  it("styles ERROR and WARN lines", () => {
    const { add } = setup();
    add("plain", "12:00 [main] ERROR boom", "12:00 [main] WARN careful");
    expect(screen.getByText("plain").className).not.toMatch(/red|amber/);
    expect(screen.getByText(/ERROR boom/).className).toContain("text-red-400");
    expect(screen.getByText(/WARN careful/).className).toContain("text-amber-400");
  });

  it("follows new lines unless the user scrolled up", () => {
    const { el, add } = setup();
    add(...Array.from({ length: 30 }, (_, i) => `l${i}`));
    expect(el.scrollTop).toBe(300);
    el.scrollTop = 0; // user scrolls up
    el.dispatchEvent(new Event("scroll"));
    add("more");
    expect(el.scrollTop).toBe(0);
    el.scrollTop = 195; // within 20px of the bottom (310 - 100 = 210)
    el.dispatchEvent(new Event("scroll"));
    add("again");
    expect(el.scrollTop).toBe(320);
  });
});
