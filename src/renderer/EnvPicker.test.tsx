import { render, screen, cleanup, fireEvent } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { EnvView } from "../shared/env";
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
    saveProperties: vi.fn(async () => true),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  // a fresh object each time, as after IPC
  const view = (over: Partial<EnvView> = {}): EnvView => ({
    env: null,
    suggestions: [],
    overlays: [],
    properties: { shared: [], byEnv: {} },
    secretsAvailable: true,
    ...over,
  });
  // main sends `project`, `filter`, then `environment`
  const load = (over: Partial<EnvView> = {}) => {
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: null,
      error: null,
    });
    send({ type: "environment", view: view(over) });
  };
  return { api, send, load, view };
}

const input = () => screen.getByTestId("env-input") as HTMLInputElement;
const type = (value: string) => {
  fireEvent.input(input(), { target: { value } });
  flush();
};
const press = (key: string) => {
  fireEvent.keyDown(input(), { key });
  flush();
};
const enter = () => press("Enter");
const focus = () => {
  fireEvent.focus(input());
  flush();
};
const blur = () => {
  fireEvent.blur(input());
  flush();
};
const options = () =>
  [...document.querySelectorAll(`#${input().getAttribute("list")} option`)].map(
    (o) => (o as HTMLOptionElement).value,
  );

afterEach(cleanup);

