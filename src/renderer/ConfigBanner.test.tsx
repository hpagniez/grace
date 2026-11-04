import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent, ScenarioUpdate } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";

const FIRST = "s:tests/shared.feature#1";
const SECOND = "s:tests/shared.feature#2";
const REASON =
  "Config evaluation failed: karate-config.js - js failed:\n==========\n  Line: 4, Col: 2\n  Code: karate.callSingle('classpath:auth.feature', { baseUrl: baseUrl });\n  Error: callSingle failed: classpath:auth.feature - expected status: 200, actual: 201\n==========";

const leaf = (key: string, name: string, line: number): TestNode => ({
  key,
  kind: "scenario",
  name,
  file: "tests/shared.feature",
  line,
  refId: `[${line}]`,
  tags: [],
  children: [],
});
const tree: TestNode = {
  key: "d:",
  kind: "folder",
  name: "p",
  file: "",
  line: null,
  refId: null,
  tags: [],
  children: [
    {
      key: "f:tests/shared.feature",
      kind: "feature",
      name: "shared",
      file: "tests/shared.feature",
      line: null,
      refId: null,
      tags: [],
      children: [leaf(FIRST, "first", 7), leaf(SECOND, "second", 10)],
    },
  ],
};

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    steps: vi.fn(async () => null),
    openInEditor: vi.fn(async () => {}),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  send({ type: "project", project: { root: "D:/p", layout: "standalone" }, tree, error: null });
  let runs = 0;
  const start = (keys: string[]) =>
    send({
      type: "runStarted",
      selection: "d:",
      runId: `20261104-15000${runs++}-000`,
      keys,
      total: keys.length,
      startedAt: Date.now(),
      env: null,
      debug: false,
    });
  /** A run of `updates`' keys that ends. */
  const run = (updates: ScenarioUpdate[]) => {
    start(updates.map((u) => u.key));
    send({ type: "scenarios", updates });
    send({
      type: "runFinished",
      outcome: "finished",
      exitCode: 1,
      summary: null,
      reportAvailable: false,
    });
  };
  const configFailed = (key: string, reason = REASON): ScenarioUpdate => ({
    key,
    status: "failed",
    error: reason,
    reasonAt: 0,
  });
  return { api, send, start, run, configFailed };
}

const banner = () => screen.queryByTestId("config-failure");

afterEach(cleanup);

describe("config banner (#229 R6)", () => {
  it("names the file and its line", () => {
    const { api, run, configFailed } = setup();
    run([configFailed(FIRST), configFailed(SECOND)]);
    expect(screen.getByRole("alert")).toBe(banner());
    expect(banner()!.textContent).toContain("karate-config.js failed for 2 tests");
    expect(screen.getByTestId("config-failure-reason").textContent).toBe(REASON);
    fireEvent.click(screen.getByRole("button", { name: "Open karate-config.js in VS Code" }));
    expect(api.openInEditor).toHaveBeenCalledWith("karate-config.js", 4);
  });

  it("opens line 1 when the message has none", () => {
    const { api, run, configFailed } = setup();
    run([configFailed(FIRST, "Config evaluation failed: classpath:karate-config-dev.js - boom")]);
    expect(banner()!.textContent).toContain("karate-config-dev.js failed for 1 test");
    fireEvent.click(screen.getByRole("button", { name: "Open karate-config-dev.js in VS Code" }));
    expect(api.openInEditor).toHaveBeenCalledWith("karate-config-dev.js", 1);
  });

  it("goes when the next run starts", () => {
    const { run, start, configFailed } = setup();
    run([configFailed(FIRST), configFailed(SECOND)]);
    expect(banner()).not.toBeNull();
    start([FIRST]);
    expect(banner()).toBeNull();
  });

  it("counts only the last run's failures", () => {
    const { run, configFailed } = setup();
    run([configFailed(FIRST), configFailed(SECOND)]);
    run([configFailed(FIRST)]);
    expect(banner()!.textContent).toContain("karate-config.js failed for 1 test");
  });

  it("shows nothing for ordinary failures", () => {
    const { run } = setup();
    run([
      {
        key: FIRST,
        status: "failed",
        error: "C:/p/x.feature:7\nThen status 200\nboom",
        reasonAt: 33,
      },
    ]);
    expect(banner()).toBeNull();
  });

  it("sits under the notice", () => {
    const { send, run, configFailed } = setup();
    run([configFailed(FIRST), configFailed(SECOND)]);
    send({ type: "notice", message: "Something to say" });
    const notice = screen.getByTestId("notice");
    expect(
      notice.compareDocumentPosition(banner()!) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });
});
