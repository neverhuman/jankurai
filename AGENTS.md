# Jankurai Agent Instructions

## Workspace Boundary

- Work only in the user-named active repo/worktree.
- Never switch to sibling clones, archives, backups, resolved symlink targets, `/tmp` worktrees, or duplicate roots.
- Never create repo copies or side folders outside the active repo; preserve work with git branches.
- Before edits, report `pwd`, `git rev-parse --show-toplevel`, and `git status --short --branch`.
- Use Jeryu APIs/CLI for local GitLab/MR work; no `glab`, credential scraping, or raw local GitLab API calls.

Read `agent/JANKURAI_STANDARD.md` first. The full standard is in
`docs/agent-native-standard.md`; the paper mission is in `docs/mission.md`.
When a user provides a paper, release, implementation, or handoff plan in the
conversation, treat that plan as the controlling plan. Do not route such plans
through the separate local phase workflow unless the user explicitly names it.

This workspace is writing and validating the paper:
`Jankurai: A Versioned Repository Conformance Standard for Trustworthy AI-Assisted Merge`.

Access contract: local agent workspaces use `~/.jeryu/access.toml`, `jeryu access doctor`, and `jeryu access repair --repo . --yes`; do not install/use `glab`, scrape credential stores, or keep HTTP local GitLab origins.

## Rules

- Keep new files inside the repository root.
- Treat `reference/` as read-only source material.
- Do not hand-edit generated artifacts unless the generator/source is changed.
- Keep root guidance short; put durable detail in `docs/` or `agent/`.
- Agents must not hide audit failures by adding Rust files, core files, or broad
  source roots to masking/exclusion policy, generated-zone shields, ignore
  lists, or post-audit filters. Only the user may intentionally edit audit
  masking policy by manual, visible review.
- Use `cargo run -p jankurai -- audit . --json .jankurai/repo-score.json --md .jankurai/repo-score.md`
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
