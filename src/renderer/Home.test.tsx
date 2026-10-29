import { render, screen, cleanup, fireEvent } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AppEvent, RecentEntry } from "../shared/events";
import type { Api } from "../shared/ipc";
import type { TestNode } from "../shared/model";
import { App } from "./App";

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

afterEach(() => {
  cleanup();
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
      expect(screen.getByText("Pick a folder, or reopen one you used before.")).toBeTruthy();
      expect(button("open-button").textContent).toBe("Open folder…");
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
    const rows = screen.getAllByTestId("recent-row") as HTMLButtonElement[];
    expect(rows).toHaveLength(2);
    expect(rows[0].textContent).toContain("api");
    expect(rows[0].textContent).toContain("D:\\work\\api");
    expect(rows[1].textContent).toContain("old");
    expect(rows[1].textContent).toContain("E:\\usb\\old");
    expect(rows[0].disabled).toBe(false);
    expect(rows[1].disabled).toBe(true);
    expect(screen.getAllByTestId("recent-missing").map((e) => e.textContent)).toEqual([
      "not found",
    ]);
    expect(rows[1].textContent).toContain("not found");
    fireEvent.click(rows[0]);
    expect(api.openRecent).toHaveBeenCalledExactlyOnceWith("D:\\work\\api");
    const remove = screen.getAllByRole("button", { name: "Remove from list" });
    expect(remove).toHaveLength(2);
    expect(remove[1].getAttribute("title")).toBe("Remove from list");
    fireEvent.click(remove[1]);
    expect(api.removeRecent).toHaveBeenCalledExactlyOnceWith("E:\\usb\\old");
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

  it("the project view sits on the sheet under the toolbar; home has none", () => {
    const { send, open } = setup();
    expect(screen.queryByTestId("sheet")).toBeNull();
    open();
    send({ type: "notice", message: "Nothing to run" });
    const sheet = screen.getByTestId("sheet");
    for (const c of ["mx-(--sheet-inset)", "rounded-(--sheet-radius)", "bg-surface"])
      expect(sheet.className).toContain(c);
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
    expect(header.firstElementChild).toBe(button("home-button"));
    expect(button("home-button").textContent).toBe("Home");
  });
});
