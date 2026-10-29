import { render, screen, cleanup, fireEvent, within } from "@solidjs/testing-library";
import { CircleAlert, Lock, Plus, Trash2, TriangleAlert } from "lucide";
import { flush } from "solid-js";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { readDuration } from "../../test/cssTokens";
import { drawn } from "../../test/drawn";
import type { EnvView, PropertiesEdit } from "../shared/env";
import type { AppEvent } from "../shared/events";
import type { Api } from "../shared/ipc";
import { App } from "./App";
import { btn, btnDialog, btnIconSmall, primaryDialog } from "./buttons";
import { EXIT_MS } from "./PropertiesDialog";
import { dialog as dialogStyle, input as inputStyle } from "./ui";

// Every array store the app creates is the dialog's draft rows: this lets a test look inside it.
const drafts = vi.hoisted(() => [] as unknown[][]);
vi.mock("solid-js", async (importOriginal) => {
  const mod = await importOriginal<typeof import("solid-js")>();
  const createStore = mod.createStore as unknown as (...args: unknown[]) => unknown[];
  return {
    ...mod,
    createStore: (init: unknown, ...rest: unknown[]) => {
      const store = createStore(init, ...rest);
      if (Array.isArray(init)) drafts.push(store[0] as unknown[]);
      return store;
    },
  };
});

// jsdom has no modal dialog: open and close only flip the attribute.
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute("open", "");
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute("open");
  };
});

function setup() {
  let emit: (e: AppEvent) => void = () => {};
  const api = {
    ready: vi.fn(async () => {}),
    openProject: vi.fn(async () => {}),
    refresh: vi.fn(async () => {}),
    run: vi.fn(async () => {}),
    cancel: vi.fn(async () => {}),
    openReport: vi.fn(async () => {}),
    setFilter: vi.fn(async () => {}),
    setEnv: vi.fn(async () => {}),
    saveProperties: vi.fn(async (_edit: PropertiesEdit): Promise<boolean> => true),
    onEvent: (cb: (e: AppEvent) => void) => ((emit = cb), () => {}),
  };
  render(() => <App api={api as unknown as Api} />);
  const send = (e: AppEvent) => {
    emit(e);
    flush();
  };
  const load = (over: Partial<EnvView> = {}) => {
    send({
      type: "project",
      project: { root: "D:/p", layout: "standalone" },
      tree: null,
      error: null,
    });
    send({
      type: "environment",
      view: {
        env: null,
        suggestions: [],
        overlays: [],
        properties: { shared: [], byEnv: {} },
        secretsAvailable: true,
        ...over,
      },
    });
  };
  return { api, send, load };
}

// timeout is "5000" for everyone and "9000" in qa, where api.token is a saved secret
const saved: EnvView = {
  env: "dev",
  suggestions: [],
  overlays: [],
  secretsAvailable: true,
  properties: {
    shared: [{ name: "timeout", value: "5000" }],
    byEnv: {
      qa: [
        { name: "timeout", value: "9000" },
        { name: "api.token", secret: true, state: "set" },
      ],
      prod: [],
    },
  },
};

const openDialog = () => {
  fireEvent.click(screen.getByTestId("properties-button"));
  flush();
};
const dialog = () => screen.getByTestId("properties-dialog") as HTMLDialogElement;
const scope = (s: string) =>
  screen.getAllByTestId("scope-item").find((i) => i.dataset.scope === s) as HTMLButtonElement;
const choose = (s: string) => {
  fireEvent.click(scope(s));
  flush();
};
const rows = () => screen.queryAllByTestId("property-row");
const inRow = (i: number, id: string) => within(rows()[i]).getByTestId(id) as HTMLInputElement;
const typeIn = (el: HTMLElement, value: string) => {
  fireEvent.input(el, { target: { value } });
  flush();
};
const addRow = (name: string, value = "v") => {
  fireEvent.click(screen.getByTestId("add-property"));
  flush();
  const i = rows().length - 1;
  typeIn(inRow(i, "prop-name"), name);
  typeIn(inRow(i, "prop-value"), value);
  return i;
};
const save = () => screen.getByTestId("save-properties") as HTMLButtonElement;
const cancel = () => screen.getByTestId("cancel-properties") as HTMLButtonElement;
const errorOf = (i: number) => within(rows()[i]).queryByTestId("row-error")?.textContent;
/** Lets main's answer to Save arrive. */
const settle = async () => {
  await vi.advanceTimersByTimeAsync(0);
  flush();
};
/** Lets the fade-out end (R5.2: 100 ms): the dialog drops its draft and its content then. */
const afterExit = () => {
  vi.advanceTimersByTime(150);
  flush();
};
const RAW = /\b(zinc|red|green|sky|amber|slate|gray)-\d|dark:| title=/; // R9.3

