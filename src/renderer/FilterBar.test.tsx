import path from "node:path";
import { render, screen, cleanup, fireEvent } from "@solidjs/testing-library";
import { Check, Minus, type IconNode } from "lucide";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import type { FilterState } from "../shared/tags";
import { readAllEvents } from "../main/karate/events";
import { buildTree } from "../main/karate/tree";
import { App } from "./App";
import { openFeatures } from "../../test/openFeatures";

// The clean sample: 9 tests, tags @slow (2), @smoke (4), @wip (2); outline.feature has none.
const clean = () =>
  buildTree(readAllEvents(path.resolve("test/fixtures/events", "dryrun-clean.jsonl")), "clean");
const off = (): FilterState => ({ include: [], exclude: [], hideNonMatching: false });
const smokeNotWip = (hideNonMatching = false): FilterState => ({
  include: ["smoke"],
  exclude: ["wip"],
  hideNonMatching,
});

function setup() {
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
  // main sends `project`, then `filter`
  const load = (filter: FilterState = off(), tree: TestNode = clean()) => {
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree,
      error: null,
    });
    send({ type: "filter", filter });
    openFeatures();
  };
  return { api, send, load };
}

const chip = (tag: string) =>
  screen.getAllByTestId("tag-chip").find((c) => c.dataset.tag === tag) as HTMLButtonElement;
const nodeByKey = (key: string) =>
  screen.queryAllByTestId("tree-node").find((n) => n.dataset.key === key);
const runButtonOf = (key: string) => nodeByKey(key)!.querySelector('[data-testid="run-node"]');
const checkbox = () => screen.getByTestId("hide-non-matching") as HTMLInputElement;
/** Whether the element holds this icon: every shape of the Lucide icon is in its svg. */
const drawn = (el: Element, icon: IconNode) => {
  const svg = el.querySelector("svg");
  return (
    !!svg &&
    icon.every(([, attrs]) => Object.values(attrs).every((v) => svg.innerHTML.includes(String(v))))
  );
};

/** jsdom has no IntersectionObserver: this one reports only what a test tells it to. */
class FakeIO {
  static all: FakeIO[] = [];
  targets: Element[] = [];
  constructor(
    public cb: IntersectionObserverCallback,
    public opts?: IntersectionObserverInit,
  ) {
    FakeIO.all.push(this);
  }
  observe(el: Element) {
    this.targets.push(el);
  }
  disconnect() {
    this.targets = [];
  }
}
const io = () => FakeIO.all.at(-1)!;
/** The browser's report: the chips of `tags` past the line, every other chip on it. */
const fold = (...tags: string[]) => {
  io().cb(
    io().targets.map((t) => ({
      target: t,
      isIntersecting: !tags.includes((t as HTMLElement).dataset.tag!),
    })) as unknown as IntersectionObserverEntry[],
    io() as unknown as IntersectionObserver,
  );
  flush();
};
const more = () => screen.queryByTestId("tag-more") as HTMLButtonElement | null;
const panel = () => screen.getByTestId("tag-panel");
/** jsdom has no Popover API: the browser's toggle of the panel, as its event. */
const toggle = (newState: "open" | "closed") => {
  panel().dispatchEvent(Object.assign(new Event("toggle"), { newState }));
  flush();
};
const panelChips = () => [...panel().querySelectorAll<HTMLButtonElement>("[data-testid=tag-chip]")];

