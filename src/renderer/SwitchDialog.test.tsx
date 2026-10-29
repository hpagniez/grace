import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { Backup, Release } from "../shared/updates";
import { btnDialog, dangerDialog, primaryDialog } from "./buttons";
import { SwitchDialog } from "./SwitchDialog";
import { dialog as dialogStyle } from "./ui";

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
// local noon, so the local date is the same in every time zone
const copy = (version: string): Backup => ({
  version,
  savedAt: new Date(2026, 9, 29, 12).toISOString(),
});

function show(target: string, backup: Backup | null) {
  const onConfirm = vi.fn();
  const onCancel = vi.fn();
  render(() => (
    <SwitchDialog
      open
      target={release(target)}
      running="1.0.0-rc.5"
      backup={backup}
      onConfirm={onConfirm}
      onCancel={onCancel}
    />
  ));
  return { onConfirm, onCancel };
}

describe("SwitchDialog rollback (R7)", () => {
  it("says what the saved copy restores and warns about the backup of the running version", () => {
    show("1.0.0-rc.4", copy("1.0.0-rc.4"));
    expect(screen.getByRole("dialog").getAttribute("open")).toBe("");
    expect(screen.getByRole("heading", { name: "Roll back to 1.0.0-rc.4?" })).toBeTruthy();
    expect(
      screen.getByText(
        "Grace backs up your current data, then restores the data saved when you last used 1.0.0-rc.4 (2026-10-29).",
      ),
    ).toBeTruthy();
    expect(screen.getByTestId("switch-warning").textContent).toContain(
      "Going back to an older version may break your data. Runs, settings and secrets added since then are not in the restored copy; they stay in the backup of 1.0.0-rc.5, and come back if you install 1.0.0-rc.5 again and use its saved data.",
    );
  });

  it("focuses Cancel", () => {
    show("1.0.0-rc.4", copy("1.0.0-rc.4"));
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
  });

  it("says the target starts from the current data when no copy exists", () => {
    show("1.0.0-rc.4", null);
    expect(
      screen.getByText(
        "Grace backs up your current data. 1.0.0-rc.4 has no saved data on this computer, so it starts from your current data, which it may not read correctly.",
      ),
    ).toBeTruthy();
    // only the first sentence of the warning
    expect(screen.getByTestId("switch-warning").textContent).toContain(
      "Going back to an older version may break your data.",
    );
    expect(screen.getByTestId("switch-warning").textContent).not.toContain("Runs, settings");
  });

  it("Roll back and restart restores the copy when there is one", () => {
    const { onConfirm } = show("1.0.0-rc.4", copy("1.0.0-rc.4"));
    fireEvent.click(screen.getByRole("button", { name: "Roll back and restart" }));
    expect(onConfirm).toHaveBeenCalledWith(true);
  });

  it("Roll back and restart keeps the data when there is no copy", () => {
    const { onConfirm } = show("1.0.0-rc.4", null);
    fireEvent.click(screen.getByRole("button", { name: "Roll back and restart" }));
    expect(onConfirm).toHaveBeenCalledWith(false);
  });

  it("Cancel and Escape cancel", () => {
    const { onCancel } = show("1.0.0-rc.4", null);
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    // Escape: the browser closes the dialog, then fires close
    screen.getByRole("dialog").removeAttribute("open");
    fireEvent(screen.getByRole("dialog", { hidden: true }), new Event("close"));
    expect(onCancel).toHaveBeenCalledTimes(2);
  });
});

describe("SwitchDialog look (spec R19.1)", () => {
  const confirm = () => screen.getByTestId("switch-confirm");
  const cancel = () => screen.getByTestId("switch-cancel");

  it("is a dialog-style box with a 16 px title, the action first and Cancel last", () => {
    show("1.0.0-rc.6", copy("1.0.0-rc.6"));
    const dialog = screen.getByRole("dialog");
    expect(dialog.className).toContain(dialogStyle);
    expect(screen.getByRole("heading").className).toContain("text-lg");
    expect(confirm().className).toContain(primaryDialog);
    expect(cancel().className).toContain(btnDialog);
    expect(
      confirm().compareDocumentPosition(cancel()) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(dialog.outerHTML).not.toMatch(/dark:|zinc-|amber-|sky-/);
  });

  it("styles Roll back and restart as a danger and its warning as a notice", () => {
    show("1.0.0-rc.4", copy("1.0.0-rc.4"));
    expect(confirm().className).toContain(dangerDialog);
    const warning = screen.getByTestId("switch-warning");
    expect(warning.getAttribute("role")).toBe("status");
    expect(warning.className).toContain("bg-cancelled-tint");
  });
});

describe("SwitchDialog data choice on an install (R7.6)", () => {
  it("offers the saved data or the current data, the current data selected", () => {
    show("1.0.0-rc.6", copy("1.0.0-rc.6"));
    expect(screen.getByRole("heading", { name: "Install 1.0.0-rc.6?" })).toBeTruthy();
    const saved = screen.getByRole("radio", {
      name: "Use the data you had in 1.0.0-rc.6 (2026-10-29)",
    }) as HTMLInputElement;
    const current = screen.getByRole("radio", {
      name: "Keep your current data",
    }) as HTMLInputElement;
    expect([saved.checked, current.checked]).toEqual([false, true]);
    expect(screen.queryByTestId("switch-warning")).toBeNull();
  });

  it("Install and restart keeps the current data by default", () => {
    const { onConfirm } = show("1.0.0-rc.6", copy("1.0.0-rc.6"));
    fireEvent.click(screen.getByRole("button", { name: "Install and restart" }));
    expect(onConfirm).toHaveBeenCalledWith(false);
  });

  it("Install and restart restores the saved data when chosen", () => {
    const { onConfirm } = show("1.0.0-rc.6", copy("1.0.0-rc.6"));
    fireEvent.click(
      screen.getByRole("radio", { name: "Use the data you had in 1.0.0-rc.6 (2026-10-29)" }),
    );
    flush(); // Solid 2 holds a write until the next flush; a person's next click comes later
    fireEvent.click(screen.getByRole("button", { name: "Install and restart" }));
    expect(onConfirm).toHaveBeenCalledWith(true);
  });

  it("Cancel cancels", () => {
    const { onCancel, onConfirm } = show("1.0.0-rc.6", copy("1.0.0-rc.6"));
    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });
});
