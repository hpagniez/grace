# Patched crates

GPUI Grace builds `gpui-pre` with two patches to its Windows platform crate, applied in the order [series](series) lists them ([ADR 0031](../docs/decisions/0031-move-grace-to-rust-and-gpui.md), the stack). The workspace's `Cargo.toml` points `[patch.crates-io]` at the patched copy here. The crate's version is the one [check.sh](check.sh) names, the same as the `gpui-pre` pin.

| Crate | Copy | Patch | Why |
|---|---|---|---|
| `gpui-pre-windows` | [gpui-pre-windows/](gpui-pre-windows/) | [gpui-pre-windows-idle.patch](gpui-pre-windows-idle.patch) | The vsync thread sleeps while every window is minimised or hidden, instead of redrawing them on every vsync ([ADR 0030](../docs/decisions/0030-no-wake-ups-while-hidden-and-an-idle-cpu-budget.md), upstream [zed#66221](https://github.com/zed-industries/zed/issues/66221)). +74 lines in 3 files; how it works and what it measured: [the prototype's patches](../docs/research/2026-11-07-gpui-prototype/patches/README.md) |
| `gpui-pre-windows` | same | [gpui-pre-windows-a11y-focus.patch](gpui-pre-windows-a11y-focus.patch) | AccessKit learns that the window is focused. The window is shown, and Windows activates it, before GPUI sets up AccessKit, so that `WM_ACTIVATE` found no adapter and was lost; AccessKit then took the window as unfocused and reported no focused control to UI Automation, even after a click or Tab. The window now keeps the last activation and hands it to AccessKit at set-up, and a `WM_ACTIVATE` that comes while AccessKit is busy is passed on at the next tree update. Tracked in [#301](https://github.com/hpagniez/grace/issues/301) until upstream has a fix |

The copy is the crate as published on crates.io, without `.cargo_vcs_info.json`, `.cargo-ok`, `Cargo.lock` and `Cargo.toml.orig`, with the patches applied. Nothing else in it changes: a fix goes in a `.patch` file first, then into the copy. CI checks that they agree: [check.sh](check.sh) downloads the published crate (Cargo never does, since the workspace patches it), checks its crates.io checksum, applies the patches of `series`, and fails on any difference with the copy.

## Upgrade

The patches follow the newest `gpui-pre` ([ADR 0032](../docs/decisions/0032-keep-the-gpui-set-rust-and-cargo-lock-current.md)). Every Tuesday `cargo-updates` runs `npm run cargo:bump`, which does it all, and opens a draft PR with `test` running on it: the Windows e2e for the focus patch, `gpui-idle` for the idle patch. By hand, `npm run cargo:bump` makes the same change in the working tree:

1. It takes the newest GPUI Kit and the `gpui-pre` that Kit pins, and moves the pins in the workspace's `Cargo.toml`.
2. It downloads that `gpui-pre-windows` from `https://static.crates.io/crates/gpui-pre-windows/gpui-pre-windows-<version>.crate`, checks it against the crates.io index's checksum, drops the four files above, and applies each patch of `series` in order.
3. It writes each `.patch` again from the copy before and after it, as `git diff` writes it, and sets `check.sh`'s version and checksum.
4. It moves `rust-toolchain.toml` to the newest stable Rust and runs `cargo update`.

When a patch does not apply, or the new release already has it, the script changes nothing here and says which; `cargo-updates` opens an issue. Then do it by hand: unpack the crate over `gpui-pre-windows/`, drop the four files, apply each patch in order (`patch -d gpui-pre-windows -p1 < <patch>`), fix the rejects, and write each patch from the copy before and after it (`git diff --no-index`). Run `cargo test -p gpui-pre-windows` (its gate test also runs on Linux), then `npm run bench:gpui` on Windows for the minimised idle.

When an upstream release has the fix, drop the patch and its line in `series`; when it has every fix, drop the copy, `series`, `check.sh` and the `[patch.crates-io]` entry.
