# App check: HTTP inspector (F11)

Date: 2026-10-26
Branch: `feature/http-inspector` at f038867 (PR #17)
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [HTTP inspector](../specs/2026-10-25-http-inspector.md).

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 529/529 passed |
| `npm run e2e` | 52/52 passed, including the 4 tests in `e2e/http.spec.ts` |
| Built app driven on a copy of the `http` fixture, Run all, screenshots | HTTP tab reads `HTTP (13)`; chips 1xx 0, 2xx 6, 3xx 1, 4xx 3, 5xx 1, No response 2; errors-only filter leaves 6 rows; XML, refused, timeout and called-feature calls show as the spec says; dark mode and a 900×650 window are readable |
| Built app on a throwaway project calling public APIs over the internet (httpbin.org, jsonplaceholder.typicode.com; 13 scenarios, Run all in 8.6 s) | 15 calls: 2xx 12, 4xx 1 (418), 5xx 1 (503), No response 1 (read timeout on `/delay/3`, request and error shown, cURL enabled). HTTPS and the encoded query `q=karate%20kid` shown as sent; the gzip body readable; `/redirect/2` followed into one 200 call under its first URL; form body `user=kid&note=it%27s+here` copied as `--data-raw`; `Authorization: ***` masked by Karate (the echoed token in the response body is not: Karate masks headers only); HEAD copied as `curl --head`; a 1 MB JSON body (`/photos`) selected in 223 ms and scrolled without lag. Known limits seen: a multipart request body is Karate's part summary, so its cURL cannot replay it; a binary response (`/bytes/256`) shows headers only, because Karate logs no binary body. |

Not covered by the automated part: a person's own Karate project, and how it feels to use.

## By hand (done by a person)

Start the app with `npm start` from the repo (or the built zip). Use a writable copy of the `http` fixture: `%TEMP%\kk-http-demo` (copy `test/fixtures/projects/http` there if it is missing). Write the result under each step: passed, or what you saw instead.

### A. The sample project

1. Click **Open…** and pick `%TEMP%\kk-http-demo`. The tree shows `tests` with `http.feature` (10 scenarios) and `shared.feature` (`first`, `second`).
   - Result: passed (checked by Tusquito, 2026-10-26).
2. Click **Run all**. It finishes with 9 passed and 3 failed (`retry`, `refused`, `timeout`).
   - Result: passed (checked by Tusquito, 2026-10-26).
3. Click the **HTTP (13)** tab. The chips read `All 13`, `1xx Informational 0`, `2xx Success 6`, `3xx Redirect 1`, `4xx Client error 3`, `5xx Server error 1`, `No response 2`.
   - Result: passed (checked by Tusquito, 2026-10-26).
4. Turn on **4xx**, **5xx** and **No response**. Exactly 6 rows are left: three `404` on `/nothing` (attempt 1, 2 and 3 of 3), one `500` on `/boom`, and two `—` (refused, timeout). Click **All**: 13 rows again.
   - Result: passed (checked by Tusquito, 2026-10-26).
5. Click the scenario `retry` in the tree. The list narrows to its 3 calls. Click `http.feature`: 12 calls. Click `tests`: 13.
   - Result: passed (checked by Tusquito, 2026-10-26). Feedback: the tab read "HTTP (13)" while the list showed 3 calls for `retry`. Changed: the tab now counts the rows the list shows (spec R3).
6. Click the `POST …/echo` row (scenario `xml`). The right pane shows `POST`, the URL, `200 · 2xx Success`, the request headers, and the body indented on three lines (`<user>`, `  <name>bob</name>`, `</user>`). The response shows the same XML.
   - Result: passed (checked by Tusquito, 2026-10-26).
7. Click the `refused` row. The pane shows `GET`, `No response`, and the error in a red box. **Copy as cURL** is disabled.
   - Result: passed (checked by Tusquito, 2026-10-26).
8. Click the `timeout` row (`/slow`). The pane shows the request and `Read timed out` in the red box.
   - Result: passed (checked by Tusquito, 2026-10-26).
9. Click the `POST …/users` row of `via helper`. It reads `via tests/common/create-user.feature:8`. Click the scenario `first` (shared.feature): its list shows the same kind of call (shared setup). Click `second`: "No HTTP calls in second".
   - Result: passed (checked by Tusquito, 2026-10-26).
10. Select the `ok` call (`GET …/hello`, 200) and click **Copy as cURL**. The button reads "Copied". Paste it into Git Bash while the app is still open. The command starts `curl -X GET 'http://localhost:` and has no `Host`, `Content-Length` or `Connection` header.
    - Running it fails with "connection refused". That is expected: the mock server lives only while Karate runs.
    - Result: passed (checked by Tusquito, 2026-10-26).
11. With a call selected, click a test in the tree: the pane shows that test again. Select a call, then click **Console**: the pane shows the test again.
    - Result: passed (checked by Tusquito, 2026-10-26).
12. Keyboard: click a row, then use Tab and Shift+Tab to move between rows, and Enter or Space to select one. The selected row is highlighted and the pane follows.
    - Result: partly (checked by Tusquito, 2026-10-26). Enter and Space select a row. Tab and Shift+Tab also stop on every tree button, including each ▶ run button: that is the browser's normal tab order, not an F11 defect. Arrow-key navigation is a separate idea.
13. Secret: click **Properties…**, add `api.token` = `s3cret-token-123` in **All environments**, tick **Secret**, and save. Run the scenario `secret header`. Its call shows `X-Token: ****`. Copy as cURL, paste it, and check it holds `****`, never `s3cret-token-123`.
    - Result: passed (checked by Tusquito, 2026-10-26).
14. Run only `ok` (its ▶ button). The tab still reads **HTTP (13)**: other tests keep their calls, and `ok` gets a fresh one.
    - Result: passed (checked by Tusquito, 2026-10-26). Same feedback as step 5: "HTTP (13)" was misleading with one test selected. Fixed by the same change.
15. Start Run all and open the HTTP tab while it runs. Calls appear feature by feature as each feature ends.
    - Result: changed (checked by Tusquito, 2026-10-26). Run all switched the bottom panel to Console, so the HTTP tab could not be watched filling. Changed: starting a run keeps the open tab (spec R3; run history spec R3 updated).

### B. A real project of yours

16. Open one of your own Karate projects, ideally one with auth helpers (`call`, `callonce`, `karate.callSingle`) and an expected failure. Run all. Every HTTP call you expect is in the HTTP tab, including calls inside called features (they read `via …`).
    - Result: not done by a person: no real project available. Covered in part by the public-API run in the automated part.
17. For a failing scenario, select it in the tree and look at its calls. The request and response you need to understand the failure are there, readable, and the bodies are indented.
    - Result: not done by a person: no real project available. Covered in part by the public-API run in the automated part.
18. Copy a call that needs no secret and no local mock as cURL, paste it into Git Bash, and run it. It replays the request.
    - Result: not done by a person: no real project available. Covered in part by the public-API run in the automated part.
19. Anything confusing, missing or wrong while using it:
    - Notes: none (no real project).
