# UX QA Package

## Workspace Boundary

- Work only in the user-named active repo/worktree.
- Never switch to sibling clones, archives, backups, resolved symlink targets, `/tmp` worktrees, or duplicate roots.
- Never create repo copies or side folders outside the active repo; preserve work with git branches.
- Before edits, report `pwd`, `git rev-parse --show-toplevel`, and `git status --short --branch`.
- Use Jeryu APIs/CLI for local GitLab/MR work; no `glab`, credential scraping, or raw local GitLab API calls.

Read the root `AGENTS.md` and `agent/JANKURAI_STANDARD.md` first.

This package owns the TypeScript and Playwright rendered UX QA runtime. Keep
reports deterministic: routes, screenshots, ARIA snapshots, accessibility
results, geometry checks, and artifact paths must stay machine-readable and
repo-relative.

For package changes, run:

```bash
npm --workspace @jankurai/ux-qa run build
npm --workspace @jankurai/ux-qa run test
```

Do not loosen UX rules without updating the schema, docs, and focused tests.