describe("environment combo", () => {
  it("suggestions and none", () => {
    const { load } = setup();
    load({ suggestions: ["prod", "qa"] });
    // no environment is an empty box: Chromium narrows the list to options matching the text in it
    expect(input().value).toBe("");
    expect(input().placeholder).toBe("(none)");
    expect(options()).toEqual(["(none)", "prod", "qa"]);
    expect(screen.getByRole("combobox", { name: "Env" })).toBe(input());
    expect(screen.queryByTestId("env-hint")).toBeNull();
    expect(screen.queryByTestId("env-error")).toBeNull();
  });

  it("is empty and disabled with no project", () => {
    const { send, load, view } = setup();
    expect(input().value).toBe("");
    expect(input().placeholder).toBe("");
    expect(input().disabled).toBe(true);
    // main can still hold the previous project's view after "Folder not found"
    send({ type: "environment", view: view({ env: "qa" }) });
    expect(input().value).toBe("");
    load({ env: "qa" });
    expect(input().value).toBe("qa");
    expect(input().disabled).toBe(false);
    send({ type: "project", project: null, tree: null, error: "Folder not found" });
    expect(input().value).toBe("");
    expect(input().disabled).toBe(true);
  });

  it("Enter commits, typing alone does not", () => {
    const { api, load } = setup();
    load({ suggestions: ["qa"] });
    type("qa");
    expect(api.setEnv).not.toHaveBeenCalled();
    enter();
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith("qa");
    // the window waits for main's echo
    expect(input().value).toBe("qa");
  });

  it("(none) and an empty box mean no environment", () => {
    const { api, send, load, view } = setup();
    load({ env: "qa" });
    type("(none)");
    enter();
    expect(api.setEnv).toHaveBeenLastCalledWith(null);
    send({ type: "environment", view: view({ env: "qa" }) }); // refused: main re-sends qa
    type("");
    enter();
    expect(api.setEnv).toHaveBeenCalledTimes(2);
    expect(api.setEnv).toHaveBeenLastCalledWith(null);
  });

  it("blur commits, a pick commits at once", () => {
    const { api, load } = setup();
    load({ suggestions: ["prod", "qa"] });
    type("prod");
    fireEvent.blur(input());
    expect(api.setEnv).toHaveBeenLastCalledWith("prod");
    fireEvent.input(input(), { target: { value: "qa" }, inputType: "insertReplacementText" });
    expect(api.setEnv).toHaveBeenLastCalledWith("qa");
    expect(api.setEnv).toHaveBeenCalledTimes(2);
  });

  it("an Electron pick commits at once", () => {
    const { api, load } = setup();
    load({ suggestions: ["prod", "qa"] });
    // Electron 45 sends a plain input Event for a datalist pick, not an InputEvent
    input().value = "prod";
    fireEvent(input(), new Event("input", { bubbles: true }));
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith("prod");
  });

  it("picking (none) from the list clears the environment", () => {
    const { api, load } = setup();
    load({ env: "qa", suggestions: ["qa"] });
    expect(input().value).toBe("qa");
    fireEvent.input(input(), { target: { value: "(none)" }, inputType: "insertReplacementText" });
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith(null);
  });

  it("does not call main when nothing changed", () => {
    const { api, load } = setup();
    load({ env: "qa" });
    enter();
    type("qa ");
    fireEvent.blur(input());
    type("(none)");
    type("qa");
    enter();
    expect(api.setEnv).not.toHaveBeenCalled();
  });

  it("puts the environment back when nothing is sent", () => {
    const { send, load, view } = setup();
    load();
    type("(none)");
    enter();
    expect(input().value).toBe("");
    send({ type: "environment", view: view({ env: "qa" }) });
    type(" qa");
    fireEvent.blur(input());
    flush();
    expect(input().value).toBe("qa");
    fireEvent.input(input(), { target: { value: "qa" }, inputType: "insertReplacementText" });
    expect(input().value).toBe("qa");
  });

  it("focus empties the box so the list offers every name", () => {
    const { api, load } = setup();
    load({ env: "qa", suggestions: ["prod", "qa"] });
    focus();
    // Chromium narrows the list to the options that match the text in the box
    expect(input().value).toBe("");
    expect(input().placeholder).toBe("qa");
    // leaving, Enter or Escape without typing keeps the environment
    blur();
    expect(input().value).toBe("qa");
    focus();
    enter();
    expect(input().value).toBe("qa");
    focus();
    press("Escape");
    expect(input().value).toBe("qa");
    expect(api.setEnv).not.toHaveBeenCalled();
    focus();
    fireEvent.input(input(), { target: { value: "prod" }, inputType: "insertReplacementText" });
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith("prod");
  });

  it("after focus, a cleared box means none and Escape drops the typing", () => {
    const { api, load } = setup();
    load({ env: "qa" });
    focus();
    type("pr");
    type("");
    blur();
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith(null);
    focus();
    type("prod");
    press("Escape");
    expect(input().value).toBe("qa");
    blur();
    expect(api.setEnv).toHaveBeenCalledOnce();
  });

  it("invalid name shows error and does not call setEnv", () => {
    const { api, load } = setup();
    load();
    type("not valid!");
    enter();
    fireEvent.blur(input());
    expect(api.setEnv).not.toHaveBeenCalled();
    expect(screen.getByTestId("env-error").textContent).toMatch(/letters, digits/);
    expect(input().getAttribute("aria-invalid")).toBe("true");
    // typing again clears it
    type("qa");
    expect(screen.queryByTestId("env-error")).toBeNull();
  });

  it("resets to main's view whenever a new one arrives", () => {
    const { send, load, view } = setup();
    load({ env: "qa" });
    type("stag");
    expect(input().value).toBe("stag");
    // a refused change: main re-sends the environment in force
    send({ type: "environment", view: view({ env: "qa" }) });
    expect(input().value).toBe("qa");
    type("not valid!");
    enter();
    expect(screen.getByTestId("env-error")).toBeTruthy();
    send({ type: "environment", view: view({ env: null }) });
    expect(input().value).toBe("");
    expect(screen.queryByTestId("env-error")).toBeNull();
  });

  it("hint for a name without overlay", () => {
    const { send, load, view } = setup();
    load({ env: "staging", overlays: ["qa"] });
    expect(screen.getByTestId("env-hint").textContent).toBe(
      "No karate-config-staging.js found. Fine if karate-config.js handles it.",
    );
    expect(input().getAttribute("aria-describedby")).toBe(screen.getByTestId("env-hint").id);
    send({ type: "environment", view: view({ env: "qa", overlays: ["qa"] }) });
    expect(screen.queryByTestId("env-hint")).toBeNull();
    // Windows finds karate-config-QA.js for -e qa
    send({ type: "environment", view: view({ env: "qa", overlays: ["QA"] }) });
    expect(screen.queryByTestId("env-hint")).toBeNull();
    send({ type: "environment", view: view({ env: null, overlays: [] }) });
    expect(screen.queryByTestId("env-hint")).toBeNull();
    // typing alone does not change the hint: the chosen environment does
    type("other");
    expect(screen.queryByTestId("env-hint")).toBeNull();
  });

  it("sends one change: Electron blurs the box it disables", () => {
    const { api, send, load } = setup();
    load({ env: "qa", suggestions: ["prod", "qa"] });
    type("prod");
    enter();
    send({ type: "busy", busy: true });
    fireEvent.blur(input());
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith("prod");
    // nothing reaches main while it is busy
    type("staging");
    enter();
    fireEvent.input(input(), { target: { value: "qa" }, inputType: "insertReplacementText" });
    fireEvent.blur(input());
    expect(api.setEnv).toHaveBeenCalledOnce();
  });

  it("sends a change once until main's view comes back", () => {
    const { api, send, load, view } = setup();
    load({ suggestions: ["prod", "qa"] });
    // an Electron pick, then the Enter or blur that follows before main's busy echo
    const pick = (value: string) => {
      input().value = value;
      fireEvent(input(), new Event("input", { bubbles: true }));
      flush();
    };
    pick("prod");
    enter();
    expect(api.setEnv).toHaveBeenCalledExactlyOnceWith("prod");
    blur();
    expect(api.setEnv).toHaveBeenCalledOnce();
    // a refusal re-sends the environment in force: the same name can be sent again
    send({ type: "environment", view: view({ suggestions: ["prod", "qa"] }) });
    pick("prod");
    blur();
    expect(api.setEnv).toHaveBeenCalledTimes(2);
    send({ type: "environment", view: view({ env: "prod", suggestions: ["prod", "qa"] }) });
    type("qa");
    enter();
    enter();
    expect(api.setEnv).toHaveBeenCalledTimes(3);
    expect(api.setEnv).toHaveBeenLastCalledWith("qa");
    // the text typed is not put back while the change is on its way
    expect(input().value).toBe("qa");
  });

  it("disabled while busy", () => {
    const { send, load } = setup();
    load({ env: "qa" });
    expect(input().disabled).toBe(false);
    send({ type: "busy", busy: true });
    expect(input().disabled).toBe(true);
    send({ type: "busy", busy: false });
    expect(input().disabled).toBe(false);
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: [],
      total: 1,
      startedAt: Date.now(),
      env: "qa",
    });
    expect(input().disabled).toBe(true);
  });
});

describe("progress", () => {
  it("run env shown", () => {
    const { send, load, view } = setup();
    load({ env: "qa" });
    const runEnv = () => screen.getByTestId("run-env").textContent;
    expect(runEnv()).toBe("");
    const start = (env: string | null) =>
      send({
        type: "runStarted",
        selection: "d:",
        runId: "20261025-120000-000",
        keys: [],
        total: 1,
        startedAt: 1,
        env,
      });
    start("qa");
    expect(runEnv()).toBe("qa");
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 0,
      summary: null,
      reportAvailable: true,
    });
    expect(runEnv()).toBe("qa"); // the last run's
    start(null);
    expect(runEnv()).toBe("");
    start("prod");
    send({ type: "environment", view: view({ env: "dev" }) });
    expect(runEnv()).toBe(""); // another environment: the old run is gone
  });
});
