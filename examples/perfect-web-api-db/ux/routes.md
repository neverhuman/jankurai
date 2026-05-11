# UX Routes

## Route Matrix

| Route | Component | States | Auth Required | Critical Path |
|-------|-----------|--------|--------------|---------------|
| `/admin/accounts` | AdminDashboard | loading, empty, error, denied, success | Yes (admin+) | Yes |
| `/resources` | ResourceDashboard | loading, empty, error, success | Yes (member+) | Yes |

## State Coverage

Every route must handle these UI states:

1. **Loading** — shown while API call is in flight
2. **Empty** — shown when the response is an empty list
3. **Error** — shown when the API returns a non-2xx response (displays ProblemDetail)
4. **Permission Denied** — shown when the caller lacks the required role
5. **Success** — shown with the data rendered

## Accessibility

- All interactive elements have unique `id` attributes.
- Tables use `aria-label` and `scope` attributes.
- Navigation uses `aria-label="Primary navigation"`.
- Error states use `role="alert"`.
- Loading states use `aria-busy="true"`.

## Generated Client Policy

Frontend API calls must use generated clients from `contracts/openapi.json`.
Do not handwrite `fetch()` calls that mirror the contract schema. The
scaffold currently uses inline types as a placeholder; production must
replace these with generated types from `openapi-typescript-codegen` or
equivalent tooling declared in `agent/generated-zones.toml`.
