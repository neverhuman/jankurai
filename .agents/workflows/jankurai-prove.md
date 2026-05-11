# jankurai prove

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.
For explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.
For explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.
Use `jankurai prove . --changed <path> --plan-out target/jankurai/proof-plan.json --plan-md target/jankurai/proof-plan.md` to build a proof plan, then run the proof receipts and evidence index under `target/jankurai/`.
Expected receipts: `target/jankurai/proof-plan.json`, `target/jankurai/proof-plan.md`, `target/jankurai/proof-receipts/`, `target/jankurai/evidence-index.json`.
Next command: `jankurai witness`.
Stop: commands are unsigned, not in proof lanes or the test map, or the plan would mutate generated zones without allowlisted proof.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
