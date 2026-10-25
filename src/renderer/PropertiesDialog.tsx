import { createMemo, createSignal, createStore, flush, For, Show } from "solid-js";
import {
  MAX_ENVS,
  MAX_ROWS,
  propertyNameError,
  propertyValueError,
  validEnvName,
  type PropertiesEdit,
  type RowEdit,
  type RowView,
} from "../shared/env";
import type { Api } from "../shared/ipc";
import { btn, primary } from "./buttons";
import { INVALID_ENV } from "./EnvPicker";
import type { AppState } from "./reduce";

/** A property row being edited. Scope "" is "All environments". */
interface Row {
  id: number;
  scope: string;
  name: string;
  value: string;
  secret: boolean;
  /**
   * Set while the row still stands for a saved secret, whose value only main holds. Replacing,
   * renaming or unticking Secret ends it: the row then needs a value of its own.
   */
  saved: "set" | "unreadable" | null;
}

const field =
  "min-w-0 rounded border border-zinc-300 bg-white px-2 py-1 text-sm dark:border-zinc-600 dark:bg-zinc-800";
const SECRETS_NOTE =
  "Secrets are hidden in this app, except inside encoded text such as a Basic auth header. Karate's HTML report and event file can still contain them unless the project sets configure logging mask. Run history keeps the reports of the last 20 runs; Clear history deletes them.";

let nextId = 0;
const rowsOf = (scope: string, list: RowView[]): Row[] =>
  list.map((r) => ({
    id: nextId++,
    scope,
    name: r.name,
    value: "secret" in r ? "" : r.value,
    secret: "secret" in r,
    saved: "secret" in r ? r.state : null,
  }));

/**
 * The Properties… button and its dialog. Edits live in a draft that Save sends and Cancel drops; it
 * is dropped on Save only once main says it applied it, so a refused save loses nothing.
 */
