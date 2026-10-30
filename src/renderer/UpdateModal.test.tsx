import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { createStore, flush } from "solid-js";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { Api } from "../shared/ipc";
import { defaultUpdateSettings, type Release, type UpdateView } from "../shared/updates";
import { initialState, type AppState } from "./reduce";
import { btnDialog, primaryDialog } from "./buttons";
import { dialog as dialogStyle } from "./ui";
import { UpdateModal } from "./UpdateModal";

// jsdom has no modal dialog: open and close only flip the attribute.
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
});
afterEach(cleanup);

const release = (version: string): Release => ({
  version,
  prerelease: true,
  publishedAt: "2026-10-29T10:00:00Z",
  url: `https://github.com/hpagniez/grace/releases/download/v${version}/Grace-win32-x64-${version}.zip`,
  size: 218_557_130,
  sha256: "a".repeat(64),
});
const view = (): UpdateView => ({
  running: "1.0.0-rc.5",
  packaged: true,
  settings: defaultUpdateSettings(),
  releases: [release("1.0.0-rc.6"), release("1.0.0-rc.5")],
  checking: false,
  error: null,
  backups: [],
});

function setup(more: Partial<AppState> = {}) {
  const [state, setState] = createStore<AppState>({
    ...initialState(),
    updates: view(),
    updatePrompt: release("1.0.0-rc.6"),
    ...more,
  });
  const api = {
    skipVersion: vi.fn(async () => {}),
    setUpdateSettings: vi.fn(async () => {}),
    installVersion: vi.fn(async () => {}),
    openWhatsNew: vi.fn(async () => {}),
  };
  // what App passes: the store's dismissPrompt
  const dismiss = vi.fn(() => setState((d) => void (d.updatePrompt = null)));
  render(() => (
    <>
      <button type="button" data-testid="outside">
        outside
      </button>
      <UpdateModal state={state} api={api as unknown as Api} dismiss={dismiss} />
    </>
  ));
  return { api, dismiss, setState };
}
const dialog = () => screen.getByTestId("update-modal") as HTMLDialogElement;
const button = (name: string) => screen.getByRole("button", { name }) as HTMLButtonElement;
const checkbox = () => screen.getByRole("checkbox") as HTMLInputElement;

