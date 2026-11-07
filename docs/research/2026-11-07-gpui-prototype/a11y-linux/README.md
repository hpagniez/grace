# The GPUI prototype's accessibility on Linux, through AT-SPI (#269)

Date 2026-11-07. The same cloud Linux machine as the [prototype's measurements](../README.md) (4 vCPU, no GPU, Ubuntu 24.04, Xvfb), with `at-spi2-core` 2.52 and PyGObject's `Atspi` 2.0. This is the Linux half of #269; Narrator, Accessibility Insights and UI Automation need Windows.

On Linux, GPUI's X11 window creates an `accesskit_unix` 0.22 adapter (with `accesskit_atspi_common` 0.19.1), so the tree reaches AT-SPI the way it reaches UI Automation on Windows, through AccessKit. Gaps in the AccessKit tree will show on Windows too; gaps in the AT-SPI adapter alone (marked below) may not.

## Verdict

GPUI can be made accessible and tested through its accessibility tree, at no measurable cost, but only by hand, element by element, and with two holes on Linux that Grace cannot fix itself.

- **A real e2e test runs through the tree.** [e2e_run_all.py](e2e_run_all.py) ports e2e `run.spec.ts` "Run all with live progress": it opens two features, presses Run all, waits for "0 of 9", for "slow loop" to run, for a passed and a failed scenario, for "9 of 9", the progress value, the announced outcome "Run failed: 8 passed, 1 failed" and `basic.feature` failed. All through AT-SPI roles, names, descriptions, values and announcements; no pixels, no coordinates. It passed 3 runs out of 3, about 11 s each, with the real Karate.
- **Announcements work** with a 90-line custom element ([src/a11y.rs](../src/a11y.rs)): GPUI's `div` has no live region, so the element wraps one and sets AccessKit's `live`. A screen reader hears "Running 9 tests", then "Run failed: 8 passed, 1 failed".
- **No cost.** On the 2,000-test project, 5 rounds each, AccessKit off (no AT-SPI bus) against on with a client listening to every event (about 545 events a run): the same idle memory (169 MB), peak memory (173.1 against 173.8 MB), Run all CPU (3,780 against 3,770 ms) and time (5.1 s), all inside the rounds' spread ([cost.log](cost.log), [cost.py](cost.py)).
- **Two holes in AccessKit's AT-SPI adapter 0.19**, which GPUI 0.3.11 pins:
  - A disabled button reads as enabled. The adapter adds `enabled` and `sensitive` to every node whose role has no read-only state, disabled or not. AccessKit 0.21 fixes it, but GPUI would need a new release or a patch. GPUI Kit does not set AccessKit's disabled flag either; a disabled Kit `Button` only loses its click action and focus. The e2e test waits for Run all's click action for that reason.
  - Open or closed, and a row's level, are not mapped at all (also not in 0.21), and there is no expand action. The prototype now makes the chevron a button named "Open" or "Close" so the tree can be opened through AT-SPI.

## Two passes

**First pass** (prototype at `9569e1c`, before GPUI Kit, [tree.txt](tree.txt)): only buttons and tree-row names were in the tree. No text, no statuses, no states, no focus. Clicking Run all through AT-SPI already ran Karate.

**Second pass** (the GPUI Kit prototype at `73bcdf1`, with the accessibility props the other thread added, then this branch's changes, [tree-kit.txt](tree-kit.txt)):

| | Exposed now | Still missing |
|---|---|---|
| Toolbar | Filter (entry with placeholder), Environment (combo box), Threads (spin button with value 1 and Increment, Decrement), Properties, Open…, Cancel, Run all; Kit's buttons are focusable | The Threads spin button and the Properties and row ▶ icon buttons have an empty name (their word is only the description); spin button min and max unset. Cancel reads as enabled (adapter hole) |
| Progress | Progress bar with value and maximum; the summary as a status ("9 tests · Not run yet", "4 of 4 · 0.7 s") | The passed and failed counts are not in the summary's name |
| Announcements | "Running N tests" and "Run passed/failed/cancelled: X passed, Y failed" (this branch) | Nothing between start and end, by choice: the summary changes every 100 ms |
| Tree | Rows with their name and status ("Not run", "Running", "Passed", "Failed") as description; selectable; status changes send events; chevron as Open/Close button (this branch) | No `tree` container: rows are flat siblings of the window. Level and expanded are set but not mapped (adapter hole). Only rows on screen exist (virtual list), with no set size |
| Tags, bottom tabs | Tag chips as buttons; Console, History, HTTP as page tabs, selected state included | The console's text and the tab panels are not in the tree |
| Ids | None | No `accessibility_id`, so tests find nodes by name |

## What it means for #269

- An accessibility-tree e2e layer can replace Playwright for flows like Run all: [e2e_run_all.py](e2e_run_all.py) checks what the Playwright spec checks, in about the same time. But it can only check what someone remembered to put in the tree, and nothing fails when a role or a label is forgotten. Electron gets all of this from the DOM.
- The tests would need one driver per platform (AT-SPI here, UI Automation on Windows) unless Grace stays Windows-only, as it is today.
- Two things depend on GPUI upstream: a release on a newer AccessKit (disabled state on Linux), and AccessKit mapping expanded and level to AT-SPI.

## How to repeat it

```sh
apt-get install at-spi2-core gir1.2-atspi-2.0 libxkbcommon-x11-dev xdotool
Xvfb :99 & export DISPLAY=:99
export DBUS_SESSION_BUS_ADDRESS=$(dbus-daemon --session --fork --print-address=1 | head -1)
/usr/libexec/at-spi-bus-launcher --launch-immediately &
cargo build --release
export GRACE_JAVA=$(which java)
python3.12 a11y-linux/e2e_run_all.py target/release/grace-gpui karate-2.1.4.jar <copy of test/fixtures/projects/clean>
python3.12 a11y-linux/cost.py target/release/grace-gpui karate-2.1.4.jar <writeBigProject(dir, 5)> 5
GRACE_KARATE_JAR=karate-2.1.4.jar target/release/grace-gpui <project> &
python3.12 a11y-linux/dump.py            # the tree, with states, actions and bounds
python3.12 a11y-linux/act.py "push button" "Run all"   # press a node by role and name
python3.12 a11y-linux/listen.py 20       # AT-SPI events for 20 s
```

Use the system Python that matches `python3-gi` (3.12 on Ubuntu 24.04), and unset `JAVA_TOOL_OPTIONS` if the machine sets it.
