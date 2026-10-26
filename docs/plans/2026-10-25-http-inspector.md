# HTTP Inspector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** After each feature of a run ends, list every HTTP call its tests made in a new HTTP tab, grouped by status class and narrowed by the tree selection, and show a selected call's request and response in the details pane with Copy as cURL.

**Architecture:** Pure rules (the call type, status groups, cURL text, body indenting) live in `src/shared/http.ts`. Main parses the step logs of each depth-0 `FEATURE_EXIT`, nested `callResults` included, in `src/main/karate/http.ts`, and `RunState` sends a `calls` event per test next to its `steps` event; `maskEvent` masks it like every event. The window keeps calls per test key as it keeps steps, derives the narrowed and filtered list in `src/renderer/http.ts`, and shows it in a third bottom-panel tab and a call view in the details pane.

**Tech Stack:** Electron 45, Solid 2 RC, Tailwind v4, TypeScript, Vitest 5, Playwright 1.64, bundled Temurin 21 + Karate 2.1.4.

**Spec:** [docs/specs/2026-10-25-http-inspector.md](../specs/2026-10-25-http-inspector.md) (R1–R6). Builds on the walking-skeleton, environment-switcher and run-history specs.

## Global Constraints

- Branch `feature/http-inspector` (the spec commit is already on it). Conventional Commits; AGENTS.md rules, including the Solid 2 rules.
- No new dependencies. No new IPC channel: calls reach the window only through events. The clipboard is written with `navigator.clipboard.writeText` in the renderer.
- Parse rules: spec R1, word for word, plus one guard (Review Focus 1): a `request:` line starts a call only when the next line matches `^\d+ > `.
- Status groups, in this order, with these labels: `1xx Informational`, `2xx Success`, `3xx Redirect`, `4xx Client error`, `5xx Server error`, `No response`; plus `All` first. A code outside 100–599 is in no group (counts in All only).
- Badge colours (white text, AA contrast): 1xx `bg-zinc-600`, 2xx `bg-green-700`, 3xx `bg-sky-700`, 4xx `bg-amber-700`, 5xx `bg-red-600`, No response `bg-red-900`, outside 100–599 `bg-zinc-600`.
- cURL: `curl -X <METHOD> '<URL>'` (for `HEAD`, any case: `curl --head '<URL>'` and no body, because `-X HEAD` makes curl wait for a body), then ` -H '<Name>: <value>'` per request header in logged order except `Host`, `Content-Length`, `Connection` (case-insensitive), then ` --data-raw '<body>'` when there is a body, as logged. Every `'` inside a value is written `'\''`. Masked values (`***`, `****`) are copied unchanged.
- Exact copy:
  - tab `HTTP (<n>)` after `Console` and `History (<n>)`; tablist label `Console, run history and HTTP calls`
  - chips `All <n>`, `<group label> <n>`
  - row: `—` badge for No response; `attempt <i> of <n>`; `via <file>:<line>`; time with `formatMillis`, `—` when none
  - empty states `No HTTP calls yet. Calls appear when a feature finishes.`, `No HTTP calls in <node name>`, `No calls in the chosen groups`
  - details: status `<code> · <group label>` or `No response`; sections `Request`, `Response`; button `Copy as cURL`, then `Copied` for 2 s, `Copy failed` if the write rejects
- Never run `npm test` and `npm run e2e` at the same time.

## Review Focus

1. **A body that contains a line `request:`** (a log dump, a text API): the body stays whole and no extra call appears. Test in Task 3 (`a request: line inside a body does not start a call`).
2. **CRLF bodies** (a server answering with `\r\n` lines): parsed like `\n`; no line of a header or body ends with `\r`. Test in Task 3 (`reads CRLF lines`).
3. **Bodies that are neither clean JSON nor clean XML** (`{not json`, HTML with unclosed tags, XML with `>` in an attribute or a CDATA section, an empty body): shown exactly as logged, never throws. Test in Task 1 (`prettyBody leaves anything else as logged`).
4. **Quotes, `$`, backticks and newlines in a URL, header or body**: the copied command passes them literally to curl in bash. Test in Task 1 (`curlCommand quotes every value`).
5. **The selected call disappears** (its test re-runs, a chip hides it, the environment changes, the project changes): the details pane goes back to the node and nothing throws. Tests in Task 5 (`runStarted drops the selected call of a test it runs`, `toggleGroup drops a call it hides`).

