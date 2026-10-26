import { describe, expect, it } from "vitest";
import type { HttpCall } from "../shared/http";
import type { TestNode } from "../shared/model";
import {
  findCall,
  groupCounts,
  narrowedCalls,
  shownCalls,
  toggledGroups,
  totalCalls,
  type CallRef,
} from "./http";

const node = (key: string, kind: TestNode["kind"], children: TestNode[] = []): TestNode => ({
  key,
  kind,
  name: key,
  file: "",
  line: null,
  refId: null,
  tags: [],
  children,
});
const tree = node("d:", "folder", [
  node("f:a.feature", "feature", [
    node("s:a.feature#1", "scenario"),
    node("s:a.feature#2", "scenario"),
  ]),
  node("f:b.feature", "feature", [node("s:b.feature#1", "scenario")]),
]);

const call = (status: number | null, startTime: number): HttpCall => ({
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
  startTime,
  attempt: null,
});
const calls: Record<string, HttpCall[]> = {
  "s:a.feature#1": [call(200, 100), call(404, 300)],
  "s:a.feature#2": [call(500, 200)],
  "s:b.feature#1": [call(null, 50), call(999, 400)], // 999 is in no group
};
const state = (selected: string | null = null) => ({ tree, selected, calls });
const refs = (selected: string | null = null) =>
  narrowedCalls(state(selected)).map((l) => `${l.ref.key}/${l.ref.index}`);

describe("narrowedCalls", () => {
  it("lists every call in start-time order", () => {
    const all = [
      "s:b.feature#1/0",
      "s:a.feature#1/0",
      "s:a.feature#2/0",
      "s:a.feature#1/1",
      "s:b.feature#1/1",
    ];
    expect(refs()).toEqual(all);
    expect(refs("d:")).toEqual(all);
    expect(narrowedCalls({ tree: null, selected: null, calls })).toEqual([]);
    // the same start time keeps the log order
    const tied = { "s:a.feature#1": [call(200, 7), call(201, 7), call(202, 7)] };
    expect(narrowedCalls({ tree, selected: null, calls: tied }).map((l) => l.call.status)).toEqual([
      200, 201, 202,
    ]);
  });

  it("narrows to the selected node", () => {
    expect(refs("s:a.feature#1")).toEqual(["s:a.feature#1/0", "s:a.feature#1/1"]);
    expect(refs("s:a.feature#2")).toEqual(["s:a.feature#2/0"]);
    expect(refs("f:a.feature")).toEqual(["s:a.feature#1/0", "s:a.feature#2/0", "s:a.feature#1/1"]);
    const [first] = narrowedCalls(state("f:b.feature"));
    expect(first.test.key).toBe("s:b.feature#1");
    expect(first.call.status).toBeNull();
  });
});

describe("groupCounts", () => {
  it("counts every group, zeros included", () => {
    expect(groupCounts(narrowedCalls(state()))).toEqual({
      all: 5,
      "1xx": 0,
      "2xx": 1,
      "3xx": 0,
      "4xx": 1,
      "5xx": 1,
      none: 1,
    });
  });
});

describe("shownCalls", () => {
  it("with no group on shows all; with 4xx and none on shows only those", () => {
    const list = narrowedCalls(state());
    expect(shownCalls(list, [])).toEqual(list);
    expect(shownCalls(list, ["4xx", "none"]).map((l) => l.call.status)).toEqual([null, 404]);
  });
});

describe("toggledGroups", () => {
  it("turns a group on and off; All turns every group off", () => {
    expect(toggledGroups([], "4xx")).toEqual(["4xx"]);
    expect(toggledGroups(["4xx"], "none")).toEqual(["4xx", "none"]);
    expect(toggledGroups(["4xx", "none"], "4xx")).toEqual(["none"]);
    expect(toggledGroups(["4xx", "none"], "all")).toEqual([]);
  });
});

describe("totalCalls", () => {
  it("adds up the calls of every test", () => {
    expect(totalCalls(calls)).toBe(5);
    expect(totalCalls({})).toBe(0);
  });
});

describe("findCall", () => {
  const find = (selectedCall: CallRef | null, c = calls) =>
    findCall({ tree, calls: c, selectedCall });

  it("resolves the selected call and is null once it is gone", () => {
    expect(find({ key: "s:a.feature#2", index: 0 })).toMatchObject({
      ref: { key: "s:a.feature#2", index: 0 },
      call: { status: 500 },
      test: { key: "s:a.feature#2" },
    });
    expect(find(null)).toBeNull();
    expect(find({ key: "s:a.feature#2", index: 1 })).toBeNull(); // no such call
    expect(find({ key: "s:gone", index: 0 })).toBeNull(); // no such test
    expect(find({ key: "s:a.feature#2", index: 0 }, {})).toBeNull(); // calls cleared
    expect(
      findCall({ tree: null, calls, selectedCall: { key: "s:a.feature#2", index: 0 } }),
    ).toBeNull();
  });
});
