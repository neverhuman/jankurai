# UX QA Package

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