// the dialog drops its draft on a timer, once its exit has ended
beforeEach(() => {
  vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("properties dialog", () => {
  it("lists scopes and rows", () => {
    const { load } = setup();
    load(saved);
    expect(dialog().open).toBe(false);
    openDialog();
    expect(dialog().open).toBe(true);
    // All environments, every environment with a list, then the current one if it has none
    expect(screen.getAllByTestId("scope-item").map((i) => i.dataset.scope)).toEqual([
      "",
      "qa",
      "prod",
      "dev",
    ]);
    expect(scope("").textContent).toBe("All environments");
    expect(scope("").getAttribute("aria-current")).toBe("true");
    expect(rows()).toHaveLength(1);
    expect(inRow(0, "prop-name").value).toBe("timeout");
    expect(inRow(0, "prop-value").value).toBe("5000");
    expect(inRow(0, "prop-secret").checked).toBe(false);
    expect(screen.queryByTestId("row-error")).toBeNull();
    choose("qa");
    expect(scope("qa").getAttribute("aria-current")).toBe("true");
    expect(scope("").getAttribute("aria-current")).toBeNull();
    expect(
      rows()
        .map((r) => within(r).getByTestId("prop-name"))
        .map((n) => (n as HTMLInputElement).value),
    ).toEqual(["timeout", "api.token"]);
    expect(inRow(0, "prop-value").value).toBe("9000");
    choose("prod");
    expect(rows()).toHaveLength(0);
    choose("dev");
    expect(rows()).toHaveLength(0);
    // add and delete a row, and an empty name is an error
    fireEvent.click(screen.getByTestId("add-property"));
    flush();
    expect(rows()).toHaveLength(1);
    expect(errorOf(0)).toBe("Enter a name");
    fireEvent.click(within(rows()[0]).getByTestId("prop-delete"));
    flush();
    expect(rows()).toHaveLength(0);
    expect(save().disabled).toBe(false);
  });

  it("adds and deletes environment lists", () => {
    const { api, load } = setup();
    load(saved);
    openDialog();
    expect((screen.getByTestId("delete-env") as HTMLButtonElement).disabled).toBe(true); // All environments
    choose("dev");
    expect((screen.getByTestId("delete-env") as HTMLButtonElement).disabled).toBe(true); // no list yet
    const field = screen.getByTestId("add-env-input");
    typeIn(field, "not valid!");
    fireEvent.click(screen.getByTestId("add-env"));
    flush();
    expect(screen.getByTestId("add-env-error").textContent).toMatch(/letters, digits/);
    typeIn(field, "staging");
    expect(screen.queryByTestId("add-env-error")).toBeNull();
    fireEvent.keyDown(field, { key: "Enter" });
    flush();
    expect(screen.getAllByTestId("scope-item").map((i) => i.dataset.scope)).toEqual([
      "",
      "qa",
      "prod",
      "staging",
      "dev",
    ]);
    expect(scope("staging").getAttribute("aria-current")).toBe("true");
    expect((field as HTMLInputElement).value).toBe("");
    addRow("a.b");
    // naming an environment that has a list just selects it
    typeIn(field, "qa");
    fireEvent.click(screen.getByTestId("add-env"));
    flush();
    expect(scope("qa").getAttribute("aria-current")).toBe("true");
    expect(rows()).toHaveLength(2);
    choose("prod");
    fireEvent.click(screen.getByTestId("delete-env"));
    flush();
    expect(screen.getAllByTestId("scope-item").map((i) => i.dataset.scope)).toEqual([
      "",
      "qa",
      "staging",
      "dev",
    ]);
    expect(scope("").getAttribute("aria-current")).toBe("true");
    fireEvent.click(save());
    expect(api.saveProperties).toHaveBeenCalledExactlyOnceWith({
      shared: [{ name: "timeout", value: "5000", secret: false }],
      byEnv: {
        qa: [
          { name: "timeout", value: "9000", secret: false },
          { name: "api.token", secret: true, keep: true },
        ],
        staging: [{ name: "a.b", value: "v", secret: false }],
      },
    });
  });

  it("rows added under the current environment, which has no list yet, are saved", () => {
    const { api, load } = setup();
    load(saved); // current environment dev, lists qa and prod only
    openDialog();
    choose("dev");
    expect((screen.getByTestId("delete-env") as HTMLButtonElement).disabled).toBe(true);
    addRow("dev.only", "1");
    expect((screen.getByTestId("delete-env") as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(save());
    const edit = api.saveProperties.mock.calls[0][0];
    expect(edit.byEnv.dev).toEqual([{ name: "dev.only", value: "1", secret: false }]);
    expect(Object.keys(edit.byEnv)).toEqual(["qa", "prod", "dev"]);
  });

  it("an invalid row under the current environment blocks Save and marks its scope", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    choose("dev");
    addRow("karate.env");
    expect(errorOf(0)).toBe("Reserved name");
    expect(scope("dev").dataset.invalid).toBe("true");
    expect(save().disabled).toBe(true);
    fireEvent.click(within(rows()[0]).getByTestId("prop-delete"));
    flush();
    expect(save().disabled).toBe(false);
  });

  it("Save, Cancel and Escape drop the draft, so a typed secret does not linger", async () => {
    const { api, load } = setup();
    load(saved);
    const draft = () => JSON.stringify(drafts.at(-1));
    const typeSecret = () => {
      choose("qa");
      const i = addRow("fresh.token", "s3cret-token-123");
      fireEvent.click(inRow(i, "prop-secret"));
      flush();
      expect(draft()).toContain("s3cret-token-123");
    };
    for (const close of [
      () => fireEvent.click(save()),
      () => fireEvent.click(screen.getByTestId("cancel-properties")),
      () => {
        dialog().removeAttribute("open"); // Escape: the browser closes it and fires `close`
        fireEvent(dialog(), new Event("close"));
      },
    ]) {
      openDialog();
      typeSecret();
      close();
      await settle();
      afterExit();
      expect(drafts.at(-1)).toHaveLength(0);
      expect(draft()).not.toContain("s3cret-token-123");
    }
    // Save still sent the typed value: dropping the draft must not touch the payload
    expect(api.saveProperties.mock.calls[0][0].byEnv.qa.at(-1)).toEqual({
      name: "fresh.token",
      value: "s3cret-token-123",
      secret: true,
    });
    // and the next dialog starts from main's view again
    openDialog();
    choose("qa");
    expect(
      rows().map((r) => (within(r).getByTestId("prop-name") as HTMLInputElement).value),
    ).toEqual(["timeout", "api.token"]);
    fireEvent.click(save());
    expect(api.saveProperties.mock.lastCall![0].byEnv.qa[1]).toEqual({
      name: "api.token",
      secret: true,
      keep: true,
    });
  });

  it("Save keeps the draft until main has applied it", async () => {
    const { api, load } = setup();
    load(saved);
    let answer!: (applied: boolean) => void;
    api.saveProperties.mockImplementation(() => new Promise<boolean>((r) => (answer = r)));
    openDialog();
    choose("qa");
    const i = addRow("fresh.token", "s3cret-token-123");
    fireEvent.click(inRow(i, "prop-secret"));
    flush();
    fireEvent.click(save());
    flush();
    // waiting for main: it can be neither cancelled nor sent again
    expect(save().disabled).toBe(true);
    expect(save().textContent).toBe("Saving…");
    expect(cancel().disabled).toBe(true);
    // refused (main's notice says why): the dialog stays, with the typed secret
    answer(false);
    await settle();
    expect(dialog().open).toBe(true);
    expect(inRow(i, "prop-value").value).toBe("s3cret-token-123");
    expect(save().disabled).toBe(false);
    expect(save().textContent).toBe("Save");
    expect(cancel().disabled).toBe(false);
    // a call that fails too
    api.saveProperties.mockImplementationOnce(async () => {
      throw new Error("gone");
    });
    fireEvent.click(save());
    await settle();
    expect(dialog().open).toBe(true);
    expect(inRow(i, "prop-value").value).toBe("s3cret-token-123");
    expect(save().disabled).toBe(false);
    // applied: it closes and drops the draft
    fireEvent.click(save());
    flush();
    expect(dialog().open).toBe(true);
    answer(true);
    await settle();
    expect(dialog().open).toBe(false);
    afterExit();
    expect(drafts.at(-1)).toHaveLength(0);
    expect(api.saveProperties).toHaveBeenCalledTimes(3);
  });

  it("a list of more than 500 rows blocks Save", () => {
    const { load } = setup();
    const many = Array.from({ length: 501 }, (_, i) => ({ name: `p${i}`, value: "v" }));
    load({ properties: { shared: many, byEnv: {} } });
    openDialog();
    expect(errorOf(499)).toBeUndefined();
    expect(errorOf(500)).toBe("At most 500 properties per list");
    expect(scope("").dataset.invalid).toBe("true");
    expect(save().disabled).toBe(true);
    fireEvent.click(within(rows()[0]).getByTestId("prop-delete"));
    flush();
    expect(screen.queryByTestId("row-error")).toBeNull();
    expect(save().disabled).toBe(false);
  });

  it("a 101st environment is refused", () => {
    const { load } = setup();
    const byEnv = Object.fromEntries(Array.from({ length: 100 }, (_, i) => [`e${i}`, []]));
    load({ env: "dev", properties: { shared: [], byEnv } });
    openDialog();
    const field = screen.getByTestId("add-env-input");
    typeIn(field, "e100");
    fireEvent.click(screen.getByTestId("add-env"));
    flush();
    expect(screen.getByTestId("add-env-error").textContent).toBe("At most 100 environments");
    expect(screen.getAllByTestId("scope-item")).toHaveLength(102); // All environments, e0-e99, dev
    // nor by the first row of the current environment, which has no list yet
    choose("dev");
    typeIn(field, "");
    expect(screen.queryByTestId("add-env-error")).toBeNull();
    fireEvent.click(screen.getByTestId("add-property"));
    flush();
    expect(rows()).toHaveLength(0);
    expect(screen.getByTestId("add-env-error").textContent).toBe("At most 100 environments");
    // an environment that has a list can still be chosen
    typeIn(field, "e5");
    fireEvent.click(screen.getByTestId("add-env"));
    flush();
    expect(screen.queryByTestId("add-env-error")).toBeNull();
    expect(scope("e5").getAttribute("aria-current")).toBe("true");
  });

  it("secret shows dots and Replace, never a value", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    choose("qa");
    const secretRow = () => within(rows()[1]);
    expect(inRow(1, "prop-name").value).toBe("api.token");
    expect(inRow(1, "prop-secret").checked).toBe(true);
    expect(secretRow().getByTestId("prop-dots").textContent).toMatch(/^•+$/);
    expect(secretRow().getByTestId("prop-replace").textContent).toBe("Replace");
    expect(secretRow().queryByTestId("prop-value")).toBeNull();
    expect(secretRow().queryByTestId("prop-unreadable")).toBeNull();
    expect([...dialog().querySelectorAll("input")].some((i) => i.value === "set")).toBe(false);
    // Replace takes a new value, and the typed text is hidden
    fireEvent.click(secretRow().getByTestId("prop-replace"));
    flush();
    expect(secretRow().queryByTestId("prop-dots")).toBeNull();
    expect(secretRow().queryByTestId("prop-replace")).toBeNull();
    expect(inRow(1, "prop-value").value).toBe("");
    expect(inRow(1, "prop-value").type).toBe("password");
    expect(document.activeElement).toBe(inRow(1, "prop-value"));
    expect(errorOf(1)).toBe("Enter the secret value");
    expect(save().disabled).toBe(true);
  });

  it("an unreadable secret says so and can be replaced", () => {
    const { load } = setup();
    load({
      env: "qa",
      properties: { shared: [], byEnv: { qa: [{ name: "t", secret: true, state: "unreadable" }] } },
    });
    openDialog();
    choose("qa");
    expect(within(rows()[0]).getByTestId("prop-unreadable").textContent).toBe(
      "Can't be read, enter it again",
    );
    expect(within(rows()[0]).queryByTestId("prop-dots")).toBeNull();
    // a <p> is not valid inside a <span>
    expect(within(rows()[0]).getByTestId("prop-unreadable").parentElement!.tagName).toBe("DIV");
    fireEvent.click(within(rows()[0]).getByTestId("prop-replace"));
    flush();
    expect(within(rows()[0]).queryByTestId("prop-unreadable")).toBeNull();
    expect(inRow(0, "prop-value").value).toBe("");
  });

  it("reserved name blocks Save", () => {
    const { api, load } = setup();
    load(saved);
    openDialog();
    const i = addRow("karate.options", "-t @x");
    expect(errorOf(i)).toBe("Reserved name");
    expect(save().disabled).toBe(true);
    fireEvent.click(save());
    expect(api.saveProperties).not.toHaveBeenCalled();
    typeIn(inRow(i, "prop-name"), "KARATE.ENV");
    expect(errorOf(i)).toBe("Reserved name");
    typeIn(inRow(i, "prop-name"), "my.own");
    expect(errorOf(i)).toBeUndefined();
    expect(save().disabled).toBe(false);
  });

  it("a name that repeats within a list blocks Save, in another list it does not", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    addRow("timeout"); // shared already has timeout
    expect(errorOf(0)).toBe("Duplicate name");
    expect(errorOf(1)).toBe("Duplicate name");
    expect(save().disabled).toBe(true);
    // an error in another list is shown on that list's name in the scope list
    choose("qa");
    expect(rows().every((r) => within(r).queryByTestId("row-error") === null)).toBe(true);
    expect(scope("").dataset.invalid).toBe("true");
    expect(scope("qa").dataset.invalid).toBeUndefined();
    expect(save().disabled).toBe(true);
    choose("");
    fireEvent.click(within(rows()[1]).getByTestId("prop-delete"));
    flush();
    expect(save().disabled).toBe(false);
    expect(scope("").dataset.invalid).toBeUndefined();
  });

  it("non-ASCII value shows the exact error", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    const i = addRow("greeting", "h\u00e9llo");
    expect(errorOf(i)).toBe("This value cannot be passed to Java exactly.");
    expect(save().disabled).toBe(true);
    typeIn(inRow(i, "prop-value"), "hello");
    expect(errorOf(i)).toBeUndefined();
    typeIn(inRow(i, "prop-value"), "x".repeat(10_001));
    expect(errorOf(i)).toBe("Too long");
  });

  it("save sends keep for an untouched secret and the new value after Replace", async () => {
    const { api, load, send } = setup();
    load(saved);
    openDialog();
    choose("qa");
    fireEvent.click(save());
    await settle();
    expect(dialog().open).toBe(false);
    expect(api.saveProperties).toHaveBeenCalledTimes(1);
    expect(api.saveProperties).toHaveBeenLastCalledWith({
      shared: [{ name: "timeout", value: "5000", secret: false }],
      byEnv: {
        qa: [
          { name: "timeout", value: "9000", secret: false },
          { name: "api.token", secret: true, keep: true },
        ],
        prod: [],
      },
    });
    // nothing changes in the window until main says so
    afterExit();
    expect(screen.queryAllByTestId("property-row")).toHaveLength(0);
    send({
      type: "environment",
      view: { ...saved, env: "qa" },
    });
    openDialog();
    choose("qa");
    fireEvent.click(within(rows()[1]).getByTestId("prop-replace"));
    flush();
    typeIn(inRow(1, "prop-value"), "s3cret-token-123");
    expect(save().disabled).toBe(false);
    fireEvent.click(save());
    expect(api.saveProperties).toHaveBeenLastCalledWith({
      shared: [{ name: "timeout", value: "5000", secret: false }],
      byEnv: {
        qa: [
          { name: "timeout", value: "9000", secret: false },
          { name: "api.token", value: "s3cret-token-123", secret: true },
        ],
        prod: [],
      },
    });
  });

  it("renaming or unticking a saved secret clears it and asks for a value", () => {
    const { api, load } = setup();
    load(saved);
    openDialog();
    choose("qa");
    typeIn(inRow(1, "prop-name"), "api.token2");
    expect(within(rows()[1]).queryByTestId("prop-dots")).toBeNull();
    expect(inRow(1, "prop-value").value).toBe("");
    expect(errorOf(1)).toBe("Enter the secret value");
    expect(save().disabled).toBe(true);
    typeIn(inRow(1, "prop-value"), "abcdef");
    expect(save().disabled).toBe(false);
    // a fresh dialog: untick Secret on the saved row
    fireEvent.click(screen.getByTestId("cancel-properties"));
    openDialog();
    choose("qa");
    fireEvent.click(inRow(1, "prop-secret"));
    flush();
    expect(inRow(1, "prop-secret").checked).toBe(false);
    expect(within(rows()[1]).queryByTestId("prop-dots")).toBeNull();
    expect(inRow(1, "prop-value").value).toBe("");
    expect(inRow(1, "prop-value").type).toBe("text");
    expect(errorOf(1)).toBeUndefined();
    typeIn(inRow(1, "prop-value"), "plain");
    fireEvent.click(save());
    const edit = api.saveProperties.mock.calls[0][0];
    expect(edit.byEnv.qa[1]).toEqual({ name: "api.token", value: "plain", secret: false });
  });

  it("payload is structured-cloneable", async () => {
    const { api, load } = setup();
    load(saved);
    openDialog();
    addRow("one");
    choose("qa");
    fireEvent.click(within(rows()[1]).getByTestId("prop-replace"));
    flush();
    typeIn(inRow(1, "prop-value"), "new-secret");
    fireEvent.click(save());
    await settle();
    openDialog();
    fireEvent.click(save());
    expect(api.saveProperties).toHaveBeenCalledTimes(2);
    for (const [arg] of api.saveProperties.mock.calls) {
      const copy = structuredClone(arg);
      expect(copy).toEqual(arg);
    }
  });

  it("secrets note shown when a secret exists", () => {
    const { load } = setup();
    load({ env: "dev", properties: { shared: [{ name: "a", value: "b" }], byEnv: {} } });
    openDialog();
    expect(screen.queryByTestId("secrets-note")).toBeNull();
    addRow("token", "abcdef");
    fireEvent.click(inRow(1, "prop-secret"));
    flush();
    expect(screen.getByTestId("secrets-note").textContent).toBe(
      "Secrets are hidden in this app, except inside encoded text such as a Basic auth header. Karate's HTML report and event file can still contain them unless the project sets configure logging mask. Run history keeps the reports of the last 20 runs; Clear history deletes them.",
    );
    expect(inRow(1, "prop-value").type).toBe("password");
    fireEvent.click(inRow(1, "prop-secret"));
    flush();
    expect(screen.queryByTestId("secrets-note")).toBeNull();
    expect(inRow(1, "prop-value").type).toBe("text");
  });

  it("the note also shows for a saved secret", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    expect(screen.getByTestId("secrets-note")).toBeTruthy(); // the secret is in qa, not the list in view
  });

  it("short secret shows the hint", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    const i = addRow("pin", "abc");
    const hint = () => within(rows()[i]).queryByTestId("row-hint")?.textContent;
    expect(hint()).toBeUndefined(); // a plain value
    fireEvent.click(inRow(i, "prop-secret"));
    flush();
    expect(hint()).toBe("Too short to hide in the console");
    expect(errorOf(i)).toBeUndefined();
    expect(save().disabled).toBe(false);
    typeIn(inRow(i, "prop-value"), "abcd");
    expect(hint()).toBeUndefined();
  });

  it("secret checkbox disabled when secrets are unavailable", () => {
    const { load } = setup();
    load({ ...saved, secretsAvailable: false });
    openDialog();
    expect(inRow(0, "prop-secret").disabled).toBe(true);
    expect(screen.getByTestId("secrets-unavailable").textContent).toBe(
      "Secret storage is not available on this system",
    );
    // a saved secret can still be turned into a plain value, but not replaced
    choose("qa");
    expect(inRow(1, "prop-secret").disabled).toBe(false);
    expect((within(rows()[1]).getByTestId("prop-replace") as HTMLButtonElement).disabled).toBe(
      true,
    );
  });

  it("the unavailable message is absent when secrets work", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    expect(screen.queryByTestId("secrets-unavailable")).toBeNull();
    expect(inRow(0, "prop-secret").disabled).toBe(false);
  });

  it("Cancel, Escape and reopening discard edits and start from the current view", () => {
    const { api, load } = setup();
    load(saved);
    openDialog();
    addRow("gone");
    fireEvent.click(screen.getByTestId("cancel-properties"));
    flush();
    expect(dialog().open).toBe(false);
    afterExit();
    expect(screen.queryAllByTestId("property-row")).toHaveLength(0);
    openDialog();
    expect(rows()).toHaveLength(1);
    expect(scope("").getAttribute("aria-current")).toBe("true");
    addRow("gone2");
    dialog().removeAttribute("open"); // Escape: the browser closes it and fires `close`
    fireEvent(dialog(), new Event("close"));
    flush();
    afterExit();
    expect(screen.queryAllByTestId("property-row")).toHaveLength(0);
    openDialog();
    expect(rows()).toHaveLength(1);
    expect(api.saveProperties).not.toHaveBeenCalled();
  });

  it("the Properties button is disabled with no project, while busy and while running", () => {
    const { send, load } = setup();
    send({ type: "opening", root: "D:/p" }); // no project yet: the project view, not home
    const button = () => screen.getByTestId("properties-button") as HTMLButtonElement;
    expect(button().disabled).toBe(true);
    load(saved);
    expect(button().disabled).toBe(false);
    send({ type: "busy", busy: true });
    expect(button().disabled).toBe(true);
    send({ type: "busy", busy: false });
    send({
      type: "runStarted",
      selection: "d:",
      runId: "20261025-120000-000",
      keys: [],
      total: 1,
      startedAt: 1,
      env: null,
    });
    expect(button().disabled).toBe(true);
  });
});

