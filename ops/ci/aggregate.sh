#!/usr/bin/env bash
# Exact hub aggregate: require the complete job collection, not merely
# "every present dependency succeeded". Empty, array, renamed, missing,
# extra, or non-success collections fail. Matrix release-build remains one
# needs key whose overall result covers both Linux and macOS legs.
set -euo pipefail
jq -e -s '
  length == 1 and (
    .[0]
    | type == "object"
    and keys == ["fast", "integration", "release-build"]
    and all(.[]; type == "object" and .result == "success")
  )
' <<< "${NEEDS_JSON:?NEEDS_JSON is required}"
echo 'all required lanes passed'
