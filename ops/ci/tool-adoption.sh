#!/usr/bin/env bash
# Tool-adoption evidence lane for the jankurai hub.
#
# The hub adopts Jankurai's first-class audit subcommand in place of ad-hoc repo
# scoring, manual proof routing, hand-rolled contract-drift checks, manual authz
# and agent-tool-supply review, and a manual launch checklist. All of those are
# proven by the single ratchet audit command below, whose repo-score artifacts
# are uploaded by the workflow's actions/upload-artifact step so the audit can
# prove the replacement actually executed in CI. Rust/product-only lanes are not
# applicable to this control-plane-only hub (see agent/tool-adoption.toml).
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
cd "$REPO_ROOT"

mkdir -p target/jankurai .jankurai

# audit-ci / proof-routing / contract-drift / authz-matrix / agent-tool-supply
# / release-readiness all adopt the ratchet audit command.
log "tool-adoption: ratchet audit"
    # Recompute accepted-source evidence with the locked auditor (do not copy
    # a frozen predecessor JSON).
    bash ops/ci/prepare-baseline.sh
    jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md --repair-queue-jsonl target/jankurai/repair-queue.jsonl --full
# Adopted artifacts: .jankurai/repo-score.json .jankurai/repo-score.md
# target/jankurai/repair-queue.jsonl
cp -f target/jankurai/repo-score.json .jankurai/repo-score.json
cp -f target/jankurai/repo-score.md .jankurai/repo-score.md

assert_artifact .jankurai/repo-score.json
assert_artifact .jankurai/repo-score.md
assert_artifact target/jankurai/repair-queue.jsonl
