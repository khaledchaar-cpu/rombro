#!/usr/bin/env bash
# Print release notes for a tag: docs/releases/<tag>.md if present,
# otherwise generated from feat/fix/perf commits since the previous tag.
set -euo pipefail
tag="${1:?usage: release-notes.sh <tag>}"
file="docs/releases/$tag.md"
if [[ -f "$file" ]]; then cat "$file"; exit 0; fi
prev=$(git describe --tags --abbrev=0 "$tag^" 2>/dev/null || true)
range="${prev:+$prev..}$tag"
section() {
  local title="$1" pattern="$2" lines
  lines=$(git log --format=%s "$range" | grep -E "^$pattern(\([^)]*\))?!?: " \
    | sed -E 's/^[a-z]+(\([^)]*\))?!?: (.)/- \u\2/' || true)
  [[ -n "$lines" ]] && printf '### %s\n%s\n\n' "$title" "$lines"
  return 0
}
section Features feat
section Fixes fix
section Performance perf