---

### Task 1: Shared call model, status groups, cURL and body indenting

**Files:**
- Create: `src/shared/http.ts`, `src/shared/http.test.ts`

**Interfaces:**
- Produces (no Node imports):
  - `interface HttpHeader { name: string; value: string }`
  - `interface HttpCall { method: string; url: string | null; requestHeaders: HttpHeader[]; requestBody: string | null; status: number | null; responseHeaders: HttpHeader[]; responseBody: string | null; millis: number | null; error: string | null; via: string | null; startTime: number; attempt: { n: number; of: number } | null }`: `url` null when no request was logged; `status` null for No response; `via` like `tests/common/create-user.feature:8`.
  - `type HttpGroup = "1xx" | "2xx" | "3xx" | "4xx" | "5xx" | "none"`
  - `HTTP_GROUPS: readonly { group: HttpGroup; label: string }[]` in Global Constraints order
  - `httpGroup(status: number | null): HttpGroup | null`
  - `curlCommand(c: HttpCall): string | null`: null when `url` is null
  - `prettyBody(body: string): string`

- [ ] **Step 1: Write the failing tests** in `src/shared/http.test.ts`:
  - `httpGroup classifies by hundred`: 100 and 199 → `1xx`, 200 → `2xx`, 302 → `3xx`, 404 → `4xx`, 500 and 599 → `5xx`, `null` → `none`, 99 and 600 → `null`
  - `HTTP_GROUPS lists the six groups in order`: labels equal the Global Constraints list
  - `curlCommand drops the headers curl sets`: GET `http://h/x?q=a%20b` with `Host`, `content-length`, `Connection`, `Accept: application/json`, `Authorization: ***` → `curl -X GET 'http://h/x?q=a%20b' -H 'Accept: application/json' -H 'Authorization: ***'`
  - `curlCommand adds the body as logged`: POST with body `{\n  "a": "it's"\n}` ends with ` --data-raw '{\n  "a": "it'\''s"\n}'`
  - `curlCommand quotes every value`: a header value `$HOME \`id\`` and a URL with `'` come out inside single quotes, the `'` as `'\''`
  - `curlCommand is null without a URL`
  - `prettyBody re-indents JSON`: `{"a":1,"b":[1,2]}` → `JSON.stringify(JSON.parse(x), null, 2)`; an already indented body is unchanged
  - `prettyBody indents XML`: `<user><name>bob</name></user>` → `<user>\n  <name>bob</name>\n</user>`; `<?xml version="1.0"?><a x="1"><b/><c>t</c></a>` → `<?xml version="1.0"?>\n<a x="1">\n  <b/>\n  <c>t</c>\n</a>`
  - `prettyBody leaves anything else as logged`: `{not json`, `<a><b></a>`, `<a x="1>2">t</a>`, `<a><![CDATA[<b>]]></a>`, `plain text` and `""` are returned unchanged
  - `prettyBody handles a 200 KB JSON array`: 3000 objects in one line come back indented and parse to the same value
