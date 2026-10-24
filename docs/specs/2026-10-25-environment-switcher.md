# Environment switcher and run properties

Date 2026-10-25. Status: approved. Covers [features](../features.md) F08.

## Intent

Karate projects run against several environments (dev, qa, prod) and read settings and secrets from system properties. Today every karate-kid run has no environment and no extra properties, so a project that needs `karate.env` or a token cannot be run from the app. This feature adds an environment picker and a per-project list of run properties, some of them secret, so the person can run the same suite against qa and then prod with the right values, without typing tokens on a command line.

It builds on the [walking skeleton](2026-10-24-walking-skeleton.md) and the [tag filter](2026-10-24-tag-filter.md): same toolbar, same per-project settings, same one-process-at-a-time runner.

## Who and what success looks like

The walking skeleton's user, with a project whose `karate-config.js` reads `karate.env` and `karate.properties`.

Success:
- Picking an environment takes two clicks, and the tree shows the tests that exist in it.
- A run gets `karate.env` and the right properties for that environment, and a secret never appears on screen, on the Java command line or in plain text on disk.
- Environment and properties are still set after a restart.

## Goals, non-goals, anti-goals

**Goals:** the behaviour in the Requirements section below.

**Non-goals for this version:**
- Editing `karate-config*.js` or reading environment names out of JavaScript or JSON.
- Passing OS environment variables to the run (they are inherited as today; see R7).
- Masking secrets in Karate's own outputs (HTML report, event file): only the project's `configure logging mask` controls those.
- Importing or exporting properties, sharing them between projects or machines.
- A config directory (`-g`) setting.

**Anti-goals** (failure even if everything else works):
- A secret value readable on screen after it was entered, on the Java command line (Task Manager, process lists), or in plain text in the app's files.
- A run using a different environment or different property values than the window shows.
- Running without a secret the person set, because it could not be decrypted, without saying so.
- A property silently replacing the app's own Karate options (`karate.options`, `karate.env`).

## Constraints

- Everything in the walking skeleton's Constraints section.
- Electron `safeStorage` for encryption; no new dependency. Decided in [ADR 0008](../decisions/0008-secret-run-properties-and-java-argument-file.md).

### What Karate does (probes, 2026-10-24/25, Karate 2.1.4, bundled Java 21)

