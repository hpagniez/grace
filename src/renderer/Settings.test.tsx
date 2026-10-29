import { cleanup, fireEvent, render, screen, within } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../shared/ipc";
import type { Theme } from "../shared/theme";
import { defaultUpdateSettings, type Release, type UpdateView } from "../shared/updates";
import { initialState, type AppState } from "./reduce";
import { Settings } from "./Settings";
import { badge, TINT } from "./ui";

// jsdom has no modal dialog: open and close only flip the attribute.
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
});
// a tooltip shows after 500 ms
beforeEach(() => vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] }));
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

const release = (version: string, prerelease = true): Release => ({
  version,
  prerelease,
  publishedAt: "2026-10-29T10:00:00Z",
  url: `https://github.com/hpagniez/grace/releases/download/v${version}/Grace-win32-x64-${version}.zip`,
  size: 218_557_130,
  sha256: "a".repeat(64),
});
const settings = (more: Partial<UpdateView["settings"]> = {}) => ({
  ...defaultUpdateSettings(),
  ...more,
});
const view = (more: Partial<UpdateView> = {}): UpdateView => ({
  running: "1.0.0-rc.5",
  packaged: true,
  settings: settings(),
  releases: [release("1.0.0-rc.6"), release("1.0.0-rc.5"), release("1.0.0-rc.4")],
  checking: false,
  error: null,
  backups: [],
  ...more,
});
const copyOf = (version: string) => [{ version, savedAt: "2026-10-29T10:00:00Z" }];

function show(
  updates: UpdateView | null = view(),
  more: Partial<AppState> = {},
  theme: Theme = "system",
) {
  const api = {
    checkUpdates: vi.fn(async () => {}),
    installVersion: vi.fn(async () => {}),
    setUpdateSettings: vi.fn(async () => {}),
    getAppearance: vi.fn(async () => theme),
    setAppearance: vi.fn(async () => {}),
  };
  const back = vi.fn();
  render(() => (
    <Settings
      state={{ ...initialState(), updates, ...more }}
      api={api as unknown as Api}
      back={back}
    />
  ));
  return { api, back };
}
const rows = () => screen.getAllByTestId("version-row");
const versions = () => rows().map((r) => r.dataset.version);
const rowOf = (version: string) => rows().find((r) => r.dataset.version === version)!;
const action = (version: string) =>
  within(rowOf(version)).queryByTestId("version-action") as HTMLButtonElement | null;
const text = (id: string) => screen.getByTestId(id).textContent;
const checkbox = (name: string) => screen.getByRole("checkbox", { name }) as HTMLInputElement;
// Solid 2 holds a write until the next flush; a person's next click comes later
const click = (el: Element) => {
  fireEvent.click(el);
  flush();
};
const radio = (name: string) => screen.getByRole("radio", { name }) as HTMLInputElement;
/** The tooltip `el` shows when the pointer rests on it (R9.4), or "" when it has none. */
const tipOf = (el: HTMLElement) => {
  const wrap = el.parentElement!;
  fireEvent.pointerEnter(wrap);
  vi.advanceTimersByTime(500);
  const tip = document.querySelector('[role="tooltip"]')?.textContent ?? "";
  fireEvent.pointerLeave(wrap);
  return tip;
};

