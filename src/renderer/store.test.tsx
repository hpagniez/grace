import { flush } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { CONSOLE_LIMIT } from "./reduce";
import { createAppStore } from "./store";

describe("createAppStore", () => {
  it("subscribes before ready, applies events, selects, disposes", () => {
    const calls: string[] = [];
    let cb: (e: AppEvent) => void = () => {};
    const off = vi.fn();
    const api = {
      ready: async () => void calls.push("ready"),
      onEvent: (f: (e: AppEvent) => void) => {
        calls.push("onEvent");
        cb = f;
        return off;
      },
    } as unknown as Api;
    const store = createAppStore(api);
    expect(calls).toEqual(["onEvent", "ready"]);
    cb({ type: "notice", message: "hi" });
    flush();
    expect(store.state.notice).toBe("hi");
    cb({ type: "project", project: null, tree: null, error: "boom" });
    flush();
    expect(store.state.notice).toBeNull();
    expect(store.state.error).toBe("boom");
    store.select("f:a");
    flush();
    expect(store.state.selected).toBe("f:a");
    store.dispose();
    expect(off).toHaveBeenCalled();
  });

  it("trims console and drops stale keys through a real store draft", () => {
    let cb: (e: AppEvent) => void = () => {};
    const api = {
      ready: async () => {},
      onEvent: (f: (e: AppEvent) => void) => ((cb = f), () => {}),
    } as unknown as Api;
    const store = createAppStore(api);
    const node = (key: string): TestNode => ({
      key,
      kind: "scenario",
      name: key,
      file: "a.feature",
      line: 1,
      refId: null,
      tags: [],
      children: [],
    });
    const tree = (...kids: TestNode[]): TestNode => ({
      ...node("f:a.feature"),
      kind: "feature",
      children: kids,
    });
    const project = { root: "D:/p", layout: "standalone" } as const;
    cb({ type: "project", project, tree: tree(node("s:a#1"), node("s:a#2")), error: null });
    cb({
      type: "runStarted",
      selection: "f:a.feature",
      keys: ["s:a#1", "s:a#2"],
      total: 2,
      startedAt: 1,
    });
    for (const [from, to] of [
      [0, 4000],
      [4000, 8000],
      [8000, 10500],
    ])
      cb({ type: "console", lines: Array.from({ length: to - from }, (_, i) => `l${from + i}`) });
    flush();
    expect(store.state.console).toHaveLength(CONSOLE_LIMIT);
    expect(store.state.console.at(-1)).toBe("l10499");
    expect(Object.keys(store.state.statuses)).toEqual(["s:a#1", "s:a#2"]);
    cb({ type: "project", project, tree: tree(node("s:a#1")), error: null });
    flush();
    expect(Object.keys(store.state.statuses)).toEqual(["s:a#1"]);
  });
});
