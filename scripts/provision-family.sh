#!/usr/bin/env bash
set -euo pipefail

# Historical entrypoint retained so old automation fails closed. Jankurai no
# longer provisions or publishes repositories through GitHub. Repository
# creation, imports, protected merges, and immutable tags must use the governed
# forge lifecycle selected by the tracked family manifest.
printf '%s\n' \
  'provision-family: disabled; use the protected manifest-selected forge lifecycle' >&2
exit 1
