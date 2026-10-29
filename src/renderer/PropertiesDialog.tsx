import { CircleAlert, Lock, Plus, SlidersHorizontal, Trash2 } from "lucide";
import { createMemo, createSignal, createStore, flush, For, onCleanup, Show } from "solid-js";
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
import { btn, btnDialog, btnIconSmall, narrowIcon, narrowLabel, primaryDialog } from "./buttons";
import { Cut } from "./Cut";
import { INVALID_ENV } from "./EnvPicker";
import { FieldMessage } from "./FieldMessage";
import { Icon } from "./Icon";
import { createNarrow } from "./narrow";
import type { AppState } from "./reduce";
import { Tooltip } from "./Tooltip";
import { dialog as dialogStyle, input } from "./ui";

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

/** The dialog fades out over 100 ms (R5.2): its content stays this long after it closes. */
const EXIT_MS = 120;
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
  const narrow = createNarrow(); // the button's label is hidden then: its tooltip says it
  let list!: HTMLUListElement;
  let session = 0; // each opening; an answer to an earlier one is ignored
  let exit: ReturnType<typeof setTimeout> | undefined; // drops the draft once the dialog has faded out
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
    clearTimeout(exit); // reopened while it still fades out: this draft is the new one
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
  // Closing drops the draft, so a typed secret does not linger in the window's memory. The content
  // stays through the exit, or the box would fade out empty; the timer drops it once the dialog is gone.
  const drop = () => {
    setOpen(false);
    setRows((d) => void d.splice(0, d.length));
    setLists([]);
    setScope("");
    setNewEnv("");
    setEnvError(null);
  };
  const dropLater = () => {
    clearTimeout(exit);
    exit = setTimeout(drop, EXIT_MS);
  };
  onCleanup(() => clearTimeout(exit));
  const close = () => {
    dialog.close();
    dropLater();
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
      <Tooltip text={narrow() ? "Properties…" : undefined}>
        <button
          type="button"
          data-testid="properties-button"
          class={`${btn} ${narrowIcon}`}
          disabled={locked() || !view()}
          onClick={show}
        >
          <Icon icon={SlidersHorizontal} />
          <span class={narrowLabel}>Properties…</span>
        </button>
      </Tooltip>
      <dialog
        ref={(e) => (dialog = e)}
        data-testid="properties-dialog"
        aria-labelledby="properties-title"
        class={`${dialogStyle} w-[48rem] max-w-[95vw]`}
        onClose={() => dialog.open || dropLater()} // Escape; not a late event after a reopen
      >
        <Show when={open()}>
          <h2 id="properties-title" class="mb-4 text-lg font-semibold">
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
                        class={`flex w-full items-center gap-1 rounded-sm px-2 py-1 text-left text-sm transition-colors duration-(--duration-fast) ease-standard ${s === scope() ? "bg-selected font-medium shadow-[inset_2px_0_0_var(--color-accent)]" : "hover:bg-hover"}`}
                        onClick={() => setScope(s)}
                      >
                        <Cut full={s === "" ? "All environments" : s} wrap="flex-1" class="flex-1">
                          {s === "" ? "All environments" : s}
                        </Cut>
                        <Show when={invalidScopes().has(s)}>
                          <span class="sr-only"> has errors</span>
                          <Icon icon={CircleAlert} size={14} class="text-failed" />
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
                  class={`${input} min-w-0 flex-1`}
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
                <FieldMessage kind="error" testid="add-env-error">
                  {envError()}
                </FieldMessage>
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
              {/* -mx-1 px-1: room for the focus ring of the first and last column, which the scroll would clip */}
              <ul ref={(e) => (list = e)} class="-mx-1 max-h-[50vh] overflow-y-auto px-1">
                <For each={visible()}>
                  {(row) => (
                    <li data-testid="property-row" class="flex flex-wrap items-center gap-2 py-1">
                      <input
                        data-testid="prop-name"
                        aria-label="Property name"
                        autocomplete="off"
                        spellcheck={false}
                        class={`${input} w-40 min-w-0`}
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
                            class={`${input} min-w-0 flex-1`}
                            value={row.value}
                            aria-invalid={errors().get(row.id) ? "true" : undefined}
                            aria-describedby={
                              errors().get(row.id) ? `row-error-${row.id}` : undefined
                            }
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
                              <FieldMessage kind="warning" testid="prop-unreadable">
                                Can't be read, enter it again
                              </FieldMessage>
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
                          class="accent-accent"
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
                      <Tooltip text={`Delete ${row.name || "property"}`}>
                        <button
                          type="button"
                          data-testid="prop-delete"
                          aria-label={`Delete ${row.name || "property"}`}
                          class={btnIconSmall}
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
                          <Icon icon={Trash2} />
                        </button>
                      </Tooltip>
                      <Show when={errors().get(row.id)}>
                        <FieldMessage
                          kind="error"
                          id={`row-error-${row.id}`}
                          testid="row-error"
                          class="w-full"
                        >
                          {errors().get(row.id)}
                        </FieldMessage>
                      </Show>
                      <Show when={shortSecret(row)}>
                        <FieldMessage kind="hint" testid="row-hint" class="w-full">
                          Too short to hide in the console
                        </FieldMessage>
                      </Show>
                    </li>
                  )}
                </For>
              </ul>
              <div>
                <button type="button" data-testid="add-property" class={btn} onClick={addRow}>
                  <Icon icon={Plus} />
                  Add property
                </button>
              </div>
              <Show when={!available()}>
                <FieldMessage kind="warning" testid="secrets-unavailable">
                  Secret storage is not available on this system
                </FieldMessage>
              </Show>
              <Show when={rows.some((r) => r.secret)}>
                <FieldMessage kind="hint" icon={Lock} testid="secrets-note">
                  {SECRETS_NOTE}
                </FieldMessage>
              </Show>
            </section>
          </div>
          <div class="mt-5 flex justify-end gap-2">
            <button
              type="button"
              data-testid="save-properties"
              class={primaryDialog}
              disabled={saving() || [...errors().values()].some(Boolean)}
              onClick={() => void save()}
            >
              {saving() ? "Saving…" : "Save"}
            </button>
            <button
              type="button"
              data-testid="cancel-properties"
              class={btnDialog}
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
