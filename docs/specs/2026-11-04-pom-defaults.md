# Start from the project's pom

Date 2026-11-04. Status: approved. Covers [#228](https://github.com/hpagniez/grace/issues/228). A later slice of [#64](https://github.com/hpagniez/grace/issues/64); builds on [Project setup](2026-11-02-project-setup.md).

## Intent

A project run with `karate run` or `mvn test` runs differently in Grace on first open. Seen on a Maven suite whose `karate-pom.json` holds `"env": "dev"`, `"tags": ["~@data-specific"]` and `"threads": 4`, and whose `karate-config.js` calls `karate.fail` when `karate.env` is unset. Opened in Grace, no environment is picked, so every scenario fails with the same error; Run all includes the `@data-specific` scenarios; and the run uses 1 thread.

Grace should start where the project's own setup starts, as the Karate CLI does ([ADR 0026](../decisions/0026-follow-the-karate-cli-first.md)): the environment, tag filter and thread count come from `karate-pom.json`, and a Maven project's default `karate.env` from its `pom.xml`. The person's own choices still win, and the controls stay the only source of a run.

This replaces two earlier choices: the project setup spec's non-goal "`env`, `tags`, `threads` … of the pom as starting values", and the thread count spec's "The box starts at 1 … even when its `karate-pom.json` sets `threads`" ([2026-10-26-thread-count.md](2026-10-26-thread-count.md), Design decisions).

## Who and what success looks like

A person opening an existing Karate project, often for the first time.

Success:
- On first open, the environment picker, the filter chips and the thread count show what `karate run` (or `mvn test`, for the environment) would use, and Run all runs those tests that way.
- Once the person picks a value, Grace keeps it, whatever the pom says.
- Editing the pom and pressing Refresh shows the new values in every control the person has not set.

## Goals, non-goals, anti-goals

- Goals: the Requirements below.
- Non-goals: Maven profiles, parent poms, `settings.xml` and `-D` flags; Gradle (a build file is code); `scenarioName`, `dryRun` and the pom's other keys; a sign showing where a value came from; writing `karate-pom.json` or `pom.xml`; running Maven (`help:evaluate`).
- Anti-goals: a run whose settings come from a file and not from the controls; a pom value overriding a choice the person made; any cost while idle; a new dependency.

## Constraints

- Karate 2.1.4's `KaratePom` reads `env` as a string, `tags` as a list of strings (each one `-t`, so the entries are ANDed) and `threads` as a whole number.
- Grace's filter is one OR group of included tags and a list of excluded tags; `tagArgs` passes it as `-t=@a,@b` and one `-t=~@c` per excluded tag. So a pom entry `@a,@b` or `~@c` maps to it one to one; `anyOf(…)`, `not(…)`, `~@a,~@b`, a mixed `@a,~@b` or a second include entry do not.
- Environment names must match `ENV_NAME`; thread counts run from 1 to `MAX_THREADS` (32).
- Maven passes `karate.env` to the tests' JVM through Surefire's (or, for `verify`, Failsafe's) `systemPropertyVariables`. A `<karate.env>` in `<properties>` alone never reaches the tests' JVM; the issue counts it anyway, as a fallback.

### What exists today

- `readPom` (`src/main/karate/pom.ts`) reads `paths`, `configDir` and `output.junitXml`, and lists every other key as unused. `detectLayout` (`src/main/karate/command.ts`) turns that into `pomNotice`, shown on open only (project setup R1.5, R1.7).
- `Runner.open` loads the saved filter, environment and thread count (`loadProjectFilter`, `loadProjectEnv`, `loadProjectThreads` in `src/main/settings.ts`), which default to no filter, no environment and 1 when nothing is saved. `refresh` keeps the values in force.
- Saving "no environment" deletes the `env` key, so it reads the same as never chosen.
- `rediscover` drops filter tags the new tree lacks and saves the result. Saving run properties writes the environment in force too.
- Every dry run and run passes `--no-pom`, `-e` (when an environment is picked), the filter's `-t` and `-T`.

## Approaches considered

The model:
- **A. Read the pom on open and Refresh, never save its values (chosen by the owner).** A control the person has not set follows the pom, as `karate run` reads the file each time. Saved values win.
- **B. Copy the pom's values into the settings on first open.** Simple, but editing the pom later changes nothing in Grace, unlike the CLI. Rejected.

Reading `pom.xml`:
- **A. A small XML reader in Grace (chosen by the owner).** About 50 lines: elements, nesting, comments, CDATA. It knows where each element sits, so a `<profiles>` section never leaks in. No dependency.
- **B. Regular expressions over the text.** Shorter, but cannot tell a profile's `<properties>` from the project's. Rejected.
- **C. An XML library.** Robust, but a dependency and a decision record for two values. Rejected.
- **D. Ask Maven (`help:evaluate -Dexpression=karate.env`).** Evaluates `<properties>` only, not Surefire's entry; needs Maven or a wrapper, the help plugin (network on first use) and a JVM start per pom change. Rejected by the owner.

## Requirements

### R1. Starting values

1. On open and on Refresh, each of the environment, the tag filter and the thread count is the saved value when the project has one; otherwise the pom's value (R2, R3); otherwise Grace's default: no environment, no filter, 1 thread. Each control decides on its own.
2. A value is saved only when the person chooses it: picking an environment, `(none)` included; any change in the filter bar; setting the thread count; a Re-run or Re-run failed from the history, which saves its filter, and its environment when it switches to another one (as today). Saving run properties does not save the environment. Dropping the tags a new tree lacks from a filter that is not saved does not save it; the dropped tags come back on the next open or Refresh, not on an environment switch.
3. Picking `(none)` is saved as `"env": null` in the project's entry of `settings.json`, so the pom's environment no longer applies.
4. Refresh reads the pom files again and applies their values to the controls with nothing saved. A value the pom no longer gives goes back to Grace's default.
5. Runs are unchanged: every dry run and run passes `--no-pom`, and `-e`, `-t` and `-T` from the controls in force. The run record keeps those values.

### R2. `karate-pom.json`

1. `env`: a string that matches `ENV_NAME` is the starting environment. An empty string counts as absent. Any other value is unused.
2. `threads`: a whole number from 1 to 32 is the starting count. Any other value is unused.
3. `tags`: a list of strings. Each entry, in order, with spaces around a comma or at either end ignored:
   - `~@t` adds `t` to the excluded tags;
   - `@t1,@t2,…` adds `t1`, `t2`, … to the included tags, for the first such entry only;
   - every tag must pass `filterable` and must not already be in the other list, or the entry is unused;
   - any other entry is unused: a second include entry, `~@a,~@b`, `@a,~@b`, `anyOf(…)`, `not(…)`.
   A `tags` that is not a list of strings is unused whole. The used entries make the starting filter, with tests that do not match dimmed, not hidden. Dropping an entry Karate would AND can only show more tests than `karate run` runs, never fewer.
4. The open notice no longer names `env`, `tags` or `threads` when Grace uses them. An unused value is named in the same sentence, in the file's order, as the key, a space and the value as JSON: `Grace does not use these karate-pom.json settings: env "x y", tags "anyOf('@a','@b')", dryRun.` Other unused keys are named as today. A value is named even when a saved value is in force.
5. A file that cannot be read gives no starting value, and the notice says so as today (project setup R1.6).

### R3. Maven's `karate.env`

1. Only when the project folder holds a `pom.xml` and its `karate-pom.json` gives no `env` (no file, a file that cannot be read, which counts as absent as in project setup R1.6, no `env` key, or an empty string). A `karate-pom.json` `env` that is unused (R2.1) still keeps `pom.xml` out: `karate-pom.json` wins over `pom.xml`, without a notice.
2. Grace reads only the `pom.xml` in the project folder: no parent, profile, `settings.xml` or `-D`.
3. The entry is the first `karate.env` child of `<configuration><systemPropertyVariables>` found, in this order: `maven-surefire-plugin` in `<build><plugins>`, then in `<build><pluginManagement><plugins>`, then `maven-failsafe-plugin` in the same two places. Only a plugin's own `<configuration>` counts, not an `<execution>`'s. With no such entry, the entry is `<properties><karate.env>` of the project.
4. The value is the entry's text, trimmed. One `${name}` in it is replaced by the project's `<properties><name>`, trimmed. A value holding more than one `${`, naming a property that is missing, or still holding `${` after the replacement gives no environment; Grace then does not fall back to `<properties><karate.env>`.
5. A value that does not match `ENV_NAME` gives no environment, without a notice: `pom.xml` is not the Karate CLI's file.
6. A `pom.xml` that cannot be read, or whose elements are not well nested, gives no environment, without a notice.
7. A Gradle project gets nothing from its build file.

## Cost

- Idle: nothing. No timer, poll or watcher; the values are read on open and Refresh only.
- Open and Refresh: one more read of `karate-pom.json` (already read by `detectLayout`) and, for a Maven project, one read and parse of `pom.xml`, a few KB, well under a millisecond. Nothing during a run.
- Memory: none kept; the parsed `pom.xml` is dropped after the read.
- Start time and size: no change to start; about 3 KB of code, no dependency. The bench's projects have no `karate-pom.json` and no `karate.env` in a `pom.xml`, so no bench measure should move.

## Design decisions already made

- Read on open and Refresh, never save the pom's values; Refresh applies an edited pom to the controls the person has not set: the owner.
- `karate-pom.json` wins over `pom.xml`, without a notice: the owner.
- Read `pom.xml` with a small reader in Grace, not Maven or a library: the owner.
- My call: Surefire's or Failsafe's `systemPropertyVariables` entry before `<properties>`, since that is what reaches the tests' JVM; the bare property only when no entry exists.
- My call: picking `(none)` is saved, and saving run properties no longer saves the environment, so "nothing saved" means the person never chose.
- My call: values Grace cannot take from `karate-pom.json` (a bad `env`, `threads` out of range, a tag entry the filter cannot show) are named in the open notice; a bad `pom.xml` value is not.
- My call: the tag entries that can be shown are used even when others cannot; Karate ANDs them, so this stays closest to `karate run`.
- My call: Surefire and Failsafe plugin-level configuration only, both in `<plugins>` and `<pluginManagement>`; an `<execution>`'s configuration is left out.
- My call: a Re-run saves its environment only when it switches to another one, as today; re-running a run in the pom's environment leaves the environment following the pom.
- My call: a pom filter that is not saved loses the tags a new tree lacks until the next open or Refresh, like a saved filter today; an environment switch does not bring them back.
- My call: a `karate-pom.json` that cannot be read counts as absent, so `pom.xml`'s `karate.env` applies, as project setup R1.6 treats the file.
- My call: "saved" means written in `settings.json`. A choice whose save failed (the notice already says so) counts as not saved, so Refresh may replace it with the pom's value.

## Acceptance criteria

Scenarios run in Vitest (Node for main) unless marked **(e2e)**, which run in Playwright against the built app.

```gherkin
Feature: Start from the project's pom

  # R1
  Scenario: karate-pom.json starts the controls (e2e)
    Given a project whose karate-pom.json holds "env": "qa", "tags": ["@smoke", "~@slow"] and "threads": 3, with nothing saved for it
    When I open it
    Then the environment is qa
    And the filter includes smoke and excludes slow
    And the thread count is 3
    And no notice names env, tags or threads
    When I run all
    Then the run passes --no-pom, -e qa, -t=@smoke, -t=~@slow and -T 3

  Scenario: What the person saved wins
    Given a project whose karate-pom.json holds "env": "qa", "tags": ["@smoke"] and "threads": 3
    And the project has the environment dev, an empty filter and 1 thread saved
    When I open it
    Then the environment is dev, the filter is empty and the thread count is 1

  Scenario: Each control on its own
    Given a project whose karate-pom.json holds "env": "qa" and "threads": 3
    And the project has only 2 threads saved
    When I open it
    Then the environment is qa and the thread count is 2

  Scenario: Picking no environment is saved
    Given an open project whose karate-pom.json holds "env": "qa", with nothing saved
    When I pick (none)
    And I open the project again
    Then no environment is picked

  Scenario: Saving run properties does not save the pom's environment
    Given an open project whose karate-pom.json holds "env": "qa", with nothing saved
    When I save a run property
    And karate-pom.json then holds "env": "dev"
    And I open the project again
    Then the environment is dev

  Scenario: A pom filter whose tags no test has is not saved
    Given a project whose karate-pom.json holds "tags": ["~@gone"] and no test is tagged @gone
    When I open it
    Then the filter is empty
    And settings.json holds no filter for the project

  Scenario: Refresh applies an edited pom to the controls with nothing saved
    Given an open project whose karate-pom.json holds "env": "qa" and "threads": 3, with nothing saved
    And I set the thread count to 2
    When karate-pom.json then holds "env": "dev" and "threads": 4
    And I press Refresh
    Then the environment is dev and the thread count is 2

  Scenario: Refresh drops a value the pom no longer gives
    Given an open project whose karate-pom.json holds "env": "qa", with nothing saved
    When karate-pom.json then holds no env
    And I press Refresh
    Then no environment is picked

  # R2
  Scenario Outline: Tag entries the filter can show
    Given a project whose karate-pom.json holds "tags": <tags>, with nothing saved
    When I open it
    Then the filter includes <include> and excludes <exclude>
    And no notice names tags

    Examples:
      | tags                         | include | exclude |
      | ["@smoke"]                   | smoke   |         |
      | ["~@slow"]                   |         | slow    |
      | ["@a,@b"]                    | a, b    |         |
      | [" @a , @b ", "~@c", "~@d"]  | a, b    | c, d    |

  Scenario Outline: Tag entries the filter cannot show are named
    Given a project whose karate-pom.json holds "tags": <tags>, with nothing saved
    When I open it
    Then the filter includes <include> and excludes <exclude>
    And the notice says "Grace does not use these karate-pom.json settings: <named>."

    Examples:
      | tags                    | include | exclude | named                     |
      | ["anyOf('@a','@b')"]    |         |         | tags "anyOf('@a','@b')"   |
      | ["@a", "@b"]            | a       |         | tags "@b"                 |
      | ["~@a,~@b"]             |         |         | tags "~@a,~@b"            |
      | ["@a", "~@a"]           | a       |         | tags "~@a"                |
      | "@a"                    |         |         | tags "@a"                 |

  Scenario Outline: An env or thread count Grace cannot take is named
    Given a project whose karate-pom.json holds <json>, with nothing saved
    When I open it
    Then no environment is picked and the thread count is 1
    And the notice says "Grace does not use these karate-pom.json settings: <named>."

    Examples:
      | json              | named        |
      | "env": "x y"      | env "x y"    |
      | "threads": 64     | threads 64   |
      | "threads": "4"    | threads "4"  |

  Scenario: The notice names only what Grace does not use
    Given a project whose karate-pom.json holds "env": "qa", "tags": ["~@slow"], "threads": 2 and "dryRun": true
    When I open it
    Then the notice says "Grace does not use these karate-pom.json settings: dryRun."

  # R3
  Scenario: Surefire passes the project's karate.env (e2e)
    Given a Maven project with no karate-pom.json, whose pom.xml holds <karate.env>dev</karate.env> in <properties>
    And Surefire's systemPropertyVariables hold <karate.env>${karate.env}</karate.env>
    And nothing is saved for it
    When I open it
    Then the environment is dev

  Scenario Outline: Maven's karate.env
    Given a Maven project with no karate-pom.json whose pom.xml holds <pom>, with nothing saved
    When I open it
    Then the environment is <env>

    Examples:
      | pom                                                                          | env    |
      | <properties><karate.env>dev</karate.env> only                                | dev    |
      | Surefire's entry qa                                                          | qa     |
      | Surefire's entry ${karate.env} and the property dev                          | dev    |
      | Surefire's entry qa-${region} and the property region eu                     | qa-eu  |
      | Surefire's entry ${missing} and the property karate.env dev                  | (none) |
      | Failsafe's entry ${karate.env} and the property dev                          | dev    |
      | Surefire's entry qa in <pluginManagement>                                    | qa     |
      | Surefire's entry qa and Failsafe's entry uat                                 | qa     |
      | Surefire's entry qa in an <execution>'s configuration only                   | (none) |
      | <karate.env>dev</karate.env> only in a profile's <properties>                | (none) |
      | <karate.env>dev</karate.env> only in a comment                               | (none) |
      | the property karate.env "x y"                                                | (none) |
      | elements that are not well nested                                            | (none) |

  Scenario: karate-pom.json wins over pom.xml
    Given a Maven project whose karate-pom.json holds "env": "qa" and whose pom.xml's karate.env is dev
    When I open it
    Then the environment is qa

  Scenario: An unused karate-pom.json env keeps pom.xml out
    Given a Maven project whose karate-pom.json holds "env": "x y" and whose pom.xml's karate.env is dev
    When I open it
    Then no environment is picked
    And the notice says "Grace does not use these karate-pom.json settings: env "x y"."

  Scenario: A Gradle project gets nothing from its build file
    Given a Gradle project whose build.gradle sets karate.env to dev, with nothing saved
    When I open it
    Then no environment is picked
```
