# Accepted Baselines

This directory holds reviewed score baselines used by ratchet and public badge checks.

Rules:

- Baselines must come from a clean accepted tree, not from a candidate CI run.
- Generate candidate reports under `target/jankurai/` first.
- Review the report, then copy the accepted JSON here in a dedicated baseline update.
- Do not hand-edit baseline JSON.
- CI copies `agent/baselines/main.repo-score.json` to `target/jankurai/accepted-baseline.json` before the final ratchet audit.
