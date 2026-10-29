import type { JSX } from "@solidjs/web";
import { ArrowLeft, Check, CircleArrowUp, Download } from "lucide";
import { createSignal, For, onCleanup, onSettled, Show } from "solid-js";
import type { Api } from "../shared/ipc";
import { parseTheme, type Theme } from "../shared/theme";
import { channelOf, onChannel, type Release, type UpdateView } from "../shared/updates";
import { compareVersions, isPrerelease } from "../shared/version";
import { btnSmall, ghost, primary } from "./buttons";
import { FieldMessage } from "./FieldMessage";
import { Icon } from "./Icon";
import { lastCheckedText } from "./lastChecked";
import type { AppState } from "./reduce";
import { SwitchDialog } from "./SwitchDialog";
import { Tooltip } from "./Tooltip";
import { badge, link, sheet, TINT } from "./ui";
import { latestAvailable, updateBlocked } from "./updateState";

const legend = "text-sm font-medium";
/** A radio button or a checkbox with its label: a 24 px row. */
const choice = "flex h-6 items-center gap-2 text-sm";

const SECTIONS = [
  { id: "updates", label: "Updates" },
  { id: "appearance", label: "Appearance" },
] as const;
const THEME_CHOICES: { theme: Theme; label: string }[] = [
  { theme: "system", label: "Follow Windows" },
  { theme: "light", label: "Light" },
  { theme: "dark", label: "Dark" },
];

/** Settings › Appearance (spec R8): the theme main applies to the whole window at once and saves. */
function Appearance(props: { api: Api; hidden: boolean }) {
  const [theme, setTheme] = createSignal<Theme>("system");
  onSettled(() => void props.api.getAppearance().then((t) => setTheme(parseTheme(t))));
  return (
    <section
      aria-labelledby="settings-appearance"
      hidden={props.hidden}
      class="min-h-0 min-w-0 flex-1 overflow-auto px-6 py-5"
    >
      <h2 id="settings-appearance" class="text-lg font-semibold">
        Appearance
      </h2>
      <fieldset class="mt-4">
        <legend class={legend}>Theme</legend>
        <div class="mt-1">
          <For each={THEME_CHOICES}>
            {(c) => (
              <label class={choice}>
                <input
                  type="radio"
                  name="theme"
                  value={c.theme}
                  data-testid={`theme-${c.theme}`}
                  class="accent-accent"
                  checked={theme() === c.theme}
                  onChange={() => {
                    setTheme(c.theme);
                    void props.api.setAppearance(c.theme);
                  }}
                />
                {c.label}
              </label>
            )}
          </For>
        </div>
      </fieldset>
    </section>
  );
}

