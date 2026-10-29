import { Match, Show, Switch } from "solid-js";
import { bannerState, testsText } from "./bannerText";
import type { AppState } from "./reduce";

const v = __KK_VERSIONS__;
const Label = (props: { text: string }) => <span class="text-muted">{props.text.padEnd(9)}</span>;

/** The console's first lines: versions, project, tests found, and what the app is doing (F24). */
export function Banner(props: { state: AppState }) {
  const tests = () => testsText(props.state);
  const state = () => bannerState(props.state);
  return (
    <div data-testid="console-banner">
      <div data-testid="banner-versions">
        <span class="font-semibold text-accent">Grace</span>
        {` ${v.app} · Karate ${v.karate} · Java ${v.java}`}
      </div>
      <div data-testid="banner-project">
        <Label text="project" />
        {props.state.project?.root ?? "none"}
      </div>
      <Show when={tests()}>
        {(t) => (
          <div data-testid="banner-tests">
            <Label text="tests" />
            {t()}
          </div>
        )}
      </Show>
      <Show when={state()}>
        {(s) => (
          <div data-testid="console-state" class="text-muted">
            <Switch>
              <Match when={s() === "none"}>Open a Karate project folder</Match>
              <Match when={s() === "finding"}>Finding tests…</Match>
              <Match when={s() === "starting"}>Starting Karate…</Match>
              <Match when={s() === "ready"}>
                <span class="text-passed">Ready.</span> Press Run all, or ▶ on a test.
              </Match>
            </Switch>
            <span
              data-testid="console-cursor"
              aria-hidden="true"
              class="ml-1 inline-block h-3 w-[0.6em] translate-y-0.5 animate-blink bg-ink motion-reduce:animate-none"
            />
          </div>
        )}
      </Show>
    </div>
  );
}
