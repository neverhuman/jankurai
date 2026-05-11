# jankurai witness

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.
For explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.
For explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.
Use `jankurai witness . --changed-from origin/main --baseline agent/baselines/main.repo-score.json --out target/jankurai/merge-witness.json --md target/jankurai/merge-witness.md` to compare the current branch against the accepted baseline.
Expected receipts: `target/jankurai/merge-witness.json`, `target/jankurai/merge-witness.md`.
Next command: `jankurai repair-plan`.
Stop: changed-path routing, generated-zone touches, baseline score delta, or proof coverage cannot be justified.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
