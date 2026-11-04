import { cleanup, fireEvent, render, screen, within } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { HistoryEntry } from "../shared/history";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import type { TrendPoint } from "../shared/trends";
import { openFeatures } from "../../test/openFeatures";
import { App } from "./App";

// "now" is 2026-11-04 18:00 local time; only Date is faked so flush() keeps working
beforeEach(() => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 10, 4, 18, 0));
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
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

const entry = (id: string): HistoryEntry => ({
  id,
  startedAt: new Date(2026, 10, 4, 14, 32).getTime(),
  durationMillis: 5000,
  env: null,
  threads: 1,
  filter: { include: [], exclude: [], hideNonMatching: false },
  selection: { kind: "folder", file: "", name: "root", line: null, key: "d:" },
  failedOf: null,
  outcome: "finished",
  exitCode: 1,
  passed: 1,
  failed: 1,
  total: 2,
  reportAvailable: true,
});
const point = (id: string, over: Partial<TrendPoint> = {}): TrendPoint => ({
  id,
  startedAt: new Date(2026, 10, 4, 14, 32).getTime(),
  passed: 3,
  failed: 1,
  total: 2,
  meanMillis: 1000,
  ...over,
});

/** Answers to `trend` the test settles by hand, in the order asked. */
function deferred() {
  const pending: ((p: TrendPoint[]) => void)[] = [];
  const trend = vi.fn((_key: string | null) => new Promise<TrendPoint[]>((r) => pending.push(r)));
  const answer = async (i: number, points: TrendPoint[]) => {
    pending[i](points);
    await Promise.resolve();
    await Promise.resolve();
    flush();
  };
  return { trend, answer };
}

function setup(trend: Api["trend"] = vi.fn(async () => [])) {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    steps: vi.fn(async () => null),
    trend,
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  send({ type: "project", project: { root: "D:/p", layout: "standalone" }, tree, error: null });
  openFeatures();
  const load = (...ids: string[]) => send({ type: "history", runs: ids.map(entry) });
  return { api, send, load };
}

const click = (el: HTMLElement) => {
  fireEvent.click(el);
  flush();
};
const settle = async () => {
  for (let i = 0; i < 3; i++) await Promise.resolve();
  flush();
};
const showHistory = () => click(screen.getByTestId("tab-history"));
const fromTree = (name: string) => click(within(screen.getByTestId("tree")).getByText(name));
const columns = () => screen.queryAllByTestId("trend-run");
const strip = () => screen.queryByTestId("trends");

