import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { Check, Globe } from "lucide";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { HttpCall } from "../shared/http";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";
import { callName, GROUP_TINT } from "./HttpCalls";
import { TINT } from "./ui";
import { scrollToShow } from "./virtual";
import { drawn } from "../../test/drawn";

afterEach(() => {
  cleanup();
  vi.useRealTimers();
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

const node = (key: string, kind: TestNode["kind"], name: string, children: TestNode[] = []) => ({
  key,
  kind,
  name,
  file: "tests/users.feature",
  line: 1,
  refId: null,
  tags: [],
  children,
});
const tree: TestNode = node("d:", "folder", "root", [
  node("f:tests/users.feature", "feature", "users", [
    node("s:tests/users.feature#1", "scenario", "create user"),
    node("s:tests/users.feature#2", "scenario", "list users"),
  ]),
]);

// Four calls: 201, two 404 attempts, and one with no response.
const ALL: Record<string, HttpCall[]> = {
  "s:tests/users.feature#1": [
    call({
      method: "POST",
      status: 201,
      millis: 6,
      via: "tests/common/create-user.feature:8",
      startTime: 1,
    }),
    call({
      url: "http://localhost/users/9",
      status: 404,
      millis: 12,
      attempt: { n: 1, of: 2 },
      startTime: 2,
    }),
    call({
      url: "http://localhost/users/9",
      status: 404,
      attempt: { n: 2, of: 2 },
      startTime: 3,
    }),
  ],
  "s:tests/users.feature#2": [
    call({
      url: null,
      status: null,
      error: "Connection refused\n  at java.net.Socket.connect",
      startTime: 4,
    }),
  ],
};

function setup(calls: Record<string, HttpCall[]> = ALL) {
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
  for (const [key, list] of Object.entries(calls)) send({ type: "calls", key, calls: list });
  return { send };
}

const click = (el: HTMLElement) => {
  fireEvent.click(el);
  flush();
};
const showHttp = () => click(screen.getByTestId("tab-http"));
// The focus row is first in the DOM, wherever it sits in the list: read the rows by position, as a person does.
const place = (el: HTMLElement) => Number(el.getAttribute("aria-posinset"));
const rows = () => screen.queryAllByTestId("http-call").sort((a, b) => place(a) - place(b));
/** The call at this 1-based position, when it is drawn. */
const option = (n: number) => rows().find((r) => place(r) === n)!;
/** What a row draws inside: a click on it selects the call. */
const body = (i: number) => rows()[i].firstElementChild as HTMLElement;
const text = (i: number, id: string) => within(rows()[i]).getByTestId(id).textContent;
const chips = () => screen.getAllByTestId("http-group");
const chip = (group: string) => chips().find((c) => c.dataset.group === group)!;
const chipTexts = () => chips().map((c) => c.textContent);
const fromTree = (name: string) => click(within(screen.getByTestId("tree")).getByText(name));

describe("HTTP tab", () => {
  it("adds the HTTP tab after History", () => {
    const { send } = setup();
    const tablist = screen.getByRole("tablist");
    expect(tablist.getAttribute("aria-label")).toBe("Console, run history and HTTP calls");
    expect(
      within(tablist)
        .getAllByRole("tab")
        .map((t) => t.textContent),
    ).toEqual(["Console", "History (0)", "HTTP (4)"]);
    expect(screen.getByTestId("tab-console").getAttribute("aria-selected")).toBe("true");
    expect(screen.queryByTestId("http-calls")).toBeNull();
    showHttp();
    expect(screen.getByTestId("tab-http").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("http-calls")).toBeTruthy();
    expect(screen.queryByTestId("console")).toBeNull();
    // starting a run keeps the HTTP tab open, and the calls of tests outside the run stay
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-150000-000",
      keys: [],
      total: 0,
      startedAt: 0,
      env: null,
    });
    expect(screen.getByTestId("tab-http").getAttribute("aria-selected")).toBe("true");
    expect(screen.getByTestId("http-calls")).toBeTruthy();
    expect(screen.getByTestId("tab-http").textContent).toBe("HTTP (4)");
  });

  it("shows every group chip with its count", () => {
    setup();
    showHttp();
    expect(chipTexts()).toEqual([
      "All 4",
      "1xx Informational 0",
      "2xx Success 1",
      "3xx Redirect 0",
      "4xx Client error 2",
      "5xx Server error 0",
      "No response 1",
    ]);
    expect(chips().map((c) => c.dataset.group)).toEqual([
      "all",
      "1xx",
      "2xx",
      "3xx",
      "4xx",
      "5xx",
      "none",
    ]);
    expect(chips().map((c) => c.getAttribute("aria-pressed"))).toEqual([
      "true",
      "false",
      "false",
      "false",
      "false",
      "false",
      "false",
    ]);
  });

  it("shows a row per call", () => {
    setup();
    showHttp();
    expect(rows()).toHaveLength(4);
    expect(rows().map((r) => r.dataset.status)).toEqual(["201", "404", "404", "none"]);
    expect(rows().map((r) => r.getAttribute("role"))).toEqual(Array(4).fill("option"));

    expect(text(0, "http-call-status")).toBe("201");
    expect(text(0, "http-call-method")).toBe("POST");
    expect(text(0, "http-call-url")).toBe("http://localhost/users");
    expect(text(0, "http-call-test")).toBe("create user");
    expect(text(0, "http-call-via")).toBe("via tests/common/create-user.feature:8");
    expect(text(0, "http-call-time")).toBe("6 ms");
    expect(within(rows()[0]).queryByTestId("http-call-attempt")).toBeNull();

    expect(text(1, "http-call-status")).toBe("404");
    expect(within(rows()[1]).getByTestId("http-call-url").hasAttribute("title")).toBe(false);
    expect(text(1, "http-call-attempt")).toBe("attempt 1 of 2");
    expect(within(rows()[1]).queryByTestId("http-call-via")).toBeNull();
    expect(text(2, "http-call-attempt")).toBe("attempt 2 of 2");
    expect(text(2, "http-call-time")).toBe("—");

    expect(text(3, "http-call-status")).toBe("—");
    expect(text(3, "http-call-url")).toBe("Connection refused");
    expect(text(3, "http-call-test")).toBe("list users");
    expect(text(3, "http-call-time")).toBe("—");
  });

  it("maps each group to the tint of its colour", () => {
    expect(GROUP_TINT).toEqual({
      "1xx": "skipped",
      "2xx": "passed",
      "3xx": "running",
      "4xx": "cancelled",
      "5xx": "failed",
      none: "failed",
    });
  });

  describe("status badge", () => {
    const badge = (i: number) => within(rows()[i]).getByTestId("http-call-status");
    // The tooltip of one badge: rest the pointer on its wrapper, read it, then leave.
    const tipOf = (el: HTMLElement) => {
      fireEvent.pointerEnter(el.parentElement!);
      vi.advanceTimersByTime(500);
      const tip = document.querySelector<HTMLElement>("[role=tooltip]");
      // right of the badge, on its own row: below, it would take the click meant for the next row
      expect(tip?.dataset.side).toBe("right");
      fireEvent.pointerLeave(el.parentElement!);
      flush();
      return tip?.textContent ?? null;
    };

    it("shows the code tinted by its group, with the group in a tooltip", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      setup({
        "s:tests/users.feature#1": [
          call({ status: 200 }),
          call({ status: 404 }),
          call({ status: 503 }),
        ],
        "s:tests/users.feature#2": [call({ status: null, url: null })],
      });
      showHttp();
      expect(rows().map((_, i) => badge(i).textContent)).toEqual(["200", "404", "503", "—"]);
      const tints = [GROUP_TINT["2xx"], GROUP_TINT["4xx"], GROUP_TINT["5xx"], GROUP_TINT.none];
      tints.forEach((t, i) => expect(badge(i).className).toContain(TINT[t]));
      expect(rows().map((_, i) => tipOf(badge(i)))).toEqual([
        "2xx Success",
        "4xx Client error",
        "5xx Server error",
        "No response",
      ]);
    });

    it("is a rounded pill with the code in tabular mono, never white on a fill", () => {
      setup();
      showHttp();
      const cls = badge(0).className.split(" ");
      for (const c of ["rounded-full", "h-5", "font-mono", "tabular-nums", "bg-passed-tint"])
        expect(cls).toContain(c);
      for (const row of rows()) {
        const b = within(row).getByTestId("http-call-status").className;
        expect(b).not.toContain("text-white");
        expect(b).toMatch(/bg-\w+-tint/);
      }
    });

    it("keeps a code outside 100-599 neutral, named in the tooltip", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      setup({
        "s:tests/users.feature#1": [call({ status: 999 }), call({ status: 0 })],
        "s:tests/users.feature#2": [call({ status: null, url: null })],
      });
      showHttp();
      expect(badge(0).textContent).toBe("999");
      expect(badge(0).className).toContain(TINT.skipped);
      expect(tipOf(badge(0))).toBe("Status 999");
      expect(badge(1).textContent).toBe("0");
      expect(badge(1).className).toContain(TINT.skipped);
      expect(tipOf(badge(1))).toBe("Status 0");
      expect(chipTexts()[0]).toBe("All 3"); // they count in All only
    });
  });

  it("filters by the chosen groups", () => {
    setup();
    showHttp();
    click(chip("4xx"));
    expect(rows()).toHaveLength(2);
    expect(chip("4xx").getAttribute("aria-pressed")).toBe("true");
    expect(chip("all").getAttribute("aria-pressed")).toBe("false");
    click(chip("none"));
    expect(rows()).toHaveLength(3);
    expect(rows().map((r) => r.dataset.status)).toEqual(["404", "404", "none"]);
    expect(chipTexts()[0]).toBe("All 4"); // counts follow the narrowing, not the chips
    click(chip("all"));
    expect(rows()).toHaveLength(4);
    expect(chips().map((c) => c.getAttribute("aria-pressed"))).toEqual([
      "true",
      "false",
      "false",
      "false",
      "false",
      "false",
      "false",
    ]);
  });

  it("narrows to the tree selection", () => {
    setup();
    showHttp();
    fromTree("list users");
    expect(rows()).toHaveLength(1);
    expect(text(0, "http-call-status")).toBe("—");
    expect(chipTexts()).toEqual([
      "All 1",
      "1xx Informational 0",
      "2xx Success 0",
      "3xx Redirect 0",
      "4xx Client error 0",
      "5xx Server error 0",
      "No response 1",
    ]);
    expect(screen.getByTestId("tab-http").textContent).toBe("HTTP (1)"); // the rows the tab shows
    fromTree("create user");
    expect(rows()).toHaveLength(3);
    expect(chipTexts()[0]).toBe("All 3");
    expect(screen.getByTestId("tab-http").textContent).toBe("HTTP (3)");
    click(chip("2xx"));
    expect(rows()).toHaveLength(1);
    expect(screen.getByTestId("tab-http").textContent).toBe("HTTP (1)"); // chips count too
  });

  it("shows the three empty states", () => {
    const { send } = setup({});
    showHttp();
    expect(screen.getByTestId("http-empty").textContent).toBe(
      "No HTTP calls yet. Calls appear when a feature finishes.",
    );
    expect(chipTexts()[0]).toBe("All 0"); // the chips stay
    expect(screen.getByTestId("tab-http").textContent).toBe("HTTP (0)");

    send({ type: "calls", key: "s:tests/users.feature#1", calls: [call({ status: 200 })] });
    expect(screen.queryByTestId("http-empty")).toBeNull();
    expect(rows()).toHaveLength(1);
    fromTree("list users");
    expect(screen.getByTestId("http-empty").textContent).toBe("No HTTP calls in list users");
    fromTree("create user");
    click(chip("5xx"));
    expect(screen.getByTestId("http-empty").textContent).toBe("No calls in the chosen groups");
    click(chip("all"));
    expect(rows()).toHaveLength(1);
  });

  it("keeps a row and its focus when more calls arrive", () => {
    const { send } = setup();
    showHttp();
    const row = rows()[1];
    row.focus();
    send({
      type: "calls",
      key: "s:tests/users.feature#2",
      calls: [call({ startTime: 5 }), call({ startTime: 6 })],
    });
    expect(rows()).toHaveLength(5);
    expect(rows()[1]).toBe(row);
    expect(document.activeElement).toBe(row);
  });

  it("shows the new calls in place when a test's calls are replaced", () => {
    const { send } = setup();
    showHttp();
    expect(rows()).toHaveLength(4);
    send({
      type: "calls",
      key: "s:tests/users.feature#1",
      calls: [
        call({
          method: "PUT",
          url: "http://localhost/users/42",
          status: 500,
          millis: 31,
          startTime: 1,
        }),
        call({ url: "http://localhost/users/43", status: 302, millis: 4, startTime: 2 }),
      ],
    });
    expect(rows()).toHaveLength(3);
    expect(rows().map((r) => r.dataset.status)).toEqual(["500", "302", "none"]);
    expect(text(0, "http-call-status")).toBe("500");
    expect(text(0, "http-call-method")).toBe("PUT");
    expect(text(0, "http-call-url")).toBe("http://localhost/users/42");
    expect(text(0, "http-call-time")).toBe("31 ms");
    expect(within(rows()[0]).queryByTestId("http-call-via")).toBeNull();
    expect(text(1, "http-call-url")).toBe("http://localhost/users/43");
    expect(text(1, "http-call-time")).toBe("4 ms");
    expect(within(rows()[1]).queryByTestId("http-call-attempt")).toBeNull();
    expect(text(2, "http-call-test")).toBe("list users"); // the other test's row stays
    expect(screen.getByTestId("tab-http").textContent).toBe("HTTP (3)");
    expect(chipTexts()[0]).toBe("All 3");
  });

  it("selects a call and marks it selected", () => {
    setup();
    showHttp();
    const row = rows()[1];
    expect(row.getAttribute("aria-selected")).toBe("false");
    row.focus();
    expect(document.activeElement).toBe(row);
    click(body(1));
    expect(rows().map((r) => r.getAttribute("aria-selected"))).toEqual([
      "false",
      "true",
      "false",
      "false",
    ]);
    click(body(3));
    expect(rows().map((r) => r.getAttribute("aria-selected"))).toEqual([
      "false",
      "false",
      "false",
      "true",
    ]);
  });

  it("marks the pressed chips with a check and the selected fill", () => {
    setup();
    showHttp();
    const check = (g: string) => drawn(chip(g).querySelector("svg"), Check);
    expect(chip("all").className).toContain("bg-selected");
    expect(check("all")).toBe(true);
    expect(chip("2xx").className).not.toContain("bg-selected");
    expect(chip("2xx").querySelector("svg")).toBeNull();
    click(chip("4xx"));
    expect(chip("4xx").className).toContain("bg-selected");
    expect(chip("4xx").className).toContain("text-ink");
    expect(check("4xx")).toBe(true);
    expect(chip("all").className).not.toContain("bg-selected");
    expect(chip("all").querySelector("svg")).toBeNull();
    expect(chip("4xx").className).toContain("tabular-nums");
    expect(chip("4xx").className).toContain("h-6");
  });

  it("makes a row 28 px, with the selected one on the selected fill and the accent bar", () => {
    setup();
    showHttp();
    rows().forEach((row, i) => {
      expect(row.style.height).toBe("28px");
      for (const c of ["h-7", "text-xs", "border-b", "border-line"])
        expect(body(i).className).toContain(c);
      expect(body(i).className).not.toContain("bg-selected");
    });
    expect(body(0).className).toContain("hover:bg-hover");
    click(body(1));
    expect(body(1).className).toContain("bg-selected");
    expect(body(1).className).toContain("shadow-[inset_2px_0_0_var(--color-accent)]");
    expect(body(1).className).not.toContain("hover:bg-hover");
    expect(body(0).className).not.toContain("shadow-[inset");
  });

  it("sets the method, URL and time in the mono and the rest in muted", () => {
    setup();
    showHttp();
    const part = (id: string) => within(rows()[0]).getByTestId(id);
    expect(part("http-call-method").className).toContain("font-mono");
    expect(part("http-call-method").className).toContain("font-medium");
    expect(part("http-call-url").className).toContain("font-mono");
    expect(part("http-call-time").className).toContain("font-mono");
    expect(part("http-call-time").className).toContain("tabular-nums");
    expect(part("http-call-time").className).toContain("text-right");
    expect(part("http-call-test").className).toContain("text-muted");
    expect(part("http-call-via").className).toContain("text-muted");
    expect(within(rows()[1]).getByTestId("http-call-attempt").className).toContain("text-muted");
  });

  it("is a listbox of options named HTTP calls", () => {
    setup({
      "s:tests/users.feature#1": [
        call({ startTime: 1 }),
        call({ startTime: 2 }),
        call({ startTime: 3 }),
      ],
    });
    showHttp();
    const list = screen.getByRole("listbox", { name: "HTTP calls" });
    const options = within(list).getAllByRole("option");
    expect(options).toHaveLength(3);
    expect(options.every((o) => o.getAttribute("data-testid") === "http-call")).toBe(true);
    expect(options.map((o) => o.getAttribute("aria-setsize"))).toEqual(["3", "3", "3"]);
    expect(options.map(place).sort()).toEqual([1, 2, 3]);
    expect(
      screen.getAllByRole("option", { name: "200, GET, http://localhost/users, create user" }),
    );
  });

  it("names a call by status, method, URL and test", () => {
    const test = tree.children[0].children[1];
    const listed = (over: Partial<HttpCall>) => ({
      ref: { key: test.key, index: 0 },
      call: call({ url: "http://localhost/x", ...over }),
      test,
    });
    expect(callName(listed({ status: 404 }))).toBe("404, GET, http://localhost/x, list users");
    expect(callName(listed({ status: null }))).toBe(
      "no status, GET, http://localhost/x, list users",
    );
    expect(callName(listed({ status: null, url: null, error: "Refused\n  at java.net" }))).toBe(
      "no status, GET, Refused, list users",
    );
  });

  describe("keyboard", () => {
    const press = (key: string) => {
      document.activeElement!.dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
      );
      flush();
    };

    it("selects a call with the arrow keys and opens it", () => {
      setup();
      showHttp();
      // one tab stop: the first call, with nothing selected
      expect(rows().map((r) => r.getAttribute("tabindex"))).toEqual(["0", "-1", "-1", "-1"]);
      rows()[0].focus();
      press("ArrowDown");
      expect(rows()[1].getAttribute("aria-selected")).toBe("true");
      expect(rows()[0].getAttribute("aria-selected")).toBe("false");
      expect(document.activeElement).toBe(rows()[1]);
      expect(screen.getByTestId("call-url").textContent).toBe("http://localhost/users/9");
      press("End");
      expect(rows()[3].getAttribute("aria-selected")).toBe("true");
    });

    it("moves focus up when the focused call is cleared by a new run", () => {
      const { send } = setup();
      showHttp();
      const error = vi.spyOn(console, "error").mockImplementation(() => {});
      rows()[2].focus();
      send({
        type: "runStarted",
        selection: "s:tests/users.feature#1",
        runId: "20261025-150000-000",
        keys: ["s:tests/users.feature#1"], // clears the three calls of the focused call's test
        total: 1,
        startedAt: 0,
        env: null,
      });
      expect(rows()).toHaveLength(1);
      expect(document.activeElement).toBe(rows()[0]);
      expect(error).not.toHaveBeenCalled();
      error.mockRestore();
    });
  });

  describe("long lists", () => {
    const many = (n: number, from = 0) =>
      Array.from({ length: n }, (_, i) =>
        call({ url: `http://localhost/${from + i}`, startTime: from + i }),
      );
    const scrollTo = (top: number) => {
      const scroller = screen.getByTestId("http-calls");
      scroller.scrollTop = top;
      scroller.dispatchEvent(new Event("scroll"));
      flush();
      return scroller;
    };
    // jsdom has no layout: the list reads a 600 px view
    const view = (scrollTop: number) => ({ scrollTop, height: 0, rowHeight: 28, pad: 0 });

    it("scrolls to the selected call when the chips change", () => {
      setup({ "s:tests/users.feature#1": many(200) });
      showHttp();
      scrollTo(4000);
      click(option(150).firstElementChild as HTMLElement);
      const scroller = scrollTo(0);
      expect(option(150).getAttribute("aria-selected")).toBe("true"); // drawn though out of view
      click(chip("2xx")); // every call is a 2xx: the same calls stay, the selected one out of view
      expect(scroller.scrollTop).toBe(scrollToShow(149, view(0)));
      expect(scroller.scrollTop).toBe(3600);
    });

    it("shows the selected call after the chips take rows away above the view", () => {
      // every second call is a 404: the 4xx chip keeps the top row in view but moves it up the list
      const odd = (i: number) => call({ startTime: i, status: i % 2 ? 404 : 200 });
      setup({ "s:tests/users.feature#1": Array.from({ length: 100 }, (_, i) => odd(i)) });
      showHttp();
      scrollTo(4000);
      click(option(100).firstElementChild as HTMLElement); // the 100th call, a 404
      const scroller = scrollTo(28 * 51); // the 52nd call, a 404, is the top row
      click(chip("4xx"));
      expect(option(50).getAttribute("aria-selected")).toBe("true");
      // the 52nd call is now the 26th: the list moved to keep it, then the selected call came into view
      expect(scroller.scrollTop).toBe(scrollToShow(49, view(28 * 25)));
      expect(scroller.scrollTop).toBe(800);
    });

    it("keeps the calls in view still when an earlier call arrives", () => {
      const { send } = setup({ "s:tests/users.feature#1": many(60, 2) });
      showHttp();
      const scroller = scrollTo(28 * 3); // the call at 5 s is the top row
      send({ type: "calls", key: "s:tests/users.feature#2", calls: [call({ startTime: 1 })] });
      expect(scroller.scrollTop).toBe(28 * 3 + 28);
      // the call at 5 s is still the top row, one place further down the list
      expect(option(5).querySelector("[data-testid=http-call-url]")!.textContent).toBe(
        "http://localhost/5",
      );
    });
  });

  describe("cut text", () => {
    // jsdom has no layout: say how wide the text is and how much of it shows
    const measure = (el: HTMLElement, scrollWidth: number, clientWidth: number) => {
      Object.defineProperty(el, "scrollWidth", { value: scrollWidth, configurable: true });
      Object.defineProperty(el, "clientWidth", { value: clientWidth, configurable: true });
    };
    const rest = (el: HTMLElement) => {
      fireEvent.pointerEnter(el.parentElement!);
      vi.advanceTimersByTime(500);
    };
    const tip = () => document.querySelector("[role=tooltip]");

    it("shows a cut URL, test name and via whole in a tooltip, measured when it would show", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      setup();
      showHttp();
      const row = within(rows()[0]);
      for (const [id, whole] of [
        ["http-call-url", "http://localhost/users"],
        ["http-call-test", "create user"],
        ["http-call-via", "via tests/common/create-user.feature:8"],
      ] as const) {
        const el = row.getByTestId(id);
        expect(el.className).toContain("truncate");
        expect(el.hasAttribute("title")).toBe(false);
        measure(el, 200, 100);
        rest(el);
        expect(tip()!.textContent, id).toBe(whole);
        // over its own text: below, it would take the click meant for the next row
        expect((tip() as HTMLElement).dataset.side, id).toBe("over");
        fireEvent.pointerLeave(el.parentElement!);
        flush();
      }
      expect(row.getByTestId("http-call-url").parentElement!.className).toContain("min-w-0");
    });

    it("shows nothing for text that fits", () => {
      vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
      setup();
      showHttp();
      const el = within(rows()[0]).getByTestId("http-call-url");
      measure(el, 100, 100);
      rest(el);
      expect(tip()).toBeNull();
    });
  });

  it("shows an empty list as an EmptyState with the globe", () => {
    setup({});
    showHttp();
    const empty = screen.getByTestId("http-empty");
    expect(empty.className).toContain("text-muted");
    expect(drawn(empty.parentElement!.querySelector("svg"), Globe)).toBe(true);
    expect(empty.parentElement!.querySelector("svg")!.getAttribute("width")).toBe("24");
  });
});