| Probe | Result |
|---|---|
| Dry run, no `-e` | `SUITE_ENTER.env` null; a scenario tagged `@env=dev` is absent, one tagged `@envnot=dev` is present |
| Dry run, `-e dev` | `SUITE_ENTER.env` `"dev"`; `@env=dev` present, `@envnot=dev` absent |
| Run, `-e nope` | exit 0, no warning; `karate.env` is `nope` |
| Run, `java -Dmy.prop=hello …` | `karate.properties['my.prop']` is `hello` |
| Run, `java -Dkarate.env=qa …` without `-e` | `karate.env` is `qa` |
| Run, `java "-Dkarate.options=-t @nomatch" …` | nothing ran, exit 0: the option replaced the command line |
| `java @args.txt …` with `"-Da=x y \"q\""` | `karate.properties['a']` is `x y "q"` |
| same, `"-Db=C:\Users\me"` | `C:Usersme`: inside quotes `\` is an escape and must be doubled |
| same, a UTF-8 `é` | arrived as `Ã©`: Java reads the file in the Windows code page |

Karate's [configuration docs](https://docs.karatelabs.io/core-syntax/configuration) show three ways to define environments, none mandatory: one `karate-config.js` branching on `karate.env` (the main example, defaulting to `dev` in the config), `karate-config-<env>.js` overlay files loaded after it, and per-environment JSON read from the config. Secrets are read from environment variables or `-D` properties.

## Approaches considered

How properties reach Java:
1. **Every property in one Java argument file** (`java @file`), written just before a run and deleted after it. The command line shows only the file path; one code path; escaping tested once. **Chosen.**
2. Plain properties as `-D` arguments, secrets in the file. Two paths, and plain values show in process lists.
3. Everything as `-D` arguments. Secrets readable by any process that lists command lines. Rejected.

Where secrets are stored:
1. **Electron `safeStorage`** (Windows DPAPI, bound to the Windows user), ciphertext in `settings.json`. Built in, no new dependency. **Chosen.**
2. Windows Credential Manager through a native module (`keytar`). Visible in Windows' own UI, but a native dependency rebuilt per Electron version, and `keytar` is archived.
3. The app's own encryption with a key file. No real protection: the key sits next to the data.

Editor layout (mockups compared in the brainstorm):
1. **A dialog with "All environments" and one list per environment.** Edit any environment without switching to it. **Chosen.**
2. A drawer under the toolbar showing the merged list for the chosen environment. Shows exactly what the next run gets, but editing prod means switching the run to prod.

## Requirements

### R1. Environment picker

- The toolbar has an **Env** combo box next to the project path. Its value is the project's environment, or "(none)".
- The person can type any name or pick a suggestion. Suggestions, de-duplicated and sorted:
  - `<env>` of every `karate-config-<env>.js` in the project's classpath roots and in the project folder itself (walking skeleton R1 layouts);
  - every value of `@env=` and `@envnot=` tags written in the project's feature files (`@env=dev,qa` gives `dev` and `qa`). Main reads them from the files under the feature roots, not from the tree, because the tree only holds the scenarios of the current environment (builder check);
  - every environment that has a property list (R3);
  - the current environment.
- A name must match `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$`. An invalid name is not applied and the combo says why.
- If the chosen name has no `karate-config-<name>.js`, a hint under the combo says "No karate-config-<name>.js found. Fine if karate-config.js handles it." It is a hint, not an error.
- Choosing "(none)" or clearing the box means no environment.
- A typed name takes effect on Enter or when the box loses focus; a picked suggestion takes effect at once. Typing alone changes nothing.
- Changing the environment re-runs discovery (walking skeleton R2) with the new environment, because `@env`/`@envnot` tags change which tests exist. The tag filter is pruned as after a Refresh (tag filter R5).
  - The new environment is saved and used once that discovery ends, also when it ends with a parse error (exit 3: the tree area shows the error, as today).
  - Cancelling that discovery puts the previous environment, tree and filter back, and nothing is saved.
  - All statuses, results, steps, the run view and the report are cleared, as when another folder is opened: a result from qa must not show as the state of prod.
- The combo is disabled while the runner is busy.

### R2. Discovery and runs use the environment

- Every discovery dry run and every run passes `-e <env>` when an environment is set, and nothing when it is not. The app never passes `-Dkarate.env`.
- Properties (R3) are passed to runs only, never to dry runs (dry runs do not execute config).
- `KARATE_ENV` and `KARATE_CONFIG_DIR` are removed from the child environment, as `KARATE_OPTIONS` already is, so the machine's environment never overrides what the window shows.
- The progress section shows the environment of the run in progress or last finished ("qa", or nothing for none).

### R3. Run properties

- A **Properties…** button in the toolbar opens a dialog. The left side lists "All environments", then every environment that has a list, then the currently chosen environment if it has none yet, and a "+ environment" control that adds a list for a valid name (R1 rule). The right side shows the selected list.
- Each row has a name, a value, a **Secret** checkbox and a delete control; "+ Add property" adds a row.
- Name rules: matches `^[A-Za-z_][A-Za-z0-9_.-]{0,199}$`; unique within its list; not reserved. A rejected name shows the reason on its row. Reserved, compared case-insensitively:
  - the `karate.*` system properties that Karate 2.1.4 itself reads, found by scanning the bundled jar for `System.getProperty` keys: `karate.env`, `karate.options`, `karate.config.dir`, `karate.output.dir`, `karate.debug.port`, `karate.js.slotFrames`. Other `karate.*` names stay allowed, because projects use their own;
  - names starting with `java.`, `jdk.` or `sun.`;
  - `user.dir`, `user.home`, `file.encoding`, `native.encoding`, `stdout.encoding`, `stderr.encoding`, `line.separator`, `path.separator`.
- Value rules: at most 10,000 characters; no line breaks; it must survive the trip through the argument file unchanged (see "The argument file"). A value that cannot is rejected on its row with "This value cannot be passed to Java exactly."
- Edits apply on **Save**; **Cancel** discards them. Save is refused while any row is invalid. The dialog cannot be opened while the runner is busy.
- An environment list can be deleted with its rows.

### R4. Effective properties of a run

- With an environment chosen: all rows of "All environments" plus all rows of that environment's list; on the same name, the environment's row wins.
- With no environment: the rows of "All environments" only.

### R5. Secrets

- A row marked Secret stores its value encrypted with `safeStorage` (R6). If `safeStorage.isEncryptionAvailable()` is false, the Secret checkbox is disabled with "Secret storage is not available on this system" and existing secrets cannot be used (runs are refused as below).
- Once saved, a secret's value is never sent to the window again. Its row shows dots and a **Replace** control; replacing takes a new value. Unticking Secret on a saved secret empties the value: the person types the plain value again.
- If a secret of the effective set (R4) cannot be decrypted (for example the settings file came from another Windows account), its row shows "Can't be read, enter it again", and runs are refused with the notice "Secret <name> (<scope>) can't be read. Replace it in Properties." Discovery still works.
- The app replaces every occurrence of each effective secret value with `****` in everything it displays from a run: console lines, scenario errors and step logs in the details pane. It masks the value as typed, its URL-encoded form (`encodeURIComponent`) and its JSON-escaped form. It cannot recognise a secret folded into other text, such as `Authorization: Basic <base64 of user:secret>`; the dialog note says so. A secret shorter than 4 characters is allowed, with the row hint "Too short to hide in the console".
- The dialog shows, while any secret exists: "Secrets are hidden in this app, except inside encoded text such as a Basic auth header. Karate's HTML report and event file can still contain them unless the project sets `configure logging mask`."

### R6. Saved settings

- The environment and the property lists are saved per project in `settings.json`, in the same per-project entry as the tag filter, and restored when the project opens. Saving one part of the entry keeps the others (today `saveProjectFilter` replaces the whole entry: that changes).
- A secret is stored as the base64 of `safeStorage.encryptString(value)`. On Windows, Electron keeps the key for that in the app data folder (`Local State`), itself protected by DPAPI, so a secret can be read back only by the same Windows user with the same app data folder. Its plain value is never written to disk by the app, except inside the short-lived argument file (below).
- A malformed entry, list or row reads as missing; it never blocks opening a project.

## Design decisions already made

### Settings entry

```json
"projects": {
  "D:\\work\\api-tests": {
    "include": ["smoke"], "exclude": [], "hideNonMatching": false,
    "env": "qa",
    "properties": {
      "shared": [{ "name": "timeout", "value": "5000" }],
      "byEnv": {
        "qa": [
          { "name": "timeout", "value": "9000" },
          { "name": "api.token", "secret": "djEw…base64…" }
        ]
      }
    }
  }
}
```

`env` is absent for no environment. A row has `value` (plain) or `secret` (ciphertext), never both.

### The argument file

- Before each run, main writes `<app data>\projects\<hash>\run-args.txt` with one line per effective property: `"-D<name>=<value>"`, with `\` written as `\\` and `"` as `\"`. It is written in the encoding Java reads it with (the Windows ANSI code page); a value that does not encode losslessly in it is rejected at Save time (R3).
- The Java command line becomes `java @<that file> -Dstdout.encoding=UTF-8 -Dstderr.encoding=UTF-8 -cp … io.karatelabs.Main run …`, so the app's own `-D` options come after the file.
- The file is deleted when the run ends, is cancelled or fails to start, and at app start if one is left over from a crash. With no effective properties, no file is written and no `@` argument is passed.
- `<app data>` is under the user's profile, so the file inherits that folder's user-only permissions.

