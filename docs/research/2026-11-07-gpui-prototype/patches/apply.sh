#!/bin/sh
# Copies gpui-pre-linux and gpui-pre-windows 0.3.11 from Cargo's registry into patches/vendor/
# and applies the idle patches (#268). Then add to ../Cargo.toml:
#
#   [patch.crates-io]
#   gpui-pre-linux = { path = "patches/vendor/gpui-pre-linux" }
#   gpui-pre-windows = { path = "patches/vendor/gpui-pre-windows" }
set -eu
here=$(cd "$(dirname "$0")" && pwd)
(cd "$here/.." && cargo fetch --quiet)
registry="${CARGO_HOME:-$HOME/.cargo}/registry/src"
for crate in gpui-pre-linux gpui-pre-windows; do
  src=$(ls -d "$registry"/*/"$crate-0.3.11" | head -n 1)
  rm -rf "$here/vendor/$crate"
  mkdir -p "$here/vendor"
  cp -R "$src" "$here/vendor/$crate"
  patch -d "$here/vendor/$crate" -p1 --quiet < "$here/$crate-0.3.11-idle.patch"
  echo "patched $here/vendor/$crate"
done
