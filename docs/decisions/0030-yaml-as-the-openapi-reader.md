---
status: accepted
date: 2026-11-04
decision-makers: Tusquito
---

# Read OpenAPI specs with `yaml` and Grace's own resolver

## Context and Problem Statement

[#69](https://github.com/hpagniez/grace/issues/69) generates Karate features from an OpenAPI 3.0, 3.1 or 3.2 spec, in JSON or YAML, split over files and URLs, and refuses an invalid one with its file, line and reason ([spec](../specs/2026-11-04-openapi-import.md)). Grace has no YAML reader and no `$ref` resolver. A dependency is hard to remove once features rely on it, and Grace runs lightly first ([ADR 0023](0023-run-lightly-as-a-release-requirement.md)).

Which library reads the spec, and who resolves and checks it?

## Considered Options

1. `yaml` 2.9.1 alone, with Grace's own `$ref` resolver and checks.
2. `@apidevtools/swagger-parser` 13.1.0 (plus `yaml` for lines).
3. `@redocly/openapi-core` 2.73.0.
4. `@scalar/openapi-parser` 0.29.19.

Sizes from `npm view` on 2026-11-04; option 1 bundled and minified with esbuild.

## Decision Outcome

Chosen: option 1, by the owner.

- `yaml` 2.9.1 (ISC, no dependencies): 686 KB unpacked, about 115 KB minified and 36 KB gzipped once bundled. It reads YAML 1.2 and keeps every node's position, so each error gets its line, JSON's included (Grace checks JSON strictly with `JSON.parse` first, then reads its positions with `yaml`). No custom tags: it builds plain data only.
- Pinned exact in `dependencies`, as every runtime package is.
- Loaded by a dynamic `import()` into a separate chunk of the main bundle: nothing loads at start or while idle.
- Grace's own resolver (`fs` for files, `net.fetch` for `http(s)` with its own redirect checks) and checks of the parts it reads and generates from, about 20 KB of code.

Why not the others:

- Option 2 validates every rule against the official schemas, but brings `ajv`, `ajv-draft-04`, `json-schema-ref-parser` and four more packages, several hundred KB. Its errors name a JSON path, not a line, so `yaml` would still be needed for the line. OpenAPI 3.2 support is not confirmed. Its resolver reads local files from any spec unless configured, which Grace would have to lock down anyway.
- Option 3 reports lines but has 13 dependencies (`graphql`, `ajv`, `js-yaml`, `yaml-ast-parser` among them), 2 MB unpacked before them.
- Option 4 is 1.9 MB unpacked with six `@scalar` packages on top of `yaml`, for validation and upgrading Grace does not need.

### Consequences

- Good: one small dependency with none of its own; errors with lines in both formats; nothing loads unless the person imports.
- Good: Grace decides what a remote spec may reach (no local files, `http(s)` only) in its own code.
- Bad: Grace does not check every rule of the OpenAPI standard, only what it reads; a spec that breaks a rule Grace does not read imports anyway.
- Bad: Grace owns a `$ref` resolver and its tests.
