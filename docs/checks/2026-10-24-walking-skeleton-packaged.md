# Packaged app check: walking skeleton

Date: 2026-10-24
Machine: Windows 11 Professionnel 10.0.26300 (x64), Node 24.15, no Java on PATH (`where java` finds nothing).
Spec: [walking skeleton](../specs/2026-10-24-walking-skeleton.md), "How they are checked".

Playwright cannot drive the packaged exe (ADR 0004), so this check covers what only packaging can break.

## Automated part (run by an agent)

| Check | Result |
|---|---|
| `npm run make` | Passed, exit 0 |
| Zip | `out/make/zip/win32/x64/karate-kid-win32-x64-1.0.0.zip`, 218,524,876 bytes (208.4 MB), 570 entries |
| Zip contains `resources/jre/bin/java.exe` | Yes |
| Zip contains `resources/karate.jar` | Yes |
| Zip contains `resources/licenses/` | Yes (`karate-LICENSE.txt`, `temurin/`) |
| Unzipped to a temp folder | Yes; `resources/` holds `app.asar`, `jre/`, `karate.jar`, `licenses/` |
| `where java` | Finds nothing |
| Start `karate-kid.exe` from the unzipped folder (real user data) | Process alive after 10 s. The saved last project was a deleted temp folder, so no project opened and no `java.exe` appeared (expected). |
| Start again with `KARATE_KID_USER_DATA` pointing at a temp profile whose last project is a copy of the clean fixture | Process alive after 10 s. A `java.exe` from `<unzipped>\resources\jre\bin\java.exe` appeared (the discovery dry run). It was the only java path seen. |
| Close the window (`CloseMainWindow`) | No `karate-kid` and no `java.exe` process left |
| Cleanup | Unzipped folder, temp project and temp profile deleted |

Not covered by the automated part: what the window shows. Nobody looked at it.

## Pending — needs a person

On a Windows 11 machine without Java (`where java` finds nothing), unzip the zip and start `karate-kid.exe`. Then:

1. Open the clean fixture (`test/fixtures/projects/clean`, copied somewhere writable). The tree shows `tests` with `basic.feature`, `outline.feature`, `slow.feature`; the toolbar shows the path and "Standalone".
   - Result:
2. Click Run all. The run reaches `Finished` with 8 passed and 1 failed.
   - Result:
3. Run `slow.feature`, then click Cancel. The run is marked cancelled and Task Manager shows no `java.exe` left.
   - Result:
4. After a finished run, click Report. The HTML report opens in the default browser.
   - Result:
