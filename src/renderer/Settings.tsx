import { createSignal, For, onCleanup, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { onChannel, type Release, type UpdateView } from "../shared/updates";
import { compareVersions, isPrerelease } from "../shared/version";
import { btn, btnSmall, primary } from "./buttons";
import { lastCheckedText } from "./lastChecked";
import type { AppState } from "./reduce";
import { SwitchDialog } from "./SwitchDialog";
import { latestAvailable, updateBlocked } from "./updateState";

const tag = "rounded px-1.5 py-px text-[11px]";
const link =
  "rounded text-sky-700 underline hover:text-sky-900 disabled:cursor-not-allowed disabled:no-underline disabled:opacity-60 dark:text-sky-400 dark:hover:text-sky-300";
const legend = "text-sm font-medium";

/** What the table lists (R6.3): the channel's releases plus the running version, newest first. */
function rowsOf(u: UpdateView): Release[] {
  const all = u.releases ?? [];
  const rows = [...onChannel(all, u.settings.channel)]; // a copy: the channel may hand back `all` itself
  if (!rows.some((r) => r.version === u.running))
    rows.push(
      all.find((r) => r.version === u.running) ?? {
        // never installed, so the fields an install needs stay empty
        version: u.running,
        prerelease: isPrerelease(u.running),
        publishedAt: "",
        url: "",
        size: 0,
        sha256: "",
      },
    );
  return rows.sort((a, b) => compareVersions(b.version, a.version));
}

/**
 * The Settings page (R6), which replaces the view until Back or Escape. Its one section, Updates, shows
 * the running version, the check, the channel and startup settings, and the versions to switch to.
 */
export function Settings(props: { state: AppState; api: Api; back(): void }) {
  const [switching, setSwitching] = createSignal<Release | null>(null);
  const blocked = () => updateBlocked(props.state);

  // Escape goes back, unless it closes a dialog (the SwitchDialog, or one of App's)
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape" && !document.querySelector("dialog[open]")) props.back();
  };
  document.addEventListener("keydown", onKey);
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <div data-testid="settings" class="flex min-h-0 flex-1 flex-col">
      <header class="flex shrink-0 items-center gap-3 border-b border-zinc-200 px-3 py-2 dark:border-zinc-700">
        <button type="button" data-testid="settings-back" class={btn} onClick={() => props.back()}>
          ← Back
        </button>
        <h1 class="text-base font-semibold">Settings</h1>
      </header>
      <div class="flex min-h-0 flex-1">
        <nav
          aria-label="Settings sections"
          class="w-40 shrink-0 border-r border-zinc-200 py-2 dark:border-zinc-700"
        >
          <ul>
            <li>
              <span
                aria-current="page"
                class="block border-l-2 border-sky-600 bg-sky-50 px-3 py-1.5 text-sm text-sky-700 dark:bg-sky-950 dark:text-sky-300"
              >
                Updates
              </span>
            </li>
          </ul>
        </nav>
        <section aria-label="Updates" class="min-h-0 min-w-0 flex-1 overflow-auto p-4">
          <Show when={props.state.updates}>
            {(u) => (
              <div class="max-w-3xl">
                <div class="flex items-start justify-between gap-4">
                  <div>
                    <div class="flex items-center gap-2">
                      <p data-testid="settings-running" class="text-base font-semibold">
                        {`Grace ${u().running}`}
                      </p>
                      <Show when={latestAvailable(u())}>
                        {(v) => (
                          <span
                            data-testid="settings-marker"
                            class={`${tag} bg-amber-100 text-amber-900 dark:bg-amber-950 dark:text-amber-300`}
                          >
                            {`${v()} available`}
                          </span>
                        )}
                      </Show>
                    </div>
                    <Show when={u().error}>
                      {(e) => (
                        <p
                          data-testid="settings-error"
                          class="mt-1 text-sm text-red-700 dark:text-red-400"
                        >
                          {e()}
                        </p>
                      )}
                    </Show>
                    <p class="mt-1 text-sm text-zinc-500 dark:text-zinc-400">
                      <Show when={lastCheckedText(u().settings.lastChecked, Date.now())}>
                        {(t) => (
                          <>
                            <span data-testid="settings-last-checked">{t()}</span>
                            {" · "}
                          </>
                        )}
                      </Show>
                      <button
                        type="button"
                        data-testid="settings-check"
                        class={link}
                        disabled={u().checking}
                        onClick={() => void props.api.checkUpdates()}
                      >
                        {u().checking ? "Checking…" : "Check for updates"}
                      </button>
                    </p>
                  </div>
                  <Show when={latestAvailable(u())}>
                    {(v) => (
                      <button
                        type="button"
                        data-testid="settings-update"
                        class={primary}
                        disabled={!!blocked()}
                        title={blocked()}
                        onClick={() => void props.api.installVersion(v(), false)}
                      >
                        {`Update to ${v()}`}
                      </button>
                    )}
                  </Show>
                </div>

                <fieldset class="mt-5">
                  <legend class={legend}>Channel</legend>
                  <label class="mt-1 flex items-center gap-2 text-sm">
                    <input
                      type="radio"
                      name="channel"
                      data-testid="channel-stable"
                      checked={u().settings.channel === "stable"}
                      onChange={() => void props.api.setUpdateSettings({ channel: "stable" })}
                    />
                    Stable releases only
                  </label>
                  <label class="mt-1 flex items-center gap-2 text-sm">
                    <input
                      type="radio"
                      name="channel"
                      data-testid="channel-prerelease"
                      checked={u().settings.channel === "prerelease"}
                      onChange={() => void props.api.setUpdateSettings({ channel: "prerelease" })}
                    />
                    Stable and pre-releases
                  </label>
                </fieldset>

                <fieldset class="mt-5">
                  <legend class={legend}>At startup</legend>
                  <label class="mt-1 flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      data-testid="check-at-startup"
                      checked={u().settings.checkAtStartup}
                      onChange={(e) =>
                        void props.api.setUpdateSettings({
                          checkAtStartup: e.currentTarget.checked,
                        })
                      }
                    />
                    Check for updates
                  </label>
                  <label class="mt-1 ml-6 flex items-center gap-2 text-sm has-disabled:text-zinc-500 dark:has-disabled:text-zinc-400">
                    <input
                      type="checkbox"
                      data-testid="auto-update"
                      checked={u().settings.autoUpdate}
                      disabled={!u().settings.checkAtStartup}
                      onChange={(e) =>
                        void props.api.setUpdateSettings({ autoUpdate: e.currentTarget.checked })
                      }
                    />
                    Update automatically, without asking
                  </label>
                </fieldset>

                <h2 class={`${legend} mt-5`}>Versions</h2>
                <table data-testid="versions" class="mt-1 w-full text-sm">
                  <thead class="sr-only">
                    <tr>
                      <th>Version</th>
                      <th>Released</th>
                      <th>Tags</th>
                      <th>Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={rowsOf(u())}>
                      {(r) => {
                        const order = () => compareVersions(r.version, u().running);
                        const saved = () => u().backups.find((b) => b.version === r.version);
                        const pick = () => {
                          // a newer version installs at once, unless it has saved data to choose (R6.5)
                          if (order() > 0 && !saved())
                            void props.api.installVersion(r.version, false);
                          else setSwitching(r);
                        };
                        return (
                          <tr
                            data-testid="version-row"
                            data-version={r.version}
                            class="border-t border-zinc-200 dark:border-zinc-700"
                          >
                            <th scope="row" class="py-1.5 pr-4 text-left font-semibold">
                              {r.version}
                            </th>
                            <td
                              data-testid="version-date"
                              class="pr-4 text-zinc-500 dark:text-zinc-400"
                            >
                              {r.publishedAt.slice(0, 10)}
                            </td>
                            <td class="pr-4">
                              <span class="flex flex-wrap gap-1">
                                <Show when={r.prerelease}>
                                  <span
                                    class={`${tag} bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400`}
                                  >
                                    pre-release
                                  </span>
                                </Show>
                                <Show when={order() === 0}>
                                  <span
                                    class={`${tag} bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-300`}
                                  >
                                    installed
                                  </span>
                                </Show>
                                <Show when={saved()}>
                                  <span
                                    class={`${tag} bg-sky-100 text-sky-800 dark:bg-sky-950 dark:text-sky-300`}
                                  >
                                    data saved
                                  </span>
                                </Show>
                              </span>
                            </td>
                            <td class="py-1 text-right">
                              <Show when={order() !== 0}>
                                <button
                                  type="button"
                                  data-testid="version-action"
                                  class={btnSmall}
                                  disabled={!!blocked()}
                                  title={blocked()}
                                  onClick={pick}
                                >
                                  {order() > 0 ? "Install" : "Roll back"}
                                </button>
                              </Show>
                            </td>
                          </tr>
                        );
                      }}
                    </For>
                  </tbody>
                </table>
                <Show when={u().releases === null}>
                  <p class="mt-2 text-sm text-zinc-500 dark:text-zinc-400">
                    Check for updates to see other versions
                  </p>
                </Show>

                <Show when={switching()}>
                  {(t) => (
                    <SwitchDialog
                      target={t()}
                      running={u().running}
                      backup={u().backups.find((b) => b.version === t().version) ?? null}
                      onConfirm={(useSavedData) => {
                        const version = t().version; // `t` ends with the dialog
                        setSwitching(null);
                        void props.api.installVersion(version, useSavedData);
                      }}
                      onCancel={() => setSwitching(null)}
                    />
                  )}
                </Show>
              </div>
            )}
          </Show>
        </section>
      </div>
    </div>
  );
}