- [ ] **Step 2: Run** `npx vitest run src/shared/http.test.ts`. Expected: FAIL, module not found.
- [ ] **Step 3: Implement** `src/shared/http.ts`. `prettyBody`: try `JSON.parse` first. Otherwise, if the trimmed body starts with `<`, indent it without a DOM (this file runs in Node too): split on tags (`/(<[^>]*>)/`), drop whitespace-only text, and walk the tokens with a stack of open tag names. Declarations, comments and self-closing tags sit at the current depth. An open tag immediately followed by text and its own close tag stays on one line. A close tag must match the stack's top. Any mismatch, a leftover open tag, a `<![CDATA[` token, or a quote left open inside a tag returns the body unchanged.
- [ ] **Step 4: Run** `npx vitest run src/shared/http.test.ts`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(http): add the call model, status groups, curl text and body indenting`

### Task 2: HTTP sample project and recorded events

**Files:**
- Create: `test/fixtures/projects/http/` with `karate-config.js`, `start-mock.feature` and `tests/common/create-user.feature` copied from `test/fixtures/projects/clean/`, plus `mocks/api-mock.feature`, `tests/http.feature`, `tests/shared.feature`
- Modify: `test/fixtures/copy.ts` (name `"http"` and its doc line), `e2e/helpers.ts` (`project()` accepts `"http"`), `scripts/capture-fixtures.mjs`
- Create (generated): `test/fixtures/events/run-http.jsonl`, `run-http-config.jsonl`, `run-http-config-fail.jsonl`

**Interfaces:**
- Produces: `copyFixture("http")`, `project("http")`, and the three event files, read later with `readAllEvents`.

- [ ] **Step 1: Write the project.** `mocks/api-mock.feature` is the clean mock with these scenarios inserted before the catch-all:
  - `pathMatches('/slow')`: `responseDelay = 2000`, response `{ slow: true }`
  - `pathMatches('/moved')`: `responseStatus = 302`, `responseHeaders = { Location: '/hello' }`
  - `pathMatches('/boom')`: `responseStatus = 500`, response `{ error: 'boom' }`
  - `pathMatches('/echo') && methodIs('post')`: response = `request`, `Content-Type: application/xml`

  `tests/http.feature`, Background `* url baseUrl`, one scenario per row of the spec's table, named exactly `ok`, `retry`, `server error`, `refused`, `timeout`, `redirect followed`, `redirect kept`, `via helper`, `xml`, `secret header`:
  - `retry` uses `* configure retry = { count: 3, interval: 50 }`, `path 'nothing'`, `retry until responseStatus == 200`
  - `refused` uses `* configure connectTimeout = 500` and `url 'http://127.0.0.1:1/x'`
  - `timeout` uses `* configure readTimeout = 500`
  - `via helper` uses `* def result = call read('common/create-user.feature') { name: 'alice' }`
  - `xml` posts `<user><name>bob</name></user>` to `echo`
  - `secret header` uses `And header X-Token = karate.properties['api.token'] || 'none'`

  `tests/shared.feature`, Background `* url baseUrl` and `* def made = callonce read('common/create-user.feature') { name: 'shared' }`, then scenarios `first` and `second`, each `* match made.user.name == 'shared'`.
- [ ] **Step 2: Extend `scripts/capture-fixtures.mjs`.** Copy the http project to the temp work folder and run `[join(http, "tests")]` → `run-http.jsonl`. Then, in that copy, write `auth.feature` with the layout of `create-user.feature`, so `method post` is line 8 (`@ignore`; `url baseUrl`, `path 'users'`, `request { name: 'token' }`, `method post`, `status 201`) and replace `karate-config.js` with one that starts the mock, then calls `karate.callSingle('classpath:auth.feature', { baseUrl: baseUrl })` and returns `{ baseUrl: baseUrl }`. Run `[join(http, "tests", "shared.feature")]` → `run-http-config.jsonl`. Change `status 201` to `status 200` in `auth.feature` and run the same target → `run-http-config-fail.jsonl`.
- [ ] **Step 3: Run** `npm run fixtures`. Expected: the three new files are written; the existing ones are unchanged (`git diff --stat test/fixtures/events` lists only new files).
- [ ] **Step 4: Check the recording against the spec.** In `run-http.jsonl`, the depth-0 `FEATURE_EXIT` step logs of `tests/http.feature` and `tests/shared.feature` must hold the calls of the spec table: retry 3× 404, server error 500, timeout a request with no response, refused no log and a failed `method` step, redirect followed 2 requests (1 unanswered), redirect kept 302, via helper and shared one `POST` 201 in `callResults`, xml 200. If Karate disagrees, stop and report; do not edit the spec.
- [ ] **Step 5: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS (nothing reads the new files yet).
- [ ] **Step 6: Commit** `test(fixtures): add the http sample project and its recorded events`

### Task 3: Step-log parser

**Files:**
- Create: `src/main/karate/http.ts`, `src/main/karate/http.test.ts`

**Interfaces:**
- Consumes: Task 1 (`HttpCall`, `HttpHeader`); Task 2's event files.
- Produces:
  - `type LoggedCall = Omit<HttpCall, "error" | "via" | "startTime">`
  - `parseStepLog(log: string): LoggedCall[]`: spec R1 for one log: followed-redirect hops dropped, a last unanswered request kept with `status: null`, `attempt` set on every call when the log yields more than one.
  - `extractCalls(featureExitData: Record<string, any>): { refId: string; calls: HttpCall[] }[]`: one entry per scenario, like `extractSteps`.

- [ ] **Step 1: Write the failing tests** in `src/main/karate/http.test.ts`. `parseStepLog` on inline logs (copy the spec's example as the base):
  - `parses one call`: method `POST`, the URL, 6 request headers in order, request body `{\n  "name": "alice"\n}`, status 201, 2 response headers, the response body, millis 6, attempt `null`
  - `ignores text before request: and the summary lines`: the same log with `karate-config.js running, env = null\n` before it gives the same call
  - `keeps blank lines inside a body`: a response body `a\n\nb` stays `a\n\nb`
  - `a request: line inside a body does not start a call`: a response body with a line `request:` followed by `not a header` gives one call whose body holds both lines
  - `reads CRLF lines`: the base log with `\r\n` line ends gives the same call
  - `numbers retry attempts`: three numbered pairs give attempts `{ n: 1, of: 3 }` to `{ n: 3, of: 3 }`
  - `drops a followed redirect's unanswered request`: request 1 alone, then pair 2 with status 200 → one call, status 200
  - `keeps a last unanswered request as no response`: status, millis, response body `null`, response headers `[]`
  - `finds nothing in a log without request:`: `env is null devFlag is false` → `[]`

  `extractCalls` on `run-http.jsonl` (`tests/http.feature`, by scenario name through its refId):
  - `reads every case of the http sample`: `ok` 1 call GET 200 with via `null`; `retry` 3× 404 with attempts 1–3 of 3; `server error` 500; `refused` 1 call with method `GET`, url `null`, status `null`, error containing `Connection refused`; `timeout` 1 call with a URL ending `/slow`, status `null`, error containing `Read timed out`; `redirect followed` 1 call 200; `redirect kept` 1 call 302 with a `Location` response header; `via helper` 1 call POST 201 with via `tests/common/create-user.feature:8`; `xml` request body `<user><name>bob</name></user>` and status 200; `secret header` 1 call 200. 12 calls in all.
  - `puts a callonce call under the first scenario only`: `tests/shared.feature` gives `first` 1 call via `tests/common/create-user.feature:8` and `second` none
  - `puts a callSingle call under the first scenario`: in `run-http-config.jsonl`, `first` has a call via `auth.feature:8`, `second` has none
  - `puts a failed callSingle's call under the first scenario's config step`: in `run-http-config-fail.jsonl`, `first` has the auth call (status 201), `second` has none
  - `walks callResults two deep`: an inline `FEATURE_EXIT` whose step calls `a.feature`, whose step calls `b.feature`, whose step logs a call, gives that call with via `b.feature:<its line>`
  - `orders a test's calls by start time`