beforeEach(() => vi.stubGlobal("IntersectionObserver", FakeIO));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("folding", () => {
  it("folds the chips past the line behind a count", () => {
    const { load } = setup();
    load();
    fold("smoke", "wip");
    expect(io().opts!.root).toBe(screen.getByTestId("tag-strip"));
    expect(io().targets).toEqual(screen.getAllByTestId("tag-chip"));
    expect(more()!.textContent).toBe("+2");
    expect(more()!.getAttribute("aria-label")).toBe("2 more tags");
    expect(chip("smoke").className).toContain("invisible");
    expect(chip("wip").className).toContain("invisible");
    expect(chip("slow").className).not.toContain("invisible");
  });

  it("counts the folded tags that are on", () => {
    const { load } = setup();
    load({ include: [], exclude: ["wip"], hideNonMatching: false });
    fold("smoke", "wip");
    expect(more()!.textContent).toBe("+2 · 1 on");
    expect(more()!.getAttribute("aria-label")).toBe("2 more tags, 1 on");
    fold("wip");
    expect(more()!.textContent).toBe("+1 · 1 on");
    expect(more()!.getAttribute("aria-label")).toBe("1 more tag, 1 on");
  });

  it("has no fold chip when every chip fits", () => {
    const { load } = setup();
    load();
    fold();
    expect(more()).toBeNull();
    expect(screen.getByTestId("filter-bar").className).not.toContain("flex-wrap");
    const strip = screen.getByTestId("tag-strip").className;
    for (const c of ["flex-wrap", "h-6", "overflow-clip"]) expect(strip).toContain(c);
  });

  it("moves the focus to the fold chip when the focused chip folds", () => {
    const { load } = setup();
    load();
    fold("wip");
    chip("smoke").focus();
    fold("smoke", "wip"); // turning it on made Clear and the counter appear, pushing it past the line
    expect(document.activeElement).toBe(more());
  });

  it("keeps one observer while only the filter changes", () => {
    const { send, load } = setup();
    load();
    fold("wip");
    const n = FakeIO.all.length;
    send({ type: "filter", filter: smokeNotWip() });
    expect(FakeIO.all.length).toBe(n);
    expect(more()!.textContent).toBe("+1 · 1 on");
  });

  it("opens a panel of every tag that cycles them", () => {
    const { api, load } = setup();
    load();
    fold("smoke", "wip");
    expect(more()!.getAttribute("popovertarget")).toBe("tag-panel");
    expect(panel().getAttribute("popover")).toBe("auto");
    expect(panel().getAttribute("aria-label")).toBe("All tags");
    expect(panelChips()).toEqual([]);
    expect(more()!.getAttribute("aria-expanded")).toBe("false");
    toggle("open");
    expect(more()!.getAttribute("aria-expanded")).toBe("true");
    expect(panelChips().map((c) => c.textContent)).toEqual(["@slow", "@smoke", "@wip"]);
    expect(panelChips().some((c) => c.className.includes("invisible"))).toBe(false);
    fireEvent.click(panelChips()[1]);
    expect(api.setFilter).toHaveBeenCalledWith({
      include: ["smoke"],
      exclude: [],
      hideNonMatching: false,
    });
    expect(panelChips()).toHaveLength(3);
    toggle("closed");
    expect(panelChips()).toEqual([]);
    expect(more()!.getAttribute("aria-expanded")).toBe("false");
  });

  it("closes the panel when nothing is folded", () => {
    const { load } = setup();
    load();
    fold("wip");
    toggle("open");
    fold();
    expect(more()).toBeNull();
    expect(panelChips()).toEqual([]);
  });

  it("disables the panel's chips during a run", () => {
    const { send, load } = setup();
    load();
    fold("wip");
    send({ type: "busy", busy: true });
    toggle("open");
    expect(panelChips().length).toBe(3);
    expect(panelChips().every((c) => c.disabled)).toBe(true);
    expect(more()!.disabled).toBe(false);
  });

  it("shows the next click on the fold chip in a tooltip", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      const { load } = setup();
      load();
      fold("wip");
      fireEvent.pointerEnter(more()!.parentElement!);
      vi.advanceTimersByTime(500);
      expect(document.querySelector("[role=tooltip]")!.textContent).toBe("Show all tags");
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("filter bar", () => {
  it("shows a chip per tag", () => {
    const { send, load } = setup();
    send({ type: "opening", root: "D:/p" }); // the project view, no tree yet
    expect(screen.getByTestId("project-path")).toBeTruthy();
    expect(screen.queryByTestId("filter-bar")).toBeNull();
    load();
    const chips = screen.getAllByTestId("tag-chip");
    expect(chips.map((c) => c.textContent)).toEqual(["@slow", "@smoke", "@wip"]);
    expect(chips.map((c) => c.dataset.tag)).toEqual(["slow", "smoke", "wip"]);
    expect(chips.map((c) => c.dataset.state)).toEqual(["off", "off", "off"]);
    expect(screen.getByRole("button", { name: "@smoke, not filtered" })).toBe(chip("smoke"));
    // A live region must exist before its text changes, or screen readers miss the first count.
    expect(screen.getByRole("status").textContent).toBe("");
    expect(chips.every((c) => c.tagName === "BUTTON" && c.getAttribute("type") === "button")).toBe(
      true,
    );
    expect(screen.queryByTestId("tag-counter")).toBeNull();
    expect(screen.queryByTestId("tag-clear")).toBeNull();
    expect(checkbox().checked).toBe(false);
    expect(screen.getByRole("checkbox", { name: "Hide non-matching" })).toBe(checkbox());
  });

  it("a chip click asks main for the next state", () => {
    const { api, send, load } = setup();
    load();
    fireEvent.click(chip("smoke"));
    expect(api.setFilter).toHaveBeenCalledWith({
      include: ["smoke"],
      exclude: [],
      hideNonMatching: false,
    });
    // the window waits for main's echo
    expect(chip("smoke").dataset.state).toBe("off");
    send({ type: "filter", filter: { include: ["smoke"], exclude: [], hideNonMatching: false } });
    expect(chip("smoke").dataset.state).toBe("include");
    fireEvent.click(chip("smoke"));
    expect(api.setFilter).toHaveBeenLastCalledWith({
      include: [],
      exclude: ["smoke"],
      hideNonMatching: false,
    });
  });

  it("shows include and exclude states by shape and colour, the counter and Clear", () => {
    const { api, load } = setup(); // clean sample, @smoke included and @wip excluded
    load(smokeNotWip(true));
    expect(chip("slow").dataset.state).toBe("off");
    expect(chip("smoke").dataset.state).toBe("include");
    expect(chip("wip").dataset.state).toBe("exclude");
    // R7.3: included = check + accent fill; excluded = minus + outline + strike-through; off = plain
    expect(drawn(chip("smoke"), Check)).toBe(true);
    expect(chip("smoke").className).toContain("bg-accent");
    expect(drawn(chip("wip"), Minus)).toBe(true);
    expect(chip("wip").className).toContain("line-through");
    expect(chip("wip").className).not.toContain("bg-accent");
    expect(chip("slow").querySelector("svg")).toBeNull();
    expect(chip("slow").className).not.toContain("line-through");
    for (const c of screen.getAllByTestId("tag-chip")) {
      expect(c.className).toContain("h-6");
      expect(c.className).toContain("rounded-sm");
    }
    expect(screen.getByRole("button", { name: "@smoke, included" })).toBe(chip("smoke"));
    expect(screen.getByRole("button", { name: "@wip, excluded" })).toBe(chip("wip"));
    fireEvent.click(screen.getByTestId("tag-clear"));
    expect(api.setFilter).toHaveBeenCalledWith({ include: [], exclude: [], hideNonMatching: true });
  });

  it("styles Clear, the counter and the checkbox with the tokens", () => {
    const { load } = setup();
    load(smokeNotWip());
    expect(screen.getByTestId("tag-clear").className).toContain("hover:bg-hover");
    expect(screen.getByTestId("tag-counter").parentElement!.className).toContain("text-muted");
    expect(checkbox().className).toContain("accent-accent");
    // a native checkbox in a 24 px label, so the whole line is the target
    expect(checkbox().closest("label")!.className).toContain("h-6");
  });

  it("shows the next click in a tooltip, not in a title attribute", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    try {
      const { load } = setup();
      load(smokeNotWip());
      for (const c of screen.getAllByTestId("tag-chip"))
        expect(c.hasAttribute("title")).toBe(false);
      fireEvent.pointerEnter(chip("smoke").parentElement!);
      vi.advanceTimersByTime(500);
      expect(document.querySelector("[role=tooltip]")!.textContent).toBe(
        "Included. Click to exclude.",
      );
      expect(chip("smoke").getAttribute("aria-describedby")).toBe(
        document.querySelector("[role=tooltip]")!.id,
      );
    } finally {
      vi.useRealTimers();
    }
  });

  it("mutes and hides non-matching tests", () => {
    const { send, load } = setup();
    load(smokeNotWip());
    expect(screen.getByTestId("tag-counter").textContent).toBe("2 of 9 tests match");
    expect(screen.getByRole("status").textContent).toBe("2 of 9 tests match");
    const matching = ["s:tests/basic.feature#1", "s:tests/slow.feature#2"];
    for (const n of screen.getAllByTestId("tree-node")) {
      const match = matching.includes(n.dataset.key!);
      if (n.dataset.kind === "scenario" || n.dataset.kind === "row") {
        expect(n.dataset.match).toBe(String(match));
        // R15.2: a non-matching row is muted, never faded
        expect(n.className.includes("text-muted")).toBe(!match);
        expect(n.className).not.toContain("opacity");
        expect(n.querySelector('[data-testid="run-node"]') !== null).toBe(match);
      }
    }
    // folders and features that hold a match stay lit; the others dim
    expect(nodeByKey("f:tests/basic.feature")!.dataset.match).toBe("true");
    expect(nodeByKey("d:tests")!.dataset.match).toBe("true");
    for (const key of ["f:tests/outline.feature", "o:tests/outline.feature#1"]) {
      expect(nodeByKey(key)!.dataset.match).toBe("false");
      expect(runButtonOf(key)).toBeNull();
    }
    expect(nodeByKey("s:tests/basic.feature#2")!.dataset.match).toBe("false");
    expect(runButtonOf("s:tests/basic.feature#2")).toBeNull();
    // a dimmed test can still be selected
    fireEvent.click(screen.getByText("failing match"));
    flush();
    expect(screen.getByRole("heading").textContent).toBe("failing match");

    send({ type: "filter", filter: smokeNotWip(true) });
    expect(checkbox().checked).toBe(true);
    expect(nodeByKey("s:tests/basic.feature#2")).toBeUndefined();
    // (the details pane still shows the selected test; the tree does not)
    expect(screen.getByTestId("tree").textContent).not.toContain("failing match");
    expect(nodeByKey("f:tests/outline.feature")).toBeUndefined();
    expect(screen.getByTestId("tree").textContent).not.toContain("outline.feature");
    expect(
      screen
        .getAllByTestId("tree-node")
        .filter((n) => n.dataset.kind === "scenario")
        .map((n) => n.dataset.key),
    ).toEqual(matching);
    expect(nodeByKey("f:tests/basic.feature")).toBeDefined();
  });

  it("sends plain data, never store proxies, which cannot cross IPC", () => {
    const { api, load } = setup();
    load({ include: ["smoke"], exclude: ["wip"], hideNonMatching: false });
    for (const tag of ["slow", "smoke", "wip"]) fireEvent.click(chip(tag));
    fireEvent.click(checkbox());
    fireEvent.click(screen.getByTestId("tag-clear"));
    expect(api.setFilter).toHaveBeenCalledTimes(5);
    for (const [arg] of api.setFilter.mock.calls as unknown as [FilterState][])
      expect(() => structuredClone(arg)).not.toThrow();
  });

  it("dims nothing and hides nothing while the filter is off", () => {
    const { load } = setup();
    load({ include: [], exclude: [], hideNonMatching: true });
    const nodes = screen.getAllByTestId("tree-node");
    expect(nodes).toHaveLength(14);
    expect(nodes.every((n) => n.dataset.match === "true")).toBe(true);
    expect(nodes.every((n) => !n.className.includes("text-muted"))).toBe(true);
    expect(screen.getAllByTestId("run-node")).toHaveLength(14);
  });

  it("toggles Hide non-matching through main", () => {
    const { api, load } = setup();
    load(smokeNotWip());
    fireEvent.click(checkbox());
    expect(api.setFilter).toHaveBeenCalledWith(smokeNotWip(true));
  });

  it("Run all is disabled when nothing matches", () => {
    const { send, load } = setup();
    load();
    expect((screen.getByTestId("run-all-button") as HTMLButtonElement).disabled).toBe(false);
    // @gone is no longer in the project, but the user has not pruned it yet
    send({ type: "filter", filter: { include: ["gone"], exclude: [], hideNonMatching: false } });
    expect(screen.getByTestId("tag-counter").textContent).toBe("0 of 9 tests match");
    expect(chip("gone").dataset.state).toBe("include");
    expect((screen.getByTestId("run-all-button") as HTMLButtonElement).disabled).toBe(true);
  });

  it("controls are disabled while busy or running", () => {
    const { send, load } = setup();
    load(smokeNotWip());
    const controls = () => [
      ...(screen.getAllByTestId("tag-chip") as HTMLButtonElement[]),
      screen.getByTestId("tag-clear") as HTMLButtonElement,
      checkbox(),
    ];
    expect(controls().some((c) => c.disabled)).toBe(false);
    send({ type: "busy", busy: true });
    expect(controls().every((c) => c.disabled)).toBe(true);
    send({ type: "busy", busy: false });
    expect(controls().some((c) => c.disabled)).toBe(false);
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: [],
      total: 2,
      startedAt: Date.now(),
      env: null,
      debug: false,
    });
    expect(controls().every((c) => c.disabled)).toBe(true);
  });

  it("is absent without a tree or without tags", () => {
    const { send, load } = setup();
    load(smokeNotWip());
    expect(screen.getByTestId("filter-bar")).toBeTruthy();
    // main still holds the old filter after a failed discovery
    send({ type: "project", project: null, tree: null, error: "Folder not found" });
    expect(screen.queryByTestId("filter-bar")).toBeNull();
    expect(screen.queryByTestId("tag-chip")).toBeNull();

    const untagged: TestNode = clean();
    const strip = (n: TestNode) => {
      n.tags = [];
      n.children.forEach(strip);
    };
    strip(untagged);
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: untagged,
      error: null,
    });
    send({ type: "filter", filter: off() });
    expect(screen.queryByTestId("filter-bar")).toBeNull();
    expect(screen.getAllByTestId("tree-node").every((n) => n.dataset.match === "true")).toBe(true);
  });
});
