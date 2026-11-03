import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";
import { exampleColumns } from "./Examples";

const F = "tests/o.feature";
const OUTLINE = `o:${F}#1`;
const ROW = (n: number) => `s:${F}#1.${n}`;

const node = (
  kind: TestNode["kind"],
  key: string,
  name: string,
  extra: Partial<TestNode> = {},
): TestNode => ({
  key,
  kind,
  name,
  file: F,
  line: null,
  refId: null,
  tags: [],
  children: [],
  ...extra,
});
const row = (n: number, name: string, example?: Record<string, string>) =>
  node("row", ROW(n), name, { line: 7 + n, refId: `[1.${n}:7]`, example });
const VALUES = [
  { a: "1", b: "2", expected: "3" },
  { a: "2", b: "3", expected: "5" },
  { a: "5", b: "5", expected: "10" },
];
const tree = (rows: TestNode[] = VALUES.map((v, i) => row(i + 1, `add ${v.a} and ${v.b}`, v))) =>
  node("folder", "d:", "p", {
    children: [
      node("feature", `f:${F}`, "o", {
        children: [
          node("outline", OUTLINE, "add <a> and <b>", { line: 3, children: rows }),
          node("scenario", `s:${F}#2`, "plain", { line: 12, refId: "[2:12]" }),
        ],
      }),
    ],
  });

function setup(t: TestNode = tree()) {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async () => {}),
    steps: vi.fn(async () => null),
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
  const project = (tr: TestNode) =>
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: tr,
      error: null,
    });
  project(t);
  const select = (name: string) => {
    fireEvent.click(within(screen.getByTestId("tree")).getByText(name));
    flush();
  };
  return { send, select, project };
}

const details = () => screen.getByTestId("details");
const heading = () => within(details()).getByRole("heading").textContent;
const table = () => within(details()).getByRole("listbox", { name: "Examples" });
const options = () => within(table()).getAllByRole("option");
const nameOf = (el: HTMLElement) => el.getAttribute("aria-label");
/** Line n (1-based): the focus line is drawn first, so DOM order is not line order. */
const line = (n: number) => options().find((o) => o.dataset.key === ROW(n))!;

afterEach(cleanup);

describe("exampleColumns", () => {
  it("gives the columns of all rows in the order first met", () => {
    const rows = [row(1, "x", { a: "1", b: "2" }), row(2, "y", { a: "1", c: "3" }), row(3, "z")];
    expect(exampleColumns(rows)).toEqual(["a", "b", "c"]);
  });
});

