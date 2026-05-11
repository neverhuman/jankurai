# Organization/Team UX Route Matrix

The `organization-team` cell must show membership state explicitly and avoid
letting the browser own durable organization truth.

## Critical Routes

| Route ID | Path | States | Required evidence |
| --- | --- | --- | --- |
| `org-teams-list` | `/admin/org/:organizationId/teams` | loading, empty, success, error, permission-denied | screenshot, ARIA snapshot, accessibility scan |
| `org-team-create` | `/admin/org/:organizationId/teams/new` | success, validation-error, permission-denied | screenshot, ARIA snapshot, accessibility scan |
| `org-team-members` | `/admin/teams/:teamId/members` | loading, empty, success, error, permission-denied | screenshot, ARIA snapshot, accessibility scan |
| `org-team-member-add` | `/admin/teams/:teamId/members/add` | success, duplicate-member, limit-reached, permission-denied | screenshot, ARIA snapshot, accessibility scan |

## Accessibility Requirements

- Team list and membership tables expose row headers or labels.
- Empty state must explain how to create the first team.
- Permission-denied state must identify the missing permission without exposing
  hidden team/member data.
- Validation errors must be associated with their form controls.
- Role selection must be keyboard accessible and announce the selected role.

## State Coverage

Required state query parameter:

```text
?jankuraiState=<state-id>
```

Required state IDs:

- `loading`
- `empty`
- `success`
- `error`
- `permission-denied`
- `validation-error`
- `duplicate-member`
- `limit-reached`

## Proof Notes

The UI may call contract-backed API clients only. It must not:

- define its own durable membership role vocabulary
- infer cross-organization membership rules locally
- hide permission-denied flows behind generic error text
- mutate provider directory sync state

Future generated clients must be declared as generated zones before they become
part of this route matrix.
