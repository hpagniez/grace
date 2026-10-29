import { cleanup, fireEvent, render, screen } from "@solidjs/testing-library";
import { createStore, flush } from "solid-js";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import type { Api } from "../shared/ipc";
import type { UpdateProgress as Progress } from "../shared/updates";
import { initialState, type AppState } from "./reduce";
import { UpdateProgress } from "./UpdateProgress";

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

const download = (received: number, rollback = false): Progress => ({
  version: rollback ? "1.0.0-rc.4" : "1.0.0-rc.6",
  rollback,
  step: "download",
  received,
  total: 218_557_130,
});

function setup(progress: Progress | null) {
  const [state, setState] = createStore<AppState>({ ...initialState(), updateProgress: progress });
  const api = {
    cancelUpdate: vi.fn(async () => {}),
    openWhatsNew: vi.fn(async () => {}),
  };
  render(() => (
    <>
      <button type="button" data-testid="outside">
        outside
      </button>
      <UpdateProgress state={state} api={api as unknown as Api} />
    </>
  ));
  const set = (p: Progress | null) => {
    setState((d) => void (d.updateProgress = p));
    flush();
  };
  return { api, set };
}
const dialog = () => screen.getByTestId("update-progress") as HTMLDialogElement;
const title = () => screen.getByRole("heading").textContent;
const step = () => screen.getByTestId("update-step").textContent;
const cancel = () => screen.getByTestId("update-cancel") as HTMLButtonElement;
/** Escape: the browser fires `cancel`, then closes the dialog unless that was prevented. */
const escape = () => {
  const cancelled = fireEvent(dialog(), new Event("cancel", { cancelable: true }));
  if (cancelled) {
    dialog().removeAttribute("open");
    fireEvent(dialog(), new Event("close"));
    flush();
  }
  return !cancelled;
};

describe("UpdateProgress", () => {
  it("is closed until an install is under way", () => {
    const { set } = setup(null);
    expect(dialog().open).toBe(false);
    set(download(0));
    expect(dialog().open).toBe(true);
    set(null);
    expect(dialog().open).toBe(false);
  });

  it("names an update and a rollback", () => {
    const { set } = setup(download(0));
    expect(title()).toBe("Updating to 1.0.0-rc.6");
    expect(dialog().getAttribute("aria-labelledby")).toBe(screen.getByRole("heading").id);
    set(download(0, true));
    expect(title()).toBe("Rolling back to 1.0.0-rc.4");
  });

  it("shows the download in whole MB, then the check and the unpack (R8)", () => {
    const { set } = setup(download(92_000_000));
    expect(step()).toBe("Downloading… 92 of 219 MB");
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuenow")).toBe("42");
    set({ ...download(0), step: "check" });
    expect(step()).toBe("Checking…");
    set({ ...download(0), step: "unpack" });
    expect(step()).toBe("Unpacking…");
    expect(screen.getByRole("progressbar").hasAttribute("aria-valuenow")).toBe(false);
  });

  it("shows the hand-over without Cancel: it cannot be stopped (R8.5)", () => {
    const { api } = setup({ ...download(0), step: "restart" });
    expect(step()).toBe("Restarting Grace…");
    expect(screen.queryByTestId("update-cancel")).toBeNull();
    expect(escape()).toBe(true); // prevented
    expect(dialog().open).toBe(true);
    expect(api.cancelUpdate).not.toHaveBeenCalled();
  });

  it("focuses Close when the hand-over fails (R4.6)", () => {
    const { set } = setup({ ...download(0), step: "restart" });
    set({ version: "1.0.0-rc.6", rollback: false, error: "The switch script did not start" });
    expect(cancel().textContent).toBe("Close");
    expect(document.activeElement).toBe(cancel());
  });

  it("Cancel stops the install", () => {
    const { api } = setup(download(1));
    expect(cancel().textContent).toBe("Cancel");
    expect(document.activeElement).toBe(cancel());
    fireEvent.click(cancel());
    expect(api.cancelUpdate).toHaveBeenCalledOnce();
  });

  it("Escape does nothing while a step runs, even if the browser closes the dialog anyway", () => {
    const { api } = setup(download(1));
    expect(escape()).toBe(true); // prevented
    expect(dialog().open).toBe(true);
    // Chrome closes a dialog on a repeated Escape whatever `cancel` did: it opens again
    dialog().removeAttribute("open");
    fireEvent(dialog(), new Event("close"));
    flush();
    expect(dialog().open).toBe(true);
    expect(api.cancelUpdate).not.toHaveBeenCalled();
  });

  it("shows a failure with its release page and Close", () => {
    const { api, set } = setup(download(1));
    set({
      version: "1.0.0-rc.6",
      rollback: false,
      error: "The download was damaged. Try again, or use the release page.",
    });
    expect(title()).toBe("Updating to 1.0.0-rc.6");
    expect(screen.getByTestId("update-error").textContent).toBe(
      "The download was damaged. Try again, or use the release page.",
    );
    expect(screen.queryByRole("progressbar")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "release page" }));
    expect(api.openWhatsNew).toHaveBeenCalledWith("1.0.0-rc.6");
    expect(cancel().textContent).toBe("Close");
    expect(document.activeElement).toBe(cancel()); // the focused Cancel became Close
    fireEvent.click(cancel());
    expect(api.cancelUpdate).toHaveBeenCalledOnce();
  });

  it("Escape closes a failure's message", () => {
    const { api, set } = setup(null);
    set({ version: "1.0.0-rc.6", rollback: false, error: "Nope" });
    expect(escape()).toBe(false);
    expect(api.cancelUpdate).toHaveBeenCalledOnce();
  });

  it("gives focus back to where it was when it closes (R4.6)", () => {
    const { set } = setup(null);
    const outside = screen.getByTestId("outside");
    outside.focus();
    set(download(0));
    expect(document.activeElement).toBe(cancel());
    set(null);
    expect(document.activeElement).toBe(outside);
  });
});
