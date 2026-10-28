# Grace

<img src="assets/mascot.svg" alt="Grace, a martial artist in a white gi and green belt, punching a moth with a strip of tape on its wing" width="320">

A free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2. Named after Grace Hopper, whose team taped a moth into a logbook in 1947 as the "first actual case of bug being found".

The walking skeleton (open a project, browse the test tree, run, watch progress, read failures, open the report) works from source (`npm start`) and under the end-to-end suite (`npm run e2e`). Version 1.0.0 adds the tag filter and the environment switcher. Its packaged Windows zip was checked by hand on 2026-10-25: see [docs/checks/2026-10-25-release-1.0.0-packaged.md](docs/checks/2026-10-25-release-1.0.0-packaged.md). See [docs/features.md](docs/features.md) for what it does and what is planned, and [AGENTS.md](AGENTS.md) for how the repo is organised.

## Run from source

Windows 11 and Node 24.

```
npm install
npm run vendor
npm start
```

`npm run vendor` downloads the bundled Java runtime and Karate jar once. `npm run make` builds the Windows zip.

## Licence

Grace is under the [MIT licence](LICENSE).

Grace is not affiliated with or endorsed by Karate Labs Inc. Karate is a trademark of Karate Labs Inc.

## Bundled software

The Windows zip bundles programs that are not part of Grace, each under its own licence:

- [Electron](https://www.electronjs.org/), with Chromium and Node.js. The `LICENSE` and `LICENSES.chromium.html` files next to `Grace.exe` are Electron's.
- The [Eclipse Temurin](https://adoptium.net/) Java runtime, under the GPLv2 with the Classpath Exception; licence texts in `resources/licenses/temurin`. Its source is the repository and commit named by `SOURCE_REPO` and `SOURCE` in `resources/jre/release`.
- [Karate](https://github.com/karatelabs/karate), under the MIT licence in `resources/licenses/karate-LICENSE.txt`. The libraries inside `karate.jar` keep their own licences, in the jar's `META-INF`.
