import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { Check, CircleAlert, Copy } from "lucide";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import { curlCommand, type HttpCall } from "../shared/http";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";
import { btnSmall } from "./buttons";
import { GROUP_TINT } from "./HttpCalls";
import { TINT } from "./ui";
import { drawn } from "../../test/drawn";

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
// nothing logged to show under Request or Response
const noContent = call({
  method: "DELETE",
  url: "http://localhost/users/7",
  status: 204,
  startTime: 5,
});
const silent = call({ url: "http://localhost/hang", status: null, error: null, startTime: 6 });

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
  send({ type: "calls", key: "s:a.feature#2", calls: [timedOut, refused, noContent, silent] });
  return { send };
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
const heading = (name: string) => screen.queryByRole("heading", { name });
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
    expect(heading("Request")).toBeTruthy();
    expect(heading("Response")).toBeTruthy();
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
    expect(heading("Request")).toBeTruthy();
    expect(heading("Response")).toBeTruthy(); // it holds the error
  });

  it("has no heading with nothing under it", () => {
    setup();
    // a 204 with no headers and no body: the status says it all
    selectCall(4);
    expect(textOf("call-status")).toBe("204 · 2xx Success");
    expect(heading("Request")).toBeNull();
    expect(heading("Response")).toBeNull();
    // No response and no error either
    click(screen.getAllByTestId("http-call")[5]);
    expect(textOf("call-status")).toBe("No response");
    expect(textOf("call-url")).toBe("http://localhost/hang");
    expect(screen.queryByTestId("call-error")).toBeNull();
    expect(heading("Request")).toBeNull();
    expect(heading("Response")).toBeNull();
    // no logged request: Response still holds the error
    click(screen.getAllByTestId("http-call")[3]);
    expect(heading("Request")).toBeNull();
    expect(heading("Response")).toBeTruthy();
    expect(textOf("call-error")).toBe("Connection refused");
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
    expect(drawn(id("copy-curl").querySelector("svg"), Check)).toBe(true);
    vi.advanceTimersByTime(1999);
    flush();
    expect(textOf("copy-curl")).toBe("Copied");
    vi.advanceTimersByTime(1);
    flush();
    expect(textOf("copy-curl")).toBe("Copy as cURL");
    expect(drawn(id("copy-curl").querySelector("svg"), Copy)).toBe(true);
  });

  it("says when the copy failed", async () => {
    setup();
    stubClipboard(vi.fn(async () => Promise.reject(new Error("denied"))));
    selectCall(0);
    click(id("copy-curl"));
    await settle();
    expect(textOf("copy-curl")).toBe("Copy failed");
    expect(drawn(id("copy-curl").querySelector("svg"), Check)).toBe(false); // it was not copied
  });

  it("styles Copy as cURL as a small button with the copy icon, announced as it changes", () => {
    setup();
    selectCall(0);
    const button = id("copy-curl");
    expect(button.className).toBe(btnSmall);
    expect(button.getAttribute("aria-live")).toBe("polite");
    const svg = button.querySelector("svg")!;
    expect(drawn(svg, Copy)).toBe(true);
    expect(svg.getAttribute("width")).toBe("14");
    expect(svg.getAttribute("aria-hidden")).toBe("true");
  });

  it("heads the call with its method and URL in the mono, then a tinted badge and the time", () => {
    setup();
    selectCall(0);
    const title = within(id("call-details")).getByRole("heading", { level: 2 });
    expect(title.className).toContain("font-mono");
    expect(title.className).toContain("text-lg");
    // 500: the mono is bundled at 400 and 500 only, so 600 would be a faux bold (R2.1)
    expect(title.className).toContain("font-medium");
    expect(title.className).not.toContain("font-semibold");
    expect(title.textContent).toBe("POSThttp://localhost/users");
    const status = id("call-status");
    expect(status.className).toContain(TINT[GROUP_TINT["2xx"]]);
    expect(status.className).toContain("rounded-full");
    expect(status.querySelector(".font-mono.tabular-nums")!.textContent).toBe("201");
    expect(id("call-time").className).toContain("font-mono");
    expect(id("call-time").className).toContain("tabular-nums");
    expect(id("call-location").className).toContain("text-muted");
    expect(id("call-via").className).toContain("text-muted");
  });

  it("tints the badge by group, with a code outside 100-599 neutral", () => {
    const { send } = setup();
    send({
      type: "calls",
      key: "s:a.feature#2",
      calls: [
        call({ status: 404, startTime: 10 }),
        call({ status: 503, startTime: 11 }),
        call({ status: 999, startTime: 12 }),
        call({ status: null, error: "x", startTime: 13 }),
      ],
    });
    selectCall(2);
    const check = (text: string, tint: keyof typeof TINT) => {
      expect(textOf("call-status")).toBe(text);
      expect(id("call-status").className).toContain(TINT[tint]);
    };
    check("404 · 4xx Client error", GROUP_TINT["4xx"]);
    click(screen.getAllByTestId("http-call")[3]);
    check("503 · 5xx Server error", GROUP_TINT["5xx"]);
    click(screen.getAllByTestId("http-call")[4]);
    check("999", "skipped");
    click(screen.getAllByTestId("http-call")[5]);
    check("No response", GROUP_TINT.none);
  });

  it("sets Request and Response as 12 px section titles over sunken boxes", () => {
    setup();
    selectCall(0);
    for (const name of ["Request", "Response"]) {
      const h = heading(name)!;
      expect(h.className).toContain("text-xs");
      expect(h.className).toContain("font-medium");
    }
    for (const testid of [
      "call-request-headers",
      "call-request-body",
      "call-response-headers",
      "call-response-body",
    ]) {
      const box = id(testid).className;
      for (const c of ["bg-sunken", "rounded-lg", "font-mono", "text-xs", "overflow-auto"])
        expect(box, `${testid} ${c}`).toContain(c);
    }
  });

  it("shows the no-response error in a failed box titled with the alert icon", () => {
    setup();
    selectCall(2);
    const error = id("call-error");
    const box = error.parentElement!;
    expect(box.className).toContain(TINT.failed);
    expect(box.className).toContain("rounded-lg");
    expect(box.textContent).toContain("Error");
    expect(drawn(box.querySelector("svg"), CircleAlert)).toBe(true);
    expect(error.className).toContain("text-ink");
    expect(error.className).toContain("whitespace-pre-wrap");
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

  it("gives the pane back to the node when a new run empties its calls", () => {
    const { send } = setup();
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    fromTree("create user");
    selectCall(0);
    expect(screen.getByTestId("call-details")).toBeTruthy();
    send({ type: "calls", key: "s:a.feature#1", calls: [] });
    expect(screen.queryByTestId("call-details")).toBeNull();
    expect(screen.queryAllByTestId("http-call")).toHaveLength(0);
    expect(textOf("details-location")).toBe("a.feature:3");
    expect(error).not.toHaveBeenCalled();
    error.mockRestore();
  });
});
