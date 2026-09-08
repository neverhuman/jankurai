#!/usr/bin/env bash
set -euo pipefail
node --input-type=module - <<'JS_RESULTS'
const jobs = JSON.parse(process.env.NEEDS_JSON);
const failed = Object.keys(jobs).filter(name => jobs[name].result !== 'success');
if (failed.length) throw new Error('required lanes did not pass: ' + failed.join(', '));
console.log('all required lanes passed');
JS_RESULTS
