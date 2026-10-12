#!/usr/bin/env bash
# Bump the package version everywhere: scripts/bump.sh 0.2.0
set -euo pipefail
cd "$(dirname "$0")/.."
new="${1:?usage: bump.sh X.Y.Z}"
[[ "$new" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "not a semver triple: $new"; exit 1; }
old=$(sed -n 's/^version *= *"\(.*\)"/\1/p' typst.toml)
[ "$old" != "$new" ] || { echo "already at $new"; exit 1; }
export old new today="$(date +%F)"

perl -pi -e 's/^version = "\Q$ENV{old}\E"/version = "$ENV{new}"/' typst.toml
# Only the [package] version (the first one) in Cargo.toml.
perl -pi -e '$d ||= s/^version = "\Q$ENV{old}\E"/version = "$ENV{new}"/' plugin/Cargo.toml
perl -0pi -e 's/(name = "dsviz"\nversion = )"\Q$ENV{old}\E"/$1"$ENV{new}"/' plugin/Cargo.lock
perl -pi -e 's{\@preview/dsviz:\Q$ENV{old}\E}{\@preview/dsviz:$ENV{new}}g; s{/(blob|tree)/v\Q$ENV{old}\E/}{/$1/v$ENV{new}/}g' \
  README.md examples/*.typ tests/*.typ
grep -q "^## \[$new\]" CHANGELOG.md \
  || perl -pi -e 's/^## \[Unreleased\]\n/## [Unreleased]\n\n## [$ENV{new}] - $ENV{today}\n/' CHANGELOG.md

scripts/check-version.sh
echo "Review CHANGELOG.md, rebuild (make wasm images), then:"
echo "  git commit -am 'Release v$new' && git tag v$new && git push --follow-tags"
