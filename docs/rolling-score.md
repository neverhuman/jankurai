# Rolling Score

The rolling score is Jankurai's trust ledger. Each audit can append a compact JSONL row to `.jankurai/score-history.jsonl` and a CSV companion. The dedicated history interface is the stable surface for external tools and recovery workflows.

Compare an accepted baseline to a candidate report:

```bash
jankurai score diff \
  --base agent/baselines/main.repo-score.json \
  --head target/jankurai/repo-score.json \
  --out target/jankurai/score-diff.json \
  --md target/jankurai/score-diff.md
```

Summarize the recent ledger:

```bash
jankurai score trend \
  --history .jankurai/score-history.jsonl \
  --window 30 \
  --out target/jankurai/score-trend.json \
  --md target/jankurai/score-trend.md
```

`jankurai history latest` returns the latest JSONL row, `history export` emits a bounded window with markdown, `history compact` rewrites the ledger in place, and `history restore` rebuilds local history from the mirror sink. `score diff` compares final score, raw score, caps, and findings by fingerprint first, then by rule/path/problem fallback. `score trend` reports the latest window, score delta, best/worst score, latest decision, and high/critical count. Ratchet gates must use an explicit accepted baseline; no implicit current score can become the baseline.
