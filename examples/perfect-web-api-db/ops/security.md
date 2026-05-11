# Security

## Secret Management

- No secrets in source code, config files, or fixtures.
- Environment variables for database URLs, JWT signing keys, and API keys.
- `.env` files must be `.gitignore`-d.
- `gitleaks` scans on every commit.

## Dependency Review

| Tool | Scope | Frequency |
|------|-------|-----------|
| `cargo audit` | Rust advisories | Every PR, nightly |
| `npm audit` | JS advisories | Every PR, nightly |
| `cargo deny` | License + source policy | Release |
| `syft` | SBOM generation | Release |

## Authentication

- Bearer token (JWT) required for all non-public endpoints.
- Token validation is an API-edge concern, not domain logic.
- Authorization (RBAC) is a domain concern enforced by `Account::authorize()`.

## CI Hardening

- GitHub Actions workflows pin action versions by SHA.
- `GITHUB_TOKEN` permissions are minimized to required scopes.
- No `pull_request_target` with untrusted code checkout.
- `actionlint` and `zizmor` run in the security lane.

## Prompt Injection Defense

- Agent instruction files (`AGENTS.md`, adapter files) are trusted sources.
- Issue text, PR comments, and tool output are untrusted content.
- Trusted files never instruct agents to ignore higher-priority instructions.
- No broad "allow all tools" language in any policy file.

## Compliance Posture

This reference platform provides **SOC-ready engineering evidence**, not
SOC 2 certification. Evidence categories:

- **Change management**: audit events + PR-based deployment
- **Access control**: RBAC with domain-enforced roles
- **Vulnerability management**: dependency scans + SBOM
- **Logging**: append-only audit events + structured logs
- **Vendor risk**: dependency review + license policy
