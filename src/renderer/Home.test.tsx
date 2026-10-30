import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { Folder, FolderOpen, TriangleAlert, X } from "lucide";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import { drawn } from "../../test/drawn";
import type { AppEvent, RecentEntry } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";
import { btnIconSmall, primary } from "./buttons";

const tree: TestNode = {
  key: "d:",
  kind: "folder",
  name: "p",
  file: "",
  line: null,
  refId: null,
  tags: [],
  children: [],
};

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    openRecent: vi.fn(async () => {}),
    removeRecent: vi.fn(async () => {}),
    closeProject: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (...es: AppEvent[]) => {
    for (const e of es) {
      emit(e);
      flush();
    }
  };
  const open = (root = "D:/p") =>
    send({ type: "project", project: { root, layout: "standalone" }, tree, error: null });
  return { api, send, open };
}

const button = (id: string) => screen.getByTestId(id) as HTMLButtonElement;
const home = () => screen.queryByTestId("home");
const recent = (entries: RecentEntry[]): AppEvent => ({ type: "recent", entries });
const list: RecentEntry[] = [
  { path: "D:\\work\\api", exists: true },
  { path: "E:\\usb\\old", exists: false },
];

/** The wrapper a Tooltip puts around its target, and the tooltip it opens. */
const wrapperOf = (el: HTMLElement) => el.closest("span.inline-flex") as HTMLElement;
const tip = () => document.querySelector('[role="tooltip"]');
const hover = (el: HTMLElement) => {
  fireEvent.pointerEnter(wrapperOf(el));
  vi.advanceTimersByTime(500);
};

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  document.getElementById("startup")?.remove();
});

