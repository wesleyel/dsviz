#!/usr/bin/env bash
# Fail if the version in typst.toml disagrees with Cargo.toml, README or examples.
set -euo pipefail
cd "$(dirname "$0")/.."
v=$(sed -n 's/^version *= *"\(.*\)"/\1/p' typst.toml)
cargo=$(sed -n 's/^version *= *"\(.*\)"/\1/p' plugin/Cargo.toml | head -1)
status=0
if [ "$cargo" != "$v" ]; then
  echo "plugin/Cargo.toml has $cargo, typst.toml has $v"; status=1
fi
while IFS= read -r line; do
  echo "stale import: $line"; status=1
done < <(grep -rnoE '@preview/dsviz:[0-9]+\.[0-9]+\.[0-9]+' README.md examples tests | grep -v ":$v\$" || true)
if ! grep -q "^## \[$v\]" CHANGELOG.md; then
  echo "CHANGELOG.md has no entry for $v"; status=1
fi
if [ -n "${GITHUB_REF_NAME:-}" ] && [[ "$GITHUB_REF_NAME" == v* ]] && [ "$GITHUB_REF_NAME" != "v$v" ]; then
  echo "tag $GITHUB_REF_NAME does not match version $v"; status=1
fi
[ $status -eq 0 ] && echo "version $v is consistent"
exit $status
