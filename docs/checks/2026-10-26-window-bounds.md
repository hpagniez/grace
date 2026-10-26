# App check: window bounds (F41)

Date: 2026-10-26
Branch: `feature/window-bounds` at d09724e
Machine: Windows 11 Pro 10.0.26300 (x64), Node 24.15.0, bundled Temurin 21.0.12.1 and Karate 2.1.4.
Spec: [Window bounds](../specs/2026-10-26-window-bounds.md).

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm test` | 554/554 passed, including the new `src/main/window.test.ts` (12) and 5 new window-state tests in `src/main/settings.test.ts` |
| `npm run e2e` | 86/86 passed, including the new `e2e/window.spec.ts` (12). Run with the mouse cursor off the area where test windows open: a real cursor there ends the panel splitter's synthetic drags in `e2e/panel.spec.ts` (seen on this machine, not caused by F41) |
| `npm run typecheck`, `npm run lint` | passed |

Not covered by the automated part: a second screen (the machine running the tests has one), unplugging a screen, Windows Snap, how the start looks, and a second launch while the first window is still hidden.

## By hand (done by a person)

Build with `npm run package`, then start `out\karate-kid-win32-x64\karate-kid.exe`. Each step closes the app with its X button unless it says otherwise. Write the result under each step: passed, or what you saw instead.

### A. Two screens

1. Move the window to the second screen, resize it, close the app and start it again. It opens on the second screen at the same place and size, with no white or wrong-theme frame.
   - Result: passed (checked by Tusquito, 2026-10-26).
2. Maximize it on the second screen, close the app and start it again. It opens maximized on the second screen. Unmaximize it: it goes back to the size and place it had before you maximized it.
   - Result: passed (checked by Tusquito, 2026-10-26).
3. Maximize the window on the main screen, move it to the second screen with Win+Shift+Right, close the app and start it again. It opens maximized on the second screen.
   - Result: passed (checked by Tusquito, 2026-10-26).

### B. The second screen is gone

1. Close the app maximized on the second screen. Disconnect the second screen (or set Windows to "Show only on 1") and start the app. It opens maximized on the main screen.
   - Result: passed (checked by Tusquito, 2026-10-26).
2. Reconnect the screen, move the window there not maximized, close the app, disconnect the screen again and start the app. It opens 1200×800, centred on the main screen, fully visible.
   - Result: passed (checked by Tusquito, 2026-10-26).

### C. Partly off the screen

1. Drag the window so half of it sticks out past the second screen's outer edge (the edge with no other screen beyond it). Close the app and start it again. It opens fully on the second screen, the same size.
   - Result: passed (checked by Tusquito, 2026-10-26).

### D. Snapped

1. Snap the window to the left half of a screen with Win+Left. Close the app and start it again. It opens at the size and place it had while snapped.
   - Result: passed (checked by Tusquito, 2026-10-26).

### E. A second launch during start-up

1. Maximize the window and close the app. Start it, and start it again before the window shows (double-click the exe twice quickly). One window appears, maximized, and comes to the front.
   - Result: passed (checked by Tusquito, 2026-10-26).
