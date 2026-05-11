Append-only operational logs for moonshot phase work.

These files are the canonical tracked cross-agent phase history. Keep volatile proof receipts, generated reports, screenshots, SARIF, and command output under target/jankurai/ and cite those paths from log entries when they matter.

Required format for new entries, one line per event, UTF-8:
  timestamp_utc | actor/tool | phase | action | changed_paths | validation | artifacts | git_sha | residual_risk

Required events for every phase attempt:
  start - before edits, with owned paths and expected proof lane
  progress - when scope, validation choice, or residual risk changes
  finish - at handoff, with changed paths, validation results, artifacts, git SHA, and residual risk

Files:
  00-phase-index.log - Phase 00 phase index
  01-standard-stabilization.log - Phase 01 standard stabilization
  02-rule-engine-semantic-oracle.log - Phase 02 rule engine / semantic oracle
  03-proof-router-evidence-ledger.log - Phase 03 proof router / evidence ledger
  04-init-profiles-golden-repos.log - Phase 04 init profiles / golden repos
  05-ux-proof-platform.log - Phase 05 UX proof / UX QA policy validation
  06-security-supply-chain-compliance.log - Phase 06 security / supply chain evidence envelope
  07-contracts-db-generated-boundaries.log - Phase 07 contracts, DB, and generated boundaries
  08-agent-context-repair.log - Phase 08 agent context / repair artifacts
  09-reference-product-platform.log - Phase 09 reference product platform
  10-reuse-registry-certified-cells.log - Phase 10 reuse registry / certified cells
  11-migration-engine.log - Phase 11 migration engine
  12-benchmark-certification-governance.log - Phase 12 benchmark certification / governance
  13-autonomous-repair-optimization.log - Phase 13 autonomous repair / optimization

Do not rewrite history; append only. Tracked in git, unlike target/jankurai/.