describe("the Examples table (data-driven view R2, R3)", () => {
  it("shows an outline's rows with their values, numbers and status before a run", () => {
    const { select } = setup();
    select("add <a> and <b>");
    expect(heading()).toBe("add <a> and <b>");
    const head = within(details()).getByTestId("examples-head");
    expect(
      [...head.querySelectorAll("[data-testid=example-column]")].map((c) => c.textContent),
    ).toEqual(["a", "b", "expected"]);
    expect(options()).toHaveLength(3);
    expect(options().map(nameOf)).toEqual([
      "Row 1: a 1, b 2, expected 3, Not run",
      "Row 2: a 2, b 3, expected 5, Not run",
      "Row 3: a 5, b 5, expected 10, Not run",
    ]);
    expect(options().map((o) => o.getAttribute("aria-selected"))).toEqual([
      "false",
      "false",
      "false",
    ]);
    const values = within(options()[2])
      .getAllByTestId("example-value")
      .map((v) => v.textContent);
    expect(values).toEqual(["5", "5", "10"]);
  });

  it("updates a line's status and time during a run without redrawing it", () => {
    const { select, send } = setup();
    select("add <a> and <b>");
    const second = options()[1];
    send({
      type: "runStarted",
      selection: OUTLINE,
      runId: "20261103-150000-000",
      keys: [ROW(1), ROW(2), ROW(3)],
      total: 3,
      startedAt: Date.now(),
      env: null,
      debug: false,
    });
    send({ type: "scenarios", updates: [{ key: ROW(2), status: "passed", durationMillis: 6 }] });
    expect(options()[1]).toBe(second);
    expect(nameOf(second)).toBe("Row 2: a 2, b 3, expected 5, Passed, 6 ms");
    expect(second.getAttribute("data-status")).toBe("passed");
    expect(within(second).getByTestId("example-time").textContent).toBe("6 ms");
  });

  it("opens a row on a click on its line", () => {
    const { select } = setup();
    select("add <a> and <b>");
    fireEvent.click(within(options()[2]).getByText("10"));
    flush();
    expect(heading()).toBe("add 5 and 5");
    const treeRow = within(screen.getByTestId("tree"))
      .getAllByRole("treeitem")
      .find((r) => r.dataset.key === ROW(3))!;
    expect(treeRow.getAttribute("aria-selected")).toBe("true");
  });

  it("moves with the arrow keys without opening, and opens with Enter", () => {
    const { select } = setup();
    select("add <a> and <b>");
    line(1).focus();
    flush();
    const key = (k: string) => {
      document.activeElement!.dispatchEvent(
        new KeyboardEvent("keydown", { key: k, bubbles: true, cancelable: true }),
      );
      flush();
    };
    key("ArrowDown");
    expect(document.activeElement).toBe(line(2));
    expect(heading()).toBe("add <a> and <b>");
    key("Enter");
    expect(heading()).toBe("add 2 and 3");
    // the focused line is gone: focus goes to the row's heading, not the page
    expect(document.activeElement).toBe(within(details()).getByRole("heading"));
  });

  it("shows empty cells for a row without values, and a column only some rows have", () => {
    const { select } = setup(
      tree([row(1, "one", { a: "1", b: "2" }), row(2, "two", { a: "3", c: "4" }), row(3, "three")]),
    );
    select("add <a> and <b>");
    expect(options().map(nameOf)).toEqual([
      "Row 1: a 1, b 2, Not run",
      "Row 2: a 3, c 4, Not run",
      "Row 3: Not run",
    ]);
    const cells = (i: number) =>
      within(options()[i])
        .getAllByTestId("example-value")
        .map((v) => v.textContent);
    expect(cells(1)).toEqual(["3", "", "4"]);
    expect(cells(2)).toEqual(["", "", ""]);
  });

  it("follows a refresh of the tree while the outline is selected", () => {
    const { select, project } = setup();
    select("add <a> and <b>");
    const changed = [{ a: "9", b: "2", expected: "11" }, ...VALUES.slice(1)];
    project(tree(changed.map((v, i) => row(i + 1, `add ${v.a} and ${v.b}`, v))));
    expect(nameOf(options()[0])).toBe("Row 1: a 9, b 2, expected 11, Not run");
  });

  it("draws only the lines in view of a big outline", () => {
    const rows = Array.from({ length: 10_000 }, (_, i) =>
      row(i + 1, `big ${i + 1}`, { n: `${i + 1}` }),
    );
    const { select } = setup(tree(rows));
    select("add <a> and <b>");
    expect(table().getAttribute("data-rows")).toBe("10000");
    expect(options().length).toBeLessThan(60);
  });

  it("opens another outline's table at its top, and keeps the place on a refresh", () => {
    const outline = (n: number) =>
      node("outline", `o:${F}#${n}`, `outline ${n}`, {
        line: n,
        children: Array.from({ length: 20 }, (_, i) =>
          node("row", `s:${F}#${n}.${i + 1}`, `o${n} row ${i + 1}`, {
            refId: `[${n}.${i + 1}:${n}]`,
            example: { i: `${i + 1}` },
          }),
        ),
      });
    const big = () =>
      node("folder", "d:", "p", {
        children: [node("feature", `f:${F}`, "o", { children: [outline(1), outline(2)] })],
      });
    const { select, project } = setup(big());
    const scroller = () => table().parentElement!;
    select("outline 1");
    scroller().scrollTop = 200;
    scroller().dispatchEvent(new Event("scroll"));
    flush();
    project(big());
    expect(scroller().scrollTop).toBe(200);
    select("outline 2");
    expect(heading()).toBe("outline 2");
    expect(scroller().scrollTop).toBe(0);
  });

  it("is not shown for a scenario", () => {
    const { select } = setup();
    select("plain");
    expect(within(details()).queryByRole("listbox", { name: "Examples" })).toBeNull();
  });
});
