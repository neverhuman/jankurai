#!/usr/bin/env bash
# Compatibility entrypoint. Components live in canonical sibling checkouts.
set -euo pipefail
while [[ $# -gt 0 ]]; do
  case "$1" in
    --all) shift ;;
    --source)
      case "${2:-}" in
        github|local) shift 2 ;;
        *) printf 'fuse: GitHub is the family source; use --source github or local\n' >&2; exit 2 ;;
      esac ;;
    *) printf 'usage: scripts/fuse.sh [--source github|local] [--all]\n' >&2; exit 2 ;;
  esac
done
exec bash "$(dirname "${BASH_SOURCE[0]}")/family.sh" fuse
