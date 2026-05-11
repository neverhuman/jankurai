# jankurai kickoff

<!-- jankurai generated adapter -->
<!-- jankurai agent request v1 sha256:REPLACE_WITH_HASH -->
Read `AGENTS.md` first. Use `agent/JANKURAI_STANDARD.md` as the canonical jankurai standard.
When a user provides a paper, release, implementation, or handoff plan in the conversation, treat that plan as the controlling plan. Do not route such plans through the separate local phase workflow unless the user explicitly names MASTER_PLAN phase work.
For explicit MASTER_PLAN/phase work only, read `agent/MASTER_PLAN.md`, then `tips/phases/00-phase-index.md`, then the active `tips/phases/*.md` phase file. Log explicit phase work in `tips/phases/logs/`.
For explicit MASTER_PLAN/phase planning only, follow `agent/MASTER_PLAN.md#detailed-planner-protocol`.
Use `jankurai kickoff . --intent "<change request>" --out target/jankurai/kickoff.json --md target/jankurai/kickoff.md` to turn user intent into a no-write handoff. If changed paths are missing, keep the result planning-safe and ask bounded questions before any mutable command runs.
Expected receipts: `target/jankurai/kickoff.json`, `target/jankurai/kickoff.md`.
Next command: `jankurai context-pack`.
Stop: the task crosses owners, touches generated zones without source regeneration, or needs a broader proof lane than the receipt can justify.
If jankurai is installed, run `jankurai update --client-start --quiet` before work; do not apply updates unless the user asks.
