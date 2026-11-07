# The GPUI prototype's accessibility tree on Linux, read through AT-SPI (#269)

Date 2026-11-07. The same cloud Linux machine as the [prototype's measurements](../README.md) (4 vCPU, no GPU, Ubuntu 24.04, Xvfb), with `at-spi2-core` 2.52 and PyGObject's `Atspi` 2.0. The prototype is the one at `9569e1c` on PR #266: `gpui-pre` 0.3.11, built with `cargo build --release`, run on the `tags` fixture project and on a 400-scenario project. This is the Linux half of #269 only; Narrator, Accessibility Insights and UI Automation need Windows.

On Linux, GPUI's X11 window creates an `accesskit_unix` 0.22 adapter, so the tree reaches AT-SPI the way it reaches UI Automation on Windows, through AccessKit. What is missing below is missing from the AccessKit tree, so Windows will very likely see the same gaps. That is an inference: it has to be checked on Windows.

## Verdict

The bridge works: an AT-SPI client can find the prototype's buttons and tree rows by role and name and press them, and **Run all pressed through AT-SPI ran the real Karate to the end**. But the tree holds only what was set by hand, and the prototype set very little. A screen reader would hear a list of names and buttons with no state, and an e2e test could press things but not check results. Today's 30 Playwright specs check text, statuses and counts on screen; none of that is in the tree.

Every gap but two can be closed in the app with GPUI's own API. The two that cannot (disabled state and live regions) need a custom `Element` that writes the AccessKit node itself. So it is work per element, not a dead end, and an e2e layer through the tree would be only as good as that work.

## What is exposed

The tree of the `tags` project, opened ([tree.txt](tree.txt)):

```
[application] 'grace-gpui'
  [frame] ''
    [embedded] 'Grace'
      [push button] '−'          actions=click
      [push button] '+'          actions=click
      [push button] 'Open…'      actions=click
      [push button] 'Cancel'     actions=click
      [push button] 'Run all'    actions=click
      [tree item] 'proj'         actions=click
        [push button] 'Run'
      [tree item] 'tests'        actions=click
        [push button] 'Run'
      [tree item] 'tags.feature' actions=click
        [push button] 'Run'
```

- Every element the prototype gave a role and a label: the five toolbar buttons, each tree row and each row's ▶ button. Each has a `click` action and its bounds.
- AT-SPI actions work. `click` on a tree item selected it (the details pane showed it); `click` on Run all started a run that passed 4 of 4 ([act.py](act.py)).
- Structure changes are announced. Opening a node with the mouse sent `object:children-changed:add` for each new row ([listen.py](listen.py)).
- Turning accessibility on costs nothing measurable: idle CPU was 150 ms per 30 s and memory 143 MB, before and after a client read the whole tree.

## What is not exposed

| Gap | What a user or a test loses | Fix in GPUI 0.3.11 |
|---|---|---|
| No text at all: the title, the "4 tests · Not run yet" line, the progress, the details pane, the console, the status bar | A screen reader cannot read any result; a test cannot assert one | Wrap text in `text!` (a `Label` node), or give its container a role and a label |
| Tree rows have no status ("passed", "failed"), no title, no tags | The core of Grace, the result of each test, is invisible | `aria_description` or a label built from name + status |
| Tree rows are flat siblings of the window: no `tree` container, no level, no position, not nested | No "level 2, 3 of 12"; no tree navigation | A `Role::Tree` container, `aria_level`, `aria_position_in_set`, `aria_size_of_set` |
| No expanded state, and the chevron has no role | Folders cannot be opened or closed through the tree | `aria_expanded`, and a role on the chevron (or an expand action on the row) |
| No selected state, though the click selects | The selection is not announced | `aria_selected` |
| Disabled buttons look disabled (45 % opacity) but say `enabled`: Cancel while nothing runs, Run all during a run | A screen reader offers a button that does nothing | None in `div`: GPUI has no `aria_disabled`. Needs a custom `Element` writing `set_disabled` |
| Threads stepper is "−" and "+", and its value (1) is not exposed | Meaningless names | `aria_label("Fewer threads")`, or a `SpinButton` with `aria_numeric_value` |
| No focus: nothing is focusable, Tab does nothing, the frame has no name | No keyboard use at all apart from the three shortcuts | `track_focus` on each control, plus focus handling. GPUI has the plumbing; the prototype has none |
| No live region for progress and the end of a run | "Run finished, 4 passed" is never announced | None in `div`. Needs a custom `Element` writing `set_live` |
| No stable ids | A test has to find things by name | `accessibility_id(...)`, mapped to AT-SPI `AccessibleId` and UIA `AutomationId` |
| The tree is virtual: on a project of 200 features (400 scenarios), only the 25 rows on screen were in the tree | A test or a screen reader sees only what is scrolled into view, with no set size to say there are more | Set `aria_size_of_set` and positions; a test scrolls, as Playwright does today with the virtual list |

Every row during a run kept its role and name, so no events fired as statuses changed: with nothing but names in the tree, there is nothing to update.

## What it means for #269

- **e2e through the tree** (AT-SPI here, UI Automation on Windows) can drive the app: find by role and name, press, wait for children. It cannot check anything until results are in the tree. Porting `run.spec.ts` would need, at least, statuses on rows, the summary line as text, and stable ids.
- **Electron gives all of this from the DOM.** With GPUI, each of the gaps above is code Grace would write and keep, on every screen, and a test that checks it, because nothing fails when a role or label is forgotten.
- **The Windows half still decides** how Narrator reads this tree and whether UI Automation sees the same nodes.

## How to repeat it

```sh
apt-get install at-spi2-core gir1.2-atspi-2.0 libxkbcommon-x11-dev xdotool
Xvfb :99 & export DISPLAY=:99
eval "export DBUS_SESSION_BUS_ADDRESS=$(dbus-daemon --session --fork --print-address=1 | head -1)"
/usr/libexec/at-spi-bus-launcher --launch-immediately &
GRACE_JAVA=$(which java) GRACE_KARATE_JAR=karate-2.1.4.jar target/release/grace-gpui <project> &
python3.12 dump.py                       # the tree, with states, actions and bounds
python3.12 act.py "push button" "Run all"  # press a node by role and name
python3.12 listen.py 20                  # AT-SPI events for 20 s
```

Use the system Python that matches `python3-gi` (3.12 on Ubuntu 24.04).
