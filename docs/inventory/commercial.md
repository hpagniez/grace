# Karate Labs commercial GUI tooling: feature inventory

Researched 2026-10-24 from public docs, product pages and marketplace listings. Feature names and descriptions are paraphrased. Short quoted phrases are marked with quotes. Rows marked `(unverified)` rest on inference or a source that does not state the tier.

Pricing summary (https://karatelabs.io/pricing):

- **Free ($0):** Karate core (MIT), the Xplorer free tier and community support.
- **IDE Plus ($100/user/yr):** basic IDE features.
- **Pro ($640/user/yr):** IDE Pro features, Xplorer Premium and priority email support.
- **Xplorer Premium alone ($200/user/yr).**
- **Enterprise (custom price):** IDE Enterprise features, Karate Agent, API Coverage, API Governance, the Async Protocol Pack, a license server and SSO.

The pricing page has no free IDE tier.

## Karate core framework (free baseline, for reference)

| Feature | Description | Free/Paid | Source URL |
|---|---|---|---|
| Built-in parallel runner | Runs features in parallel from the CLI, JUnit or the Java API | Free (MIT) | https://karatelabs.io/pricing |
| HTML + JUnit XML reports | Built-in report output, CI-friendly | Free (MIT) | https://karatelabs.io/pricing |
| UI automation, mocks | Browser/desktop driver, built-in test doubles | Free (MIT) | https://karatelabs.io/pricing |
| Debug adapter (DAP) server | The community VS Code extension used a core debug server in v1. In v2, `-Dkarate.debug.port` delegates to the commercial `karate-ide` jar; OSS core has no debug server (see [karate-core.md §19](karate-core.md#19-integration-surface-for-a-gui), point 14) | Paid (v2) | https://github.com/karatelabs/karate/blob/main/karate-core/src/main/java/io/karatelabs/core/Runner.java |

## Karate VS Code extension (karatelabs.karate)

The marketplace lists a "Free Trial" label (45k installs, v3.0.3). The docs group features into PLUS, PRO and ENTERPRISE tiers. No tier is free.

| Feature | Description | Free/Paid | Source URL |
|---|---|---|---|
| Syntax coloring | Gherkin/Karate highlighting | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Run feature from editor | Gutter play buttons, Cmd/Ctrl+F5, context menu | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Run single Scenario / Examples row | Hover run on an example row | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Launch configurations | Saved run configs | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Outline view | Feature/scenario outline | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Code formatting | Feature file formatter | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Test results in IDE | Pass/fail results inside VS Code | Paid: PLUS | https://docs.karatelabs.io/ide-support/vs-code |
| Test Explorer | Native VS Code test runner with real-time results (`karate-cli` mode) | Tier not stated, likely PLUS (unverified) | https://docs.karatelabs.io/ide-support/vs-code |
| HTML reports with tag filtering | Opens the Karate HTML report, filterable by tag | Tier not stated (unverified) | https://docs.karatelabs.io/ide-support/vs-code |
| ANSI-colored console | Colored run output | Tier not stated (unverified) | https://docs.karatelabs.io/ide-support/vs-code |
| Commands | Setup Dashboard, Sign In, Kill Tests, Clean Reports, New Feature File | Tier not stated (unverified) | https://docs.karatelabs.io/ide-support/vs-code |
| Auto-complete | Keyword/step completion | Paid: PRO | https://docs.karatelabs.io/ide-support/vs-code |
| Code folding, JSON re-format | Editor helpers | Paid: PRO | https://docs.karatelabs.io/ide-support/vs-code |
| Run all tests in folder | Folder-level run | Paid: PRO | https://docs.karatelabs.io/ide-support/vs-code |
| Debug Karate tests | Breakpoints and step-through in `.feature` files | Paid: PRO | https://docs.karatelabs.io/ide-support/vs-code |
| Debug embedded JS; mixed Java+Karate debug | Debugging across languages | Paid: PRO | https://docs.karatelabs.io/ide-support/vs-code |
| Match syntax diagnostics | Errors for `match` expressions | Paid: PRO | https://docs.karatelabs.io/ide-support/vs-code |
| Add-ons (Kafka etc.) | Async protocol runs | Paid: PRO (dev-time only) / ENTERPRISE (runtime) | https://karatelabs.io/pricing |
| SSO/SAML, offline license, SLA | Org features | Paid: ENTERPRISE | https://docs.karatelabs.io/ide-support/vs-code |

## Karate IntelliJ plugin (JetBrains)

Uses the same tier model as VS Code. It works on Community Edition. The marketplace page gave no readable details.

| Feature | Description | Free/Paid | Source URL |
|---|---|---|---|
| Syntax coloring, formatting | Editor basics | Paid: PLUS | https://docs.karatelabs.io/ide-support/intellij |
| Run feature / scenario / example | Gutter run actions | Paid: PLUS | https://docs.karatelabs.io/ide-support/intellij |
| Run configurations, structure view | IDE-native run configs and outline | Paid: PLUS | https://docs.karatelabs.io/ide-support/intellij |
| In-IDE test results | Results tree, with the HTML report "one click away" | Paid: PLUS | https://docs.karatelabs.io/ide-support/intellij |
| Auto-complete, jump to references | Jumps to `read()` and `Java.type()` targets | Paid: PRO | https://docs.karatelabs.io/ide-support/intellij |
| Code folding, JSON re-format | Editor helpers | Paid: PRO | https://docs.karatelabs.io/ide-support/intellij |
| Run all tests in folder | Folder-level run | Paid: PRO | https://docs.karatelabs.io/ide-support/intellij |
| Step debugger | Breakpoints, step-through, **step back**, **hot reload** | Paid: PRO | https://docs.karatelabs.io/ide-support/intellij |
| Mixed Java+Karate debug | Java debug sessions stop at Karate breakpoints | Paid: PRO | https://docs.karatelabs.io/ide-support/intellij |
| OpenAPI import, spec-to-test, spec-to-mock | Generates tests/mocks from Swagger/OpenAPI, with schema subset picking | Paid: ENTERPRISE | https://docs.karatelabs.io/ide-support/intellij |
| API change impact analysis | Impact of spec changes (test impact analysis "coming soon") | Paid: ENTERPRISE | https://docs.karatelabs.io/ide-support/intellij |

## Karate Xplorer (desktop app)

Xplorer is a desktop API client and collection runner for macOS, Windows and Linux. It is "100% local" and needs no sign-in for the free tier. It is a Postman-style client, not primarily a feature-file runner.

| Feature | Description | Free/Paid | Source URL |
|---|---|---|---|
| Collection tree | Folders/requests tree with method labels, pass/fail icons, context menu, inline rename, re-run from status icon | Free | https://docs.karatelabs.io/xplorer/interface |
| Request editor | Method/URL, Body/Params/Headers/Auth tabs, Before/After JS scripts | Free | https://docs.karatelabs.io/xplorer/interface |
| Response viewer | Status, headers, body; JSON tree, schema hints, highlighting | Free | https://karatelabs.io/xplorer |
| Output pane | Real-time HTML results, assertion results, errors | Free | https://docs.karatelabs.io/xplorer/interface |
| Run request / folder / collection | Aggregated results; clear results per level | Free | https://docs.karatelabs.io/xplorer/quick-start |
| Environments | One active env, live-updating variables pane, quick switching (dev/staging/prod), secret profiles | Free | https://karatelabs.io/xplorer |
| Request chaining & variables | Passes values from one response into the next request | Free | https://karatelabs.io/xplorer |
| Postman import | Collection v2.0/2.1 + env files, `pm` object, "no migration step" | Free | https://docs.karatelabs.io/xplorer/importing |
| cURL / HAR import | Mentioned in FAQ only | Free (unverified) | https://docs.karatelabs.io/xplorer/faq |
| Data-driven runs | CSV/JSON data file per folder, rolled-up report | Tier not stated (unverified) | https://docs.karatelabs.io/xplorer/interface |
| Export to Karate `.feature` | Converts explorations/Postman to CI-ready features | Free | https://karatelabs.io/xplorer |
| Protocols | REST, GraphQL, SOAP (free); gRPC tier not stated | Free / gRPC (unverified) | https://karatelabs.io/xplorer |
| AI-assisted authoring | Suggests assertions, edge cases, variants | Tier not stated (unverified) | https://karatelabs.io/xplorer |
| Themes, font zoom | Light/dark theme, font-size shortcuts | Free | https://docs.karatelabs.io/xplorer/interface |
| **Karate Runner** | Runs Karate feature files "with live output" | **Paid: Premium** | https://docs.karatelabs.io/xplorer/licensing |
| OpenAPI "Try It" / spec navigator | Executes requests from an OpenAPI spec | Paid: Premium | https://docs.karatelabs.io/xplorer/licensing |
| HTML report export | Planned Q3 2026 | Paid: Premium (planned) | https://docs.karatelabs.io/xplorer/faq |
| MCP control, git-friendly save format | Planned Q3 2026 | Paid: Premium (planned) | https://karatelabs.io/xplorer |
| Step debugger / breakpoints | Not documented for Xplorer | n/a (unverified) | https://docs.karatelabs.io/xplorer/interface |
| Request history | Not documented | n/a (unverified) | https://docs.karatelabs.io/xplorer/interface |

## Karate Agent (+ Karate Dashboard)

Karate Agent is an Enterprise-only, self-hosted, AI-driven browser and API testing platform. It needs Java 21+, Docker and a `karate.lic` file.

| Feature | Description | Free/Paid | Source URL |
|---|---|---|---|
| Karate Dashboard | Provisions browser containers and proxies live-mode commands. UI is not documented in detail | Paid: Enterprise | https://docs.karatelabs.io/agent/overview |
| Live mode | External coding agent (Claude Code, Cursor) drives a browser over REST/MCP | Paid: Enterprise | https://docs.karatelabs.io/agent/overview |
| Autonomous mode | Worker-side LLM loop for CI/scheduled/batch jobs | Paid: Enterprise | https://docs.karatelabs.io/agent/overview |
| Agent API | `look()`, `act()`, `wait()`, `match()` with display-text locators | Paid: Enterprise | https://docs.karatelabs.io/agent/overview |
| Flows | Reusable `.js` scripts, zero LLM tokens | Paid: Enterprise | https://docs.karatelabs.io/agent/overview |
| Self-healing | Re-drives drifting flows; heal "never silent" | Paid: Enterprise | https://karatelabs.io/agent |
| Evidence | Annotated screenshots, session video, captured traffic | Paid: Enterprise | https://karatelabs.io/agent |
| Coverage, traceability, READY/NOT-READY verdict | Requirement-to-test grading | Paid: Enterprise | https://karatelabs.io/agent |
| Mocks, contract/mutation testing, test data gen, rulebooks | Bundled in Agent license | Paid: Enterprise | https://karatelabs.io/pricing |
| RBAC, audit trail, air-gap | Org/security | Paid: Enterprise | https://karatelabs.io/agent |
| Scheduling / job history UI | Not described | n/a (unverified) | https://karatelabs.io/agent |

## Prior art: free community tooling (for gap context)

| Feature | Description | Free/Paid | Source URL |
|---|---|---|---|
| Karate Runner (kirkslota), MIT, 68k installs | CodeLens run/debug per feature/scenario/example row, tests tree, tags view, reports view, pass/fail gutter, status-bar env + re-run history, cURL smart paste | Free | https://marketplace.visualstudio.com/items?itemName=kirkslota.karate-runner |
| Indigo Runner | Fork of the above, archived | Free | https://github.com/ojlm/indigo-runner |

## User needs a free runner must cover

- **Browse feature tree** (folders, features, scenarios, outline examples): `must`. This is the core navigation surface. It is paid (PLUS) in the official IDE plugins.
- **Run a feature, scenario or single example row**: `must`. This is the most basic action, and it is paid in both IDE plugins.
- **Run a folder or the whole suite**: `must`. It is paid (PRO) in the IDE plugins and is basic for a suite runner.
- **Run by tag, with a tag browser and include/exclude expressions**: `must`. Karate suites are organized around tags. The community extension's tags view shows the demand.
- **Live progress with per-node pass/fail icons and counts**: `must`. This is Xplorer Premium's main selling point ("live output") and part of the IDE Test Explorer.
- **Live console log with ANSI colors, filterable**: `must`. It is needed to diagnose failures, and the official VS Code extension has it.
- **Failure details: assertion diff, stack/step location, jump to line**: `must`. Fast triage is the main value of a GUI over the CLI.
- **Environment switcher (`karate.env`) plus extra system properties**: `must`. Every product offers env switching, and Karate configs depend on it.
- **Open the generated Karate HTML report, with tag filter**: `must`. The report is free in core, so the GUI only needs to surface it.
- **Run history: past runs, re-run a run, re-run failed only**: `should`. The community extension keeps re-run history. Paid tools lack a clear history view, which is a differentiator.
- **HTTP inspector: request/response per step (headers, body, JSON tree, timing)**: `should`. Xplorer's response viewer is popular. It can be built from Karate's JSON report or log data.
- **Parallel thread count and stop/kill run**: `should`. The VS Code extension has Kill Tests. Parallelism is a core Karate strength.
- **Project setup for Maven, Gradle or standalone `karate.jar` / karate-cli**: `should`. Paid plugins have Setup Dashboard and launch configs. A runner must work with any of these build setups.
- **Step debugger with breakpoints, step over/into, variables view and step back**: `should`. It is the top PRO-gated feature. It is hard to build: v2 OSS core has no debug server, so it needs our own Java `RunListener` or more.
- **Report history and trend view (pass rate over time, flaky tests)**: `should`. No commercial tool shows this. It is cheap once run history is stored.
- **Watch mode (re-run on file save)**: `could`. It is a convenience, and no competitor shows it.
- **Open in external editor / jump to file:line**: `could`. A runner is not an editor, but it needs to hand off to one.
- **Import Postman, cURL or OpenAPI to `.feature`**: `could`. Xplorer and Enterprise do this. It is outside a runner's core job.
- **JUnit XML / HTML report export and sharing**: `could`. Core already writes these files. Export is planned as a Xplorer Premium feature.
- **Data-driven run view (per-row results of Examples or CSV)**: `could`. Outline rows already appear in the tree. A dedicated view is extra.
- **AI or MCP control of the runner**: `could`. Xplorer has it planned (Premium) and Agent has it (Enterprise). It is not needed for a first version.