### Where it lives

- Main owns the environment and the properties: it loads them per project, decrypts secrets only to build the argument file and the masking list, saves changes, and sends the window a view in which each secret row carries only `{ name, secret: true, state: "set" | "unreadable" }`.
- The window sends edits over IPC; a secret row carries either a new value or "keep". Main validates every field (R1, R3 rules) and refuses the whole save on any invalid row.
- Masking runs in main, on the text before it is sent to the window.

## Acceptance criteria

Test projects:
- The **clean sample project** (has `karate-config-dev.js`).
- A new **env sample project**: `karate-config.js` returns `{ env: karate.env }`; a `karate-config-qa.js` overlay; `tests/env.feature` with an untagged scenario `always` that prints `karate.env` and the properties `timeout` and `api.token`, a scenario `qa only` tagged `@env=qa`, and a scenario `not prod` tagged `@envnot=prod`. All pass in every environment.

Every scenario is automated with Playwright against the built app, each on a fresh copy of its fixture and a fresh app data folder.

```gherkin
Feature: Environment switcher and run properties

  Background:
    Given karate-kid is built with the bundled Java and Karate on Windows 11

  Scenario: Suggestions come from overlay files and env tags
    When I open the env sample project
    Then the Env combo shows "(none)"
    And its suggestions are "prod" and "qa"

  Scenario: Changing the environment updates the tree
    Given the env sample project is open
    Then "qa only" is not in the tree and "not prod" is
    When I choose "qa"
    Then "qa only" is in the tree
    When I choose "prod"
    Then "qa only" and "not prod" are not in the tree

  Scenario: Changing the environment clears results
    Given the env sample project is open with environment "qa"
    And "always" passed in a run
    When I choose "prod"
    Then "always" shows as not run and Report is disabled

  Scenario: A run uses the environment
    Given the env sample project is open with environment "qa"
    When I run "always"
    Then the console shows "karate.env = qa"
    And the progress section shows "qa"

  Scenario: An environment without an overlay file
    Given the env sample project is open
    When I type "staging" in the Env combo
    Then the hint says no karate-config-staging.js was found
    And runs use environment "staging"

  Scenario: Shared and environment properties, environment wins
    Given the env sample project is open with environment "qa"
    And "All environments" has "timeout" = "5000"
    And "qa" has "timeout" = "9000"
    When I run "always"
    Then the console shows "timeout = 9000"
    When I choose "(none)" and run "always"
    Then the console shows "timeout = 5000"

  Scenario: A secret is used but never shown
    Given the env sample project is open with environment "qa"
    And "qa" has the secret "api.token" = "s3cret-token-123"
    When I run "always"
    Then the console shows "api.token = ****"
    And no text in the window contains "s3cret-token-123"
    And the Properties dialog shows "api.token" as dots with Replace

  Scenario: A secret stays off the command line and out of the settings file
    Given the env sample project is open with environment "qa"
    And "qa" has the secret "api.token" = "s3cret-token-123"
    When a run is in progress
    Then the Java process command line does not contain "s3cret-token-123"
    And settings.json does not contain "s3cret-token-123"
    And after the run no argument file is left

  Scenario: Reserved names are refused
    Given the Properties dialog is open
    When I add the property "karate.options" = "-t @x"
    Then the row says the name is reserved
    And Save is disabled

  Scenario: An unreadable secret blocks runs
    Given the env sample project has a saved secret whose ciphertext is corrupted
    When I open it and click Run all
    Then nothing runs and the notice says the secret can't be read
    And the Properties dialog shows "Can't be read, enter it again"

  Scenario: Settings survive a restart
    Given the env sample project is open with environment "qa" and a secret "api.token"
    When I close and restart the app
    Then the Env combo shows "qa"
    And the Properties dialog shows "api.token" as a saved secret

  Scenario: The machine's KARATE_ENV is ignored
    Given the app is started with KARATE_ENV=prod in its environment
    When I open the env sample project and run "always" with no environment
    Then the console shows "karate.env = null"
```

