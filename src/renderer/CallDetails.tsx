import { createMemo, createSignal, onCleanup, Show } from "solid-js";
import { curlCommand, HTTP_GROUPS, httpGroup, prettyBody, type HttpHeader } from "../shared/http";
import type { ListedCall } from "./http";
import { formatMillis, nodeName } from "./nodes";

const BOX = "max-h-80 overflow-auto rounded bg-zinc-100 p-2 font-mono text-xs dark:bg-zinc-800";

const statusText = (status: number | null): string => {
  if (status === null) return "No response";
  const g = httpGroup(status);
  return g ? `${status} · ${HTTP_GROUPS.find((x) => x.group === g)!.label}` : String(status);
};

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
  const [copy, setCopy] = createSignal<{ id: string; label: string } | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => clearTimeout(timer));
  const copyCurl = async () => {
    const command = curl();
    if (command === null) return;
    const at = id();
    let label = "Copied";
    try {
      await navigator.clipboard.writeText(command);
    } catch {
      label = "Copy failed";
    }
    setCopy({ id: at, label });
    clearTimeout(timer);
    timer = setTimeout(() => setCopy(null), 2000);
  };

  return (
    <div data-testid="call-details" class="space-y-2">
      <h2 class="flex gap-2 break-all font-mono text-sm">
        <span data-testid="call-method" class="shrink-0 font-semibold">
          {call().method}
        </span>
        <Show when={call().url}>
          <span data-testid="call-url">{call().url}</span>
        </Show>
      </h2>
      <p class="flex items-center gap-3 text-sm">
        <span data-testid="call-status">{statusText(call().status)}</span>
        <Show when={call().millis !== null}>
          <span data-testid="call-time">{formatMillis(call().millis!)}</span>
        </Show>
      </p>
      <p class="text-sm">{nodeName(test())}</p>
      <p class="font-mono text-sm text-zinc-500" data-testid="call-location">
        {test().line === null ? test().file : `${test().file}:${test().line}`}
      </p>
      <Show when={call().via}>
        <p class="text-sm text-zinc-600 dark:text-zinc-400" data-testid="call-via">
          {`via ${call().via}`}
        </p>
      </Show>
      <button
        type="button"
        data-testid="copy-curl"
        disabled={curl() === null}
        aria-live="polite"
        class="rounded border border-zinc-300 px-2 py-0.5 text-sm hover:bg-zinc-100 disabled:cursor-not-allowed disabled:opacity-50 disabled:hover:bg-transparent dark:border-zinc-600 dark:hover:bg-zinc-800"
        onClick={copyCurl}
      >
        {copy()?.id === id() ? copy()!.label : "Copy as cURL"}
      </button>
      <Show when={hasRequest()}>
        <h3 class="text-sm font-semibold">Request</h3>
        <Message kind="request" headers={call().requestHeaders} body={call().requestBody} />
      </Show>
      <Show when={hasResponse()}>
        <h3 class="text-sm font-semibold">Response</h3>
        <Show
          when={call().status !== null}
          fallback={
            <pre
              data-testid="call-error"
              class="whitespace-pre-wrap rounded border border-red-300 bg-red-50 p-2 text-xs text-red-800 dark:border-red-800 dark:bg-red-950 dark:text-red-200"
            >
              {call().error}
            </pre>
          }
        >
          <Message kind="response" headers={call().responseHeaders} body={call().responseBody} />
        </Show>
      </Show>
    </div>
  );
}