- [ ] **Step 2: Run** `npx vitest run src/main/karate/http.test.ts`. Expected: FAIL, module not found.
- [ ] **Step 3: Implement** `src/main/karate/http.ts`. `parseStepLog` splits on `/\r?\n/` and reads lines in one pass with the R1 markers and the Global Constraints guard. `extractCalls` walks each scenario's `stepResults` with `via = null`. For each step it parses `stepLog ?? ""`. For an unanswered call it takes `error` from `result.errorMessage ?? null`. A failed step whose `step.keyword` is `method` and whose log has no call gives one call: method `step.text.trim().toUpperCase()`, URL `null`, status `null`, the error. It sets `startTime` from `result.startTime`. Then it walks every `callResults[].scenarioResults[].stepResults` with via `<that feature's relativePath>:<step.line>` (computed per inner step). Finally it stable-sorts the test's calls by `startTime`.
- [ ] **Step 4: Run** `npx vitest run src/main/karate/http.test.ts`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(http): parse http calls from karate step logs`

### Task 4: `calls` event from the run

**Files:**
- Modify: `src/shared/events.ts` (event union), `src/main/karate/run-state.ts` (FEATURE_EXIT branch, `apply` doc comment), `src/main/karate/run-state.test.ts`, `src/main/mask.test.ts`

