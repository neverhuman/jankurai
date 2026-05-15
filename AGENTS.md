# Jankurai Agent Instructions

Read `agent/JANKURAI_STANDARD.md` first. The full standard is in
`docs/agent-native-standard.md`; the paper mission is in `docs/mission.md`.
When a user provides a paper, release, implementation, or handoff plan in the
conversation, treat that plan as the controlling plan. Do not route such plans
through the separate local phase workflow unless the user explicitly names it.

This workspace is writing and validating the paper:
`Jankurai: A Versioned Repository Conformance Standard for Trustworthy AI-Assisted Merge`.

## Rules

- Keep new files inside the repository root.
- Treat `reference/` as read-only source material.
- Do not hand-edit generated artifacts unless the generator/source is changed.
- Keep root guidance short; put durable detail in `docs/` or `agent/`.
- Use `cargo run -p jankurai -- audit . --json agent/repo-score.json --md agent/repo-score.md`
  for the audit lane. After a clean full scan this defaults to a smart fast scan (git-status
  changed files only). Use `--full` to force a complete scan. Use `jankurai copy-code .`
  for an explicit copy-code check (not included in fast scans).
- Use `latexmk -pdf -interaction=nonstopmode -halt-on-error -outdir=paper paper/jankurai.tex`
  for the paper lane.

## Validation

- Fast: `just fast`
- Audit: `just score`
- Paper: `just paper`
- Full: `just check`
