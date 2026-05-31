# Database Guidance

## Workspace Boundary

- Work only in the user-named active repo/worktree.
- Never switch to sibling clones, archives, backups, resolved symlink targets, `/tmp` worktrees, or duplicate roots.
- Never create repo copies or side folders outside the active repo; preserve work with git branches.
- Before edits, report `pwd`, `git rev-parse --show-toplevel`, and `git status --short --branch`.
- Use Jeryu APIs/CLI for local GitLab/MR work; no `glab`, credential scraping, or raw local GitLab API calls.

Read `agent/JANKURAI_STANDARD.md` first.

Owns durable database truth under `db/`: migrations, constraints, adapter-owned write paths, rollback notes, backfills, and lock safety.
Forbidden: application logic, transport routing, UI concerns, and any write path that bypasses a migration, a named constraint, or an adapter-owned transaction.
Proof lane: migration and constraint tests, plus the DB proof route in `agent/test-map.json`.
