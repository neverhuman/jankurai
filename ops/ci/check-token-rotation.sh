#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY_ROTATION'
from datetime import date
import os, sys
expires = date.fromisoformat(os.environ.get('AUTOMATION_TOKEN_EXPIRES', '2026-10-08'))
remaining = (expires - date.today()).days
if remaining <= 14:
    print(f'::error::Rotate FAMILY_AUTOMATION_TOKEN before {expires}; {remaining} days remain.')
    sys.exit(1)
print(f'FAMILY_AUTOMATION_TOKEN rotation due {expires}; {remaining} days remain.')
PY_ROTATION
