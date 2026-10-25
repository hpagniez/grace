# HTTP inspector

Date 2026-10-25. Status: approved. Covers [features](../features.md) F11, after-run version. The live version waits for the `RunListener` of F14.

## Intent

Karate API tests fail because of an HTTP call: a 401 from an auth helper, a 500, a refused connection. Today the request and response sit as raw text behind the **Log** toggle of one step in the details pane. To find the cause, the person must open each scenario and read each log. To check that every call went well, they must read them all.

The HTTP inspector lists every HTTP call of the run in one place, grouped by status class, so the person can:
- find the root cause of a failure: the call that went wrong, its request and its response;
- confirm at a glance that every call succeeded, or see which ones did not.

Karate's HTML report (F09) shows each step's request and response too. It is one feature page at a time in a browser, with no run-wide list and no status grouping. Karate Xplorer's response viewer is popular, but running feature files in Xplorer is paid ([commercial inventory](../inventory/commercial.md#user-needs-a-free-runner-must-cover)). The inspector brings call inspection into the free runner, next to the tree.

It builds on the [walking skeleton](2026-10-24-walking-skeleton.md) (tree, details pane, steps from `FEATURE_EXIT`), the [environment switcher](2026-10-25-environment-switcher.md) (secret masking of every event) and [run history](2026-10-25-run-history.md) (the bottom panel's tabs).

## Who and what success looks like

The walking skeleton's user, iterating on an API suite: run, read failures, fix, run again.

Success:
- After a run, the HTTP tab shows how many calls fell in each status class, and the client errors, server errors and calls with no response are one click away.
- From a failed scenario, the call that caused it is found without reading raw logs, including a call made inside a called feature.
- A call can be copied as a cURL command to replay it outside Karate.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals for this version:**
- Live calls while a scenario runs. Calls show when their feature ends (R6). The live version needs F14's `RunListener`.
- Calls in the details pane's step list, next to the step that made them (option B of the brainstorm). The step's **Log** toggle stays as it is.
- Calls of past runs. Like steps, calls belong to this session's results (run history non-goal); a past run's HTML report shows its calls.
- Search, sort, or filters other than the status groups and the tree selection.
- Syntax colours in bodies. Bodies are indented, not coloured.
- Copy as cURL for Windows `cmd` or PowerShell, copy as fetch, export of a HAR file.
- Putting real secret values back into a copied cURL. Masked values stay masked (decided in the brainstorm).
- Calls Karate does not log: HTTP made by a UI driver, or a `karate.http()` call that failed before logging anything.
- Saving the status-group choice per project.

**Anti-goals** (failure even if everything else works):
- A secret value (environment switcher R5) shown in the tab, the details pane or a copied cURL.
- A call missing from the list while Karate logged it: the person would think every call succeeded.
- A call shown under the wrong test, or under the wrong status class.
- The window freezing on a large body.

## Constraints

- Everything in the walking skeleton's and environment switcher's Constraints sections. No new dependency.
- The app reads the HTTP text Karate 2.1.4 writes into step logs. The format is Karate's, not a contract: the parser's unit tests run on logs recorded from the real Karate, and a Karate bump re-records them (F33).

### What Karate writes (probes)

Probes for this spec (2026-10-25, Karate 2.1.4, bundled Java 21, Windows 11), on the spike project and the cases below. A depth-0 `FEATURE_EXIT` holds `scenarioResults[].stepResults[]`; each step result has `step` (`keyword`, `text`, `line`), `result` (`status`, `millis`, `startTime`, and `errorMessage` when it failed), and optional `stepLog` and `callResults`.

The HTTP text of a successful call, in the `stepLog` of the `method` step:

```text
request:
1 > POST http://localhost:60219/users
1 > Content-Type: application/json; charset=UTF-8
1 > Host: localhost:60219
1 > Content-Length: 16
1 > Connection: keep-alive
1 > User-Agent: Apache-HttpClient/5.6.5 (Java/21.0.12.1)
{
  "name": "alice"
}

response time in milliseconds: 6
1 < 201 POST http://localhost:60219/users
1 < content-length: 23
1 < Content-Type: application/json; charset=UTF-8
{
  "name": "alice",
  "id": 1
}

POST http://localhost:60219/users
201 (6 ms)
```

| Case | What the step log holds |
|---|---|
| One call | As above. The last two lines are a summary Karate adds after a `method` step. |
| Several calls in a scenario | One counter per scenario: `1 >`, then `2 >` in a later step, then `3 >`. |
| `retry until` with 3 attempts | Three request/response pairs numbered 1, 2, 3 in the same step log; no summary lines. If every attempt fails the condition, the step fails with `retry failed after 3 attempts: …`. |
| Redirect, followed (Karate's default) | Request 1 with no response, then request 2 with the final response. Both name the first URL (`GET …/moved`); the 302 never appears. |
| Redirect, `configure followRedirects = false` | One pair with `1 < 302 …` and the `Location` header. |
| Read timeout | The request with no response; the step fails with `java.net.SocketTimeoutException: Read timed out`. |
| Connection refused | No step log at all; the step fails with `org.apache.hc.client5.http.HttpHostConnectException: Connect to http://localhost:1 failed: Connection refused: getsockopt`. |
| `karate.http(url).get()` in a `def` step | The request/response pair in that step's log; no summary lines. |
| Call inside a called feature | The pair sits in the called feature's step, inside the calling step's `callResults` (a list of feature results, each with `relativePath` and its own `scenarioResults`), not in the depth-0 step. |
| `karate.callSingle` in `karate-config.js` | The helper's results sit in `callResults` of the first step of the first scenario to run only. Other scenarios carry nothing. |
| `callonce` in a Background | The helper's results sit in `callResults` of the `callonce` step of the first scenario to run only. |
| A `callSingle` helper that fails | Every scenario fails with one synthetic step (`line` 0, no keyword, text `Scenario execution failed: Config evaluation failed…`). Only the first scenario's synthetic step carries the helper's `callResults`, failing call included. |
| `print` or `karate.log` in the same step | Plain lines before `request:` (the first step of each scenario carries the config's `karate-config.js running` line). |
| XML request body | Logged on one line as sent: `<user><name>bob</name></user>`. |
| JSON body | Indented by Karate (`configure logging pretty`, default true). |
| `Authorization` header | Logged as `Authorization: ***` (Karate's default mask). |
| 140 KB JSON response | Logged in full: a 194 KB step log. |

The app already masks every string of every event it sends to the window with the run's secret values (`maskEvent`, environment switcher R5): step logs reach the window with `****` in place of a secret.

## Approaches considered

Where the calls show (mockups compared in the brainstorm):
1. **An HTTP tab next to Console and History in the bottom panel, details in the right pane.** One run-wide list answers "did every call succeed?"; the tree selection narrows it to one test. **Chosen.**
2. Calls under the step that made them, in the details pane. The cause sits next to the failing step, but there is no run-wide list: the person checks one scenario at a time.
3. Both. The most to build; the step-list half can come later.

How the calls are produced:
1. **Main parses the step logs of each depth-0 `FEATURE_EXIT`, including nested `callResults`, and sends a `calls` event, masked like every event.** Same path as steps today (`extractSteps` → `steps` event). Unit-tested on recorded logs. The risk is Karate's log format changing; the recorded logs catch it. **Chosen.**
2. The window parses the step logs it already receives. Steps leave out called features and the step error text, so both would have to be added anyway, and parsing would sit in the UI.
3. A Java `RunListener` sending structured `HTTP_*` events. No text parsing, and live, but it needs a Java toolchain, a bundled jar and a decision record: F14's groundwork, far bigger than this version.

Rejected without a mockup: reading Karate's HTML report, whose inlined JSON is an internal format.

## Requirements

### R1. What a call is

Main reads every depth-0 `FEATURE_EXIT`, as for steps.
- For each scenario (or outline row) it walks the step results in order. For each step it reads the step log, then walks that step's `callResults`: every scenario of every called feature, at any depth, in order.
- A call made in a called feature belongs to the depth-0 test whose step made the call. It records **via**: the called feature's `relativePath` and the line of the step that logged it, for example `tests/common/create-user.feature:8`. For calls nested several levels deep, via names the innermost feature.
- **Shared setup** (`karate.callSingle`, `callonce`) follows the same rule (decided in the brainstorm): its calls belong to the test whose step carries its `callResults`, the first test Karate ran, and are marked via. Other tests show none. Which test carries them can change from one run to the next. When the helper fails, they belong to the test whose synthetic `Config evaluation failed` step carries them.

In one step log:
- A call starts at a line `request:`. Its request line is `N > METHOD URL`. Request headers are the following `N > Name: value` lines. The request body is every line after the headers, up to the line `response time in milliseconds: T` or the next `request:` or the end of the log. One trailing blank line is dropped. A body with no lines is no body.
- The response starts at `response time in milliseconds: T`, then `N < STATUS METHOD URL`, then `N < Name: value` headers, then the body. The body runs up to the next `request:`, the two summary lines (`METHOD URL`, then `STATUS (T ms)`, when they end the log), or the end of the log. Trailing blank lines are dropped.
- A request and a response pair by their number `N`.
- Lines before the first `request:` and the summary lines belong to no call. Bodies may contain blank lines: only the markers above end them.
- A request with no response, followed by another request in the same step log, is a followed redirect: it is not a call. The next request carries the final status.
- A request with no response that is the last of the step log is a **No response** call. Its error is the step's `errorMessage` (none if the step passed).
- A failed step whose keyword is `method` and whose log has no `request:` is one **No response** call: method from the step text (`method get` → `GET`), no URL, no request, the step's `errorMessage` as its error.

Each call keeps: method; URL; request headers (name, value, in logged order); request body or none; status code, or none for No response; response headers; response body or none; response time in ms, or none; error or none; via or none; the step's start time; and, when one step logged more than one answered call (`retry until`), its attempt number and the attempt count.

Status classes, from the code: 100–199 **1xx Informational**, 200–299 **2xx Success**, 300–399 **3xx Redirect**, 400–499 **4xx Client error**, 500–599 **5xx Server error**. A call with no status is **No response**. A code outside 100–599 counts only in All.

### R2. Calls in the window

- Main sends the calls of a test when its feature ends, next to its `steps` event: one `calls` event per test that has steps, carrying the test's key and its calls in run order (empty list for none). The event goes through `maskEvent` like every event.
- The window keeps calls per test key and clears them exactly as it clears steps: a run clears the calls of the tests it runs, tests outside the run keep theirs; a new project, an environment change, and a key that no longer names the same test clear them.
- So the tab lists the calls behind the results the tree shows, from whichever run produced each. After a run of one test, a shared setup call can be listed twice: once under a test kept from the earlier run, once under the test just run. Both calls were made, so both count.
- Like steps, calls are not replayed to a window that reloads, and the app writes no call data to disk.

### R3. HTTP tab

- The bottom panel gets a third tab, **HTTP (n)**, after Console and History. n is the number of calls the window holds. The panel keeps its height. Starting a run still switches to Console.
- **Narrowing.** The list shows the calls of the tests under the tree selection: a scenario or row shows its own; an outline, feature or folder shows all under it; the root, or no selection, shows all.
- **Rows**, in run order (the step start time, then the order in the log), one per call:
  - status: the code, or "—" for No response, on a badge coloured by class;
  - method;
  - URL, truncated, full in a tooltip. A No response call with no URL shows the first line of its error instead;
  - the test's name;
  - "attempt 2 of 3" when the call is one of several attempts;
  - "via tests/common/create-user.feature:8" when it was made in a called feature;
  - response time, "—" when there is none.
- **Status groups.** Above the list, chips: **All**, **1xx Informational**, **2xx Success**, **3xx Redirect**, **4xx Client error**, **5xx Server error**, **No response**. Each shows its count of calls in the narrowed list. All seven always show, zeros included, so "5xx Server error 0" confirms there was none.
  - Clicking a group chip turns it on or off. Several can be on: the list shows calls in any of them. With none on, All is on and the list shows every narrowed call. Clicking All turns the others off.
  - The choice lasts for the session; runs and narrowing do not reset it.
- **Colours:** 1xx zinc, 2xx green, 3xx sky, 4xx amber, 5xx red, No response dark red. The code or the group name is always written: colour is never the only signal.
- **Empty states:** "No HTTP calls yet. Calls appear when a feature finishes." when the window holds none; "No HTTP calls in <selected node name>" when narrowing leaves none; "No calls in the chosen groups" when the chips leave none.
- The list and chips are reachable and usable with the keyboard; a row is selected with Enter or Space.

### R4. Call details

- Selecting a row shows the call in the right pane, in place of the node details. The row shows as selected.
- The pane shows:
  - a header: method, URL (wrapped, selectable), status code and group name, response time;
  - the test's name and location (`file:line`), and the via line when there is one;
  - **Copy as cURL** (R5);
  - **Request**: headers as `Name: value` lines in logged order, then the body;
  - **Response**: status line, headers, body. For a No response call: the error in the red box the details pane uses for a failed test, instead of the response.
- Bodies show in monospace, in a scrolling box. A body that parses as JSON is re-indented with 2 spaces. A body that parses as XML is indented with 2 spaces. Any other body shows as logged.
- The pane goes back to the node details when a tree node is clicked, another bottom tab is chosen, or the selected call leaves the list (a run clears it, narrowing or a chip hides it).

### R5. Copy as cURL

- **Copy as cURL** puts a bash-quoted command on the clipboard:
  - `curl -X <METHOD> '<URL>'`;
  - one `-H '<Name>: <value>'` per request header, in logged order, except `Host`, `Content-Length` and `Connection`: curl sets those itself, and a stale `Content-Length` breaks the request;
  - `--data-raw '<body>'` when the request has a body, as logged (not re-indented);
  - single quotes around every value; a `'` inside is written `'\''`.
- Values Karate masked (`***`) and the app masked (`****`) are copied as they are.
- The button is disabled for a No response call with no logged request.
- After copying, the button reads "Copied" for about 2 s.

### R6. During and after a run

- Calls appear feature by feature as features end; the tab count and chips update as they arrive. The HTTP tab can be opened during a run.
- A cancelled run shows the calls of the features that ended before the cancel. Karate writes nothing for the others.

## Design decisions already made

- The parser lives in main next to `extractSteps` (`src/main/karate/`), and the `calls` event is emitted where `steps` is (`RunState`, on depth-0 `FEATURE_EXIT`). The call and event types go in `src/shared/events.ts`.
- Status classes, the cURL text and body indenting are pure functions with no Node imports, so the window can use them.
- No new IPC request: the window gets calls only from events. Copying to the clipboard is the only new effect.

## Acceptance criteria

Test projects:
- The **clean sample project** (walking skeleton): "passing hello" makes `GET /hello` → 200; "calls helper feature" makes `POST /users` → 201 inside `tests/common/create-user.feature`, line 8.
- A new **http sample project**: the clean project's mock, `karate-config.js` and `tests/common/create-user.feature`, the mock extended with `/slow` (2 s delay), `/moved` (302 to `/hello`), `/boom` (500) and `/echo` (returns the XML body it receives), and `tests/http.feature` with one scenario per case, `url baseUrl` in its Background:

| Scenario | Calls | Groups |
|---|---|---|
| ok | `GET /hello` → 200 | 2xx |
| retry | `retry until responseStatus == 200` on `/nothing`, 3 attempts → 404 ×3; fails | 4xx ×3 |
| server error | `GET /boom` → 500, `status 500` passes | 5xx |
| refused | `GET http://127.0.0.1:1/x`; fails | No response, no request |
| timeout | `readTimeout = 500`, `GET /slow`; fails | No response, request logged |
| redirect followed | `GET /moved` → 200 | 2xx |
| redirect kept | `followRedirects = false`, `GET /moved` → 302 | 3xx |
| via helper | calls `tests/common/create-user.feature` → 201 | 2xx, via |
| xml | `POST /echo` with `<user><name>bob</name></user>` → 200 | 2xx |
| secret header | `header X-Token = karate.properties['api.token'] \|\| 'none'`, `GET /hello` → 200 | 2xx |

And `tests/shared.feature`, whose Background has `* def made = callonce read('common/create-user.feature') { name: 'shared' }`, with two passing scenarios, "first" then "second". Its one `POST /users` → 201 belongs to "first".

Run all makes 13 calls: 2xx 6, 3xx 1, 4xx 3, 5xx 1, No response 2, 1xx 0.

Every scenario is automated with Playwright against the built app, each on a fresh copy of its fixture and a fresh app data folder.

```gherkin
Feature: HTTP inspector

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  Scenario: Every call of a run is listed and grouped
    Given the http sample project is open
    When I click Run all and the run finishes
    Then the HTTP tab reads "HTTP (13)"
    And its chips read All 13, 1xx 0, 2xx 6, 3xx 1, 4xx 3, 5xx 1, No response 2

  Scenario: Showing only the calls that went wrong
    Given the http sample project has finished Run all
    When I open the HTTP tab and turn on 4xx, 5xx and No response
    Then the list shows 6 calls
    And none of them has a 2xx or 3xx status

  Scenario: The tree selection narrows the list
    Given the http sample project has finished Run all
    When I open the HTTP tab and select the scenario "retry" in the tree
    Then the list shows 3 calls with status 404
    And they read "attempt 1 of 3", "attempt 2 of 3" and "attempt 3 of 3"

  Scenario: A call's request and response
    Given the http sample project has finished Run all
    When I open the HTTP tab and select the call of "xml"
    Then the details pane shows "POST" and a URL ending in "/echo" with status 200
    And the request body shows "<name>bob</name>" on its own indented line

  Scenario: A call made in a called feature
    Given the clean sample project has finished Run all
    When I open the HTTP tab and select the scenario "calls helper feature"
    Then the list shows 1 call "POST" with status 201
    And it reads "via tests/common/create-user.feature:8"

  Scenario: A shared setup call shows under the scenario Karate ran it in
    Given the http sample project has finished Run all
    When I open the HTTP tab and select the scenario "first" in the tree
    Then the list shows 1 call "POST" with status 201 reading "via tests/common/create-user.feature:8"
    When I select the scenario "second" in the tree
    Then the list says "No HTTP calls in second"

  Scenario: A call that got no response
    Given the http sample project has finished Run all
    When I open the HTTP tab and select the call of "refused"
    Then its row shows "—" and "Connection refused"
    And the details pane shows the error in a red box
    And Copy as cURL is disabled

  Scenario: Copy as cURL
    Given the http sample project has finished Run all
    When I open the HTTP tab, select the call of "ok" and click Copy as cURL
    Then the clipboard holds a command starting "curl -X GET 'http://localhost:"
    And it has no Host, Content-Length or Connection header

  Scenario: A secret never shows in a call
    Given the http sample project is open
    And the run property "api.token" is saved as a secret with value "s3cret-token-123"
    When I run "secret header" and it finishes
    And I select its call in the HTTP tab and click Copy as cURL
    Then the details pane shows "X-Token: ****"
    And neither the details pane nor the clipboard contains "s3cret-token-123"

  Scenario: Clicking a test gives the details pane back
    Given a call is shown in the details pane
    When I click the scenario "ok" in the tree
    Then the details pane shows the scenario "ok" and its steps

  Scenario: A run of one test replaces only its calls
    Given the http sample project has finished Run all
    When I run "ok" and it finishes
    Then the HTTP tab still reads "HTTP (13)"
```

Also automated (Vitest):
- The parser, on step logs recorded from a real run of the http sample project (stored as a fixture like `test/fixtures/events/run-clean.jsonl`): every row of the table above, plus text before `request:`, summary lines, a body with blank lines, pairing by number, a called feature nested two deep, the followed-redirect rule, and a failed `method` step with no log. Plus logs recorded from a project whose `karate-config.js` `callSingle`s a helper that makes a call: the call belongs to the first scenario only; and when that helper fails, to the first scenario's synthetic `Config evaluation failed` step.
- Status classes, including codes outside 100–599.
- cURL text: quoting of `'`, dropped headers, body and no body, masked values unchanged.
- Body indenting: JSON, XML, invalid JSON or XML left as logged, a 200 KB body.
- `RunState` emits a `calls` event per test on a depth-0 `FEATURE_EXIT`, and none for called features' own `FEATURE_EXIT`.
- The window state: calls kept and cleared as steps are, narrowing by tree node, chip filtering and counts.

## Left to the builder

- Component structure, styling within the colours above, exact event and type names, how the tab reuses the existing tab markup.
- How the clipboard is written (the renderer's `navigator.clipboard` or Electron's `clipboard` through the preload).
- How bodies are rendered so a large body keeps the window responsive.
- Exact wording of labels and tooltips beyond the ones quoted above.

Resolved by the builder check (2026-10-25). The person decided that shared setup calls (`callSingle`, `callonce`) show under the test Karate ran them in, marked via. The spec author's calls: calls in called features are included with the innermost feature as via; a refused connection or timeout is a No response call, and a failed `method` step with no log is one; a followed redirect's unanswered first request is not a call, so 3xx shows only with `followRedirects = false`; calls are kept and cleared per test as steps are; status chips are multi-select, always all shown with zeros, and kept for the session; the tree selection narrows the list (a folder, feature or outline to everything under it); cURL is bash-quoted and drops `Host`, `Content-Length` and `Connection`; bodies are indented, not coloured; the details pane goes back to the node on a tree click, a tab change, or when the call leaves the list.