// The dialog fades out over 100 ms (R5.2): its title, rows and buttons stay until it is gone, and only
// then does the draft go, so an unsaved secret does not linger in the window's memory.
describe("while it fades out", () => {
  it("keeps the content longer than the exit transition lasts", () => {
    expect(EXIT_MS).toBeGreaterThan(readDuration("fast"));
  });

  const SECRET = "s3cret-token-123";
  /** The dialog with a typed secret in a new row, open. */
  const typed = () => {
    const ctx = setup();
    ctx.load(saved);
    openDialog();
    choose("qa");
    const i = addRow("fresh.token", SECRET);
    fireEvent.click(inRow(i, "prop-secret"));
    flush();
    return ctx;
  };
  const draft = () => JSON.stringify(drafts.at(-1));
  const whole = () => {
    expect(within(dialog()).getByText("Run properties")).toBeTruthy();
    expect(rows()).toHaveLength(3);
    expect(scope("qa")).toBeTruthy();
    expect(save().textContent).toMatch(/^Sav/); // "Saving…" while it fades after a Save
    expect(cancel().textContent).toBe("Cancel");
    expect(draft()).toContain(SECRET);
  };
  const gone = () => {
    expect(within(dialog()).queryByText("Run properties")).toBeNull();
    expect(rows()).toHaveLength(0);
    expect(screen.queryByTestId("save-properties")).toBeNull();
    expect(drafts.at(-1)).toHaveLength(0);
    expect(draft()).not.toContain(SECRET);
  };
  /** The box is whole for the exit's 100 ms, and gone soon after. */
  const fades = () => {
    whole();
    vi.advanceTimersByTime(100);
    flush();
    whole();
    vi.advanceTimersByTime(50);
    flush();
    gone();
  };

  it("keeps its content after Cancel for the whole exit, then drops the draft", () => {
    typed();
    fireEvent.click(cancel());
    flush();
    expect(dialog().open).toBe(false);
    fades();
  });

  it("does the same after Escape", () => {
    typed();
    dialog().removeAttribute("open"); // the browser closes it, then fires `close`
    fireEvent(dialog(), new Event("close"));
    flush();
    fades();
  });

  it("does the same once main has applied Save", async () => {
    const { api } = typed();
    fireEvent.click(save());
    await settle();
    expect(api.saveProperties).toHaveBeenCalledOnce();
    expect(dialog().open).toBe(false);
    fades();
  });

  it("keeps the draft when main refused Save: no timer starts", async () => {
    const { api } = typed();
    api.saveProperties.mockResolvedValueOnce(false);
    fireEvent.click(save());
    await settle();
    expect(dialog().open).toBe(true);
    vi.advanceTimersByTime(1000);
    flush();
    whole();
  });

  it("reopened within the fade, it starts from main's view and keeps it", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    addRow("gone");
    fireEvent.click(cancel());
    flush();
    vi.advanceTimersByTime(60); // mid-fade
    flush();
    openDialog();
    expect(dialog().open).toBe(true);
    expect(rows()).toHaveLength(1);
    expect(inRow(0, "prop-name").value).toBe("timeout");
    expect(draft()).not.toContain("gone");
    // the first close's timer was cleared: it must not drop the new draft
    vi.advanceTimersByTime(1000);
    flush();
    expect(rows()).toHaveLength(1);
    expect(within(dialog()).getByText("Run properties")).toBeTruthy();
    expect(draft()).toContain("timeout");
    expect(dialog().open).toBe(true);
  });

  it("ignores a close event that arrives after a reopen", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    fireEvent.click(cancel());
    flush();
    openDialog();
    addRow("kept");
    fireEvent(dialog(), new Event("close")); // late: the dialog is open again
    flush();
    vi.advanceTimersByTime(1000);
    flush();
    expect(dialog().open).toBe(true);
    expect(rows()).toHaveLength(2);
  });
});

