# Observability

## Trace IDs And Request IDs

Every API request must carry a correlation ID. The API edge layer generates one
if the caller does not supply `X-Request-Id`. The correlation ID flows through:

1. API handler → application command → adapter calls
2. Structured log fields (JSON)
3. Database audit events (`audit_events.detail`)
4. Error responses (ProblemDetail `instance` field)

## Structured Logging

All log output must be machine-readable JSON with at minimum:

- `timestamp` (ISO 8601)
- `level` (trace, debug, info, warn, error)
- `request_id`
- `service` (from `service_name()`)
- `message`
- `target` (module path)

## Metrics

Recommended metrics for the reference platform:

| Metric | Type | Labels |
|--------|------|--------|
| `http_requests_total` | counter | method, path, status |
| `http_request_duration_seconds` | histogram | method, path |
| `db_query_duration_seconds` | histogram | query_name |
| `audit_events_total` | counter | action, outcome |

## Error Reporting

All errors must be:

1. **Typed** — use `DomainError` variants, not string messages
2. **Traceable** — include request ID in error response
3. **Agent-repairable** — structured enough for an agent to diagnose from logs

## Health Check

`GET /health` returns `200` with:

```json
{
  "status": "ok",
  "service": "perfect-web-api-db",
  "version": "0.1.0"
}
```

No health check should expose internal state, secrets, or connection strings.
