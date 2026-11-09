# Patched crates

GPUI Grace builds `gpui-pre` 0.3.11 with two patches to its Windows platform crate, applied in the order of the table ([ADR 0031](../docs/decisions/0031-move-grace-to-rust-and-gpui.md), the stack). The workspace's `Cargo.toml` points `[patch.crates-io]` at the patched copy here.

| Crate | Copy | Patch | Why |
|---|---|---|---|
| `gpui-pre-windows` 0.3.11 | [gpui-pre-windows/](gpui-pre-windows/) | [gpui-pre-windows-0.3.11-idle.patch](gpui-pre-windows-0.3.11-idle.patch) | The vsync thread sleeps while every window is minimised or hidden, instead of redrawing them on every vsync ([ADR 0030](../docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md), upstream [zed#66221](https://github.com/zed-industries/zed/issues/66221)). +74 lines in 3 files; how it works and what it measured: [the prototype's patches](../docs/research/2026-11-07-gpui-prototype/patches/README.md) |
| `gpui-pre-windows` 0.3.11 | same | [gpui-pre-windows-0.3.11-a11y-focus.patch](gpui-pre-windows-0.3.11-a11y-focus.patch) | AccessKit learns that the window is focused. The window is shown, and Windows activates it, before GPUI sets up AccessKit, so that `WM_ACTIVATE` found no adapter and was lost; AccessKit then took the window as unfocused and reported no focused control to UI Automation, even after a click or Tab . The window now keeps the last activation and hands it to AccessKit at set-up, and a `WM_ACTIVATE` that comes while AccessKit is busy is passed on at the next tree update. Tracked in [#301](https://github.com/hpagniez/grace/issues/301) until upstream has a fix |

The copy is the crate as published on crates.io, without `.cargo_vcs_info.json`, `.cargo-ok`, `Cargo.lock` and `Cargo.toml.orig`, with the patches applied. Nothing else in it changes: a fix goes in a `.patch` file first, then into the copy. CI checks that they agree: [check.sh](check.sh) downloads the published crate (Cargo never does, since the workspace patches it), checks its crates.io checksum, applies the patches, and fails on any difference with the copy.

## Rebase on a GPUI upgrade

GPUI and GPUI Kit are upgraded as one set, between migration phases only (migration guide R9.4):

1. Bump the pins in the workspace's `Cargo.toml`.
2. Download the new `gpui-pre-windows` from `https://static.crates.io/crates/gpui-pre-windows/gpui-pre-windows-<version>.crate`, unpack it over `gpui-pre-windows/`, drop the four files above, and apply each patch in order (`patch -d gpui-pre-windows -p1 < <patch>`); fix the rejects by hand.
3. Write each new `.patch` from the copy before and after it (`git diff --no-index`), named after the new version, and update this table and `check.sh` (version and the crate's checksum from the crates.io index).
4. Run `cargo test -p gpui-pre-windows` (its gate test also runs on Linux), then the bench's minimised idle on Windows.

When an upstream release has the fix, drop the copy, the patch and the `[patch.crates-io]` entry.