**Interfaces:**
- Consumes: Task 3 (`extractCalls`).
- Produces: `AppEvent` member `{ type: "calls"; key: string; calls: HttpCall[] }`, emitted by `RunState.apply` right after each `steps` event of the same key, for the same keys (`keyOf`), empty `calls` included.

- [ ] **Step 1: Write the failing tests:**
  - `run-state.test.ts` `emits calls when a feature ends`: on `run-clean.jsonl`, `s:tests/basic.feature#1` gets 1 call GET 200; `s:tests/basic.feature#3` gets 1 call POST 201 with via `tests/common/create-user.feature:8`; `s:tests/basic.feature#4` gets `[]`; there are 4 `calls` events for `basic.feature` keys, each right after its `steps` event
  - `run-state.test.ts` `emits no calls for called features`: no `calls` event key contains `common/create-user` or `start-mock`
  - `mask.test.ts` `masks a calls event`: a `calls` event with a header value `Bearer s3cret` and a URL `?t=s3cret` comes back with `****` in both and the same `key`
- [ ] **Step 2: Run** `npx vitest run src/main/karate/run-state.test.ts src/main/mask.test.ts`. Expected: FAIL.
- [ ] **Step 3: Implement** the union member and the push in `RunState.apply`.
- [ ] **Step 4: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS. Until Task 5, `reduce` ignores the new event: its switch has no case for it.
- [ ] **Step 5: Commit** `feat(runner): send each test's http calls when its feature ends`

### Task 5: Calls and their selection in the window state

**Files:**
- Create: `src/renderer/http.ts`, `src/renderer/http.test.ts`
- Modify: `src/renderer/reduce.ts`, `src/renderer/reduce.test.ts`, `src/renderer/store.ts`, `src/renderer/store.test.tsx`

**Interfaces:**
- Consumes: Task 1 (`HttpCall`, `HttpGroup`, `httpGroup`); Task 4 (`calls` event); `findNode`, `leaves` from `src/renderer/nodes.ts`.
- Produces:
  - `AppState` gains `calls: Record<string, HttpCall[]>`, `httpGroups: HttpGroup[]` (chips on; empty means All), `selectedCall: CallRef | null`, and `panel: "console" | "history" | "http"`
  - in `src/renderer/http.ts`: `interface CallRef { key: string; index: number }`, `interface ListedCall { ref: CallRef; call: HttpCall; test: TestNode }`, `narrowedCalls(s: Pick<AppState, "tree" | "selected" | "calls">): ListedCall[]`, `groupCounts(list: ListedCall[]): Record<HttpGroup | "all", number>`, `shownCalls(list: ListedCall[], groups: HttpGroup[]): ListedCall[]`, `toggledGroups(groups: HttpGroup[], g: HttpGroup | "all"): HttpGroup[]`, `totalCalls(calls: Record<string, HttpCall[]>): number`, `findCall(s: Pick<AppState, "tree" | "calls" | "selectedCall">): ListedCall | null`
  - store gains `selectCall(ref: CallRef): void` and `toggleGroup(g: HttpGroup | "all"): void`; `select(key)` also clears `selectedCall`; `showPanel(p)` clears it when `p !== "http"`

