# Bad SQL Behavior: Comprehensive Guide

This document organizes the worst SQL behaviors that are inexcusable in production.

## Known Best Practices

To counteract the anti-patterns listed below, ensure adherence to these core SQL best practices:

- **Parameterize all inputs**: Never concatenate strings to build SQL queries.
- **Normalize schemas carefully**: Start with 3NF and denormalize only with measured performance justification.
- **Enforce data integrity**: Use `NOT NULL`, `UNIQUE`, `CHECK` constraints, and foreign keys.
- **Manage connections efficiently**: Use connection pooling and configure timeouts.
- **Version control all schema changes**: Use migration tools and review query plans for all DDL operations.

## 1. Immediate rejection behaviors

1. **Building SQL by string concatenating untrusted input.** This is the classic SQL injection failure: user-controlled data is interpreted as SQL code. Use parameters/bind variables; do not “sanitize later.” OWASP recommends prepared statements/parameterized queries, and MITRE CWE-89 describes the core failure as constructing SQL from externally influenced input without neutralizing special elements.
2. **Relying on deny-lists, regex filters, or manual escaping as the primary SQL injection defense.** MITRE explicitly warns deny-lists are incomplete and that manual escaping/stored procedures can still fail when dynamic SQL is built unsafely.
3. **Letting an application connect as a PostgreSQL superuser, table owner, or migration owner for normal runtime work.** Runtime code should use least-privilege roles. MITRE lists least privilege as a SQL injection mitigation, and PostgreSQL privileges can be granted or revoked per object and role.
4. **Using `trust` authentication over TCP/IP in production.** The PostgreSQL wiki calls out that `host all all 0.0.0.0/0 trust` lets anyone who can reach the server authenticate as any PostgreSQL user, including superusers.
5. **Shipping code that depends on app-only data integrity when the database can enforce it.** Primary keys, foreign keys, unique constraints, `NOT NULL`, and `CHECK` constraints exist to protect data regardless of which app, script, migration, or admin session writes to the database. PostgreSQL documentation explicitly frames constraints as the mechanism for controlling allowed data and raising errors on violations.
6. **No tested restore path.** A backup that has not been restored is an assumption, not a recovery strategy. A GitHub SQL anti-pattern list calls out “not testing restores,” and GitLab’s 2017 database incident is a real-world reminder that backup mechanisms can silently fail.
7. **Running destructive production migrations without a compatibility, rollback, lock, and data-safety plan.** GitLab’s migration guide treats avoiding downtime as a first-order requirement and documents multi-step approaches for destructive changes such as dropping columns.
8. **Accepting AI/vibe-coded SQL without inspecting the generated SQL, constraints, query plans, migrations, and rollback behavior.** Current PostgreSQL/AI community work notes that AI tools often produce SQL that mixes dialects, misses modern PostgreSQL features, or omits constraints and indexes.

## 1. SQL injection and unsafe dynamic SQL

1. Concatenate, interpolate, template, or format untrusted input into SQL.
2. Treat “internal” request fields as trusted: headers, cookies, JWT claims, admin forms, queue messages, CSV imports, webhooks, LLM output, or feature flags.
3. Build dynamic `WHERE`, `ORDER BY`, `LIMIT`, `OFFSET`, table names, schema names, column names, operators, or direction strings directly from user input.
4. Use escaping or regex sanitization as the main injection defense.
5. Assume stored procedures are safe while they build SQL strings internally.
6. Use PL/pgSQL `EXECUTE` with unsafely concatenated values or identifiers.
7. Use ORM “raw SQL” APIs with string interpolation.
8. Build `IN (...)` lists by joining user-provided strings.
9. Accept user-provided SQL fragments for filtering, reporting, search, sorting, dashboards, or “advanced query” features.
10. Expose detailed SQL errors, constraint names, query text, schema names, or stack traces to users.
11. Log fully expanded SQL with secrets, passwords, tokens, session IDs, or PII.
12. Assume client-side parameterization is enough if the final server query is still a string.
13. Use dynamic identifiers without a strict allow-list mapping from user-facing names to known safe identifiers.
14. Use “AI-generated safe SQL” as a substitute for bind parameters.
15. Allow analytics, BI, or admin tools to run arbitrary SQL as an application owner role.
16. Treat escaping quotes as enough for non-string contexts such as identifiers, operators, JSON paths, `ORDER BY`, or function names.
17. Reuse a database connection after changing session state, role, `search_path`, time zone, or temp objects without resetting it.
18. Concatenating user input into SQL strings.
19. Interpolating request parameters into `WHERE`, `ORDER BY`, `LIMIT`, `OFFSET`, `JOIN`, or `RETURNING` clauses.
20. Treating “we escaped quotes” as a SQL injection defense.
21. Building raw SQL from JSON/API payloads without strict allowlists.
22. Letting users supply column names, table names, sort directions, operators, or function names without allowlisting.
23. Using bind parameters for values but forgetting that identifiers still need allowlisting and safe quoting.
24. Building PL/pgSQL dynamic SQL with raw variables instead of `format()`, `%I`, `%L`, `quote_ident`, `quote_literal`, `quote_nullable`, or `USING`.
25. Dollar-quoting dynamic variable content in PL/pgSQL and pretending it is safe.
26. Using ORM “raw SQL” fragments with string interpolation.
27. Passing unreviewed GraphQL/filter-builder input directly into SQL.
28. Letting frontend state define arbitrary SQL predicates.
29. Accepting AI-generated SQL that contains interpolated strings.
30. Using “internal admin tool” as an excuse for injectable SQL.
31. Relying on WAFs, API gateways, or input validation instead of parameterization.
32. Exposing SQL error details to end users.
33. Logging full SQL with secrets, tokens, passwords, session IDs, or PII.
34. Using `EXECUTE` in stored procedures without proving every interpolated part is safe.
35. Building `IN (...)` lists by joining raw strings instead of binding values safely.
36. Using “trusted users” as the security model for SQL construction.
37. Copy-pasting SQL from dashboards into code without parameter review.

## 10. JSON/document behavior that is never excusable in Postgres design

1. Putting core relational entities in JSON because “migrations are annoying.”
2. Storing required fields in JSON with no constraints.
3. Storing foreign keys inside JSON with no enforceable referential integrity.
4. Storing values in JSON that must be indexed, joined, validated, and reported constantly.
5. Allowing multiple incompatible JSON shapes in the same column without versioning.
6. Querying deeply nested JSON in hot paths without indexes and statistics strategy.
7. Using JSON as an EAV system.
8. Using JSON for data that must participate in uniqueness constraints.
9. Storing timestamps, money, booleans, and numbers as JSON strings.
10. Letting application versions write different JSON shapes without compatibility tests.
11. Having no migration plan from JSON to typed columns once fields become core.
12. Treating JSON schema validation only in application code as enough for critical data.

## 10. Migrations and DDL release behavior

1. Make manual production schema changes outside version control.
2. Run migrations no one reviewed.
3. Run migrations generated by an ORM or AI without reading the SQL.
4. Run migrations that require application downtime unless the system is intentionally offline and approved.
5. Combine destructive schema changes with application deployment in one irreversible step.
6. Make backward-incompatible changes without expand/contract rollout.
7. Rename or drop columns before all running app versions stop using them.
8. Drop tables before proving no code, jobs, reports, views, functions, or replicas depend on them.
9. Add `NOT NULL`, defaults, generated columns, foreign keys, unique constraints, or indexes to large live tables without checking locks, rewrites, validation, and version-specific behavior.
10. Add multiple foreign keys in one risky migration.
11. Add foreign keys and leave them unvalidated forever.
12. Add constraints without cleaning existing bad data.
13. Create unique indexes without duplicate cleanup.
14. Use non-concurrent index creation on large hot tables without a maintenance window.
15. Assume concurrent index creation is safe without monitoring and failure cleanup.
16. Run `ALTER TABLE` casually on large tables.
17. Run broad `UPDATE` / `DELETE` migrations without batching.
18. Write batch migrations that cannot resume.
19. Write batch migrations without an upper bound, progress table, or break condition.
20. Write batch migrations that can infinite-loop.
21. Backfill data without measuring WAL, bloat, locks, replication lag, and job duration.
22. Depend on current application code inside migrations that may be rerun later.
23. Make migrations non-idempotent when deployment tooling may retry.
24. Omit `lock_timeout` and `statement_timeout` for production migrations.
25. Run DDL while long transactions are open.
26. Run `VACUUM FULL`, `CLUSTER`, blocking `REINDEX`, or table rewrites during normal traffic without an explicit maintenance plan.
27. Modify RLS policies, privileges, or security-definer functions without security review.
28. Modify financial, identity, tenant, or audit tables without extra validation.
29. Fail to test migration rollback or roll-forward.
30. Fail to test migrations on production-like data volume.
31. Treat staging success on tiny data as production proof.
32. Run migrations without a fresh backup / PITR confidence.
33. Ignore migration observability: locks, waits, rows processed, lag, errors, and rollback state.
34. Leave schema drift between environments.
35. Leave old columns, old indexes, old triggers, and old functions forever because cleanup is scary.

## 10. Migrations, DDL, and backfill recklessness

1. Changing production schema by hand outside versioned migrations.
2. Running unreviewed SQL in production consoles.
3. Running migrations generated by AI without reading every line.
4. Running migrations without knowing table size.
5. Running migrations without knowing lock level.
6. Running migrations without `lock_timeout`.
7. Running migrations without `statement_timeout`.
8. Running migrations without a rollback or forward-fix plan.
9. Running destructive migrations without fresh backups.
10. Running destructive migrations without restore confidence.
11. Combining schema change, code change, and data rewrite in one risky deploy.
12. Dropping a column in the same deploy that stops using it.
13. Renaming a column/table in one step while old code may still run.
14. Changing column type on a large hot table without rewrite/lock analysis.
15. Adding `NOT NULL` to an existing large table in one step.
16. Adding a column with an expensive volatile default to a large table without understanding rewrite/lock behavior.
17. Adding a foreign key to a large table without considering validation strategy.
18. Adding a check constraint to a large table without considering validation strategy.
19. Not using staged `NOT VALID` / `VALIDATE CONSTRAINT` patterns when appropriate.
20. Creating a non-concurrent index on a large live table without a maintenance window.
21. Dropping a large index in a way that creates avoidable blocking.
22. Running huge backfills in one transaction.
23. Running huge deletes in one transaction.
24. Updating every row when only changed rows need updates.
25. Backfilling without batching.
26. Backfilling without progress logging.
27. Backfilling without pause/resume.
28. Backfilling without idempotency.
29. Backfilling without throttling.
30. Backfilling without monitoring replication lag.
31. Backfilling without monitoring locks and vacuum impact.
32. Backfilling without an index supporting the batch predicate.
33. Running backfills through application code that may change before the migration finishes.
34. Using ORM models inside migrations when model definitions can drift.
35. Not testing migrations on production-like volume.
36. Not replaying migrations from scratch in CI.
37. Not testing rollback/forward-fix paths.
38. Using `DROP CASCADE` because the database complained.
39. Dropping constraints to make deploys pass.
40. Leaving half-applied migrations unresolved.
41. Not cleaning up invalid indexes after failed concurrent index creation.
42. No release choreography for expand/contract changes.
43. No compatibility window for rolling deploys.
44. No migration owner watching production during risky changes.

## 10. Postgres-specific footguns that are never justified to ignore

1. Using app roles as superusers.
2. Using schema-owner roles for app traffic.
3. Forgetting default privileges for future tables, sequences, and functions.
4. Forgetting sequence privileges.
5. Assuming `SERIAL`/sequence increments roll back.
6. Assuming sequence values prove creation order under concurrency.
7. Assuming `now` string literals in stored definitions behave like `now()` calls.
8. Using `'tomorrow'::date`, `'today'::date`, or `'now'::timestamp` in stored definitions where a function should be used.
9. Marking table-reading functions `IMMUTABLE`.
10. Marking time-dependent functions `IMMUTABLE`.
11. Marking random or side-effect functions non-volatile.
12. Using `SECURITY DEFINER` without fixed `search_path`.
13. Relying on RLS while app role owns the table or bypasses RLS.
14. Writing RLS policies without `WITH CHECK`.
15. Forgetting that `TRUNCATE` is not subject to row-level security.
16. Leaving invalid indexes after failed concurrent index creation.
17. Creating foreign keys and forgetting supporting indexes.
18. Creating partial indexes and assuming all queries can use them.
19. Creating multicolumn B-tree indexes while ignoring leading-column rules.
20. Creating indexes concurrently inside a transaction block.
21. Running ordinary `CREATE INDEX` on a write-heavy production table.
22. Ignoring `ANALYZE` after large data changes.
23. Disabling autovacuum.
24. Letting long transactions prevent vacuum.
25. Ignoring wraparound warnings.
26. Relying on `search_path` for object resolution in production-critical SQL.
27. Using `ctid` as a permanent identifier.
28. Using physical tuple order as business order.
29. Using table inheritance as partitioning without understanding constraint and FK implications.
30. Using JSONB for frequently updated large documents that could be normalized.
31. Using GIN indexes casually on write-heavy JSONB workloads.
32. Using `text` for everything because Postgres can store it.
33. Using `varchar(n)` limits that have no business meaning.
34. Using `money`/locale-sensitive formatting without understanding behavior.
35. Using `timestamp without time zone` for cross-timezone event instants.
36. Assuming `timestamptz` preserves the original timezone name.
37. Assuming `time with time zone` solves scheduling.
38. Using advisory locks as security controls.
39. Creating extensions in production migrations without privilege, version, and environment checks.
40. Depending on a local extension existing in production without migration-managed installation.
41. Ignoring logical replication identity when tables are replicated and updated/deleted.
42. Running bulk jobs on primary without `statement_timeout`, batching, and monitoring.
43. Letting `idle in transaction` sessions persist.
44. Ignoring lock waits during migrations.
45. Using `DROP CASCADE` instead of understanding dependencies.

## 11. Backup, restore, and disaster-recovery malpractice

1. No backups.
2. “The replica is our backup.”
3. “The dump probably works.”
4. Backups with no restore test.
5. Backups with no owner.
6. Backups with no monitoring.
7. Backups with no retention policy.
8. Backups stored only on the same host.
9. Backups stored only in the same account/project with the same blast radius.
10. Backups not encrypted.
11. Backup keys unavailable during disaster.
12. Backup keys accessible to too many people.
13. No point-in-time recovery plan for critical systems.
14. No WAL archiving where RPO requires it.
15. No documented RPO.
16. No documented RTO.
17. No restore runbook.
18. No regular restore drill.
19. No test of restoring into a clean environment.
20. No test of restoring after accidental table drop.
21. No test of restoring after bad migration.
22. No logical dump strategy where logical restore is needed.
23. No physical backup strategy where fast full restore is needed.
24. Filesystem-level copies taken without a safe database backup method.
25. Backups that silently exclude extensions, roles, privileges, schemas, or large objects.
26. Backups that include secrets and PII but are treated as harmless files.
27. Copying production backups into insecure dev laptops.
28. No anonymization/synthetic-data strategy for lower environments.
29. No verification of backup size/completeness.
30. No alert on failed WAL archiving.
31. No alert on replica lag if replicas are part of recovery.
32. No plan for restoring across major Postgres versions.

## 11. Backup, restore, and operational safety

1. Run production without backups.
2. Run production without testing restores.
3. Assume a backup works because a job says “success.”
4. Store backups in the same failure domain as the primary database.
5. Store unencrypted backups containing sensitive data.
6. Give broad backup access to people or systems that should not have production data.
7. Run destructive migrations before confirming restore/PITR posture.
8. Omit WAL archiving/PITR for systems where point-in-time recovery is required.
9. Fail to define RPO and RTO.
10. Fail to monitor disk space, WAL growth, replication slots, replication lag, locks, long transactions, autovacuum, bloat, errors, and slow queries.
11. Let replication slots fill disks.
12. Let logs fill disks.
13. Use production as the only copy of critical data.
14. Restore production data into development without masking or access controls.
15. Keep data longer than policy permits because backups are unmanaged.
16. Fail to test failover.
17. Fail to test minor/major version upgrades.
18. Run unsupported PostgreSQL versions.
19. Ignore extension compatibility during upgrades.
20. Ignore collation/version drift.
21. Change time zone, locale, encoding, or collation assumptions without impact review.
22. Let cron jobs, ETL, BI, and admin scripts mutate production without the same controls as application code.
23. Have no runbook for corruption, accidental delete, bad migration, bad deploy, credential leak, or disk-full events.
24. Have no owner for database health.

## 11. Community-observed recurring bad behavior

1. **“We don’t use foreign keys because the app handles it.”**
2. **“Foreign keys make inserts slower, so we removed them everywhere.”**
3. **“Everything is JSON because schema changes are annoying.”**
4. **“Everything is EAV because requirements might change.”**
5. **“We have a universal lookup table for all enums.”**
6. **“We store arrays of IDs because joins are hard.”**
7. **“We do deletes manually because cascades are scary,” while leaving orphans.**
8. **“We cascade everything,” while deleting independent business records.**
9. **“We don’t need transactions because each statement succeeds.”**
10. **“We don’t need indexes until it is slow.”**
11. **“We add indexes whenever it is slow,” without measuring.**
12. **“We use `SELECT *` because it is easier.”**
13. **“We use text for all IDs/dates/enums because it is flexible.”**
14. **“We can fix bad data later.”**
15. **“The migration worked locally.”**
16. **“The backup job is green,” without a restore test.**
17. **“The replica is our backup.”**
18. **“The ORM protects us from SQL.”**
19. **“The LLM said this is best practice.”**
20. **“The query is fine because it returns the right result on my sample.”**
21. **“The database is just storage; business logic belongs only in the app.”**
22. **“The database should enforce everything with triggers nobody can see.”**
23. **Correctness:** What invariant is being protected, and is it protected in the database where appropriate?
24. **Security:** Can any user-controlled value become SQL code?
25. **Permissions:** What role runs this, and is it least privilege?
26. **Concurrency:** What happens under simultaneous writes?
27. **Performance:** What is the plan on production-shaped data?
28. **Migration safety:** What locks, rewrites, backfills, and rollback paths exist?
29. **Recovery:** Can we restore after this breaks production?
30. **Observability:** How will we know it is slow, blocked, bloated, wrong, or failing?
31. **AI review:** Did a human verify the actual SQL, not just the explanation?
32. Raw string SQL built from user input.
33. No bind parameters for values.
34. Dynamic identifiers without allow-lists.
35. Application role has superuser or broad DDL privileges.
36. Missing primary key on an important OLTP table.
37. Missing foreign keys for real relationships.
38. Missing `NOT NULL` on required columns.
39. Missing unique constraints for business identity.
40. Money stored as float.
41. Real timestamps stored as ambiguous local/plain timestamps.
42. Core relational data stored as CSV, arrays, or JSONB to avoid schema design.
43. Polymorphic foreign keys with no enforceable integrity.
44. Soft delete added without uniqueness, FK, query, privacy, and archival design.
45. `SELECT *` in app/API code.
46. `UPDATE` or `DELETE` with unsafe/missing `WHERE`.
47. `DISTINCT` used to hide a bad join.
48. N+1 queries in hot paths.
49. Read-modify-write without locking, atomic update, serializable retry, or optimistic locking.
50. Long transactions around network calls or user interaction.
51. No deadlock/serialization retry strategy where needed.
52. Blocking index creation on a hot table.
53. Direct `NOT NULL` or constraint validation on a large hot table with no safe migration pattern.
54. Large backfill in one transaction.
55. No migration timeouts.
56. Destructive migration with no rollback/roll-forward plan.
57. `DROP CASCADE` or `TRUNCATE CASCADE` as a convenience.
58. No backup/restore confidence before destructive work.
59. No representative-data test for performance-sensitive SQL.
60. No authorization/tenant-boundary tests for multi-tenant queries.
61. “Generated by AI” used as a substitute for database review.

## 11. Naming, style, and schema hygiene behavior that is never excusable

1. Quoted uppercase identifiers such as `"User"` or `"Order"`.
2. Spaces, punctuation, or reserved words in identifiers.
3. Mixed naming conventions across the same schema.
4. Ambiguous column names such as `type`, `status`, `data`, `value`, `object_id`, or `name` without context.
5. Inconsistent foreign-key names.
6. Foreign-key columns whose names do not reveal the referenced entity.
7. Tables named after UI screens rather than domain entities.
8. Columns named after temporary product copy.
9. Reusing a column name for a different concept.
10. No comments or documentation for non-obvious tables, columns, constraints, or triggers.
11. No ownership for schemas, tables, jobs, materialized views, and critical queries.
12. Multiple services writing the same table without a clear contract.
13. Hidden coupling through undocumented views, triggers, or functions.

## 12. Privacy, compliance, and data governance

1. Store plaintext passwords.
2. Store reversible passwords.
3. Store API keys, tokens, private keys, or secrets without encryption and rotation design.
4. Put PII, secrets, or regulated data in query logs, app logs, traces, analytics events, prompts, tickets, screenshots, or error messages.
5. Use `SELECT *` in places that return or export sensitive data.
6. Give developers unrestricted production PII by default.
7. Use production data in local development without masking.
8. Export full customer tables for debugging.
9. Send production rows to LLMs or third-party tools without a data-processing agreement and minimization.
10. Create ad-hoc “temporary” copies of sensitive tables and forget them.
11. Fail to classify sensitive columns.
12. Fail to implement retention, deletion, anonymization, or legal hold rules.
13. Treat soft delete as compliance deletion.
14. Treat encryption as a substitute for access control.
15. Treat hashing as anonymization when joins or re-identification remain possible.
16. Store audit logs that can be edited by the same role being audited.
17. Allow cross-tenant queries without strong scoping and review.
18. Build admin search tools that bypass user privacy rules.
19. Replicate sensitive data to systems with weaker controls.
20. Forget that backups and replicas also contain deleted or sensitive data.
21. Use materialized views, caches, search indexes, embeddings, or analytics tables without deletion propagation.

## 12. Production operations and observability negligence

1. No monitoring for database availability.
2. No monitoring for disk space.
3. No monitoring for WAL volume.
4. No monitoring for replication lag.
5. No monitoring for long-running transactions.
6. No monitoring for idle-in-transaction sessions.
7. No monitoring for lock waits.
8. No monitoring for deadlocks.
9. No monitoring for slow queries.
10. No monitoring for failed autovacuum.
11. No monitoring for table/index bloat.
12. No monitoring for connection saturation.
13. No monitoring for checkpoint pressure.
14. No monitoring for temp-file spills.
15. No monitoring for cache hit ratio/context-specific IO health.
16. No monitoring for invalid indexes.
17. No monitoring for failed jobs/backfills.
18. No alerting that actually pages the owner.
19. No runbook for lock pileups.
20. No runbook for disk-full events.
21. No runbook for runaway queries.
22. No runbook for failed migrations.
23. No runbook for replication breakage.
24. No runbook for restoring backups.
25. No ownership of database health.
26. No regular review of top queries.
27. No regular review of unused/duplicate indexes.
28. No regular review of table growth.
29. No capacity planning.
30. Treating Postgres config as folklore.
31. Cargo-culting `work_mem`, `shared_buffers`, `max_connections`, or autovacuum settings.
32. Increasing `max_connections` instead of using pooling.
33. Opening a new database connection per request.
34. Letting serverless functions stampede the database with connections.
35. No connection pool.
36. Pool size far above actual database capacity.
37. Pooling without transaction/session-mode compatibility review.
38. Assuming read replicas are current.
39. Serving stale replica reads for read-your-writes flows.
40. Using production as the first realistic load test.
41. Letting ad hoc analytics queries run against primary OLTP.
42. Letting dashboards perform full scans every refresh.
43. Letting cron jobs pile up overlapping executions.
44. No kill policy for runaway queries.
45. No `statement_timeout` policy.
46. No `idle_in_transaction_session_timeout` policy.
47. No separation between OLTP, analytics, migrations, and maintenance workloads.

## 13. ORM and application-layer database behavior

1. Assume the ORM prevents SQL injection in raw queries.
2. Accept ORM-generated migrations blindly.
3. Accept ORM-default column types blindly.
4. Let the ORM create tables without reviewing constraints and indexes.
5. Use app-side validation instead of database constraints.
6. Hide database constraint errors instead of mapping them to domain errors.
7. Use lazy loading that creates N+1 queries in production paths.
8. Use per-row queries where bulk SQL is required.
9. Use bulk SQL where per-row validation or batching is required.
10. Open transactions around web requests without understanding lifecycle.
11. Leak connections.
12. Disable pooling safeguards.
13. Use the same transaction for unrelated operations.
14. Store serialized application objects in a column as a substitute for schema.
15. Let application code assume rows exist without foreign keys or constraints.
16. Let background workers and web requests race on the same rows without locking/idempotency.
17. Use retries without idempotency keys.
18. Swallow database errors and continue.
19. Convert all database errors to generic 500s without preserving safe diagnostics.
20. Treat migrations as application code but exempt them from tests.
21. Treat database performance as something the ORM will solve later.

## 14. Application/database boundary mistakes

1. Putting all validation in application code.
2. Putting all authorization in application code when database-level boundaries are needed.
3. Putting all uniqueness checks in application code.
4. Putting all referential integrity in application code.
5. Doing joins in application memory for core relational flows.
6. Loading entire tables into memory to filter.
7. N+1 queries in hot paths.
8. Per-row SQL inside loops when set-based SQL is appropriate.
9. Per-row commits in bulk jobs.
10. One giant transaction for unrelated user-visible work.
11. ORM models that hide dangerous queries.
12. ORM cascades that delete more than expected.
13. ORM migrations accepted without review.
14. Letting the ORM choose all types/defaults/indexes blindly.
15. Treating ORM validations as substitutes for constraints.
16. Treating ORM associations as substitutes for foreign keys.
17. Ignoring generated SQL because “the ORM handles it.”
18. Using database triggers for hidden business logic with no tests/docs.
19. Avoiding triggers categorically where database-side enforcement is the correct tool.
20. Having business invariants split unpredictably across app code, triggers, jobs, and stored procedures.
21. No single documented source of truth for each invariant.
22. Caching query results without invalidation.
23. Caching authorization decisions without tenant/security invalidation.
24. Writing to cache and database non-atomically where consistency matters.
25. Implementing idempotency in app memory instead of durable keys.
26. Retrying writes without idempotency keys.

## 14. Reporting, analytics, and ad-hoc SQL abuse

1. Let analysts run arbitrary heavy queries on the primary OLTP database during business hours.
2. Give analytics tools production write privileges.
3. Let dashboards issue expensive joins, full scans, or unbounded time ranges repeatedly.
4. Export entire tables when a filtered extract is sufficient.
5. Run reports without time filters on append-only event tables.
6. Join production PII into broad analytics datasets without purpose limitation.
7. Use replica queries without monitoring lag and replica load.
8. Assume read replicas are free capacity.
9. Let BI users create temp tables or functions in shared schemas without controls.
10. Use production SQL consoles with no audit trail.
11. Run “one quick query” that updates or deletes data without a transaction, preview, and rollback plan.
12. Paste SQL from Slack, Reddit, Stack Overflow, GitHub issues, or an AI chat directly into production.
13. Debug by changing production data manually.
14. Create permanent “temporary” tables for analysis.
15. Leave analysis indexes, helper columns, or debug triggers in production.

## 14. Specific PostgreSQL “don’t do this” items to bake into reviews

1. `NOT IN` when `NULL` can appear.
2. Uppercase or quoted identifiers.
3. `BETWEEN` for timestamp ranges.
4. `timestamp without time zone` for real instants.
5. `timetz`.
6. `current_time`.
7. `timestamp(0)`.
8. Storing timezone offsets as text.
9. `char(n)`.
10. Casual `varchar(n)`.
11. Casual `money`.
12. New use of `serial` where identity columns are preferred.
13. `trust` authentication over TCP/IP.
14. SQL rules.
15. Table inheritance for ordinary modeling.
16. `psql -W` in scripts.
17. `SQL_ASCII`.

## 15. Code-review “hard reject” summary

1. Makes invalid states representable when the database could prevent them.
2. Moves core integrity exclusively into application code.
3. Uses string-built SQL with untrusted input.
4. Uses superuser/owner privileges for normal application work.
5. Stores secrets or passwords improperly.
6. Uses weak, ambiguous, or fake data types.
7. Treats timezones casually.
8. Hides relational data inside text, arrays, or JSON without a good reason.
9. Adds large-table DDL without lock analysis.
10. Adds large data changes without batching.
11. Ships queries without plan review.
12. Ships migrations without rollback or forward-fix strategy.
13. Skips production-scale testing.
14. Ignores backup and restore.
15. Lets AI-generated SQL bypass normal engineering review.

## 15. Naming, documentation, and maintainability failures

