#!/usr/bin/env bash
set -euo pipefail
python3 - <<'PY_RESULTS'
import json, os, sys
results = json.loads(os.environ['NEEDS_JSON'])
failed = [name for name, job in results.items() if job['result'] != 'success']
if failed:
    sys.exit('required lanes did not pass: ' + ', '.join(failed))
print('all required lanes passed')
PY_RESULTS