describe("Settings › Updates", () => {
  it("shows the running version, the marker and Update to the latest while out of date", () => {
    show();
    expect(text("settings-running")).toBe("Grace 1.0.0-rc.5");
    expect(text("settings-marker")).toBe("1.0.0-rc.6 available");
    expect(text("settings-update")).toBe("Update to 1.0.0-rc.6");
  });

  it("shows neither the marker nor Update when up to date", () => {
    show(view({ releases: [release("1.0.0-rc.5"), release("1.0.0-rc.4")] }));
    expect(screen.queryByTestId("settings-marker")).toBeNull();
    expect(screen.queryByTestId("settings-update")).toBeNull();
  });

  it("Update to <latest> installs it with the current data, without asking", () => {
    const { api } = show(view({ backups: copyOf("1.0.0-rc.6") }));
    click(screen.getByTestId("settings-update"));
    expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.6", false);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("goes back with the Back button and with Escape", () => {
    const { back } = show();
    click(screen.getByRole("button", { name: "Back" }));
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(back).toHaveBeenCalledTimes(2);
  });

  it("leaves Escape to an open dialog", () => {
    const { back } = show();
    click(action("1.0.0-rc.4")!);
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(back).not.toHaveBeenCalled();
  });

  it("leaves Escape to a tooltip, which closes alone (R9.4)", () => {
    const { back } = show(view({ packaged: false }));
    const update = screen.getByTestId("settings-update");
    fireEvent.pointerEnter(update.parentElement!);
    vi.advanceTimersByTime(500);
    expect(document.querySelector('[role="tooltip"]')).not.toBeNull();
    fireEvent.keyDown(document.body, { key: "Escape" });
    flush();
    expect(document.querySelector('[role="tooltip"]')).toBeNull();
    expect(back).not.toHaveBeenCalled();
    // an Escape another handler already took is not this page's either
    const taken = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    taken.preventDefault();
    document.body.dispatchEvent(taken);
    expect(back).not.toHaveBeenCalled();
    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(back).toHaveBeenCalledOnce();
  });
});

describe("the version table (R6.3)", () => {
  it("lists the releases on the channel and the running version, newest first", () => {
    show();
    expect(versions()).toEqual(["1.0.0-rc.6", "1.0.0-rc.5", "1.0.0-rc.4"]);
    expect(within(rowOf("1.0.0-rc.6")).getByTestId("version-date").textContent).toBe("2026-10-29");
    expect(rowOf("1.0.0-rc.6").textContent).toContain("pre-release");
  });

  it("adds the running version when the channel does not list it", () => {
    show(view({ running: "1.0.0-rc.3", settings: settings() }));
    expect(versions()).toEqual(["1.0.0-rc.6", "1.0.0-rc.5", "1.0.0-rc.4", "1.0.0-rc.3"]);
    expect(rowOf("1.0.0-rc.3").textContent).toContain("installed");
    expect(rowOf("1.0.0-rc.3").textContent).toContain("pre-release");
    expect(action("1.0.0-rc.3")).toBeNull();
    expect(action("1.0.0-rc.5")!.textContent).toBe("Install");
  });

  it("hides pre-releases on the stable channel but keeps the running row", () => {
    show(
      view({
        settings: settings({ channel: "stable" }),
        releases: [release("1.0.0", false), release("1.0.0-rc.5"), release("1.0.0-rc.4")],
      }),
    );
    expect(versions()).toEqual(["1.0.0", "1.0.0-rc.5"]);
    expect(rowOf("1.0.0").textContent).not.toContain("pre-release");
    expect(rowOf("1.0.0-rc.5").textContent).toContain("installed");
  });

  it("tags the running row installed, with no button, and a row with a saved copy data saved", () => {
    show(view({ backups: copyOf("1.0.0-rc.4") }));
    expect(rowOf("1.0.0-rc.5").textContent).toContain("installed");
    expect(action("1.0.0-rc.5")).toBeNull();
    expect(rowOf("1.0.0-rc.4").textContent).toContain("data saved");
    expect(rowOf("1.0.0-rc.6").textContent).not.toContain("data saved");
    expect(rowOf("1.0.0-rc.6").textContent).not.toContain("installed");
  });

  it("offers Install above the running version and Roll back below it", () => {
    show();
    expect(action("1.0.0-rc.6")!.textContent).toBe("Install");
    expect(action("1.0.0-rc.4")!.textContent).toBe("Roll back");
  });

  it("shows an empty date for a release without one", () => {
    show(
      view({ releases: [{ ...release("1.0.0-rc.6"), publishedAt: "" }, release("1.0.0-rc.5")] }),
    );
    expect(within(rowOf("1.0.0-rc.6")).getByTestId("version-date").textContent).toBe("");
  });

  it("holds the running version only before a check", () => {
    show(view({ releases: null }));
    expect(versions()).toEqual(["1.0.0-rc.5"]);
    expect(screen.getByText("Check for updates to see other versions")).toBeTruthy();
    expect(screen.queryByTestId("settings-update")).toBeNull();
  });

  it("does not ask for a check after one", () => {
    show();
    expect(screen.queryByText("Check for updates to see other versions")).toBeNull();
  });
});