1. Use names that hide meaning: `tbl1`, `new_table`, `data`, `value`, `thing`, `object`, `flag`, `misc`.
2. Use inconsistent singular/plural naming across the schema.
3. Use inconsistent suffixes for IDs, timestamps, booleans, and foreign keys.
4. Use `created`, `created_at`, `creation_time`, and `inserted_on` interchangeably.
5. Store timestamps without documenting whether they are event time, ingestion time, update time, or processing time.
6. Create columns whose valid values are known only to one service.
7. Create triggers/functions/policies without comments when behavior is non-obvious.
8. Use magic numbers or strings instead of lookup tables, constraints, or documented enums.
9. Fail to document ownership of tables.
10. Fail to document data lifecycle.
11. Fail to document migration ordering requirements.
12. Fail to document manual operational procedures.
13. Leave dead tables, dead columns, dead functions, dead triggers, and dead indexes indefinitely.
14. Allow schema drift between environments.
15. Allow undocumented database changes outside pull requests.
16. Make schema review optional for schema changes.

## 15. Privacy, compliance, and sensitive-data failures

1. Production dumps on laptops.
2. Production PII in local dev.
3. Production PII in staging without masking or access controls.
4. Sensitive data in query logs.
5. Sensitive data in error logs.
6. Sensitive data in slow-query logs.
7. Sensitive data in analytics tables without classification.
8. Sensitive data in materialized views forgotten by deletion/export systems.
9. Sensitive data in backups with weaker controls than production.
10. Sensitive data in screenshots, tickets, Slack messages, or GitHub issues.
11. No data-retention policy.
12. No deletion/anonymization strategy.
13. No audit trail for sensitive reads where required.
14. No audit trail for sensitive writes where required.
15. No classification of tables/columns by sensitivity.
16. No column-level access strategy for highly sensitive fields.
17. Giving BI tools access to raw sensitive tables.
18. Storing passwords, tokens, SSNs, or payment data without domain-specific controls.
19. Encrypting data but leaving keys next to the data.
20. Hashing low-entropy sensitive values without salt/pepper/rate-limit strategy.
21. Using reversible encryption where one-way hashing is required.
22. Using one-way hashing where lookup/recovery requirements actually need a designed tokenization/encryption approach.
23. Forgetting that deleted rows may remain in backups, replicas, logs, queues, and caches.

## 16. Replication, sharding, partitioning, and scale mistakes

1. Adding read replicas without understanding replication lag.
2. Reading from replicas in flows that require read-your-writes.
3. Running heavy reads on replicas without monitoring lag and conflicts.
4. Treating replicas as backups.
5. Creating logical replication without conflict/DDL/version planning.
6. Creating replication slots without monitoring disk growth.
7. Partitioning without a pruning strategy.
8. Partitioning without indexes/constraints on partitions.
9. Partitioning without lifecycle management.
10. Partitioning by the wrong key.
11. Over-partitioning into thousands of tiny partitions without evidence.
12. Under-partitioning and then being unable to manage retention.
13. Sharding before the data model is correct.
14. Sharding without a clear shard key.
15. Sharding without cross-shard transaction strategy.
16. Sharding without tenant/data rebalancing strategy.
17. Sharding without operational ownership.
18. Using globally unique IDs but no locality/tenant strategy.
19. Ignoring sequence/ID generation under replication or multi-writer setups.
20. Assuming distributed SQL removes the need for constraints, transactions, and query plans.
21. Assuming queue/eventual-consistency systems make database correctness less important.

## 16. “Looks clever, is dangerous” SQL behavior

1. Use clever SQL no one on the team can maintain in a critical path.
2. Use recursive queries without cycle protection.
3. Use triggers that call external services.
4. Use database functions with hidden side effects in ordinary `SELECT` queries.
5. Use volatile functions in indexes or constraints incorrectly.
6. Use random ordering or sampling in billing, compliance, or audit logic.
7. Use approximate counts or approximate distinct values where exact values are legally or financially required.
8. Use exact counting where approximate/cached values are required for system survival, without documenting the tradeoff.
9. Use time-based job logic that ignores DST, leap seconds, local calendars, holidays, or tenant time zones when those matter.
10. Use `now()`/transaction timestamp semantics without understanding whether statement time, transaction time, or clock time is required.
11. Use scheduled jobs that assume they run exactly once.
12. Use sequences as proof that no insert rolled back or that IDs are gapless.
13. Require gapless IDs from ordinary sequences.
14. Use database-generated IDs as security tokens.
15. Expose sequential IDs when enumeration is a security or privacy problem.
16. Use `LIMIT 1` without deterministic `ORDER BY`.
17. Use “latest row” logic without a tie-breaker.
18. Use background cleanup to enforce invariants after users have already seen bad data.
19. Use materialized views as source of truth without refresh correctness.
20. Use caches as source of truth.
21. Use database locks as application feature flags without observability and timeout.

## 17. Community-backed PostgreSQL pet peeves that should be treated seriously

1. Copy MySQL, SQL Server, SQLite, or Oracle habits into PostgreSQL without checking PostgreSQL behavior.
2. Add an auto-increment `id` to every table while ignoring the actual key.
3. Defend broken soft-delete behavior as “normal.”
4. Ignore PostgreSQL-specific guidance because generic SQL advice said otherwise.
5. Treat Reddit, Stack Overflow, blog snippets, or AI output as authoritative without checking PostgreSQL docs.
6. Use a pattern because it is fashionable rather than because it fits the workload and invariants.

## 17. Dangerous DDL/DML habits

1. `DROP TABLE` in production without explicit reviewed migration and backup/restore confidence.
2. `DROP COLUMN` without proving no running code uses it.
3. `DROP INDEX` without workload review.
4. `DROP CONSTRAINT` to make bad data load.
5. `TRUNCATE` without understanding locks, cascades, identity reset, and rollback context.
6. `DELETE` millions of rows in one transaction without batching/vacuum plan.
7. `UPDATE` millions of rows in one transaction without batching/vacuum plan.
8. `ALTER TABLE` on hot paths without lock analysis.
9. `ALTER TYPE` or enum changes without deploy compatibility review.
10. `REINDEX`/`CLUSTER`/`VACUUM FULL` without lock/downtime review.
11. `CREATE EXTENSION` in production without security/backup/upgrade review.
12. Installing untrusted extensions because a blog post said so.
13. Disabling triggers to “fix” a migration.
14. Disabling RLS to “fix” an import.
15. Disabling FK checks and never validating.
16. Using `CASCADE` as a reflex.
17. Running DDL from a GUI with no migration artifact.
18. Running SQL from shell history with production credentials.
19. Running copied SQL against the wrong database.
20. No visible environment indicator in SQL consoles.
21. No production write guardrails.

## 18. Naming, documentation, and schema-governance failures

1. No naming convention for tables.
2. No naming convention for columns.
3. No naming convention for indexes.
4. No naming convention for constraints.
5. No naming convention for foreign keys.
6. Inconsistent singular/plural table names across the schema.
7. Inconsistent ID names: `id`, `user_id`, `uid`, `userId`, `owner`.
8. Same concept represented by different types in different tables.
9. Same status represented by different strings in different tables.
10. Ambiguous timestamp columns like `date`, `time`, `timestamp`.
11. Ambiguous user columns like `user_id`, `owner_id`, `created_by`, `actor_id` with no semantics.
12. No comments/docs for non-obvious constraints.
13. No docs for lifecycle columns.
14. No docs for soft-delete semantics.
15. No docs for tenant ownership.
16. No docs for time-zone policy.
17. No docs for money/currency policy.
18. No docs for PII/sensitive columns.
19. No docs for generated/cached columns.
20. No docs for materialized view refresh semantics.
21. No ADR for denormalization.
22. No ADR for JSONB-heavy design.
23. No ADR for partitioning/sharding.
24. No schema review checklist.
25. No database owner for each service/domain.
26. Hiding schema changes inside giant app PRs.
27. Merging database changes without database review.
28. No migration linting.
29. No SQL linting.
30. No static analysis for dangerous SQL patterns.

## 18. The “feature is okay, abuse is not” list

1. **Raw SQL** is fine; raw SQL with interpolation is not.
2. **Dynamic SQL** is fine for controlled administrative code; dynamic SQL from untrusted input is not.
3. **JSONB** is fine for semi-structured data; JSONB as a substitute for relational integrity is not.
4. **Soft delete** is fine for some audit/lifecycle models; soft delete that breaks uniqueness, FKs, privacy deletion, or query correctness is not.
5. **Denormalization** is fine when measured; denormalization without consistency rules is not.
6. **Triggers** are fine for carefully documented database-side behavior; invisible business logic nobody tests is not.
7. **Stored procedures** are fine; procedures that concatenate unsafe SQL or bypass review are not.
8. **Materialized views** are fine; stale materialized views used as truth without refresh semantics are not.
9. **Caches** are fine; caches as the only source of truth are not.
10. **Read replicas** are fine; ignoring lag and consistency is not.
11. **Partitioning** is fine; table-per-month/customer hacks are not.
12. **UUIDs** are fine; using IDs as secrets is not.
13. **ORMs** are fine; assuming the ORM replaces schema design is not.
14. **AI assistance** is fine; AI autonomy over production data is not.

## 2. Authentication, roles, privileges, and RLS

1. Use `trust` authentication over TCP/IP in production.
2. Expose Postgres directly to the internet with permissive `pg_hba.conf` rules.
3. Let the application connect as `postgres`, a superuser, database owner, schema owner, migration role, or replication role.
4. Give the runtime app role DDL privileges.
5. Give read-only services write privileges.
6. Give BI/reporting users write, DDL, owner, or broad production privileges.
7. Use one shared database credential across apps, humans, jobs, CI, and migrations.
8. Leave default or stale credentials active.
9. Store database passwords in repos, prompts, tickets, logs, screenshots, shell history, or notebooks.
10. Grant broad privileges to `PUBLIC`.
11. Leave function execution broadly available when the function performs privileged operations.
12. Depend on network location alone as the security boundary.
13. Rely only on application-side tenant filters for sensitive multi-tenant data.
14. Give an app role `BYPASSRLS`.
15. Disable RLS “temporarily” in production without a controlled maintenance window and verification.
16. Create permissive RLS policies that accidentally allow all rows.
17. Use `SECURITY DEFINER` functions without a locked-down `search_path`, schema-qualified object references, and restricted `EXECUTE` grants. PostgreSQL specifically warns to exclude writable schemas from `search_path`, put `pg_temp` last, and revoke public execution where needed.
18. Put security-sensitive functions in writable schemas.
19. Let untrusted users create objects in schemas that privileged code searches.
20. Use ownership as an access-control model for applications.
21. Run migrations as the same role used by the web app.
22. Let AI tools, background jobs, or ETL systems share the web app’s credential.
23. Fail to audit DDL, privilege changes, security policy changes, and mass writes.
24. Ignore TLS/encryption requirements for untrusted networks.
25. Treat backups, replicas, logs, query samples, and data exports as less sensitive than the primary database.

## 20. “Not automatically bad” tools that become inexcusable when cargo-culted

1. JSONB is fine for flexible/semi-structured data; it is bad when used to avoid schema design for core relational facts.
2. Soft deletes are fine with a full lifecycle, uniqueness, FK, retention, and query strategy; they are bad as a random `deleted_at` column slapped everywhere.
3. Denormalization is fine for measured read performance; it is bad without source-of-truth, invalidation, and reconciliation.
4. Materialized views are fine; they are bad without refresh semantics and staleness guarantees.
5. Triggers are fine for database-owned invariants/auditing; they are bad as hidden, undocumented business logic.
6. Stored procedures are fine; they are bad when they hide privilege escalation, unsafe dynamic SQL, or untestable behavior.
7. Raw SQL is fine; it is bad when unparameterized, unreviewed, or dialect-misunderstood.
8. ORMs are fine; they are bad when treated as a substitute for understanding SQL.
9. UUIDs are fine; they are bad when write locality/index/storage consequences are ignored.
10. Natural keys are fine for truly stable identifiers; they are bad when mutable/PII values become primary keys by accident.
11. Enums are fine for stable finite domains; they are bad for fast-changing product taxonomy without migration planning.
12. Partitioning is fine; it is bad when added before query/retention/access patterns are understood.
13. Read replicas are fine; they are bad when used where fresh reads are required.
14. Caching is fine; it is bad when it becomes the source of truth.
15. `SELECT *` is fine for ad hoc exploration; it is bad in durable application contracts.
16. Temporary tables are fine; they are bad when used to smuggle unbounded production work into request paths.
17. Index hints/workarounds are sometimes useful in other databases; in Postgres, fighting the planner without statistics/query evidence is bad engineering.
18. “NoSQL-style” flexible attributes are fine for genuinely unknowable sparse metadata; they are bad for known business entities.
19. Application validation is fine; it is bad when it replaces database constraints for durable facts.
20. Background jobs are fine; they are bad when they create race conditions or inconsistent invariants.
21. Manual admin scripts are fine; they are bad without review, dry run, transaction strategy, and audit trail.
22. Concatenating, interpolating, f-stringing, template-stringing, or string-formatting untrusted values into SQL.
23. Treating “it came from the frontend,” “it came from an admin,” “it came from a webhook,” or “it came from an LLM” as trusted input.
24. Building `WHERE`, `ORDER BY`, `LIMIT`, `OFFSET`, column names, table names, sort directions, operators, schema names, or function names directly from request parameters.
25. Using escaping as the primary defense instead of bind parameters for values.
26. Quoting bind parameters inside SQL text, for example `WHERE id = '$1'`, which turns a parameter into text rather than a parameter.
27. Allowing users to submit raw SQL fragments, raw filters, raw expressions, raw JSONPath/regex predicates, or arbitrary “advanced search syntax” that compiles directly into SQL.
28. Letting generated SQL from an AI tool run against a real database before review, parameterization, tests, and an explicit migration/query plan.
29. Allowing multiple SQL statements in a user-driven query path when only one statement is needed.
30. Dynamically choosing identifiers without an allow-list. Parameterization protects values; identifiers require strict allow-listing and identifier quoting.
31. Writing dynamic SQL inside PL/pgSQL with unsafe `EXECUTE` string concatenation.
32. Using `SECURITY DEFINER` functions without a secure `search_path`.
33. Logging fully interpolated SQL statements that include passwords, tokens, session IDs, personal data, or secrets.
34. “Fixing” injection risk by regex-stripping bad characters.
35. “Fixing” injection risk by blacklisting keywords like `DROP`, `UNION`, or `--`.
36. Assuming an ORM automatically protects raw SQL fragments.
37. Assuming GraphQL, REST, RPC, or internal admin routes are not injection surfaces.
38. Passing comma-joined ID lists into SQL instead of using proper array parameters or temporary/staging tables.
39. Using `LIKE '%user_input%'` without escaping wildcard semantics when the user input is meant to be literal text.
40. Using database superuser credentials in a query path that also accepts external input.
41. Using `trust` authentication over TCP/IP in production.
42. Allowing application code to connect as `postgres`, a superuser, table owner, migration owner, or DBA role.
43. Giving the application role `CREATE`, `DROP`, `ALTER`, `TRUNCATE`, `BYPASSRLS`, `SUPERUSER`, `CREATEDB`, `CREATEROLE`, or replication privileges without a documented need.
44. Using one shared database account for humans, services, jobs, analytics, and migrations.
45. Reusing the same database credentials across dev, staging, test, and production.
46. Storing database credentials in source code, migrations, notebooks, committed `.env` files, Docker images, Slack messages, tickets, or SQL comments.
47. Letting app logs, query logs, or error reports expose credentials, tokens, API keys, session IDs, reset tokens, or PII.
48. Connecting to the database without TLS where the network is not fully trusted.
49. Using weak or deprecated password authentication methods when stronger methods are available.
50. Leaving default accounts, default passwords, unused roles, or stale contractor/employee access active.
51. Granting broad `SELECT` on all tables to app roles, BI tools, notebooks, or support tooling.
52. Granting write permissions to analytics/reporting users.
53. Making the app role own the schema it queries.
54. Failing to revoke unnecessary privileges from `PUBLIC`.
55. Installing arbitrary extensions or procedural languages in production without security review.
56. Treating “internal network” as authorization.
57. Exposing Postgres directly to the public internet.
58. Using RLS but connecting as a table owner/superuser or a role with `BYPASSRLS`, thereby bypassing the policy.
59. Using RLS without tests that prove cross-tenant reads and writes fail.
60. Assuming views, functions, triggers, or ORMs automatically enforce least privilege.
61. Running migrations with production data from a developer laptop using personal superuser credentials.
62. Storing passwords in plaintext.
63. Storing passwords with reversible encryption so support staff or developers can “recover” them.
64. Hashing passwords with MD5, SHA-1, SHA-256, SHA-512, or any fast general-purpose hash.
65. Hashing passwords without unique salts.
66. Using one global salt.
67. Storing password reset tokens, magic login links, API tokens, OAuth tokens, MFA backup codes, or session tokens in plaintext.
68. Keeping expired reset/session tokens forever.
69. Letting application users query password/token columns unnecessarily.
70. Replicating sensitive columns into analytics databases without masking or purpose limitation.
71. Including sensitive values in audit logs, SQL logs, dead-letter queues, exception telemetry, or BI extracts.
72. Using production personal data in dev/test without masking and access controls.
73. Designing tables with no retention/deletion/anonymization strategy for regulated or sensitive data.
74. “Soft deleting” sensitive data while continuing to expose it through joins, search indexes, materialized views, caches, or replicas.
75. Tables without primary keys.
76. Tables with duplicate rows that cannot be distinguished.
77. Business-unique facts without `UNIQUE` constraints.
78. Required attributes without `NOT NULL`.
79. Domain rules enforced only in application code when the database can enforce them.
80. Missing `CHECK` constraints for obvious invariants: positive quantities, valid percentages, nonnegative balances where required, start date before end date, nonempty strings, allowed ranges, valid status transitions where modelable.
81. Missing foreign keys for real relationships.
82. Using “foreign key by convention” names such as `user_id` without an actual FK.
83. Allowing orphan rows.
84. Allowing child rows to point to deleted, hidden, wrong-tenant, or wrong-type parents.
85. Using nullable foreign keys when the relationship is mandatory.
86. Using `ON DELETE CASCADE` without explicit business approval of every cascaded deletion.
87. Using `ON DELETE SET NULL` on data that must retain accountability.
88. Using app code, cron jobs, or “cleanup scripts” to restore referential integrity after the fact.
89. Creating polymorphic associations like `(entity_type, entity_id)` where the database cannot enforce valid parents.
90. Using generic relationship tables that can link anything to anything without constraints.
91. Encoding multiple states with contradictory booleans, for example `is_active`, `is_deleted`, `is_archived`, `is_pending`, `is_cancelled`.
92. Allowing impossible status combinations because no state machine or constraint exists.
93. Using magic sentinel values such as `0`, `-1`, `9999-12-31`, `'unknown'`, or empty string instead of a clearly modeled `NULL`, separate status, or proper domain value.
94. Treating `NULL`, empty string, zero, missing JSON key, and “unknown” as interchangeable.
95. Duplicating the same fact in multiple tables without a declared source of truth and reconciliation mechanism.
96. Storing derived totals as authoritative values without constraints, refresh logic, or consistency checks.
97. Designing schemas where insert/update/delete anomalies are inevitable.
98. Storing comma-separated IDs in a text column.
99. Storing pipe-separated, JSON-stringified, or newline-separated lists in scalar columns.
100. Storing multiple values in columns like `phone1`, `phone2`, `phone3`.
101. Creating columns like `jan_sales`, `feb_sales`, `mar_sales` instead of rows.
102. Creating one table per month, year, customer, tenant, status, or region instead of using rows, partitioning, or a clear physical design.
103. Encoding metadata in table names or column names.
104. Using Entity-Attribute-Value for known attributes.
105. Using JSONB as an excuse to avoid modeling stable entities and relationships.
106. Using arrays of foreign keys where relationship rows are required.
107. Using a spreadsheet import as the final schema.
108. Designing one “god table” with hundreds of unrelated columns.
109. Designing one “god JSONB” column with no constraints, generated columns, indexes, or schema validation.
110. Designing generic `objects`, `attributes`, `relationships`, or `events` tables for everything when concrete entities are known.
111. Mixing multiple entity types in one table without a discriminator, constraints, and type-specific validity rules.
112. Using names, emails, slugs, labels, or mutable business codes as permanent references when they can change.
113. Failing to model many-to-many relationships with intersection tables.
114. Failing to model history for facts that must be audited historically.
115. Overwriting time-varying facts when the business requires historical truth.
116. Deleting records that represent legal, financial, security, or audit events.
117. Treating soft delete as a universal answer while breaking uniqueness, FKs, counts, analytics, and retention.
118. Storing “current state” only when the system needs both current state and state transitions.
119. Mixing event logs and current-state tables without clear ownership.
120. Designing tables around current frontend screens rather than durable business concepts.
121. Using surrogate IDs as a substitute for real uniqueness constraints.
122. Using natural keys dogmatically when the natural key is mutable or not truly unique.
123. Failing to document cardinality: one-to-one, one-to-many, many-to-many, optional, mandatory.
124. Using `float`, `real`, or `double precision` for money, balances, prices, counts, inventory, tax, accounting, measurements requiring exactness, or anything that must reconcile exactly.
125. Using PostgreSQL `money` as the default money type instead of modeling amount and currency deliberately.
126. Storing money without currency.
127. Storing units without the unit: meters vs feet, cents vs dollars, kilograms vs pounds.
128. Storing dates/timestamps as text.
129. Storing actual points in time as `timestamp without time zone` instead of `timestamptz`.
130. Storing local future appointments without preserving the intended time zone or locale rules.
131. Storing timezone names as raw offsets like `-07:00`.
132. Using `timetz`.
133. Using `CURRENT_TIME` for application timestamps.
134. Using `timestamp(0)` / `timestamptz(0)` to “remove milliseconds,” because PostgreSQL rounds rather than truncates.
135. Using `BETWEEN` for timestamp ranges, because it is inclusive on both ends and commonly creates boundary bugs.
136. Using `char(n)` for ordinary strings.
137. Spraying `varchar(255)` everywhere without a real domain reason.
138. Using `text` where a stricter type/domain is required and no `CHECK` exists.
139. Using `SQL_ASCII` encoding.
140. Using uppercase or mixed-case quoted identifiers.
141. Using reserved words as table or column names.
142. Using `serial` for new designs when identity columns are the modern SQL-standard option.
143. Storing booleans as `'Y'/'N'`, `'true'/'false'`, `0/1`, or `'active'/'inactive'` without a compelling compatibility reason and constraints.
144. Storing UUIDs as text.
145. Storing IP addresses as text instead of `inet`/`cidr` where appropriate.
146. Storing JSON as text.
147. Storing numeric strings as text because “the frontend sends strings.”
148. Using unconstrained `numeric` where scale and precision matter.
149. Using `integer` for values that can realistically exceed 2.1 billion.
150. Using `bigint` everywhere as a substitute for capacity thinking.
151. Assuming sequence gaps mean missing data or need to be “fixed.”
152. Designing IDs that must be gapless except where legally required and explicitly serialized.
153. Using `NULL`-unsafe types or defaults without documenting semantics.
154. Relying on physical row order without `ORDER BY`.
155. Using `LIMIT` without a deterministic `ORDER BY`.
156. Paginating mutable data with unstable ordering.
157. Using `OFFSET` pagination for correctness-sensitive workflows without understanding skipped/duplicated rows under concurrent changes.
158. Using `SELECT *` in application code, APIs, views intended as contracts, ETL, migrations, or inserts.
159. Using `INSERT INTO table VALUES (...)` without naming columns.
160. Relying on column ordinal positions.
161. Using `NOT IN` against a subquery or list that can contain `NULL`.
162. Using `= NULL` or `<> NULL` instead of `IS NULL` / `IS NOT NULL`.
163. Using `DISTINCT` to hide duplicate rows caused by a wrong join.
164. Using `GROUP BY` at the wrong grain.
165. Aggregating after joining to a one-to-many table and accidentally double-counting.
166. Joining multiple one-to-many relationships before aggregation without pre-aggregating.
167. Counting rows where the business asks for distinct users/orders/accounts.
168. Writing accidental cross joins.
169. Using old comma joins in complex queries where join predicates are easy to miss.
170. Putting filters on the nullable side of an outer join in `WHERE` and accidentally turning it into an inner join.
171. Using `OR` conditions that change intended null semantics.
172. Mixing `WHERE` and `HAVING` without understanding evaluation order.
173. Comparing timestamps/dates across time zones without explicit semantics.
174. Using text sorting for numeric values.
175. Using case-sensitive comparisons when the business expects case-insensitive uniqueness or lookup.
176. Using case-insensitive comparisons without matching indexes or collations.
177. Using `LIKE` / `ILIKE` as a fake search engine for large text search.
178. Using leading wildcard searches like `ILIKE '%term%'` on large tables without trigram/full-text indexes.
179. Using `ORDER BY random()` on large tables.
180. Using functions on indexed columns in predicates without expression indexes, for example `WHERE date(created_at) = ...`.
181. Hiding broken logic in giant “spaghetti queries.”
182. Copy-pasting generated SQL without proving row counts and edge cases.
183. Failing to test empty input, duplicate input, null input, timezone boundaries, daylight-saving transitions, and concurrent changes.
184. Ignoring `EXPLAIN` / `EXPLAIN ANALYZE` for important queries.
185. Optimizing based on intuition instead of actual plans and data distribution.
186. Performing a multi-step business operation outside a transaction.
187. Using autocommit for a logical unit that must be atomic.
188. Doing read-modify-write without a constraint, lock, atomic update, or serializable transaction.
189. Assuming `READ COMMITTED` prevents race conditions.
190. Assuming `REPEATABLE READ` or `SERIALIZABLE` removes the need to handle retries.
191. Failing to retry serialization failures and deadlocks.
192. Generating business sequence numbers with `SELECT max(id) + 1`.
193. Checking if a row exists and then inserting without a unique constraint.
194. Checking a balance/inventory/capacity and then updating without locking or atomic conditions.
195. Holding transactions open while calling external APIs.
196. Holding transactions open during user interaction.
197. Leaving sessions `idle in transaction`.
198. Running long reports in transactions that prevent vacuum cleanup.
199. Locking tables when row-level locks would do.
200. Locking rows in inconsistent order across code paths.
201. Using advisory locks without namespacing, timeouts, or release discipline.
202. Using `SELECT ... FOR UPDATE` everywhere as a cargo-cult fix.
203. Not setting reasonable `statement_timeout`, `lock_timeout`, and `idle_in_transaction_session_timeout`.
204. Retrying failed transactions without idempotency.
205. Running migrations or batch jobs that fight OLTP traffic for locks.
206. Assuming replica reads are fresh enough for read-after-write behavior.
207. Using background jobs that process the same rows concurrently without `FOR UPDATE SKIP LOCKED`, status transitions, idempotency, or uniqueness guards.
208. Ignoring isolation-level documentation because “the ORM handles transactions.”
209. Adding no index for high-traffic joins, filters, uniqueness checks, or lookup paths.
210. Forgetting indexes on foreign-key referencing columns when parent updates/deletes or child lookups matter.
211. Adding indexes to every column.
212. Adding indexes because “indexes make queries faster” without measuring write cost and disk cost.
213. Creating duplicate or redundant indexes.
214. Creating wide indexes that bloat storage and slow writes.
215. Creating the wrong column order in composite indexes.
216. Creating indexes that do not match actual predicates.
217. Creating expression queries without expression indexes.
218. Creating partial-index opportunities but using huge global indexes instead.
219. Adding an index that helps one rare query and hurts frequent writes.
220. Adding indexes without `EXPLAIN ANALYZE` before and after.
221. Assuming an index will be used.
222. Ignoring row counts, selectivity, statistics, correlation, and data distribution.
223. Ignoring stale planner statistics.
224. Failing to `ANALYZE` after large loads or major data changes.
225. Leaving invalid indexes after failed concurrent builds.
226. Creating normal blocking indexes on large production tables when concurrent index creation is required.
227. Running `CREATE INDEX CONCURRENTLY` inside a transaction block.
228. Creating a concurrent unique index and ignoring the failure mode where an invalid index may still enforce uniqueness.
229. Keeping unused indexes forever.
230. Using indexes to compensate for a broken schema or broken query shape.
231. Ignoring GIN/GiST/BRIN maintenance costs and fit.
232. Building indexes on low-cardinality columns without a plan.
233. Indexing encrypted/randomized columns where the index cannot serve the query.
234. Not monitoring slow queries, lock waits, index usage, and table/index bloat.
235. Making manual production schema changes outside versioned migrations.
236. Running migrations that were never tested on production-like data volume.
237. Running migrations without a rollback/roll-forward plan.
238. Running migrations without knowing the lock level.
239. Running `ALTER TABLE` on a large production table without assessing locks and rewrite behavior.
240. Adding a `NOT NULL` column before backfill and application compatibility.
241. Adding a heavy default, type change, or rewrite-prone operation without checking the PostgreSQL version and table size.
242. Renaming or dropping columns in the same release that removes app usage.
243. Dropping tables/columns/indexes/constraints before proving no code path uses them.
244. Changing column types in place on large tables without an online strategy.
245. Adding constraints without backfilling and validating safely.
246. Adding foreign keys without considering locks, validation, and supporting indexes.
247. Creating non-concurrent indexes on large hot production tables.
248. Running huge backfills in one transaction.
249. Running huge deletes/updates without batching.
250. Running data migrations in request paths or deploy hooks that must complete quickly.
251. Mixing irreversible data destruction with routine deploys.
252. Depending on application models/classes inside migrations that may change later.
253. Writing migrations that assume exact current production data shape without guards.
254. Making migrations non-idempotent where retries are possible.
255. Not setting `lock_timeout` and `statement_timeout` for risky migrations.
256. Ignoring replicas, logical decoding, replication lag, and downstream consumers.
257. Not considering materialized views, generated columns, triggers, partitions, and dependent views.
258. Using `DROP ... CASCADE` as a convenience.
259. Disabling triggers or constraints for “just this import” without reconciliation and validation.
260. Changing primary-key types after growth without a tested bigint/UUID migration path.
261. Running DDL while long transactions are open.
262. Letting AI generate migrations and committing them unread.
263. Disabling autovacuum globally.
264. Disabling autovacuum on hot tables without a replacement plan.
265. Treating vacuum as optional.
266. Ignoring dead tuples and bloat.
267. Running `VACUUM FULL` casually on production tables.
268. Not understanding that `VACUUM FULL` takes an exclusive lock.
269. Ignoring transaction ID wraparound risk.
270. Leaving long-running transactions, old prepared transactions, or abandoned replication slots that prevent cleanup.
271. Never running or relying on `ANALYZE` after large data changes.
272. Ignoring partitioned/foreign table statistics needs.
273. Allowing high-churn tables to grow without fillfactor/autovacuum/index strategy.
274. Not monitoring autovacuum activity, dead tuples, table age, and bloat.
275. Treating “the query got slow” as mysterious when stats and bloat were never maintained.
276. Resetting statistics or changing maintenance settings without understanding consequences.
277. Running maintenance jobs at peak load without throttling.
278. Using table rewrites as routine cleanup.
279. No automated backups.
280. No point-in-time recovery strategy for important production data.
281. No restore testing.
282. No documented RPO/RTO.
283. No backup monitoring or alerting.
284. No offsite/cross-region backup where required.
285. Backups stored with the same credentials/security boundary as production.
286. Unencrypted backups containing sensitive data.
287. Backups readable by developers who should not read production data.
288. No retention policy.
289. No deletion policy for data that legally must be removed.
290. No tested procedure for restoring one table, one tenant, or one accidentally deleted account.
291. Assuming replication is a backup.
292. Assuming snapshots are enough without logical consistency checks.
293. Failing to back up roles, extensions, schema, grants, sequences, and configuration needed to restore service.
294. Not testing restores after major schema or version changes.
295. Not knowing how long restore actually takes.
296. No runbook for accidental `DELETE`, `DROP`, `TRUNCATE`, bad migration, ransomware, region outage, or credential compromise.
297. Using production as the only copy of important data.
298. Running destructive SQL without a fresh backup/restore path.
299. Running `UPDATE` or `DELETE` in production without a transaction, preview query, row-count expectation, and rollback plan.
300. Running destructive SQL from a GUI without saving the exact statement.
301. Running `DELETE`/`UPDATE` without `WHERE` unless intentionally full-table and reviewed.
302. Using `TRUNCATE` because it is faster without understanding locks, triggers, FKs, identity reset, and rollback implications.
303. Using `DROP TABLE`, `DROP COLUMN`, `DROP INDEX`, or `DROP CONSTRAINT` as a quick fix.
304. Using `DROP ... CASCADE` without listing every dependent object and consequence.
305. Deleting audit, financial, legal, security, or event data to “clean up.”
306. Rewriting history when append-only/audit semantics are required.
307. Manually editing production data without a ticket, reason, actor, timestamp, before/after snapshot, and validation.
308. Fixing one row while leaving the invariant broken for other rows.
309. Running one-off correction scripts that cannot be re-run safely.
310. Not recording data corrections.
311. Not reconciling after disabling constraints/triggers.
312. Treating production data as test data.
313. Loading untrusted CSVs into production without staging, validation, encoding checks, and transaction boundaries.
314. Using `COPY`/bulk import to bypass constraints permanently.
315. Running generated “cleanup SQL” from an LLM directly against production.
316. Relying only on application `WHERE tenant_id = ...` filters for tenant isolation.
317. Missing `tenant_id` from child tables that need tenant scoping.
318. Missing composite uniqueness such as `(tenant_id, slug)` where uniqueness is tenant-local.
319. Global unique constraints where business uniqueness is tenant-local.
320. Tenant-local foreign keys that do not include tenant scope.
321. Joining across tenant-scoped tables without tenant predicates.
322. Background jobs that process by `id` only and can cross tenants.
323. Admin/support tools that bypass tenant isolation without audit.
324. RLS policies that cover reads but not writes.
325. RLS policies that allow inserts with wrong tenant IDs.
326. Connecting as a role that bypasses RLS.
327. Using table owners for app traffic under RLS.
328. Forgetting that superusers and `BYPASSRLS` roles bypass policies.
329. Failing to test cross-tenant denial cases.
330. Exporting/search-indexing/caching data without tenant scoping.
331. Sharing sequences or externally visible IDs in ways that leak tenant activity, where that matters.
332. Mixing tenant deletion/retention policies in shared tables without lifecycle design.
333. Using PostgreSQL rules for nontrivial behavior.
334. Using table inheritance for new designs where declarative partitioning or foreign keys are the right tool.
335. Creating triggers that silently mutate unrelated tables without tests and documentation.
336. Creating recursive triggers accidentally.
337. Hiding core business logic in triggers when application developers and tests do not account for it.
338. Using triggers to compensate for missing constraints.
339. Using triggers for audit trails without proving they cannot be bypassed.
340. Marking functions `IMMUTABLE` or `STABLE` when they are not.
341. Using volatile functions in indexes or constraints incorrectly.
342. Using `SECURITY DEFINER` without locked-down `search_path`.
343. Creating definer-rights functions owned by superuser.
344. Letting untrusted users create functions in schemas present in `search_path`.
345. Dynamic SQL inside functions without parameterization and identifier allow-listing.
346. Ignoring exception handling and transaction semantics inside functions.
347. Not versioning function definitions with migrations.
348. Manually editing functions in production outside migrations.
349. Using extensions without reviewing permissions, upgrade path, and operational impact.
350. Using triggers to maintain counters on a single hot row that becomes a write bottleneck.
351. Designing high-write counters as one row everyone updates.
352. Updating the same parent row on every child event.
353. Storing frequently updated and rarely updated columns in the same very wide table without considering bloat.
354. Creating very wide rows that make scans, updates, and vacuum more expensive.
355. Storing large blobs inline without considering TOAST, object storage, backup impact, and access patterns.
356. Putting unrelated lifecycle data in one table so every operation touches the same hot relation.
357. Ignoring partitioning for truly massive time-series or lifecycle-managed data.
358. Partitioning prematurely without understanding query pruning, indexes, constraints, and maintenance.
359. Partitioning by tenant when tenant count/cardinality makes it operationally explosive.
360. Overusing materialized views without refresh, staleness, and locking strategy.
361. Creating summary tables with no rebuild/reconciliation procedure.
362. Using cache tables as source of truth.
363. Ignoring WAL volume, replication lag, and vacuum cost of high-churn designs.
364. Designing batch jobs that update millions of rows repeatedly instead of append/versioning or staged swaps.
365. Using random UUID primary keys in hot insert paths without considering locality, index size, and write amplification.
366. Using sequential public IDs where enumeration/security concerns require non-guessable identifiers.
367. Choosing ID strategy without considering write pattern, exposure, sharding, replication, and sortability.
368. No tests for schema constraints.
369. No tests for migrations.
370. No tests for rollback or roll-forward.
371. No tests for nulls, duplicates, empty sets, large sets, and boundary values.
372. No concurrency tests for inventory, balances, seat reservations, idempotency, job claiming, or uniqueness races.
373. No tenant-isolation tests.
374. No restore tests.
375. No slow-query monitoring.
376. No lock-wait monitoring.
377. No deadlock monitoring.
378. No connection-pool monitoring.
379. No replication-lag monitoring.
380. No backup-failure alerting.
381. No disk-space alerting.
382. No autovacuum/bloat/statistics monitoring.
383. No query fingerprints/`pg_stat_statements`-style visibility.
384. No plan review for critical queries.
385. No row-count assertions for data migrations.
386. No checksums/reconciliation for backfills.
387. No production-like load tests for high-risk queries.
388. No database review for complex SQL or migrations.
389. No schema comments or design docs for non-obvious choices.
390. Treating SQL as second-class code that does not need review, linting, formatting, tests, ownership, or documentation.
391. Assuming ORM-level validations replace database constraints.
392. Assuming ORM associations replace foreign keys.
393. Assuming ORM uniqueness validation replaces a unique index.
394. Using ORM migrations without reading the generated SQL.
395. Letting ORM defaults choose bad types, nullability, indexes, and cascades.
396. Using lazy loading that creates N+1 query storms.
397. Pulling huge result sets into application memory to filter/sort/group instead of using SQL appropriately.
398. Looping row-by-row when one set-based statement is correct.
399. Doing “SELECT then UPDATE” loops instead of atomic updates.
400. Using raw SQL fragments inside ORM calls without bind parameters.
401. Hiding slow queries behind ORM abstractions.
402. Treating database errors as impossible.
403. Catching and ignoring constraint violations.
404. Swallowing serialization/deadlock errors instead of retrying correctly.
405. Mapping database `NULL` into application defaults that alter meaning.
406. Letting application enum values drift from database constraints.
407. Not pinning migrations to explicit SQL when ORM abstraction is unsafe.
408. Creating app-side cascades that partially fail.
409. Using app timestamps where database timestamps are required for consistency, or vice versa, without a clear rule.
410. Using multiple services to write the same tables without ownership boundaries.
411. Querying production OLTP directly with unbounded BI queries.
412. Giving analysts write access to production OLTP.
413. Building dashboards from inconsistent replica lag without labeling freshness.
414. Joining event data and current-state dimensions without temporal semantics.
415. Counting soft-deleted, test, spam, internal, or duplicated data accidentally.
416. Ignoring time zones in daily/weekly/monthly reports.
417. Using local date truncation without specifying locale/time zone.
418. Using `COUNT(*)` where the metric definition requires distinct entities.
419. Backfilling analytics tables without idempotency.
420. Re-running ETL jobs that duplicate facts.
421. No primary/unique keys in warehouse fact tables where idempotency requires them.
422. No data lineage.
423. No metric definitions.
424. No late-arriving data strategy.
425. No reconciliation to source-of-truth tables.
426. Copying PII to analytics without masking, retention, and access controls.
427. Using CSV exports as permanent integration contracts.
428. Letting notebooks become production pipelines.
429. Quoted mixed-case identifiers: `"User"`, `"orderId"`, `"CreatedAt"`.
430. Reserved words as identifiers: `user`, `order`, `select`, `group`.
431. Inconsistent singular/plural conventions across a schema.
432. Columns named only `name`, `type`, `status`, or `value` in generic tables where meaning becomes ambiguous.
433. Different columns for the same concept: `user_id`, `userid`, `uid`, `account_user_id`.
434. Different timestamp conventions: `created`, `created_on`, `created_at`, `create_date`.
435. Ambiguous time columns without timezone semantics.
436. Abbreviations nobody can decode.
437. Encoding business meaning in ID prefixes without constraints.
438. No comments on non-obvious tables, columns, constraints, indexes, functions, or triggers.
439. Names that hide units: `amount`, `duration`, `size`.
440. Names that hide currency.
441. Names that hide whether a timestamp is event time, ingestion time, processing time, or update time.
442. `SQL_ASCII` database encoding.
443. `trust` authentication over TCP/IP.
444. PostgreSQL rules for business logic.
445. Table inheritance instead of declarative partitioning or relational modeling.
446. Uppercase/mixed-case quoted identifiers.
447. `NOT IN` when `NULL` can appear.
448. `BETWEEN` for timestamp intervals.
449. `timestamp without time zone` for actual instants.
450. `CURRENT_TIME`.
451. `timestamp(0)` / `timestamptz(0)` to remove fractional seconds.
452. Offset strings like `-07:00` as timezone identifiers.
453. `char(n)`.
454. Arbitrary `varchar(n)` limits.
455. `money`.
456. `serial` in new schemas instead of identity columns.
457. Unsafe `SECURITY DEFINER`.
458. Unqualified names in security-sensitive functions.
459. App role as schema/table owner.
460. `CREATE INDEX` on hot large tables without `CONCURRENTLY`.
461. `CREATE INDEX CONCURRENTLY` inside a transaction.
462. Casual `VACUUM FULL`.
463. Disabling autovacuum.
464. Long `idle in transaction` sessions.
465. Ignoring invalid indexes after failed concurrent builds.
466. Ignoring `ALTER TABLE` lock behavior.
467. Treating `jsonb` as a replacement for relational design.
468. Treating `public` schema defaults as security design.
469. Asking an AI to design the schema and accepting it without a data model review.
470. Asking an AI to “fix the migration error” and accepting destructive SQL.
471. Letting AI remove constraints because inserts are failing.
472. Letting AI replace database constraints with application validations.
473. Letting AI invent columns/tables and then writing queries against them.
474. Letting AI generate raw SQL with interpolated variables.
475. Letting AI generate migrations without checking lock behavior.
476. Letting AI generate indexes without checking query plans.
477. Letting AI generate data backfills without batching and row-count validation.
478. Letting AI generate `DROP`, `TRUNCATE`, `DELETE`, `CASCADE`, `DISABLE TRIGGER`, or `session_replication_role` statements without human DBA review.
479. Letting AI-generated SQL run against production from a notebook.
480. Letting AI-generated “admin tools” accept arbitrary filters.
481. Letting AI generate RLS policies without adversarial tests.
482. Letting AI generate multi-tenant queries without tenant isolation tests.
483. Letting AI generate password/token storage schemes.
484. Letting AI generate audit logic that can be bypassed.
485. Letting AI generate “optimized” queries without correctness tests.
486. Letting AI generate SQL that passes only on toy data.
487. Letting AI generate schema from JSON examples rather than durable entities.
488. Letting AI generate denormalized schemas because “joins are slow.”
489. Letting AI generate undocumented triggers/functions.
490. Letting AI generate production data correction scripts without dry-run mode.
491. Letting AI generate code that swallows database errors.
492. Letting AI decide whether a migration is safe.
493. Treating a green local run as proof of production safety.
494. Primary key.
495. Required `NOT NULL`s.
496. Real foreign keys.
497. Real unique constraints.
498. `CHECK` constraints for obvious invariants.
499. Correct types for money, time, IDs, units, and JSON.
500. Ownership and grants defined.
501. Tenant model defined, if applicable.
502. Audit/history model defined, if applicable.
503. Delete/retention behavior defined.
504. Indexes justified by query patterns, not vibes.
505. Expected row volume and write rate documented.
506. Parameterized values.
507. Allow-listed identifiers.
508. Deterministic ordering where needed.
509. Correct null semantics.
510. Correct join grain.
511. Correct transaction/isolation behavior.
512. `EXPLAIN` / `EXPLAIN ANALYZE` for performance-sensitive paths.
513. Tests for nulls, duplicates, empty sets, boundaries, and tenant isolation.
514. Versioned migration file.
515. Tested on production-like volume.
516. Lock behavior known.
517. Roll-forward/rollback strategy.
518. Backfill batched if large.
519. Constraints added safely.
520. Indexes built concurrently when required.
521. Destructive changes separated from compatibility deploys.
522. Monitoring and abort plan.
523. Database review for nontrivial changes.
524. Least-privilege roles.
525. TLS where required.
526. No app superuser.
527. No credentials in source.
528. PITR where required.
529. Vacuum/autovacuum monitored.
530. Slow queries monitored.
531. Lock waits monitored.
532. Disk growth monitored.
533. Replication lag monitored.
534. Timeouts configured.
535. Security patches applied.
536. **PostgreSQL official docs/wiki:** SQL injection handling, constraints, transaction isolation, locking, `CREATE INDEX CONCURRENTLY`, `ALTER TABLE` locks, vacuum/analyze, authentication, RLS, and function security.
537. **OWASP / NIST:** SQL injection prevention, database hardening, least privilege, TLS, secrets, backups, and password storage.
538. **GitLab engineering guidelines:** database review, migration safety, avoiding downtime, indexing discipline, SQL query guidelines, timeouts, wide tables, and hot-row update risks.
539. **GitHub anti-pattern collections and publications:** recurring SQL/database design antipatterns such as comma-separated lists, missing constraints, EAV, polymorphic associations, float money, index shotgun, `SELECT *`, readable passwords, SQL injection, autocommit misuse, and untested restores.
540. **Relational design / normalization references:** Codd’s relational model and normalization guidance on avoiding insert/update/delete anomalies.
541. **Community signal:** Reddit discussions are useful for spotting recurring pain points, but I would not use Reddit or X as normative authority for a permanent engineering standard. I treated them as anecdotal signal and relied on official docs, engineering guides, and publications for the rules above.

