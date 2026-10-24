# karate-kid

A free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2.

Early stage: the walking skeleton (open a project, browse the test tree, run, watch progress, read failures, open the report) works from source (`npm start`) and under the end-to-end suite (`npm run e2e`). The packaged Windows zip still needs a person to finish its manual check: see [docs/checks/2026-10-24-walking-skeleton-packaged.md](docs/checks/2026-10-24-walking-skeleton-packaged.md). See [docs/features.md](docs/features.md) for what it does and what is planned, and [AGENTS.md](AGENTS.md) for how the repo is organised.

## Run from source

Windows 11 and Node 24.

```
npm install
npm run vendor
npm start
```

`npm run vendor` downloads the bundled Java runtime and Karate jar once. `npm run make` builds the Windows zip.
