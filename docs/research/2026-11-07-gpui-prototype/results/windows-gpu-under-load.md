### Windows: GPUI prototype against Grace, gpu-under-load

Under load: not a quiet machine. The owner was to watch a video stream on the main screen, and other work ran on the PC during the run. Runbook step 2 once, no retry.

Date 2026-11-08T19:04:22.775Z. Windows 11 Pro, AMD Ryzen 7 5800X 8-Core Processor (16 threads), 32 GB. Rounds after the warm-up: Grace 5, GPUI 5; settle 20 s, idle 60 s. Medians, the range of the rounds in brackets.

Grace commit 1bcfdcd. Display adapters: NVIDIA GeForce RTX 4060 Ti (119 Hz, driver 32.0.16.2114). GPUI window: moved to the middle of \\.\DISPLAY2, the screen Grace uses.

| Measure | Unit | Grace (Electron) | GPUI prototype | GPUI / Grace | Grace's limit |
|---|---|---|---|---|---|
| First paint, profile reused | ms | 497 [470–527] | 325 [322–337] | 0.65x | 612 |
| Tree shown (2,000 tests), profile reused | ms | 1,706 [1,586–1,798] | 1,558 [1,465–1,573] | 0.91x | 2,389 |
| First paint, fresh profile | ms | 571 [550–581] | 327 [321–356] | 0.57x | 598 |
| Tree shown (2,000 tests), fresh profile | ms | 1,588 [1,537–1,657] | 1,573 [1,555–1,662] | 0.99x | 1,963 |
| Idle memory: private working set, whole tree | MB | 166.8 [166.2–167.3] | 27.6 [27.6–28.1] | 0.17x | 163.3 |
| Idle processes |  | 4 [4–4] | 1 [1–1] | 0.25x |  |
| Idle CPU, window shown | ms | 172 [47–312] | 422 [266–766] | 2.45x | 1,453 |
| Idle CPU, window minimised | ms | 47 [15–78] | 891 [406–1,000] | 18.96x | 125 |
| Run all: peak memory, app | MB | 248.9 [244.9–252.8] | 34 [33.9–34.1] | 0.14x | 322.3 |
| Run all: peak memory, Java | MB | 232.2 [160.8–240.3] | 245.5 [177.7–330.7] | 1.06x | 249.9 |
| Run all: CPU, app | ms | 3,609 [3,219–4,484] | 344 [234–453] | 0.10x | 5,375 |
| Run all: CPU, Java | ms | 10,594 [10,391–11,141] | 8,813 [8,250–9,750] | 0.83x | 14,875 |
| Run all: wall time | ms | 5,843 [5,772–5,929] | 4,118 [3,993–4,387] | 0.70x |  |
| Run all: memory 3 s after the end, app | MB | 249.5 [243.9–251.5] | 29.5 [29.2–29.8] | 0.12x |  |
| Run all (10,000 tests): peak memory, app | MB | 670.7 [669.4–676.9] | 40.3 [40.3–41.5] | 0.06x | 606.5 |
| Run all (10,000 tests): peak memory, Java | MB | 258.3 [256.8–260.1] | 270 [250.3–290.6] | 1.05x | 281.3 |
| Run all (10,000 tests): CPU, app | ms | 14,188 [13,313–15,266] | 1,250 [1,078–1,531] | 0.09x | 17,671 |
| Run all (10,000 tests): CPU, Java | ms | 29,906 [29,063–30,766] | 23,891 [22,859–27,469] | 0.80x | 45,702 |
| Run all (10,000 tests): wall time | ms | 17,809 [17,221–17,999] | 12,310 [11,612–14,102] | 0.69x |  |
| Run all (10,000 tests): memory 3 s after the end, app | MB | 673.2 [672.5–678.1] | 36.1 [36–36.1] | 0.05x |  |
| Wheel over the tree, idle: frames drawn |  | 170 [168–173] | 116 [115–116] | 0.68x |  |
| Wheel over the tree, idle: longest frame | ms | 8.6 [8.5–8.7] | 9.5 [9.2–9.6] | 1.10x |  |
| Wheel over the tree, idle: 95th percentile frame | ms | 8.4 [8.4–8.4] | 8.8 [8.6–8.9] | 1.05x |  |
| Wheel over the tree during Run all: frames drawn |  | 41 [38–46] | 116 [115–116] | 2.83x |  |
| Wheel over the tree during Run all: longest frame | ms | 8.4 [8.4–8.5] | 9.7 [9.4–9.7] | 1.15x |  |
| Wheel over the tree during Run all: 95th percentile frame | ms | 8.4 [8.4–8.4] | 9.1 [8.9–9.1] | 1.08x |  |
| Size: the app's exe | MB | 249.8 | 27.3 | 0.11x |  |
| Size: unpacked folder, Java included | MB | 488.5 | 191.6 | 0.39x | 489.1 |
| Size: unpacked, without Java | MB | 324.2 | 27.3 | 0.08x |  |
| Size: zip (GPUI: deflate of the exe, an estimate) | MB | n/a | 10.1 | n/a | 207.2 |

Each stack's empty 1400 x 900 window, idle:

| Empty window | Electron | GPUI |
|---|---|---|
| Idle memory (private working set), MB | 69.9 [69.7–71] | 16.2 [15.9–16.3] |
| Idle CPU, ms | 875 [62–1,203] | 625 [578–657] |
| Processes | 4 [4–4] | 1 [1–1] |
