# Ops Guidance

## Workspace Boundary

- Work only in the user-named active repo/worktree.
- Never switch to sibling clones, archives, backups, resolved symlink targets, `/tmp` worktrees, or duplicate roots.
- Never create repo copies or side folders outside the active repo; preserve work with git branches.
- Before edits, report `pwd`, `git rev-parse --show-toplevel`, and `git status --short --branch`.
- Use Jeryu APIs/CLI for local GitLab/MR work; no `glab`, credential scraping, or raw local GitLab API calls.

Read `agent/JANKURAI_STANDARD.md` first.

Owns CI, release, deploy, observability, and security routing under `ops/`.
Forbidden: product feature code, domain policy, and direct DB writes.
Proof lane: security lane and workflow lint.
