#!/bin/sh
# Fails unless patches/gpui-pre-windows is the crates.io crate with the patches of series applied in order, and nothing
# else (patches/README.md). The workspace patches the crate, so Cargo never downloads the original: this script does,
# and checks it against crates.io's checksum. Run from anywhere; needs curl, sha256sum, tar, patch and diff.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
crate=gpui-pre-windows
# the version and the checksum of the published .crate, as the crates.io index gives it; scripts/bump-cargo.mjs sets both
version=0.3.11
sha256=5141a3db59bab761a16ab19529fe4117a86e91922e8fc00af6dfc4829cb1744d
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
curl -sSfL -o "$work/$crate.crate" "https://static.crates.io/crates/$crate/$crate-$version.crate"
echo "$sha256  $work/$crate.crate" | sha256sum -c --quiet -
tar -xzf "$work/$crate.crate" -C "$work"
mv "$work/$crate-$version" "$work/$crate"
rm -f "$work/$crate/.cargo_vcs_info.json" "$work/$crate/.cargo-ok" "$work/$crate/Cargo.lock" "$work/$crate/Cargo.toml.orig"
# the patches of series, in order, without its comments
grep -v '^#' "$here/series" > "$work/series"
while IFS= read -r file; do
  if [ -n "$file" ]; then patch -d "$work/$crate" -p1 --quiet --no-backup-if-mismatch < "$here/$file"; fi
done < "$work/series"
if diff -r "$work/$crate" "$here/$crate"; then
  echo "$crate: the copy is crates.io $version with its patches"
else
  echo "$crate: the copy differs from crates.io $version with its patches (patches/README.md)" >&2
  exit 1
fi
