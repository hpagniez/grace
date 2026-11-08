#!/bin/sh
# Fails unless patches/gpui-pre-windows is the crates.io crate with the idle patch applied, and nothing else
# (patches/README.md). The workspace patches the crate, so Cargo never downloads the original: this script does,
# and checks it against crates.io's checksum. Run from anywhere; needs curl, sha256sum, tar, patch and diff.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
crate=gpui-pre-windows
version=0.3.11
# the checksum of the published .crate, as the crates.io index and the prototype's Cargo.lock give it
sha256=5141a3db59bab761a16ab19529fe4117a86e91922e8fc00af6dfc4829cb1744d
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -sSfL -o "$work/$crate.crate" "https://static.crates.io/crates/$crate/$crate-$version.crate"
echo "$sha256  $work/$crate.crate" | sha256sum -c --quiet -
tar -xzf "$work/$crate.crate" -C "$work"
mv "$work/$crate-$version" "$work/$crate"
rm -f "$work/$crate/.cargo_vcs_info.json" "$work/$crate/.cargo-ok" "$work/$crate/Cargo.lock" "$work/$crate/Cargo.toml.orig"
patch -d "$work/$crate" -p1 --quiet --no-backup-if-mismatch < "$here/$crate-$version-idle.patch"
if diff -r "$work/$crate" "$here/$crate"; then
  echo "$crate: the copy is crates.io $version with the idle patch"
else
  echo "$crate: the copy differs from crates.io $version with the idle patch (patches/README.md)" >&2
  exit 1
fi
