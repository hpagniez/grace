# Run properties as environment variables, imported from a secrets file

Date 2026-11-04. Status: approved. Covers [#230](https://github.com/hpagniez/grace/issues/230). Builds on [Environment switcher and run properties](2026-10-25-environment-switcher.md) (#59) and [ADR 0008](../decisions/0008-secret-run-properties-and-java-argument-file.md).

## Intent

Run properties reach Karate as `-D` system properties. Many suites read their secrets from environment variables instead, so the same values work in CI: a suite's `karate-config.js` calls `java.lang.System.getenv('<PREFIX>_<KEY>')` and falls back to a `secrets-<env>.properties` file, which ends up committed because there is no better local place for it. Grace can set none of these today, and the person would retype every value from the file.

With this feature a run property row can be an environment variable of the Karate process, kept encrypted and masked like today's secrets, and **Import from file…** turns a project's secrets file into secret rows once. The person imports `secrets-dev.properties` into `dev`, deletes the file from the repository, and the suite reads the same values from the environment, in Grace as in CI.

## Goals, non-goals, anti-goals

- Goals: each row says whether it is a system property (`-Dname=value`, as today) or an environment variable (`NAME=value`); environment variables reach runs and debug runs per environment; a `.properties` file is imported as secret rows of one environment in one step, with what it replaces and skips shown before anything changes.
- Non-goals: other file formats (`.env`, JSON, YAML); keeping the file and Grace in sync; exporting rows to a file; guessing the environment or the prefix from the file name; remembering the prefix, the kind or the file; importing a key as a non-secret row (the person unticks Secret before Save instead).
- Anti-goals: any cost while idle; Grace writing into the project, or keeping the file's path; a secret's value in a notice, a log, settings in plain text, or a dry run; a way to set the variables Grace strips from Karate's environment; a new dependency.

## Constraints

- Follow the Karate CLI first ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)). The CLI has no way to set an environment variable: `karate-pom.json` has no such key and `RunCommand.java` no such flag; the shell sets them. Grace setting them on the Karate process it starts is what the shell does for the CLI. Karate's surface is the environment of its process, read by `karate-config.js` with `java.lang.System.getenv`: no new flag.
- Secrets stay encrypted at rest with `safeStorage` and masked in everything Grace shows ([environment switcher R5, R6](2026-10-25-environment-switcher.md#r5-secrets)).
- Grace never writes into a project.

### What exists today

- `src/shared/env.ts` holds the rules: `PROPERTY_NAME`, `propertyNameError` (reserved Java and Karate names), `propertyValueError` (at most 10,000 printable ASCII characters, no line break), `parsePropertiesEdit`, `editErrors` and `effectiveRows` (an environment's row wins by name over a shared one).
- `src/main/settings.ts` stores a row as `{ name, value }` or `{ name, secret: "<base64 ciphertext>" }`. A saved row today's rules reject is not loaded, is named in one notice on open, and stays in the file.
- `Runner.saveProperties` in `src/main/runner.ts` encrypts new secret values with `SecretBox` and copies the ciphertext of a `keep` row. `runProperties` decrypts the effective rows of a run, refusing it with "Secret <name> (<scope>) can't be read. Replace it in Properties." when one cannot be read, and returns the secret values for `makeMasker` (`src/main/mask.ts`), which masks every event of the run: console, details, HTTP inspector and the run history built from them.
- `runBody` writes the effective rows to `run-args.txt` as `-D` options (ADR 0008). Dry runs get no properties.
- `spawnKarate` (`src/main/karate/process.ts`) gives Java `childEnv(process.env)`, which drops `KARATE_OPTIONS`, `KARATE_ENV`, `KARATE_CONFIG_DIR`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS` and `_JAVA_OPTIONS` in any letter case (`STRIPPED` in `src/main/karate/command.ts`). The build tool that lists libraries gets `toolEnv` instead.
- `src/renderer/PropertiesDialog.tsx` edits a draft of all lists; Save sends it and closes once main applied it; Cancel and Escape drop it. A row has a name, a value (dots and **Replace** for a saved secret), a **Secret** checkbox and a delete button.

## Approaches considered

How it is built:

1. **A per-row kind and an import step inside the dialog. Chosen by the owner.** Main shows the file picker, reads and parses the file and hands the keys to the dialog; the dialog's import step maps, checks and adds them to the draft; Save encrypts them as it does a typed secret. Reuses the draft, Save, Cancel and every rule; the values pass through the window as typed secrets already do.
2. **The import done entirely in main**, which encrypts and saves the rows so the window never holds the values. It bypasses the dialog's draft: unsaved edits would have to be saved or merged first, and the replace list would need a second round trip without values. More code for little gain.
3. **A separate list of environment variables per environment.** Clearer to scan, but doubles the lists, the rules and the room they take, and an environment's row overriding a shared row of the other kind gets awkward.

The owner's answers to the open points of #230:

- Formats: `.properties` only. `.env` has no single standard (quotes, `export`, `${VAR}` expansion differ between tools); it can be its own issue.
- Every imported key becomes a secret row; no non-secret import. An imported row is in the draft with its value until Save, so unticking Secret keeps the value.
- Names: the prefix as typed, then the key in upper case with every character other than a letter, digit or `_` turned into `_`. Not remembered.
- Replacing: the import step lists what it adds, replaces and skips; **Import** is the confirmation; the draft, Save and Cancel do the rest. No second confirmation box.

## Requirements

### R1. The kind of a row

1. Each row has an **Env variable** checkbox between the value and **Secret**. Unticked, the row is a system property, as today; ticked, an environment variable. A new row starts unticked.
2. Ticking or unticking it keeps the row's value, a saved secret's included: the kind is not part of the secret.
3. The name rules depend on the kind. A system property keeps today's rules. An environment variable's name matches `^[A-Za-z_][A-Za-z0-9_]{0,199}$` ("Invalid name" otherwise) and is not one of the variables Grace strips from Karate's environment, compared case-insensitively: `KARATE_OPTIONS`, `KARATE_ENV`, `KARATE_CONFIG_DIR`, `JAVA_TOOL_OPTIONS`, `JDK_JAVA_OPTIONS`, `_JAVA_OPTIONS` ("Reserved name" otherwise). Main and the window use one list for stripping and for refusing.
4. The value rules are the same for both kinds (at most 10,000 printable ASCII characters, no line break), so the masker's forms stay exact.
5. Names stay unique within a list whatever their kind, and two environment variables of a list may not differ only in letter case, since Windows sees one variable: either is a "Duplicate name". An environment's row still wins by name over a shared row (environment switcher R4), its kind with it.

### R2. Environment variables reach the run

1. A run and a debug run on environment `<env>` start Java with `childEnv(process.env)` plus every effective row (R4 of the environment switcher) that is an environment variable, with its decrypted value. Each replaces a variable of the same name in any letter case, the machine's or an earlier row's, so the process gets exactly one; an environment's row comes after the shared rows and wins over a shared one that differs only in case.
2. Only system property rows go to `run-args.txt`; with none, no argument file is written and no `@` argument is passed, as today.
3. Dry runs and the build tool that lists libraries get no environment variable rows, as they get no system properties.
4. An unreadable secret of either kind refuses the run with today's notice.
5. Every effective secret's value, of either kind, is masked as today. Plain values are not masked, of either kind.

### R3. Saved settings

1. An environment variable row is stored like a system property row with `"envVar": true` added: `{ "name": "APP_DB_USER", "secret": "djEw…", "envVar": true }` or `{ "name": "LOG_LEVEL", "value": "debug", "envVar": true }`. A row without `envVar` is a system property. A row whose `envVar` is anything but `true`, or whose name breaks its kind's rules, is not loaded and stays in the file, as today.
2. Rows saved before this change have no `envVar` and load as system properties: nothing is lost.

### R4. Import from file…

1. An **Import from file…** button sits next to **Add property**. It is disabled while secret storage is not available, since every imported row is a secret.
2. A click asks main for a file. Main shows Windows' Open dialog titled "Import properties" with the filters "Properties files (*.properties)" and "All files (*.*)". Closing it does nothing.
3. Main reads the chosen file once, on that click, and never writes it, keeps its path or logs its content:
   - over 1 MB: the dialog shows "<file name> is too large to import";
   - unreadable: "<file name> could not be read: <reason>";
   - otherwise main drops a leading UTF-8 byte order mark (Windows editors write one), decodes the rest as ISO-8859-1, as `java.util.Properties.load(InputStream)` does, and parses it by that method's rules: `#` and `!` comment lines, blank lines, `=`, `:` or white space between key and value, a line ending in an odd number of `\` continuing on the next one, and the escapes `\t`, `\n`, `\r`, `\f`, `\uXXXX` and `\<char>`. A malformed `\uXXXX` refuses the whole file, as in Java: "<file name> could not be imported: bad \uXXXX escape on line <n>". A key given twice keeps its last value, at its first place.
   - A file with no keys shows "<file name> has no properties".
4. Main sends the window the file's name (without its folder) and the keys with their values, in file order. The messages above show in the dialog under its buttons, never as notices, and never hold a value.

### R5. The import step

1. With the keys, the dialog's right side shows the import step in place of the list:
   - "Import <file name>";
   - **Environment**: a choice of the dialog's lists ("All environments", each environment with a list, and the chosen environment), starting on the list that was shown;
   - **Environment variables**, a checkbox, ticked at first; unticked, the keys become system properties named exactly as written;
   - **Prefix**, shown while Environment variables is ticked, empty at first; letters, digits and `_` only, else "Letters, digits and _ only" under it; with it a preview of the first key that can be imported: `db.user → APP_DB_USER`;
   - what Import will do, updated on every change: "Adds <n>", "Replaces <n>: <name>, <name>, …" for names that already have a row in the chosen list (the same name by R1.5, so `APP_DB_USER` replaces a variable `App_Db_User`), and "Skips <n>:" with one line per skipped key, `<key>: <reason>`, never its value;
   - **Import** (primary) and **Back**.
2. A variable's name is the prefix as typed, then the key in upper case with every character other than `A`–`Z`, `0`–`9` and `_` turned into `_`: prefix `APP_` and key `db.user` give `APP_DB_USER`.
3. A key is skipped, with the first reason that applies: its name breaks its kind's rules (R1.3; "Invalid name", "Reserved name"); its value is empty ("Empty value"); its value breaks the value rules (today's reasons); it maps to the same name as an earlier key ("Same name as <earlier key>").
4. **Import** is disabled while the prefix is invalid, while nothing would be added or replaced, while the chosen list would pass 500 rows ("At most 500 properties per list" under the summary), and while the chosen environment has no list and 100 lists exist ("At most 100 environments"). A click adds every key not skipped to the chosen list's draft as a secret row of the chosen kind with the value read. A row of the same name in that list is replaced in place: its kind becomes the chosen one, a saved secret's value is dropped for the new one. The other rows go after the list's rows, in file order. The chosen environment's list is created if it has none. The import step closes and the dialog shows that list.
5. **Back** closes the import step and changes nothing. Closing the dialog drops the keys with the draft.
6. Nothing is saved until **Save**; **Cancel** drops the import with every other edit. Rows of that list that are not in the file stay.

## Cost

- Idle: nothing. No timer, poll or listener. The checkbox and the button exist only while the dialog is open, which the bench never opens, so `page.nodes.*` do not move.
- A run: the environment variable rows are a few more entries in the environment block Node builds at spawn; no file, no process. The argument file is written as today, for system properties only.
- An import: one read of a file of at most 1 MB on a click, parsed in one pass; the keys live in the dialog's memory until it closes.
- Start time: no change. Size: about 3 to 4 KB of code; no dependency.

## Design decisions already made

- A per-row kind and an import step inside the dialog: the owner's choice (approach 1).
- `.properties` only; every imported key a secret; the name pattern; the import step as the confirmation: the owner's answers.
- Environment variables are set on the Karate process's environment ([ADR 0029](../decisions/0029-environment-variable-run-properties.md)).
- My call: the kind is a checkbox like Secret, not a select: the smallest control, and it reads the same way.
- My call: one value rule for both kinds. An environment variable could take any Unicode, but the masker's forms assume printable ASCII (`secretForms`), and one rule keeps rows switchable between kinds.
- My call: no extra reserved names (`PATH`, `SystemRoot` and the like). Setting one is the person's choice, as in a shell; Grace starts Java by its full path.
- My call: the import's messages show in the dialog, not as notices, since they are about the dialog's step.
- My call: ISO-8859-1 decoding, as `Properties.load(InputStream)`; every value is printable ASCII anyway, so only skip reasons would differ under UTF-8.
- My call: "All files" in the Open dialog, so a secrets file with another extension can still be picked; it is parsed as `.properties` all the same.
- My call: two environment variables of one list that differ only in letter case are a "Duplicate name", since Windows sees one variable; across lists, the later effective row wins, which keeps the environment over the shared list. System property names stay case-sensitive, as today.
- My call: an import that would pass 500 rows or a 101st list is blocked with the existing messages, rather than skipping keys past the limit: the person sees the whole file's effect or none.
- My call: a leading UTF-8 byte order mark is dropped, unlike Java, since Windows editors write one and it would land in the first key's name.

## Acceptance criteria

Scenarios run in Vitest (jsdom for the window, Node for main, real Karate on the `env` fixture where a run is involved) unless marked **(e2e)**, which run in Playwright against the built app with `dialog.showOpenDialog` replaced by one that returns a file in a temporary folder. The `env` fixture's `always` scenario gets one more step, `* print 'APP_API_TOKEN =', java.lang.System.getenv('APP_API_TOKEN')`, after its last; the scenarios after it move down one line, which the builder checks no test depends on.

```gherkin
Feature: Run properties as environment variables

  # R1
  Scenario: The name rules follow the kind
    Given a row named "db.user"
    When I tick its Env variable box
    Then the row shows "Invalid name" and Save is disabled
    When I rename it "DB_USER"
    Then the row shows no error

  Scenario: Stripped variables cannot be set
    When main is sent an environment variable row named "java_tool_options"
    Then it refuses the save with "Properties not saved: All environments: java_tool_options: Reserved name"

  Scenario: Two variables that differ only in case
    Given a list with the environment variable "App_Token"
    When I add the environment variable "APP_TOKEN" to it
    Then both rows show "Duplicate name"
    And the system property "app_token" beside them shows no error

  Scenario: Switching the kind keeps a saved secret
    Given a saved secret system property "API_TOKEN"
    When I tick its Env variable box and save
    Then its ciphertext is unchanged and the row is stored with "envVar": true

  # R2
  Scenario: An environment variable reaches the run and is masked
    Given the secret environment variable "APP_API_TOKEN" in "qa"
    When I run "always" on "qa"
    Then the console shows "APP_API_TOKEN = ****"
    And run-args.txt is not written

  Scenario: Both kinds in one run
    Given the system property "timeout" = "9000" and the environment variable "APP_API_TOKEN" in "qa"
    When I run "always" on "qa"
    Then Karate prints "timeout = 9000" and "APP_API_TOKEN = ****"
    And run-args.txt held only the "timeout" line

  Scenario: The variable replaces the machine's in any letter case
    Given the machine sets "app_api_token" to "machine"
    And the plain environment variable "APP_API_TOKEN" = "grace" in "All environments"
    When I run "always"
    Then Karate prints "APP_API_TOKEN = grace"

  Scenario: The environment's variable wins over a shared one in another case
    Given the plain environment variable "app_api_token" = "shared" in "All environments"
    And the plain environment variable "APP_API_TOKEN" = "qa" in "qa"
    When I run "always" on "qa"
    Then Karate prints "APP_API_TOKEN = qa"

  Scenario: Dry runs get no environment variables
    Given the environment variable "APP_API_TOKEN" in "All environments"
    When the project is opened
    Then the dry run's environment has no "APP_API_TOKEN"

  # R3
  Scenario: Old rows load as system properties
    Given settings.json holds { "name": "timeout", "value": "5000" }
    When the project opens
    Then the row is a system property

  Scenario: A malformed kind is kept, not loaded
    Given settings.json holds { "name": "A", "value": "1", "envVar": "yes" }
    When the project opens
    Then the notice names "All environments: A"
    And the row stays in settings.json after a save

  # R4
  Scenario: Main parses a .properties file as Java does
    Given a file with a comment, "a=1", "b : 2", "c 3", "d=x\\", "  y", "e=A", and "a=4"
    When main imports it
    Then the keys are a = "4", b = "2", c = "3", d = "xy", e = "A", in that order

  Scenario: A byte order mark is dropped
    Given a UTF-8 file with a byte order mark whose first line is "db.user=u"
    When main imports it
    Then the first key is "db.user"

  Scenario: A malformed escape refuses the file
    Given a file whose line 2 is "k=\u00G1"
    When main imports it
    Then the dialog shows "secrets.properties could not be imported: bad \uXXXX escape on line 2"
    And no key reaches the window

  Scenario: A file over 1 MB is refused
    When main imports a 2 MB file
    Then the dialog shows "big.properties is too large to import"

  Scenario: Closing the Open dialog does nothing
    When I click Import from file… and close the Open dialog
    Then the list is shown unchanged

  # R5
  Scenario: The import step lists adds, replaces and skips
    Given "dev" has the secret system property "APP_DB_USER"
    And a file with "db.user=u", "db.password=p", "9lives=x" and "empty="
    When I import it into "dev" as environment variables with no prefix
    Then it says "Adds 2", replaces nothing, and skips "9lives: Invalid name" and "empty: Empty value"
    When I type the prefix "APP_"
    Then it says "Adds 2", "Replaces 1: APP_DB_USER", the preview is "db.user → APP_DB_USER", and skips only "empty: Empty value"
    And no value of the file shows in the dialog

  Scenario: Two keys with one name
    Given a file with "db.user=a" and "db_user=b"
    When I import it as environment variables with the prefix "APP_"
    Then it skips "db_user: Same name as db.user"

  Scenario: An invalid prefix blocks the import
    When I type the prefix "APP-"
    Then it shows "Letters, digits and _ only" and Import is disabled

  Scenario: An import past the list limit is blocked
    Given "dev" has 499 rows
    When I pick a file with 2 new keys for "dev"
    Then it shows "At most 500 properties per list" and Import is disabled

  Scenario: Import adds to the draft only
    Given a file with "db.user=u" and "db.password=p"
    When I import it into "dev" as environment variables with the prefix "APP_"
    Then "dev" shows the secret environment variable rows APP_DB_USER and APP_DB_PASSWORD
    And settings.json is unchanged
    When I click Cancel and open the dialog again
    Then "dev" has neither row

  Scenario: Back changes nothing
    When I pick a file and click Back
    Then the list is shown unchanged

  Scenario: Import into an environment with no list yet
    Given the chosen environment "qa" has no list
    When I import a file into "qa"
    Then "qa" gets a list holding the imported rows

  Scenario: Import needs secret storage
    Given secret storage is not available
    Then Import from file… is disabled

  Scenario: Import a secrets file and run with it (e2e)
    Given the env project is open on "qa" and a file "secrets-qa.properties" holds "api.token=<secret>"
    When I import it into "qa" as environment variables with the prefix "APP_" and save
    Then settings.json holds APP_API_TOKEN with "envVar": true and a ciphertext, never the value
    And the file is unchanged
    When I run "always"
    Then the console shows "APP_API_TOKEN = ****"
    And the value shows nowhere in the window
```

## Left to the builder

- The types that carry the kind in `RowView`, `RowEdit` and the run's rows, and where the `.properties` parser lives in `src/main/`.
- The IPC channel's name and the exact shape of its answer (keys, or a message, or nothing for a closed picker).
- How the import step lays out inside the dialog's right side, with the UI conventions (tokens, `buttons.ts` and `ui.ts` classes, `FieldMessage`, no `title=`).
- Moving the stripped list into `src/shared/env.ts` so `childEnv` and the name rule read one list.
