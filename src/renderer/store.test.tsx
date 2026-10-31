import { flush } from "solid-js";
import { describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { HttpCall } from "../shared/http";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { NO_FILTER } from "../shared/tags";
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
    expect(store.state.notice?.text).toBe("hi");
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
      runId: "20261025-120000-000",
      keys: ["s:a#1", "s:a#2"],
      total: 2,
      startedAt: 1,
      env: null,
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

  it("dismissPrompt drops the startup modal's release", () => {
    let cb: (e: AppEvent) => void = () => {};
    const api = {
      ready: async () => {},
      onEvent: (f: (e: AppEvent) => void) => ((cb = f), () => {}),
    } as unknown as Api;
    const store = createAppStore(api);
    const release = {
      version: "1.0.0-rc.6",
      prerelease: true,
      publishedAt: "",
      url: "",
      size: 1,
      sha256: "a".repeat(64),
    };
    cb({ type: "updatePrompt", release });
    flush();
    expect(store.state.updatePrompt).toEqual(release);
    store.dismissPrompt();
    flush();
    expect(store.state.updatePrompt).toBeNull();
  });

  describe("tag filter", () => {
    const project = { root: "D:/p", layout: "standalone" } as const;
    const setup = () => {
      let cb: (e: AppEvent) => void = () => {};
      const api = {
        ready: async () => {},
        onEvent: (f: (e: AppEvent) => void) => ((cb = f), () => {}),
      } as unknown as Api;
      const store = createAppStore(api);
      return {
        store,
        send: (e: AppEvent) => {
          cb(e);
          flush();
        },
      };
    };
    const filter = { include: ["smoke"], exclude: ["wip"], hideNonMatching: true };
    const open = (root: string = project.root): AppEvent => ({
      type: "project",
      project: { ...project, root },
      tree: null,
      error: null,
    });

    it("starts with no filter", () => {
      expect(setup().store.state.filter).toEqual(NO_FILTER);
    });

    it("filter event replaces the filter", () => {
      const { store, send } = setup();
      send(open());
      send({ type: "filter", filter });
      expect(store.state.filter).toEqual(filter);
      send({ type: "filter", filter: { include: [], exclude: ["slow"], hideNonMatching: false } });
      expect(store.state.filter).toEqual({
        include: [],
        exclude: ["slow"],
        hideNonMatching: false,
      });
    });

    it("opening another project resets the filter", () => {
      const { store, send } = setup();
      send(open());
      send({ type: "filter", filter });
      send(open()); // a refresh keeps it
      expect(store.state.filter).toEqual(filter);
      send(open("D:/q"));
      expect(store.state.filter).toEqual(NO_FILTER);
      send({ type: "filter", filter });
      send({ type: "project", project: null, tree: null, error: "Folder not found" });
      expect(store.state.filter).toEqual(NO_FILTER);
      // the store holds its own copy, never the shared constant
      expect(store.state.filter).not.toBe(NO_FILTER);
      expect(NO_FILTER).toEqual({ include: [], exclude: [], hideNonMatching: false });
    });
  });

  describe("http calls", () => {
    const aCall = (status: number): HttpCall => ({
      method: "GET",
      url: "http://localhost/x",
      requestHeaders: [],
      requestBody: null,
      status,
      responseHeaders: [],
      responseBody: null,
      millis: null,
      error: null,
      via: null,
      startTime: status,
      attempt: null,
    });
    const node = (key: string, kind: TestNode["kind"], children: TestNode[] = []): TestNode => ({
      key,
      kind,
      name: key,
      file: "a.feature",
      line: 1,
      refId: null,
      tags: [],
      children,
    });
    const setup = () => {
      let cb: (e: AppEvent) => void = () => {};
      const api = {
        ready: async () => {},
        onEvent: (f: (e: AppEvent) => void) => ((cb = f), () => {}),
      } as unknown as Api;
      const store = createAppStore(api);
      const send = (e: AppEvent) => {
        cb(e);
        flush();
      };
      send({
        type: "project",
        project: { root: "D:/p", layout: "standalone" },
        tree: node("f:a.feature", "feature", [
          node("s:a#1", "scenario"),
          node("s:a#2", "scenario"),
        ]),
        error: null,
      });
      send({ type: "calls", key: "s:a#1", calls: [aCall(200), aCall(404)] });
      return store;
    };
    const ok = { key: "s:a#1", index: 0 };
    const notFound = { key: "s:a#1", index: 1 };

    it("selectCall, select and showPanel", () => {
      const store = setup();
      store.selectCall(ok);
      store.showPanel("http");
      flush();
      expect(store.state.selectedCall).toEqual(ok); // the HTTP tab keeps it
      store.showPanel("console");
      flush();
      expect(store.state.panel).toBe("console");
      expect(store.state.selectedCall).toBeNull();
      store.selectCall(ok);
      store.showPanel("history");
      flush();
      expect(store.state.selectedCall).toBeNull();
      store.selectCall(ok);
      store.select("s:a#2"); // a tree click gives the details pane back
      flush();
      expect(store.state.selected).toBe("s:a#2");
      expect(store.state.selectedCall).toBeNull();
    });

    it("toggleGroup drops a call it hides", () => {
      const store = setup();
      store.selectCall(ok);
      store.toggleGroup("all");
      flush();
      expect(store.state.selectedCall).toEqual(ok); // All hides nothing
      store.toggleGroup("4xx");
      flush();
      expect(store.state.httpGroups).toEqual(["4xx"]);
      expect(store.state.selectedCall).toBeNull(); // the 200 is hidden
      store.selectCall(notFound);
      store.toggleGroup("none");
      flush();
      expect(store.state.httpGroups).toEqual(["4xx", "none"]);
      expect(store.state.selectedCall).toEqual(notFound);
      store.toggleGroup("4xx");
      flush();
      expect(store.state.httpGroups).toEqual(["none"]);
      expect(store.state.selectedCall).toBeNull(); // the 404 is hidden
      store.selectCall(notFound);
      store.toggleGroup("all");
      flush();
      expect(store.state.httpGroups).toEqual([]);
      expect(store.state.selectedCall).toEqual(notFound);
    });
  });
});