Also automated (Vitest):
- Environment suggestions from overlay files in each layout and from `@env`/`@envnot` tag values; name validation.
- Property name and value validation, reserved names in any case, duplicate names, the override rule (R4).
- Argument file writing: quoting, `\` and `"` escaping, code-page encoding, the lossless check; and a Node test with the bundled Java that a value with spaces, quotes, backslashes and `%` reaches `karate.properties` unchanged.
- Settings: load and save of env and properties, keeping the filter and other projects; malformed rows read as missing.
- Secrets: encrypt, decrypt, unreadable ciphertext, and the window view never carrying a value (with `safeStorage` behind a small seam so Node tests can run without Electron).
- Masking of console lines, errors and step logs, including a secret that appears twice on a line.
- `childEnv` removes `KARATE_ENV` and `KARATE_CONFIG_DIR` in any case.

## Left to the builder

- Component structure, styling, exact IPC names, how the combo and dialog are built with the project's existing components.
- Exact wording of hints and validation messages beyond the ones quoted above.
- Whether the progress section shows the environment as text or a badge.

Resolved by the builder check (2026-10-25), as the spec author's calls: results are cleared on an environment change; a typed environment applies on Enter or blur, reverts on cancel, sticks on a parse error; reserved names are Karate's own `karate.*` keys plus JVM namespaces; `@env` suggestions come from the feature files; no separate list of recently used environments; masking covers URL-encoded and JSON-escaped forms but not text folded into base64; values must be printable ASCII (U+0020 to U+007E), because Java reads the argument file in the Windows ANSI code page and only ASCII is identical in every code page, so any other character is rejected with "This value cannot be passed to Java exactly."
