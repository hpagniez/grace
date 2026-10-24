# karate-kid

A free, open-source desktop test-suite runner for [Karate](https://docs.karatelabs.io/) v2.

Early stage: the walking skeleton works (open a project, browse the test tree, run, watch progress, read failures, open the report). See [docs/features.md](docs/features.md) for what it does and what is planned, and [AGENTS.md](AGENTS.md) for how the repo is organised.

## Run from source

Windows 11 and Node 24.

```
npm install
npm run vendor
npm start
```

`npm run vendor` downloads the bundled Java runtime and Karate jar once. `npm run make` builds the Windows zip.
