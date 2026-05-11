# jankurai context-pack

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.
For explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.
For explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.
Use `jankurai context-pack . --changed <path> --max-tokens 6000 --out target/jankurai/context-pack.json --md target/jankurai/context-pack.md` to turn a bounded change set into a repo-aware context bundle.
Expected receipts: `target/jankurai/context-pack.json`, `target/jankurai/context-pack.md`.
Next command: `jankurai prove`.
Stop: the task is too broad, owner/test routing is unclear, or generated-zone work needs source regeneration first.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