export function PropertiesDialog(props: { state: AppState; api: Api }) {
  let dialog!: HTMLDialogElement;
  let list!: HTMLUListElement;
  let session = 0; // each opening; an answer to an earlier one is ignored
  const [open, setOpen] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [rows, setRows] = createStore<Row[]>([]);
  /** Environments that have a list in the draft. */
  const [lists, setLists] = createSignal<string[]>([]);
  const [scope, setScope] = createSignal("");
  const [newEnv, setNewEnv] = createSignal("");
  const [envError, setEnvError] = createSignal<string | null>(null);

  const view = () => props.state.env;
  const locked = () => props.state.run.active || props.state.busy;
  const available = () => view()?.secretsAvailable ?? false;
  // The chosen environment is listed even before it has a list of its own.
  const scopes = () => {
    const env = view()?.env;
    return ["", ...lists(), ...(env && !lists().includes(env) ? [env] : [])];
  };
  const visible = () => rows.filter((r) => r.scope === scope());

  const errors = createMemo(() => {
    const key = (r: Row) => `${r.scope}\0${r.name}`;
    const count = new Map<string, number>();
    rows.forEach((r) => count.set(key(r), (count.get(key(r)) ?? 0) + 1));
    const place = new Map<string, number>(); // rows of each list so far
    return new Map(
      rows.map((r) => {
        place.set(r.scope, (place.get(r.scope) ?? 0) + 1);
        return [
          r.id,
          (place.get(r.scope)! > MAX_ROWS ? `At most ${MAX_ROWS} properties per list` : null) ??
            (r.name === "" ? "Enter a name" : propertyNameError(r.name)) ??
            (count.get(key(r))! > 1 ? "Duplicate name" : null) ??
            (r.saved
              ? null
              : r.secret && r.value === ""
                ? "Enter the secret value"
                : propertyValueError(r.value)),
        ];
      }),
    );
  });
  const invalidScopes = createMemo(
    () => new Set(rows.filter((r) => errors().get(r.id)).map((r) => r.scope)),
  );
  const shortSecret = (r: Row) => r.secret && !r.saved && r.value.length > 0 && r.value.length < 4;

  const patch = (id: number, change: (r: Row) => void) =>
    setRows((draft) => void change(draft.find((r) => r.id === id)!));
  const forget = (r: Row) => {
    r.saved = null; // main keeps the old value only for an untouched row
    r.value = "";
  };

  const show = () => {
    const { shared, byEnv } = view()!.properties;
    const draft = [
      ...rowsOf("", shared),
      ...Object.entries(byEnv).flatMap(([env, rs]) => rowsOf(env, rs)),
    ];
    setRows((d) => void d.splice(0, d.length, ...draft));
    setLists(Object.keys(byEnv));
    setScope("");
    setNewEnv("");
    setEnvError(null);
    session++;
    setSaving(false);
    setOpen(true);
    dialog.showModal();
  };
  // Closing drops the draft, so a typed secret does not linger in the window's memory.
  const drop = () => {
    setOpen(false);
    setRows((d) => void d.splice(0, d.length));
    setLists([]);
    setScope("");
    setNewEnv("");
    setEnvError(null);
  };
  const close = () => {
    dialog.close();
    drop();
  };

  /** Gives `name` a list in the draft; false, saying why, if main would refuse one more. */
  const ensureList = (name: string) => {
    if (lists().includes(name)) return true;
    if (lists().length >= MAX_ENVS) {
      setEnvError(`At most ${MAX_ENVS} environments`);
      return false;
    }
    setLists([...lists(), name]);
    return true;
  };
  const addEnv = () => {
    const name = newEnv().trim();
    if (!validEnvName(name)) {
      setEnvError(INVALID_ENV);
      return;
    }
    if (!ensureList(name)) return;
    setScope(name);
    setNewEnv("");
    setEnvError(null);
  };
  const deleteEnv = () => {
    const gone = scope();
    setRows((d) => {
      for (let i = d.length - 1; i >= 0; i--) if (d[i].scope === gone) d.splice(i, 1);
    });
    setLists(lists().filter((e) => e !== gone));
    setScope("");
  };
  const addRow = () => {
    // the current environment is listed before it has a list: the first row creates it
    if (scope() !== "" && !ensureList(scope())) return;
    setRows(
      (d) =>
        void d.push({
          id: nextId++,
          scope: scope(),
          name: "",
          value: "",
          secret: false,
          saved: null,
        }),
    );
    flush();
    list.querySelector<HTMLInputElement>("li:last-child [data-testid=prop-name]")?.focus();
  };

  // Built field by field: a store proxy cannot be structured-cloned, so IPC would throw.
  const edit = (): PropertiesEdit => {
    const of = (s: string) =>
      rows
        .filter((r) => r.scope === s)
        .map((r): RowEdit =>
          r.saved
            ? { name: r.name, secret: true, keep: true }
            : { name: r.name, value: r.value, secret: r.secret },
        );
    return { shared: of(""), byEnv: Object.fromEntries(lists().map((e) => [e, of(e)])) };
  };

  const save = async () => {
    const mine = session;
    setSaving(true);
    let applied = false;
    try {
      applied = (await props.api.saveProperties(edit())) === true;
    } catch {
      // refused like a false answer; main's notice says why
    }
    if (mine !== session) return; // closed and opened again meanwhile
    if (applied) close();
    else setSaving(false);
  };

  return (
    <>
      <button
        type="button"
        data-testid="properties-button"
        class={btn}
        disabled={locked() || !view()}
        onClick={show}
      >
        Properties…
      </button>
      <dialog
        ref={(e) => (dialog = e)}
        data-testid="properties-dialog"
        aria-labelledby="properties-title"
        class="m-auto w-[48rem] max-w-[95vw] rounded-lg border border-zinc-300 bg-white p-4 text-zinc-900 shadow-xl backdrop:bg-black/40 dark:border-zinc-600 dark:bg-zinc-900 dark:text-zinc-100"
        onClose={() => dialog.open || drop()} // Escape; not a late event after a reopen
      >
        <Show when={open()}>
          <h2 id="properties-title" class="mb-3 text-base font-semibold">
            Run properties
          </h2>
          <div class="flex gap-4">
            <div class="flex w-44 shrink-0 flex-col gap-2">
              <ul class="flex flex-col gap-0.5" aria-label="Environments">
                <For each={scopes()}>
                  {(s) => (
                    <li>
                      <button
                        type="button"
                        data-testid="scope-item"
                        data-scope={s}
                        data-invalid={invalidScopes().has(s) ? "true" : undefined}
                        aria-current={s === scope() ? "true" : undefined}
                        class={`w-full truncate rounded px-2 py-1 text-left text-sm hover:bg-zinc-100 dark:hover:bg-zinc-800 ${s === scope() ? "bg-zinc-200 font-medium dark:bg-zinc-700" : ""}`}
                        onClick={() => setScope(s)}
                      >
                        {s === "" ? "All environments" : s}
                        <Show when={invalidScopes().has(s)}>
                          <span class="sr-only"> has errors</span>
                          <span aria-hidden="true" class="text-red-600 dark:text-red-400">
                            {" !"}
                          </span>
                        </Show>
                      </button>
                    </li>
                  )}
                </For>
              </ul>
              <div class="flex gap-1">
                <input
                  data-testid="add-env-input"
                  aria-label="New environment"
                  placeholder="+ environment"
                  autocomplete="off"
                  spellcheck={false}
                  class={`${field} w-full`}
                  value={newEnv()}
                  onInput={(e) => {
                    setNewEnv(e.currentTarget.value);
                    setEnvError(null);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") addEnv();
                  }}
                />
                <button type="button" data-testid="add-env" class={btn} onClick={addEnv}>
                  Add
                </button>
              </div>
              <Show when={envError()}>
                <span data-testid="add-env-error" class="text-xs text-red-700 dark:text-red-400">
                  {envError()}
                </span>
              </Show>
              <button
                type="button"
                data-testid="delete-env"
                class={btn}
                disabled={scope() === "" || !lists().includes(scope())}
                onClick={deleteEnv}
              >
                Delete environment
              </button>
            </div>
            <section class="flex min-w-0 flex-1 flex-col gap-2">
              <ul ref={(e) => (list = e)} class="max-h-[50vh] overflow-y-auto">
                <For each={visible()}>
                  {(row) => (
                    <li data-testid="property-row" class="flex flex-wrap items-center gap-2 py-1">
                      <input
                        data-testid="prop-name"
                        aria-label="Property name"
                        autocomplete="off"
                        spellcheck={false}
                        class={`${field} w-40`}
                        value={row.name}
                        aria-invalid={errors().get(row.id) ? "true" : undefined}
                        aria-describedby={errors().get(row.id) ? `row-error-${row.id}` : undefined}
                        onInput={(e) => {
                          const name = e.currentTarget.value;
                          patch(row.id, (r) => {
                            r.name = name;
                            if (r.saved) forget(r); // main refuses `keep` across a rename
                          });
                        }}
                      />
                      <Show
                        when={row.saved}
                        fallback={
                          <input
                            data-testid="prop-value"
                            aria-label="Property value"
                            type={row.secret ? "password" : "text"}
                            autocomplete={row.secret ? "new-password" : "off"}
                            spellcheck={false}
                            class={`${field} flex-1`}
                            value={row.value}
                            aria-invalid={errors().get(row.id) ? "true" : undefined}
                            onInput={(e) => {
                              const value = e.currentTarget.value;
                              patch(row.id, (r) => void (r.value = value));
                            }}
                          />
                        }
                      >
                        <span class="flex flex-1 items-center gap-2">
                          <Show
                            when={row.saved === "set"}
                            fallback={
                              <span
                                data-testid="prop-unreadable"
                                class="text-sm text-amber-700 dark:text-amber-400"
                              >
                                Can't be read, enter it again
                              </span>
                            }
                          >
                            <span
                              data-testid="prop-dots"
                              aria-hidden="true"
                              class="tracking-widest"
                            >
                              ••••••••
                            </span>
                            <span class="sr-only">Saved secret</span>
                          </Show>
                          <button
                            type="button"
                            data-testid="prop-replace"
                            class={btn}
                            disabled={!available()}
                            onClick={(e) => {
                              const item = e.currentTarget.closest("li")!;
                              patch(row.id, forget);
                              flush();
                              item
                                .querySelector<HTMLInputElement>("[data-testid=prop-value]")
                                ?.focus();
                            }}
                          >
                            Replace
                          </button>
                        </span>
                      </Show>
                      <label class="flex items-center gap-1 text-sm">
                        <input
                          type="checkbox"
                          data-testid="prop-secret"
                          checked={row.secret}
                          disabled={!available() && !row.secret}
                          onChange={(e) => {
                            const on = e.currentTarget.checked;
                            patch(row.id, (r) => {
                              r.secret = on;
                              if (!on && r.saved) forget(r);
                            });
                          }}
                        />
                        Secret
                      </label>
                      <button
                        type="button"
                        data-testid="prop-delete"
                        aria-label={`Delete ${row.name || "property"}`}
                        class="rounded px-2 text-base text-red-700 hover:bg-zinc-100 dark:text-red-400 dark:hover:bg-zinc-800"
                        onClick={() =>
                          setRows(
                            (d) =>
                              void d.splice(
                                d.findIndex((r) => r.id === row.id),
                                1,
                              ),
                          )
                        }
                      >
                        ×
                      </button>
                      <Show when={errors().get(row.id)}>
                        <span
                          id={`row-error-${row.id}`}
                          data-testid="row-error"
                          class="w-full text-xs text-red-700 dark:text-red-400"
                        >
                          {errors().get(row.id)}
                        </span>
                      </Show>
                      <Show when={shortSecret(row)}>
                        <span
                          data-testid="row-hint"
                          class="w-full text-xs text-zinc-600 dark:text-zinc-400"
                        >
                          Too short to hide in the console
                        </span>
                      </Show>
                    </li>
                  )}
                </For>
              </ul>
              <div>
                <button type="button" data-testid="add-property" class={btn} onClick={addRow}>
                  + Add property
                </button>
              </div>
              <Show when={!available()}>
                <p
                  data-testid="secrets-unavailable"
                  class="text-xs text-amber-700 dark:text-amber-400"
                >
                  Secret storage is not available on this system
                </p>
              </Show>
              <Show when={rows.some((r) => r.secret)}>
                <p data-testid="secrets-note" class="text-xs text-zinc-600 dark:text-zinc-400">
                  {SECRETS_NOTE}
                </p>
              </Show>
            </section>
          </div>
          <div class="mt-4 flex justify-end gap-2">
            <button
              type="button"
              data-testid="save-properties"
              class={primary}
              disabled={saving() || [...errors().values()].some(Boolean)}
              onClick={() => void save()}
            >
              {saving() ? "Saving…" : "Save"}
            </button>
            <button
              type="button"
              data-testid="cancel-properties"
              class={btn}
              disabled={saving()}
              onClick={close}
            >
              Cancel
            </button>
          </div>
        </Show>
      </dialog>
    </>
  );
}
