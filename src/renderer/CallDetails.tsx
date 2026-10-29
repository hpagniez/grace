import { Check, CircleAlert, Copy } from "lucide";
import { createMemo, createSignal, onCleanup, Show } from "solid-js";
import { curlCommand, httpGroup, prettyBody, type HttpHeader } from "../shared/http";
import { btnSmall } from "./buttons";
import { groupLabel, statusTint } from "./HttpCalls";
import type { ListedCall } from "./http";
import { Icon } from "./Icon";
import { formatMillis, nodeName } from "./nodes";
import { badge, box, TINT } from "./ui";

const BOX = `${box} max-h-80 overflow-auto`;
const SECTION = "text-xs font-medium text-muted";

/** A message's headers as `Name: value` lines, then its body, indented once however often the pane redraws. */
function Message(props: {
  kind: "request" | "response";
  headers: HttpHeader[];
  body: string | null;
}) {
  const body = createMemo(() => (props.body ? prettyBody(props.body) : null));
  return (
    <>
      <Show when={props.headers.length > 0}>
        <pre data-testid={`call-${props.kind}-headers`} class={BOX}>
          {props.headers.map((h) => `${h.name}: ${h.value}`).join("\n")}
        </pre>
      </Show>
      <Show when={body()}>
        {(b) => (
          <pre data-testid={`call-${props.kind}-body`} class={BOX}>
            {b()}
          </pre>
        )}
      </Show>
    </>
  );
}

/** One HTTP call in the details pane: request, response or error, and Copy as cURL. */
export function CallDetails(props: { listed: ListedCall }) {
  const call = () => props.listed.call;
  const test = () => props.listed.test;
  // A heading shows only when something sits under it: a request logged with headers or a body, a response with
  // headers or a body, or (no response) the error.
  const hasRequest = () =>
    call().url !== null && (call().requestHeaders.length > 0 || !!call().requestBody);
  const hasResponse = () =>
    call().status !== null
      ? call().responseHeaders.length > 0 || !!call().responseBody
      : !!call().error;
  const curl = createMemo(() => curlCommand(call()));
  const id = () => `${props.listed.ref.key}\n${props.listed.ref.index}`;

  // Tied to the call it was clicked on, so a switch to another call within 2 s does not claim it was copied.
  const [copy, setCopy] = createSignal<{ id: string; ok: boolean } | null>(null);
  const copied = () => (copy()?.id === id() ? copy() : null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(timer));
  const copyCurl = async () => {
    const command = curl();
    if (command === null) return;
    const at = id();
    let ok = true;
    try {
      await navigator.clipboard.writeText(command);
    } catch {
      ok = false;
    }
    setCopy({ id: at, ok });
    clearTimeout(timer);
    timer = setTimeout(() => setCopy(null), 2000);
  };

  return (
    <div data-testid="call-details" class="space-y-3">
      <h2 class="flex gap-2 font-mono text-lg font-medium break-all">
        <span data-testid="call-method" class="shrink-0">
          {call().method}
        </span>
        <Show when={call().url}>
          <span data-testid="call-url">{call().url}</span>
        </Show>
      </h2>
      <p class="flex items-center gap-3 text-sm">
        {/* Room for the group's name here, where a list row only has its tooltip (R7.2). */}
        <span data-testid="call-status" class={`${badge} ${TINT[statusTint(call().status)]}`}>
          <Show when={call().status !== null} fallback={groupLabel(null)}>
            <span class="font-mono tabular-nums">{call().status}</span>
            <Show when={httpGroup(call().status)}>{` · ${groupLabel(call().status)}`}</Show>
          </Show>
        </span>
        <Show when={call().millis !== null}>
          <span data-testid="call-time" class="font-mono text-xs text-muted tabular-nums">
            {formatMillis(call().millis!)}
          </span>
        </Show>
      </p>
      <div class="space-y-1">
        <p class="text-sm text-muted">{nodeName(test())}</p>
        <p class="font-mono text-xs text-muted" data-testid="call-location">
          {test().line === null ? test().file : `${test().file}:${test().line}`}
        </p>
        <Show when={call().via}>
          <p class="text-xs text-muted" data-testid="call-via">
            {`via ${call().via}`}
          </p>
        </Show>
      </div>
      <button
        type="button"
        data-testid="copy-curl"
        disabled={curl() === null}
        aria-live="polite"
        class={btnSmall}
        onClick={copyCurl}
      >
        <Icon icon={copied()?.ok ? Check : Copy} size={14} />
        {copied() ? (copied()!.ok ? "Copied" : "Copy failed") : "Copy as cURL"}
      </button>
      <Show when={hasRequest()}>
        <h3 class={SECTION}>Request</h3>
        <Message kind="request" headers={call().requestHeaders} body={call().requestBody} />
      </Show>
      <Show when={hasResponse()}>
        <h3 class={SECTION}>Response</h3>
        <Show
          when={call().status !== null}
          fallback={
            <div class={`rounded-lg p-3 ${TINT.failed}`}>
              <p class="flex items-center gap-1.5 text-sm font-semibold">
                <Icon icon={CircleAlert} size={14} />
                Error
              </p>
              <pre
                data-testid="call-error"
                class="mt-1 font-mono text-xs break-words whitespace-pre-wrap text-ink"
              >
                {call().error}
              </pre>
            </div>
          }
        >
          <Message kind="response" headers={call().responseHeaders} body={call().responseBody} />
        </Show>
      </Show>
    </div>
  );
}
