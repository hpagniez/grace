import { flush } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
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
});