describe("UpdateModal", () => {
  it("announces the version, the size and the backup (R4.2)", () => {
    setup();
    expect(dialog().open).toBe(true);
    const title = screen.getByRole("heading");
    expect(title.textContent).toBe("Grace 1.0.0-rc.6 is available");
    expect(dialog().getAttribute("aria-labelledby")).toBe(title.id);
    // 218_557_130 bytes: Math.round(bytes / 1e6)
    expect(dialog().textContent).toContain(
      "You have 1.0.0-rc.5. Download: 219 MB. Your data is backed up before Grace restarts.",
    );
    expect(button("What's new")).toBeTruthy();
    expect(checkbox().checked).toBe(false);
    expect(screen.getByText("Update automatically at startup from now on")).toBeTruthy();
  });

  it("focuses Update and restart (R4.2)", () => {
    setup();
    expect(document.activeElement).toBe(button("Update and restart"));
  });

  it("is closed while there is no prompt, and opens when one arrives", () => {
    const { setState } = setup({ updatePrompt: null });
    expect(dialog().open).toBe(false);
    setState((d) => void (d.updatePrompt = release("1.0.0-rc.6")));
    flush();
    expect(dialog().open).toBe(true);
    expect(dialog().textContent).toContain("Grace 1.0.0-rc.6 is available");
  });

  it("Skip this version saves it and closes", () => {
    const { api, dismiss } = setup();
    fireEvent.click(button("Skip this version"));
    flush();
    expect(api.skipVersion).toHaveBeenCalledWith("1.0.0-rc.6");
    expect(dismiss).toHaveBeenCalledOnce();
    expect(api.installVersion).not.toHaveBeenCalled();
    expect(dialog().open).toBe(false);
  });

  it("Skip this version ignores the checkbox (R4.4)", () => {
    const { api } = setup();
    fireEvent.click(checkbox());
    expect(checkbox().checked).toBe(true);
    fireEvent.click(button("Skip this version"));
    expect(api.setUpdateSettings).not.toHaveBeenCalled();
    expect(api.skipVersion).toHaveBeenCalledWith("1.0.0-rc.6");
  });

  it("Update and restart installs latest with the current data, and the modal gives way", async () => {
    const { api, dismiss } = setup();
    fireEvent.click(button("Update and restart"));
    flush();
    await vi.waitFor(() => expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.6", false));
    expect(api.setUpdateSettings).not.toHaveBeenCalled();
    expect(api.skipVersion).not.toHaveBeenCalled();
    expect(dismiss).toHaveBeenCalledOnce();
    expect(dialog().open).toBe(false);
  });

  it("Update and restart saves auto-update first when the box is ticked (R4.4)", async () => {
    const { api } = setup();
    fireEvent.click(checkbox());
    fireEvent.click(button("Update and restart"));
    await vi.waitFor(() => expect(api.installVersion).toHaveBeenCalledWith("1.0.0-rc.6", false));
    expect(api.setUpdateSettings).toHaveBeenCalledWith({ autoUpdate: true });
    expect(api.setUpdateSettings.mock.invocationCallOrder[0]).toBeLessThan(
      api.installVersion.mock.invocationCallOrder[0],
    );
  });

  it("Escape closes it and saves nothing (R4.5)", () => {
    const { api, dismiss } = setup();
    fireEvent.click(checkbox());
    dialog().removeAttribute("open"); // Escape: the browser closes it and fires `close`
    fireEvent(dialog(), new Event("close"));
    flush();
    expect(dismiss).toHaveBeenCalledOnce();
    for (const call of Object.values(api)) expect(call).not.toHaveBeenCalled();
  });

  it("What's new opens the release page and keeps the modal", () => {
    const { api, dismiss } = setup();
    fireEvent.click(button("What's new"));
    expect(api.openWhatsNew).toHaveBeenCalledWith("1.0.0-rc.6");
    expect(dismiss).not.toHaveBeenCalled();
    expect(dialog().open).toBe(true);
  });

  it("gives way to the progress dialog", () => {
    const { setState } = setup();
    setState(
      (d) =>
        void (d.updateProgress = {
          version: "1.0.0-rc.6",
          rollback: false,
          step: "download",
          received: 0,
          total: 1,
        }),
    );
    flush();
    expect(dialog().open).toBe(false);
  });

  it("gives focus back to where it was when it closes (R4.6)", () => {
    const { setState } = setup({ updatePrompt: null });
    const outside = screen.getByTestId("outside");
    outside.focus();
    setState((d) => void (d.updatePrompt = release("1.0.0-rc.6")));
    flush();
    expect(document.activeElement).toBe(button("Update and restart"));
    fireEvent.click(button("Skip this version"));
    flush();
    expect(document.activeElement).toBe(outside);
  });

  it("is a dialog-style box: a 16 px title, Update and restart first, Skip this version last", () => {
    setup();
    expect(dialog().className).toContain(dialogStyle);
    expect(screen.getByRole("heading").className).toContain("text-lg");
    const update = button("Update and restart");
    const skip = button("Skip this version");
    expect(update.className).toContain(primaryDialog);
    expect(skip.className).toContain(btnDialog);
    expect(update.compareDocumentPosition(skip) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(dialog().outerHTML).not.toMatch(/dark:|zinc-|sky-/);
  });

  it("keeps its words while it fades out, and starts afresh with the next prompt (R5.2)", () => {
    const { setState } = setup();
    fireEvent.click(checkbox());
    dialog().removeAttribute("open"); // Escape
    fireEvent(dialog(), new Event("close"));
    flush();
    expect(dialog().textContent).toContain("Grace 1.0.0-rc.6 is available");
    setState((d) => void (d.updatePrompt = release("1.0.0-rc.7")));
    flush();
    expect(dialog().textContent).toContain("Grace 1.0.0-rc.7 is available");
    expect(checkbox().checked).toBe(false);
  });
});
