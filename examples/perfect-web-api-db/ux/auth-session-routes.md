# Auth/Session UX Routes

## Route Matrix

| Route | Component | States | Auth Required | Critical Path |
| --- | --- | --- | --- | --- |
| `/session/new` | SignInShell | loading, error, success | No | Yes |
| `/session/current` | CurrentSessionPanel | loading, expired, revoked, success | Yes | Yes |
| `/admin/sessions` | AdminSessionTable | loading, empty, error, denied, success | Yes (admin+) | Yes |

## State Coverage

Every route must handle:

1. Loading — request or provider check is in progress.
2. Error — ProblemDetail from the auth/session contract.
3. Permission denied — caller lacks RBAC authority.
4. Expired — current session TTL has elapsed.
5. Revoked — current or target session was explicitly revoked.
6. Success — session state rendered without exposing raw tokens.

## Accessibility

- Sign-in form fields have stable labels and IDs.
- Error output uses `role="alert"`.
- Session tables use `aria-label` and scoped headers.
- Revocation buttons have unique accessible names.
- Expired/revoked state is text-visible and not color-only.

## Generated Client Policy

UI code must call generated client methods from `contracts/auth-session.openapi.json` or the merged production OpenAPI contract. Do not handwrite fetch calls that mirror session DTOs.
