import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import { curlCommand, type HttpCall } from "../shared/http";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  delete (navigator as { clipboard?: unknown }).clipboard; // jsdom has none; tests that need one stub it
});

const call = (over: Partial<HttpCall>): HttpCall => ({
  method: "GET",
  url: "http://localhost/users",
  requestHeaders: [],
  requestBody: null,
  status: 200,
  responseHeaders: [],
  responseBody: null,
  millis: null,
  error: null,
  via: null,
  startTime: 0,
  attempt: null,
  ...over,
});

const node = (
  key: string,
  kind: TestNode["kind"],
  name: string,
  line: number,
  children: TestNode[] = [],
): TestNode => ({ key, kind, name, file: "a.feature", line, refId: null, tags: [], children });
const tree = node("d:", "folder", "root", 0, [
  node("f:a.feature", "feature", "users", 1, [
    node("s:a.feature#1", "scenario", "create user", 3),
    node("s:a.feature#2", "scenario", "list users", 9),
  ]),
]);

const created = call({
  method: "POST",
  status: 201,
  millis: 6,
  via: "tests/common/create-user.feature:8",
  requestHeaders: [
    { name: "Content-Type", value: "application/json" },
    { name: "Accept", value: "application/json" },
  ],
  requestBody: '{"name":"bob","tags":[1,2]}',
  responseHeaders: [{ name: "X-Id", value: "7" }],
  responseBody: '{"id":7}',
  startTime: 1,
});
const xml = call({
  method: "POST",
  url: "http://localhost/echo",
  requestBody: "<user><name>bob</name></user>",
  startTime: 2,
});
const timedOut = call({
  url: "http://localhost/slow",
  status: null,
  requestHeaders: [{ name: "Accept", value: "*/*" }],
  error: "Read timed out\n  at java.net.Socket.read",
  startTime: 3,
});
const refused = call({ url: null, status: null, error: "Connection refused", startTime: 4 });

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  send({ type: "project", project: { root: "D:/p", layout: "standalone" }, tree, error: null });
  send({ type: "calls", key: "s:a.feature#1", calls: [created, xml] });
  send({ type: "calls", key: "s:a.feature#2", calls: [timedOut, refused] });
}

const click = (el: HTMLElement) => {
  fireEvent.click(el);
  flush();
};
const selectCall = (i: number) => {
  click(screen.getByTestId("tab-http"));
  click(screen.getAllByTestId("http-call")[i]);
};
const id = (testid: string) => screen.getByTestId(testid);
const textOf = (testid: string) => id(testid).textContent;
const fromTree = (name: string) => click(within(screen.getByTestId("tree")).getByText(name));
// Let the awaited clipboard write settle, then apply what it set.
const settle = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve();
  flush();
};
const stubClipboard = (writeText: (text: string) => Promise<void>) =>
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });

