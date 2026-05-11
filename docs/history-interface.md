# History Interface

Jankurai keeps a bounded score ledger for external consumers that want trend plots, restore workflows, or compact receipts without reading full audit reports.

## Commands

```bash
jankurai history latest \
  --history agent/score-history.jsonl \
  --out -
```

```bash
jankurai history export \
  --history agent/score-history.jsonl \
  --window 200 \
  --out target/jankurai/history-export.json \
  --md target/jankurai/history-export.md
```

```bash
jankurai history compact \
  --history agent/score-history.jsonl \
  --max-rows 500 \
  --max-bytes 1048576
```

```bash
jankurai history restore \
  --mirror "$JANKURAI_HISTORY_MIRROR" \
  --repo-id auto \
  --out agent/score-history.jsonl
```

`jankurai score trend` remains the compact summary command and uses the same history loader as `history export`.

## Row Contract

History rows are stable JSONL entries with:

- `schema_version`
- `standard_version`
- `auditor_version`
- `generated_at`
- `run_id`
- `repo_id`
- `repo_remote`
- `branch`
- `commit`
- `dirty_worktree`
- `scope`
- `changed_paths`
- `score`
- `raw_score`
- `finding_count`
- `hard_findings`
- `soft_findings`
- `decision`
- `minimum_score`
- `caps_applied`
- `report_fingerprint`
- `input_fingerprint`
- `policy_fingerprint`
- `repo_score_json_path`
- `repo_score_md_path`

New rows use `schema_version = "1.1.0"`. The parser still accepts legacy `1.0.0` rows.

## Retention

Default retention keeps the newest `500` local rows and caps the local file at `1,048,576` bytes. The optional mirror sink defaults to `5,000` rows.

Deduplication skips consecutive equivalent rows for the same repo, commit, scope, paths, score, finding counts, decision, and caps.

## Mirror Setup

Set `JANKURAI_HISTORY_MIRROR` to a file path to mirror the compact JSONL stream. Set `JANKURAI_HISTORY_MIRROR_REQUIRED=1` to make mirror failure block the audit.

Optional override env vars:

- `JANKURAI_SCORE_HISTORY_MAX_ROWS`
- `JANKURAI_SCORE_HISTORY_MAX_BYTES`

## Recovery

If the local history file is lost, restore from the mirror and then compact it back to the local defaults. If the mirror is missing rows for the current repo ID, restore fails with a clear error instead of inventing data.

External plotting tools should read `history latest` for the latest point and `history export` for a bounded window. Do not scrape `agent/repo-score.json` for trend plots.