## 3. Data integrity and constraints

1. Create persistent application tables with no primary key or candidate key.
2. Use a surrogate `id` while failing to enforce the real business uniqueness rule.
3. Enforce uniqueness only in application code.
4. Enforce foreign-key relationships only in application code.
5. Store relationship IDs in JSON, arrays, comma-separated strings, or polymorphic pairs when the database must enforce referential integrity.
6. Use nullable columns for values that are required by the business.
7. Use `NULL` as a magic status value when the domain has real states.
8. Allow invalid enum/status/state transitions without constraints or checked transition logic.
9. Omit `CHECK` constraints for values with hard domain limits: positive quantities, valid percentages, non-empty codes, valid date ranges, sane money amounts, allowed units.
10. Allow duplicated emails, usernames, external IDs, order numbers, invoice numbers, idempotency keys, or natural keys when uniqueness matters.
11. Disable constraints or triggers for imports and forget to re-enable and validate them.
12. Leave `NOT VALID` constraints unvalidated indefinitely.
13. Add constraints that do not actually match the business invariant.
14. Use triggers as a substitute for basic `NOT NULL`, `UNIQUE`, `CHECK`, and `FOREIGN KEY` constraints.
15. Use application tests as a substitute for database constraints.
16. Let failed imports partially commit without a quarantine/reconciliation path.
17. Use soft deletes in a way that breaks uniqueness, foreign keys, retention, or restore semantics.
18. Hard-delete data that is legally or operationally required for audit.
19. Soft-delete data that is legally required to be purged or anonymized.
20. Use mismatched types between foreign-key columns and referenced keys.
21. Use `ON DELETE CASCADE` without understanding the blast radius.
22. Omit `ON DELETE` / `ON UPDATE` behavior decisions and rely on defaults by accident.
23. Store amounts without currency.
24. Store measurements without units.
25. Store local times without location/time-zone semantics when they represent real-world events.
26. Store “latest state” only when history, auditability, or reconciliation is required.
27. Let background jobs repair integrity that the database could have prevented.
28. Use `SELECT DISTINCT` or cleanup jobs to hide duplicate data that should be impossible.
29. Accept orphaned rows as “normal.”
30. Allow cross-tenant foreign keys or references without tenant scoping.
31. Fail to test constraints with bad data, not just happy-path inserts.
32. Use `CHECK` constraints for cross-row or cross-table invariants that require `UNIQUE`, `EXCLUDE`, `FOREIGN KEY`, or carefully designed triggers; PostgreSQL notes that `CHECK` constraints are intended for immutable row-level checks.

## 3. Data-integrity and schema-design behavior that is never justified

1. Creating a persistent, mutable business table with no way to uniquely identify a row.
2. Omitting a primary key or equivalent unique, non-null identifier because “the app knows.”
3. Allowing duplicate logical rows when the business domain says duplicates are impossible.
4. Relying on application code alone for uniqueness.
5. Relying on application code alone for referential integrity.
6. Omitting foreign keys for real relationships without another explicit, tested integrity mechanism.
7. Allowing orphan rows accidentally.
8. Creating foreign-key-looking columns such as `user_id`, `account_id`, or `tenant_id` that do not actually reference anything.
9. Using `text` or `integer` for relationship fields that should be foreign keys.
10. Using polymorphic references like `(entity_type, entity_id)` without enforceable integrity rules.
11. Creating many-to-many relationships without a junction table or equivalent relational structure.
12. Storing comma-separated IDs in one column.
13. Storing arrays of IDs as a substitute for a relationship table when those IDs must be joined, constrained, searched, or deleted safely.
14. Storing stable, queryable business attributes only inside unvalidated JSONB.
15. Using JSONB as a dumping ground because schema design is hard.
16. Using EAV for stable, known attributes that need constraints, joins, indexes, and readable queries.
17. Creating `field1`, `field2`, `field3`, `phone_1`, `phone_2`, `tag_1`, `tag_2` instead of a child table.
18. Creating one column per month, year, status, region, or category instead of modeling the value as data.
19. Creating one table per customer, tenant, month, year, or status without a real partitioning/sharding design.
20. Encoding data in object names: `orders_2025`, `orders_2026`, `paid_users`, `unpaid_users`, `tenant_123_orders`.
21. Using nullable columns where the value is required for a valid row.
22. Using `NOT NULL` where missingness is meaningful and should be represented.
23. Using sentinel values such as `0`, `-1`, `9999-12-31`, `'N/A'`, `'unknown'`, or empty string instead of modeling NULL or a domain value correctly.
24. Treating NULL as false, zero, empty string, or “not applicable” without distinction.
25. Creating nullable booleans when the domain is only two-state.
26. Creating non-null booleans when the domain actually has “unknown,” “not asked,” or “not applicable.”
27. Failing to add `CHECK` constraints for obvious ranges: nonnegative prices, valid percentages, valid time intervals, valid quantities.
28. Using `float`, `real`, or `double precision` for money or exact accounting values.
29. Storing dates as strings.
30. Storing prices as strings.
31. Storing numeric values with currency symbols in the same column.
32. Splitting dates into `year`, `month`, `day` integers when a date type is required.
33. Storing absolute instants as `timestamp without time zone` without a strong reason.
34. Assuming `timestamptz` stores the original timezone name. It stores an instant and displays it according to session timezone.
35. Using `time with time zone` for real-world scheduling without understanding that timezones need dates for DST rules.
36. Using quoted mixed-case identifiers everywhere.
37. Naming columns with reserved words like `user`, `order`, `select`, or `group`.
38. Using generic columns like `data`, `info`, `value`, `type`, `status`, or `metadata` without constraints or documented meaning.
39. Using free-text status fields where only a controlled set of statuses is valid.
40. Using an enum for business values that change frequently, without a migration plan.
41. Duplicating derived data without declaring the source of truth.
42. Denormalizing without tests or constraints that detect drift.
43. Maintaining totals, counters, balances, or inventory quantities without transactional correctness.
44. Storing balances as mutable facts instead of deriving from or reconciling with ledger-like events where correctness matters.
45. Updating audit/history tables in app code only, with no transactional guarantee.
46. Soft-deleting rows without deciding what happens to uniqueness, foreign keys, cascades, restores, retention, and legal deletion.
47. Hard-deleting rows where audit, compliance, restore, or downstream references require a retention model.
48. Cascading deletes without knowing the blast radius.
49. Refusing cascade/delete rules and then leaving manual cleanup to hope.
50. Treating lookup/reference data as magic constants scattered through application code.
51. Creating tenant-scoped tables without including tenant identity in keys, uniqueness rules, RLS, indexes, and tests.
52. Allowing cross-tenant joins or references unless they are explicitly part of the model.
53. Using `ctid`, `xmin`, physical row order, sequence gaps, or insertion order as durable business identity.
54. Assuming sequence values are gapless or rollback on transaction abort.
55. Relying on implicit casts for important semantics.
56. Allowing invalid states temporarily and never cleaning them up.
57. Designing the schema only around one screen or one API response instead of the underlying facts.
58. Allowing “schema later” for data that already has known relational structure.
59. Creating wide sparse tables because “joins are bad.”
60. Using database comments, docs, and diagrams nowhere for a complex domain.

## 3. Privileges, roles, RLS, and tenant isolation

1. Granting `SUPERUSER` to application roles.
2. Granting `CREATEDB`, `CREATEROLE`, `BYPASSRLS`, or replication privileges casually.
3. Granting `pg_read_all_data` or `pg_write_all_data` to app roles.
4. Granting file/server-program roles such as `pg_execute_server_program` without extreme justification.
5. Using `GRANT ALL` because permissions are annoying.
6. Granting broad privileges to `PUBLIC`.
7. Failing to revoke default privileges where needed.
8. Letting analytics/BI users write to OLTP tables.
9. Letting migration roles run during normal request handling.
10. Letting support tooling bypass tenant boundaries.
11. Relying only on application code for tenant isolation in a high-risk multi-tenant system.
12. Missing `tenant_id` from tenant-owned tables.
13. Having `tenant_id` columns but no tenant-scoped foreign keys or unique constraints.
14. Joining tenant-owned tables without tenant predicates.
15. Using globally unique IDs as an excuse to skip tenant constraints.
16. Disabling row-level security “temporarily” and forgetting to restore it.
17. Marking RLS policies as “later.”
18. Granting `BYPASSRLS` to roles used by apps, dashboards, or workers.
19. SECURITY DEFINER functions without a locked-down `search_path`.
20. SECURITY DEFINER functions owned by overly powerful roles.
21. SECURITY DEFINER functions that call unqualified object names.
22. Letting untrusted users create objects in schemas on another role’s `search_path`.
23. Using `public` schema as a dumping ground for everything.
24. Not schema-qualifying security-sensitive functions, operators, and tables.
25. Treating “internal admin” as authorization.
26. Hiding tenant filters in app helper functions instead of enforcing boundaries where possible.
27. No tests that prove cross-tenant reads/writes fail.

## 3. Schema and data-modeling bad behavior