describe("the look of the dialog (spec R19)", () => {
  it("is a dialog-style box with a 16 px title", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    expect(dialog().className).toContain(dialogStyle);
    expect(dialog().className).toContain("w-[48rem]");
    const title = document.getElementById("properties-title")!;
    expect(dialog().getAttribute("aria-labelledby")).toBe("properties-title");
    expect(title.textContent).toBe("Run properties");
    expect(title.className).toContain("text-lg");
    expect(title.className).toContain("font-semibold");
    expect(dialog().outerHTML).not.toMatch(RAW);
  });

  it("keeps Save first and Cancel last, in the 14 px dialog buttons", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    expect(save().className).toContain(primaryDialog);
    expect(cancel().className).toContain(btnDialog);
    expect(
      save().compareDocumentPosition(cancel()) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    for (const b of [save(), cancel()]) expect(b.className).not.toMatch(/text-sm|!/);
    expect(save().parentElement!.className).toContain("justify-end");
  });

  it("lists the environments as a selectable list: selected fill and the accent bar", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    choose("qa");
    const bar = "shadow-[inset_2px_0_0_var(--color-accent)]";
    expect(scope("qa").className).toContain("bg-selected");
    expect(scope("qa").className).toContain(bar);
    expect(scope("").className).not.toContain("bg-selected");
    expect(scope("").className).not.toContain(bar);
    expect(scope("").className).toContain("hover:bg-hover");
  });

  it("marks an environment with errors by an icon and the words, not by colour alone", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    addRow("karate.env");
    expect(scope("").textContent).toBe("All environments has errors");
    expect(drawn(scope("").querySelector("svg"), CircleAlert)).toBe(true);
    expect(scope("").querySelector("svg")!.getAttribute("class")).toContain("text-failed");
    expect(scope("qa").querySelector("svg")).toBeNull();
  });

  it("sets the fields in the shared input style", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    expect(screen.getByTestId("add-env-input").className).toContain(inputStyle);
    expect(inRow(0, "prop-name").className).toContain(inputStyle);
    expect(inRow(0, "prop-value").className).toContain(inputStyle);
    addRow("t", "abcdef");
    fireEvent.click(inRow(1, "prop-secret"));
    flush();
    expect(inRow(1, "prop-value").className).toContain(inputStyle); // a password field too
    for (const el of [inRow(0, "prop-name"), inRow(0, "prop-value")])
      expect(el.className).toContain("min-w-0");
    // the Secret checkbox: the accent, and a 24 px target with its label (R10.3)
    expect(inRow(0, "prop-secret").className).toContain("accent-accent");
    expect(inRow(0, "prop-secret").closest("label")!.className).toContain("h-6");
  });

  it("shows an error in failed with the error icon, under its row and linked to its fields", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    const i = addRow("karate.options", "-t @x");
    const error = within(rows()[i]).getByTestId("row-error");
    expect(error.textContent).toBe("Reserved name");
    expect(error.className).toContain("text-failed");
    expect(error.className).toContain("text-xs");
    expect(drawn(error.querySelector("svg"), CircleAlert)).toBe(true);
    for (const id of ["prop-name", "prop-value"]) {
      expect(inRow(i, id).getAttribute("aria-invalid")).toBe("true");
      expect(inRow(i, id).getAttribute("aria-describedby")).toBe(error.id);
    }
    typeIn(inRow(i, "prop-name"), "my.own");
    expect(inRow(i, "prop-name").hasAttribute("aria-describedby")).toBe(false);
    expect(inRow(i, "prop-value").hasAttribute("aria-describedby")).toBe(false);
  });

  it("shows the add-environment error with the error icon", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    typeIn(screen.getByTestId("add-env-input"), "not valid!");
    fireEvent.click(screen.getByTestId("add-env"));
    flush();
    const error = screen.getByTestId("add-env-error");
    expect(error.className).toContain("text-failed");
    expect(drawn(error.querySelector("svg"), CircleAlert)).toBe(true);
  });

  it("shows the secrets note in muted with a lock, and a short secret's hint muted", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    const note = screen.getByTestId("secrets-note");
    expect(note.className).toContain("text-muted");
    expect(drawn(note.querySelector("svg"), Lock)).toBe(true);
    const i = addRow("pin", "abc");
    fireEvent.click(inRow(i, "prop-secret"));
    flush();
    expect(within(rows()[i]).getByTestId("row-hint").className).toContain("text-muted");
  });

  it("shows the two warnings in cancelled with the warning icon", () => {
    const { load } = setup();
    load({
      ...saved,
      secretsAvailable: false,
      properties: {
        shared: [],
        byEnv: { qa: [{ name: "t", secret: true, state: "unreadable" }] },
      },
    });
    openDialog();
    choose("qa");
    for (const el of [
      screen.getByTestId("secrets-unavailable"),
      within(rows()[0]).getByTestId("prop-unreadable"),
    ]) {
      expect(el.className).toContain("text-cancelled");
      expect(drawn(el.querySelector("svg"), TriangleAlert)).toBe(true);
    }
  });

  it("deletes a row with a 24 px icon button that has a name and a tooltip", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    const button = within(rows()[0]).getByTestId("prop-delete");
    expect(button.getAttribute("aria-label")).toBe("Delete timeout");
    expect(button.className).toContain(btnIconSmall);
    expect(button.textContent).toBe("");
    expect(drawn(button.querySelector("svg"), Trash2)).toBe(true);
    fireEvent.pointerEnter(button.closest("span.inline-flex")!);
    vi.advanceTimersByTime(500);
    expect(document.querySelector('[role="tooltip"]')!.textContent).toBe("Delete timeout");
  });

  it("adds a property with a plus icon, and the other buttons are standard", () => {
    const { load } = setup();
    load(saved);
    openDialog();
    const add = screen.getByTestId("add-property");
    expect(add.className).toContain(btn);
    expect(add.textContent).toBe("Add property");
    expect(drawn(add.querySelector("svg"), Plus)).toBe(true);
    for (const id of ["add-env", "delete-env"])
      expect(screen.getByTestId(id).className, id).toContain(btn);
    choose("qa");
    expect(within(rows()[1]).getByTestId("prop-replace").className).toContain(btn);
  });

  it("tells a cut environment name in a tooltip", () => {
    const { load } = setup();
    const name = `e${"x".repeat(60)}`;
    load({ ...saved, properties: { shared: [], byEnv: { [name]: [] } } });
    openDialog();
    const long = scope(name);
    const line = long.querySelector(".truncate") as HTMLElement;
    expect(line.textContent).toBe(name);
    // jsdom has no layout: make it look cut, then hover
    Object.defineProperty(line, "scrollWidth", { value: 300 });
    Object.defineProperty(line, "clientWidth", { value: 100 });
    fireEvent.pointerEnter(line.closest("span.inline-flex")!);
    vi.advanceTimersByTime(500);
    expect(document.querySelector('[role="tooltip"]')!.textContent).toBe(name);
  });
});
