# Security policy

## Supported versions

Only the latest release candidate on the [releases page](https://github.com/hpagniez/grace/releases) gets fixes. Grace has no stable release yet.

## Report a vulnerability

Do not open a public issue. Use GitHub's private reporting form: [Report a vulnerability](https://github.com/hpagniez/grace/security/advisories/new) (Security tab). Only the maintainer sees the report, and the discussion stays private until a fix is out.

Say which Grace version you used, how to reproduce the problem and what an attacker could do with it. A proof of concept helps.

## In scope

- The Grace app: the Electron main process, the preload API and the window.
- How Grace stores secret run properties (encrypted with the Windows account) and passes them to Java.
- How Grace starts the bundled Java and Karate.
- The release zip and the GitHub workflows that build it.

## Out of scope

Bugs in Karate, the Eclipse Temurin Java runtime, Electron or Chromium themselves. Report them to [Karate](https://github.com/karatelabs/karate/security), [Adoptium](https://adoptium.net/support/) or [Electron](https://github.com/electron/electron/security). Grace ships their fixes by updating the bundled versions.

The zip is not code-signed yet, so Windows SmartScreen warns on first start. That is known.