1. **Tables without primary keys.** PostgreSQL docs describe primary keys as unique and non-null, and state that every table should usually have one.
2. **Transactional relationships without foreign keys.** App-only referential integrity is not enough when data can be written by migrations, jobs, imports, consoles, multiple apps, or future services.
3. **Using “we enforce it in the app” as a reason not to add `UNIQUE`.**
4. **Using “we validate it in the app” as a reason not to add `NOT NULL`.**
5. **Using “we validate it in the app” as a reason not to add `CHECK` constraints for simple domain rules.**
6. **Nullable-by-default schemas.** Unknown/optional values are real, but “everything nullable” creates silent invalid states.
7. **Boolean columns that allow `NULL` without a defined third meaning.**
8. **Status columns with arbitrary text and no allowed-value constraint, enum, lookup table, or domain rule.**
9. **No uniqueness constraint for natural uniqueness: emails, slugs, external IDs, provider IDs, idempotency keys, usernames, etc.**
10. **Uniqueness enforced by `SELECT` then `INSERT`.** That is a race condition; use a unique constraint and `INSERT ... ON CONFLICT`.
11. **Missing composite uniqueness for scoped uniqueness.** Example: `slug` unique globally when it should be unique per tenant, or not unique per tenant when it must be.
12. **Foreign keys that point only to `id` while ignoring tenant scope in multi-tenant systems.**
13. **Foreign keys with the wrong `ON DELETE` behavior.** PostgreSQL docs note `CASCADE` fits component/dependent rows, while independent objects often need `RESTRICT`/`NO ACTION`; choosing this casually can delete or orphan important data.
14. **Using `ON DELETE CASCADE` because cleanup is annoying.**
15. **Using `ON DELETE SET NULL` when the business relationship is actually mandatory.**
16. **Soft deletes everywhere without uniqueness, FK, partial-index, and retention semantics.**
17. **Soft-deleted rows that still block legitimate unique values forever.**
18. **Soft-deleted parent rows with live child rows and no policy for visibility, restore, or purge.**
19. **Many-to-many relationships encoded as comma-separated IDs, JSON arrays, or text blobs when referential integrity/queryability matters.** PostgreSQL docs show many-to-many relationships modeled with a join table.
20. **The “Jaywalking” anti-pattern: storing multiple foreign keys in one delimited string.** This is one of the classic SQL antipatterns cataloged in *SQL Antipatterns*.
21. **Entity-Attribute-Value for normal structured business entities.** EAV can be appropriate for sparse/extensible metadata, but not as a replacement for a schema when fields are known, constrained, indexed, and queried.
22. **One true lookup table for unrelated concepts.** Community discussions repeatedly flag this because it weakens foreign keys and domain meaning.
23. **One giant `metadata JSONB` column for core relational data.**
24. **Using JSONB to avoid migrations.**
25. **Using JSONB to avoid foreign keys.**
26. **Using JSONB to avoid type checks.**
27. **Using JSONB to hide an unstable product model rather than designing extension points.**
28. **Creating a new table per tenant, customer, month, status, event type, or feature flag when a normal relational model or partitioning should be used.**
29. **Encoding hierarchy as fragile strings without constraints when recursive queries, closure tables, materialized paths, or adjacency lists are appropriate.**
30. **No source-of-truth distinction between raw facts, derived facts, cached facts, and denormalized facts.**
31. **Storing derived values with no invalidation, recomputation, reconciliation, or ownership.**
32. **Duplicating the same business fact in multiple tables without a rule for which copy wins.**
33. **Using floating point for money.**
34. **Blindly using PostgreSQL `money`.** The PostgreSQL wiki advises against `money`; `numeric` plus an explicit currency column is usually the safer model.
35. **Currency amounts without currency codes.**
36. **Measurements without units.**
37. **Timestamps without a clear time-zone policy.**
38. **Using `timestamp without time zone` for real-world points in time.** The PostgreSQL wiki says to use `timestamptz` for points in time.
39. **Storing UTC in `timestamp without time zone`.** The PostgreSQL wiki explicitly warns against this because it hides the semantic meaning.
40. **Using `timetz`.** The PostgreSQL wiki says not to use it.
41. **Using `CURRENT_TIME`.** The PostgreSQL wiki says not to use it.
42. **Using `timestamp(0)` or `timestamptz(0)` to “drop milliseconds.”** PostgreSQL rounds, not truncates; the wiki says not to use these.
43. **Using text offsets like `-07:00` as time-zone names.** Store actual instants and named zones where the named civil zone matters.
44. **Using `char(n)`.** The PostgreSQL wiki says `char(n)` is almost never useful and recommends `text` plus constraints/domains where needed.
45. **Using arbitrary `varchar(n)` lengths as fake validation.** The PostgreSQL wiki recommends `text` or unconstrained `varchar` unless the limit is a real business rule.
46. **Using `serial` for new PostgreSQL designs.** The PostgreSQL wiki recommends identity columns for new applications.
47. **Using SQL_ASCII for normal text.** The PostgreSQL wiki warns that mixed encodings can create unrecoverable messes.
48. **Quoted mixed-case identifiers.** `"UserProfile"` and `"createdAt"` force quoting forever and cause confusion; the PostgreSQL wiki recommends avoiding uppercase table and column names.
49. **Reserved words as table or column names.**
50. **Columns named `type`, `date`, `user`, `order`, `group`, or `desc` without understanding quoting and ambiguity costs.**
51. **`id` columns whose meaning changes across tables and APIs without table-qualified names.**
52. **No naming convention for PKs, FKs, unique constraints, indexes, and check constraints.**
53. **Schema objects created manually in prod but missing from migrations.**
54. **Migrations that do not represent the real schema state.**
55. **Using database inheritance for new designs without a strong PostgreSQL-specific reason.** The PostgreSQL wiki warns against table inheritance for ordinary use and notes native partitioning replaced many historical inheritance uses.
56. **Using PostgreSQL rules instead of triggers for ordinary behavior.** The PostgreSQL wiki says not to use rules and recommends triggers instead.
57. **Modeling audit history by overwriting rows and hoping logs are enough.**
58. **Audit tables without actor, time, source, old/new value, request/job ID, or retention policy.**
59. **Using production tables as scratchpads for imports, one-off scripts, or temporary transformations.**
60. **No ownership metadata for business-critical rows when ownership matters.**
61. **No lifecycle columns where lifecycle matters: created, updated, archived, deleted, effective dates, validity windows.**
62. **Validity windows with overlapping rows and no exclusion constraint or conflict prevention.**
63. **Temporal data without a rule for “as of,” “effective at,” “recorded at,” and “corrected at.”**
64. **Analytics facts with mutable dimensions and no slowly-changing-dimension policy.**
65. **Event tables without idempotency keys or natural deduplication.**
66. **Inbox/outbox tables without uniqueness, delivery state, retry limits, and cleanup policy.**
67. **Polymorphic foreign keys that cannot be enforced.**
68. **Generic `object_type` + `object_id` references for critical data without triggers, constraints, or redesign.**
69. **A god table with hundreds of unrelated columns because joins are considered scary.** GitLab’s database docs warn that wide tables increase write amplification, WAL, vacuum overhead, and indexing costs.
70. **High-frequency updates to a single counter/config row.** GitLab warns that high-frequency updates to a single row can create lock queues, pool saturation, vacuum pressure, and WAL pressure.

## 4. Migration and DDL behavior that is never justified

1. Making manual production schema changes that are not captured in version-controlled migrations.
2. Editing an already-applied migration as if history can be rewritten.
3. Letting environments drift with different migration histories.
4. Auto-syncing ORM models to production schema.
5. Running destructive DDL without a backup and restore plan.
6. Running destructive DDL without knowing all application versions that still read or write the old schema.
7. Dropping a table, column, index, constraint, enum value, or trigger because “nothing seems to use it.”
8. Renaming a table or column in one step while old code may still be deployed.
9. Changing column meaning without changing column name, constraints, documentation, and dependent code.
10. Adding a `NOT NULL` constraint to a large existing table without a backfill/validation/lock plan.
11. Adding a foreign key to a large existing table without a validation and locking strategy.
12. Adding a unique constraint to dirty data without finding and resolving duplicates first.
13. Creating a normal index on a large production table when writes cannot be blocked.
14. Forgetting to clean up invalid indexes after failed `CREATE INDEX CONCURRENTLY`.
15. Creating indexes during peak traffic without understanding extra CPU and I/O load.
16. Backfilling a large table in one transaction.
17. Updating millions of rows without batching, throttling, monitoring, and retry.
18. Deleting millions of rows in one transaction when batching or partition drop is appropriate.
19. Migrating data without idempotency.
20. Migrating data without recording progress.
21. Migrating data without safe retry behavior.
22. Disabling triggers or constraints for a migration and not revalidating.
23. Disabling foreign keys because the import is “trusted.”
24. Using `DROP CASCADE` casually.
25. Running `TRUNCATE` on production tables without understanding locks, foreign keys, replicas, and audit requirements.
26. Running `VACUUM FULL`, `CLUSTER`, or table rewrites on production without understanding blocking.
27. Changing column types on large tables without checking whether it rewrites the table.
28. Adding defaults, generated columns, or expressions without understanding rewrite and lock behavior.
29. Adding volatile defaults during large migrations without cost analysis.
30. Creating enum changes with no downgrade path.
31. Shipping app code that writes new columns before the column exists everywhere.
32. Dropping old columns before all deployed code has stopped reading them.
33. Adding required columns before all deployed code writes them.
34. Failing to use expand/contract migrations for incompatible app/schema changes.
35. Running migrations without `lock_timeout` and `statement_timeout` policies.
36. Running migrations from a laptop against production.
37. Running migrations without CI on a clean database and an upgraded realistic database.
38. Running migrations without rollback output or roll-forward remediation.
39. Running migrations without reviewing generated SQL.
40. Running migrations without query plans for new nontrivial queries.
41. Running migrations without database-owner review for high-risk schema changes.
42. Hiding data migrations inside app startup.
43. Hiding schema mutations inside request paths.
44. Creating triggers/functions/views in production manually because “the migration tool is annoying.”
45. Using migrations to repair data silently without audit.
46. Treating staging success on tiny data as proof a production migration is safe.
47. Applying migrations out of order.
48. Skipping migrations in one environment and “catching up” manually.
49. Making irreversible migrations for convenience when a reversible design is practical.
50. Forgetting replicas, logical replication, downstream warehouses, CDC, and backup tooling when changing schema.

## 4. Primary keys, foreign keys, uniqueness, and constraints

1. Creating tables without primary keys.
2. Using heap tables with no stable row identity for mutable business data.
3. Treating primary keys as optional because “the ORM works.”
4. Allowing duplicate business records instead of enforcing uniqueness.
5. Deduplicating in application code instead of a unique constraint.
6. Using `LIMIT 1` to hide duplicates.
7. Using `DISTINCT` to hide a broken join or broken model.
8. Omitting foreign keys for real relationships.
9. Saying “the app enforces referential integrity.”
10. Removing foreign keys to make tests or imports easier.
11. Creating orphan rows as normal behavior.
12. Using nullable foreign keys where the relationship is mandatory.
13. Having foreign keys but no thought about `ON DELETE` / `ON UPDATE`.
14. Using `ON DELETE CASCADE` on important business data without explicit domain approval.
15. Using `ON DELETE SET NULL` where null would create invalid state.
16. Using `CASCADE` to make deletion errors go away.
17. Creating polymorphic references like `(object_type, object_id)` with no enforceable foreign key.
18. Having one column that sometimes references table A and sometimes table B.
19. Storing comma-separated foreign keys in text.
20. Storing arrays of foreign keys when relationships need referential integrity.
21. Storing relational facts inside `jsonb` to avoid join tables.
22. Modeling many-to-many relationships without a junction table.
23. Missing composite primary keys or unique constraints on junction tables.
24. Allowing multiple “active” rows when only one should exist.
25. Not using partial unique indexes for conditional uniqueness.
26. Not using exclusion constraints for non-overlapping reservations, bookings, ranges, or schedules where overlap is invalid.
27. Missing `CHECK` constraints for valid ranges, positive amounts, non-empty strings, or state rules.
28. Using unconstrained text for statuses, categories, and modes.
29. Letting invalid enum-like values accumulate.
30. Making every column nullable by default.
31. Refusing `NOT NULL` because “the app validates it.”
32. Using null, empty string, zero, and “unknown” interchangeably.
33. Using magic sentinel values like `-1`, `9999-12-31`, or `'N/A'` instead of a modeled state.
34. Having no invariant documented for each important table.
35. Having no database tests for constraints.
36. Disabling constraints during imports and not revalidating them.
37. Creating constraints as `NOT VALID` and never validating them.
38. Trusting a migration succeeded without checking constraint validity.

## 4. Query correctness bad behavior

1. **`SELECT *` in application code, APIs, migrations, views, exports, or long-lived jobs.** It creates hidden coupling to column order and future schema changes; GitHub’s anti-pattern list and GitLab’s query guidelines both flag implicit/ambiguous columns as dangerous.
2. **`INSERT INTO table VALUES (...)` without an explicit column list.**
3. **Relying on column order instead of column names.**
4. **Unqualified column names in joins.** GitLab documents deploy-time failures from ambiguous or changed column selection and recommends explicit table-qualified columns in relevant cases.
5. **Using `DISTINCT` to hide duplicate rows caused by a broken join.**
6. **Using `GROUP BY` to hide broken cardinality.**
7. **Using `LIMIT` without `ORDER BY` when deterministic results matter.**
8. **Using `ORDER BY created_at` alone when ties can change pagination; add a stable tiebreaker such as the primary key.**
9. **Offset pagination for deep or mutable production lists when keyset/cursor pagination is required.**
10. **Using `NOT IN` when the subquery/list can contain `NULL`.** PostgreSQL’s wiki warns that `NOT IN` behaves surprisingly with nulls and recommends `NOT EXISTS` in many cases.
11. **Using `BETWEEN` for timestamp windows.** PostgreSQL’s wiki recommends half-open timestamp ranges, for example `>= start AND < end`.
12. **Comparing to `NULL` with `=` or `<>` instead of `IS NULL` / `IS NOT NULL`.**
13. **Forgetting SQL’s three-valued logic.** Reddit discussions about SQL null behavior are a useful community reminder, but the real issue is correctness: `TRUE`, `FALSE`, and `UNKNOWN` must be modeled deliberately.
14. **Using `COUNT(*)` to test existence when `EXISTS` or `LIMIT 1` is the actual question.**
15. **Using `COUNT(DISTINCT ...)` casually on large joins without plan inspection.**
16. **Using `LEFT JOIN` and then filtering the right table in `WHERE`, accidentally turning it into an inner join.**
17. **Moving predicates between `ON` and `WHERE` without understanding outer join semantics.**
18. **Using `UNION` when `UNION ALL` is correct and deduplication is unnecessary.**
19. **Using `UNION ALL` when duplicates are semantically wrong.**
20. **Changing one branch of a `UNION` without keeping column count, order, and types stable.** GitLab documents deployment failures from inconsistent `SELECT` columns in `UNION` queries after schema changes.
21. **Sending giant `IN (...)` lists from application memory.** GitLab warns about queries with huge integer lists, accidental full scans, and large query text; prefer joins, subqueries, temp tables, `unnest`, or server-side sets when appropriate.
22. **Plucking IDs into the app just to send them back to the database.** GitLab explicitly warns against moving query logic into application memory when PostgreSQL can optimize the full query with subqueries.
23. **N+1 queries hidden behind ORM relationships.**
24. **ORM scopes that look harmless but generate unbounded joins, subqueries, or scans.**
25. **Using leading-wildcard `LIKE` / `ILIKE` on large tables without trigram/full-text support.** GitLab notes wildcard searches at the start of a pattern cannot use ordinary indexes and points to trigram indexes as a possible solution.
26. **Using `LIKE '%term%'` as a search engine.** Use full-text search, trigram indexes, or an external search system when requirements exceed simple indexed lookup.
27. **Using `ORDER BY random()` on large tables.**
28. **Applying functions to indexed columns in predicates without a matching expression index.** Example: `lower(email) = ...` without an index on `lower(email)`.
29. **Implicit type casts on indexed columns because app parameters use the wrong type.**
30. **String comparison for numeric, date, UUID, boolean, or enum values.**
31. **Storing dates as text and then sorting lexicographically.**
32. **Using local time arithmetic for global instants.**
33. **Using `now()` / `clock_timestamp()` inconsistently without knowing transaction timestamp semantics.**
34. **Using `COALESCE` in predicates to paper over bad null modeling and defeat indexes.**
35. **Using `OR` filters that defeat indexes without rewriting, partial indexes, `UNION`, or plan validation.**
36. **Unbounded `UPDATE` or `DELETE` without a `WHERE` clause, batch limit, transaction boundary, and explicit review.**
37. **Using a CTE or join in an update/delete where the intended scope does not actually constrain the target.** GitLab documents a case where a CTE plus `update_all` risked updating an entire table and recommends bounded/batched updates where volume is uncertain.
38. **Assuming CTEs, subqueries, joins, and views are performance-equivalent without checking the generated plan.**
39. **Using views as a dumping ground for business logic that nobody tests.**
40. **Using triggers to hide behavior that application developers cannot discover.** Triggers can be correct, but invisible side effects without docs/tests are not.
41. **Recursive queries without cycle prevention or depth bounds.**
42. **Window functions without deterministic ordering.**
43. **Aggregates without a correct grouping key.**
44. **Time-bucket queries that ignore time zones, daylight-saving transitions, or boundary semantics.**
45. **Reporting queries that silently drop rows because of inner joins to optional dimensions.**
46. **Backfills that compute values differently from production writes.**
47. **Migrations that update data with application logic that has since changed.**
48. **Using `NULL` and empty string interchangeably.**
49. **Using magic sentinel values such as `0`, `-1`, `'unknown'`, or `1970-01-01` instead of a modeled state.**
50. **Using `SELECT ... FOR UPDATE` without understanding what rows are actually locked.**
51. **Using advisory locks without namespacing, timeout, observability, and release guarantees.**
52. **Relying on implicit transaction behavior in client libraries.**
53. **Letting generated SQL differ silently between development, staging, and production.**
54. **Not testing query behavior against realistic data distributions.**

## 4. Schema design and modeling smells

1. Store comma-separated lists in a column instead of a junction table.
2. Store multiple values in `phone1`, `phone2`, `phone3` columns instead of a child table.
3. Use entity-attribute-value tables for core domain data simply to avoid migrations.
4. Use JSON/JSONB as a dumping ground because you do not want to design tables.
5. Use `owner_type` / `owner_id` polymorphic associations where the database cannot enforce the referenced row.
6. Create one “god table” for unrelated concepts.
7. Create table-per-customer, table-per-tenant, table-per-month, table-per-status, or table-per-type designs where partitioning or normalization is the right tool.
8. Encode metadata in table names or column names.
9. Mix unrelated lifecycles in the same table.
10. Use one generic `data` table for many business entities.
11. Use meaningless columns like `value`, `type`, `flag`, `data`, `status2`, or `misc` without a documented domain.
12. Create boolean-flag explosions where combinations represent hidden states.
13. Store derived data as the only source of truth without a recomputation strategy.
14. Denormalize without proving the read benefit and documenting invalidation/refresh rules.
15. Normalize so poorly that every query needs application-side joins or repeated lookups.
16. Create circular dependencies without a clear ownership/lifecycle model.
17. Use surrogate IDs everywhere while ignoring natural uniqueness.
18. Clone schemas for every customer instead of designing tenancy intentionally.
19. Create “future proof” columns such as `custom1`, `custom2`, `extra`, `reserved`, or `spare`.
20. Mix OLTP event writes, mutable current state, analytics rollups, and audit data in one table without clear boundaries.
21. Put large, cold, rarely used blobs in hot transactional rows.
22. Store files in the database when the system actually requires object-store semantics, or store file paths only when transactional consistency with metadata is required.
23. Fail to document non-obvious tables, columns, functions, triggers, policies, and invariants.
24. Use quoted mixed-case identifiers, reserved words, spaces, or punctuation in identifiers.
25. Design from ORM defaults without reviewing PostgreSQL types, indexes, constraints, and migrations.
26. Treat “we can change it later” as an excuse for a schema that cannot safely migrate later.

## 5. Anti-pattern modeling that destroys integrity

1. Entity-Attribute-Value tables for stable, known attributes.
2. “One true lookup table” for unrelated domains.
3. A universal `objects` table with generic key/value attributes for everything.
4. A `type` column that changes the meaning of every other column.
5. Tables named `data`, `items`, `records`, or `objects` with no precise domain meaning.
6. Columns named `value`, `data`, `json`, `payload`, `misc`, or `extra` as a substitute for modeling.
7. Encoding multiple facts in one column.
8. Encoding hierarchy paths in strings when proper hierarchy querying/integrity is needed.
9. Storing lists in delimited strings.
10. Storing units in the same text field as values.
11. Storing numbers as text.
12. Storing booleans as text.
13. Storing dates as text.
14. Storing money as floating point.
15. Storing timestamps without a timezone policy.
16. Storing local wall-clock time where an instant is required.
17. Storing only an instant where a local civil time and timezone are required.
18. Using email addresses, names, or mutable PII as primary keys without a migration strategy.
19. Using natural keys that can change, then pretending they cannot.
20. Using surrogate keys but failing to enforce the real business key.
21. Having “soft delete” columns everywhere with no uniqueness/FK/query strategy.
22. Having `deleted_at` but forgetting every unique index now needs the correct partial condition.
23. Having `deleted_at` but letting foreign keys point to logically deleted parents.
24. Having multiple lifecycle flags that contradict each other, such as `is_active`, `is_deleted`, `archived_at`, `disabled_at`.
25. Modeling state machines with random booleans instead of a constrained state.
26. Allowing impossible states because “the UI won’t send that.”
27. Denormalizing without source-of-truth ownership.
28. Denormalizing without invalidation logic.
29. Denormalizing without reconciliation checks.
30. Using materialized/cached tables without refresh semantics.
31. Using JSONB as a junk drawer for core relational data.
32. Using arrays as a workaround for missing schema design.
33. Using generated IDs but no domain constraints.
34. Creating history/audit tables that cannot reconstruct who changed what and when.
35. Mutating audit records.
36. Deleting audit records without retention policy.
37. Storing files/blobs in hot OLTP rows without a deliberate storage, backup, and performance plan.

## 5. Performance and physical-design bad behavior

1. **Merging performance-sensitive SQL without `EXPLAIN`.** PostgreSQL’s `EXPLAIN` exists to show the planner’s chosen plan, and PostgreSQL docs emphasize that plan choice is critical for performance.
2. **Using `EXPLAIN ANALYZE` on side-effecting statements outside a transaction/rollback wrapper.** PostgreSQL docs note `EXPLAIN ANALYZE` actually executes the statement.
3. **Testing only on empty or toy data.**
4. **Ignoring row-count estimates, selectivity, and skew.**
5. **No statistics maintenance.** PostgreSQL uses `ANALYZE` statistics for planning, and autovacuum/autoanalyze do not remove every need for manual awareness.
6. **Disabling autovacuum globally.** PostgreSQL docs say routine vacuuming is required and that disabling autovacuum is unwise except in very narrow, predictable workloads.
7. **Ignoring dead tuples, bloat, wraparound risk, and table/index growth.**
8. **Running long transactions that hold old row versions and block cleanup.**
9. **Leaving sessions `idle in transaction`.**
10. **No `statement_timeout` for application roles.**
11. **No `lock_timeout` for migrations.**
12. **No protection against runaway analytical queries on the primary.**
13. **No `pg_stat_statements` or equivalent query observability.** PostgreSQL’s `pg_stat_statements` tracks planning and execution statistics for SQL statements.
14. **Index shotgun: indexing every column.** Indexes can speed retrieval but add write and maintenance overhead; PostgreSQL docs advise using them sensibly.
15. **Missing indexes for critical foreign keys, joins, filters, and uniqueness checks.** PostgreSQL docs note foreign key declarations do not automatically create indexes on referencing columns, although such indexes are often useful.
16. **Adding redundant indexes that differ only trivially.**
17. **Keeping unused indexes forever.**
18. **Adding indexes to large hot tables without concurrent/index-build strategy.**
19. **Adding an index without proving it matches a real query shape.** PostgreSQL docs recommend checking index usage against the real workload with `EXPLAIN`.
20. **Using the wrong index type or operator class: B-tree when trigram, GIN, GiST, BRIN, hash, partial, expression, or composite indexes are the actual fit.**
21. **Composite indexes with columns in the wrong order for the workload.**
22. **Partial indexes whose predicate does not match application queries.**
23. **Expression indexes whose expression does not exactly match query expressions.**
24. **Case-insensitive lookups without a case-insensitive strategy: normalized column, expression index, `citext`, or appropriate collation.**
25. **Creating indexes to rescue bad data modeling without fixing the model.**
26. **Ignoring write amplification from indexes.**
27. **Bulk updates/deletes on large tables without batching.**
28. **Large backfills with no pause/resume, progress tracking, throttling, or retry.**
29. **High-frequency updates to a single row or narrow set of rows.** GitLab explicitly warns this can create lock queues, pool saturation, vacuum pressure, and WAL pressure.
30. **Using the primary database as an unbounded job queue without queue semantics.**
31. **Queues without `SKIP LOCKED`, lease timeouts, retry limits, visibility state, dead-letter handling, and cleanup.**
32. **Unbounded result sets returned to application memory.**
33. **No pagination on user-facing list endpoints.**
34. **Export queries that run on the primary during peak traffic.**
35. **Expensive reports coupled to OLTP tables without replicas, snapshots, summaries, or workload isolation.**
36. **Frequent `VACUUM FULL` on live tables as routine maintenance.** PostgreSQL docs note `VACUUM FULL` requires an `ACCESS EXCLUSIVE` lock and cannot run in parallel with normal use of the table.
37. **Assuming partitioning automatically improves performance.**
38. **Partitioning without pruning-friendly predicates.**
39. **Too many partitions without planning for maintenance overhead.**
40. **No retention policy for event, audit, log, notification, or job tables.**
41. **No archiving policy for cold data.**
42. **Wide hot tables with many cold columns.** GitLab warns that wide tables increase overhead and recommends splitting cold or frequently updated columns in appropriate cases.
43. **Storing large blobs in the main OLTP table when object storage or separate blob tables are the better operational design.**
44. **Creating materialized views without refresh strategy, locking strategy, and staleness semantics.**
45. **Refreshing materialized views synchronously on user requests.**
46. **Relying on planner luck instead of stable query/index design.**
47. **Ignoring connection count.**
48. **Opening a new DB connection per request/job without pooling.**
49. **Letting worker concurrency exceed DB capacity.**
50. **No backpressure when the database is saturated.**
51. **No load test for the database path.**
52. **No slow-query budget.**
53. **No plan-regression checks for critical queries.**
54. **Treating the database as infinitely scalable because “Postgres is fast.”**

## 5. PostgreSQL type mistakes

1. Use `timestamp without time zone` for actual instants in time.
2. Store UTC timestamps in `timestamp without time zone` and hope every reader remembers that convention.
3. Use `BETWEEN` for timestamp ranges where end boundaries can double-count or miss rows; use half-open ranges instead.
4. Use `timetz`.
5. Use `CURRENT_TIME` in PostgreSQL application logic.
6. Use `timestamp(0)` or `timestamptz(0)` when rounding could produce misleading future times.
7. Use `char(n)` for codes, identifiers, or “fixed length” strings.
8. Use arbitrary `varchar(n)` limits because another database or framework defaulted to them.
9. Use `money` for monetary values.
10. Use floating-point types for exact money, balances, counts, tax, or billing calculations.
11. Store dates, booleans, numbers, UUIDs, IP addresses, ranges, arrays, or JSON as plain text when PostgreSQL has a correct native type.
12. Use `serial` for new PostgreSQL designs instead of identity columns.
13. Use 32-bit integer primary keys for tables that can plausibly grow large.
14. Use different data types for referenced and referencing columns.
15. Store time-zone offsets as a substitute for real time-zone/location rules when future local scheduling matters.
16. Store currency amounts without currency code and scale rules.
17. Store percentages ambiguously as `0.15` in some places and `15` in others.
18. Use `json` where `jsonb` is required for indexing/querying, or `jsonb` where relational columns are required for integrity.
19. Use enums for volatile business concepts without a migration and compatibility plan.
20. Store encrypted data in a type that breaks required indexing/search semantics without designing the search model.
21. Use text arrays or JSON arrays to represent relationships that need constraints.
22. Use `uuid` values generated in the application without a collision, version, locality, and indexing discussion.
23. Use database defaults that hide missing application data instead of rejecting invalid writes.

## 5. Query-correctness behavior that is never justified

1. Shipping `SELECT *` in application contracts, APIs, jobs, views, or migrations where columns should be explicit.
2. Shipping `INSERT INTO table VALUES (...)` without an explicit column list.
3. Running `UPDATE` or `DELETE` without a deliberate predicate, row-count expectation, transaction boundary, and review.
4. Using `LIMIT` without `ORDER BY` when the result needs to be deterministic.
5. Assuming row order without `ORDER BY`.
6. Using `DISTINCT` to hide duplicate rows caused by a bad join.
7. Joining tables without understanding cardinality.
8. Using comma joins accidentally.
9. Forgetting a join predicate and creating a Cartesian product.
10. Counting rows after a join without checking duplication.
11. Summing money after a join without checking duplication.
12. Grouping by the wrong column and trusting plausible totals.
13. Selecting non-aggregated columns that are not functionally determined by the group.
14. Filtering on the wrong side of an outer join and accidentally turning it into an inner join.
15. Using `WHERE` conditions that accidentally remove NULL-extended rows from a left join.
16. Using `NOT IN` with nullable subqueries without understanding NULL behavior.
17. Comparing to NULL with `= NULL` or `<> NULL`.
18. Treating `NULL`, `false`, `0`, `''`, and missing row as interchangeable.
19. Using `BETWEEN` for timestamp date ranges without checking inclusive end behavior.
20. Filtering a “day” as midnight-to-midnight in the wrong timezone.
21. Forgetting tenant, organization, account, or authorization filters.
22. Applying tenant filters in application code after fetching cross-tenant data.
23. Paginating with unstable ordering.
24. Assuming offset pagination is stable while rows are being inserted or deleted.
25. Using cursors or keyset pagination without a stable unique ordering.
26. Doing read-modify-write outside a transaction.
27. Doing `SELECT` then `INSERT` for uniqueness instead of using a unique constraint and `INSERT ... ON CONFLICT`.
28. Doing `SELECT` then `UPDATE` without row locks when concurrent updates matter.
29. Ignoring lost-update scenarios.
30. Assuming `READ COMMITTED` gives repeatable results inside a transaction.
31. Ignoring serialization failures and deadlocks instead of retrying.
32. Leaving transactions idle and open.
33. Holding locks while making network calls.
34. Calling external services from inside database transactions through app code.
35. Writing triggers that perform surprising extra writes.
36. Using triggers to hide business behavior that the application team does not know exists.
37. Marking a function `IMMUTABLE` when it reads tables, settings, time, random values, or anything that can change.
38. Marking functions `PARALLEL SAFE` or `LEAKPROOF` without proving it.
39. Using functions in predicates that defeat indexes without an expression index or rewrite.
40. Using `LOWER(column) = ...` repeatedly without an expression index or normalized/search column.
41. Using leading-wildcard `LIKE '%term%'` on large tables without trigram/full-text/search design.
42. Using regex filters on large unindexed data as normal product behavior.
43. Using `ORDER BY random()` on large tables.
44. Using unbounded recursive CTEs.
45. Using unbounded `SELECT`, export, or report queries against OLTP primaries.
46. Querying replicas for fresh writes without understanding replication lag.
47. Writing queries that are only correct for current sample data.
48. Writing queries that rely on undocumented uniqueness.
49. Writing queries that rely on soft-deleted rows being absent when they are present.
50. Writing queries that silently include test, archived, deleted, draft, or future-effective rows.
51. Using implicit time zone/session settings for business logic.
52. Using session-local temp state in connection pools without cleanup.
53. Depending on `search_path` to resolve production objects in app SQL.
54. Returning sensitive columns because they happened to be in `SELECT *`.
55. Adding `LIMIT 1` to hide data-model ambiguity.
56. Adding `MAX(id)` or latest timestamp as a proxy for “current” without a current-state rule.
57. Treating an empty result as proof that no data exists when the query might be wrong.
58. Treating a non-empty result as proof that the query is semantically correct.