/** What the table lists (R6.3): the channel's releases plus the running version, newest first. */
function rowsOf(u: UpdateView): Release[] {
  const all = u.releases ?? [];
  const rows = [...onChannel(all, channelOf(u.settings, u.running))]; // a copy: the channel may hand back `all` itself
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
 * The Settings page (app-updates R6), which replaces the view until Back or Escape: a header on the
 * ground, then a sheet with the list of sections and the section (spec R21). Updates, the one it opens
 * on, shows the running version, the check, the channel and startup settings, and the versions to switch
 * to. Appearance (spec R8) holds the theme.
 */
export function Settings(props: {
  state: AppState;
  api: Api;
  /** The app's notice, shown on the page while it covers the view. */
  notice?: JSX.Element;
  back(): void;
}) {
  // A new object per question, so the dialog starts afresh each time; kept after the answer, so the
  // dialog keeps its words while it fades out (R5.2).
  const [question, setQuestion] = createSignal<{ target: Release } | null>(null);
  const [asking, setAsking] = createSignal(false);
  const ask = (target: Release) => {
    setQuestion({ target });
    setAsking(true);
  };
  const blocked = () => updateBlocked(props.state);
  const [section, setSection] = createSignal<(typeof SECTIONS)[number]["id"]>("updates");

  // Escape goes back, unless it closes a dialog (the SwitchDialog, or one of App's) or a tooltip. In the
  // bubble phase: a tooltip takes its Escape in the capture phase and stops it there.
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape" && !e.defaultPrevented && !document.querySelector("dialog[open]"))
      props.back();
  };
  document.addEventListener("keydown", onKey);
  onCleanup(() => document.removeEventListener("keydown", onKey));

  return (
    <div data-testid="settings" class="flex min-h-0 flex-1 flex-col">
      <header class="flex min-h-11 shrink-0 items-center gap-2 px-3">
        <button
          type="button"
          data-testid="settings-back"
          class={ghost}
          onClick={() => props.back()}
        >
          <Icon icon={ArrowLeft} />
          Back
        </button>
        <h1 class="text-base font-semibold">Settings</h1>
      </header>
      <div class={sheet}>
        {props.notice}
        <div class="flex min-h-0 flex-1">
          <nav aria-label="Settings sections" class="w-48 shrink-0 border-r border-line p-2">
            <ul>
              <For each={SECTIONS}>
                {(s) => (
                  <li>
                    <button
                      type="button"
                      data-testid={`settings-section-${s.id}`}
                      aria-current={section() === s.id ? "page" : undefined}
                      class={`flex h-7 w-full items-center rounded-sm px-2 text-left text-sm transition-colors duration-(--duration-fast) ease-standard ${section() === s.id ? "bg-selected font-medium shadow-[inset_2px_0_0_var(--color-accent)]" : "hover:bg-hover"}`}
                      onClick={() => setSection(s.id)}
                    >
                      {s.label}
                    </button>
                  </li>
                )}
              </For>
            </ul>
          </nav>
          <section
            aria-labelledby="settings-updates"
            hidden={section() !== "updates"}
            class="min-h-0 min-w-0 flex-1 overflow-auto px-6 py-5"
          >
            <h2 id="settings-updates" class="text-lg font-semibold">
              Updates
            </h2>
            <Show when={props.state.updates}>
              {(u) => (
                <div class="mt-4 max-w-3xl">
                  <div class="flex items-start justify-between gap-4">
                    <div class="min-w-0">
                      <div class="flex items-center gap-2">
                        <p data-testid="settings-running" class="text-base font-semibold">
                          {`Grace ${u().running}`}
                        </p>
                        <Show when={latestAvailable(u())}>
                          {(v) => (
                            <span data-testid="settings-marker" class={`${badge} ${TINT.running}`}>
                              <Icon icon={CircleArrowUp} size={12} />
                              {`${v()} available`}
                            </span>
                          )}
                        </Show>
                      </div>
                      <Show when={u().error}>
                        {(e) => (
                          <FieldMessage kind="error" testid="settings-error" class="mt-1">
                            {e()}
                          </FieldMessage>
                        )}
                      </Show>
                      <p class="mt-1 text-sm text-muted">
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
                        <Tooltip text={blocked()} describe class="shrink-0">
                          <button
                            type="button"
                            data-testid="settings-update"
                            class={primary}
                            disabled={!!blocked()}
                            onClick={() => void props.api.installVersion(v(), false)}
                          >
                            <Icon icon={Download} />
                            {`Update to ${v()}`}
                          </button>
                        </Tooltip>
                      )}
                    </Show>
                  </div>

                  <fieldset class="mt-6">
                    <legend class={legend}>Channel</legend>
                    <label class={`mt-1 ${choice}`}>
                      <input
                        type="radio"
                        name="channel"
                        data-testid="channel-stable"
                        class="accent-accent"
                        checked={channelOf(u().settings, u().running) === "stable"}
                        onChange={() => void props.api.setUpdateSettings({ channel: "stable" })}
                      />
                      Stable releases only
                    </label>
                    <label class={choice}>
                      <input
                        type="radio"
                        name="channel"
                        data-testid="channel-prerelease"
                        class="accent-accent"
                        checked={channelOf(u().settings, u().running) === "prerelease"}
                        onChange={() => void props.api.setUpdateSettings({ channel: "prerelease" })}
                      />
                      Stable and pre-releases
                    </label>
                  </fieldset>

                  <fieldset class="mt-6">
                    <legend class={legend}>At startup</legend>
                    <label class={`mt-1 ${choice}`}>
                      <input
                        type="checkbox"
                        data-testid="check-at-startup"
                        class="accent-accent"
                        checked={u().settings.checkAtStartup}
                        onChange={(e) =>
                          void props.api.setUpdateSettings({
                            checkAtStartup: e.currentTarget.checked,
                          })
                        }
                      />
                      Check for updates
                    </label>
                    <label class={`ml-6 ${choice} has-disabled:text-muted`}>
                      <input
                        type="checkbox"
                        data-testid="auto-update"
                        class="accent-accent disabled:opacity-40"
                        checked={u().settings.autoUpdate}
                        disabled={!u().settings.checkAtStartup}
                        onChange={(e) =>
                          void props.api.setUpdateSettings({ autoUpdate: e.currentTarget.checked })
                        }
                      />
                      Update automatically, without asking
                    </label>
                  </fieldset>

                  <h3 class={`${legend} mt-6`}>Versions</h3>
                  <table data-testid="versions" class="mt-2 w-full border-b border-line text-sm">
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
                            else ask(r);
                          };
                          return (
                            <tr
                              data-testid="version-row"
                              data-version={r.version}
                              class="h-8 border-t border-line"
                            >
                              <th scope="row" class="pr-4 text-left font-mono font-medium">
                                {r.version}
                              </th>
                              <td data-testid="version-date" class="pr-4 text-muted tabular-nums">
                                {r.publishedAt.slice(0, 10)}
                              </td>
                              <td class="pr-4">
                                <span class="flex flex-wrap gap-1">
                                  <Show when={r.prerelease}>
                                    <span class={`${badge} ${TINT.skipped}`}>pre-release</span>
                                  </Show>
                                  <Show when={order() === 0}>
                                    <span class={`${badge} ${TINT.passed}`}>
                                      <Icon icon={Check} size={12} />
                                      installed
                                    </span>
                                  </Show>
                                  <Show when={saved()}>
                                    <span class={`${badge} ${TINT.running}`}>data saved</span>
                                  </Show>
                                </span>
                              </td>
                              <td class="text-right">
                                <Show when={order() !== 0}>
                                  <Tooltip text={blocked()} describe>
                                    <button
                                      type="button"
                                      data-testid="version-action"
                                      class={btnSmall}
                                      disabled={!!blocked()}
                                      onClick={pick}
                                    >
                                      {order() > 0 ? "Install" : "Roll back"}
                                    </button>
                                  </Tooltip>
                                </Show>
                              </td>
                            </tr>
                          );
                        }}
                      </For>
                    </tbody>
                  </table>
                  <Show when={u().releases === null}>
                    <p class="mt-2 text-sm text-muted">Check for updates to see other versions</p>
                  </Show>

                  <Show when={question()} keyed>
                    {(q) => (
                      <SwitchDialog
                        open={asking()}
                        target={q.target}
                        running={u().running}
                        backup={u().backups.find((b) => b.version === q.target.version) ?? null}
                        onConfirm={(useSavedData) => {
                          setAsking(false);
                          void props.api.installVersion(q.target.version, useSavedData);
                        }}
                        onCancel={() => setAsking(false)}
                      />
                    )}
                  </Show>
                </div>
              )}
            </Show>
          </section>
          <Appearance api={props.api} hidden={section() !== "appearance"} />
        </div>
      </div>
    </div>
  );
}
