# Patched crates

GPUI Grace builds `gpui-pre` 0.3.11 with one patch to its Windows platform crate ([ADR 0031](../docs/decisions/0031-move-grace-to-rust-and-gpui.md), the stack). The workspace's `Cargo.toml` points `[patch.crates-io]` at the patched copy here.

| Crate | Copy | Patch | Why |
|---|---|---|---|
| `gpui-pre-windows` 0.3.11 | [gpui-pre-windows/](gpui-pre-windows/) | [gpui-pre-windows-0.3.11-idle.patch](gpui-pre-windows-0.3.11-idle.patch) | The vsync thread sleeps while every window is minimised or hidden, instead of redrawing them on every vsync ([ADR 0030](../docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md), upstream [zed#66221](https://github.com/zed-industries/zed/issues/66221)). +74 lines in 3 files; how it works and what it measured: [the prototype's patches](../docs/research/2026-11-07-gpui-prototype/patches/README.md) |

The copy is the crate as published on crates.io, without `.cargo-ok`, `Cargo.lock` and `Cargo.toml.orig`, with the patch applied. Nothing else in it changes: a fix goes in the `.patch` file first, then into the copy. CI checks that they agree: [check.sh](check.sh) rebuilds the copy from Cargo's registry and the patch, and fails on any difference.

## Rebase on a GPUI upgrade

GPUI and GPUI Kit are upgraded as one set, between migration phases only (migration guide R9.4):

1. Bump the pins in the workspace's `Cargo.toml` and run `cargo fetch`.
2. Copy the new `gpui-pre-windows` from `~/.cargo/registry/src/*/` over `gpui-pre-windows/`, drop the three files above, and apply the patch (`patch -d gpui-pre-windows -p1 < <patch>`); fix the rejects by hand.
3. Write the new `.patch` from the clean crate and the copy (`diff -ruN`), named after the new version, and update this table and `check.sh`.
4. Run `cargo test -p gpui-pre-windows` (its gate test also runs on Linux), then the bench's minimised idle on Windows.

When an upstream release has the fix, drop the copy, the patch and the `[patch.crates-io]` entry.