## 5. Transaction and concurrency behavior that is never excusable

1. Read-modify-write without a transaction when concurrent writes are possible.
2. “Check then insert” without a unique constraint or `ON CONFLICT`.
3. Generating sequential business numbers with `SELECT max(id) + 1`.
4. Updating balances, counters, inventory, quotas, or seats without atomic SQL or locking.
5. Assuming tests pass means concurrency is safe.
6. Assuming `READ COMMITTED` prevents race conditions.
7. Using long transactions around HTTP calls, user input, file uploads, payment calls, or LLM calls.
8. Leaving sessions `idle in transaction`.
9. Opening a transaction at request start and committing at request end by habit.
10. Holding locks while performing slow application work.
11. Locking rows without an indexable predicate.
12. `SELECT FOR UPDATE` on huge result sets because the query is under-filtered.
13. Using explicit table locks as a substitute for understanding isolation.
14. Acquiring locks in inconsistent order across code paths.
15. Ignoring deadlock errors instead of retrying safely.
16. Retrying non-idempotent transactions blindly.
17. No bounded retry policy for serialization failures.
18. Using advisory locks without naming, scope, timeout, and release discipline.
19. Using advisory locks as an undocumented global mutex.
20. Changing isolation levels without documenting which invariant requires it.
21. Relying on application caches for correctness under concurrent writes.

## 6. Data-type negligence

1. Using `text` for everything.
2. Using `numeric` with no precision/scale policy where precision matters.
3. Using `float`, `real`, or `double precision` for currency/accounting.
4. Using `money` casually without understanding locale/formatting/portability tradeoffs.
5. Using `integer` for counters that can exceed it.
6. Keeping 32-bit IDs where growth requires 64-bit.
7. Using `serial`/sequences without understanding gaps and non-transactional behavior.
8. Assuming sequence values prove chronological order.
9. Using UUIDs without considering index locality/write amplification.
10. Using random UUID primary keys on very hot tables without understanding index impact.
11. Using time zone types without a policy.
12. Using `timestamp without time zone` for global event instants by accident.
13. Using `timestamp with time zone` but misunderstanding display/session timezone behavior.
14. Comparing dates/timestamps by formatting strings.
15. Storing durations as ambiguous strings.
16. Storing measurements without units.
17. Mixing units in one column.
18. Using unconstrained `jsonb` for data that requires shape guarantees.
19. Using enums for values that product changes every week without migration planning.
20. Avoiding enums/domains/checks entirely and letting arbitrary text spread.
21. Using case-sensitive text for emails/usernames without a deliberate normalization/collation policy.
22. Ignoring collations and case-sensitivity in uniqueness.
23. Using `citext`, lowercased columns, or expression indexes inconsistently.
24. Storing IP addresses as text instead of appropriate network types when querying/ranging matters.
25. Storing geospatial data as random lat/lon floats when PostGIS/geometric constraints are required.
26. Storing encrypted blobs for fields that must be queried, sorted, or constrained without a design.

## 6. Performance and operability behavior that is never justified

1. Shipping a nontrivial query without looking at `EXPLAIN` or `EXPLAIN ANALYZE` in a safe environment.
2. Assuming a query plan on tiny dev data predicts production behavior.
3. Creating indexes “just in case.”
4. Refusing indexes because “indexes slow writes” without measuring read/write tradeoffs.
5. Indexing every column.
6. Creating duplicate indexes.
7. Creating overlapping indexes without proving they are needed.
8. Creating multicolumn indexes with the wrong leading column.
9. Creating indexes that no query can use.
10. Leaving unused indexes forever.
11. Not indexing foreign-key columns used by deletes, cascades, or joins.
12. Indexing low-cardinality columns blindly.
13. Creating huge GIN indexes on JSONB columns without query evidence.
14. Creating expression indexes whose expressions do not match queries.
15. Creating partial indexes whose predicates do not match queries.
16. Forgetting that partial indexes cannot serve every query shape.
17. Using `OR` chains when a clearer `IN`, `UNION ALL`, or query rewrite is appropriate.
18. Using `UNION` when `UNION ALL` is correct and duplicates do not matter.
19. Using `DISTINCT` as a performance-insensitive cleanup step.
20. Creating N+1 query patterns.
21. Running per-row queries in loops instead of set-based operations.
22. Running per-row updates without batching.
23. Pulling huge result sets into the application to filter, join, or aggregate.
24. Using the database as a dumb key-value store while reimplementing constraints in app code.
25. Using the application as a join engine for data already in Postgres.
26. Running heavy analytics on the OLTP primary without workload isolation.
27. Running reports with no time window, no tenant filter, no limit, and no timeout.
28. Letting BI tools generate arbitrary SQL against production tables.
29. Letting dashboards refresh expensive queries too frequently.
30. Not setting statement timeouts for application roles.
31. Not setting lock timeouts for migrations.
32. Not using connection pooling and opening unbounded database connections.
33. Creating one database connection per request without limits.
34. Letting background jobs overwhelm the database.
35. Ignoring `pg_stat_statements`, slow query logs, lock monitoring, and wait events.
36. Ignoring stale statistics.
37. Disabling autovacuum globally.
38. Disabling autovacuum on busy tables without a replacement maintenance plan.
39. Ignoring bloat.
40. Ignoring long-running transactions that prevent vacuum cleanup.
41. Ignoring transaction ID wraparound risk.
42. Ignoring disk growth from WAL, temp files, indexes, and bloat.
43. Ignoring replication lag.
44. Ignoring invalid indexes.
45. Ignoring dead tuples and table/index bloat after mass updates/deletes.
46. Keeping huge append-only tables forever without retention or partitioning.
47. Deleting old data row-by-row forever when partitioning/drop-detach is the right lifecycle.
48. Using materialized views without a refresh strategy.
49. Using caches without invalidation, TTL, ownership, and consistency expectations.
50. Caching raw SQL results with no dependency tracking or invalidation.
51. Storing large files externally with only paths in Postgres and no transactional cleanup/story.
52. Storing huge blobs in Postgres with no backup, restore, TOAST, bandwidth, or access-pattern plan.
53. Using advisory locks without namespacing, timeout, and release discipline.
54. Using `SELECT ... FOR UPDATE` over too many rows.
55. Holding locks while waiting on humans, queues, APIs, or slow I/O.
56. Failing to load-test the actual high-cardinality, high-concurrency path.
57. Failing to test the slow path: empty cache, large tenant, largest customer, month-end, year-end.
58. Assuming the optimizer is wrong before checking statistics, indexes, query shape, and data distribution.
59. Forcing planner settings globally to fix one query.
60. Treating performance as a one-time activity instead of a monitored production property.

## 6. PostgreSQL-specific “don’t do this” behavior

1. Use PostgreSQL rules for business behavior; use triggers or explicit logic.
2. Use table inheritance as a casual modeling tool or old-style partitioning substitute.
3. Use `NOT IN` with nullable subqueries.
4. Use `= NULL` or `<> NULL` instead of `IS NULL` / `IS NOT NULL`.
5. Use quoted mixed-case identifiers that force every query to quote names forever.
6. Rely on `search_path` for security-sensitive code.
7. Create objects in `public` by default without privilege review.
8. Leave `CREATE` privileges on shared schemas for untrusted roles.
9. Use session-level settings as hidden application state without resetting pooled connections.
10. Assume PostgreSQL behaves like MySQL, SQL Server, SQLite, or Oracle on types, casing, locking, isolation, upserts, booleans, time zones, or string comparison.
11. Use extensions in production without versioning, upgrade, backup, restore, and security review.
12. Install untrusted extensions because a blog post used them.
13. Use triggers that silently mutate data in surprising ways.
14. Hide critical business writes inside functions that no one reviews.
15. Use `LISTEN/NOTIFY` as a durable queue.
16. Use advisory locks casually in pooled connections; PostgreSQL notes session-level advisory locks are held until explicitly released or the session ends.

## 6. Transaction and concurrency bad behavior

1. **Using autocommit for multi-step business operations that must be atomic.** GitHub’s SQL anti-pattern examples call out loss of transaction boundaries and atomicity problems from autocommit misuse.
2. **Committing inside a logical unit of work because the batch is long.**
3. **Holding a transaction open while doing network calls, file I/O, API requests, user prompts, sleeps, or queue waits.**
4. **Starting a transaction and then waiting on user input.**
5. **Read-modify-write without locking, a version column, `UPDATE ... WHERE old_value`, or another concurrency control.**
6. **`SELECT` then `INSERT` for find-or-create without a unique constraint.**
7. **Ignoring `ON CONFLICT` where uniqueness races exist.**
8. **Ignoring serialization failures and deadlocks instead of retrying idempotently.**
9. **Assuming `READ COMMITTED` prevents lost updates, write skew, or inconsistent multi-query reads.** PostgreSQL’s MVCC model gives readers and writers useful nonblocking behavior, but isolation level still matters for correctness.
10. **Using `SERIALIZABLE` without retry logic.**
11. **Using `SELECT FOR UPDATE` as a magic wand without understanding lock scope.**
12. **Locking rows in inconsistent order across code paths.**
13. **Taking table locks as a convenience.**
14. **DDL that unexpectedly blocks production reads/writes.**
15. **No lock-timeout policy for deploys and migrations.**
16. **No statement-timeout policy for app queries.**
17. **Mixing external side effects and database transactions without an outbox/idempotency pattern.**
18. **Sending emails, webhooks, or payments before the transaction commits.**
19. **No idempotency key for externally retried operations.**
20. **Retrying failed jobs that are not safe to retry.**
21. **Using advisory locks without timeout, owner, namespace, and observability.**
22. **Using advisory locks to compensate for missing unique constraints.**
23. **Using long-running repeatable reads for background jobs on hot databases without understanding bloat and snapshot effects.**
24. **Updating rows in nondeterministic order in concurrent workers.**
25. **Worker pools competing for the same rows without `SKIP LOCKED` or equivalent coordination.**
26. **Batch jobs that scan and lock the whole table repeatedly.**
27. **No deadlock tests for high-contention paths.**
28. **No concurrency tests for inventory, balances, quotas, idempotency, or uniqueness.**

## 7. Backup, recovery, and durability behavior that is never justified

1. Having valuable production data with no backups.
2. Having backups that nobody has restored.
3. Treating an untested backup as a backup.
4. Storing backups only on the same machine.
5. Storing backups only in the same account/project with the same compromised credentials.
6. Storing backups unencrypted when they contain sensitive data.
7. Failing to define RPO and RTO.
8. Failing to know how much data the business can lose.
9. Failing to know how long restore can take.
10. Assuming `pg_dump` alone is enough for point-in-time recovery.
11. Deleting WAL archives without understanding backup windows and PITR.
12. Performing destructive migrations without a fresh backup or restore point.
13. Not testing restore after major version upgrades.
14. Not testing restore after backup-tool changes.
15. Not monitoring backup success.
16. Not alerting on backup failure.
17. Not alerting on WAL archiving failure.
18. Not documenting restore runbooks.
19. Keeping restore knowledge in one person’s head.
20. Failing to test accidental `DELETE`, `DROP TABLE`, bad migration, region loss, and credential compromise scenarios.
21. Backing up logical dumps but not roles, grants, extensions, ownership, and required server settings.
22. Backing up data without schema, or schema without data, accidentally.
23. Assuming replication is backup.
24. Assuming high availability is backup.
25. Assuming snapshots are restorable without testing.
26. Letting backups retain data that should be deleted under policy forever.
27. Letting backups expire before legal or business requirements.
28. Letting CI/dev systems use production backup data without masking controls.

## 7. Migration, deployment, and schema-evolution bad behavior

1. **Schema migration and application deploy that are not backward/forward compatible.**
2. **Dropping a column in the same deploy that stops using it.** GitLab documents a multi-release process for destructive changes because old code and cached schema can still reference removed columns.
3. **Dropping a table before all code paths, jobs, views, reports, and foreign keys are removed.**
4. **Renaming a column/table without a compatibility window.**
5. **Changing a column type on a large table without understanding rewrite, locks, index rebuilds, and rollback.**
6. **Adding a `NOT NULL` column with a default to a large table without checking PostgreSQL version behavior and lock/rewrite implications.**
7. **Adding `NOT NULL` without backfilling and validating safely.** GitLab’s migration guide explicitly calls out `NOT NULL` constraints and adding columns with defaults as areas requiring care.
8. **Adding a foreign key to dirty data without validation strategy.**
9. **Removing foreign keys casually.** GitLab notes removing foreign keys can lock both tables and should be handled carefully on large tables.
10. **Adding indexes to large tables in a blocking way.**
11. **Running large data migrations inside ordinary schema migrations.** GitLab distinguishes regular, post-deployment, and batched background migrations and recommends batched background migrations for long-running data changes.
12. **Migrations depending on application model code that can change later.**
13. **Migrations that call current application validations, callbacks, or service objects.**
14. **Migrations with no rollback or documented irreversibility.**
15. **Migrations with no dry run on production-like data.**
16. **Migrations with no estimated row count, runtime, lock impact, and WAL impact.**
17. **Migrations with no observability or progress marker.**
18. **Migrations with no pause/resume capability for large changes.**
19. **Migrations that perform unbounded `UPDATE`/`DELETE`.**
20. **Migrations that assume old dirty data does not exist.** GitLab’s migration style guide warns to guard against stale data and assumptions.
21. **Migrations that silently skip failed rows.**
22. **Migrations that write inconsistent data and rely on a later cleanup nobody owns.**
23. **Manual hotfix DDL in production that is not captured back into migrations.**
24. **Manual data fixes without transaction, audit trail, before/after query, and rollback script.**
25. **Deploying migrations out of order.**
26. **Changing enum values without compatibility across old/new app versions.**
27. **Changing defaults while old app versions still assume the previous default.**
28. **Dropping indexes without checking usage, constraints, and query plans.**
29. **Creating constraints as `NOT VALID` and forgetting to validate them.**
30. **Keeping legacy columns forever with no deprecation owner.**
31. **No schema review for generated migrations.**
32. **No linting/static checks for dangerous migration operations.**
33. **No production change window or lock-retry policy for risky DDL.**
34. **Assuming rollback is possible after destructive data changes.**

## 7. Query correctness failures

1. Rely on implicit row order without `ORDER BY`.
2. Use `SELECT *` in stable APIs, migrations, views, background jobs, or security-sensitive queries.
3. Use ambiguous column names in joins.
4. Use `NATURAL JOIN` in production application SQL.
5. Use joins without understanding cardinality.
6. Accidentally create Cartesian products.
7. Use `DISTINCT` to mask duplicate rows caused by broken joins.
8. Use `GROUP BY` while selecting non-grouped, non-aggregated columns.
9. Assume `NULL` behaves like false, zero, empty string, or missing JSON.
10. Use `NOT IN` when `NULL` may appear.
11. Use `BETWEEN` for timestamp windows.
12. Use inclusive end dates for time ranges in reporting.
13. Use offset pagination for deep or mutable result sets without understanding duplicates, omissions, and performance.
14. Use unstable pagination order, such as ordering only by non-unique timestamp.
15. Use `ORDER BY random()` on large tables in hot paths.
16. Use leading-wildcard `LIKE '%term%'` on large tables without a trigram/full-text/search design.
17. Apply functions to indexed columns in predicates without matching expression indexes.
18. Cast the column side instead of the parameter side and accidentally defeat indexes.
19. Depend on implicit casts.
20. Filter JSON fields in hot paths without appropriate expression or GIN indexes.
21. Use unbounded queries in request/response paths.
22. Use `COUNT(*)` repeatedly on huge hot tables when an estimate, cache, counter table, or rollup is the designed answer.
23. Use one giant “spaghetti query” no one can reason about.
24. Use CTEs, subqueries, lateral joins, window functions, or recursive queries without checking the actual plan.
25. Assume a query is safe because it is fast on an empty development database.
26. Run `EXPLAIN ANALYZE` for destructive statements in production without wrapping and rolling back safely; PostgreSQL documents that `EXPLAIN ANALYZE` actually executes the statement.
27. Ignore bad row estimates, sequential scans in hot paths, sort spills, hash spills, nested-loop explosions, or missing statistics.
28. Use application loops where one set-based SQL statement is correct.
29. Use one set-based SQL statement where batching is required to avoid locks, bloat, replication lag, or outages.
30. Treat read queries as harmless when they can lock, spill, saturate I/O, or starve production.
31. `UPDATE` without a `WHERE` clause unless intentionally full-table and protected.
32. `DELETE` without a `WHERE` clause unless intentionally full-table and protected.
33. Full-table destructive operations without transaction, row-count check, and rollback plan.
34. `SELECT *` in application contracts, public APIs, ETL contracts, or long-lived views.
35. Relying on implicit column order.
36. Inserting without explicit column names.
37. Relying on natural row order without `ORDER BY`.
38. Using `LIMIT` without deterministic `ORDER BY`.
39. Using `OFFSET` pagination on large/changing datasets where keyset pagination is required.
40. Using `MAX(id)` as “latest” without a defined ordering column.
41. Using sequence gaps to infer missing rows, failed transactions, or business meaning.
42. Using `now()`/timestamps from multiple systems with no clock policy.
43. Comparing `NULL` with `= NULL` or `<> NULL`.
44. Using `NOT IN` with a nullable subquery and not understanding null semantics.
45. Treating `NULL` as false, zero, empty string, or absent interchangeably.
46. Putting right-table filters in `WHERE` after a `LEFT JOIN` and accidentally turning it into an inner join.
47. Writing accidental cross joins.
48. Joining on non-key columns by accident.
49. Joining text IDs to integer IDs through implicit casts.
50. Using `DISTINCT` because the join produced too many rows.
51. Using `GROUP BY` to mask duplicates.
52. Aggregating at the wrong grain.
53. Selecting non-aggregated values that do not belong to the grouping grain.
54. Using window functions without deterministic ordering.
55. Using `ORDER BY random()` on large production tables.
56. Using leading-wildcard `LIKE '%term%'` on large tables with no search/index strategy.
57. Applying functions to indexed columns in predicates without expression indexes.
58. Casting indexed columns in predicates and defeating index usage.
59. Comparing dates by wrapping columns in `DATE(column)` instead of using ranges.
60. Using local-time date boundaries for global users without timezone handling.
61. Using `BETWEEN` incorrectly for half-open time intervals.
62. Writing `OR` predicates that defeat indexes without checking plans.
63. Writing huge `IN` lists instead of staging data/joining when appropriate.
64. Using correlated subqueries with accidental quadratic behavior.
65. Repeating identical subqueries instead of factoring or indexing.
66. Using CTEs as optimization fences based on outdated assumptions without checking current behavior.
67. Assuming the ORM-generated SQL is good because the code is pretty.
68. Not testing query behavior on duplicate, null, missing, and concurrent data.
69. Not testing query plans on representative cardinality.

## 8. Backup, recovery, and operations behavior that is never excusable

1. Running a production database with no backups.
2. Having backups but never testing restore.
3. Having backups but not knowing the recovery point objective.
4. Having backups but not knowing the recovery time objective.
5. Assuming `pg_dump` is sufficient for every reliability requirement.
6. No point-in-time recovery plan for important production data.
7. No WAL archiving where the recovery objective requires it.
8. Storing backups in the same failure domain as the database.
9. Keeping unencrypted dumps on developer laptops.
10. Running destructive maintenance without a recent verified backup.
11. Running `DROP`, `TRUNCATE`, mass `DELETE`, or mass `UPDATE` in production with no transaction guard, preview query, backup, or rollback plan.
12. No monitoring for replication lag, disk growth, locks, deadlocks, slow queries, connection saturation, autovacuum lag, and failed backups.
13. No alerting for failed migrations or invalid indexes.
14. No connection pooling under high connection counts.
15. Letting every app instance open unbounded database connections.
16. Running ad hoc analytics against primary production during peak traffic.
17. Letting BI tools run unbounded queries on the primary database.
18. Giving notebooks direct production write access.
19. Testing disaster recovery for the first time during a disaster.

## 8. Backup, restore, disaster-recovery, and operations bad behavior

1. **No backups.**
2. **Backups but no restore tests.**
3. **Restore tests that do not use the real restore procedure.**
4. **No documented RPO and RTO.**
5. **No point-in-time recovery for critical systems.** PostgreSQL docs describe continuous archiving/PITR using WAL plus base backups and warn that a successful recovery requires a continuous sequence of archived WAL files.
6. **Confusing replication with backup.** Replication copies mistakes, deletes, corruption, and bad migrations quickly.
7. **WAL archiving not monitored.** PostgreSQL docs warn that archive failures can fill `pg_wal` and lead to shutdown.
8. **Archived WAL that can be overwritten or silently lost.**
9. **Backups stored in the same failure domain as the primary database.**
10. **Backups not encrypted.**
11. **Backup credentials too broad.**
12. **No access audit for backups.**
13. **No retention policy.**
14. **Retention shorter than legal, business, or incident-detection needs.**
15. **No corruption detection.**
16. **No checks that backups contain expected databases, schemas, tables, and row counts.**
17. **No monitoring for disk growth.**
18. **No monitoring for replication lag.**
19. **No monitoring for failed autovacuum, wraparound risk, bloat, long transactions, locks, deadlocks, and slow queries.**
20. **No alert for connection saturation.**
21. **No alert for backup failure.**
22. **No alert for WAL archive failure.**
23. **No alert for migration failure.**
24. **No runbook for restore.**
25. **No runbook for accidental table drop/truncate.**
26. **No runbook for bad migration rollback.**
27. **No runbook for credential compromise.**
28. **No runbook for replica promotion/failover.**
29. **No disaster-recovery drill.**
30. **No separation between production, staging, and development databases.**
31. **Developers using production as staging.**
32. **Testing destructive scripts first in production.**
33. **No audit trail for production DDL/DML.**
34. **No owner for database health.**
35. **No capacity planning.**
36. **No version-upgrade plan.**
37. **Running unsupported PostgreSQL versions without a remediation plan.**
38. **Extensions installed without review, version pinning, or upgrade strategy.**
39. **No extension inventory.**
40. **Unreviewed `CREATE EXTENSION` in migrations.**
41. **No collation/locale strategy.**
42. **No plan for major-version upgrades, reindexing needs, or changed planner behavior.**
43. **No test for restore compatibility after PostgreSQL upgrades.**
44. **No security patch process.**

## 8. Index and performance bad behavior

1. Add no index for a high-traffic foreign key, join key, filter, uniqueness rule, or ordering requirement.
2. Add indexes blindly to every column.
3. Add duplicate or near-duplicate indexes.
4. Keep unused indexes forever.
5. Add wide multicolumn indexes without proving the query pattern.
6. Put columns in a multicolumn index in a random order.
7. Create partial indexes whose predicate does not match the actual query.
8. Create expression indexes without forcing queries to use the same expression.
9. Forget that every index slows writes and consumes storage.
10. Ignore index bloat.
11. Ignore table bloat.
12. Disable autovacuum as a “performance fix.”
13. Leave dead tuples, wraparound risk, and vacuum failures unmonitored.
14. Fail to run `ANALYZE` or maintain statistics after large data changes.
15. Ignore skewed distributions and correlation.
16. Use development data distribution to judge production plans.
17. Add indexes in production without understanding locks and build behavior.
18. Build a non-concurrent index on a large live table without a maintenance plan.
19. Assume `CREATE INDEX CONCURRENTLY` is magic and cannot fail, bloat, or take a long time.
20. Ignore write amplification from many indexes.
21. Update the same hot row or counter at high frequency; GitLab warns this queues on row locks, saturates connection pools, creates new row versions, and increases vacuum/WAL pressure.
22. Put frequently updated fields and large cold fields in the same hot row.
23. Use wide tables for hot workloads without considering page density, vacuum, and update cost.
24. Run analytics, exports, or dashboards against the primary OLTP database during peak traffic without resource isolation.
25. Let dashboards poll expensive queries every few seconds.
26. Ignore replication lag caused by heavy writes or long queries.
27. Ignore sort/hash memory spills.
28. Ignore connection storms and `max_connections` exhaustion.
29. Use no connection pool for high-concurrency apps.
30. Use a pool so large it DoSes the database.
31. Run with no `statement_timeout`, `lock_timeout`, or idle transaction timeout.
32. Treat a single successful benchmark as proof of scalability.

## 8. Privacy, compliance, and multi-tenant behavior that is never justified

1. Mixing tenants in the same tables without tenant keys, constraints, policies, indexes, and tests.
2. Trusting application code alone to enforce tenant isolation where the database can enforce or help enforce it.
3. Allowing support/admin queries without audit logs.
4. Allowing privileged staff to query sensitive data without purpose and approval controls.
5. Copying production PII into dev laptops.
6. Copying production PII into AI prompts.
7. Copying production PII into analytics sandboxes without masking or authorization.
8. Logging PII unnecessarily.
9. Logging full query parameters when they contain secrets, tokens, emails, phone numbers, addresses, or health/financial data.
10. Storing secrets in ordinary business tables without access separation.
11. Failing to classify sensitive columns.
12. Failing to know where sensitive data is duplicated.
13. Failing to delete or anonymize data according to retention policy.
14. Soft-deleting data that policy requires to be purged.
15. Hard-deleting data that policy requires to be retained or auditable.
16. Not modeling consent, retention, jurisdiction, or data subject rights where required.
17. Giving analytics roles access to raw customer identifiers when aggregates or masked views would do.
18. Allowing ad hoc joins that re-identify masked data.
19. Treating backups, replicas, logs, exports, and data warehouses as outside privacy scope.
20. Creating “temporary” exports that become permanent uncontrolled datasets.

## 8. Transaction and concurrency negligence

1. Splitting a multi-step invariant across multiple transactions.
2. Doing “check then insert” without a unique constraint or lock.
3. Assuming read-then-write logic is safe under concurrency.
4. Ignoring isolation levels.
5. Assuming default `READ COMMITTED` protects business invariants that require serial behavior.
6. Not retrying serialization failures.
7. Not retrying deadlocks where retry is the correct strategy.
8. Catching deadlock/serialization errors and pretending the operation succeeded.
9. Using `SELECT ... FOR UPDATE` without understanding lock scope.
10. Locking rows in inconsistent order across code paths.
11. Taking table locks casually.
12. Leaving transactions open across network calls.
13. Leaving transactions open across user interaction.
14. Leaving transactions open while doing file I/O or API calls.
15. Leaving sessions `idle in transaction`.
16. Holding locks while doing slow app work.
17. Running migrations while long app transactions are open.
18. Doing batch updates in one massive transaction.
19. Doing queue processing without atomic claim semantics.
20. Building job queues with `SELECT` then `UPDATE` races.
21. Using advisory locks without namespace discipline.
22. Using advisory locks without timeouts.
23. Using advisory locks and failing to release them safely.
24. Using advisory locks as a substitute for constraints.
25. Using distributed locks without understanding DB transaction boundaries.
26. Manually incrementing counters through read-modify-write races.
27. Generating IDs with `SELECT max(id) + 1`.
28. Assuming autocommit means no transaction behavior exists.
29. Mixing ORM implicit transactions with explicit SQL carelessly.
30. Ignoring lock wait metrics.

## 9. Indexing and planner malpractice

