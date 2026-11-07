# Idle shown, minimised and hidden, on Linux (ADR 0030)

Date 2026-11-07. The prototype's cloud Linux machine (4 vCPU, no GPU, Xvfb 1920 x 1080) with the openbox 3.6 window manager, the 2,000-test project open, no run. [measure/hidden-idle.sh](../measure/hidden-idle.sh), 3 rounds, each state 10 s to settle then 60 s; raw lines in [hidden-idle-linux.log](hidden-idle-linux.log). CPU is the whole process tree's, at 10 ms resolution. Wake-ups are the voluntary context switches of all its threads, per second.

Medians [range] of 3 rounds:

| State | Grace (Electron), CPU ms / 60 s | Wake-ups /s | GPUI unpatched, CPU | Wake-ups /s | GPUI with the idle patch, CPU | Wake-ups /s |
|---|---|---|---|---|---|---|
| shown | 230 [220–230] | 6.0 [5.0–6.7] | 220 [210–240] | 66.5 | 10 [10–20] | 4.0 |
| minimised | 10 [10–20] | 1.4 [0.8–1.4] | 10 | 4.0 | 10 [0–10] | 4.0 |
| covered by a maximised window | 10 [10–20] | 1.0 | 210 [210–220] | 66.5 | 10 [10–20] | 4.0 |
| on another workspace | 0 [0–10] | 1.0 | 10 [10–20] | 4.0 | 20 [10–20] | 4.0 |

The 4 wake-ups a second left in every GPUI state are not its frame loop: GPUI's executor thread and Mesa's swapchain threads ([patches/README.md](../patches/README.md)).

Against ADR 0030 (no wake-ups while minimised or hidden, `idle.cpu.minimised` limit 125 ms; a budget while shown, `idle.cpu.shown` limit 1,453 ms):

- **Minimised and on another workspace, all three pass.** GPUI drops its X11 refresh timer when the window is unmapped, patch or not.
- **Shown, all three are within the budget.** Unpatched GPUI uses what Grace uses (220 against 230 ms a minute), but wakes 11 times as often.
- **Covered, unpatched GPUI keeps drawing** at the display rate. GPUI stops its timer on `VisibilityNotify` fully obscured, but its window never received one: `xev` on it showed no visibility event at all, where a plain X window under the same covering got `VisibilityFullyObscured`. The likely cause is its 32-bit (transparent) visual, which the X server redirects off-screen, and a redirected window gets no visibility events; this was not traced further. Electron does: Chromium computes occlusion itself. On a compositing desktop (GNOME, KDE) every window is redirected, so the same would hold there. ADR 0030 does not rule it out, since no desktop reliably tells an app it is covered, but it is the one hidden case where Grace on GPUI would cost more than Grace today without the patch or zed-industries/zed#66449.
