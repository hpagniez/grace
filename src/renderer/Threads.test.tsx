import { render, screen, cleanup, fireEvent } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import { App } from "./App";

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    openReport: vi.fn(async () => {}),
    setFilter: vi.fn(async () => {}),
    setEnv: vi.fn(async () => {}),
    setThreads: vi.fn(async () => {}),
    saveProperties: vi.fn(async () => true),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  const open = (root: string) =>
    send({ type: "project", project: { root, layout: "standalone" }, tree: null, error: null });
  // main sends `project`, then `threads`
  const load = (count: number) => {
    open("D:/p");
    send({ type: "threads", count });
  };
  return { api, send, open, load };
}

const box = () => screen.getByTestId("threads-input") as HTMLInputElement;
const type = (value: string) => {
  fireEvent.input(box(), { target: { value } });
  flush();
};
const commit = () => {
  fireEvent.change(box());
  flush();
};

afterEach(cleanup);

describe("threads box", () => {
  it("shows main's count and is disabled with no project", () => {
    const { load } = setup();
    expect(box().disabled).toBe(true);
    expect(box().value).toBe("1");
    load(3);
    expect(box().disabled).toBe(false);
    expect(box().value).toBe("3");
  });

  it("sends a committed value once", () => {
    const { api, load } = setup();
    load(1);
    type("16");
    expect(api.setThreads).not.toHaveBeenCalled();
    commit();
    expect(api.setThreads).toHaveBeenCalledExactlyOnceWith(16);
  });

  it("a refused value resets the box", () => {
    const { api, send, load } = setup();
    load(2);
    type("0");
    commit();
    expect(api.setThreads).toHaveBeenCalledExactlyOnceWith(0);
    expect(box().value).toBe("0");
    // a refusal: main re-sends the count in force
    send({ type: "threads", count: 2 });
    expect(box().value).toBe("2");
  });

  it("sends NaN for an empty box and leaves the check to main", () => {
    const { api, load } = setup();
    load(2);
    type("");
    commit();
    expect(api.setThreads).toHaveBeenCalledExactlyOnceWith(NaN);
  });

  it("disabled while busy", () => {
    const { send, load } = setup();
    load(2);
    expect(box().disabled).toBe(false);
    send({ type: "busy", busy: true });
    expect(box().disabled).toBe(true);
    send({ type: "busy", busy: false });
    expect(box().disabled).toBe(false);
  });

  it("a new project shows its own count", () => {
    const { send, open, load } = setup();
    load(3);
    open("D:/q");
    expect(box().disabled).toBe(true);
    send({ type: "threads", count: 1 });
    expect(box().value).toBe("1");
    expect(box().disabled).toBe(false);
    // a Refresh sends `project` for the same root: the count stays
    open("D:/q");
    expect(box().disabled).toBe(false);
    expect(box().value).toBe("1");
  });
});