describe("switching (R6.4, R6.5, R7)", () => {
  it("Install on a newer version without a saved copy installs it at once", () => {
    const { api } = show();
    click(action("1.0.0-rc.6")!);
    expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.6", false);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Install on a newer version with a saved copy asks which data first", () => {
    const { api } = show(view({ backups: copyOf("1.0.0-rc.6") }));
    click(action("1.0.0-rc.6")!);
    expect(api.installVersion).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Install 1.0.0-rc.6?" })).toBeTruthy();
    click(radio("Use the data you had in 1.0.0-rc.6 (2026-10-29)"));
    click(screen.getByRole("button", { name: "Install and restart" }));
    expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.6", true);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Roll back opens the confirmation, and Roll back and restart switches", () => {
    const { api } = show(view({ backups: copyOf("1.0.0-rc.4") }));
    click(action("1.0.0-rc.4")!);
    expect(api.installVersion).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Roll back to 1.0.0-rc.4?" })).toBeTruthy();
    click(screen.getByRole("button", { name: "Roll back and restart" }));
    expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.4", true);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("Roll back without a saved copy asks main to keep the data", () => {
    const { api } = show();
    click(action("1.0.0-rc.4")!);
    click(screen.getByRole("button", { name: "Roll back and restart" }));
    expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.4", false);
  });

  it("Cancel closes the confirmation and installs nothing", () => {
    const { api } = show();
    click(action("1.0.0-rc.4")!);
    click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(api.installVersion).not.toHaveBeenCalled();
  });

  it("gives focus back to the row's button when the dialog closes", () => {
    show();
    const roll = action("1.0.0-rc.4")!;
    roll.focus();
    click(roll);
    expect(document.activeElement).not.toBe(roll);
    click(screen.getByRole("button", { name: "Cancel" }));
    expect(document.activeElement).toBe(roll);
  });

  const buttons = () => [
    screen.getByTestId("settings-update"),
    action("1.0.0-rc.6")!,
    action("1.0.0-rc.4")!,
  ];
  const states = () => buttons().map((b) => [(b as HTMLButtonElement).disabled, tipOf(b)]);
  const all = (why: string) => [true, true, true].map((d) => [d, why]);

  it("disables every switch button while a run or a discovery is active", () => {
    const why = "Finish or cancel the run first";
    show(view(), { busy: true });
    expect(states()).toEqual(all(why));
    cleanup();
    show(view(), { run: { ...initialState().run, active: true } });
    expect(states()).toEqual(all(why));
  });

  it("disables them in a build that is not packaged, a run coming first", () => {
    show(view({ packaged: false }));
    expect(states()).toEqual(all("Updates install only in the packaged app"));
    cleanup();
    show(view({ packaged: false }), { busy: true });
    expect(states()).toEqual(all("Finish or cancel the run first"));
  });

  it("enables them when packaged and idle", () => {
    show();
    expect(states()).toEqual([false, false, false].map((d) => [d, ""]));
  });
});

describe("the settings (R2, R5, R6.2)", () => {
  it("selects the channel the settings hold", () => {
    show();
    expect(radio("Stable and pre-releases").checked).toBe(true);
    expect(radio("Stable releases only").checked).toBe(false);
  });

  it("changing the channel saves it", () => {
    const { api } = show();
    click(radio("Stable releases only"));
    expect(api.setUpdateSettings).toHaveBeenCalledWith({ channel: "stable" });
    click(radio("Stable and pre-releases"));
    expect(api.setUpdateSettings).toHaveBeenLastCalledWith({ channel: "prerelease" });
  });

  it("saves the startup check and auto-update, one key at a time", () => {
    const { api } = show();
    const check = checkbox("Check for updates");
    const auto = checkbox("Update automatically, without asking");
    expect([check.checked, auto.checked]).toEqual([true, false]);
    click(auto);
    expect(api.setUpdateSettings).toHaveBeenLastCalledWith({ autoUpdate: true });
    click(check);
    expect(api.setUpdateSettings).toHaveBeenLastCalledWith({ checkAtStartup: false });
  });

  it("disables auto-update while the startup check is off, keeping its value", () => {
    show(view({ settings: settings({ checkAtStartup: false, autoUpdate: true }) }));
    const auto = checkbox("Update automatically, without asking");
    expect([auto.disabled, auto.checked]).toEqual([true, true]);
  });

  it("checks for updates on request and shows Checking… meanwhile", () => {
    const { api } = show();
    click(screen.getByRole("button", { name: "Check for updates" }));
    expect(api.checkUpdates).toHaveBeenCalledTimes(1);
    cleanup();
    show(view({ checking: true }));
    expect(text("settings-check")).toBe("Checking…");
  });

  it("says when it last checked, if it ever did", () => {
    const ago = new Date(Date.now() - 2 * 60_000).toISOString();
    show(view({ settings: settings({ lastChecked: ago }) }));
    expect(text("settings-last-checked")).toBe("Last checked 2 minutes ago");
    cleanup();
    show();
    expect(screen.queryByTestId("settings-last-checked")).toBeNull();
  });

  it("shows a failed check's reason with the last good check", () => {
    show(
      view({
        error: "Couldn't check for updates: offline",
        settings: settings({ lastChecked: new Date().toISOString() }),
      }),
    );
    expect(text("settings-error")).toBe("Couldn't check for updates: offline");
    expect(text("settings-last-checked")).toContain("Last checked");
  });

  it("shows no error after a good check", () => {
    show();
    expect(screen.queryByTestId("settings-error")).toBeNull();
  });

  it("shows only the header and the section's title before main sends the update view", () => {
    show(null);
    expect(screen.getByRole("heading", { name: "Settings" })).toBeTruthy();
    expect(screen.queryByTestId("version-row")).toBeNull();
  });
});

describe("the look (spec R21)", () => {
  it("puts the page on a sheet under a header on the ground", () => {
    show();
    const page = screen.getByTestId("settings");
    const header = page.querySelector("header")!;
    expect(header.className).toContain("min-h-11");
    expect(header.className).not.toContain("bg-"); // the ground shows through
    const sheet = header.nextElementSibling!;
    for (const c of ["bg-surface", "rounded-(--sheet-radius)", "mx-(--sheet-inset)"])
      expect(sheet.className).toContain(c);
    expect(page.outerHTML).not.toMatch(/dark:|zinc-|amber-|sky-|green-|red-|title=/);
  });

  it("lists the sections in the selectable row style, the open one selected", () => {
    show();
    const entry = within(screen.getByRole("navigation", { name: "Settings sections" })).getByText(
      "Updates",
    );
    expect(entry.getAttribute("aria-current")).toBe("page");
    for (const c of ["h-7", "bg-selected", "shadow-[inset_2px_0_0_var(--color-accent)]"])
      expect(entry.className).toContain(c);
  });

  it("titles the section in text-lg", () => {
    show();
    const title = screen.getByRole("heading", { name: "Updates" });
    expect(title.className).toContain("text-lg");
    expect(screen.getByRole("region", { name: "Updates" })).toBeTruthy();
  });

  it("tags the versions with tinted badges", () => {
    show(view({ backups: copyOf("1.0.0-rc.4") }));
    const tag = (version: string, word: string) =>
      within(rowOf(version)).getByText(word).closest("span")!.className;
    expect(tag("1.0.0-rc.6", "pre-release")).toBe(`${badge} ${TINT.skipped}`);
    expect(tag("1.0.0-rc.5", "installed")).toBe(`${badge} ${TINT.passed}`);
    expect(tag("1.0.0-rc.4", "data saved")).toBe(`${badge} ${TINT.running}`);
    expect(screen.getByTestId("settings-marker").className).toBe(`${badge} ${TINT.running}`);
  });

  it("shows a failed check's reason as an error message", () => {
    show(view({ error: "Couldn't check for updates: offline" }));
    expect(screen.getByTestId("settings-error").className).toContain("text-failed");
  });

  it("keeps the dialog's words while it fades out, and asks afresh the next time", () => {
    show(view({ backups: copyOf("1.0.0-rc.6") }));
    click(action("1.0.0-rc.6")!);
    click(radio("Use the data you had in 1.0.0-rc.6 (2026-10-29)"));
    click(screen.getByRole("button", { name: "Cancel" }));
    const dialog = screen.getByTestId("switch-dialog") as HTMLDialogElement;
    expect(dialog.open).toBe(false);
    expect(dialog.textContent).toContain("Install 1.0.0-rc.6?");
    click(action("1.0.0-rc.6")!);
    expect(radio("Keep your current data").checked).toBe(true);
  });
});

describe("Settings › Appearance (spec R8)", () => {
  const section = (name: string) => screen.getByRole("button", { name });
  /** The page after the saved theme has arrived: Solid holds a write until the next flush. */
  const settled = async () => {
    await Promise.resolve();
    await Promise.resolve();
    flush();
  };
  const open = async (theme: Theme = "system") => {
    const shown = show(view(), {}, theme);
    click(section("Appearance"));
    await settled();
    return shown;
  };

  it("opens on Updates, and the list switches to Appearance and back", async () => {
    show();
    expect(section("Updates").getAttribute("aria-current")).toBe("page");
    expect(section("Appearance").getAttribute("aria-current")).toBeNull();
    expect(screen.queryByRole("radio", { name: "Dark" })).toBeNull();
    click(section("Appearance"));
    expect(section("Appearance").getAttribute("aria-current")).toBe("page");
    expect(section("Updates").getAttribute("aria-current")).toBeNull();
    const title = screen.getByRole("heading", { name: "Appearance" });
    expect(title.className).toContain("text-lg");
    expect(screen.getByRole("region", { name: "Appearance" })).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Updates" })).toBeNull();
    click(section("Updates"));
    expect(screen.getByRole("region", { name: "Updates" })).toBeTruthy();
    expect(screen.queryByRole("region", { name: "Appearance" })).toBeNull();
  });

  it("lists the three themes in a Theme group, Follow Windows checked by default", async () => {
    const { api } = await open();
    const group = screen.getByRole("group", { name: "Theme" });
    expect(
      within(group)
        .getAllByRole("radio")
        .map((r) => (r as HTMLInputElement).value),
    ).toEqual(["system", "light", "dark"]);
    expect(
      within(group)
        .getAllByRole("radio")
        .map((r) => [r.parentElement!.textContent, (r as HTMLInputElement).checked]),
    ).toEqual([
      ["Follow Windows", true],
      ["Light", false],
      ["Dark", false],
    ]);
    expect(api.getAppearance).toHaveBeenCalledOnce();
    expect(api.setAppearance).not.toHaveBeenCalled();
  });

  it("selects the saved theme once main answers", async () => {
    await open("dark");
    expect([radio("Follow Windows"), radio("Light"), radio("Dark")].map((r) => r.checked)).toEqual([
      false,
      false,
      true,
    ]);
  });

  it("choosing a theme saves it and selects it at once", async () => {
    const { api } = await open();
    click(radio("Dark"));
    expect(api.setAppearance).toHaveBeenLastCalledWith("dark");
    expect(radio("Dark").checked).toBe(true);
    expect(radio("Follow Windows").checked).toBe(false);
    click(radio("Follow Windows"));
    expect(api.setAppearance).toHaveBeenLastCalledWith("system");
    click(radio("Light"));
    expect(api.setAppearance).toHaveBeenLastCalledWith("light");
    expect(api.setAppearance).toHaveBeenCalledTimes(3);
  });

  it("styles the radios like the channel's, with the accent", async () => {
    await open();
    for (const name of ["Follow Windows", "Light", "Dark"]) {
      expect(radio(name).className).toContain("accent-accent");
      expect(radio(name).parentElement!.className).toContain("h-6");
    }
  });
});
