#!/usr/bin/env bash
set -euo pipefail
node --input-type=module - <<'JS_ROTATION'
const expires = process.env.AUTOMATION_TOKEN_EXPIRES || '2026-10-08';
const remaining = Math.ceil((Date.parse(expires + 'T00:00:00Z') - Date.now()) / 86400000);
if (!Number.isFinite(remaining) || remaining <= 14) {
  console.error(`::error::Rotate FAMILY_AUTOMATION_TOKEN before ${expires}; ${remaining} days remain.`);
  process.exitCode = 1;
} else console.log(`FAMILY_AUTOMATION_TOKEN rotation due ${expires}; ${remaining} days remain.`);
JS_ROTATION