1. No indexes on common filter predicates.
2. No indexes on common join keys.
3. No indexes supporting foreign-key child lookups where deletes/updates/checks will suffer.
4. No index supporting high-volume backfills or cleanup jobs.
5. Adding indexes blindly without `EXPLAIN`.
6. Adding indexes blindly without checking write overhead.
7. Indexing every column.
8. Keeping duplicate indexes.
9. Keeping indexes with same leading columns and no reason.
10. Creating composite indexes in the wrong column order.
11. Creating single-column indexes where a composite index is needed.
12. Creating composite indexes where single-column or partial indexes would be better.
13. Not using partial indexes for sparse predicates.
14. Not using expression indexes for expression predicates.
15. Not using GIN/GiST/BRIN where the data type and workload call for them.
16. Using the wrong operator class.
17. Indexing huge text/json fields without understanding size and write cost.
18. Creating indexes that are never used and never reviewed.
19. Dropping indexes without checking constraints or query workload.
20. Relying on local tiny data to judge index needs.
21. Never running `ANALYZE` after large data changes.
22. Ignoring stale statistics.
23. Ignoring table bloat.
24. Disabling autovacuum because it is “annoying.”
25. Globally weakening autovacuum without table-level evidence.
26. Letting transaction ID wraparound risk build.
27. Running `VACUUM FULL` on live production without a downtime/lock plan.
28. Ignoring `pg_stat_statements`.
29. Not collecting slow query logs.
30. Not comparing before/after query plans.
31. Optimizing a query without representative data distribution.
32. Shipping a performance fix without measuring p95/p99 impact.
33. Adding an index in production without considering lock behavior.
34. Creating a normal index on a large live table when concurrent creation or a maintenance window is required.
35. Ignoring failed concurrent index builds and leaving invalid indexes around.

## 9. Maintainability behavior that is never justified

1. No naming convention.
2. No migration convention.
3. No index naming convention.
4. No foreign-key naming convention.
5. No ownership for schema changes.
6. No database review for dangerous changes.
7. No comments or docs for non-obvious constraints, partial indexes, triggers, RLS, partitioning, or materialized views.
8. Unreadable 400-line queries with no decomposition, tests, or explanation.
9. Business logic scattered randomly across app code, triggers, functions, jobs, and reports.
10. Hidden trigger side effects that surprise normal writers.
11. Silent trigger failures or swallowed exceptions.
12. Views that look like tables but hide expensive or security-sensitive logic.
13. Materialized views that look fresh but are stale.
14. Columns whose names no longer match their meaning.
15. Tables whose names no longer match their contents.
16. Keeping dead tables, dead columns, and dead indexes forever because nobody knows whether they are used.
17. No dependency tracking for downstream reports, jobs, exports, or APIs.
18. No ERD or schema map for a complex domain.
19. No glossary for overloaded words like account, user, customer, tenant, organization, subscription, status, active, deleted, archived.
20. Abbreviations only the original developer understands.
21. Inconsistent singular/plural naming.
22. Inconsistent timestamp names and meanings.
23. Inconsistent timezone policy.
24. Inconsistent ID types across related tables.
25. Inconsistent soft-delete columns: `deleted`, `is_deleted`, `deleted_at`, `archived`, `active`.
26. Inconsistent audit columns.
27. Unversioned SQL files.
28. SQL embedded in many places with no tests.
29. SQL generated by string builders that nobody can inspect.
30. SQL hidden behind ORM calls where the generated SQL is never reviewed.

## 9. Partitioning, sharding, and scaling behavior that is never excusable

1. Partitioning because a table is “big” without knowing query patterns.
2. Not partitioning when lifecycle, retention, or pruning requirements clearly need it.
3. Partitioning by a key that most queries do not filter on.
4. Creating thousands of partitions without understanding planning and maintenance cost.
5. Forgetting indexes on partitions.
6. Forgetting constraints on partitions.
7. Assuming partitioning automatically improves every query.
8. Using old-style inheritance partitioning for new designs when native partitioning is appropriate.
9. Sharding before the single-node design is correct.
10. Sharding without a resharding strategy.
11. Sharding without cross-shard consistency rules.
12. Sharding without tenant placement, backup, restore, and migration plans.
13. Using table inheritance for ordinary modeling.
14. Using inheritance for partitioning in new PostgreSQL designs when declarative partitioning fits.

## 9. PostgreSQL-specific “don’t do this” behavior

1. **Do not use SQL_ASCII for ordinary databases.**
2. **Do not use `psql -W` in scripts; it forces password prompting behavior that is usually wrong for automation.**
3. **Do not use PostgreSQL rules for normal business behavior; use triggers where server-side behavior is required.**
4. **Do not use table inheritance as a casual modeling feature.**
5. **Do not use `NOT IN` with nullable subqueries/lists.**
6. **Do not use uppercase or quoted identifiers casually.**
7. **Do not use `BETWEEN` for timestamp intervals.**
8. **Do not use `timestamp without time zone` for real instants.**
9. **Do not store UTC timestamps in `timestamp without time zone`.**
10. **Do not use `timetz`.**
11. **Do not use `CURRENT_TIME`.**
12. **Do not use `timestamp(0)` / `timestamptz(0)`.**
13. **Do not use text UTC offsets as time-zone names.**
14. **Do not use `char(n)`.**
15. **Do not use arbitrary `varchar(n)` as fake validation.**
16. **Do not use `money` casually.**
17. **Do not use `serial` for new designs when identity columns are the modern PostgreSQL feature.**
18. **Do not use `trust` authentication for production TCP/IP access.**

## 9. Transactions, locking, and concurrency

1. Split a multi-step business invariant across statements without a transaction.
2. Hold a transaction open across network calls, user input, file uploads, API calls, sleeps, or queue waits.
3. Leave sessions `idle in transaction`.
4. Do `SELECT` then `INSERT` to enforce uniqueness without a unique constraint or `INSERT ... ON CONFLICT`.
5. Do `SELECT` then `UPDATE` assuming no one else can change the row.
6. Read-modify-write counters without atomic SQL, row locking, or a contention-aware design.
7. Ignore deadlocks instead of retrying at the transaction boundary.
8. Use serializable isolation without retry logic.
9. Assume `READ COMMITTED` gives repeatable reads.
10. Assume transactions prevent all race conditions by default.
11. Use `SELECT ... FOR UPDATE` without understanding lock scope.
12. Lock rows in inconsistent order across code paths.
13. Lock more rows than needed.
14. Use table locks in application request paths.
15. Run DDL in long transactions.
16. Run bulk updates/deletes without batching, progress markers, and lock limits.
17. Use `SKIP LOCKED` without accepting that skipped work needs explicit handling.
18. Use advisory locks as hidden global mutexes without timeout, naming, release, and pooling rules.
19. Use session-level advisory locks in pooled connections without guaranteed release.
20. Mix transaction pooling and session-dependent features carelessly.
21. Ignore lock waits in observability.
22. Retry non-idempotent transactions without idempotency keys.
23. Commit partial business operations because “the rest of the job will fix it later.”
24. Use cron jobs that overlap and mutate the same data without locking or idempotency.

## A compact merge-gate checklist

1. What invariant is this enforcing, and is it enforced in the database where possible?
2. What data volume exists today, and what volume is expected later?
3. What locks will this take?
4. What is the rollback or forward-fix?
5. What happens if it fails halfway?
6. What is the query plan on realistic data?
7. What indexes support the query and constraints?
8. What credentials/roles can run this?
9. What sensitive data is read, written, logged, exported, or retained?
10. What concurrency race would break this?
11. What monitoring proves it is safe after deploy?
12. What restore path exists if the worst case happens?

## AI & Vibe-Coding

1. Ship AI-generated SQL, DDL, DML, functions, policies, or migrations without human review by someone who understands the schema.
2. Let an AI agent connect to production with write, DDL, owner, superuser, replication, `BYPASSRLS`, or unrestricted migration privileges.
3. Auto-execute text-to-SQL output from a user prompt against a live database.
4. Trust an AI-generated tenant filter, authorization predicate, RLS policy, or permission check without tests proving isolation.
5. Accept hallucinated table names, columns, constraints, indexes, or migration safety claims.
6. Run destructive SQL from chat output: `DROP`, `TRUNCATE`, broad `DELETE`, broad `UPDATE`, `ALTER TABLE`, `VACUUM FULL`, `REINDEX`, `CLUSTER`, privilege changes, or policy changes.
7. Treat “the query returned plausible rows” as validation.
8. Let an AI generate migrations without lock analysis, production-like data testing, rollback, batching, and monitoring.
9. Give AI tools raw production credentials, secrets, customer PII, unmasked dumps, or query logs containing sensitive values.
10. Allow AI-generated SQL to bypass parameterization because “the model wrote safe input handling.”
11. Allow generated SQL to run without `statement_timeout`, `lock_timeout`, least-privilege roles, audit logging, and blast-radius limits.
12. Let AI-generated SQL change schema and application code in one step without compatibility across rolling deploys.
13. Use generated anonymization, deletion, encryption, or retention logic without proving it satisfies legal and business requirements.
14. Hide AI-authored SQL changes from normal code review because “it is only a migration.”
15. Permit AI tools to “fix” production data without an approved runbook and before/after verification.
16. Accepting AI-generated SQL because it “looks right.”
17. Accepting AI-generated migrations without checking lock levels.
18. Accepting AI-generated migrations without checking reversibility.
19. Accepting AI-generated indexes without checking workload.
20. Accepting AI-generated constraints without checking existing dirty data.
21. Accepting AI-generated `DROP`, `CASCADE`, `TRUNCATE`, or mass `UPDATE`.
22. Prompting an AI without giving it actual schema, constraints, indexes, row counts, and Postgres version.
23. Letting AI invent table names or column names.
24. Letting AI infer cardinality instead of verifying it.
25. Letting AI choose data types without domain requirements.
26. Letting AI normalize or denormalize without workload requirements.
27. Letting AI design multi-tenant security without threat modeling.
28. Letting AI write RLS policies without adversarial tests.
29. Letting AI write SECURITY DEFINER functions without search-path hardening.
30. Letting AI write dynamic SQL without injection review.
31. Letting AI write migrations that use ORM models that may drift.
32. Letting AI write backfills with no batching.
33. Letting AI write queries with `SELECT *`.
34. Letting AI write queries with `LIMIT 1` instead of constraints.
35. Letting AI write queries that rely on row order.
36. Letting AI write app-level uniqueness checks instead of DB constraints.
37. Letting AI use JSONB as a junk drawer.
38. Letting AI remove foreign keys “for performance.”
39. Letting AI remove indexes “for write speed” without evidence.
40. Letting AI add indexes “for performance” without plans.
41. Letting AI suggest `VACUUM FULL` on production without lock review.
42. Letting AI suggest increasing `max_connections` without pool/capacity review.
43. Letting AI convert SQL semantics across dialects without checking Postgres behavior.
44. Letting AI generate SQL from screenshots of schema instead of actual DDL.
45. Treating green tests on toy data as database correctness.
46. Treating a successful local migration as production safety.
47. Not asking the AI: “What locks does this take?”
48. Not asking the AI: “What invariant is enforced by the database?”
49. Not asking the AI: “What happens under two concurrent requests?”
50. Not asking the AI: “What happens if this migration is interrupted?”
51. Not asking the AI: “How do we roll forward if rollback is impossible?”
52. Not checking AI output against official Postgres docs.
53. Not checking AI output with `EXPLAIN`.
54. Not checking AI output with representative data.
55. Not checking AI output with adversarial null/duplicate/tenant cases.
56. Pasting AI SQL directly into production.
57. Using AI to bypass code review.
58. Using AI to bypass database review.
59. Using AI to produce fake confidence instead of evidence.
60. Prompting an LLM for SQL and running it directly on production.
61. Prompting an LLM for a migration and applying it without review.
62. Letting an AI agent infer the schema from vague names.
63. Letting an AI agent invent columns, indexes, constraints, enum values, or tables.
64. Letting an AI agent “fix” a slow query by adding indexes until the plan looks better locally.
65. Letting an AI agent use production credentials.
66. Letting an AI agent connect as superuser.
67. Letting an AI agent perform DDL automatically.
68. Letting an AI agent perform destructive DML automatically.
69. Letting an AI agent run `DROP`, `TRUNCATE`, `DELETE`, `UPDATE`, `ALTER`, `CREATE INDEX`, or `VACUUM FULL` against production.
70. Accepting AI SQL without checking `NULL` behavior.
71. Accepting AI SQL without checking join cardinality.
72. Accepting AI SQL without checking tenant isolation.
73. Accepting AI SQL without checking constraints.
74. Accepting AI SQL without checking indexes and query plans.
75. Accepting AI SQL without checking lock behavior.
76. Accepting AI SQL without checking migration ordering.
77. Accepting AI SQL without checking rollback.
78. Accepting AI SQL without tests.
79. Accepting AI SQL because it “looks professional.”
80. Asking an LLM to design a schema and accepting the first answer.
81. Asking an LLM to normalize or denormalize without providing workload, constraints, retention, and consistency requirements.
82. Asking an LLM to “make it faster” without providing `EXPLAIN ANALYZE`, table sizes, indexes, statistics, and query frequency.
83. Pasting production data into prompts.
84. Pasting credentials, dumps, stack traces, or logs containing personal data into prompts.
85. Using AI-generated fake data that violates real constraints, then designing around the fake data.
86. Treating database design as a UI scaffolding task.
87. Treating migrations as text edits rather than operational events.
88. **Not checking whether the generated SQL is PostgreSQL dialect rather than MySQL, SQLite, SQL Server, or generic SQL.** Recent AI/PostgreSQL community work argues that LLMs often learn SQL from mixed internet examples, burying idiomatic PostgreSQL behavior.
89. **Generated schema with no primary keys.**
90. **Generated schema with no foreign keys.**
91. **Generated schema with no unique constraints.**
92. **Generated schema with no `NOT NULL` constraints.**
93. **Generated schema with no `CHECK` constraints for obvious domains.**
94. **Generated migration that creates tables but no indexes for expected access paths.**
95. **Generated query merged without `EXPLAIN`.**
96. **Generated migration run against production before staging.**
97. **Generated data migration with no batching.**
98. **Generated destructive migration with no rollback.**
99. **Generated code that interpolates variables into SQL strings.**
100. **Generated dynamic SQL in PL/pgSQL without `USING`, `format('%I', ...)`, `format('%L', ...)`, or quote functions as appropriate.**
101. **Generated SQL that uses features from the wrong database engine.**
102. **Generated SQL that ignores time zones.**
103. **Generated SQL that uses `SELECT *`.**
104. **Generated SQL that uses offset pagination on large mutable tables.**
105. **Generated SQL that turns search into `ILIKE '%term%'` with no trigram/full-text plan.**
106. **Generated schema that uses JSONB for everything.**
107. **Generated schema that uses EAV for normal entities.**
108. **Generated migrations that assume empty tables.**
109. **Generated migrations that assume clean legacy data.**
110. **Generated SQL with no tests for nulls, duplicates, concurrent writes, tenant isolation, and boundary dates.**
111. **Prompting the model to “just make it work” and accepting hidden denormalization, silent casts, or lossy transformations.**
112. **Letting an AI agent have production write access without a human approval gate.**
113. **Letting an AI agent run `DROP`, `TRUNCATE`, `ALTER`, `DELETE`, or `UPDATE` outside a reviewed transaction/change workflow.**
114. **No schema diff after AI-generated migrations.**
115. **No review of generated indexes.**
116. **No review of generated constraints.**
117. **No review of generated permissions.**
118. **No review of generated rollback path.**
119. **No review of generated query plans.**
120. **Trusting an AI’s verbal explanation over the actual SQL.**
121. **Copying AI-generated “best practices” that contradict PostgreSQL docs.**
122. **Using AI to generate production data-fix scripts without before/after validation queries.**
123. **No golden tests for generated SQL behavior.**
124. **No fixture data covering duplicate, null, deleted, tenant-crossing, and timezone edge cases.**
125. **No static analysis, migration lint, or SQL formatting in an AI-heavy workflow.**
126. **Treating database design as reversible just because code generation is fast.**
127. Accepting AI-generated SQL, DDL, migrations, triggers, functions, or RLS policies without reading every line.
128. Letting an LLM execute SQL directly against production or enterprise data without deterministic validation, catalog binding, permissions checks, risk checks, and audit logs.
129. Treating “the query ran” as proof that the query is correct.
130. Prompting an AI with partial schema context and trusting it not to hallucinate tables, columns, joins, constraints, or relationships.
131. Not specifying **PostgreSQL**, version, extensions, timezone assumptions, tenant model, and relevant schema in database-related prompts.
132. Copying generated SQL from another dialect—MySQL, SQL Server, SQLite, Oracle—into Postgres without dialect review.
133. Accepting generated joins without verifying cardinality and whether the relationship is actually enforced by a foreign key or unique constraint.
134. Accepting generated aggregates without hand-checking grouping, deduplication, time filters, and tenant filters.
135. Running generated migrations without a rollback/roll-forward plan.
136. Running generated DDL against production before trying it on a production-like copy.
137. Using an AI-generated migration that says “drop and recreate” when the table contains real data.
138. Using AI to “fix” a failing migration by deleting constraints, dropping data, disabling triggers, or disabling foreign keys.
139. Letting an LLM choose indexes without checking real query plans and real workload.
140. Letting an LLM choose data types without validating domain semantics.
141. Letting an LLM invent business rules that are not encoded in product requirements.
142. Treating generated tests that only cover the happy path as sufficient for SQL correctness.
143. Failing to test generated SQL against edge cases: no rows, duplicate rows, NULLs, multiple tenants, daylight saving time, concurrent writes, deleted rows, and unexpected cardinality.
144. Allowing natural-language-to-SQL systems to touch sensitive data without a policy layer.
145. Allowing read-only LLM SQL to bypass tenant, row-level, purpose-of-use, or sensitive-field policies.
146. Failing to log who requested generated SQL, what SQL was generated, what policies were applied, and why the query was allowed or denied.
147. Asking an AI for “the schema” without providing cardinality, invariants, retention, security, tenant model, read/write paths, and growth assumptions.
148. Accepting an AI-generated schema because it “looks normal.”
149. Accepting AI-generated SQL that uses syntax from another database.
150. Accepting AI-generated SQL without checking the PostgreSQL version.
151. Accepting AI-generated migrations without lock analysis.
152. Accepting AI-generated migrations without rollback/forward-fix.
153. Accepting AI-generated indexes without query plans.
154. Accepting AI-generated `DROP`, `TRUNCATE`, or bulk `UPDATE` statements without dry run.
155. Accepting AI-generated `CASCADE`.
156. Accepting AI-generated permissions/RLS policies without adversarial tests.
157. Accepting AI-generated crypto/password/storage code.
158. Asking the AI to “make it faster” and adding every suggested index.
159. Asking the AI to “simplify the schema” and losing constraints.
160. Asking the AI to “avoid joins” and creating JSON/EAV garbage.
161. Asking the AI to “fix migration failure” and manually mutating migration state.
162. Asking the AI to “clean duplicate data” without first enforcing the invariant that allowed duplicates.
163. Letting AI-generated code use `SELECT *`.
164. Letting AI-generated code build SQL strings with template literals/f-strings.
165. Letting AI-generated code catch and ignore database exceptions.
166. Letting AI-generated code retry writes without idempotency.
167. Letting AI-generated code open one DB connection per request/job/function.
168. Letting AI-generated code create background jobs that scan entire tables.
169. Letting AI-generated code put secrets in `.env.example`, logs, migrations, or comments.
170. Letting AI-generated code create “temporary” admin bypasses.
171. Letting AI-generated code invent audit/compliance behavior that was never verified.
172. Treating “the AI explained it” as equivalent to understanding it.
173. Treating “tests pass” as sufficient when tests use tiny data and no concurrency.
174. Treating “works locally” as sufficient for DDL.
175. Treating “the ORM generated it” as sufficient for DDL.
176. Treating “the database is managed” as sufficient for backups, restore, security, and query design.

## Anti-relational designs

1. Comma-separated IDs in a column.
2. JSON arrays of foreign keys instead of a join table.
3. Repeating columns like `phone1`, `phone2`, `phone3`.
4. Repeating attribute columns like `tag_1`, `tag_2`, `tag_3`.
5. Table-per-month/year/customer/tenant when native partitioning or a tenant key is the correct model.
6. One “god table” for unrelated entity types.
7. One “god lookup table” for unrelated domains where foreign keys become meaningless.
8. Entity-Attribute-Value as the default schema because the model was not understood. Bill Karwin’s *SQL Antipatterns* explicitly groups dangerous antipatterns across logical design, physical design, queries, and application development, and EAV is one of the classic patterns surfaced repeatedly in SQL anti-pattern discussions.
9. JSONB as an excuse to avoid modeling stable, queryable, relational data.
10. Storing structured data as unvalidated text blobs.
11. Storing numbers, booleans, timestamps, or money as text.
12. Storing multiple meanings in one column.
13. Storing one concept in multiple columns with inconsistent rules.
14. Duplicating data as if both copies are authoritative.
15. Denormalizing before proving the normalized model cannot meet requirements.
16. Normalizing into unusability without understanding access patterns.
17. Creating a schema directly from UI forms rather than domain entities and invariants.
18. Letting ORM defaults define the data model without database review.
19. “We can migrate later” as an excuse for a schema that cannot enforce today’s truth.
20. No ERD, relationship map, or written invariants for a nontrivial schema.

## Anti-relational schema design

1. Storing comma-separated IDs in a text column.
2. Storing pipe-delimited, JSON-delimited, or array-packed foreign keys instead of using a join table.
3. Storing multiple logical values in one scalar column.
4. Creating “list of IDs” columns that cannot be constrained with foreign keys.
5. Creating many-to-many relationships without a junction table and appropriate uniqueness.
6. Using entity-attribute-value design for core business entities just to avoid migrations.
7. Using one giant `jsonb` column for relational data that needs constraints, joins, reporting, or indexing.
8. Creating one table per customer, tenant, month, status, region, or type without a partitioning or lifecycle design.
9. Creating a “god table” with hundreds of nullable unrelated columns.
10. Creating polymorphic associations where the database cannot enforce the referenced target.
11. Using a generic `object_id` that can point to anything.
12. Using a generic `type` plus `id` pair instead of enforceable foreign keys.
13. Encoding values in table names or column names instead of rows.
14. Creating a column per day, per month, per feature flag, per metric, or per user.
15. Using arrays or JSON to hide normalization problems.
16. Repeating groups instead of child tables.
17. Duplicating data across tables without a single source of truth, synchronization rule, or reconciliation job.
18. Creating “shadow tables” manually maintained by application code with no consistency guarantees.

## Bad keys and identity design

1. Using a surrogate `id` while allowing duplicate real-world business records.
2. Failing to define the natural/business uniqueness of an entity.
3. Using mutable external provider IDs as the only identity.
4. Using email as a primary key when email changes are allowed.
5. Using names, titles, slugs, or display labels as permanent identifiers.
6. Using globally meaningful IDs without tenant scoping in multitenant systems.
7. Forgetting tenant ID in unique constraints, foreign keys, indexes, and row-level policies.
8. Letting two tenants accidentally reference each other’s rows.
9. Using UUIDs as a substitute for authorization or tenant isolation.
10. Treating sequence gaps as errors.
11. Exposing internal sequential IDs in places where enumeration is a security risk without separate public identifiers.

## Broken authentication and authorization

1. Using `trust` authentication over TCP/IP.
2. Letting application code connect as `postgres`, a superuser, or the schema owner.
3. Sharing one powerful database account across humans, services, migrations, and dashboards.
4. Granting broad privileges to `PUBLIC` without deliberate review.
5. Granting `ALL` because the exact needed privileges are inconvenient to identify.
6. Letting web applications run migrations with the same role they use for normal traffic.
7. Giving read-only tools write privileges.
8. Giving background jobs DDL privileges.
9. Running ad hoc production queries as a superuser.
10. Using `SECURITY DEFINER` functions without a locked-down `search_path` and privilege review.
11. Granting `BYPASSRLS` casually.
12. Enabling row-level security but forgetting that table owners and superusers can bypass it unless configured carefully.
13. Depending only on application-side tenant filters when the database can enforce tenant isolation.
14. Leaving default privileges unreviewed after creating new schemas, tables, functions, or extensions.
15. Using hard-coded credentials in application code, migration scripts, notebooks, shell history, CI logs, or test fixtures.

## Dangerous production index operations

1. Creating a regular index on a large hot table in production without understanding locks.
2. Reindexing large production tables casually.
3. Creating multiple heavyweight indexes at once during peak traffic.
4. Running index builds with no `lock_timeout` or `statement_timeout`.
5. Ignoring failed `CREATE INDEX CONCURRENTLY` artifacts.
6. Leaving invalid indexes behind.
7. Assuming an index is used because it exists.
8. Not checking `pg_stat_user_indexes` or query plans for index usage.
9. Not measuring index impact on writes.
10. Treating indexes as a substitute for correct schema design.

## Encoding and collation mistakes

1. Using `SQL_ASCII` for a new database.
2. Mixing encodings in one database.
3. Assuming case-insensitive uniqueness without enforcing it.
4. Using `lower(email)` in queries without a matching unique expression index or `citext`/collation plan.
5. Ignoring collation behavior in sorting, uniqueness, and upgrades.
6. Letting application code normalize strings differently from the database.

## Expand/contract failures

1. Deploying code that requires a new column before the column exists everywhere.
2. Dropping a column while old app versions still read it.
3. Reusing a column for a different meaning.
4. Reusing a table for a different entity.
5. Reusing enum values for different semantics.
6. Changing a constraint and assuming all services understand the new rule.
7. Making incompatible changes to views consumed by downstream systems.
8. Breaking analytics, exports, jobs, or replicas because only the web app was checked.
9. Skipping deprecation periods for public database contracts.

## Incorrect SQL semantics

1. Using `NOT IN` when the subquery can return `NULL`.
2. Using `NULL = NULL` logic as if it were true.
3. Treating `NULL`, empty string, zero, empty array, and missing JSON key as interchangeable.
4. Using `WHERE column = NULL` instead of `IS NULL`.
5. Using `COUNT(column)` when `COUNT(*)` or a different expression is intended.
6. Accidentally turning a `LEFT JOIN` into an inner join by putting right-table filters in the `WHERE` clause.
7. Writing joins without explicit join predicates.
8. Hiding accidental duplicate rows with `DISTINCT`.
9. Using `GROUP BY` to mask a broken join.
10. Aggregating after joining tables at different granularities and double-counting.
11. Using `UNION` when `UNION ALL` is intended.
12. Using `ORDER BY random()` on large tables.
13. Using window functions without deterministic ordering.
14. Comparing floating-point values for exact equality.
15. Using implicit casts that change semantics or defeat indexes.
16. Assuming JSON missing key, JSON null, SQL null, and empty string are the same thing.
17. Assuming `LIKE` case behavior without checking collation and operator semantics.

## Index behavior that is not excusable

1. Adding no index for a hot lookup.
2. Adding an index without knowing which query it supports.
3. Adding indexes until performance improves without measuring write cost, storage, bloat, and planner usage.
4. Creating redundant indexes that differ only trivially.
5. Indexing every column.
6. Missing composite indexes for multi-column filters.
7. Composite indexes in the wrong column order.
8. Assuming `(a, b)` supports efficient lookup on `b` alone.
9. Missing partial indexes for hot selective subsets.
10. Using a partial index that cannot support the actual predicate.
11. Forgetting that foreign-key referencing columns are not automatically indexed.
12. Using a partial index to support cascading deletes on a foreign key. GitLab explicitly says partial indexes cannot serve that purpose.
13. Adding a unique index after duplicates already exist, with no cleanup plan.
14. Adding non-unique indexes when the real requirement is uniqueness.
15. Failing to verify index validity after `CREATE INDEX CONCURRENTLY`. GitLab’s process explicitly includes checking that asynchronous/concurrent indexes exist and are not invalid.
16. Leaving invalid indexes around.
17. Creating huge indexes during peak traffic.
18. Dropping indexes without checking query dependencies.
19. Dropping indexes that support constraints or foreign keys.
20. Ignoring index bloat and vacuum behavior.
21. Using GIN/GiST/BRIN because they sound powerful without understanding operator classes and workload.
22. Creating expression indexes where the application query does not exactly match the expression.
23. Ignoring collation/case-sensitivity requirements.
24. Adding indexes in the same release as code that depends on them when the index may not exist before the code runs. GitLab recommends splitting index creation and dependent application changes when needed.

## Integrity and keys

