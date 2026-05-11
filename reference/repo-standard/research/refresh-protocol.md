# Refresh Protocol

Last reviewed: 2026-03-31

## Monthly Checks

1. Re-open the official Rust, Cargo, Clippy, and agent-instructions docs.
2. Re-scan Google, AWS, Cloudflare, Microsoft, Firecracker, and Hugging Face sources for new Rust adoption or benchmark posts.
3. Check for new coding-agent papers touching repository context, test impact, or graph-guided repair.

## Quarterly Checks

1. Re-run:
   - `cargo run -p cargo-vrc -- map --output-dir .`
   - `cargo run -p cargo-aer -- scan --output aer-findings.json`
   - `cargo run -p arc-bench -- run repo-shape`
   - `cargo run -p arc-bench -- run runtime`
   - `cargo run -p arc-bench -- run exceptions`
   - `cargo run -p arc-bench -- report --input-dir benchmark/results --output benchmark/results/comparison.md`
2. Review every note-level finding with an existing AER to confirm the exception still makes sense.
3. Refresh `paper/paper-for-agents.md`, `paper/executive-brief.md`, and `paper/citation-index.md`.

## Before Paper Updates

1. Add new evidence-backed claims to `docs/research/claim-citation-ledger.md`.
2. Update `docs/research/dated-bibliography.md` and `docs/research/source-notes.md`.
3. Run `cargo run -p paper-sync -- index`.
4. Run `cargo run -p paper-sync -- check`.

## Failure Policy

- If a claim loses support, downgrade it to proposed doctrine before editing the manuscript.
- If a benchmark flips sign, keep the result and update the interpretation; do not hide the measurement.
- If an AER no longer has a convincing reason, remove it and refactor the code.
