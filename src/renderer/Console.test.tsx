import { render, screen, cleanup } from "@solidjs/testing-library";
import { createSignal, flush, untrack } from "solid-js";
import { afterEach, describe, expect, it } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import { Console } from "./Console";
import { CONSOLE_LIMIT } from "./reduce";
import { createAppStore } from "./store";

afterEach(cleanup);

function setup() {
  const [lines, setLines] = createSignal<string[]>([]);
  render(() => <Console lines={lines()} />);
  const el = screen.getByTestId("console");
  // jsdom has no layout: fake a 10px-per-line scroller 100px tall
  Object.defineProperty(el, "scrollHeight", { get: () => untrack(() => lines().length * 10) });
  Object.defineProperty(el, "clientHeight", { value: 100 });
  const add = (...more: string[]) => {
    setLines((l) => [...l, ...more]);
    flush();
  };
  return { el, add };
}

describe("Console", () => {
  it("keeps following once the store holds the line cap", () => {
    let emit: (e: AppEvent) => void = () => {};
    const api = {
      ready: async () => {},
      onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
    } as unknown as Api;
    const store = createAppStore(api);
    render(() => <Console lines={store.state.console} />);
    const el = screen.getByTestId("console");
    // scrollHeight grows with the newest line number, not with the array length
    Object.defineProperty(el, "scrollHeight", {
      get: () => untrack(() => (Number(store.state.console.at(-1)?.slice(1) ?? -1) + 1) * 10),
    });
    Object.defineProperty(el, "clientHeight", { value: 100 });
    let n = 0;
    const append = (count: number) => {
      emit({ type: "console", lines: Array.from({ length: count }, () => `l${n++}`) });
      flush();
    };
    append(CONSOLE_LIMIT);
    append(10);
    expect(store.state.console).toHaveLength(CONSOLE_LIMIT);
    expect(el.scrollTop).toBe((CONSOLE_LIMIT + 10) * 10);
    append(5);
    expect(el.scrollTop).toBe((CONSOLE_LIMIT + 15) * 10);
  });

  it("keeps its children above the lines past the line cap", () => {
    let emit: (e: AppEvent) => void = () => {};
    const api = {
      ready: async () => {},
      onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
    } as unknown as Api;
    const store = createAppStore(api);
    render(() => (
      <Console lines={store.state.console}>
        <p data-testid="head">h</p>
      </Console>
    ));
    const el = screen.getByTestId("console");
    Object.defineProperty(el, "scrollHeight", {
      get: () => untrack(() => (store.state.console.length + 1) * 10),
    });
    Object.defineProperty(el, "clientHeight", { value: 100 });
    emit({ type: "console", lines: Array.from({ length: CONSOLE_LIMIT + 10 }, (_, i) => `l${i}`) });
    flush();
    expect(store.state.console).toHaveLength(CONSOLE_LIMIT);
    expect(el.firstElementChild).toBe(screen.getByTestId("head"));
    expect(el.scrollTop).toBe((CONSOLE_LIMIT + 1) * 10);
  });

  it("styles ERROR and WARN lines", () => {
    const { add } = setup();
    add("plain", "12:00 [main] ERROR boom", "12:00 [main] WARN careful");
    expect(screen.getByText("plain").className).not.toMatch(/failed|cancelled/);
    expect(screen.getByText(/ERROR boom/).className).toContain("text-failed");
    expect(screen.getByText(/WARN careful/).className).toContain("text-cancelled");
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