1. Tables with no primary key or stable unique identifier.
2. Tables where the “real” uniqueness rule exists only in application code.
3. Missing `NOT NULL` on required fields. PostgreSQL’s docs note that most database designs should mark the majority of columns not null.
4. Nullable columns used to dodge modeling decisions.
5. Nullable columns in unique constraints when the business rule means “only one,” but the designer forgot that PostgreSQL treats nulls as distinct unless `NULLS NOT DISTINCT` is used.
6. No foreign keys for real relationships inside the same database, followed by hand-written cleanup jobs to chase orphans.
7. Foreign keys with no supporting index on the referencing side when deletes/updates can happen. PostgreSQL notes that referenced columns are indexed, but referencing columns are not automatically indexed; GitLab requires indexes for foreign keys because missing ones have caused timeout incidents.
8. Polymorphic foreign keys such as `(target_type, target_id)` that can point to many tables with no real referential integrity.
9. Dual-purpose foreign keys where one column can reference different tables depending on another column.
10. “Soft foreign keys” documented in comments but not enforced, monitored, or repaired.
11. Cascading deletes added casually without proving the child rows are true components of the parent.
12. Missing `ON DELETE` decision. Every relationship should choose `RESTRICT`, `NO ACTION`, `CASCADE`, `SET NULL`, or `SET DEFAULT` intentionally.
13. `ON DELETE CASCADE` on relationships where the child is independently meaningful.
14. `ON DELETE SET NULL` on columns that should never be null.
15. Business invariants implemented only in background jobs.
16. Relying on API validation for constraints when imports, admin scripts, migrations, queues, or future services can bypass the API.
17. “We trust the app” as a reason to avoid constraints.
18. Fixing duplicate rows periodically instead of enforcing uniqueness.
19. Allowing orphan rows because “the UI won’t show them.”
20. Adding constraints only after launch with no plan to clean existing bad data.

## Migration and DDL behavior that is not excusable

1. Production migrations with no peer review.
2. Production migrations with no tested staging run.
3. Production migrations with no estimate of affected rows.
4. Production migrations with no lock analysis.
5. Production migrations with no query-plan analysis for validation/backfill queries.
6. Production migrations with no rollback, forward-fix, or restore plan.
7. Production migrations with no monitoring plan.
8. Running schema changes manually instead of through versioned migrations.
9. Editing old migrations after they have shipped.
10. Relying on ORM auto-migrate in production without review.
11. Combining risky schema changes and risky data changes in one opaque migration.
12. Long-running migrations inside one giant transaction.
13. Adding a column with a volatile default to a huge table casually.
14. Rewriting a huge table without knowing it will rewrite.
15. Adding `NOT NULL` to a populated table without backfill and validation strategy.
16. Adding foreign keys to large tables without low-lock validation strategy.
17. Dropping a column while old application versions may still reference it.
18. Renaming a column/table during rolling deployments without compatibility shims.
19. Changing a column type without checking casts, indexes, constraints, and rewrite cost.
20. Changing enum/domain/check values without checking old application versions.
21. Adding a constraint `NOT VALID` and never validating it.
22. Backfilling all rows in one statement when batching is required.
23. Deleting data in a migration without a data-deletion label, business approval, and recovery plan. GitLab’s database review guidance treats data migrations as inherently risky and requires extra reversal/recovery detail for potentially destructive changes.
24. Migration code that calls live application models whose behavior may change later.
25. Migration code depending on external APIs.
26. Migration code depending on current wall-clock behavior unless that is explicitly intended.
27. `DROP ... CASCADE` without enumerating what will be dropped.
28. Running migrations during peak traffic when the operation is known to contend.
29. Ignoring failed partial migrations.
30. Re-running `CREATE INDEX CONCURRENTLY IF NOT EXISTS` blindly without checking whether a previous invalid index exists.
31. No `disable_ddl_transaction!` where Postgres requires operations outside a transaction, such as `CREATE INDEX CONCURRENTLY`. GitLab’s migration style guide calls this out directly.
32. Treating “the migration succeeded” as proof the app is safe.

## Missing database-enforced integrity

1. Creating production tables without primary keys.
2. Treating primary keys as “optional because the ORM has IDs.”
3. Referring to another table without a foreign key when both tables live in the same database and the relationship is real.
4. Relying only on application code for referential integrity.
5. Relying only on “check before insert” code for uniqueness.
6. Missing `UNIQUE` constraints for business identities that must be unique.
7. Missing `NOT NULL` constraints for required attributes.
8. Missing `CHECK` constraints for domain rules the database can enforce.
9. Allowing invalid enum/status values because “the frontend validates it.”
10. Allowing negative quantities, impossible dates, invalid percentages, or out-of-range scores when simple constraints can prevent them.
11. Allowing orphan rows and planning to “clean them up later.”
12. Disabling or dropping constraints to make imports easier, then not restoring and validating them.
13. Marking constraints `NOT VALID` and never validating them.
14. Creating nullable foreign keys to avoid modeling the actual lifecycle of the relationship.
15. Using triggers to fake constraints when native constraints would work.

## Missing or wrong indexes

1. Shipping a new high-traffic query without checking its plan.
2. Adding a foreign key without an index strategy for the referencing column.
3. Creating composite indexes with the foreign-key column not usable as the leftmost prefix when cascade/delete checks need it.
4. Using a partial index where a full foreign-key-supporting index is required.
5. Adding indexes only after production incidents.
6. Assuming a primary key index covers every query.
7. Indexing the wrong side of a relationship.
8. Indexing low-cardinality columns blindly.
9. Creating duplicate indexes.
10. Creating indexes that differ only trivially.
11. Creating unused indexes and never removing them.
12. Creating every possible index “just in case.”
13. Ignoring index bloat.
14. Ignoring write amplification from indexes.
15. Creating wide covering indexes without checking write cost.
16. Using the wrong index type for the operator.
17. Using `ILIKE '%term%'` without trigram/full-text/search infrastructure.
18. Using JSONB queries without appropriate GIN/expression indexes.
19. Using expression predicates without expression indexes.
20. Creating indexes on expressions and never running `ANALYZE`.
21. Forgetting that partial indexes only help when the predicate matches.

## Non-sargable and planner-hostile queries

1. Leading-wildcard `LIKE`/`ILIKE` on large tables without trigram/full-text/search architecture.
2. Wrapping indexed columns in functions in predicates without expression indexes.
3. `WHERE lower(email) = ...` without a matching expression index or case-insensitive type strategy.
4. `WHERE date(created_at) = ...` instead of a timestamp range.
5. Giant `OR` chains for optional filters.
6. “Smart” catch-all queries that try to handle every filter combination and perform badly for most of them.
7. Huge `IN` lists generated by application code.
8. Nested subqueries written because the author did not understand joins.
9. Correlated subqueries that run once per row when a join or pre-aggregation is needed.
10. CTEs used as cargo-cult structure without checking planner behavior.
11. Query hints, planner toggles, or disabled join methods used as permanent fixes without root-cause analysis.
12. Relying on ORM-generated SQL without inspecting the actual SQL.
13. Merging a query that has never been run with `EXPLAIN`.
14. Merging a query that has only been tested on tiny seed data.
15. Running `EXPLAIN ANALYZE` on destructive statements outside a transaction/rollback guard.

## Operational behavior that is not excusable

1. No monitoring of locks.
2. No monitoring of long-running queries.
3. No monitoring of replication lag.
4. No monitoring of WAL/archive failures.
5. No monitoring of disk growth.
6. No monitoring of autovacuum health.
7. No monitoring of dead tuples/table bloat.
8. No monitoring of connection saturation.
9. No alerts for backup failures.
10. No restore drills.
11. No runbook for failover.
12. No runbook for point-in-time recovery.
13. No runbook for accidental delete/drop.
14. WAL archiving not monitored. PostgreSQL warns that if archiving falls behind, data-loss exposure increases and `pg_wal` can fill disk, taking the database offline.
15. Letting `pg_wal` fill the disk.
16. Disabling autovacuum globally.
17. Ignoring vacuum starvation from long transactions.
18. Running `VACUUM FULL` casually on hot production tables.
19. Running `REINDEX`, `CLUSTER`, or heavy maintenance without lock/traffic analysis.
20. Unbounded application connections.
21. Serverless/functions opening direct Postgres connections without pooling.
22. No PgBouncer/pool strategy where connection count can spike.
23. No capacity planning.
24. No load testing with realistic cardinality.
25. No staging environment with production-like data distributions.
26. Running unsupported PostgreSQL versions in production without a documented exception and upgrade plan. Current PostgreSQL docs distinguish supported and unsupported versions on the documentation page.
27. Ignoring minor version security/bugfix updates.
28. Ignoring collation/version changes during OS/Postgres upgrades.
29. No extension upgrade plan.
30. No migration rehearsal for major upgrades.
31. Treating replicas as backups.
32. Treating snapshots as backups without restore validation.
33. Assuming managed database means no database engineering is required.
34. No ownership for tables, indexes, jobs, migrations, dashboards, and alerts.
35. No data-retention policy.
36. Infinite growth tables with no archive/partition/delete strategy.
37. Partitioning without understanding pruning, indexes, constraints, and operational overhead.
38. Too many tiny partitions.
39. Partitioning on the wrong key.
40. Archival data and hot OLTP data mixed forever.

## Password and secret handling

1. Storing plaintext passwords.
2. Storing reversibly encrypted passwords when authentication only needs verification.
3. Using unsalted hashes.
4. Using fast general-purpose hashes such as raw MD5 or SHA for passwords.
5. Storing password reset tokens or API tokens in plaintext.
6. Putting production credentials in migration files, seed files, SQL comments, or README examples.
7. Including credentials in connection URLs that are printed to logs.
8. Creating database dumps that contain secrets without encryption, access controls, and retention limits.

## Phrases that should trigger immediate review

1. “We can fix the data later.”
2. “The app validates it.”
3. “No one will call it that way.”
4. “There will only ever be one writer.”
5. “This table will stay small.”
6. “We do not need foreign keys.”
7. “Foreign keys are slow.”
8. “Indexes are always good.”
9. “Sequential scan means bad.”
10. “It worked on my machine.”
11. “It is just a migration.”
12. “It is just a backfill.”
13. “It is just an admin script.”
14. “It is just analytics.”
15. “It is just a nullable column.”
16. “It is just a one-time cleanup.”
17. “It is fine because we have backups.”
18. “The replica is our backup.”
19. “We can add constraints later.”
20. “We can add indexes later.”
21. “We can normalize later.”
22. “We can denormalize later.”
23. “We can shard later.”
24. “We can partition later.”
25. “We can delete old data later.”
26. “Nobody uses that column.”
27. “I checked one row.”
28. “I asked ChatGPT.”
29. “The dashboard seems fine.”
30. “The query usually runs fast.”

## Postgres-specific type/design mistakes

1. Creating new Postgres databases with `SQL_ASCII`. The PostgreSQL wiki warns that `SQL_ASCII` means “no conversions” and can leave mixed encodings that cannot be recovered reliably.
2. Using quoted mixed-case table/column names in normal application schemas.
3. Using reserved words as identifiers.
4. Using names that require quoting everywhere.
5. Inconsistent singular/plural naming that hides relationships.
6. Ambiguous columns like `status`, `type`, `data`, `value`, `flag`, `source`, `kind` without clear domain constraints.
7. Using `char(n)` by habit.
8. Using `varchar(255)` by habit when no actual 255-character business rule exists.
9. Using `money` for monetary systems without understanding locale, rounding, and currency issues. The PostgreSQL wiki recommends `numeric` or sometimes integers instead.
10. Storing money as floating point.
11. Storing currency amount without currency code when multi-currency is possible.
12. Storing “UTC” in `timestamp without time zone` for real instants. The PostgreSQL wiki recommends `timestamptz` for instants in time.
13. Storing offsets like `-07:00` as if they were time zones.
14. Using `timetz` or `CURRENT_TIME` in ordinary application schemas; the PostgreSQL wiki says these are generally not useful.
15. Using timestamp precision like `timestamptz(0)` without realizing it rounds, not truncates.
16. Using `serial` for new schemas when identity columns are available, unless you are intentionally supporting old PostgreSQL versions or a special sequence-sharing case.
17. Assuming sequence values are gapless, meaningful, rollback-safe, or chronological. PostgreSQL documents that sequence changes are visible immediately and are not rolled back on transaction abort.
18. Using table inheritance for ordinary partitioning or object modeling. The PostgreSQL wiki says to avoid table inheritance now that native partitioning exists.
19. Using PostgreSQL rules for business logic instead of triggers or explicit SQL. The PostgreSQL wiki says rules rewrite queries and “don’t do what they look like they do.”
20. Using `bytea`, JSON, or text to hide data that needs constraints, indexes, joins, or permissions.

## Query readability failures

1. Shipping SQL nobody on the team can explain.
2. Shipping complex SQL with no tests for edge cases.
3. Shipping business-critical SQL with no comments explaining non-obvious logic.
4. Using aliases like `a`, `b`, `c`, `t1`, `t2` in complex production queries.
5. Copying Stack Overflow, Reddit, GitHub, or LLM SQL without adapting it to the actual schema and data distribution.
6. Rewriting correct simple SQL into clever SQL to appear advanced.
7. Duplicating complex query fragments across the codebase instead of encapsulating them carefully.
8. Using views to hide complexity without ownership, tests, or performance checks.
9. Using stored procedures as a dumping ground for unreviewed business logic.
10. Creating triggers that silently mutate data in surprising ways.

## Query-writing behavior that is not excusable

1. Writing queries whose correctness depends on implicit row order. Use `ORDER BY`.
2. Using `LIMIT` without deterministic `ORDER BY`.
3. Paginating without a stable tie-breaker.
4. Deep `OFFSET` pagination on large hot paths without considering keyset pagination.
5. `SELECT *` in application contracts, APIs, joins, migrations, or long-lived queries where schema changes can break behavior or pull unnecessary data.
6. `SELECT *` from wide tables with sensitive columns.
7. Joining large tables without understanding cardinality.
8. Using `DISTINCT` to hide duplicate rows caused by a wrong join.
9. Using `GROUP BY` to mask a modeling bug.
10. Cross joins by accident.
11. Unbounded queries in request paths.
12. Unbounded `DELETE` or `UPDATE` without guardrails.
13. `DELETE FROM table` when `TRUNCATE`, partition drop, archival, or batched delete is the real operation.
14. `TRUNCATE` without understanding locks, triggers, identity reset behavior, and referential impact.
15. `NOT IN (subquery)` where nulls are possible. The PostgreSQL wiki warns that `NOT IN` behaves unexpectedly with nulls and can produce very bad plans.
16. `BETWEEN` on timestamps for date ranges. The PostgreSQL wiki recommends half-open ranges like `>= start AND < end`.
17. Using local time arithmetic for global events.
18. Comparing timestamps across time zones without explicit semantics.
19. Casting indexed columns in predicates instead of casting parameters.
20. Wrapping indexed columns in functions in hot predicates without matching expression indexes.
21. `LOWER(email) = LOWER($1)` on a huge table without a functional index or `citext`-style decision.
22. Leading-wildcard `LIKE '%term%'` on large tables without a trigram/search design.
23. `ORDER BY random()` on large tables.
24. `COUNT(*)` on huge filtered result sets in every page render without a plan.
25. Re-querying the same row repeatedly in a loop.
26. N+1 queries hidden behind an ORM.
27. Per-row queries in migrations instead of set-based or batched operations.
28. Fetching entire result sets into application memory.
29. Relying on ORM-generated SQL without looking at the emitted SQL.
30. Not using `EXPLAIN`/`EXPLAIN ANALYZE` for important queries.
31. Comparing query performance only on tiny dev data.
32. Assuming a query is safe because it is fast locally.
33. Ignoring row estimates, sequential scans, sort spill, hash spill, nested loop explosions, or temp files on hot queries.
34. Ignoring `NULL` semantics in joins and filters.
35. Using `=` when `IS NOT DISTINCT FROM` is required for null-aware equality.
36. Using `!=`/`<>` and expecting nulls to match.
37. Building dynamic SQL in stored procedures without bind variables. OWASP notes that stored procedures can still introduce SQL injection if dynamic SQL is constructed unsafely.
38. Treating stored procedures as automatically safer than parameterized SQL.
39. Returning sensitive columns because “the frontend won’t display them.”
40. Using read replicas for read-after-write flows without handling replica lag.

## Reporting, analytics, and warehouse behavior that is not excusable

1. Running heavy analytics directly on the OLTP primary without limits.
2. BI users with production write access.
3. Dashboards that repeatedly run expensive unbounded queries.
4. Reports that depend on implicit ordering.
5. Reports that silently drop rows because joins are wrong.
6. Metrics queries that double-count due to fan-out joins.
7. Slowly changing dimensions modeled as destructive updates when history matters.
8. No reconciliation for financial/counting reports.
9. No definition of metric semantics.
10. No time-zone policy for reporting.
11. No snapshot/as-of-time logic for reports that require historical truth.
12. Materialized views with no refresh strategy.
13. Materialized views refreshed with blocking operations during peak traffic.
14. Caches treated as source of truth.
15. Derived tables with no lineage.
16. Analytics queries given access to raw secrets or unnecessary PII.

## SQL injection and unsafe query construction

1. Building SQL by concatenating user input.
2. Using string interpolation, f-strings, template literals, or ORM “raw SQL” with untrusted values.
3. Treating escaping as the main defense when parameterized queries are possible.
4. Allowing an LLM or user prompt to produce executable SQL without parameter binding.
5. Passing user-controlled table names, column names, sort directions, operators, schemas, or function names without strict allowlists.
6. Dynamically generating `ORDER BY`, `LIMIT`, `OFFSET`, `WHERE`, or `JOIN` clauses from request parameters without validation.
7. Using dynamic SQL inside PL/pgSQL without safe identifier quoting and parameter binding.
8. Logging full SQL statements that include passwords, tokens, personal data, session IDs, or API keys.
9. Pasting production SQL errors, connection strings, or dumps containing secrets into AI tools, tickets, chat, or public forums.
10. Giving an AI coding agent write access to production databases.

## Security

1. Using `trust` authentication for non-local or shared environments.
2. Allowing network access to Postgres without strong authentication.
3. Using cleartext password authentication where avoidable.
4. Continuing to use weak/deprecated password methods when stronger options are available.
5. Disabling TLS for sensitive data paths.
6. Using TLS encryption without server identity verification when MITM matters.
7. Sharing one database credential across developers, services, CI, and production.
8. Hardcoding database passwords in source code.
9. Committing `.env` files, dumps, connection strings, or service-account keys.
10. Reusing production DB credentials in staging, local development, notebooks, or BI tools.
11. Giving long-lived credentials to scripts with no rotation plan.
12. Having no emergency credential revocation path.
13. Letting developers connect to production as the application role.
14. Letting the application connect as a superuser.
15. Letting the application connect as the schema owner.
16. Using passwordless admin access because “it’s behind VPN.”
17. Not auditing production DB logins.
18. Not separating human, app, migration, analytics, and replication roles.
19. Storing user passwords in plaintext.
20. Storing user passwords with reversible encryption instead of password hashing.
21. Storing API keys or secrets in normal app tables without encryption/access policy.
22. Backing up secrets into less secure environments.
23. Printing database URLs in logs, test failures, CI output, or error pages.
24. **Concatenating values into SQL strings.**
25. **Concatenating identifiers into SQL strings without safe identifier quoting.** In PostgreSQL dynamic SQL, identifiers and literals must be handled differently; use parameters for values and safe quoting/formatting such as `quote_ident`, `quote_literal`, `quote_nullable`, or `format()` where dynamic identifiers are unavoidable.
26. **Assuming stored procedures automatically prevent SQL injection.** Stored procedures that internally build dynamic SQL can still be injectable.
27. **Using ORM query builders as a security blanket while still interpolating raw fragments.**
28. **Allowing raw SQL escape hatches without review, tests, and parameterization.**
29. **Exposing database error messages to end users.** MITRE notes SQL error details can reveal query structure and implementation details.
30. **Logging full SQL statements containing passwords, tokens, session IDs, API keys, PII, or payment data.**
31. **Storing plaintext passwords.**
32. **Storing reversible passwords when password verification only requires a salted password hash.**
33. **Storing secrets in normal application tables without encryption, access controls, rotation, and audit logging.**
34. **Using the same database credentials for every service, worker, analyst, and migration.**
35. **Using shared human credentials instead of named users, roles, or audited access.**
36. **Granting broad privileges to `PUBLIC` by accident.** PostgreSQL’s docs describe `PUBLIC` as the implicit group containing all roles; unreviewed grants to it are dangerous.
37. **Granting `ALL` on schemas/tables/sequences as a convenience.**
38. **Letting app roles run arbitrary DDL.**
39. **Letting app roles disable triggers, alter constraints, or bypass RLS.**
40. **Skipping row-level security or equivalent tenant isolation in multi-tenant databases where the database is part of the security boundary.** PostgreSQL supports row-level security policies by command and role; failing to use a database-level boundary when needed leaves tenant isolation to application discipline alone.
41. **Using tenant IDs only in application code while database queries can accidentally cross tenants.**
42. **Making production databases reachable from the public internet without a very narrow, audited, encrypted access path.**
43. **Disabling TLS for client-server database traffic that crosses untrusted networks.** PostgreSQL has native SSL/TLS support for encrypting client-server communication.
44. **Running migrations or scripts as superuser because permissions are messy.**
45. **Letting AI agents, code generators, BI tools, notebooks, or admin dashboards run unrestricted production DML/DDL.**
46. **Copy-pasting SQL from tickets, Slack, Reddit, Stack Overflow, or an LLM into production without review and a transaction/rollback plan.**
47. **Using production data in dev/test without masking, minimization, and access control.**
48. **Allowing ad hoc analyst queries to run without statement timeouts, resource controls, or replica isolation.**
49. **Failing open on permission errors.** Security checks should deny by default.
50. **Using database comments, enum labels, logs, or error strings to store secrets.**
51. **Ignoring SQL injection in non-HTTP surfaces: cron jobs, CSV imports, webhooks, queues, GraphQL filters, BI parameters, and admin tools.** OWASP’s injection category covers SQL and other interpreters generally; SQL injection is not limited to web forms.
52. Concatenating user input into SQL strings.
53. Using string interpolation, f-strings, template literals, `format()`, or ORM raw fragments with untrusted values.
54. Treating escaping as the primary SQL injection defense instead of parameterized queries.
55. Parameterizing values but still allowing users to control table names, column names, sort direction, operators, or raw predicates without an allowlist.
56. Building `ORDER BY`, `LIMIT`, `OFFSET`, `WHERE`, `JOIN`, or `SELECT` clauses from untrusted strings.
57. Exposing a “run SQL” endpoint to users, support agents, admins, or AI agents without strong sandboxing, allowlists, row limits, timeouts, and audit.
58. Running the application as a database superuser.
59. Running normal app traffic as the schema owner.
60. Running app traffic with migration privileges.
61. Granting `ALL` when the app only needs `SELECT`, `INSERT`, `UPDATE`, or `DELETE`.
62. Sharing one database user across unrelated applications, services, jobs, and humans.
63. Using one role for migrations, app writes, app reads, analytics, support tools, and admin tasks.
64. Hardcoding database passwords, URLs, API keys, or service credentials in source code.
65. Committing `.env`, SQL dumps, connection strings, or credentials to Git.
66. Logging full SQL statements with secrets or sensitive parameter values.
67. Storing user passwords with reversible encryption when authentication only needs password verification.
68. Storing weak password hashes without per-password salts and a modern password-hashing scheme.
69. Letting production credentials appear in screenshots, bug reports, prompts, notebooks, traces, crash reports, or CI logs.
70. Disabling RLS to “make it work.”
71. Using a role with `BYPASSRLS` for ordinary application traffic.
72. Assuming RLS protects you when the table owner or superuser is the role executing the query.
73. Writing RLS policies that filter `SELECT` but forget `WITH CHECK` for `INSERT` and `UPDATE`.
74. Writing tenant policies that allow users to move rows into another tenant.
75. Creating `SECURITY DEFINER` functions without a locked-down `search_path`.
76. Creating `SECURITY DEFINER` functions owned by overly privileged roles.
77. Letting untrusted users create functions, triggers, views, operators, or schemas that privileged code might execute.
78. Relying on `search_path` in privileged functions instead of schema-qualifying trusted objects.
79. Creating untrusted procedural-language functions outside tightly controlled superuser-only administration.
80. Letting AI or user-generated SQL reveal schema, sensitive table names, or hidden business structure unnecessarily.
81. Using production data in local dev, CI, demos, or AI prompts without masking or authorization.
82. Backing up sensitive databases to unencrypted, public, or loosely permissioned storage.
83. Granting analysts direct access to production OLTP tables when governed views or replicas are required.
84. Assuming “read-only” means “safe”; read-only queries can still leak sensitive fields, scan too much data, or produce misleading business results.
85. One database role shared by app, migrations, admins, analysts, cron jobs, and BI tools.
86. Analysts connecting with write roles.
87. Migrations running as the same role as the web app.
88. The app owning the schema it uses in production.
89. Public schema create privileges left open in shared databases.
90. Default grants no one understands.
91. RLS policies with no tests.
92. RLS enabled but bypassed by table owner behavior unintentionally; PostgreSQL documents that owners and superusers can bypass RLS unless forced/controlled.
93. Multi-tenant queries that forget `tenant_id`.
94. Tenant isolation enforced only in frontend code.
95. Tenant isolation enforced only in middleware with no DB-level checks for sensitive systems.
96. Storing secrets in tables readable by the app role that does not need them.
97. Storing encryption keys next to encrypted data.
98. Logging full SQL statements with passwords, tokens, API keys, session IDs, reset tokens, or PII.
99. Dumping production databases to developer laptops without masking/minimization.
100. Restoring production data into lower environments without access controls.
101. Backups stored unencrypted or world-readable.
102. No audit trail for privileged access.
103. No rotation plan for database credentials.
104. Long-lived static credentials everywhere when dynamic/rotated credentials are feasible.
105. Exposing Postgres directly to the public internet without a strong network/auth/TLS posture.
106. Using TLS-disabled connections for sensitive data over untrusted networks.
107. Storing PCI/health/legal/sensitive data without classification and controls.
108. Granting `BYPASSRLS` casually.
109. Installing unreviewed extensions in production.
110. Running untrusted procedural languages/extensions without a threat model.

## Soft-delete abuse

1. Adding `deleted_at` everywhere by default with no purge policy.
2. Soft-deleting rows that are still referenced by active rows without a clear integrity model.
3. Letting foreign keys point to soft-deleted rows accidentally.
4. Forgetting partial unique constraints for “only one active row” cases.
5. Letting normal queries forget `WHERE deleted_at IS NULL`.
6. Treating soft delete as compliance deletion when the data must actually be erased or scrubbed.
7. Mixing active data, deleted data, archive data, and audit data in the same table with no lifecycle plan.
8. Keeping soft-deleted data forever because nobody owns deletion.
9. Using soft deletes to avoid understanding business state transitions.
10. Soft-deleting parent rows while child rows remain “active.”

## Soft-delete and lifecycle behavior that is not excusable

1. Soft deletes everywhere by default.
2. Soft deletes without partial unique indexes for active records.
3. Soft deletes that break foreign keys.
4. Soft deletes that make every query remember `deleted_at IS NULL`.
5. Soft deletes with no purge/retention policy.
6. Soft deletes with no restore semantics.
7. Soft deletes used to avoid real audit tables.
8. Audit history stored by overwriting rows.
9. No append-only audit log for security/financial events.
10. “Deleted” data still visible to analytics, exports, or permissions queries.
11. Hard-deleting data that must be retained for audit/legal/business reasons.
12. Retaining data that must be deleted/minimized for privacy/security reasons.

## Statistics, vacuum, and planner neglect

1. Disabling autovacuum because it is “annoying.”
2. Disabling autoanalyze and never running `ANALYZE`.
3. Ignoring stale planner statistics.
4. Bulk-loading or backfilling data and never analyzing afterward.
5. Treating `VACUUM FULL` as routine maintenance on hot production tables.
6. Ignoring dead tuples, table bloat, transaction ID wraparound, and freeze age.
7. Ignoring slow query logs and query statistics.
8. Not enabling or using `pg_stat_statements` in serious production systems.
9. Optimizing based on one local query run rather than production-like data and statistics.

## Testing