describe("Trends", () => {
  it("asks for the whole tree and draws a column per point", async () => {
    const { api, load } = setup(vi.fn(async () => [point("a"), point("b"), point("c")]));
    load("c", "b", "a");
    showHistory();
    await settle();
    expect(api.trend).toHaveBeenCalledExactlyOnceWith(null);
    expect(columns().map((c) => c.getAttribute("data-id"))).toEqual(["a", "b", "c"]);
    expect(screen.getByTestId("trends-caption").textContent).toBe("All tests");
    expect(strip()!.getAttribute("role")).toBe("group");
    expect(strip()!.getAttribute("aria-label")).toBe("Trend of All tests");
  });

  it("follows the tree selection", async () => {
    const { api, load } = setup(
      vi.fn(async () => [point("a", { total: 1, passed: 1, failed: 0 })]),
    );
    load("a");
    showHistory();
    await settle();
    fromTree("create user");
    await settle();
    expect(api.trend).toHaveBeenLastCalledWith("s:tests/users.feature#1");
    expect(screen.getByTestId("trends-caption").textContent).toBe("create user");
    expect(strip()!.getAttribute("aria-label")).toBe("Trend of create user");
    expect(screen.getByTestId("trends-pass-label").textContent).toBe("Pass rate Passed");
  });

  it("asks one at a time and draws only the newest answer", async () => {
    const { trend, answer } = deferred();
    const { load } = setup(trend);
    load("a");
    showHistory();
    fromTree("create user");
    fromTree("list users");
    expect(trend).toHaveBeenCalledTimes(1);
    await answer(0, [point("stale")]);
    expect(columns()).toHaveLength(0); // a newer selection came: this answer is dropped
    expect(trend).toHaveBeenCalledTimes(2);
    expect(trend).toHaveBeenLastCalledWith("s:tests/users.feature#2");
    await answer(1, [point("fresh")]);
    expect(columns().map((c) => c.getAttribute("data-id"))).toEqual(["fresh"]);
    expect(trend).toHaveBeenCalledTimes(2);
  });

  it("asks again when the history changes", async () => {
    const { api, load } = setup();
    load("a");
    showHistory();
    await settle();
    load("b", "a");
    await settle();
    expect(api.trend).toHaveBeenCalledTimes(2);
  });

  it("shows no strip and asks nothing with no run", async () => {
    const { api } = setup();
    showHistory();
    await settle();
    expect(strip()).toBeNull();
    expect(api.trend).not.toHaveBeenCalled();
  });

  it("asks nothing while History is hidden", async () => {
    const { api, load } = setup();
    load("a");
    fromTree("create user");
    await settle();
    expect(api.trend).not.toHaveBeenCalled();
  });

  it("splits the pass bar and scales the time bar", async () => {
    const { load } = setup(
      vi.fn(async () => [
        point("a", { passed: 3, failed: 1, meanMillis: 500 }),
        point("b", { passed: 4, failed: 0, meanMillis: 2000 }),
        point("c", { meanMillis: null }),
      ]),
    );
    load("c", "b", "a");
    showHistory();
    await settle();
    const [a, b, c] = columns();
    expect(within(a).getByTestId("trend-passed").style.flexGrow).toBe("3");
    expect(within(a).getByTestId("trend-failed").style.flexGrow).toBe("1");
    expect(within(b).queryByTestId("trend-failed")).toBeNull();
    expect(within(b).getByTestId("trend-time").style.height).toBe("100%");
    expect(within(a).getByTestId("trend-time").style.height).toBe("25%");
    expect(within(c).queryByTestId("trend-time")).toBeNull();
    expect(screen.getByTestId("trends-time-max").textContent).toBe("2.00 s");
  });

  it("labels the newest point", async () => {
    const { load } = setup(
      vi.fn(async () => [point("a"), point("b", { passed: 299, failed: 1, meanMillis: 1500 })]),
    );
    load("b", "a");
    showHistory();
    await settle();
    expect(screen.getByTestId("trends-pass-label").textContent).toBe("Pass rate 99%");
    expect(screen.getByTestId("trends-time-label").textContent).toBe("Time per test 1.50 s");
  });

  it("names each column by its run", async () => {
    const { load } = setup(vi.fn(async () => [point("a", { total: 412, passed: 12, failed: 3 })]));
    load("a");
    showHistory();
    await settle();
    expect(columns()[0].getAttribute("aria-label")).toBe(
      "Today 14:32: 12 passed, 3 failed, 15 of 412 tests, 1.00 s per test",
    );
  });

  it("says so when no stored run ran the selection", async () => {
    const { load } = setup();
    load("a");
    showHistory();
    await settle();
    expect(screen.getByTestId("trends-none").textContent).toBe("No stored run ran these tests");
    expect(columns()).toHaveLength(0);
  });

  it("a click scrolls to the run's row", async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    const { load } = setup(vi.fn(async () => [point("a"), point("b")]));
    load("b", "a");
    showHistory();
    await settle();
    click(columns()[0]);
    expect(scroll).toHaveBeenCalledOnce();
    expect((scroll.mock.contexts[0] as HTMLElement).getAttribute("data-id")).toBe("a");
    expect(scroll).toHaveBeenCalledWith({ block: "nearest" });
  });
});
