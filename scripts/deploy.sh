#!/usr/bin/env bash
# Cut a release: bump version, commit, tag, push.
# Usage: ./scripts/deploy.sh [patch|minor|major]
set -euo pipefail

bump="${1:-patch}"
here="$(dirname "$0")"

"$here/check-clean-tree.sh"

new=$("$here/bump-version.sh" "$bump")
echo "Releasing v$new"

cargo check --target x86_64-pc-windows-gnu --offline
git add Cargo.toml Cargo.lock
git commit -m "release: v$new"
git tag -a "v$new" -m "v$new"
git push origin master
git push origin "v$new"
echo "Pushed v$new — CI build should start shortly."
