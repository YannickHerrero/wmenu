#!/usr/bin/env bash
# Bump the version in Cargo.toml. Prints the new version to stdout.
# Usage: ./scripts/bump-version.sh [patch|minor|major]
set -euo pipefail

bump="${1:-patch}"
cargo_toml="$(dirname "$0")/../Cargo.toml"

current=$(grep -m1 '^version = ' "$cargo_toml" | sed -E 's/^version = "(.*)"$/\1/')
IFS=. read -r maj min pat <<<"$current"

case "$bump" in
    patch) pat=$((pat + 1)) ;;
    minor) min=$((min + 1)); pat=0 ;;
    major) maj=$((maj + 1)); min=0; pat=0 ;;
    *) echo "unknown bump: $bump (expected patch|minor|major)" >&2; exit 1 ;;
esac

new="$maj.$min.$pat"
sed -i -E "s/^version = \"$current\"$/version = \"$new\"/" "$cargo_toml"
echo "$new"
