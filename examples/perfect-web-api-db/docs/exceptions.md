# Exception Inventory

## Current Exceptions

| Exception | Owner | Reason | Expiry | Migration Path |
|-----------|-------|--------|--------|---------------|
| Inline TypeScript types in `App.tsx` | frontend | Generated client tooling not yet integrated | When `openapi-typescript-codegen` or equivalent is added to build pipeline | Replace inline interfaces with generated imports from `contracts/openapi.json` |
| Adapter layer is documentation-only | backend | Reference scaffold does not ship a running database | When a running integration test is added | Implement `PgAccountRepository`, `PgResourceRepository`, `PgAuditLog` with sqlx |
| No Playwright E2E tests | frontend | Scaffold does not run a dev server | When Vite build pipeline is added | Add critical path E2E tests per `ux/routes.md` state matrix |

## Exception Policy

Every exception must have:

1. **Owner** — the team or layer responsible for resolving it.
2. **Reason** — why the exception exists today.
3. **Expiry** — the condition under which the exception should be resolved.
4. **Migration path** — how to fix it when the expiry condition is met.

Exceptions without expiry are rejected by the Jankurai standard.
