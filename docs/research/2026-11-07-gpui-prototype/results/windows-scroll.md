# Windows: why the GPUI prototype's wheel frames read slow

2026-11-08, Ryzen 7 5800X, RTX 4060 Ti, both screens 2560 x 1440 at 120 Hz, window on the second screen, 2,000-test project, release build of the prototype at 339d2c1 plus temporary `Instant` tracing. Single launches, so these are one sample of each pass. All times in ms.

**Cause: the probe, not GPUI.** GPUI draws on demand, and the probe logged the gap between frames that laid out the tree. So it read the gap between wheel notches, plus the wait before the first notch and the run's 100 ms redraws after the last one. Grace's probe counts `requestAnimationFrame`, which fires on every vsync whatever is drawn. The two columns did not measure the same thing.

The quick run's figures (48.7 ms idle and 124 ms during Run all, against 8.5 and 11.8 for Grace) do not show slow rendering.

## Method

Temporary trace points: `W` a wheel event (capture phase), `R` start of `render`, `L` the tree's row processor, `P` end of paint (a last-child canvas). Passes: `stock` is `window-tools.ps1` as it was (60 notches, `Start-Sleep -Milliseconds 16`), `16 ms` is the same 60 notches paced by a compiled spin loop on a 1 ms timer, `4 ms` is 240 notches, faster than the screen's 8.3 ms.

## Numbers

| Pass | Wheel gap p50 / p95 / max | Old probe's gap p50 / p95 / max | Frame gap while the wheel turns p50 / p95 / max | render to paint end p50 / p95 / max | wheel to paint end p50 / p95 / max |
|---|---|---|---|---|---|
| idle, stock | 24.9 / 34 / 40.3 | 23.9 / 40.6 / 114.4 | n/a | 4.2 / 4.9 / 6.5 | 4.9 / 12.4 / 14.1 |
| idle, 16 ms | 16 / 16.7 / 17.8 | 15.5 / 16.2 / 106.6 | n/a | 3.7 / 4.5 / 4.9 | 7.6 / 11.2 / 11.6 |
| idle, 4 ms | 4.1 / 7.8 / 9.8 | 7.5 / 8 / 8.6 | 8.3 / 8.6 / 9.1 | 3.7 / 4.5 / 5.6 | 7.3 / 8.7 / 9.7 |
| Run all, stock | 25 / 40.6 / 41.9 | 23.9 / 74.3 / 107.8 | n/a | 4.2 / 5.3 / 5.5 | 4.5 / 12.2 / 12.3 |
| Run all, 16 ms | 16 / 16.5 / 19.5 | 15.7 / 57 / 107.4 | n/a | 4.3 / 5.1 / 5.7 | 7.7 / 12.1 / 13.1 |
| Run all, 4 ms | 4.6 / 8.3 / 8.9 | 7.4 / 9 / 116 | 8.3 / 9.2 / 9.6 | 4.1 / 5 / 6.4 | 7.1 / 8.9 / 10.1 |

What they show:

- The stock wheel sent a notch every 25 to 30 ms, not 16: `Start-Sleep` in a PowerShell loop overshoots. The old probe's median follows the wheel's gap in every pass.
- With the wheel faster than the screen, GPUI draws at vsync: 8.3 ms a frame, p95 8.6 idle and 9.2 during Run all.
- The gaps over 100 ms had no wheel event in them: the wait before the first notch (`W` at the end of the gap), or the run's redraw timer after the last notch (`A`, a batch of run events, at the end of the gap, 100 ms apart by the prototype's own batching).
- CPU work per frame up to the end of paint is 4 to 6 ms, 2 to 3 draws of the row processor per frame included. A batch of run events applies in 0.2 ms.

## The probe's fix

- `window-tools.ps1` sends 240 notches, one every 4 ms, from compiled code on a 1 ms timer. Every frame has work, so a frame gap is a frame time for both apps.
- `src/main.rs` counts a frame only if a wheel event came since the last one counted.

Check of the fixed probe, one launch: idle p50 8.3, p95 8.8, longest 9.4 (116 frames); during Run all p50 8.3, p95 9, longest 10.4 (116 frames).

## Not checked

- Grace was not launched for this, so it has no figure with the new wheel; the next run of `build-and-measure.ps1` gives one.
- The trace stops at the end of paint: it does not time `Present` or the GPU. Holding 8.3 ms a frame at 120 Hz shows they fit in the frame; it says nothing about how close to the limit they are.
- GPUI's Windows backend (`DwmFlush` pacing, `Present(0)` on a flip-sequential swap chain) needed no change, so no patched build was compared.
