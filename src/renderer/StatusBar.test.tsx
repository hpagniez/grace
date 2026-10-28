import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Api } from "../shared/ipc";
import { defaultUpdateSettings, type Release, type UpdateView } from "../shared/updates";
import { initialState, type AppState } from "./reduce";
import { StatusBar } from "./StatusBar";

afterEach(cleanup);

const release = (version: string, prerelease = true): Release => ({
  version,
  prerelease,
  publishedAt: "2026-10-29T10:00:00Z",
  url: `https://github.com/hpagniez/grace/releases/download/v${version}/Grace-win32-x64-${version}.zip`,
  size: 218_557_130,
  sha256: "a".repeat(64),
});
const view = (more: Partial<UpdateView> = {}): UpdateView => ({
  running: "1.0.0-rc.5",
  packaged: true,
  settings: defaultUpdateSettings("1.0.0-rc.5"),
  releases: [release("1.0.0-rc.6"), release("1.0.0-rc.5"), release("1.0.0-rc.4")],
  checking: false,
  error: null,
  backups: [],
  ...more,
});

function show(updates: UpdateView | null, more: Partial<AppState> = {}) {
  const api = { installVersion: vi.fn(async () => {}) };
  const openSettings = vi.fn();
  render(() => (
    <StatusBar
      state={{ ...initialState(), updates, ...more }}
      api={api as unknown as Api}
      openSettings={openSettings}
    />
  ));
  return { api, openSettings };
}
const text = (id: string) => screen.getByTestId(id).textContent;

describe("StatusBar", () => {
  it("shows the running version and Settings", () => {
    show(view({ releases: null }));
    expect(text("status-version")).toBe("Grace 1.0.0-rc.5");
    expect(text("settings-button")).toBe("⚙ Settings");
    expect(screen.queryByTestId("update-marker")).toBeNull();
  });

  it("shows the marker and Update while out of date on the channel", () => {
    show(view());
    expect(text("update-marker")).toBe("1.0.0-rc.6 available");
    expect(text("update-link")).toBe("Update");
  });

  it("keeps the marker for a skipped version", () => {
    show(view({ settings: { ...defaultUpdateSettings("1.0.0-rc.5"), skipped: "1.0.0-rc.6" } }));
    expect(text("update-marker")).toBe("1.0.0-rc.6 available");
  });

  it("shows no marker when the channel has nothing newer", () => {
    show(view({ settings: { ...defaultUpdateSettings("1.0.0-rc.5"), channel: "stable" } }));
    expect(screen.queryByTestId("update-marker")).toBeNull();
    expect(screen.queryByTestId("update-link")).toBeNull();
    cleanup();
    show(view({ releases: [release("1.0.0-rc.5"), release("1.0.0-rc.4")] }));
    expect(screen.queryByTestId("update-marker")).toBeNull();
  });

  it("names the marker with the running and the latest version", () => {
    show(view());
    expect(screen.getByRole("button", { name: "Grace 1.0.0-rc.5, 1.0.0-rc.6 available" })).toBe(
      screen.getByTestId("update-marker"),
    );
  });

  it("Update installs latest with the current data", () => {
    const { api } = show(view());
    fireEvent.click(screen.getByTestId("update-link"));
    expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.6", false);
  });

  it("Update is disabled while a run or a discovery is active (R6.4)", () => {
    const busy = "Finish or cancel the run first";
    show(view(), { busy: true });
    const link = () => screen.getByTestId("update-link") as HTMLButtonElement;
    expect([link().disabled, link().title]).toEqual([true, busy]);
    cleanup();
    show(view(), { run: { ...initialState().run, active: true } });
    expect([link().disabled, link().title]).toEqual([true, busy]);
    cleanup();
    show(view({ packaged: false }), { busy: true }); // the run comes first
    expect([link().disabled, link().title]).toEqual([true, busy]);
  });

  it("Update is disabled in a build that is not packaged (R13.1)", () => {
    show(view({ packaged: false }));
    const link = screen.getByTestId("update-link") as HTMLButtonElement;
    expect([link.disabled, link.title]).toEqual([true, "Updates install only in the packaged app"]);
  });

  it("Update is enabled when packaged and idle", () => {
    show(view());
    const link = screen.getByTestId("update-link") as HTMLButtonElement;
    expect([link.disabled, link.title]).toEqual([false, ""]);
  });

  it("the marker, the version and Settings open the settings", () => {
    const { openSettings } = show(view());
    for (const id of ["update-marker", "status-version", "settings-button"])
      fireEvent.click(screen.getByTestId(id));
    expect(openSettings).toHaveBeenCalledTimes(3);
  });

  it("shows nothing before main sends the update view", () => {
    show(null);
    expect(screen.queryByTestId("status-version")).toBeNull();
    expect(screen.getByRole("contentinfo")).toBeTruthy();
  });
});