describe("call details", () => {
  it("shows the selected call", () => {
    setup();
    selectCall(0);
    expect(textOf("call-method")).toBe("POST");
    expect(textOf("call-url")).toBe("http://localhost/users");
    expect(textOf("call-status")).toBe("201 · 2xx Success");
    expect(textOf("call-time")).toBe("6 ms");
    expect(textOf("call-location")).toBe("a.feature:3");
    expect(textOf("call-via")).toBe("via tests/common/create-user.feature:8");
    expect(textOf("call-request-headers")).toBe(
      "Content-Type: application/json\nAccept: application/json",
    );
    expect(textOf("call-request-body")).toBe(
      '{\n  "name": "bob",\n  "tags": [\n    1,\n    2\n  ]\n}',
    );
    expect(textOf("call-response-headers")).toBe("X-Id: 7");
    expect(textOf("call-response-body")).toBe('{\n  "id": 7\n}');
    expect(screen.queryByTestId("call-error")).toBeNull();
    // the node's own details give way to the call
    expect(screen.queryByTestId("details-location")).toBeNull();
    expect(textOf("copy-curl")).toBe("Copy as cURL");
  });

  it("indents an XML body and leaves out what the call lacks", () => {
    setup();
    selectCall(1);
    expect(textOf("call-request-body")).toBe("<user>\n  <name>bob</name>\n</user>");
    expect(textOf("call-status")).toBe("200 · 2xx Success");
    expect(screen.queryByTestId("call-time")).toBeNull();
    expect(screen.queryByTestId("call-via")).toBeNull();
    expect(screen.queryByTestId("call-request-headers")).toBeNull();
    expect(screen.queryByTestId("call-response-headers")).toBeNull();
    expect(screen.queryByTestId("call-response-body")).toBeNull();
  });

  it("shows a call with no response", () => {
    setup();
    selectCall(2);
    expect(textOf("call-status")).toBe("No response");
    expect(textOf("call-url")).toBe("http://localhost/slow");
    expect(textOf("call-location")).toBe("a.feature:9");
    expect(textOf("call-error")).toBe("Read timed out\n  at java.net.Socket.read");
    expect(textOf("call-request-headers")).toBe("Accept: */*"); // the request was logged
    expect(screen.queryByTestId("call-response-headers")).toBeNull();
    expect(screen.queryByTestId("call-response-body")).toBeNull();
  });

  it("copies the call as cURL", async () => {
    setup();
    const writeText = vi.fn(async (_text: string) => {});
    stubClipboard(writeText);
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    selectCall(0);
    click(id("copy-curl"));
    await settle();
    expect(writeText).toHaveBeenCalledExactlyOnceWith(curlCommand(created));
    expect(writeText.mock.calls[0][0]).toBe(
      "curl -X POST 'http://localhost/users' -H 'Content-Type: application/json' -H 'Accept: application/json' --data-raw '{\"name\":\"bob\",\"tags\":[1,2]}'",
    );
    expect(textOf("copy-curl")).toBe("Copied");
    vi.advanceTimersByTime(1999);
    flush();
    expect(textOf("copy-curl")).toBe("Copied");
    vi.advanceTimersByTime(1);
    flush();
    expect(textOf("copy-curl")).toBe("Copy as cURL");
  });

  it("says when the copy failed", async () => {
    setup();
    stubClipboard(vi.fn(async () => Promise.reject(new Error("denied"))));
    selectCall(0);
    click(id("copy-curl"));
    await settle();
    expect(textOf("copy-curl")).toBe("Copy failed");
  });

  it("says the copy failed when the window has no clipboard", async () => {
    setup();
    selectCall(0);
    click(id("copy-curl"));
    await settle();
    expect(textOf("copy-curl")).toBe("Copy failed");
  });

  it("does not carry Copied over to another call", async () => {
    setup();
    stubClipboard(vi.fn(async () => {}));
    selectCall(0);
    click(id("copy-curl"));
    await settle();
    expect(textOf("copy-curl")).toBe("Copied");
    click(screen.getAllByTestId("http-call")[1]);
    expect(textOf("call-request-body")).toContain("<user>");
    expect(textOf("copy-curl")).toBe("Copy as cURL");
  });

  it("disables Copy as cURL without a URL", () => {
    setup();
    const writeText = vi.fn(async (_text: string) => {});
    stubClipboard(writeText);
    selectCall(3);
    expect(textOf("call-status")).toBe("No response");
    expect(textOf("call-method")).toBe("GET");
    expect(screen.queryByTestId("call-url")).toBeNull();
    expect(textOf("call-error")).toBe("Connection refused");
    const button = id("copy-curl") as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    click(button);
    expect(writeText).not.toHaveBeenCalled();
  });

  it("gives the pane back to the node", () => {
    setup();
    selectCall(0);
    expect(screen.getByTestId("call-details")).toBeTruthy();
    fromTree("create user");
    expect(screen.queryByTestId("call-details")).toBeNull();
    expect(textOf("details-location")).toBe("a.feature:3");
    // the HTTP tab is still up, but nothing is selected in it
    expect(id("tab-http").getAttribute("aria-selected")).toBe("true");

    click(screen.getAllByTestId("http-call")[0]);
    expect(screen.getByTestId("call-details")).toBeTruthy();
    click(id("tab-console"));
    expect(screen.queryByTestId("call-details")).toBeNull();
    expect(textOf("details-location")).toBe("a.feature:3");
  });
});