- [ ] **Step 1: Write the failing tests.** `src/renderer/http.test.ts` on a literal tree (feature `f:a.feature` with scenarios `s:a.feature#1`, `s:a.feature#2`, and feature `f:b.feature` with `s:b.feature#1`) and literal calls:
  - `narrowedCalls lists every call in start-time order`: with nothing selected and with the root selected
  - `narrowedCalls narrows to the selected node`: a scenario gives its own calls, a feature gives all under it
  - `groupCounts counts every group, zeros included`: `{ all, 1xx: 0, 2xx, 3xx: 0, 4xx, 5xx, none }`
  - `shownCalls with no group on shows all; with 4xx and none on shows only those`
  - `toggledGroups turns a group on and off; All turns every group off`
  - `findCall resolves the selected call and is null once it is gone`

  `reduce.test.ts`:
  - `stores calls per test`
  - `runStarted clears the calls of the tests it runs and keeps the others`
  - `runStarted drops the selected call of a test it runs`
  - `a new project and an environment change clear calls and the selected call`; `httpGroups` survive both
  - `a key that no longer names the same test loses its calls`

  `store.test.tsx`: `selectCall, select and showPanel`: `select(key)` clears the selected call, `showPanel("console")` clears it, `showPanel("http")` keeps it; `toggleGroup drops a call it hides`.
- [ ] **Step 2: Run** `npx vitest run src/renderer/http.test.ts src/renderer/reduce.test.ts src/renderer/store.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement.** In `reduce.ts`, add the fields to `initialState` and the `calls` case. Clear `calls` wherever `steps` is cleared: the project reset, the stale-key loop, `runStarted`'s per-key delete, and the environment reset. Clear `selectedCall` in those same places when its key is affected. In `store.ts`, `toggleGroup` sets `httpGroups`, then clears `selectedCall` if `findCall` no longer finds it in `shownCalls(narrowedCalls(state), httpGroups)`.
- [ ] **Step 4: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): keep http calls and the selected call in the window state`

### Task 6: HTTP tab

**Files:**
- Create: `src/renderer/HttpCalls.tsx`, `src/renderer/HttpCalls.test.tsx`
- Modify: `src/renderer/BottomPanel.tsx`, `src/renderer/App.tsx`

**Interfaces:**
- Consumes: Task 5 (state fields, `narrowedCalls`, `groupCounts`, `shownCalls`, `totalCalls`, store `selectCall`, `toggleGroup`); `HTTP_GROUPS`, `httpGroup`; `nodeName`, `formatMillis`, `findNode`.
- Produces: `HttpCalls(props: { state: AppState; selectCall(ref: CallRef): void; toggleGroup(g: HttpGroup | "all"): void })`; `BottomPanel` props gain `selectCall` and `toggleGroup`, passed from `App`. Test ids: tab `tab-http`; list `http-calls`; chip `http-group` with `data-group` (`all` or a group) and `aria-pressed`; row `http-call` (a `<button>` with `aria-pressed`, `data-status` = the code or `none`) holding `http-call-status`, `http-call-method`, `http-call-url`, `http-call-test`, `http-call-attempt`, `http-call-via`, `http-call-time`; empty state `http-empty`.

- [ ] **Step 1: Write the failing tests** in `src/renderer/HttpCalls.test.tsx`, rendering `App` with a fake `Api` as `History.test.tsx` does, a literal tree and `calls` events:
  - `adds the HTTP tab after History`: tab texts `Console`, `History (0)`, `HTTP (4)`; Console stays selected after a `runStarted`
  - `shows every group chip with its count`: `All 4`, `1xx Informational 0`, `2xx Success 1`, `3xx Redirect 0`, `4xx Client error 2`, `5xx Server error 0`, `No response 1`; All has `aria-pressed="true"`
  - `shows a row per call`: badge `404` and `—`; method; URL; test name; `attempt 2 of 2`; `via tests/common/create-user.feature:8`; `6 ms` and `—`; the No response row with no URL shows the first line of its error
  - `filters by the chosen groups`: turning on 4xx then No response shows 3 rows; clicking All shows 4
  - `narrows to the tree selection`: clicking a scenario in the tree leaves its rows and updates the chip counts
  - `shows the three empty states`, with the exact copy
  - `selects a row with Enter`: focus a row, press Enter, `aria-pressed="true"`