1. Testing only on empty databases.
2. Testing only on tiny seed data.
3. Testing without duplicates.
4. Testing without nulls.
5. Testing without old rows.
6. Testing without skewed distributions.
7. Testing without large tenants/customers/accounts.
8. Testing without concurrent writers.
9. Testing without concurrent readers.
10. Testing without realistic indexes.
11. Testing with SQLite while production is Postgres and assuming behavior is equivalent.
12. Testing without the same constraints as production.
13. Testing without the same extensions as production.
14. Testing without the same collations as production.
15. Testing without the same timezone assumptions.
16. Testing without migration replay.
17. Testing migrations only forward, never rollback/repair.
18. Testing application code but not database invariants.
19. Mocking the database for code whose correctness depends on SQL semantics.
20. Not testing failure paths: duplicate insert, FK violation, deadlock, timeout, serialization failure.
21. Not testing permission failures.
22. Not testing tenant isolation failures.
23. No fixtures for edge-case timestamps: DST, leap days, month-end, year-end.
24. No fixtures for high-cardinality and low-cardinality query plans.
25. No test that generated SQL is parameterized.
26. No test that every migration is idempotent or safely re-runnable where expected.
27. Merging SQL that has no tests.
28. Merging migrations that have not been tested from a realistic previous schema state.
29. Testing only with tiny seed data.
30. Testing only the happy path.
31. No tests for `NULL`.
32. No tests for duplicate data.
33. No tests for boundary timestamps.
34. No tests for daylight-saving transitions when time matters.
35. No tests for multi-tenant isolation.
36. No tests for soft-deleted rows.
37. No tests for concurrent inserts/updates.
38. No tests for race conditions on uniqueness or inventory-like invariants.
39. No tests for rollback or failed migration scenarios.
40. No `EXPLAIN` review for high-traffic queries.
41. No review by someone who understands PostgreSQL when the change is database-significant.
42. Treating “the ORM generated it” as review.
43. Treating “the LLM generated it” as review.
44. Treating “it worked locally” as review.
45. Treating “the query returned the right rows once” as review.
46. Not checking production-like cardinality, skew, and selectivity.
47. Not checking lock behavior.
48. Not checking operational impact.
49. No SQL tests for critical business logic.
50. No migration tests.
51. No rollback or roll-forward tests.
52. No tests for constraints.
53. No tests for RLS policies.
54. No tests for tenant isolation.
55. No tests for soft-delete behavior.
56. No tests for cascade behavior.
57. No tests for duplicate prevention.
58. No tests for NULL behavior.
59. No tests for timezone boundaries.
60. No tests for daylight saving time.
61. No tests for month-end, quarter-end, and year-end.
62. No tests for concurrent updates.
63. No tests for retryable transaction failures.
64. No tests for largest tenant/customer/account.
65. No tests on production-like row counts.
66. No tests for slow queries.
67. No query-plan review for nontrivial query changes.
68. No review of generated ORM SQL.
69. No review of raw SQL.
70. No review of triggers/functions.
71. No review of RLS.
72. No review of privileges.
73. No review of migrations by someone who understands locks.
74. No review of destructive changes.
75. No restore drill.
76. No backup failure drill.
77. No monitoring/alert tests.
78. No data-quality checks after migration.
79. No tests for database constraints.
80. No tests for migrations.
81. No tests for rollback or forward-fix.
82. No tests for RLS/tenant isolation.
83. No tests for concurrency races.
84. No tests for duplicate inserts.
85. No tests for time-zone boundary behavior.
86. No tests around daylight saving transitions when local time matters.
87. No tests for destructive data migrations.
88. No fixture or seed data that resembles real cardinality.
89. Performance tests only on empty tables.
90. Query snapshots never reviewed.
91. Schema diffs never reviewed.
92. `EXPLAIN` output absent for critical query changes.
93. No review of generated SQL from ORM changes.
94. No linting/static analysis for SQL injection or dangerous migrations.
95. Ignoring CodeQL/linters/security scanners because “the code works.”
96. No migration checklist.
97. No database owner approval for high-risk schema changes.
98. No incident postmortem after a database-caused outage.
99. Repeating the same bad migration pattern after an incident.
100. No documentation for invariants that the schema cannot express.

## The highest-severity red lines

1. **Concatenating user input into SQL.** Use server-side parameters or prepared statements. Escaping is not an acceptable primary defense. OWASP explicitly says to stop writing dynamic queries with string concatenation, and GitHub CodeQL flags user-controlled query construction as a high-severity SQL injection pattern.
2. **Concatenating user-controlled identifiers, sort columns, table names, `ORDER BY`, `LIMIT`, or `WHERE` fragments without an allowlist.** Parameters protect values, not arbitrary SQL structure.
3. **Running AI-generated SQL directly against production.** Generated SQL must be reviewed like hand-written destructive code: threat model, transaction behavior, locks, rollback, plan, data volume, and blast radius.
4. **Using a production application role with superuser, owner, `CREATEDB`, `CREATEROLE`, broad DDL, or unrestricted write privileges.** PostgreSQL’s role system exists to control object ownership and privileges; using one omnipotent role for everything defeats that control.
5. **Using `trust` authentication over TCP/IP in production.** The PostgreSQL wiki explicitly says not to use `trust` over TCP/IP in production and calls `host all all 0.0.0.0/0 trust` especially dangerous.
6. **Hardcoding database passwords, API keys, encryption keys, or connection strings in source, migrations, notebooks, shell history, Dockerfiles, SQL files, or CI logs.** OWASP identifies database credentials as secrets and recommends centralized, auditable, rotatable secrets management with least privilege.
7. **Storing plaintext passwords, reversible-encrypted passwords, or fast hashes such as MD5/SHA-1 for passwords.** Passwords require purpose-built adaptive hashing such as Argon2id, bcrypt, scrypt, or PBKDF2 with proper salts/work factors; OWASP also says passwords should not be stored using reversible encryption.
8. **Storing sensitive data unnecessarily.** Data that is not retained cannot be stolen; OWASP explicitly recommends minimizing sensitive storage and encrypting sensitive data at rest and in transit.
9. **Turning off `fsync` or `full_page_writes` for persistent production data.** PostgreSQL warns that disabling `fsync` can cause unrecoverable corruption after a crash, and disabling `full_page_writes` can lead to unrecoverable or silent corruption.
10. **Operating without tested backups and point-in-time recovery for important data.** PostgreSQL’s WAL-based backup/PITR design exists so you can restore a consistent database state to a chosen time; having “backups” you have never restored is not a recovery plan.
11. **Running destructive DDL/DML without a rollback or recovery plan.** `DROP`, `TRUNCATE`, bulk `DELETE`, bulk `UPDATE`, `ALTER TABLE`, and backfills must have a tested undo/recovery story.
12. **Deploying migrations to production without understanding locks.** PostgreSQL DDL takes locks; migration-safety tools and GitLab’s own guidelines exist because unsafe migrations can block traffic or corrupt application behavior.
13. **Creating indexes non-concurrently on large hot production tables.** Standard `CREATE INDEX` blocks writes; PostgreSQL documents `CREATE INDEX CONCURRENTLY` for avoiding write-blocking, and GitLab requires special care, testing, and asynchronous creation for very large tables.
14. **Adding foreign keys, `NOT NULL`, large defaults, or validation constraints to big tables in one naïve migration.** These can scan/rewrite/lock large tables; safe migration helpers split, validate, retry, and time-limit operations.
15. **Doing bulk data migrations in a normal request/transaction path.** GitLab recommends batched background migrations when a data migration exceeds time limits or touches high-traffic/large tables.
16. **Assuming the application alone will preserve integrity when the database can enforce it.** PostgreSQL constraints exist to prevent invalid data regardless of which client writes it; constraints reject invalid values even if the value came from a default.
17. **Using a multi-tenant database without database-enforced tenant isolation where the blast radius is high.** PostgreSQL Row-Level Security can restrict rows per user/role; by default, if RLS is enabled but no policy exists, access is default-deny.
18. **Disabling constraints, triggers, RLS, permissions, or auditing “temporarily” without a tracked, time-boxed, reviewed recovery step.** Temporary safety removal becomes permanent corruption.
19. **Manual production fixes that are not captured in versioned migrations or audited runbooks.** “I fixed it in psql” is how systems become unreproducible.
20. **Treating a database as disposable cache when it is actually the source of truth.** If customers, money, permissions, orders, ledgers, inventory, or audit events depend on it, it needs durability, integrity, observability, and recovery.

## Time and timezone mistakes

1. Using `timestamp without time zone` for real-world instants.
2. “Storing UTC” in a naive timestamp column and expecting everyone to remember that forever.
3. Storing timezone offsets as text instead of modeling time correctly.
4. Using `timetz`.
5. Using `current_time`.
6. Using `timestamp(0)` and silently rounding time.
7. Using `BETWEEN` for timestamp ranges where the correct logic is a half-open interval.
8. Comparing timestamps in application-local time without an explicit timezone policy.
9. Mixing server timezone, database timezone, user timezone, and business timezone accidentally.
10. Using local wall-clock time for ordering globally distributed events.
11. Building daily/monthly reports without defining which timezone defines the day or month.
12. Ignoring daylight-saving transitions in scheduling and recurrence logic.

## Transaction and concurrency behavior that is not excusable

1. Assuming `READ COMMITTED` means “the whole transaction sees one consistent world.” PostgreSQL documents the different isolation levels and which anomalies are possible.
2. Assuming `READ UNCOMMITTED` gives dirty reads in PostgreSQL; PostgreSQL maps it to `READ COMMITTED`.
3. Read-modify-write cycles outside a transaction.
4. `SELECT` then `INSERT` race conditions instead of `INSERT ... ON CONFLICT`, constraints, or locks.
5. Checking “does row exist?” in application code without a unique constraint.
6. Implementing counters by reading a value, adding one in app code, and writing it back without atomic SQL or locking.
7. Not handling deadlocks.
8. Not retrying serialization failures when using serializable isolation.
9. Holding transactions open while making network calls.
10. Holding transactions open while waiting for user input.
11. “Idle in transaction” sessions in production.
12. Running long reports in the primary OLTP transaction context without limits.
13. Mixing OLTP writes and long analytical reads without isolation/resource planning.
14. No `statement_timeout`.
15. No `lock_timeout` for migrations or risky maintenance.
16. No `idle_in_transaction_session_timeout`.
17. Acquiring locks in inconsistent order across code paths.
18. Using table locks as application mutexes.
19. Using advisory locks without a documented key namespace and failure behavior.
20. Assuming advisory locks protect data from sessions that do not use the same advisory lock.
21. Swallowing database errors and continuing the transaction.
22. Retrying non-idempotent writes blindly.
23. No idempotency key for externally retried financial/order/job operations.
24. Generating IDs in the app and assuming collisions cannot happen.
25. Using timestamp order as a correctness mechanism.
26. Assuming two SQL statements are atomic because they are adjacent in code.

## Unsafe and unstable query shape

1. `SELECT *` in production APIs, views, unions, ETL contracts, application queries, or migrations.
2. Relying on implicit column order.
3. `INSERT INTO table VALUES (...)` without a column list.
4. Unqualified column names in joins.
5. Creating views or functions whose behavior changes when a column is added.
6. Using `SELECT *` on both sides of a `UNION`.
7. Adding columns to a table without checking code that uses `SELECT *`.
8. Returning more columns than the caller needs by default.
9. Pulling entire rows just to check existence.
10. Fetching large result sets into application memory instead of streaming or paginating.
11. Using `LIMIT` without deterministic `ORDER BY`.
12. Paginating by non-unique sort keys without a tie-breaker.
13. Assuming `created_at` is unique enough for stable pagination.
14. Relying on default row order.
15. Using offset pagination for large, frequently changing datasets where keyset pagination is needed.

## Unsafe production migrations

1. Running hand-written production DDL outside version control.
2. Running migrations without review.
3. Running migrations without knowing lock behavior.
4. Running migrations without `lock_timeout` and `statement_timeout`.
5. Running migrations that require downtime when an online path exists.
6. Mixing risky schema changes and large data backfills in one transaction.
7. Backfilling millions of rows synchronously in a deploy migration.
8. Running large data migrations inside web requests.
9. Adding a required column, constraint, or index without checking existing data.
10. Adding `NOT NULL` without a safe validation/backfill path.
11. Adding unique constraints without deduplicating existing data first.
12. Adding foreign keys without validating existing references.
13. Dropping columns before all old code paths stop reading them.
14. Renaming columns or tables in one deploy without compatibility handling.
15. Changing column types on hot large tables without testing rewrite/lock behavior.
16. Creating indexes non-concurrently on large live tables.
17. Running multiple heavyweight migrations in one deploy.
18. Having no rollback plan for destructive migrations.
19. Writing migrations that depend on application models that may change later.
20. Using irreversible migrations casually.
21. Failing to test migrations on production-like data volume.
22. Failing to test rollback or forward-fix paths.
23. Failing to coordinate application deploy order with schema compatibility.
24. Treating ORM-generated migrations as automatically safe.
25. Accepting an LLM-generated migration that has not been lock-tested, reviewed, and rehearsed.

## Wrong data types

1. Storing dates as text.
2. Storing timestamps as text.
3. Storing numbers as text.
4. Storing booleans as `'true'`, `'false'`, `'Y'`, `'N'`, `0`, or `1` when a boolean type is appropriate.
5. Storing UUIDs as text when `uuid` is available.
6. Storing IP addresses as text when `inet`/`cidr` would be appropriate.
7. Storing money in floating-point columns.
8. Using approximate floats for exact quantities such as money, inventory, votes, counts, or billing.
9. Using `real`/`double precision` for equality joins or exact grouping.
10. Using `numeric` with no thought to precision, scale, and performance.
11. Using `varchar(255)` everywhere by habit.
12. Using `char(n)` for ordinary text.
13. Using `text` for small enumerated domains without a constraint or lookup table.
14. Using Postgres `money` casually instead of a deliberate currency model.
15. Mixing currencies in one numeric column without a currency column and constraint.
16. Mixing units in one column without a unit column and constraint.
17. Using magic sentinel values such as `-1`, `0`, `'unknown'`, `'N/A'`, `'1970-01-01'`, or `'9999-12-31'` instead of modeling the state correctly.
18. Using JSON strings to represent typed values that PostgreSQL can natively validate.

## References

[1] https://faculty.cc.gatech.edu/~jarulraj/courses/8803-f18/papers/smelly_relations.pdf "Smelly Relations:Measuring and Understanding Database Schema Quality"
[2] https://www.red-gate.com/simple-talk/ai/vibe-coding-and-databases-the-hidden-risks-of-ai-generated-database-code/ "Vibe coding and databases: the hidden risk of AI-generated database code"
[3] https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html "SQL Injection Prevention - OWASP Cheat Sheet Series"
[4] https://www.postgresql.org/docs/current/ddl-priv.html "PostgreSQL: Documentation: 18: 5.8. Privileges"
[5] https://www.postgresql.org/docs/current/sql-createfunction.html "PostgreSQL: Documentation: 18: CREATE FUNCTION"
[6] https://www.postgresql.org/docs/current/ddl-constraints.html "PostgreSQL: Documentation: 18: 5.5. Constraints"
[7] https://pragprog.com/titles/bksap1/sql-antipatterns-volume-1/?utm_source=chatgpt.com "SQL Antipatterns, Volume 1"
[8] https://www.enterprisedb.com/blog/postgresql-anti-patterns-unnecessary-jsonhstore-dynamic-columns "PostgreSQL anti-patterns: Unnecessary json/hstore dynamic columns | EDB"
[9] https://wiki.postgresql.org/wiki/Don%27t_Do_This "Don't Do This - PostgreSQL wiki"
[10] https://www.postgresql.org/docs/current/explicit-locking.html?utm_source=chatgpt.com "Documentation: 18: 13.3. Explicit Locking"
[11] https://www.postgresql.org/docs/current/using-explain.html?utm_source=chatgpt.com "Documentation: 18: 14.1. Using EXPLAIN"
[12] https://www.postgresql.org/docs/current/sql-explain.html?utm_source=chatgpt.com "PostgreSQL: Documentation: 18: EXPLAIN"
[13] https://www.postgresql.org/docs/8.0/indexes.html?utm_source=chatgpt.com "Indexes - PostgreSQL: Documentation: 8.0"
[14] https://docs.gitlab.com/development/database/layout_and_access_patterns/ "Best practices for data layout and access patterns | GitLab Docs"
[15] https://www.postgresql.org/docs/current/transaction-iso.html?utm_source=chatgpt.com "Documentation: 18: 13.2. Transaction Isolation"
[16] https://docs.gitlab.com/development/migration_style_guide/ "Migration Style Guide | GitLab Docs"
[17] https://www.postgresql.org/docs/current/continuous-archiving.html?utm_source=chatgpt.com "25.3. Continuous Archiving and Point-in-Time Recovery ..."
[18] https://www.reddit.com/r/PostgreSQL/comments/1jstmhz/postgres_antipatterns_pet_peeves/ "Postgres anti-patterns & pet peeves : r/PostgreSQL"
[19] https://www.postgresql.org/docs/current/auth-trust.html "PostgreSQL: Documentation: 18: 20.4. Trust Authentication"
[20] https://www.postgresql.org/docs/current/predefined-roles.html "PostgreSQL: Documentation: 18: 21.5. Predefined Roles"
[21] https://github.com/boralp/sql-anti-patterns?utm_source=chatgpt.com "boralp/sql-anti-patterns"
[22] https://www.postgresql.org/docs/8.1/datatype.html?utm_source=chatgpt.com "Documentation: 8.1: Data Types"
[23] https://www.postgresql.org/docs/current/mvcc.html?utm_source=chatgpt.com "Documentation: 18: Chapter 13. Concurrency Control"
[24] https://www.postgresql.org/docs/current/sql-createindex.html "PostgreSQL: Documentation: 18: CREATE INDEX"
[25] https://www.postgresql.org/docs/current/sql-altertable.html "PostgreSQL: Documentation: 18: ALTER TABLE"
[26] https://www.postgresql.org/docs/9.1/backup.html?utm_source=chatgpt.com "PostgreSQL: Documentation: 9.1: Backup and Restore"
[27] https://www.postgresql.org/docs/current/pgstatstatements.html?utm_source=chatgpt.com "F.32. pg_stat_statements — track statistics of SQL planning ..."
[28] https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html?utm_source=chatgpt.com "Password Storage Cheat Sheet"
[29] https://docs.gitlab.com/development/database/?utm_source=chatgpt.com "Database development guidelines"
[30] https://www.reddit.com/r/PostgreSQL/comments/1jstmhz/postgres_antipatterns_pet_peeves/?utm_source=chatgpt.com "Postgres anti-patterns & pet peeves : r/PostgreSQL"
[31] https://cheatsheetseries.owasp.org/cheatsheets/Database_Security_Cheat_Sheet.html "Database Security - OWASP Cheat Sheet Series"
[32] https://www.postgresql.org/docs/current/auth-pg-hba-conf.html "PostgreSQL: Documentation: 18: 20.1. The pg_hba.conf File"
[33] https://www.postgresql.org/docs/current/ddl-rowsecurity.html "PostgreSQL: Documentation: 18: 5.9. Row Security Policies"
[34] https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html "Password Storage - OWASP Cheat Sheet Series"
[35] https://pages.nist.gov/800-63-4/sp800-63b.html "NIST Special Publication 800-63B"
[36] https://cvw.cac.cornell.edu/RelationalDBs/design-create/database_normalization "Cornell Virtual Workshop > Relational Databases > Designing/creating Relational Databases > Database Normalization"
[37] https://pragprog.com/titles/bksap1/sql-antipatterns-volume-1/ "SQL Antipatterns, Volume 1: Avoiding the Pitfalls of Database Programming by Bill Karwin"
[38] https://github.com/boralp/sql-anti-patterns "GitHub - boralp/sql-anti-patterns: List of anti patterns in SQL · GitHub"
[39] https://www.postgresql.org/docs/current/datatype-numeric.html?utm_source=chatgpt.com "Documentation: 18: 8.1. Numeric Types"
[40] https://docs.gitlab.com/development/sql/ "SQL Query Guidelines | GitLab Docs"
[41] https://github.com/sql-anti-patterns/sql-anti-patterns.github.io "GitHub - sql-anti-patterns/sql-anti-patterns.github.io: A collection of SQL Anti-Patterns · GitHub"
[42] https://www.postgresql.org/docs/current/transaction-iso.html "PostgreSQL: Documentation: 18: 13.2. Transaction Isolation"
[43] https://www.postgresql.org/docs/current/explicit-locking.html "PostgreSQL: Documentation: 18: 13.3. Explicit Locking"
[44] https://docs.gitlab.com/development/database/adding_database_indexes/ "Adding Database Indexes | GitLab Docs"
[45] https://docs.gitlab.com/development/database_review/ "Database Review Guidelines | GitLab Docs"
[46] https://www.postgresql.org/docs/current/routine-vacuuming.html "PostgreSQL: Documentation: 18: 24.1. Routine Vacuuming"
[47] https://arxiv.org/abs/2004.10232 "[2004.10232] SQLCheck: Automated Detection and Diagnosis of SQL Anti-Patterns"
[48] https://www.postgresql.org/docs/current/libpq-exec.html "PostgreSQL: Documentation: 18: 32.3. Command Execution Functions"
[49] https://docs.gitlab.com/development/database/foreign_keys/ "Foreign keys and associations | GitLab Docs"
[50] https://www.postgresql.org/docs/current/pgstatstatements.html "PostgreSQL: Documentation: 18: F.32. pg_stat_statements — track statistics of SQL planning and execution"
[51] https://www.postgresql.org/docs/current/backup.html "PostgreSQL: Documentation: 18: Chapter 25. Backup and Restore"
[52] https://link.springer.com/article/10.1007/s10664-023-10295-x "Studying the characteristics of SQL-related development tasks: An empirical study | Empirical Software Engineering | Springer Nature Link"
[53] https://arxiv.org/abs/2201.02215?utm_source=chatgpt.com "On the Prevalence, Impact, and Evolution of SQL Code Smells in Data-Intensive Systems"
[54] https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html?utm_source=chatgpt.com "SQL Injection Prevention Cheat Sheet"
[55] https://cwe.mitre.org/data/definitions/89.html "CWE -
[56] https://github.com/timescale/pg-aiguide?utm_source=chatgpt.com "timescale/pg-aiguide: MCP server and Claude plugin for ..."
[57] https://www.postgresql.org/docs/current/plpgsql-statements.html?utm_source=chatgpt.com "PostgreSQL: Documentation: 18: 41.5. Basic Statements"
[58] https://www.postgresql.org/docs/current/ddl-priv.html?utm_source=chatgpt.com "Documentation: 18: 5.8. Privileges"
[59] https://www.postgresql.org/docs/current/ddl-rowsecurity.html?utm_source=chatgpt.com "PostgreSQL: Documentation: 18: 5.9. Row Security Policies"
[60] https://www.postgresql.org/docs/current/ssl-tcp.html?utm_source=chatgpt.com "18: 18.9. Secure TCP/IP Connections with SSL"
[61] https://owasp.org/Top10/2021/A03_2021-Injection/?utm_source=chatgpt.com "A03 Injection - OWASP Top 10:2021"
[62] https://www.reddit.com/r/PostgreSQL/comments/17ujp7d/best_way_to_store_and_reference_enums/?utm_source=chatgpt.com "Best way to store and reference \"enums\" : r/PostgreSQL"
[63] https://www.reddit.com/r/SQL/comments/1o42xnz/question_whats_one_of_those_sql_gotchas_that_only/?utm_source=chatgpt.com "What's one of those SQL “gotchas” that only made sense to ..."
[64] https://www.postgresql.org/docs/current/planner-stats.html?utm_source=chatgpt.com "Documentation: 18: 14.2. Statistics Used by the Planner"
[65] https://www.postgresql.org/docs/current/indexes.html?utm_source=chatgpt.com "Documentation: 18: Chapter 11. Indexes"
[66] https://www.postgresql.org/docs/current/indexes-examine.html?utm_source=chatgpt.com "Documentation: 18: 11.12. Examining Index Usage"
[67] https://www.postgresql.org/docs/current/mvcc-intro.html?utm_source=chatgpt.com "PostgreSQL: Documentation: 18: 13.1. Introduction"
[68] https://docs.gitlab.com/development/database/avoiding_downtime_in_migrations/ "Avoiding downtime in migrations | GitLab Docs"
[69] https://www.postgresql.org/docs/current/continuous-archiving.html "PostgreSQL: Documentation: 18: 25.3. Continuous Archiving and Point-in-Time Recovery (PITR)"
[70] https://www.tigerdata.com/blog/we-taught-ai-to-write-real-postgres-code-open-sourced-it?utm_source=chatgpt.com "We Taught AI to Write Real Postgres Code | Tiger Data"
[71] https://x.com/_avichawla/status/2005527034127433933?utm_source=chatgpt.com "Big moment for Postgres! AI coding tools have been ..."
[72] https://www.reddit.com/r/SQL/comments/57mhs6/how_common_are_bad_db_designs/?utm_source=chatgpt.com "How common are bad DB designs? : r/SQL"
[73] https://www.postgresql.org/docs/current/ddl-schemas.html "PostgreSQL: Documentation: 18: 5.10. Schemas"
[74] https://www.postgresql.org/docs/current/perm-functions.html "PostgreSQL: Documentation: 18: 21.6. Function Security"
[75] https://www.reddit.com/r/SQL/comments/57mhs6/how_common_are_bad_db_designs/ "How common are bad DB designs? : r/SQL"
[76] https://www.cybertec-postgresql.com/en/json-postgresql-how-to-use-it-right/ "JSON in PostgreSQL: how to use it right | CYBERTEC PostgreSQL | Services & Support"
[77] https://www.postgresql.org/docs/current/using-explain.html "PostgreSQL: Documentation: 18: 14.1. Using EXPLAIN"
[78] https://github.com/mfvanek/pg-index-health/blob/master/doc/available_checks.md "pg-index-health/doc/available_checks.md at master · mfvanek/pg-index-health · GitHub"
[79] https://www.postgresql.org/docs/current/routine-vacuuming.html?utm_source=chatgpt.com "Documentation: 18: 24.1. Routine Vacuuming"
[80] https://www.enterprisedb.com/blog/postgresql-anti-patterns-read-modify-write-cycles "Avoid PostgreSQL Anti-patterns: Understanding Read-Modify-Write Cycles"
[81] https://squawkhq.com/docs/require-concurrent-index-creation "require-concurrent-index-creation | Squawk — a linter for Postgres migrations"
[82] https://squawkhq.com/docs/adding-not-nullable-field "adding-not-nullable-field | Squawk — a linter for Postgres migrations"
[83] https://squawkhq.com/docs/safe_migrations "Applying migrations safely | Squawk — a linter for Postgres migrations"
[84] https://retool.com/blog/vibe-coding-risks "Retool Blog | The Risks of Vibe Coding: Security Vulnerabilities and Enterprise Pitfalls"
[85] https://www.dpriver.com/blog/why-enterprises-should-not-let-llms-execute-sql-directly/ "Why Enterprises Should Not Let LLMs Execute SQL Directly"
[86] https://cwe.mitre.org/data/definitions/798.html "CWE -
[87] https://www.postgresql.org/docs/current/datatype-json.html "PostgreSQL: Documentation: 18: 8.14. JSON Types"
[88] https://www.postgresql.org/docs/current/datatype-datetime.html "PostgreSQL: Documentation: 18: 8.5. Date/Time Types"
[89] https://www.postgresql.org/docs/current/mvcc.html "PostgreSQL: Documentation: 18: Chapter 13. Concurrency Control"
[90] https://www.postgresql.org/docs/current/xfunc-volatility.html "PostgreSQL: Documentation: 18: 36.7. Function Volatility Categories"
[91] https://www.postgresql.org/docs/current/planner-stats.html "PostgreSQL: Documentation: 18: 14.2. Statistics Used by the Planner"
[92] https://www.postgresql.org/docs/current/indexes-multicolumn.html "PostgreSQL: Documentation: 18: 11.3. Multicolumn Indexes"
[93] https://www.postgresql.org/docs/current/sql-vacuum.html "PostgreSQL: Documentation: 18: VACUUM"
[94] https://www.postgresql.org/docs/current/user-manag.html "PostgreSQL: Documentation: 18: Chapter 21. Database Roles"
[95] https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html "Secrets Management - OWASP Cheat Sheet Series"
[96] https://owasp.org/Top10/2021/A02_2021-Cryptographic_Failures/ "A02 Cryptographic Failures - OWASP Top 10:2021"
[97] https://www.postgresql.org/docs/current/runtime-config-wal.html "PostgreSQL: Documentation: 18: 19.5. Write Ahead Log"
[98] https://github.com/doctolib/safe-pg-migrations "GitHub - doctolib/safe-pg-migrations: Make your PostgreSQL migrations safe · GitHub"
[99] https://www.postgresql.org/docs/current/sql-createindex.html?utm_source=chatgpt.com "Documentation: 18: CREATE INDEX"
[100] https://docs.gitlab.com/development/database/batched_background_migrations/ "Batched background migrations | GitLab Docs"
[101] https://pragprog.com/titles/bksqla/sql-antipatterns/?utm_source=chatgpt.com "SQL Antipatterns - Bill Karwin"
[102] https://cheatsheetseries.owasp.org/cheatsheets/Query_Parameterization_Cheat_Sheet.html "Query Parameterization - OWASP Cheat Sheet Series"
