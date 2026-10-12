#!/usr/bin/env bash
# Stage the files that go into typst/packages: <out>/dsviz/<version>/
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:?usage: package.sh <out-dir>}"
version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' typst.toml)
dest="$out/dsviz/$version"
rm -rf "$dest"
mkdir -p "$dest/src" "$dest/docs/images"
cp typst.toml lib.typ README.md LICENSE "$dest/"
cp src/dsviz.typ src/dsviz.wasm "$dest/src/"
cp docs/images/*.png "$dest/docs/images/" 2>/dev/null || true
echo "staged $dest"
