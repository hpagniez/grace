#!/bin/sh
# Fails unless patches/gpui-pre-windows is the crates.io crate with the idle patch applied, and nothing else
# (patches/README.md). Run from anywhere; needs cargo, patch and diff.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
crate=gpui-pre-windows
version=0.3.11
(cd "$here/.." && cargo fetch --quiet --target x86_64-pc-windows-msvc)
src=$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/"$crate-$version" | head -n 1)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp -R "$src" "$work/$crate"
rm -f "$work/$crate/.cargo-ok" "$work/$crate/Cargo.lock" "$work/$crate/Cargo.toml.orig"
patch -d "$work/$crate" -p1 --quiet --no-backup-if-mismatch < "$here/$crate-$version-idle.patch"
if diff -r "$work/$crate" "$here/$crate"; then
  echo "$crate: the copy is crates.io $version with the idle patch"
else
  echo "$crate: the copy differs from crates.io $version with the idle patch (patches/README.md)" >&2
  exit 1
fi