- [ ] **Step 2: Run** `npx vitest run src/renderer/HttpCalls.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** `HttpCalls.tsx` with the badge colours of Global Constraints and the History tab's markup and classes. Rows truncate the URL, with the full URL in `title`. Then add the third tab to `BottomPanel` and change its tablist label.
- [ ] **Step 4: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS, `History.test.tsx` included.
- [ ] **Step 5: Commit** `feat(ui): add the http tab with status groups`

### Task 7: Call details and Copy as cURL

**Files:**
- Create: `src/renderer/CallDetails.tsx`, `src/renderer/CallDetails.test.tsx`
- Modify: `src/renderer/Details.tsx`

**Interfaces:**
- Consumes: Task 5 (`findCall`, `ListedCall`); Task 1 (`curlCommand`, `prettyBody`, `httpGroup`, `HTTP_GROUPS`).
- Produces: `CallDetails(props: { listed: ListedCall })`. `Details` shows it instead of the node when `state.panel === "http"` and `findCall(state)` is not null. Test ids: `call-details`, `call-method`, `call-url`, `call-status`, `call-time`, `call-location`, `call-via`, `copy-curl`, `call-request-headers`, `call-request-body`, `call-response-headers`, `call-response-body`, `call-error`.

- [ ] **Step 1: Write the failing tests** in `src/renderer/CallDetails.test.tsx`, through `App` as in Task 6:
  - `shows the selected call`: method, URL, `201 · 2xx Success`, `6 ms`, location `a.feature:3`, via line; request headers as `Name: value` lines in order; a JSON body re-indented; an XML request body indented
  - `shows a call with no response`: status `No response`, the error in `call-error`, no `call-response-*`
  - `copies the call as cURL`: with `navigator.clipboard.writeText` stubbed, clicking `copy-curl` writes `curlCommand(call)` and the button reads `Copied`; a rejected write reads `Copy failed`
  - `disables Copy as cURL without a URL`
  - `gives the pane back to the node`: after selecting a call, clicking a tree node shows that node's details; selecting a call again and clicking `tab-console` shows the node's details
- [ ] **Step 2: Run** `npx vitest run src/renderer/CallDetails.test.tsx`. Expected: FAIL.
- [ ] **Step 3: Implement** `CallDetails.tsx`. Bodies go in a `<pre>` scroll box (`max-h-80 overflow-auto`, monospace). Run `prettyBody` in a `createMemo` so a large body is indented once. The error uses the classes of `details-error` in `Details.tsx`. "Copied" resets after 2000 ms. Then add the switch in `Details`.
- [ ] **Step 4: Run** `npm test`, `npm run typecheck` and `npm run lint`. Expected: PASS.
- [ ] **Step 5: Commit** `feat(ui): show a call's request and response with copy as curl`

### Task 8: End-to-end tests and ship

**Files:**
- Create: `e2e/http.spec.ts`
- Modify: `docs/features.md` (F11 status `done`)

**Interfaces:**
- Consumes: everything above; `launch`, `openProject`, `project("http")`, `project("clean")`, `node`, `nodeByKey`, `openProperties`, `addProperty`, `saveProperties`, `SECRET` from `e2e/helpers.ts`. The clipboard is read with `app.evaluate(({ clipboard }) => clipboard.readText())`.

- [ ] **Step 1: Write `e2e/http.spec.ts`**, one Playwright test per Gherkin scenario of the spec. The scenarios that only read the result of one finished Run all on the http sample project may share one test, one `test.step` per scenario: listed and grouped, only errors, narrowing, request and response, no response, copy as cURL, shared setup, pane given back. These stay separate tests, each on a fresh copy: the called feature on the clean project; the secret, with `api.token` saved as a secret in the All environments scope (`""`) before running `secret header`; and a run of one test replacing only its calls.
- [ ] **Step 2: Run** `npm run e2e -- e2e/http.spec.ts` (not alongside `npm test`). Expected: PASS. If the copy test fails because the window has no focus, report it rather than switching to an IPC copy: that changes the spec's "no new IPC" decision.
- [ ] **Step 3: Set the F11 row's status to `done`** in `docs/features.md`.
- [ ] **Step 4: Run** `npm test`, then `npm run e2e`, then `npm run typecheck` and `npm run lint`. Expected: all PASS.
- [ ] **Step 5: Commit** `test(e2e): cover the http inspector and mark f11 done`
