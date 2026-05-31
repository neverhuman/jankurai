# Jankurai Auditor Crate

## Workspace Boundary

- Work only in the user-named active repo/worktree.
- Never switch to sibling clones, archives, backups, resolved symlink targets, `/tmp` worktrees, or duplicate roots.
- Never create repo copies or side folders outside the active repo; preserve work with git branches.
- Before edits, report `pwd`, `git rev-parse --show-toplevel`, and `git status --short --branch`.
- Use Jeryu APIs/CLI for local GitLab/MR work; no `glab`, credential scraping, or raw local GitLab API calls.

Read the root `AGENTS.md` and `agent/JANKURAI_STANDARD.md` first.

This crate owns the Rust auditor, CLI commands, report schemas, renderers, and
conformance checks. Keep proof and repo tooling Rust-first; do not add Python
helpers for auditor behavior.

For scoring or report changes, add focused Rust tests under
`crates/jankurai/tests/`, then run:

```bash
cargo test -p jankurai
cargo run -p jankurai -- versions
```

Do not hand-edit generated report artifacts. Regenerate score outputs with the
root `just score` lane.