describe("home screen", () => {
  it("draws nothing until main says what is open", () => {
    const startup = document.createElement("div");
    startup.id = "startup";
    document.body.append(startup);
    const { send } = setup();
    expect(home()).toBeNull();
    expect(screen.queryByTestId("project-path")).toBeNull();
    expect(document.getElementById("startup")).toBe(startup);
    send({ type: "opening", root: null });
    expect(home()).not.toBeNull();
    expect(document.getElementById("startup")).toBeNull();
  });

  it("a first start shows the home screen without a list", () => {
    const { send } = setup();
    send({ type: "project", project: null, tree: null, error: null });
    const check = () => {
      expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Open a Karate project");
      expect(
        screen.getByText(
          "Pick the folder that holds your .feature files. Grace brings its own Java and Karate.",
        ),
      ).toBeTruthy();
      expect(screen.queryByText(/reopen one you used before/)).toBeNull();
      expect(button("open-button").textContent).toBe("Open folder…");
      expect(button("open-button").className).toContain(primary);
      expect(drawn(button("open-button").querySelector("svg"), FolderOpen)).toBe(true);
      expect(screen.queryByTestId("recent-list")).toBeNull();
      expect(screen.queryByText("Recent folders")).toBeNull();
      expect(screen.queryByTestId("project-path")).toBeNull();
      expect(screen.queryByTestId("tree-message")).toBeNull();
    };
    check();
    send(recent([]));
    check();
  });

  it("the mascot is decorative", () => {
    const { send } = setup();
    send({ type: "project", project: null, tree: null, error: null });
    const mascot = screen.getByTestId("mascot") as HTMLImageElement;
    expect(mascot.tagName).toBe("IMG");
    expect(mascot.getAttribute("alt")).toBe("");
    expect(mascot.getAttribute("src")).toMatch(/mascot\.svg/);
  });

  it("an open project hides the mascot", () => {
    const { open } = setup();
    open();
    expect(screen.queryByTestId("mascot")).toBeNull();
  });

  it("Open folder… opens the folder dialog", () => {
    const { api, send } = setup();
    send({ type: "opening", root: null });
    fireEvent.click(button("open-button"));
    expect(api.openProject).toHaveBeenCalledOnce();
  });

  it("lists the recent folders", () => {
    const { api, send } = setup();
    send({ type: "opening", root: null }, recent(list));
    expect(screen.getByRole("heading", { level: 2 }).textContent).toBe("Recent folders");
    // with a list to reopen, the sentence says so
    expect(screen.getByText("Pick a folder, or reopen one you used before.")).toBeTruthy();
    const rows = screen.getAllByTestId("recent-row") as HTMLButtonElement[];
    expect(rows).toHaveLength(2);
    expect(rows[0].textContent).toContain("api");
    expect(rows[0].textContent).toContain("D:\\work\\api");
    expect(rows[1].textContent).toContain("old");
    expect(rows[1].textContent).toContain("E:\\usb\\old");
    expect(rows[0].disabled).toBe(false);
    expect(rows[1].disabled).toBe(true);
    expect(screen.getAllByTestId("recent-missing").map((e) => e.textContent)).toEqual([
      "Not found",
    ]);
    expect(rows[1].textContent).toContain("Not found");
    fireEvent.click(rows[0]);
    expect(api.openRecent).toHaveBeenCalledExactlyOnceWith("D:\\work\\api");
    const remove = screen.getAllByRole("button", { name: "Remove from list" });
    expect(remove).toHaveLength(2);
    fireEvent.click(remove[1]);
    expect(api.removeRecent).toHaveBeenCalledExactlyOnceWith("E:\\usb\\old");
  });

  it("shows the list in one surface box with a rule between rows (R12.3)", () => {
    const { send } = setup();
    send({ type: "opening", root: null }, recent(list));
    const box = screen.getByTestId("recent-list");
    for (const c of ["bg-surface", "rounded-lg", "border-line", "divide-line", "divide-y"])
      expect(box.className).toContain(c);
    const [ok, gone] = screen.getAllByTestId("recent-row");
    // a folder icon, the name in 500, the path in the mono muted below
    expect(drawn(ok.querySelector("svg"), Folder)).toBe(true);
    expect(within(ok).getByTestId("recent-name").className).toContain("font-medium");
    expect(within(ok).getByTestId("recent-name").className).not.toContain("text-muted");
    const path = within(ok).getByText("D:\\work\\api");
    for (const c of ["font-mono", "text-muted", "truncate"]) expect(path.className).toContain(c);
    // a flex column, so the tooltip's inline wrapper adds no line-box height under the name
    expect(path.closest("span.flex-col")).not.toBeNull();
    expect(ok.className).toContain("enabled:hover:bg-hover");
    expect(ok.className).toContain("focus-visible:-outline-offset-2"); // inset, inside a clipped list
    // the missing one: its name in muted, and nothing dimmed by opacity
    expect(within(gone).getByTestId("recent-name").className).toContain("text-muted");
    expect(gone.outerHTML).not.toMatch(/opacity/);
    expect(box.outerHTML).not.toMatch(/\b(zinc|red|amber|sky)-\d|dark:| title=/);
  });

  it("says Not found in a cancelled badge with the warning icon, and the row is disabled", () => {
    const { send } = setup();
    send({ type: "opening", root: null }, recent(list));
    const [ok, gone] = screen.getAllByTestId("recent-row") as HTMLButtonElement[];
    expect(within(ok).queryByTestId("recent-missing")).toBeNull();
    const badge = within(gone).getByTestId("recent-missing");
    expect(badge.textContent).toBe("Not found");
    expect(badge.className).toContain("bg-cancelled-tint");
    expect(badge.className).toContain("text-cancelled");
    expect(drawn(badge.querySelector("svg"), TriangleAlert)).toBe(true);
    expect(badge.querySelector("svg")!.getAttribute("width")).toBe("12");
    expect(gone.disabled).toBe(true);
  });

  it("explains a missing folder in a tooltip, though its row is disabled", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const { send } = setup();
    send({ type: "opening", root: null }, recent(list));
    const [ok, gone] = screen.getAllByTestId("recent-row");
    hover(ok);
    expect(tip()).toBeNull();
    hover(gone);
    expect(tip()!.textContent).toBe("Folder not found");
    // over its row: below, it would take the click meant for the next row
    expect((tip() as HTMLElement).dataset.side).toBe("over");
    expect(gone.hasAttribute("title")).toBe(false);
  });

  it("shows a path that is cut in a tooltip, and only then (R9.4)", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const { send } = setup();
    send({ type: "opening", root: null }, recent(list));
    const [ok, gone] = screen.getAllByTestId("recent-row");
    // taken before any tooltip opens: the tooltip repeats the text
    const okLine = within(ok).getByText("D:\\work\\api");
    const goneLine = within(gone).getByText("E:\\usb\\old");
    const cut = (el: HTMLElement) => {
      // jsdom has no layout: make the line look cut
      Object.defineProperty(el, "scrollWidth", { value: 300 });
      Object.defineProperty(el, "clientWidth", { value: 100 });
    };
    // it fits: nothing to say
    hover(okLine);
    expect(tip()).toBeNull();
    fireEvent.pointerLeave(wrapperOf(okLine));
    flush();
    cut(okLine);
    hover(okLine);
    expect(tip()!.textContent).toBe("D:\\work\\api");
    expect((tip() as HTMLElement).dataset.side).toBe("over");
    fireEvent.pointerLeave(wrapperOf(okLine));
    flush();
    // on a missing row the cut path wins over the row's own tooltip
    cut(goneLine);
    fireEvent.pointerEnter(wrapperOf(gone)); // the pointer enters the row, then its path
    fireEvent.pointerEnter(wrapperOf(goneLine));
    vi.advanceTimersByTime(500);
    expect(document.querySelectorAll('[role="tooltip"]')).toHaveLength(1); // one at a time
    expect(tip()!.textContent).toBe("E:\\usb\\old");
  });

  it("removes a row with a 24 px icon button that has a name and a tooltip (R10.3)", () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const { send } = setup();
    send({ type: "opening", root: null }, recent(list));
    const [first, second] = screen.getAllByTestId("recent-remove");
    for (const b of [first, second]) {
      expect(b.getAttribute("aria-label")).toBe("Remove from list");
      expect(b.hasAttribute("title")).toBe(false);
      expect(b.className).toContain(btnIconSmall);
      expect(b.className).toContain("size-6");
      expect(drawn(b.querySelector("svg"), X)).toBe(true);
      expect(b.textContent).toBe("");
    }
    hover(second);
    expect(tip()!.textContent).toBe("Remove from list");
  });

  it("removing a row moves the focus to Open folder…", () => {
    const { api, send } = setup();
    send({ type: "opening", root: null }, recent(list));
    const remove = screen.getAllByTestId("recent-remove");
    remove[0].focus();
    fireEvent.click(remove[0]);
    expect(api.removeRecent).toHaveBeenCalledWith("D:\\work\\api");
    expect(document.activeElement).toBe(button("open-button"));
    // main answers with a new list: the rows are rebuilt, the focus stays
    send(recent([list[1]]));
    expect(document.activeElement).toBe(button("open-button"));
  });

  it("names a folder by its last segment, whichever slash it uses", () => {
    const { send } = setup();
    send(
      { type: "opening", root: null },
      recent([
        { path: "D:/work/slash", exists: true },
        { path: "D:\\work\\back", exists: true },
        { path: "C:\\", exists: true },
      ]),
    );
    const names = screen.getAllByTestId("recent-name").map((e) => e.textContent);
    expect(names).toEqual(["slash", "back", "C:\\"]);
  });

  it("opening with no project shows the project view, not home", () => {
    const { send } = setup();
    send(
      { type: "opening", root: null },
      { type: "busy", busy: true },
      { type: "opening", root: "D:\\p" },
    );
    expect(home()).toBeNull();
    expect(screen.getByTestId("project-path").textContent).toBe("D:\\p");
    expect(button("cancel-button").disabled).toBe(false);
    expect(button("home-button").disabled).toBe(true);
    expect(screen.getByTestId("tree-message").textContent).toBe("Finding tests…");
  });

  it("a stale folder error does not show while the next folder opens", () => {
    const { send } = setup();
    send({ type: "project", project: null, tree: null, error: "Folder not found: D:\\x" });
    send({ type: "opening", root: "D:\\p" });
    expect(screen.getByTestId("tree-message").textContent).toBe("Finding tests…");
  });

  it("opening another folder keeps the open project on screen", () => {
    const { send, open } = setup();
    open();
    send({ type: "opening", root: "D:\\q" });
    expect(home()).toBeNull();
    expect(screen.getByTestId("project-path").textContent).toBe("D:/p");
  });

  it("a cancelled open goes back home with the notice", () => {
    const { send } = setup();
    send(
      { type: "opening", root: null },
      { type: "busy", busy: true },
      { type: "opening", root: "D:\\p" },
      { type: "project", project: null, tree: null, error: null },
      { type: "notice", message: "Cancelled" },
      { type: "opening", root: null },
      { type: "busy", busy: false },
    );
    expect(home()).not.toBeNull();
    expect(screen.getByTestId("notice").textContent).toBe("Cancelled");
  });

  it("a missing folder shows its reason on home", () => {
    const { send } = setup();
    send({ type: "project", project: null, tree: null, error: "Folder not found: D:\\x" });
    expect(home()).not.toBeNull();
    expect(screen.getByTestId("notice").textContent).toBe("Folder not found: D:\\x");
  });

  it("the notice is a warning Notice: a status with the cancelled tint and its icon", () => {
    const { send } = setup();
    send({ type: "opening", root: null }, { type: "notice", message: "Cancelled" });
    const box = screen.getByTestId("notice");
    expect(box.getAttribute("role")).toBe("status");
    expect(box.className).toContain("bg-cancelled-tint");
    expect(box.querySelector("svg")).not.toBeNull();
  });

  it("a notice shows above the column, inside home", () => {
    const { send } = setup();
    send({ type: "opening", root: null }, recent(list), { type: "notice", message: "Cancelled" });
    const column = screen.getByRole("heading", { level: 1 }).parentElement!;
    const notice = screen.getByTestId("notice");
    expect(screen.getByTestId("home").contains(notice)).toBe(true);
    expect(notice.compareDocumentPosition(column) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(column.contains(notice)).toBe(false);
  });

  it("home sits on the ground in both themes, with no sheet (R12.1)", () => {
    setup().send({ type: "opening", root: null });
    const cls = screen.getByTestId("home").className;
    expect(cls).toContain("bg-ground");
    expect(cls).not.toMatch(/dark:|bg-surface|home-bg|rounded|border/);
    expect(screen.queryByTestId("sheet")).toBeNull();
  });

  it("the project view sits on the sheet under the toolbar; home has none", () => {
    const { send, open } = setup();
    expect(screen.queryByTestId("sheet")).toBeNull();
    open();
    send({ type: "notice", message: "Nothing to run" });
    const sheet = screen.getByTestId("sheet");
    for (const c of [
      "relative", // so a positioned descendant (sr-only text) is clipped here, not scrolled with the page
      "mx-(--sheet-inset)",
      "rounded-(--sheet-radius)",
      "bg-surface",
      "overflow-clip",
    ])
      expect(sheet.className).toContain(c);
    expect(sheet.className).not.toContain("overflow-hidden"); // hidden scrolls on focus; clip does not
    for (const id of ["notice", "bottom-panel"])
      expect(sheet.contains(screen.getByTestId(id))).toBe(true);
    expect(sheet.querySelector("main")).not.toBeNull();
    expect(sheet.querySelector("section")).not.toBeNull(); // progress
    expect(sheet.contains(document.querySelector("header"))).toBe(false); // the toolbar stays on the ground
  });

  it("the app root is ground and ink, with no dark: classes", () => {
    setup().send({ type: "opening", root: null });
    const cls = screen.getByTestId("app").className;
    expect(cls).toContain("bg-ground");
    expect(cls).toContain("text-ink");
    expect(cls).not.toContain("dark:");
  });

  it("a dry-run error shows in the project view, not as a notice", () => {
    const { send } = setup();
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: null,
      error: "syntax error",
    });
    expect(home()).toBeNull();
    expect(screen.queryByTestId("notice")).toBeNull();
    expect(screen.getByTestId("tree-message").textContent).toBe("syntax error");
  });

  it("Home closes the project", () => {
    const { api, send, open } = setup();
    open();
    expect(home()).toBeNull();
    fireEvent.click(button("home-button"));
    expect(api.closeProject).toHaveBeenCalledOnce();
    send({ type: "busy", busy: true });
    expect(button("home-button").disabled).toBe(true);
  });

  it("Home is the first item of the toolbar", () => {
    const { open } = setup();
    open();
    const header = screen.getByRole("banner");
    expect(header.querySelector("button")).toBe(button("home-button")); // under the logo's tooltip
    expect(screen.getByRole("button", { name: "Home" })).toBe(button("home-button"));
  });
});
